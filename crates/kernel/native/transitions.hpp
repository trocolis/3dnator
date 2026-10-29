// Transições exatas entre perfis. A compatibilidade do OCCT subdivide arestas
// quando necessário (por exemplo retângulo → círculo), sem poligonizar curvas.
#pragma once
#include <BRepOffsetAPI_ThruSections.hxx>
#include <BOPAlgo_ArgumentAnalyzer.hxx>
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

#include "loft_alignment.hpp"
extern "C" Shape* nator_loft_wire(const Shape* first,const Shape* last,std::size_t wire,bool ruled) noexcept {
    return build([&](Shape& output) {
        BRepOffsetAPI_ThruSections loft(true,ruled,1e-7);
        loft.CheckCompatibility(true);
        const auto sections=loft_aligned_wires(transition_wire(first,wire),transition_wire(last,wire));
        loft.AddWire(sections.first);
        loft.AddWire(sections.second);
        loft.Build();
        if (!loft.IsDone()) throw std::runtime_error("Não foi possível criar a transição entre os contornos.");
        output.value=loft.Shape();
        OriginTable names(output.value);
        names.mark_all(loft.FirstShape(),StartCap,0,0);
        names.mark_all(loft.LastShape(),EndCap,0,0);
        output.origins=names.take();
    });
}

#include <BRepOffsetAPI_MakePipeShell.hxx>
#include <BRepFill_CompatibleWires.hxx>
#include <Geom_BezierCurve.hxx>
#include <TColgp_Array1OfPnt.hxx>
#include <TopTools_SequenceOfShape.hxx>
extern "C" Shape* nator_guide_wire(const double* records,std::size_t count) noexcept {
    return build([&](Shape& output) {
        if (!count || count>1024) throw std::runtime_error("Trajetória vazia ou excessiva.");
        BRepBuilderAPI_MakeWire wire;
        for (std::size_t i=0;i<count;++i) {
            const auto* r=records+i*13;
            const gp_Pnt a(r[1],r[2],r[3]),b(r[4],r[5],r[6]),c(r[7],r[8],r[9]);
            if (r[0]==0) wire.Add(BRepBuilderAPI_MakeEdge(a,b).Edge());
            else if (r[0]==1) wire.Add(BRepBuilderAPI_MakeEdge(GC_MakeArcOfCircle(a,b,c).Value()).Edge());
            else if (r[0]==3) {
                TColgp_Array1OfPnt poles(1,4);poles.SetValue(1,a);poles.SetValue(2,b);poles.SetValue(3,c);
                poles.SetValue(4,gp_Pnt(r[10],r[11],r[12]));
                wire.Add(BRepBuilderAPI_MakeEdge(new Geom_BezierCurve(poles)).Edge());
            } else throw std::runtime_error("Curva de trajetória inválida.");
            if (!wire.IsDone()) throw std::runtime_error("A trajetória precisa ser uma cadeia conectada, sem ramificações.");
        }
        output.value=wire.Wire();
        if (output.value.Closed()) throw std::runtime_error("Escolha uma trajetória aberta entre os dois perfis.");
    });
}
extern "C" Shape* nator_pipe_wire(const Shape* first,const Shape* last,const Shape* guide,std::size_t wire) noexcept {
    return build([&](Shape& output) {
        const auto spine=TopoDS::Wire(guide->value);
        TopoDS_Vertex begin,end;TopExp::Vertices(spine,begin,end);
        if (begin.IsNull() || end.IsNull()) throw std::runtime_error("A trajetória não tem duas extremidades válidas.");
        TopTools_SequenceOfShape sections;
        sections.Append(transition_wire(first,wire));sections.Append(transition_wire(last,wire));
        BRepFill_CompatibleWires compatible(sections);compatible.Perform(false);
        if (!compatible.IsDone()) throw std::runtime_error("Não foi possível compatibilizar os perfis da transição guiada.");
        BRepOffsetAPI_MakePipeShell pipe(spine);
        pipe.SetMode(false); // Triedro de Frenet corrigido, evitando torção espúria.
        pipe.SetTransitionMode(BRepBuilderAPI_RoundCorner);
        pipe.SetTolerance(1e-6,1e-6,1e-4);
        pipe.Add(compatible.Shape()(1),begin,false,false);
        pipe.Add(compatible.Shape()(2),end,false,false);
        pipe.Build();
        if (!pipe.IsDone() || !pipe.MakeSolid()) throw std::runtime_error("A trajetória e os perfis não formam uma transição sólida válida.");
        output.value=pipe.Shape();
        BOPAlgo_ArgumentAnalyzer check;check.SetShape1(output.value);
        check.SelfInterMode()=true;check.StopOnFirstFaulty()=true;check.Perform();
        if (check.HasFaulty()) throw std::runtime_error("A transição guiada se autointersecta; aumente o raio da curva ou reduza os perfis.");
        // Somente tampas geometricamente identificáveis recebem nomes estáveis.
        // Laterais interpoladas usam referências conservadoras por assinatura.
        OriginTable names(output.value);
        const auto a=BRepAdaptor_Surface(TopoDS::Face(first->value)).Plane();
        const auto b=BRepAdaptor_Surface(TopoDS::Face(last->value)).Plane();
        for (TopExp_Explorer it(output.value,TopAbs_FACE);it.More();it.Next()) {
            const auto face=TopoDS::Face(it.Current());BRepAdaptor_Surface surface(face);
            if (surface.GetType()!=GeomAbs_Plane) continue;
            const auto plane=surface.Plane();
            if (std::abs(plane.Axis().Direction().Dot(a.Axis().Direction()))>1.-1e-8 && a.Distance(plane.Location())<1e-6) names.mark(face,StartCap,0,0);
            else if (std::abs(plane.Axis().Direction().Dot(b.Axis().Direction()))>1.-1e-8 && b.Distance(plane.Location())<1e-6) names.mark(face,EndCap,0,0);
        }
        output.origins=names.take();
    });
}
