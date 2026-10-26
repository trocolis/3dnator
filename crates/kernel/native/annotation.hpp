// Pontos geométricos de apoio à cotagem. Índices são resolvidos a partir dos nomes
// estáveis em Rust antes de entrar nesta ponte; nada é associado pela proximidade.
#pragma once
extern "C" {
bool nator_face_anchors(const Shape* body,std::size_t index,double* output) noexcept {
    return guarded([&] {
        const auto faces=map_of(body->value,TopAbs_FACE);
        if (index>=static_cast<std::size_t>(faces.Extent())) throw std::runtime_error("Face da cota inexistente.");
        const auto face=TopoDS::Face(faces(int(index)+1));
        GProp_GProps properties; BRepGProp::SurfaceProperties(face,properties);
        if (properties.Mass()<=0.) throw std::runtime_error("Face da cota sem área.");
        const auto centroid=properties.CentreOfMass();
        std::fill(output,output+10,0.);
        output[0]=centroid.X();output[1]=centroid.Y();output[2]=centroid.Z();
        BRepAdaptor_Surface surface(face);
        gp_Pnt center,point;
        if (surface.GetType()==GeomAbs_Cylinder) {
            const auto cylinder=surface.Cylinder();
            const auto axis=cylinder.Axis();
            const double offset=gp_Vec(axis.Location(),centroid).Dot(gp_Vec(axis.Direction()));
            center=axis.Location().Translated(gp_Vec(axis.Direction())*offset);
            point=center.Translated(gp_Vec(cylinder.Position().XDirection())*cylinder.Radius());
            output[9]=1.;
        } else if (surface.GetType()==GeomAbs_Sphere) {
            const auto sphere=surface.Sphere();
            center=sphere.Location();
            point=center.Translated(gp_Vec(sphere.Position().XDirection())*sphere.Radius());
            output[9]=1.;
        }
        if (output[9]>0.) {
            output[3]=center.X();output[4]=center.Y();output[5]=center.Z();
            output[6]=point.X();output[7]=point.Y();output[8]=point.Z();
        }
    });
}
bool nator_edge_anchors(const Shape* body,std::size_t index,double* output) noexcept {
    return guarded([&] {
        BRepAdaptor_Curve curve(edge_at(body,index));
        const auto start=curve.Value(curve.FirstParameter()), end=curve.Value(curve.LastParameter());
        std::fill(output,output+10,0.);
        output[0]=start.X();output[1]=start.Y();output[2]=start.Z();
        output[3]=end.X();output[4]=end.Y();output[5]=end.Z();
        if (curve.GetType()==GeomAbs_Circle) {
            const auto center=curve.Circle().Location();
            output[6]=center.X();output[7]=center.Y();output[8]=center.Z();output[9]=1.;
        }
    });
}
}
