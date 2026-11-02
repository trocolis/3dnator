// Saída automática de uma casca. Amostramos pontos interiores da abertura,
// lançando raios para dentro do corpo. Só aceitamos quando todos encontram a
// mesma primeira face: degraus/ramificações exigem seleção explícita da saída.
#pragma once
#include <BRepClass_FaceClassifier.hxx>
#include <IntCurvesFace_ShapeIntersector.hxx>
#include <gp_Lin.hxx>
#include <gp_Pnt2d.hxx>
extern "C" {
bool nator_opposite_face(const Shape* body, std::size_t index, std::size_t* output) noexcept {
    return guarded([&] {
        const auto& faces = faces_of(body);
        if (index>=static_cast<std::size_t>(faces.Extent())) throw std::runtime_error("Face de entrada inexistente.");
        const auto face=TopoDS::Face(faces(static_cast<int>(index)+1));
        BRepAdaptor_Surface surface(face);
        if (surface.GetType()!=GeomAbs_Plane) throw std::runtime_error("Selecione a face de saída para uma abertura não plana.");
        auto normal=surface.Plane().Axis().Direction();
        if (face.Orientation()==TopAbs_REVERSED) normal.Reverse();
        normal.Reverse(); // Raio contra a normal externa, em milímetros.
        double u0,u1,v0,v1;
        BRepTools::UVBounds(face,u0,u1,v0,v1);
        IntCurvesFace_ShapeIntersector ray;
        ray.Load(body->value,1e-7);
        int chosen=0;
        for (double u : {0.5,0.15,0.35,0.65,0.85}) for (double v : {0.5,0.15,0.35,0.65,0.85}) {
            const double pu=u0+(u1-u0)*u, pv=v0+(v1-v0)*v;
            BRepClass_FaceClassifier inside(face,gp_Pnt2d(pu,pv),1e-7);
            if (inside.State()!=TopAbs_IN) continue; // Furos não são apoios.
            const auto start=surface.Value(pu,pv);
            ray.Perform(gp_Lin(start,normal),1e-5,RealLast());
            if (!ray.IsDone()) throw std::runtime_error("Não foi possível localizar a saída da casca.");
            double distance=RealLast(); int exit=0;
            for (int hit=1;hit<=ray.NbPnt();++hit) {
                const int other=faces.FindIndex(ray.Face(hit));
                const double along=ray.WParameter(hit);
                if (other!=static_cast<int>(index)+1 && along<distance && along>1e-5) { distance=along; exit=other; }
            }
            if (!exit || (chosen && chosen!=exit)) throw std::runtime_error("Saída oposta ambígua; selecione a face de saída da casca.");
            chosen=exit;
        }
        if (!chosen) throw std::runtime_error("Não há saída oposta válida; selecione a face de saída.");
        *output=static_cast<std::size_t>(chosen-1);
    });
}
}
