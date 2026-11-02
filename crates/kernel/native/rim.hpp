// Anel plano junto a um contorno de uma face, base de ressaltos e rebaixos de tampas. O lado
// do contorno de referência usa o próprio fio da face, para acompanhar a parede exata; o lado
// deslocado chega pronto do Rust. BRepOffsetAPI_MakeOffset não serve aqui: ele dá falha de
// segmentação no eixo medial em contornos reais, sem exceção que a ponte possa capturar.
#pragma once
#include <BRepAlgoAPI_Common.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepTools.hxx>
#include <BRepTools_WireExplorer.hxx>
#include <GCPnts_TangentialDeflection.hxx>
extern "C" {
namespace {
constexpr double rim_deflection_mm = 0.005;
constexpr double rim_angle_rad = 0.2;

struct Rim {
    TopoDS_Face face;
    gp_Pln plane;
    gp_Dir normal;
    // O contorno externo vem primeiro.
    std::vector<TopoDS_Wire> wires;
};

Rim rim_of(const Shape* body, std::size_t index) {
    const auto& faces = faces_of(body);
    if (index >= static_cast<std::size_t>(faces.Extent())) {
        throw std::runtime_error("Face inexistente.");
    }
    Rim rim;
    rim.face = TopoDS::Face(faces(static_cast<int>(index) + 1));
    BRepAdaptor_Surface surface(rim.face);
    if (surface.GetType() != GeomAbs_Plane) throw std::runtime_error("O anel exige uma face plana.");
    rim.plane = surface.Plane();
    rim.normal = rim.plane.Axis().Direction();
    if (rim.face.Orientation() == TopAbs_REVERSED) rim.normal.Reverse();
    const TopoDS_Wire outer = BRepTools::OuterWire(rim.face);
    rim.wires.push_back(outer);
    for (TopExp_Explorer it(rim.face, TopAbs_WIRE); it.More(); it.Next()) {
        if (!it.Current().IsSame(outer)) rim.wires.push_back(TopoDS::Wire(it.Current()));
    }
    return rim;
}

// Registro: origem, U e V do plano, quantidade de contornos e, por contorno, a quantidade de
// pontos e suas coordenadas no plano.
std::vector<double> rim_records(const Shape* body, std::size_t index) {
    const Rim rim = rim_of(body, index);
    const gp_Ax3 frame = rim.plane.Position();
    const gp_Pnt origin = frame.Location();
    const gp_Dir u = frame.XDirection(), v = frame.YDirection();
    std::vector<double> records{origin.X(), origin.Y(), origin.Z(), u.X(), u.Y(), u.Z(),
                                v.X(),      v.Y(),      v.Z(),      static_cast<double>(rim.wires.size())};
    for (const TopoDS_Wire& wire : rim.wires) {
        std::vector<double> points;
        for (BRepTools_WireExplorer it(wire, rim.face); it.More(); it.Next()) {
            BRepAdaptor_Curve curve(it.Current());
            GCPnts_TangentialDeflection sampled(curve, rim_angle_rad, rim_deflection_mm);
            const int count = sampled.NbPoints();
            const bool reversed = it.Current().Orientation() == TopAbs_REVERSED;
            // O último ponto de cada aresta é o primeiro da seguinte.
            for (int i = 1; i < count; ++i) {
                const gp_XYZ delta = sampled.Value(reversed ? count + 1 - i : i).XYZ() - origin.XYZ();
                points.push_back(delta.Dot(u.XYZ()));
                points.push_back(delta.Dot(v.XYZ()));
            }
        }
        records.push_back(static_cast<double>(points.size() / 2));
        records.insert(records.end(), points.begin(), points.end());
    }
    return records;
}

TopoDS_Wire polygon_wire(const gp_Pln& plane, const double* points, std::size_t count) {
    const gp_Ax3 frame = plane.Position();
    const gp_XYZ origin = frame.Location().XYZ(), u = frame.XDirection().XYZ(),
                 v = frame.YDirection().XYZ();
    BRepBuilderAPI_MakePolygon polygon;
    for (std::size_t i = 0; i < count; ++i) {
        polygon.Add(gp_Pnt(origin + u * points[2 * i] + v * points[2 * i + 1]));
    }
    polygon.Close();
    if (!polygon.IsDone()) throw std::runtime_error("Contorno deslocado inválido.");
    return polygon.Wire();
}

TopoDS_Face ring_between(const gp_Pln& plane, const TopoDS_Wire& outside, const TopoDS_Wire& inside) {
    BRepBuilderAPI_MakeFace maker(plane, outside, true);
    // Uma face só do contorno interno dá o sentido certo: o furo é esse contorno invertido.
    const TopoDS_Face filled = BRepBuilderAPI_MakeFace(plane, inside, true);
    maker.Add(TopoDS::Wire(BRepTools::OuterWire(filled).Reversed()));
    return maker.Face();
}

double face_area(const TopoDS_Shape& face) {
    GProp_GProps properties;
    BRepGProp::SurfaceProperties(face, properties);
    return properties.Mass();
}
}  // namespace

bool nator_rim_size(const Shape* body, std::size_t index, std::size_t* size) noexcept {
    return guarded([&] { *size = rim_records(body, index).size(); });
}

bool nator_rim_copy(const Shape* body, std::size_t index, double* output, std::size_t length) noexcept {
    return guarded([&] {
        const auto records = rim_records(body, index);
        if (records.size() != length) throw std::runtime_error("Contornos da face mudaram.");
        std::copy(records.begin(), records.end(), output);
    });
}

// Prisma do anel do contorno `wire`, entre `start` e `end` mm ao longo da normal externa. O lado
// perto do contorno é o próprio fio quando `near_count` é zero; `far` é sempre deslocado.
Shape* nator_rim_ring(const Shape* body, std::size_t index, std::size_t wire, const double* near_points,
                      std::size_t near_count, const double* far_points, std::size_t far_count,
                      double start, double end) noexcept {
    return build([&](Shape& shape) {
        const Rim rim = rim_of(body, index);
        if (wire >= rim.wires.size() || far_count < 3 || !std::isfinite(start) ||
            !std::isfinite(end) || end <= start) {
            throw std::runtime_error("Medidas do anel inválidas.");
        }
        const TopoDS_Wire near_wire = near_count == 0
                                          ? rim.wires[wire]
                                          : polygon_wire(rim.plane, near_points, near_count);
        const TopoDS_Wire far_wire = polygon_wire(rim.plane, far_points, far_count);
        const TopoDS_Face section = wire == 0 ? ring_between(rim.plane, near_wire, far_wire)
                                              : ring_between(rim.plane, far_wire, near_wire);
        const double area = face_area(section);
        BRepAlgoAPI_Common common(section, rim.face);
        // O desvio da amostragem cabe na tolerância: o anel só pode faltar na borda da face.
        if (area < 1e-9 || !common.IsDone() || face_area(common.Shape()) < 0.999 * area) {
            throw std::runtime_error("O anel passa da borda da face.");
        }
        gp_Trsf lift;
        lift.SetTranslation(gp_Vec(rim.normal) * start);
        const TopoDS_Shape moved = BRepBuilderAPI_Transform(section, lift, true).Shape();
        BRepPrimAPI_MakePrism prism(moved, gp_Vec(rim.normal) * (end - start), true);
        if (!prism.IsDone()) throw std::runtime_error("Falha na extrusão do anel.");
        shape.value = prism.Shape();
    });
}
}
