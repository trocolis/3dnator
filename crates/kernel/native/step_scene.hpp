// Intercâmbio XCAF: nomes, cores por sólido/face e definições compartilhadas de montagem.
// Incluído após a ponte para reutilizar Shape e a conversão centralizada de exceções.
#pragma once
#include <STEPCAFControl_Reader.hxx>
#include <STEPCAFControl_Writer.hxx>
#include <TDocStd_Document.hxx>
#include <TDataStd_Name.hxx>
#include <TDF_Tool.hxx>
#include <TDF_LabelSequence.hxx>
#include <XCAFDoc_DocumentTool.hxx>
#include <XCAFDoc_ShapeTool.hxx>
#include <XCAFDoc_ColorTool.hxx>
#include <Quantity_Color.hxx>
#include <ShapeFix_Shape.hxx>
#include <ShapeBuild_ReShape.hxx>
#include <map>
#include <set>
#include <functional>

namespace {
struct StepBody {
    std::string name;
    TopoDS_Shape shape;
    std::array<int, 3> color{-1, -1, -1};
    std::vector<std::array<int,3>> face_colors;
    // Superfícies abertas, fora de qualquer sólido: não têm volume e viram malha.
    bool surface = false;
    // Falso quando o sólido segue reprovado pelo BRepCheck mesmo após o reparo.
    bool verified = true;
};
struct StepPart { std::string name; std::vector<StepBody> bodies; };
struct StepOccurrence {
    std::size_t part;
    std::int64_t parent;
    std::string name;
    gp_Trsf placement;
    std::array<int, 3> color{-1, -1, -1};
};
struct StepScene { std::vector<StepPart> parts; std::vector<StepOccurrence> instances; };
constexpr std::size_t max_step_elements = 10000;

double solid_volume(const TopoDS_Shape& shape) {
    GProp_GProps properties;
    BRepGProp::VolumeProperties(shape, properties);
    return properties.Mass();
}
// O STEP não guarda tolerâncias por aresta. Uma face cujas curvas paramétricas só se
// cruzavam dentro da tolerância de origem (comum em B-splines vindas de outro CAD)
// pode voltar reprovada pelo BRepCheck depois que o leitor recalcula curvas e encolhe
// tolerâncias. O reparo só é aceito se deixar o sólido válido com as mesmas faces, na
// mesma ordem, e o mesmo volume; senão o sólido lido segue intacto e não verificado.
// Retorna nulo quando o reparo não serve.
TopoDS_Shape repaired_step_solid(const TopoDS_Shape& solid) {
    ShapeFix_Shape fix(solid);
    fix.Perform();
    const TopoDS_Shape result = fix.Shape();
    if (result.IsNull() || result.ShapeType() != TopAbs_SOLID || !BRepCheck_Analyzer(result).IsValid())
        return {};
    const auto before = map_of(solid, TopAbs_FACE);
    const auto after = map_of(result, TopAbs_FACE);
    if (before.Extent() != after.Extent()) return {};
    for (int f = 1; f <= before.Extent(); ++f)
        if (!fix.Context()->Value(before(f)).IsSame(after(f))) return {};
    const double original = solid_volume(solid);
    if (std::abs(solid_volume(result) - original) > 1e-6 * std::max(std::abs(original), 1.)) return {};
    return result;
}
std::string step_name(const TDF_Label& label) {
    Handle(TDataStd_Name) attr;
    if (!label.FindAttribute(TDataStd_Name::GetID(), attr)) return "Peça STEP";
    const auto& text = attr->Get();
    if (text.LengthOfCString() > 1024) throw std::runtime_error("Nome STEP excede 1024 bytes.");
    std::string result(text.LengthOfCString() + 1, '\0');
    char* buffer = result.data();
    result.resize(text.ToUTF8CString(buffer));
    return result.empty() ? "Peça STEP" : result;
}
// No OCCT 7.6 GetColor é membro; nas versões novas também aceita esta chamada.
std::array<int,3> step_color(const Handle(XCAFDoc_ColorTool)& colors, const TDF_Label& label) {
    Quantity_Color color;
    if (!colors->GetColor(label, XCAFDoc_ColorSurf, color) &&
        !colors->GetColor(label, XCAFDoc_ColorGen, color)) return {-1,-1,-1};
    double r, g, b;
    color.Values(r, g, b, Quantity_TOC_sRGB);
    return {int(std::lround(r * 255)), int(std::lround(g * 255)), int(std::lround(b * 255))};
}
std::array<int,3> shape_color(const Handle(XCAFDoc_ColorTool)& colors, const TopoDS_Shape& shape) {
    Quantity_Color color;
    if (!colors->GetColor(shape, XCAFDoc_ColorSurf, color) &&
        !colors->GetColor(shape, XCAFDoc_ColorGen, color)) return {-1,-1,-1};
    double r, g, b;
    color.Values(r, g, b, Quantity_TOC_sRGB);
    return {int(std::lround(r*255)),int(std::lround(g*255)),int(std::lround(b*255))};
}
void set_step_name(const TDF_Label& label, const std::string& name) {
    TDataStd_Name::Set(label, TCollection_ExtendedString(name.c_str(), true));
}
void set_step_color(const Handle(XCAFDoc_ColorTool)& colors, const TDF_Label& label, const std::array<int,3>& rgb) {
    if (rgb[0] >= 0) colors->SetColor(label, Quantity_Color(rgb[0]/255., rgb[1]/255., rgb[2]/255., Quantity_TOC_sRGB), XCAFDoc_ColorSurf);
}
}

extern "C" {
StepScene* nator_step_scene_new() noexcept {
    StepScene* result = nullptr;
    guarded([&] { result = new StepScene(); });
    return result;
}
void nator_step_scene_release(StepScene* scene) noexcept { delete scene; }
StepScene* nator_step_scene_read(const char* path) noexcept {
    auto scene = std::make_unique<StepScene>();
    const bool ok = guarded([&] {
        Handle(TDocStd_Document) doc = new TDocStd_Document("BinXCAF");
        STEPCAFControl_Reader reader;
        if (reader.ReadFile(path) != IFSelect_RetDone || !reader.Transfer(doc))
            throw std::runtime_error("Não foi possível importar a montagem STEP.");
        if (!reader.ExternFiles().IsEmpty())
            throw std::runtime_error("STEP com referências externas não é autocontido; reúna a montagem em um arquivo.");
        auto shapes = XCAFDoc_DocumentTool::ShapeTool(doc->Main());
        auto colors = XCAFDoc_DocumentTool::ColorTool(doc->Main());
        std::map<std::string, std::size_t> definitions;
        std::set<std::string> visiting;
        std::size_t body_count = 0;
        std::function<std::size_t(const TDF_Label&, std::size_t)> definition;
        std::function<void(const TDF_Label&, std::int64_t, std::size_t)> occurrence;
        occurrence = [&](const TDF_Label& label, std::int64_t parent, std::size_t depth) {
            if (scene->instances.size() >= max_step_elements) throw std::runtime_error("Montagem STEP excede dez mil instâncias.");
            TDF_Label target = label;
            XCAFDoc_ShapeTool::GetReferredShape(label, target);
            if (target.IsNull()) target = label;
            const auto part = definition(target, depth);
            scene->instances.push_back({part, parent, step_name(label), XCAFDoc_ShapeTool::GetLocation(label).Transformation(), step_color(colors,label)});
        };
        definition = [&](const TDF_Label& label, std::size_t depth) -> std::size_t {
            if (depth > 64) throw std::runtime_error("Hierarquia STEP excede 64 níveis.");
            TCollection_AsciiString entry;
            TDF_Tool::Entry(label, entry);
            const std::string key = entry.ToCString();
            if (visiting.count(key)) throw std::runtime_error("Montagem STEP contém um ciclo.");
            auto found = definitions.find(key);
            if (found != definitions.end()) return found->second;
            if (scene->parts.size() >= max_step_elements) throw std::runtime_error("STEP excede dez mil definições.");
            const auto index = scene->parts.size();
            definitions[key] = index;
            visiting.insert(key);
            scene->parts.push_back({step_name(label), {}});
            if (XCAFDoc_ShapeTool::IsAssembly(label)) {
                TDF_LabelSequence children;
                XCAFDoc_ShapeTool::GetComponents(label, children, false);
                for (int i = 1; i <= children.Length(); ++i) occurrence(children(i), index, depth + 1);
            } else {
                auto shape = XCAFDoc_ShapeTool::GetShape(label);
                // A posição de topo é representada pela ocorrência. Transformações de
                // sólidos internos ao composto continuam na geometria da definição.
                shape.Location(TopLoc_Location());
                std::vector<StepBody> bodies;
                for (TopExp_Explorer it(shape, TopAbs_SOLID); it.More(); it.Next()) {
                    if (++body_count > max_step_elements) throw std::runtime_error("STEP excede dez mil corpos.");
                    const auto solid = it.Current();
                    auto color = step_color(colors,label);
                    const auto specific = shape_color(colors, solid);
                    if (specific[0] >= 0) color = specific;
                    // Cores das faces lidas antes do reparo, que preserva a ordem das faces.
                    std::vector<std::array<int,3>> face_colors;
                    const auto faces=map_of(solid,TopAbs_FACE);
                    for (int f=1;f<=faces.Extent();++f) face_colors.push_back(shape_color(colors,faces(f)));
                    TopoDS_Shape accepted = solid;
                    bool verified = BRepCheck_Analyzer(solid).IsValid();
                    if (!verified) {
                        const auto repaired = repaired_step_solid(solid);
                        if (!repaired.IsNull()) {
                            accepted = repaired;
                            verified = true;
                        }
                    }
                    bodies.push_back({step_name(label), accepted, color, std::move(face_colors), false, verified});
                }
                // Cascas e faces fora de sólidos, como a serigrafia exportada pelo KiCad.
                // Juntas formam um corpo por definição, para não multiplicar itens na árvore.
                TopoDS_Compound surfaces;
                BRep_Builder builder;
                builder.MakeCompound(surfaces);
                bool open = false;
                for (TopExp_Explorer it(shape, TopAbs_SHELL, TopAbs_SOLID); it.More(); it.Next()) {
                    builder.Add(surfaces, it.Current());
                    open = true;
                }
                for (TopExp_Explorer it(shape, TopAbs_FACE, TopAbs_SHELL); it.More(); it.Next()) {
                    builder.Add(surfaces, it.Current());
                    open = true;
                }
                if (open) {
                    if (++body_count > max_step_elements) throw std::runtime_error("STEP excede dez mil corpos.");
                    auto color = step_color(colors,label);
                    std::vector<std::array<int,3>> face_colors;
                    const auto faces=map_of(surfaces,TopAbs_FACE);
                    for (int f=1;f<=faces.Extent();++f) {
                        face_colors.push_back(shape_color(colors,faces(f)));
                        if (color[0] < 0) color = face_colors.back();
                    }
                    bodies.push_back({step_name(label), surfaces, color, std::move(face_colors), true});
                }
                if (bodies.empty()) throw std::runtime_error("STEP contém uma peça sem sólidos nem superfícies.");
                scene->parts[index].bodies = std::move(bodies);
            }
            visiting.erase(key);
            return index;
        };
        TDF_LabelSequence roots;
        shapes->GetFreeShapes(roots);
        for (int i = 1; i <= roots.Length(); ++i) occurrence(roots(i), -1, 0);
        if (scene->parts.empty()) throw std::runtime_error("STEP não contém peças.");
    });
    return ok ? scene.release() : nullptr;
}
std::size_t nator_step_part_count(const StepScene* scene) noexcept { return scene->parts.size(); }
std::size_t nator_step_instance_count(const StepScene* scene) noexcept { return scene->instances.size(); }
std::size_t nator_step_body_count(const StepScene* scene, std::size_t part) noexcept { return part < scene->parts.size() ? scene->parts[part].bodies.size() : 0; }
const char* nator_step_name(const StepScene* scene, std::size_t part, std::size_t body, int kind) noexcept {
    if (kind == 0 && part < scene->parts.size()) return scene->parts[part].name.c_str();
    if (kind == 1 && part < scene->parts.size() && body < scene->parts[part].bodies.size()) return scene->parts[part].bodies[body].name.c_str();
    if (kind == 2 && part < scene->instances.size()) return scene->instances[part].name.c_str();
    return nullptr;
}
// A forma é entregue como foi aceita na leitura, sem novo BRepCheck: sólidos não
// verificados são informados por `nator_step_body_verified`.
Shape* nator_step_body(const StepScene* scene, std::size_t part, std::size_t body, int* color) noexcept {
    return adopt_shape([&] {
        const auto& b = scene->parts.at(part).bodies.at(body);
        std::copy(b.color.begin(), b.color.end(), color);
        return b.shape;
    });
}
bool nator_step_body_verified(const StepScene* scene, std::size_t part, std::size_t body) noexcept {
    bool verified = false;
    guarded([&] { verified = scene->parts.at(part).bodies.at(body).verified; });
    return verified;
}
bool nator_step_body_surface(const StepScene* scene, std::size_t part, std::size_t body) noexcept {
    bool surface = false;
    guarded([&] { surface = scene->parts.at(part).bodies.at(body).surface; });
    return surface;
}
bool nator_step_face_colors(const StepScene* scene,std::size_t part,std::size_t body,int* colors,std::size_t count) noexcept {
    return guarded([&] {
        const auto& b=scene->parts.at(part).bodies.at(body);
        if (count!=b.face_colors.size()) throw std::runtime_error("Contagem de cores STEP incompatível.");
        for (std::size_t f=0;f<count;++f) std::copy(b.face_colors[f].begin(),b.face_colors[f].end(),colors+f*3);
    });
}
bool nator_step_instance(const StepScene* scene, std::size_t index, std::size_t* part, std::int64_t* parent, double* matrix, int* color) noexcept {
    return guarded([&] {
        const auto& i = scene->instances.at(index);
        *part = i.part; *parent = i.parent;
        for (int r = 0; r < 3; ++r) for (int c = 0; c < 4; ++c) matrix[r*4+c] = i.placement.Value(r+1,c+1);
        std::copy(i.color.begin(), i.color.end(), color);
    });
}
bool nator_step_add_part(StepScene* scene, const char* name) noexcept {
    return guarded([&] { scene->parts.push_back({name,{}}); });
}
bool nator_step_add_body(StepScene* scene, std::size_t part, const Shape* body, const char* name, const int* color, const int* face_colors,std::size_t count) noexcept {
    return guarded([&] {
        const auto& faces = faces_of(body);
        if (count && count!=static_cast<std::size_t>(faces.Extent())) throw std::runtime_error("Contagem de cores STEP incompatível.");
        StepBody result{name,body->value,{color[0],color[1],color[2]}, {}, false};
        for (std::size_t f=0;f<count;++f) result.face_colors.push_back({face_colors[f*3],face_colors[f*3+1],face_colors[f*3+2]});
        scene->parts.at(part).bodies.push_back(std::move(result));
    });
}
bool nator_step_add_instance(StepScene* scene, std::size_t part, std::int64_t parent, const char* name, const double* m, const int* color) noexcept {
    return guarded([&] {
        gp_Trsf t; t.SetValues(m[0],m[1],m[2],m[3],m[4],m[5],m[6],m[7],m[8],m[9],m[10],m[11]);
        scene->instances.push_back({part,parent,name,t,{color[0],color[1],color[2]}});
    });
}
bool nator_step_scene_write(const StepScene* scene, const char* path) noexcept {
    return guarded([&] {
        Handle(TDocStd_Document) doc = new TDocStd_Document("BinXCAF");
        auto shapes = XCAFDoc_DocumentTool::ShapeTool(doc->Main());
        auto colors = XCAFDoc_DocumentTool::ColorTool(doc->Main());
        std::vector<TDF_Label> labels;
        const auto paint_body=[&](const TDF_Label& label,const StepBody& body) {
            set_step_color(colors,label,body.color);
            const auto faces=map_of(body.shape,TopAbs_FACE);
            for (std::size_t f=0;f<body.face_colors.size();++f) {
                if (body.face_colors[f][0]<0) continue;
                const auto sub=shapes->AddSubShape(label,faces(static_cast<int>(f)+1));
                if (sub.IsNull()) throw std::runtime_error("Não foi possível associar a cor à face STEP.");
                set_step_color(colors,sub,body.face_colors[f]);
            }
        };
        // Uma definição com componentes é um conjunto XCAF; corpos próprios entram
        // nele como componentes sólidos nomeados. A geometria compartilhada é gravada uma vez.
        for (std::size_t p = 0; p < scene->parts.size(); ++p) {
            const auto& part = scene->parts[p];
            const bool assembly = std::any_of(scene->instances.begin(),scene->instances.end(),[&](const StepOccurrence& i){return i.parent == std::int64_t(p);});
            TDF_Label label;
            if (!assembly && part.bodies.size() == 1) {
                label = shapes->AddShape(part.bodies[0].shape, false);
                paint_body(label,part.bodies[0]);
            } else {
                label = shapes->NewShape();
                for (const auto& body : part.bodies) {
                    const auto child = shapes->AddShape(body.shape,false);
                    set_step_name(child,body.name); paint_body(child,body);
                    shapes->AddComponent(label,child,TopLoc_Location());
                }
            }
            set_step_name(label,part.name); labels.push_back(label);
        }
        for (const auto& instance : scene->instances) {
            if (instance.parent < 0 && instance.color[0] < 0 && instance.name == scene->parts.at(instance.part).name) {
                const auto uses = std::count_if(scene->instances.begin(),scene->instances.end(),[&](const StepOccurrence& i){ return i.part == instance.part; });
                bool identity = true;
                for (int r=1;r<=3;++r) for (int c=1;c<=4;++c)
                    identity = identity && std::abs(instance.placement.Value(r,c) - (r==c ? 1. : 0.)) < 1e-12;
                // Uma raiz livre, única e sem deslocamento já é representada pela
                // própria definição. Isso evita acrescentar um nível a cada ida e volta.
                if (uses == 1 && identity) continue;
            }
            // Raízes também usam referências: sua localização não deve ser incorporada
            // à definição compartilhada usada por componentes de outros conjuntos.
            TDF_Label parent;
            if (instance.parent < 0) { parent = shapes->NewShape(); set_step_name(parent,instance.name); }
            else parent = labels.at(instance.parent);
            auto label = shapes->AddComponent(parent,labels.at(instance.part),TopLoc_Location(instance.placement));
            set_step_name(label,instance.name); set_step_color(colors,label,instance.color);
        }
        shapes->UpdateAssemblies();
        STEPCAFControl_Writer writer;
        if (!writer.Transfer(doc,STEPControl_AsIs) || writer.Write(path) != IFSelect_RetDone)
            throw std::runtime_error("Não foi possível exportar a montagem STEP.");
    });
}
}
