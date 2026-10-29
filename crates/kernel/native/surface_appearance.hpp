// Particiona apenas a representação de aparência das faces. O B-rep de origem
// permanece intacto; dentro/fora dos prismas preservam curvas e furos exatos.
#pragma once
extern "C" {
Shape* nator_surface_appearance(const Shape* body,const std::size_t* faces,const Shape* const* tools,std::size_t count) noexcept {
    return build([&](Shape& output) {
        if (count==0 || count>255) throw std::runtime_error("Quantidade de pinturas inválida.");
        const auto original=map_of(body->value,TopAbs_FACE);
        struct Piece {TopoDS_Face face; int source; int material;};
        std::vector<Piece> pieces;
        for (int i=1;i<=original.Extent();++i) pieces.push_back({TopoDS::Face(original(i)),i-1,0});
        for (std::size_t layer=0;layer<count;++layer) {
            if (faces[layer]>=static_cast<std::size_t>(original.Extent())) throw std::runtime_error("Face pintada inexistente.");
            std::vector<Piece> next;
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
        TopoDS_Compound compound;BRep_Builder builder;builder.MakeCompound(compound);
        for (const auto& piece:pieces) builder.Add(compound,piece.face);
        output.value=compound;
        const auto mapped=map_of(output.value,TopAbs_FACE);
        output.origins.assign(static_cast<std::size_t>(mapped.Extent())*3,0);
        for (const auto& piece:pieces) {
            const auto index=mapped.FindIndex(piece.face);
            if (index<1) throw std::runtime_error("Aparência perdeu a face de origem.");
            output.origins[(index-1)*3+1]=piece.source;
            output.origins[(index-1)*3+2]=piece.material;
        }
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
