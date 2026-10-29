// Particiona apenas a representação de aparência das faces. O B-rep de origem
// permanece intacto; dentro/fora dos prismas preservam curvas e furos exatos.
#pragma once
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepBuilderAPI_MakeSolid.hxx>
#include <TopoDS_Shell.hxx>
#include <TopoDS_Solid.hxx>
namespace {
struct PaintPiece {TopoDS_Face face; int source; int material;};
std::vector<PaintPiece> paint_pieces(const Shape* body,const std::size_t* faces,const Shape* const* tools,std::size_t count) {
        if (count==0 || count>255) throw std::runtime_error("Quantidade de pinturas inválida.");
        const auto original=map_of(body->value,TopAbs_FACE);
        std::vector<PaintPiece> pieces;
        for (int i=1;i<=original.Extent();++i) pieces.push_back({TopoDS::Face(original(i)),i-1,0});
        for (std::size_t layer=0;layer<count;++layer) {
            if (faces[layer]>=static_cast<std::size_t>(original.Extent())) throw std::runtime_error("Face pintada inexistente.");
            std::vector<PaintPiece> next;
            bool touched=false;
            for (const auto& piece:pieces) {
                if (piece.source!=static_cast<int>(faces[layer])) {next.push_back(piece);continue;}
                if (!tools[layer]) {next.push_back({piece.face,piece.source,static_cast<int>(layer)+1});touched=true;continue;}
                BRepAlgoAPI_Common inside(piece.face,tools[layer]->value);
                BRepAlgoAPI_Cut outside(piece.face,tools[layer]->value);
                if (!inside.IsDone() || !outside.IsDone()) throw std::runtime_error("Não foi possível delimitar a pintura pelo rascunho.");
                const auto append=[&](const TopoDS_Shape& shape,int material,bool selected) {
                    for (TopExp_Explorer it(shape,TopAbs_FACE);it.More();it.Next()) {
                        const auto face=TopoDS::Face(it.Current());
                        GProp_GProps properties;BRepGProp::SurfaceProperties(face,properties);
                        if (properties.Mass()<=1e-12) continue;
                        next.push_back({face,piece.source,material});
                        if (selected) touched=true;
                        if (next.size()>1000000) throw std::runtime_error("Pintura excede o limite de faces.");
                    }
                };
                append(outside.Shape(),piece.material,false);
                append(inside.Shape(),static_cast<int>(layer)+1,true);
            }
            if (!touched) throw std::runtime_error("A área do rascunho não alcança a face escolhida.");
            pieces.swap(next);
        }
        return pieces;
}
// A ordem de faces do resultado determina os índices de cores atravessando o ABI.
void paint_origins(Shape& output,const std::vector<PaintPiece>& pieces) {
        const auto mapped=map_of(output.value,TopAbs_FACE);
        output.origins.assign(static_cast<std::size_t>(mapped.Extent())*3,0);
        std::vector<bool> assigned(mapped.Extent(),false);
        for (const auto& piece:pieces) {
            const auto index=mapped.FindIndex(piece.face);
            if (index<1) throw std::runtime_error("Aparência perdeu a face de origem.");
            if (assigned[index-1] && output.origins[(index-1)*3+2]!=piece.material)
                throw std::runtime_error("Aparência associou cores incompatíveis à mesma face.");
            assigned[index-1]=true;
            output.origins[(index-1)*3+1]=piece.source;
            output.origins[(index-1)*3+2]=piece.material;
        }
        if (std::find(assigned.begin(),assigned.end(),false)!=assigned.end())
            throw std::runtime_error("Aparência perdeu a cor de uma face.");
}
}
extern "C" {
Shape* nator_surface_appearance(const Shape* body,const std::size_t* faces,const Shape* const* tools,std::size_t count) noexcept {
    return build([&](Shape& output) {
        const auto pieces=paint_pieces(body,faces,tools,count);
        TopoDS_Compound compound;BRep_Builder builder;builder.MakeCompound(compound);
        for (const auto& piece:pieces) builder.Add(compound,piece.face);
        output.value=compound;
        paint_origins(output,pieces);
    });
}
}

// Sobreposição exata para transferir pintura somente às faces que descendem da
// superfície pintada. Não resolve nomes repetidos por proximidade ou por ordem.
extern "C" bool nator_face_common_area(const Shape* first,std::size_t a,const Shape* second,std::size_t b,double* output) noexcept {
    return guarded([&] {
        const auto left=map_of(first->value,TopAbs_FACE);
        const auto right=map_of(second->value,TopAbs_FACE);
        if (a>=static_cast<std::size_t>(left.Extent()) || b>=static_cast<std::size_t>(right.Extent())) throw std::runtime_error("Face de pintura inexistente.");
        BRepAlgoAPI_Common common(left(static_cast<int>(a)+1),right(static_cast<int>(b)+1));
        if (!common.IsDone()) throw std::runtime_error("Não foi possível transferir a pintura entre superfícies.");
        GProp_GProps properties;BRepGProp::SurfaceProperties(common.Shape(),properties);
        *output=properties.Mass();
    });
}

// O modelo e suas referências ficam intactos. Costuramos apenas uma cópia para
// intercâmbio, incluindo as arestas compartilhadas cortadas pelas máscaras.
// Cada sólido e suas cascas (inclusive cavidades internas) são reconstruídos
// separadamente: regiões coloridas jamais viram corpos independentes.
extern "C" Shape* nator_painted_solid(const Shape* body,const std::size_t* faces,const Shape* const* tools,std::size_t count) noexcept {
    return build([&](Shape& output) {
        const auto pieces=paint_pieces(body,faces,tools,count);
        if (std::all_of(tools,tools+count,[](const Shape* tool){return tool==nullptr;})) {
            output.value=body->value;
            paint_origins(output,pieces);
            return;
        }
        const auto original_faces=map_of(body->value,TopAbs_FACE);
        std::vector<PaintPiece> sewn_pieces;
        TopoDS_Compound compound; BRep_Builder builder; builder.MakeCompound(compound);
        std::size_t solids=0;
        for (TopExp_Explorer solid(body->value,TopAbs_SOLID);solid.More();solid.Next()) {
            BRepBuilderAPI_MakeSolid closed;
            for (TopExp_Explorer shell(solid.Current(),TopAbs_SHELL);shell.More();shell.Next()) {
                const auto shell_faces=map_of(shell.Current(),TopAbs_FACE);
                // Tolerância linear em mm, independente da tesselação de exibição.
                BRepBuilderAPI_Sewing sewing(1e-7);
                std::vector<PaintPiece> selected;
                for (const auto& piece:pieces) {
                    if (!shell_faces.Contains(original_faces(piece.source+1))) continue;
                    sewing.Add(piece.face); selected.push_back(piece);
                }
                sewing.Perform();
                const auto sewn=sewing.SewedShape();
                if (sewn.IsNull() || (sewn.ShapeType()!=TopAbs_SHELL && sewn.ShapeType()!=TopAbs_FACE) || sewing.NbFreeEdges()!=0)
                    throw std::runtime_error("A subdivisão da pintura não formou uma casca fechada para STEP.");
                // Uma casca periódica de face única pode ser devolvida como Face
                // pelo Sewing. Recriamos seu invólucro e validamos o sólido abaixo.
                if (sewn.ShapeType()==TopAbs_FACE) {
                    TopoDS_Shell single; builder.MakeShell(single); builder.Add(single,sewn);
                    closed.Add(single);
                } else closed.Add(TopoDS::Shell(sewn));
                for (const auto& piece:selected) {
                    const auto modified=sewing.IsModified(piece.face) ? sewing.Modified(piece.face) : piece.face;
                    for (TopExp_Explorer f(modified,TopAbs_FACE);f.More();f.Next())
                        sewn_pieces.push_back({TopoDS::Face(f.Current()),piece.source,piece.material});
                }
            }
            if (!closed.IsDone() || !BRepCheck_Analyzer(closed.Solid()).IsValid())
                throw std::runtime_error("Sólido pintado inválido para exportação STEP.");
            GProp_GProps before,after;
            BRepGProp::VolumeProperties(solid.Current(),before);
            BRepGProp::VolumeProperties(closed.Solid(),after);
            // Volume em mm³: limite absoluto para peças pequenas e relativo para
            // modelos grandes, rejeitando qualquer alteração material do sólido.
            if (std::abs(before.Mass()-after.Mass())>std::max(1e-7,std::abs(before.Mass())*1e-9))
                throw std::runtime_error("A subdivisão da pintura alterou o volume do corpo.");
            builder.Add(compound,closed.Solid()); ++solids;
        }
        if (!solids) throw std::runtime_error("A exportação de pintura requer um sólido fechado.");
        output.value=compound;
        paint_origins(output,sewn_pieces);
    });
}
