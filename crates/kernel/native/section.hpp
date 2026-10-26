// Interseção de um corpo com plano infinito, preservando sólidos e suas tolerâncias.
#pragma once
#include <BRepAlgoAPI_Section.hxx>
#include "curve_sampling.hpp"
namespace { struct SectionSegments { std::vector<double> values; }; }
extern "C" {
SectionSegments* nator_section(const Shape* body,const double* origin,const double* normal,double tolerance) noexcept {
    auto result=std::make_unique<SectionSegments>();
    const bool ok=guarded([&] {
        BRepAlgoAPI_Section intersection(body->value,gp_Pln(gp_Pnt(origin[0],origin[1],origin[2]),gp_Dir(normal[0],normal[1],normal[2])),false);
        intersection.Approximation(false);
        intersection.Build();
        if (!intersection.IsDone()) throw std::runtime_error("Não foi possível calcular o corte do B-rep.");
        sample_edges(intersection.Shape(),tolerance,[&](const gp_Pnt& a,const gp_Pnt& b){
            if (result->values.size()/6>=500000) throw std::runtime_error("Corte excede quinhentos mil segmentos.");
            result->values.insert(result->values.end(),{a.X(),a.Y(),a.Z(),b.X(),b.Y(),b.Z()});
        });
    });
    return ok ? result.release() : nullptr;
}
std::size_t nator_section_size(const SectionSegments* section) noexcept {return section->values.size();}
bool nator_section_copy(const SectionSegments* section,double* output,std::size_t size) noexcept {
    return guarded([&]{
        if (size!=section->values.size()) throw std::runtime_error("Destino de corte inválido.");
        std::copy(section->values.begin(),section->values.end(),output);
    });
}
void nator_section_release(SectionSegments* section) noexcept {delete section;}
}
