#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <BRepAlgoAPI_BooleanOperation.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepAlgoAPI_Common.hxx>
#include <BRepOffsetAPI_MakeThickSolid.hxx>
#include <BRepTools.hxx>
#include <BRep_Builder.hxx>
#include <GC_MakeArcOfCircle.hxx>
#include <Geom_TrimmedCurve.hxx>
#include <gp_Circ.hxx>
#include <gp_Pln.hxx>
#include <gp_Ax1.hxx>
#include <gp_Ax3.hxx>
#include <gp_Trsf.hxx>
#include <gp_Vec.hxx>
#include <TopoDS_Wire.hxx>
#include <sstream>
// Ponte original do 3dnator. Somente esta camada conhece os tipos do OCCT.
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepFilletAPI_MakeChamfer.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepGProp.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRep_Tool.hxx>
#include <GProp_GProps.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <Poly_Triangulation.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <Standard_Failure.hxx>
#include <Standard_Version.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopTools_ListOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <TopLoc_Location.hxx>
#include <gp_Ax2.hxx>
#include <gp_Dir.hxx>
#include <gp_Pnt.hxx>
#include <algorithm>
#include <cstdint>
#include <exception>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
thread_local std::string last_error;
constexpr std::size_t max_triangles = 2'000'000;

struct Shape {
    TopoDS_Shape value;
    std::vector<double> mesh;
    std::string brep;
};

// Todas as exceções ficam do lado C++; nenhuma atravessa o ABI C/Rust.
template <typename Function>
bool guarded(Function function) noexcept {
    try {
        last_error.clear();
        function();
        return true;
    } catch (const Standard_Failure& error) {
        last_error = error.GetMessageString() ? error.GetMessageString() : "Falha interna do OCCT.";
    } catch (const std::exception& error) {
        last_error = error.what();
    } catch (...) {
        last_error = "Falha nativa desconhecida.";
    }
    return false;
}

template <typename Function>
Shape* make_shape(Function function) noexcept {
    std::unique_ptr<Shape> result;
    const bool success = guarded([&] {
        auto value = function();
        if (value.IsNull() || !BRepCheck_Analyzer(value).IsValid()) {
            throw std::runtime_error("O resultado não é uma geometria B-rep válida.");
        }
        result = std::make_unique<Shape>();
        result->value = value;
    });
    return success ? result.release() : nullptr;
}

TopoDS_Edge edge_at(const Shape* shape, std::size_t index) {
    TopTools_IndexedMapOfShape edges;
    TopExp::MapShapes(shape->value, TopAbs_EDGE, edges);
    if (index >= static_cast<std::size_t>(edges.Extent())) {
        throw std::runtime_error("Aresta inexistente no resultado atual.");
    }
    return TopoDS::Edge(edges(static_cast<int>(index) + 1));
}
}

extern "C" {
const char* nator_last_error() noexcept { return last_error.c_str(); }
const char* nator_version() noexcept { return OCC_VERSION_COMPLETE; }
void nator_release(Shape* shape) noexcept { delete shape; }

Shape* nator_box(double x, double y, double z, double dx, double dy, double dz) noexcept {
    return make_shape([&] { return BRepPrimAPI_MakeBox(gp_Pnt(x,y,z), dx,dy,dz).Shape(); });
}

Shape* nator_cylinder(double x, double y, double z, double radius, double height) noexcept {
    return make_shape([&] {
        return BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(x,y,z), gp_Dir(0,0,1)), radius, height).Shape();
    });
}

Shape* nator_cut(const Shape* body, const Shape* tool) noexcept {
    return make_shape([&] {
        // Configurar antes de Build evita que o construtor execute um corte destrutivo.
        BRepAlgoAPI_Cut operation;
        TopTools_ListOfShape arguments;
        TopTools_ListOfShape tools;
        arguments.Append(body->value);
        tools.Append(tool->value);
        operation.SetArguments(arguments);
        operation.SetTools(tools);
        operation.SetNonDestructive(true);
        operation.Build();
        if (!operation.IsDone()) { throw std::runtime_error("Não foi possível concluir o corte."); }
        return operation.Shape();
    });
}

Shape* nator_fillet(const Shape* body, std::size_t edge, double radius) noexcept {
    return make_shape([&] {
        BRepFilletAPI_MakeFillet operation(body->value);
        operation.Add(radius, edge_at(body, edge));
        operation.Build();
        if (!operation.IsDone()) { throw std::runtime_error("Não foi possível arredondar a aresta."); }
        return operation.Shape();
    });
}

Shape* nator_chamfer(const Shape* body, std::size_t edge, double distance) noexcept {
    return make_shape([&] {
        BRepFilletAPI_MakeChamfer operation(body->value);
        operation.Add(distance, edge_at(body, edge));
        operation.Build();
        if (!operation.IsDone()) { throw std::runtime_error("Não foi possível chanfrar a aresta."); }
        return operation.Shape();
    });
}

bool nator_volume(const Shape* body, double* volume) noexcept {
    return guarded([&] {
        GProp_GProps properties;
        BRepGProp::VolumeProperties(body->value, properties);
        *volume = properties.Mass();
    });
}

Shape* nator_read_step(const char* path) noexcept {
    return make_shape([&] {
        STEPControl_Reader reader;
        if (reader.ReadFile(path) != IFSelect_RetDone || reader.TransferRoots() <= 0) {
            throw std::runtime_error("Não foi possível importar a geometria STEP.");
        }
        return reader.OneShape();
    });
}

bool nator_write_step(const Shape* body, const char* path) noexcept {
    return guarded([&] {
        STEPControl_Writer writer;
        if (writer.Transfer(body->value, STEPControl_AsIs) != IFSelect_RetDone || writer.Write(path) != IFSelect_RetDone) {
            throw std::runtime_error("Não foi possível exportar a geometria STEP.");
        }
    });
}

bool nator_tessellate(Shape* body, double linear, double angular) noexcept {
    return guarded([&] {
        BRepMesh_IncrementalMesh mesher(body->value, linear, false, angular, false);
        if (!mesher.IsDone()) { throw std::runtime_error("Falha na tesselação do sólido."); }
        // Construir em memória temporária preserva a malha anterior em caso de falha.
        std::vector<double> positions;
        for (TopExp_Explorer explorer(body->value, TopAbs_FACE); explorer.More(); explorer.Next()) {
            const auto face = TopoDS::Face(explorer.Current());
            TopLoc_Location location;
            const auto mesh = BRep_Tool::Triangulation(face, location);
            if (mesh.IsNull()) { throw std::runtime_error("Uma face não possui triangulação."); }
            for (int i = 1; i <= mesh->NbTriangles(); ++i) {
                if (positions.size() / 9 >= max_triangles) { throw std::runtime_error("Tesselação excedeu dois milhões de triângulos."); }
                int a, b, c;
                mesh->Triangle(i).Get(a,b,c);
                if (face.Orientation() == TopAbs_REVERSED) { std::swap(b,c); }
                for (const int node : {a,b,c}) {
                    const auto point = mesh->Node(node).Transformed(location.Transformation());
                    positions.insert(positions.end(), {point.X(),point.Y(),point.Z()});
                }
            }
        }
        if (positions.empty()) { throw std::runtime_error("Tesselação vazia."); }
        body->mesh.swap(positions);
    });
}

std::size_t nator_mesh_size(const Shape* body) noexcept { return body->mesh.size(); }
bool nator_mesh_copy(const Shape* body, double* output, std::size_t length) noexcept {
    return guarded([&] {
        if (length != body->mesh.size()) { throw std::runtime_error("Tamanho do destino da malha incorreto."); }
        std::copy(body->mesh.begin(), body->mesh.end(), output);
    });
}// Perfis preservam linhas/arcos/círculos exatos; amostragem é usada somente na apresentação.
Shape* nator_profile(const double* records, std::size_t count, const std::size_t* ends, std::size_t wires,
                     const double* plane) noexcept {
    return make_shape([&] {
        if (!wires || count > 1024) { throw std::runtime_error("Perfil vazio ou excessivo."); }
        std::vector<TopoDS_Wire> contours;
        std::size_t start = 0;
        for (std::size_t w=0;w<wires;++w) {
            if (ends[w]<=start || ends[w]>count) { throw std::runtime_error("Contorno inválido."); }
            BRepBuilderAPI_MakeWire wire;
            for (auto i=start;i<ends[w];++i) {
                const double* r=records+i*11;
                gp_Pnt a(r[1],r[2],r[3]), b(r[4],r[5],r[6]), c(r[7],r[8],r[9]);
                if (r[0]==0) {wire.Add(BRepBuilderAPI_MakeEdge(a,b).Edge());}
                else if (r[0]==1) {wire.Add(BRepBuilderAPI_MakeEdge(GC_MakeArcOfCircle(a,b,c).Value()).Edge());}
                else if (r[0]==2) {wire.Add(BRepBuilderAPI_MakeEdge(gp_Circ(gp_Ax2(a,gp_Dir(r[4],r[5],r[6]),gp_Dir(r[7],r[8],r[9])),r[10])).Edge());}
                else {throw std::runtime_error("Tipo de curva inválido.");}
            }
            if (!wire.IsDone()) {throw std::runtime_error("Não foi possível construir o contorno.");}
            contours.push_back(wire.Wire());start=ends[w];
        }
        gp_Pln support(gp_Ax3(gp_Pnt(plane[0],plane[1],plane[2]),gp_Dir(plane[3],plane[4],plane[5]),gp_Dir(plane[6],plane[7],plane[8])));
        BRepBuilderAPI_MakeFace face(support,contours[0],true);
        for (std::size_t i=1;i<contours.size();++i) {face.Add(contours[i]);}
        if (!face.IsDone()) {throw std::runtime_error("Perfil não forma uma face válida.");}
        return face.Shape();
    });
}
Shape* nator_extrude(const Shape* face,const double* direction) noexcept {
    return make_shape([&]{return BRepPrimAPI_MakePrism(face->value,gp_Vec(direction[0],direction[1],direction[2]),true).Shape();});
}
Shape* nator_revolve(const Shape* face,const double* origin,const double* direction,double angle) noexcept {
    return make_shape([&]{return BRepPrimAPI_MakeRevol(face->value,gp_Ax1(gp_Pnt(origin[0],origin[1],origin[2]),gp_Dir(direction[0],direction[1],direction[2])),angle,true).Shape();});
}
Shape* nator_boolean(const Shape* a,const Shape* b,int mode) noexcept {
    return make_shape([&] {
        TopTools_ListOfShape args,tools; args.Append(a->value); tools.Append(b->value);
        std::unique_ptr<BRepAlgoAPI_BooleanOperation> op;
        if(mode==0){op=std::make_unique<BRepAlgoAPI_Fuse>();}
        else if(mode==1){op=std::make_unique<BRepAlgoAPI_Cut>();}
        else if(mode==2){op=std::make_unique<BRepAlgoAPI_Common>();}
        else {throw std::runtime_error("Operação booleana inválida.");}
        op->SetArguments(args);op->SetTools(tools);op->SetNonDestructive(true);op->Build();
        if (!op->IsDone()) {throw std::runtime_error("Falha na operação booleana.");}return op->Shape();
    });
}
Shape* nator_transform(const Shape* body,const double* m) noexcept {
    return make_shape([&]{gp_Trsf tr;tr.SetValues(m[0],m[1],m[2],m[3],m[4],m[5],m[6],m[7],m[8],m[9],m[10],m[11]);return BRepBuilderAPI_Transform(body->value,tr,true).Shape();});
}
Shape* nator_shell(const Shape* body,std::size_t face,double thickness) noexcept {
    return make_shape([&]{
        TopTools_IndexedMapOfShape faces;TopExp::MapShapes(body->value,TopAbs_FACE,faces);
        if(face>=static_cast<std::size_t>(faces.Extent())){throw std::runtime_error("Face inexistente.");}
        TopTools_ListOfShape removed;removed.Append(faces(static_cast<int>(face)+1));
        BRepOffsetAPI_MakeThickSolid op;op.MakeThickSolidByJoin(body->value,removed,-thickness,1e-6);
        if(!op.IsDone()){throw std::runtime_error("Falha ao criar casca.");}return op.Shape();
    });
}
bool nator_brep_prepare(Shape* body) noexcept {
    return guarded([&]{std::ostringstream stream;BRepTools::Write(body->value,stream,false,false,TopTools_FormatVersion_CURRENT);body->brep=stream.str();if(body->brep.size()>64*1024*1024){throw std::runtime_error("B-rep excede 64 MiB.");}});
}
std::size_t nator_brep_size(const Shape* body) noexcept {return body->brep.size();}
bool nator_brep_copy(const Shape* body,char* target,std::size_t size) noexcept {
    return guarded([&]{if(size!=body->brep.size()){throw std::runtime_error("Destino B-rep inválido.");}std::copy(body->brep.begin(),body->brep.end(),target);});
}
Shape* nator_brep_read(const char* data,std::size_t size) noexcept {
    return make_shape([&]{if(!size || size>64*1024*1024){throw std::runtime_error("B-rep vazio ou excessivo.");}std::istringstream stream(std::string(data,size));TopoDS_Shape shape;BRep_Builder builder;BRepTools::Read(shape,stream,builder);if(stream.fail()){throw std::runtime_error("B-rep truncado ou inválido.");}return shape;});
}

}
