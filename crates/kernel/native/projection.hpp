// Remoção exata de linhas ocultas no B-rep; curvas resultantes são amostradas
// com desvio linear explícito em mm, independentemente da malha de exibição.
#pragma once
#include <HLRBRep_Algo.hxx>
#include <HLRBRep_HLRToShape.hxx>
#include <HLRAlgo_Projector.hxx>
#include "curve_sampling.hpp"
namespace {
struct Projection { std::vector<double> segments; };
}
extern "C" {
// Mantém os pontos da curva 3D em um resultado de propriedade exclusiva, com
// o mesmo ciclo de cópia/liberação das projeções de desenho técnico.
Projection* nator_edge_polyline(const Shape* body, std::size_t index, double tolerance) noexcept {
    auto result = std::make_unique<Projection>();
    const bool ok = guarded([&] {
        if (!std::isfinite(tolerance) || tolerance < 1e-6 || tolerance > 1.)
            throw std::runtime_error("Tolerância de curva inválida.");
        sample_edges(edge_at(body,index),tolerance,[&](const gp_Pnt& a,const gp_Pnt& b){
            if (result->segments.empty()) result->segments.insert(result->segments.end(),{a.X(),a.Y(),a.Z()});
            if (result->segments.size()/3>=500000) throw std::runtime_error("Curva complexa demais.");
            result->segments.insert(result->segments.end(),{b.X(),b.Y(),b.Z()});
        });
    });
    return ok ? result.release() : nullptr;
}
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
            sample_edges(shape,tolerance,[&](const gp_Pnt& a,const gp_Pnt& b){
                if (result->segments.size()/5>=500000) throw std::runtime_error("Projeção excede quinhentos mil segmentos.");
                result->segments.insert(result->segments.end(),{a.X(),a.Y(),b.X(),b.Y(),hidden?1.:0.});
            });
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
