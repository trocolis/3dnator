// Prismas exatos dos contornos internos das faces no plano inicial da extrusão.
// Não copiamos o contorno externo: uma saliência pode ultrapassar a borda do
// corpo, mas não deve tampar um furo já existente na face de apoio.
#pragma once
#include <BRepTools.hxx>
extern "C" {
Shape* nator_support_holes(const Shape* body,const double* origin,const double* normal,double distance) noexcept {
    return build([&](Shape& output) {
        const gp_Pnt start(origin[0],origin[1],origin[2]);
        const gp_Dir direction(normal[0],normal[1],normal[2]);
        if (!std::isfinite(distance) || std::abs(distance)<1e-7) throw std::runtime_error("Comprimento de recorte inválido.");
        TopoDS_Compound result;
        BRep_Builder builder;
        builder.MakeCompound(result);
        for (TopExp_Explorer faces(body->value,TopAbs_FACE);faces.More();faces.Next()) {
            const auto face=TopoDS::Face(faces.Current());
            BRepAdaptor_Surface surface(face);
            if (surface.GetType()!=GeomAbs_Plane) continue;
            const auto plane=surface.Plane();
            if (std::abs(plane.Axis().Direction().Dot(direction))<1.-1e-9 || plane.Distance(start)>1e-6) continue;
            const auto outer=BRepTools::OuterWire(face);
            for (TopExp_Explorer wires(face,TopAbs_WIRE);wires.More();wires.Next()) {
                auto wire=TopoDS::Wire(wires.Current());
                if (wire.IsSame(outer)) continue;
                // Um contorno interno vira uma face cheia temporária para remover
                // seu prisma da saliência, conservando círculos e arcos analíticos.
                wire.Reverse();
                BRepBuilderAPI_MakeFace filled(plane,wire,true);
                if (!filled.IsDone()) throw std::runtime_error("Não foi possível preservar o furo da face de apoio.");
                BRepPrimAPI_MakePrism prism(filled.Face(),gp_Vec(direction)*distance,false);
                if (!prism.IsDone()) throw std::runtime_error("Não foi possível prolongar o furo da face de apoio.");
                builder.Add(result,prism.Shape());
            }
        }
        output.value=result;
    });
}
}
