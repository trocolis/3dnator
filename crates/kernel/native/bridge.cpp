// Ponte original do 3dnator. Somente esta camada conhece os tipos do OCCT.
//
// Convenções do ABI:
// - Toda função é noexcept; exceções do OCCT/C++ viram `false`/nullptr e uma mensagem em
//   `nator_last_error`, válida até a próxima chamada na mesma thread.
// - `Shape*` é propriedade exclusiva do chamador e liberado com `nator_release`.
// - Índices de faces e arestas são base 0 e seguem a ordem de `TopExp::MapShapes`, que é
//   determinística para a mesma estrutura topológica e preservada na serialização B-rep.
// - Cada operação registra a origem das faces do resultado (`origins`), em trios
//   (tipo, entrada, índice). O lado Rust transforma essas origens em nomes estáveis.
#include <BRepAlgoAPI_BooleanOperation.hxx>
#include <BRepAlgoAPI_Common.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeShape.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepFilletAPI_MakeChamfer.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepGProp.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRepOffsetAPI_MakeThickSolid.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <BRepTools.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <GC_MakeArcOfCircle.hxx>
#include <GProp_GProps.hxx>
#include <Geom_TrimmedCurve.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <Message.hxx>
#include <Message_Messenger.hxx>
#include <Message_PrinterOStream.hxx>
#include <Poly_Triangulation.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <Standard_Failure.hxx>
#include <Standard_Version.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopLoc_Location.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopTools_ListOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Wire.hxx>
#include <gp_Ax1.hxx>
#include <gp_Ax2.hxx>
#include <gp_Ax3.hxx>
#include <gp_Circ.hxx>
#include <gp_Dir.hxx>
#include <gp_Pln.hxx>
#include <gp_Pnt.hxx>
#include <gp_Trsf.hxx>
#include <gp_Vec.hxx>

#include <algorithm>
#include <cstdint>
#include <exception>
#include <initializer_list>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
thread_local std::string last_error;
constexpr std::size_t max_triangles = 2'000'000;
constexpr std::size_t max_brep_bytes = 64 * 1024 * 1024;

// O OCCT imprime estatísticas (por exemplo, da escrita STEP) no stdout por padrão. A CLI do
// 3dnator usa o stdout para relatórios JSON, então a impressora padrão é removida uma vez.
// Falhas continuam chegando ao Rust pelas exceções e pelos estados IsDone das operações.
[[maybe_unused]] const bool quiet_messenger = [] {
    Message::DefaultMessenger()->RemovePrinters(STANDARD_TYPE(Message_PrinterOStream));
    return true;
}();

// Tipos de origem de uma face do resultado. Os valores fazem parte do ABI com native.rs.
enum Origin : std::int32_t {
    Unknown = 0,   // Sem origem conhecida: a face fica sem nome e não pode ser referenciada.
    Same = 1,      // A mesma face de uma entrada, sem alteração.
    Modified = 2,  // Face de uma entrada recortada/modificada pela operação.
    FromEdge = 3,  // Face nova gerada a partir de uma aresta de entrada (arredondamento etc.).
    Side = 4,      // Face lateral gerada pelo segmento `índice` do perfil.
    StartCap = 5,  // Tampa inicial da extrusão/revolução.
    EndCap = 6,    // Tampa final da extrusão/revolução.
};

struct Shape {
    TopoDS_Shape value;
    // Arestas do perfil na ordem dos registros recebidos de Rust (somente perfis).
    std::vector<TopoDS_Edge> profile_edges;
    // Trios (tipo, entrada, índice) por face do resultado, na ordem de MapShapes.
    std::vector<std::int32_t> origins;
    std::vector<double> mesh;
    // Face de origem de cada triângulo de `mesh`, para seleção gráfica.
    std::vector<std::int32_t> mesh_faces;
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

// Executa uma construção que preenche `Shape` e valida o resultado como B-rep.
template <typename Function>
Shape* build(Function function) noexcept {
    std::unique_ptr<Shape> result = std::make_unique<Shape>();
    const bool success = guarded([&] {
        function(*result);
        if (result->value.IsNull() || !BRepCheck_Analyzer(result->value).IsValid()) {
            throw std::runtime_error("O resultado não é uma geometria B-rep válida.");
        }
    });
    return success ? result.release() : nullptr;
}

// Forma simples, sem histórico: as faces ficam com origem desconhecida.
template <typename Function>
Shape* make_shape(Function function) noexcept {
    return build([&](Shape& shape) { shape.value = function(); });
}

TopTools_IndexedMapOfShape map_of(const TopoDS_Shape& shape, TopAbs_ShapeEnum kind) {
    TopTools_IndexedMapOfShape map;
    TopExp::MapShapes(shape, kind, map);
    return map;
}

TopoDS_Edge edge_at(const Shape* shape, std::size_t index) {
    const auto edges = map_of(shape->value, TopAbs_EDGE);
    if (index >= static_cast<std::size_t>(edges.Extent())) {
        throw std::runtime_error("Aresta inexistente no resultado atual.");
    }
    return TopoDS::Edge(edges(static_cast<int>(index) + 1));
}

// Acumula origens sem sobrescrever a primeira classificação encontrada para cada face.
class OriginTable {
public:
    explicit OriginTable(const TopoDS_Shape& result)
        : faces_(map_of(result, TopAbs_FACE)),
          values_(static_cast<std::size_t>(faces_.Extent()) * 3, 0) {}

    void mark(const TopoDS_Shape& face, Origin kind, int input, int index) {
        if (face.ShapeType() != TopAbs_FACE) {
            return;
        }
        const int position = faces_.FindIndex(face);
        if (position == 0) {
            return;
        }
        auto* slot = &values_[static_cast<std::size_t>(position - 1) * 3];
        if (slot[0] == Unknown) {
            slot[0] = kind;
            slot[1] = input;
            slot[2] = index;
        }
    }

    void mark_all(const TopoDS_Shape& shape, Origin kind, int input, int index) {
        for (TopExp_Explorer explorer(shape, TopAbs_FACE); explorer.More(); explorer.Next()) {
            mark(explorer.Current(), kind, input, index);
        }
    }

    std::vector<std::int32_t> take() { return std::move(values_); }

private:
    TopTools_IndexedMapOfShape faces_;
    std::vector<std::int32_t> values_;
};

// Histórico genérico de uma operação OCCT sobre até duas entradas.
// Ordem de classificação: faces preservadas, faces modificadas e por fim faces geradas por
// arestas. Assim uma face recortada nunca é confundida com uma face nova da operação.
std::vector<std::int32_t> history(BRepBuilderAPI_MakeShape& operation, const TopoDS_Shape& result,
                                  std::initializer_list<const TopoDS_Shape*> inputs) {
    OriginTable table(result);
    int input = 0;
    for (const TopoDS_Shape* source : inputs) {
        const auto faces = map_of(*source, TopAbs_FACE);
        for (int i = 1; i <= faces.Extent(); ++i) {
            table.mark(faces(i), Same, input, i - 1);
        }
        for (int i = 1; i <= faces.Extent(); ++i) {
            if (operation.IsDeleted(faces(i))) {
                continue;
            }
            for (const auto& modified : operation.Modified(faces(i))) {
                table.mark_all(modified, Modified, input, i - 1);
            }
        }
        ++input;
    }
    input = 0;
    for (const TopoDS_Shape* source : inputs) {
        const auto edges = map_of(*source, TopAbs_EDGE);
        for (int i = 1; i <= edges.Extent(); ++i) {
            for (const auto& generated : operation.Generated(edges(i))) {
                table.mark_all(generated, FromEdge, input, i - 1);
            }
        }
        ++input;
    }
    return table.take();
}

// Varredura de um perfil: laterais nomeadas pelos segmentos do perfil e tampas pelas
// formas inicial e final. Funciona para prismas e revoluções.
template <typename Sweep>
std::vector<std::int32_t> sweep_history(Sweep& sweep, const Shape* profile) {
    const TopoDS_Shape result = sweep.Shape();
    OriginTable table(result);
    for (std::size_t i = 0; i < profile->profile_edges.size(); ++i) {
        for (const auto& generated : sweep.Generated(profile->profile_edges[i])) {
            table.mark_all(generated, Side, 0, static_cast<int>(i));
        }
    }
    table.mark_all(sweep.FirstShape(), StartCap, 0, 0);
    table.mark_all(sweep.LastShape(), EndCap, 0, 0);
    return table.take();
}

void require_profile(const Shape* face) {
    if (face->profile_edges.empty() || face->value.ShapeType() != TopAbs_FACE) {
        throw std::runtime_error("A varredura exige um perfil plano construído pela ponte.");
    }
}
}  // namespace

extern "C" {
const char* nator_last_error() noexcept { return last_error.c_str(); }
const char* nator_version() noexcept { return OCC_VERSION_COMPLETE; }
void nator_release(Shape* shape) noexcept { delete shape; }

Shape* nator_box(double x, double y, double z, double dx, double dy, double dz) noexcept {
    return make_shape([&] { return BRepPrimAPI_MakeBox(gp_Pnt(x, y, z), dx, dy, dz).Shape(); });
}

Shape* nator_cylinder(double x, double y, double z, double radius, double height) noexcept {
    return make_shape([&] {
        return BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(x, y, z), gp_Dir(0, 0, 1)), radius, height)
            .Shape();
    });
}

// Corte não destrutivo usado pelos ensaios de validação.
Shape* nator_cut(const Shape* body, const Shape* tool) noexcept {
    return build([&](Shape& shape) {
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
        if (!operation.IsDone()) {
            throw std::runtime_error("Não foi possível concluir o corte.");
        }
        shape.value = operation.Shape();
        shape.origins = history(operation, shape.value, {&body->value, &tool->value});
    });
}

Shape* nator_boolean(const Shape* a, const Shape* b, int mode) noexcept {
    return build([&](Shape& shape) {
        std::unique_ptr<BRepAlgoAPI_BooleanOperation> operation;
        if (mode == 0) {
            operation = std::make_unique<BRepAlgoAPI_Fuse>();
        } else if (mode == 1) {
            operation = std::make_unique<BRepAlgoAPI_Cut>();
        } else if (mode == 2) {
            operation = std::make_unique<BRepAlgoAPI_Common>();
        } else {
            throw std::runtime_error("Operação booleana inválida.");
        }
        TopTools_ListOfShape arguments;
        TopTools_ListOfShape tools;
        arguments.Append(a->value);
        tools.Append(b->value);
        operation->SetArguments(arguments);
        operation->SetTools(tools);
        operation->SetNonDestructive(true);
        operation->Build();
        if (!operation->IsDone()) {
            throw std::runtime_error("Falha na operação booleana.");
        }
        shape.value = operation->Shape();
        shape.origins = history(*operation, shape.value, {&a->value, &b->value});
    });
}

Shape* nator_fillet(const Shape* body, std::size_t edge, double radius) noexcept {
    return build([&](Shape& shape) {
        BRepFilletAPI_MakeFillet operation(body->value);
        operation.Add(radius, edge_at(body, edge));
        operation.Build();
        if (!operation.IsDone()) {
            throw std::runtime_error("Não foi possível arredondar a aresta.");
        }
        shape.value = operation.Shape();
        shape.origins = history(operation, shape.value, {&body->value});
    });
}

Shape* nator_chamfer(const Shape* body, std::size_t edge, double distance) noexcept {
    return build([&](Shape& shape) {
        BRepFilletAPI_MakeChamfer operation(body->value);
        operation.Add(distance, edge_at(body, edge));
        operation.Build();
        if (!operation.IsDone()) {
            throw std::runtime_error("Não foi possível chanfrar a aresta.");
        }
        shape.value = operation.Shape();
        shape.origins = history(operation, shape.value, {&body->value});
    });
}

// Casca para dentro: a face indicada é removida e as paredes têm a espessura pedida.
Shape* nator_shell(const Shape* body, std::size_t face, double thickness) noexcept {
    return build([&](Shape& shape) {
        const auto faces = map_of(body->value, TopAbs_FACE);
        if (face >= static_cast<std::size_t>(faces.Extent())) {
            throw std::runtime_error("Face inexistente.");
        }
        TopTools_ListOfShape removed;
        removed.Append(faces(static_cast<int>(face) + 1));
        BRepOffsetAPI_MakeThickSolid operation;
        operation.MakeThickSolidByJoin(body->value, removed, -thickness, 1e-6);
        if (!operation.IsDone()) {
            throw std::runtime_error("Falha ao criar casca.");
        }
        shape.value = operation.Shape();
        shape.origins = history(operation, shape.value, {&body->value});
    });
}

// Matriz 3×4 por linhas (rotação | translação), em milímetros.
Shape* nator_transform(const Shape* body, const double* m) noexcept {
    return build([&](Shape& shape) {
        gp_Trsf transformation;
        transformation.SetValues(m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9], m[10],
                                 m[11]);
        BRepBuilderAPI_Transform operation(body->value, transformation, true);
        shape.value = operation.Shape();
        shape.origins = history(operation, shape.value, {&body->value});
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
        if (writer.Transfer(body->value, STEPControl_AsIs) != IFSelect_RetDone ||
            writer.Write(path) != IFSelect_RetDone) {
            throw std::runtime_error("Não foi possível exportar a geometria STEP.");
        }
    });
}

bool nator_tessellate(Shape* body, double linear, double angular) noexcept {
    return guarded([&] {
        BRepMesh_IncrementalMesh mesher(body->value, linear, false, angular, false);
        if (!mesher.IsDone()) {
            throw std::runtime_error("Falha na tesselação do sólido.");
        }
        // Construir em memória temporária preserva a malha anterior em caso de falha.
        std::vector<double> positions;
        std::vector<std::int32_t> faces_of_triangles;
        const auto faces = map_of(body->value, TopAbs_FACE);
        for (int index = 1; index <= faces.Extent(); ++index) {
            const auto face = TopoDS::Face(faces(index));
            TopLoc_Location location;
            const auto mesh = BRep_Tool::Triangulation(face, location);
            if (mesh.IsNull()) {
                throw std::runtime_error("Uma face não possui triangulação.");
            }
            for (int i = 1; i <= mesh->NbTriangles(); ++i) {
                if (positions.size() / 9 >= max_triangles) {
                    throw std::runtime_error("Tesselação excedeu dois milhões de triângulos.");
                }
                int a, b, c;
                mesh->Triangle(i).Get(a, b, c);
                if (face.Orientation() == TopAbs_REVERSED) {
                    std::swap(b, c);
                }
                for (const int node : {a, b, c}) {
                    const auto point = mesh->Node(node).Transformed(location.Transformation());
                    positions.insert(positions.end(), {point.X(), point.Y(), point.Z()});
                }
                faces_of_triangles.push_back(index - 1);
            }
        }
        if (positions.empty()) {
            throw std::runtime_error("Tesselação vazia.");
        }
        body->mesh.swap(positions);
        body->mesh_faces.swap(faces_of_triangles);
    });
}

std::size_t nator_mesh_size(const Shape* body) noexcept { return body->mesh.size(); }

bool nator_mesh_copy(const Shape* body, double* output, std::size_t length) noexcept {
    return guarded([&] {
        if (length != body->mesh.size()) {
            throw std::runtime_error("Tamanho do destino da malha incorreto.");
        }
        std::copy(body->mesh.begin(), body->mesh.end(), output);
    });
}

bool nator_mesh_faces_copy(const Shape* body, std::int32_t* output, std::size_t length) noexcept {
    return guarded([&] {
        if (length != body->mesh_faces.size()) {
            throw std::runtime_error("Tamanho do destino das faces da malha incorreto.");
        }
        std::copy(body->mesh_faces.begin(), body->mesh_faces.end(), output);
    });
}

// Perfis preservam linhas/arcos/círculos exatos; amostragem é usada somente na apresentação.
// Cada registro tem 11 doubles: tipo (0 linha, 1 arco por três pontos, 2 círculo), três
// pontos/direções e raio. `ends[w]` é o registro final (exclusivo) do contorno `w`.
// `plane` traz origem, normal e eixo U do plano de apoio.
Shape* nator_profile(const double* records, std::size_t count, const std::size_t* ends,
                     std::size_t wires, const double* plane) noexcept {
    return build([&](Shape& shape) {
        if (!wires || !count || count > 1024) {
            throw std::runtime_error("Perfil vazio ou excessivo.");
        }
        std::vector<TopoDS_Wire> contours;
        std::size_t start = 0;
        for (std::size_t w = 0; w < wires; ++w) {
            if (ends[w] <= start || ends[w] > count) {
                throw std::runtime_error("Contorno inválido.");
            }
            BRepBuilderAPI_MakeWire wire;
            for (auto i = start; i < ends[w]; ++i) {
                const double* r = records + i * 11;
                const gp_Pnt a(r[1], r[2], r[3]), b(r[4], r[5], r[6]), c(r[7], r[8], r[9]);
                if (r[0] == 0) {
                    wire.Add(BRepBuilderAPI_MakeEdge(a, b).Edge());
                } else if (r[0] == 1) {
                    wire.Add(BRepBuilderAPI_MakeEdge(GC_MakeArcOfCircle(a, b, c).Value()).Edge());
                } else if (r[0] == 2) {
                    const gp_Ax2 axis(a, gp_Dir(r[4], r[5], r[6]), gp_Dir(r[7], r[8], r[9]));
                    wire.Add(BRepBuilderAPI_MakeEdge(gp_Circ(axis, r[10])).Edge());
                } else {
                    throw std::runtime_error("Tipo de curva inválido.");
                }
                if (!wire.IsDone()) {
                    throw std::runtime_error("Não foi possível construir o contorno.");
                }
                // A aresta pode ser copiada ao compartilhar vértices; guardar a que ficou no
                // contorno mantém a correspondência registro → aresta para o histórico.
                shape.profile_edges.push_back(wire.Edge());
            }
            contours.push_back(wire.Wire());
            start = ends[w];
        }
        const gp_Pln support(gp_Ax3(gp_Pnt(plane[0], plane[1], plane[2]),
                                    gp_Dir(plane[3], plane[4], plane[5]),
                                    gp_Dir(plane[6], plane[7], plane[8])));
        BRepBuilderAPI_MakeFace face(support, contours[0], true);
        for (std::size_t i = 1; i < contours.size(); ++i) {
            face.Add(contours[i]);
        }
        if (!face.IsDone()) {
            throw std::runtime_error("Perfil não forma uma face válida.");
        }
        shape.value = face.Shape();
    });
}

Shape* nator_extrude(const Shape* face, const double* direction) noexcept {
    return build([&](Shape& shape) {
        require_profile(face);
        // Sem cópia: o histórico Generated() é consultado com as arestas originais do perfil.
        BRepPrimAPI_MakePrism prism(face->value, gp_Vec(direction[0], direction[1], direction[2]),
                                    false);
        if (!prism.IsDone()) {
            throw std::runtime_error("Falha na extrusão do perfil.");
        }
        shape.value = prism.Shape();
        shape.origins = sweep_history(prism, face);
    });
}

Shape* nator_revolve(const Shape* face, const double* origin, const double* direction,
                     double angle) noexcept {
    return build([&](Shape& shape) {
        require_profile(face);
        const gp_Ax1 axis(gp_Pnt(origin[0], origin[1], origin[2]),
                          gp_Dir(direction[0], direction[1], direction[2]));
        BRepPrimAPI_MakeRevol revolution(face->value, axis, angle, false);
        if (!revolution.IsDone()) {
            throw std::runtime_error("Falha na revolução do perfil.");
        }
        shape.value = revolution.Shape();
        shape.origins = sweep_history(revolution, face);
    });
}

std::size_t nator_face_count(const Shape* body) noexcept {
    std::size_t count = 0;
    guarded([&] { count = static_cast<std::size_t>(map_of(body->value, TopAbs_FACE).Extent()); });
    return count;
}

std::size_t nator_edge_count(const Shape* body) noexcept {
    std::size_t count = 0;
    guarded([&] { count = static_cast<std::size_t>(map_of(body->value, TopAbs_EDGE).Extent()); });
    return count;
}

// Para cada aresta, as duas faces adjacentes (índices base 0). Uma aresta de costura de
// superfície fechada repete a mesma face; aresta livre usa -1 no segundo índice.
bool nator_edge_faces(const Shape* body, std::int32_t* output, std::size_t length) noexcept {
    return guarded([&] {
        const auto edges = map_of(body->value, TopAbs_EDGE);
        const auto faces = map_of(body->value, TopAbs_FACE);
        if (length != static_cast<std::size_t>(edges.Extent()) * 2) {
            throw std::runtime_error("Destino da adjacência de arestas inválido.");
        }
        TopTools_IndexedDataMapOfShapeListOfShape ancestors;
        TopExp::MapShapesAndAncestors(body->value, TopAbs_EDGE, TopAbs_FACE, ancestors);
        for (int i = 1; i <= edges.Extent(); ++i) {
            std::vector<std::int32_t> adjacent;
            if (const auto* list = ancestors.Seek(edges(i))) {
                for (const auto& face : *list) {
                    const std::int32_t index = faces.FindIndex(face) - 1;
                    if (index >= 0 &&
                        std::find(adjacent.begin(), adjacent.end(), index) == adjacent.end()) {
                        adjacent.push_back(index);
                    }
                }
            }
            if (adjacent.size() > 2) {
                throw std::runtime_error("Aresta não manifold no sólido.");
            }
            std::int32_t first = -1;
            std::int32_t second = -1;
            if (!adjacent.empty()) {
                first = adjacent[0];
            }
            if (adjacent.size() == 2) {
                second = adjacent[1];
            } else if (adjacent.size() == 1 &&
                       BRep_Tool::IsClosed(TopoDS::Edge(edges(i)), TopoDS::Face(faces(first + 1)))) {
                // Costura de cilindro/esfera: a aresta liga a face a ela mesma.
                second = first;
            }
            const std::size_t slot = static_cast<std::size_t>(i - 1) * 2;
            output[slot] = first;
            output[slot + 1] = second;
        }
    });
}

std::size_t nator_origins_size(const Shape* body) noexcept { return body->origins.size(); }

bool nator_origins_copy(const Shape* body, std::int32_t* output, std::size_t length) noexcept {
    return guarded([&] {
        if (length != body->origins.size()) {
            throw std::runtime_error("Destino das origens de faces inválido.");
        }
        std::copy(body->origins.begin(), body->origins.end(), output);
    });
}

bool nator_brep_prepare(Shape* body) noexcept {
    return guarded([&] {
        std::ostringstream stream;
        BRepTools::Write(body->value, stream, false, false, TopTools_FormatVersion_CURRENT);
        body->brep = stream.str();
        if (body->brep.size() > max_brep_bytes) {
            throw std::runtime_error("B-rep excede 64 MiB.");
        }
    });
}

std::size_t nator_brep_size(const Shape* body) noexcept { return body->brep.size(); }

bool nator_brep_copy(const Shape* body, char* target, std::size_t size) noexcept {
    return guarded([&] {
        if (size != body->brep.size()) {
            throw std::runtime_error("Destino B-rep inválido.");
        }
        std::copy(body->brep.begin(), body->brep.end(), target);
    });
}

Shape* nator_brep_read(const char* data, std::size_t size) noexcept {
    return make_shape([&] {
        if (!size || size > max_brep_bytes) {
            throw std::runtime_error("B-rep vazio ou excessivo.");
        }
        std::istringstream stream(std::string(data, size));
        TopoDS_Shape shape;
        BRep_Builder builder;
        BRepTools::Read(shape, stream, builder);
        if (stream.fail()) {
            throw std::runtime_error("B-rep truncado ou inválido.");
        }
        return shape;
    });
}
}
