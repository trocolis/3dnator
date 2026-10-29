// Transições exatas entre perfis. A compatibilidade do OCCT subdivide arestas
// quando necessário (por exemplo retângulo → círculo), sem poligonizar curvas.
#pragma once
#include <BRepOffsetAPI_ThruSections.hxx>
namespace {
TopoDS_Wire transition_wire(const Shape* profile,std::size_t index) {
    require_profile(profile);
    const auto face=TopoDS::Face(profile->value);
    const auto outer=BRepTools::OuterWire(face);
    if (!index) return outer;
    std::size_t position=1;
    for (TopExp_Explorer it(face,TopAbs_WIRE);it.More();it.Next()) {
        auto wire=TopoDS::Wire(it.Current());
        if (wire.IsSame(outer)) continue;
        if (position++==index) {wire.Reverse();return wire;}
    }
    throw std::runtime_error("Contorno de furo inexistente na transição.");
}
}
extern "C" Shape* nator_loft_wire(const Shape* first,const Shape* last,std::size_t wire,bool ruled) noexcept {
    return build([&](Shape& output) {
        BRepOffsetAPI_ThruSections loft(true,ruled,1e-7);
        loft.CheckCompatibility(true);
        loft.AddWire(transition_wire(first,wire));
        loft.AddWire(transition_wire(last,wire));
        loft.Build();
        if (!loft.IsDone()) throw std::runtime_error("Não foi possível criar a transição entre os contornos.");
        output.value=loft.Shape();
        OriginTable names(output.value);
        names.mark_all(loft.FirstShape(),StartCap,0,0);
        names.mark_all(loft.LastShape(),EndCap,0,0);
        output.origins=names.take();
    });
}
