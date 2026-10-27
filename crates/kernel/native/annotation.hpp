// Pontos geométricos de apoio à cotagem e à captura. Índices são resolvidos a partir dos nomes
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
        const auto edge=edge_at(body,index);
        // Vinte e quatro doubles: extremos, centro, indicador circular, normal,
        // raio, fechamento, costura, ponto médio, direção radial e intervalo angular.
        std::fill(output,output+24,0.);
        // Polos de esferas/cones podem ter uma aresta sem curva 3D. Ela não
        // oferece captura, mas continua fornecendo um apoio finito para cotas.
        if (BRep_Tool::Degenerated(edge)) {
            const auto vertex=TopExp::FirstVertex(edge);
            if (vertex.IsNull()) throw std::runtime_error("Aresta degenerada sem vértice de apoio.");
            const auto point=BRep_Tool::Pnt(vertex);
            output[0]=output[3]=point.X();output[1]=output[4]=point.Y();output[2]=output[5]=point.Z();
            output[16]=point.X();output[17]=point.Y();output[18]=point.Z();
            output[15]=1.;
            return;
        }
        BRepAdaptor_Curve curve(edge);
        const auto start=curve.Value(curve.FirstParameter()), end=curve.Value(curve.LastParameter());
        output[0]=start.X();output[1]=start.Y();output[2]=start.Z();
        output[3]=end.X();output[4]=end.Y();output[5]=end.Z();
        // Linhas/círculos têm velocidade paramétrica constante; nas outras
        // curvas, dividir o parâmetro ao meio não garante metade do comprimento.
        double middle=(curve.FirstParameter()+curve.LastParameter())*0.5;
        if (curve.GetType()!=GeomAbs_Line && curve.GetType()!=GeomAbs_Circle) {
            const double length=GCPnts_AbscissaPoint::Length(curve,curve.FirstParameter(),curve.LastParameter());
            GCPnts_AbscissaPoint half(curve,length*0.5,curve.FirstParameter());
            if (!half.IsDone()) throw std::runtime_error("Não foi possível obter o ponto médio da aresta.");
            middle=half.Parameter();
        }
        const auto midpoint=curve.Value(middle);
        output[16]=midpoint.X();output[17]=midpoint.Y();output[18]=midpoint.Z();
        if (curve.GetType()==GeomAbs_Circle) {
            const auto circle=curve.Circle();
            const auto center=circle.Location();
            output[6]=center.X();output[7]=center.Y();output[8]=center.Z();output[9]=1.;
            const auto normal=circle.Axis().Direction();
            output[10]=normal.X();output[11]=normal.Y();output[12]=normal.Z();
            output[13]=circle.Radius();
            output[14]=std::abs(curve.LastParameter()-curve.FirstParameter()-2.*std::acos(-1.))<1e-7 ? 1. : 0.;
            const auto radial=circle.Position().XDirection();
            output[19]=radial.X();output[20]=radial.Y();output[21]=radial.Z();
            output[22]=curve.FirstParameter();output[23]=curve.LastParameter()-curve.FirstParameter();
        }
        // Uma costura cilíndrica possui duas curvas paramétricas na mesma face.
        // Seus extremos não são vértices geométricos para a captura pelo usuário.
        TopTools_IndexedDataMapOfShapeListOfShape ancestors;
        TopExp::MapShapesAndAncestors(body->value,TopAbs_EDGE,TopAbs_FACE,ancestors);
        if (ancestors.Contains(edge)) {
            for (const auto& shape:ancestors.FindFromKey(edge)) {
                if (BRep_Tool::IsClosed(edge,TopoDS::Face(shape))) {output[15]=1.;break;}
            }
        }
    });
}
}
