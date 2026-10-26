// Amostragem comum às curvas de projeção e interseção, com controle de desvio em mm.
#pragma once
#include <BRepLib.hxx>
#include <GCPnts_UniformDeflection.hxx>
#include <TColStd_Array1OfReal.hxx>
namespace {
template <typename Emit>
void sample_edges(const TopoDS_Shape& shape, double tolerance, Emit emit) {
    if (shape.IsNull()) return;
    BRepLib::BuildCurves3d(shape);
    for (TopExp_Explorer it(shape,TopAbs_EDGE);it.More();it.Next()) {
        BRepAdaptor_Curve curve(TopoDS::Edge(it.Current()));
        // Separar intervalos C2 mantém o controle de desvio também em splines com
        // nós de continuidade menor; a tolerância não vem da malha de exibição.
        const int count=curve.NbIntervals(GeomAbs_C2);
        if (count>500000) throw std::runtime_error("Curva tem intervalos demais.");
        TColStd_Array1OfReal intervals(1,count+1);
        curve.Intervals(intervals,GeomAbs_C2);
        for (int interval=1;interval<=count;++interval) {
            GCPnts_UniformDeflection points(curve,tolerance,intervals(interval),intervals(interval+1),true);
            if (!points.IsDone()) throw std::runtime_error("Falha ao discretizar uma curva.");
            if (points.NbPoints()>500000) throw std::runtime_error("Curva complexa demais.");
            for (int i=2;i<=points.NbPoints();++i) {
                const auto a=points.Value(i-1),b=points.Value(i);
                if (a.Distance(b)>=1e-10) emit(a,b);
            }
        }
    }
}
}
