// Pontos geométricos de apoio à cotagem e à captura. Índices são resolvidos a partir dos nomes
// estáveis em Rust antes de entrar nesta ponte; nada é associado pela proximidade.
#pragma once
extern "C" {
// Contorno externo retilíneo para o reconhecimento de triângulos/retângulos em Rust.
// Furos não deslocam o centro geométrico do contorno exterior. A consulta não usa nomes.
bool nator_face_polygon(const Shape* body,std::size_t index,double* output) noexcept {
    return guarded([&] {
        std::fill(output,output+13,0.);
        const auto& faces = faces_of(body);
        if (index>=static_cast<std::size_t>(faces.Extent())) throw std::runtime_error("Face de apoio inexistente.");
        const auto face=TopoDS::Face(faces(int(index)+1));
        if (BRepAdaptor_Surface(face).GetType()!=GeomAbs_Plane) return;
        const auto wire=BRepTools::OuterWire(face);
        if (wire.IsNull()) return;
        std::vector<gp_Pnt> points;
        for (BRepTools_WireExplorer explorer(wire,face);explorer.More();explorer.Next()) {
            if (BRepAdaptor_Curve(explorer.Current()).GetType()!=GeomAbs_Line) return;
            points.push_back(BRep_Tool::Pnt(explorer.CurrentVertex()));
        }
        // O mesmo lado pode ter várias arestas por operações anteriores. Só os cantos
        // contam na classificação; uma mudança de sentido não é descartada como reta.
        bool changed=true;
        while (changed && points.size()>=3) {
            changed=false;
            for (std::size_t i=0;i<points.size();++i) {
                const gp_Vec a(points[(i+points.size()-1)%points.size()],points[i]);
                const gp_Vec b(points[i],points[(i+1)%points.size()]);
                if (a.Magnitude()<1e-9 || (a.Dot(b)>=0. && a.Crossed(b).Magnitude()<=1e-9*a.Magnitude()*b.Magnitude())) {
                    points.erase(points.begin()+i);changed=true;break;
                }
            }
        }
        if (points.size()!=3 && points.size()!=4) return;
        output[0]=static_cast<double>(points.size());
        for (std::size_t i=0;i<points.size();++i) {
            output[1+i*3]=points[i].X();output[2+i*3]=points[i].Y();output[3+i*3]=points[i].Z();
        }
    });
}
bool nator_face_anchors(const Shape* body,std::size_t index,double* output) noexcept {
    return guarded([&] {
        const auto& faces = faces_of(body);
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
// Distância até a face e o ponto dela mais próximo de `point`.
bool nator_face_nearest(const Shape* body,std::size_t index,const double* point,double* output) noexcept {
    return guarded([&] {
        const auto& faces = faces_of(body);
        if (index>=static_cast<std::size_t>(faces.Extent())) throw std::runtime_error("Face inexistente.");
        const TopoDS_Vertex vertex=BRepBuilderAPI_MakeVertex(gp_Pnt(point[0],point[1],point[2]));
        BRepExtrema_DistShapeShape distance(faces(static_cast<int>(index)+1),vertex);
        if (!distance.IsDone()||distance.NbSolution()==0) {
            throw std::runtime_error("Não foi possível medir a distância até a face.");
        }
        const gp_Pnt nearest=distance.PointOnShape1(1);
        output[0]=distance.Value();
        output[1]=nearest.X();
        output[2]=nearest.Y();
        output[3]=nearest.Z();
    });
}

bool nator_edge_anchors(const Shape* body,std::size_t index,double* output) noexcept {
    return guarded([&] {
        const auto edge=edge_at(body,index);
        // Trinta e um doubles: extremos, centro, indicador circular, normal,
        // raio, fechamento, costura, ponto médio, direção radial, intervalo angular, a
        // soma das normais externas das faces vizinhas no ponto médio, o centro no material
        // e a normal externa da face plana que contém o círculo (zero sem face plana).
        std::fill(output,output+31,0.);
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
            // Centro sobre o material: dentro de uma face plana vizinha que contém o círculo
            // (o disco de um cilindro). No vazio de um furo ou no canto de um arredondamento
            // côncavo, o centro é virtual.
            const auto& neighbours=ancestors_of(body);
            if (neighbours.Contains(edge)) {
                for (const auto& shape:neighbours.FindFromKey(edge)) {
                    const auto face=TopoDS::Face(shape);
                    const BRepAdaptor_Surface surface(face);
                    if (surface.GetType()!=GeomAbs_Plane) continue;
                    const auto plane=surface.Plane();
                    if (plane.Distance(center)>1e-7) continue;
                    // Normal que sai do material pela face: a seta vem de fora até o centro.
                    if (output[28]==0. && output[29]==0. && output[30]==0.) {
                        auto outward=plane.Axis().Direction();
                        if (face.Orientation()==TopAbs_REVERSED) outward.Reverse();
                        output[28]=outward.X();output[29]=outward.Y();output[30]=outward.Z();
                    }
                    double u=0.,v=0.;
                    ElSLib::Parameters(plane,center,u,v);
                    const BRepClass_FaceClassifier inside(face,gp_Pnt2d(u,v),1e-7);
                    if (inside.State()==TopAbs_IN || inside.State()==TopAbs_ON) {output[27]=1.;break;}
                }
            }
        }
        // Uma costura cilíndrica possui duas curvas paramétricas na mesma face.
        // Seus extremos não são vértices geométricos para a captura pelo usuário.
        const auto& ancestors=ancestors_of(body);
        if (ancestors.Contains(edge)) {
            for (const auto& shape:ancestors.FindFromKey(edge)) {
                if (BRep_Tool::IsClosed(edge,TopoDS::Face(shape))) {output[15]=1.;break;}
            }
            // Bissetriz das faces da aresta, para a seta perpendicular ao plano do ponto.
            // A normal de BRepGProp_Face já considera a orientação da face (sai do sólido).
            gp_Vec sum(0.,0.,0.);
            for (const auto& shape:ancestors.FindFromKey(edge)) {
                const auto face=TopoDS::Face(shape);
                try {
                    const BRepAdaptor_Curve2d pcurve(edge,face);
                    const auto uv=pcurve.Value(middle);
                    gp_Pnt point; gp_Vec normal;
                    BRepGProp_Face(face).Normal(uv.X(),uv.Y(),point,normal);
                    if (normal.Magnitude()>1e-12) sum+=normal.Normalized();
                } catch (const Standard_Failure&) {
                    // Sem curva paramétrica nesta face: a outra face ainda orienta a seta.
                }
            }
            output[24]=sum.X();output[25]=sum.Y();output[26]=sum.Z();
        }
    });
}
}
