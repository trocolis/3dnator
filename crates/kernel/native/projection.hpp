// Remoção exata de linhas ocultas no B-rep; curvas resultantes são amostradas
// com desvio linear explícito em mm, independentemente da malha de exibição.
#pragma once
#include <HLRBRep_Algo.hxx>
#include <HLRBRep_HLRToShape.hxx>
#include <HLRAlgo_Projector.hxx>
#include <BRepLib.hxx>
#include <GCPnts_UniformDeflection.hxx>
#include <TColStd_Array1OfReal.hxx>
namespace {
struct Projection { std::vector<double> segments; };
}
extern "C" {
Projection* nator_project(const Shape* const* bodies, std::size_t count, const double* right, const double* direction, double tolerance) noexcept {
    auto result = std::make_unique<Projection>();
    const bool ok = guarded([&] {
        if (!count || count > 10000 || !std::isfinite(tolerance) || tolerance <= 0.) throw std::runtime_error("Parâmetros de projeção inválidos.");
        Handle(HLRBRep_Algo) algorithm = new HLRBRep_Algo();
        for (std::size_t i=0;i<count;++i) algorithm->Add(bodies[i]->value);
        algorithm->Projector(HLRAlgo_Projector(gp_Ax2(gp_Pnt(0.,0.,0.),gp_Dir(direction[0],direction[1],direction[2]),gp_Dir(right[0],right[1],right[2]))));
        algorithm->Update();
        algorithm->Hide();
        HLRBRep_HLRToShape extracted(algorithm);
        const auto append = [&](const TopoDS_Shape& shape, bool hidden) {
            if (shape.IsNull()) return;
            BRepLib::BuildCurves3d(shape);
            for (TopExp_Explorer it(shape,TopAbs_EDGE);it.More();it.Next()) {
                BRepAdaptor_Curve curve(TopoDS::Edge(it.Current()));
                // Separar intervalos C2 mantém a garantia de desvio do amostrador
                // também em splines com nós de continuidade menor que C2.
                const int intervals_count = curve.NbIntervals(GeomAbs_C2);
                if (intervals_count > 500000) throw std::runtime_error("Curva projetada tem intervalos demais.");
                TColStd_Array1OfReal intervals(1,intervals_count+1);
                curve.Intervals(intervals,GeomAbs_C2);
                for (int interval=1;interval<=intervals_count;++interval) {
                    GCPnts_UniformDeflection points(curve,tolerance,intervals(interval),intervals(interval+1),true);
                    if (!points.IsDone()) throw std::runtime_error("Falha ao discretizar uma curva da vista.");
                    if (points.NbPoints() > 500000) throw std::runtime_error("Curva projetada complexa demais.");
                    for (int i=2;i<=points.NbPoints();++i) {
                        const auto a=points.Value(i-1), b=points.Value(i);
                        if (a.Distance(b)<1e-10) continue;
                        if (result->segments.size()/5 >= 500000) throw std::runtime_error("Projeção excede quinhentos mil segmentos.");
                        result->segments.insert(result->segments.end(),{a.X(),a.Y(),b.X(),b.Y(),hidden?1.:0.});
                    }
                }
            }
        };
        append(extracted.VCompound(),false);
        append(extracted.OutLineVCompound(),false);
        append(extracted.HCompound(),true);
        append(extracted.OutLineHCompound(),true);
    });
    return ok ? result.release() : nullptr;
}
std::size_t nator_projection_size(const Projection* projection) noexcept {return projection->segments.size();}
bool nator_projection_copy(const Projection* projection, double* output, std::size_t count) noexcept {
    return guarded([&] {
        if (count!=projection->segments.size()) throw std::runtime_error("Destino de projeção inválido.");
        std::copy(projection->segments.begin(),projection->segments.end(),output);
    });
}
void nator_projection_release(Projection* projection) noexcept {delete projection;}
}
