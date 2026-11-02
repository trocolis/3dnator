// Contatos em linha: arestas compartilhadas por mais de duas faces, quando dois volumes do
// mesmo sólido se tocam apenas ao longo delas. O BRepCheck aceita essa topologia, mas ela não
// tem interior definido dos dois lados e impede arredondar, chanfrar e exportar STL.
#pragma once
#include <BRepOffsetAPI_MakePipe.hxx>
#include <set>
extern "C" {
namespace {
// Registro por contato: aresta, quantidade de faces, índices das faces, quantidade de pontos
// e coordenadas. Curvas viram 33 pontos, apenas para apresentação.
std::vector<double> contact_records(const Shape* body) {
    const auto& edges = edges_of(body);
    const auto& faces = faces_of(body);
    TopTools_IndexedDataMapOfShapeListOfShape ancestors;
    TopExp::MapShapesAndAncestors(body->value, TopAbs_EDGE, TopAbs_FACE, ancestors);
    std::vector<double> records;
    for (int i = 1; i <= edges.Extent(); ++i) {
        const auto* list = ancestors.Seek(edges(i));
        if (list == nullptr) {
            continue;
        }
        std::set<int> adjacent;
        for (const auto& face : *list) {
            adjacent.insert(faces.FindIndex(face) - 1);
        }
        if (adjacent.size() <= 2) {
            continue;
        }
        records.push_back(i - 1);
        records.push_back(static_cast<double>(adjacent.size()));
        records.insert(records.end(), adjacent.begin(), adjacent.end());
        BRepAdaptor_Curve curve(TopoDS::Edge(edges(i)));
        const int samples = curve.GetType() == GeomAbs_Line ? 2 : 33;
        records.push_back(samples);
        for (int s = 0; s < samples; ++s) {
            const double t = static_cast<double>(s) / (samples - 1);
            const gp_Pnt point =
                curve.Value(curve.FirstParameter() * (1 - t) + curve.LastParameter() * t);
            records.insert(records.end(), {point.X(), point.Y(), point.Z()});
        }
    }
    return records;
}
}  // namespace

bool nator_contacts_size(const Shape* body, std::size_t* size) noexcept {
    return guarded([&] { *size = contact_records(body).size(); });
}

bool nator_contacts_copy(const Shape* body, double* output, std::size_t length) noexcept {
    return guarded([&] {
        const auto records = contact_records(body);
        if (records.size() != length) {
            throw std::runtime_error("Tamanho do destino dos contatos incorreto.");
        }
        std::copy(records.begin(), records.end(), output);
    });
}

// Pino de seção circular ao longo da aresta: unido ao corpo solda o contato; subtraído,
// abre uma folga. Termina nas pontas da aresta para não invadir o restante da peça.
Shape* nator_contact_pin(const Shape* body, std::size_t index, double radius) noexcept {
    return build([&](Shape& shape) {
        const TopoDS_Edge spine = TopoDS::Edge(edge_at(body, index).Oriented(TopAbs_FORWARD));
        BRepAdaptor_Curve curve(spine);
        gp_Pnt start;
        gp_Vec tangent;
        curve.D1(curve.FirstParameter(), start, tangent);
        if (tangent.Magnitude() < 1e-12) {
            throw std::runtime_error("Aresta sem direção definida.");
        }
        const gp_Ax2 axis(start, gp_Dir(tangent));
        if (curve.GetType() == GeomAbs_Line) {
            shape.value =
                BRepPrimAPI_MakeCylinder(axis, radius, GCPnts_AbscissaPoint::Length(curve)).Shape();
            return;
        }
        const TopoDS_Wire section =
            BRepBuilderAPI_MakeWire(BRepBuilderAPI_MakeEdge(gp_Circ(axis, radius)).Edge()).Wire();
        BRepOffsetAPI_MakePipe pipe(BRepBuilderAPI_MakeWire(spine).Wire(),
                                    BRepBuilderAPI_MakeFace(section, true).Face());
        pipe.Build();
        if (!pipe.IsDone()) {
            throw std::runtime_error("Não foi possível construir o pino de reparo.");
        }
        shape.value = pipe.Shape();
    });
}
}
