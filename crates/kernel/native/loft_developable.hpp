// Transição desenvolvível entre polígono convexo e círculo, a clássica "quadrado para
// redondo": cada lado do polígono forma um triângulo plano com seu ponto de tangência no
// círculo, e cada canto forma um cone oblíquo com o arco entre duas tangências. O resultado
// é o fecho convexo dos contornos, sem a torção das faces de BRepOffsetAPI_ThruSections, e
// as curvas continuam exatas: os arcos são racionais e não há facetas no corpo.
//
// O ápice dos cones é um ponto degenerado que o deslocamento do OCCT não resolve. A casca
// usa o fato de o corpo ser convexo: o sólido deslocado é a interseção dos semiespaços dos
// planos tangentes, calculada aqui como poliedro com desvio limitado e subtraída do corpo.
#pragma once
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepBuilderAPI_MakeSolid.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepLib.hxx>
#include <GeomConvert.hxx>
#include <Geom_BSplineSurface.hxx>
#include <Geom_Circle.hxx>
#include <Geom_TrimmedCurve.hxx>
#include <TColStd_Array1OfInteger.hxx>
#include <TColStd_Array1OfReal.hxx>
#include <TColStd_Array2OfReal.hxx>
#include <TColgp_Array2OfPnt.hxx>
namespace {
// Desvio máximo entre o poliedro da casca e a parede ideal, em mm.
const double hull_deviation = 0.005;
struct DevelopableFrame {
  std::vector<gp_Pnt> polygon;
  gp_Circ circle;
  // Ângulo de tangência de cada lado, na ordem do polígono.
  std::vector<double> angles;
  // Sentido em que os ângulos avançam junto com os lados.
  double sense = 1.;
  // Abertura do arco no canto final do lado `side`.
  double arc(std::size_t side) const {
    const double step =
        sense * (angles[(side + 1) % angles.size()] - angles[side]);
    const double wrapped = std::fmod(step, 2. * loft_pi);
    return wrapped < 0. ? wrapped + 2. * loft_pi : wrapped;
  }
};
std::optional<std::vector<gp_Pnt>> polygon_of(const TopoDS_Wire &wire) {
  std::vector<gp_Pnt> points;
  for (BRepTools_WireExplorer it(wire); it.More(); it.Next()) {
    if (BRepAdaptor_Curve(it.Current()).GetType() != GeomAbs_Line)
      return std::nullopt;
    points.push_back(BRep_Tool::Pnt(it.CurrentVertex()));
  }
  if (points.size() < 3 || points.size() > 1024)
    return std::nullopt;
  return points;
}
// Círculo completo, inteiro ou dividido em arcos pela costura com as faces laterais.
std::optional<gp_Circ> circle_of(const TopoDS_Wire &wire) {
  std::optional<gp_Circ> circle;
  double sweep = 0.;
  for (BRepTools_WireExplorer it(wire); it.More(); it.Next()) {
    BRepAdaptor_Curve curve(it.Current());
    if (curve.GetType() != GeomAbs_Circle)
      return std::nullopt;
    const auto current = curve.Circle();
    if (circle &&
        (current.Location().Distance(circle->Location()) > 1e-6 ||
         std::abs(current.Radius() - circle->Radius()) > 1e-6))
      return std::nullopt;
    if (!circle)
      circle = current;
    sweep += curve.LastParameter() - curve.FirstParameter();
  }
  if (std::abs(sweep - 2. * loft_pi) > 1e-6)
    return std::nullopt;
  return circle;
}
gp_Pnt circle_point(const gp_Circ &circle, double angle) {
  return circle.Location().Translated(
      gp_Vec(circle.XAxis().Direction()) * (circle.Radius() * std::cos(angle)) +
      gp_Vec(circle.YAxis().Direction()) * (circle.Radius() * std::sin(angle)));
}
gp_Vec circle_tangent(const gp_Circ &circle, double angle) {
  return gp_Vec(circle.XAxis().Direction()) * -std::sin(angle) +
         gp_Vec(circle.YAxis().Direction()) * std::cos(angle);
}
// Ângulo do ponto do círculo cujo plano tangente contém o lado a→b e deixa o polígono e o
// círculo do mesmo lado. A coplanaridade de lado, tangente e corda vale
// A cos θ + B sin θ + D = 0, resolvida em forma fechada. Sem plano de apoio, o par de
// contornos não é convexo e a construção não se aplica.
std::optional<double> tangent_angle(const gp_Circ &circle, const gp_Pnt &a,
                                    const gp_Pnt &b,
                                    const std::vector<gp_Pnt> &polygon) {
  const gp_Vec x(circle.XAxis().Direction()), y(circle.YAxis().Direction());
  const gp_Vec z = x.Crossed(y), side(a, b), to_center(a, circle.Location());
  if (side.Magnitude() < 1e-9)
    return std::nullopt;
  const gp_Vec d = side.Normalized();
  const double A = d.Dot(y.Crossed(to_center)),
               B = -d.Dot(x.Crossed(to_center));
  const double D = -circle.Radius() * d.Dot(z), reach = std::hypot(A, B);
  if (reach < 1e-12 || std::abs(D) > reach)
    return std::nullopt;
  const double base = std::atan2(B, A), spread = std::acos(-D / reach);
  for (const double angle : {base + spread, base - spread}) {
    const gp_Vec normal = d.Crossed(gp_Vec(a, circle_point(circle, angle)));
    if (normal.Magnitude() < 1e-9)
      continue;
    const gp_Vec n = normal.Normalized();
    const double center = n.Dot(to_center);
    double low = std::min(center, 0.), high = std::max(center, 0.);
    for (const auto &point : polygon) {
      const double value = n.Dot(gp_Vec(a, point));
      low = std::min(low, value);
      high = std::max(high, value);
    }
    if (low > -1e-7 || high < 1e-7)
      return angle;
  }
  return std::nullopt;
}
// Sem par polígono convexo + círculo não há plano tangente de apoio em algum lado.
std::optional<DevelopableFrame> developable_frame(std::vector<gp_Pnt> polygon,
                                                  const gp_Circ &circle) {
  DevelopableFrame frame;
  frame.circle = circle;
  const std::size_t n = polygon.size();
  for (std::size_t i = 0; i < n; ++i) {
    const auto angle =
        tangent_angle(circle, polygon[i], polygon[(i + 1) % n], polygon);
    if (!angle)
      return std::nullopt;
    frame.angles.push_back(*angle);
  }
  frame.polygon = std::move(polygon);
  double total[2] = {0., 0.};
  for (int side = 0; side < 2; ++side) {
    frame.sense = side ? -1. : 1.;
    for (std::size_t i = 0; i < n; ++i)
      total[side] += frame.arc(i);
  }
  // Polígono e círculo podem correr em sentidos opostos; a volta precisa ser única.
  if (std::abs(total[0] - 2. * loft_pi) < 1e-6)
    frame.sense = 1.;
  else if (std::abs(total[1] - 2. * loft_pi) < 1e-6)
    frame.sense = -1.;
  else
    return std::nullopt;
  return frame;
}
// Cone oblíquo do canto até o arco. Polos e pesos do arco se repetem no ápice, o que
// mantém as geratrizes retas e a borda circular exata.
TopoDS_Face corner_cone(const gp_Circ &circle, double from, double to,
                        const gp_Pnt &apex) {
  const auto arc = GeomConvert::CurveToBSplineCurve(new Geom_TrimmedCurve(
      new Geom_Circle(circle), std::min(from, to), std::max(from, to)));
  const int count = arc->NbPoles();
  TColgp_Array2OfPnt poles(1, count, 1, 2);
  TColStd_Array2OfReal weights(1, count, 1, 2);
  for (int i = 1; i <= count; ++i) {
    poles.SetValue(i, 1, arc->Pole(i));
    poles.SetValue(i, 2, apex);
    weights.SetValue(i, 1, arc->Weight(i));
    weights.SetValue(i, 2, arc->Weight(i));
  }
  TColStd_Array1OfReal knots(1, arc->NbKnots()), straight(1, 2);
  TColStd_Array1OfInteger multiplicities(1, arc->NbKnots()), ends(1, 2);
  arc->Knots(knots);
  arc->Multiplicities(multiplicities);
  straight.SetValue(1, 0.);
  straight.SetValue(2, 1.);
  ends.SetValue(1, 2);
  ends.SetValue(2, 2);
  const auto surface = new Geom_BSplineSurface(
      poles, weights, knots, straight, multiplicities, ends, arc->Degree(), 1);
  return BRepBuilderAPI_MakeFace(surface, 1e-7).Face();
}
TopoDS_Face polygon_face(const std::vector<gp_Pnt> &points) {
  BRepBuilderAPI_MakePolygon wire;
  for (const auto &point : points)
    wire.Add(point);
  wire.Close();
  return BRepBuilderAPI_MakeFace(wire.Wire(), true).Face();
}
TopoDS_Solid sewn_solid(BRepBuilderAPI_Sewing &sewing) {
  sewing.Perform();
  TopoDS_Shell shell;
  int shells = 0;
  for (TopExp_Explorer it(sewing.SewedShape(), TopAbs_SHELL); it.More();
       it.Next(), ++shells)
    shell = TopoDS::Shell(it.Current());
  if (shells != 1 || sewing.NbFreeEdges() != 0)
    throw std::runtime_error("As faces da transição não fecharam um sólido.");
  auto solid = BRepBuilderAPI_MakeSolid(shell).Solid();
  BRepLib::OrientClosedSolid(solid);
  return solid;
}
struct DevelopableBody {
  TopoDS_Solid solid;
  TopoDS_Shape start_cap, end_cap;
};
DevelopableBody developable_body(const DevelopableFrame &frame,
                                 const TopoDS_Wire &start,
                                 const TopoDS_Wire &end) {
  BRepBuilderAPI_Sewing sewing(1e-6);
  const auto start_cap = BRepBuilderAPI_MakeFace(start, true).Face();
  const auto end_cap = BRepBuilderAPI_MakeFace(end, true).Face();
  sewing.Add(start_cap);
  sewing.Add(end_cap);
  const auto &polygon = frame.polygon;
  const std::size_t n = polygon.size();
  for (std::size_t i = 0; i < n; ++i) {
    const std::size_t next = (i + 1) % n;
    sewing.Add(polygon_face(
        {polygon[i], polygon[next], circle_point(frame.circle, frame.angles[i])}));
    const double step = frame.arc(i);
    if (step > 1e-9)
      sewing.Add(corner_cone(frame.circle, frame.angles[i],
                             frame.angles[i] + frame.sense * step,
                             polygon[next]));
  }
  const auto solid = sewn_solid(sewing);
  return {solid, sewing.Modified(start_cap), sewing.Modified(end_cap)};
}

// Semiespaço normal·x ≤ limit.
struct HalfSpace {
  gp_XYZ normal;
  double limit;
};
// Poliedro convexo dos semiespaços. Cada face nasce de um quadrado grande no próprio plano,
// recortado pelos demais; os vértices são soldados antes de criar as arestas, para que
// faces vizinhas compartilhem exatamente os mesmos pontos.
TopoDS_Solid convex_polyhedron(const std::vector<HalfSpace> &spaces,
                               const gp_XYZ &center, double reach) {
  std::vector<gp_XYZ> welded;
  auto weld = [&](const gp_XYZ &point) {
    for (std::size_t i = 0; i < welded.size(); ++i)
      if ((welded[i] - point).Modulus() < 1e-6)
        return i;
    welded.push_back(point);
    return welded.size() - 1;
  };
  BRepBuilderAPI_Sewing sewing(1e-6);
  int faces = 0;
  for (std::size_t k = 0; k < spaces.size(); ++k) {
    const gp_XYZ normal = spaces[k].normal;
    const gp_XYZ origin = center + normal * (spaces[k].limit - normal * center);
    gp_XYZ u = normal ^ gp_XYZ(1., 0., 0.);
    if (u.Modulus() < 0.1)
      u = normal ^ gp_XYZ(0., 1., 0.);
    u.Normalize();
    const gp_XYZ v = normal ^ u;
    std::vector<gp_XYZ> outline = {
        origin - u * reach - v * reach, origin + u * reach - v * reach,
        origin + u * reach + v * reach, origin - u * reach + v * reach};
    for (std::size_t other = 0; other < spaces.size() && !outline.empty();
         ++other) {
      if (other == k)
        continue;
      const auto &cut = spaces[other];
      std::vector<gp_XYZ> kept;
      for (std::size_t i = 0; i < outline.size(); ++i) {
        const gp_XYZ &a = outline[i], &b = outline[(i + 1) % outline.size()];
        const double from = cut.normal * a - cut.limit;
        const double to = cut.normal * b - cut.limit;
        if (from <= 0.)
          kept.push_back(a);
        if ((from < 0. && to > 0.) || (from > 0. && to < 0.))
          kept.push_back(a + (b - a) * (from / (from - to)));
      }
      outline = std::move(kept);
    }
    std::vector<std::size_t> indices;
    for (const auto &point : outline) {
      const auto index = weld(point);
      if (indices.empty() || indices.back() != index)
        indices.push_back(index);
    }
    while (indices.size() > 1 && indices.front() == indices.back())
      indices.pop_back();
    if (indices.size() < 3)
      continue;
    std::vector<gp_Pnt> points;
    for (const auto index : indices)
      points.emplace_back(welded[index]);
    sewing.Add(polygon_face(points));
    ++faces;
  }
  if (faces < 4)
    throw std::runtime_error("Espessura grande demais para esta transição.");
  return sewn_solid(sewing);
}
} // namespace

// Casca de um corpo reconhecido como transição desenvolvível, com paredes uniformes e
// bordas no plano dos perfis. Devolve falso para qualquer outro corpo ou escolha de faces.
static bool developable_shell(Shape &output, const TopoDS_Shape &body,
                              const TopTools_ListOfShape &removed,
                              double thickness, bool outward) {
  std::optional<gp_Circ> circle;
  TopoDS_Face round_face;
  for (TopExp_Explorer it(body, TopAbs_FACE); it.More(); it.Next()) {
    const auto face = TopoDS::Face(it.Current());
    const auto found = circle_of(BRepTools::OuterWire(face));
    if (found && BRepAdaptor_Surface(face).GetType() == GeomAbs_Plane) {
      circle = found;
      round_face = face;
    }
  }
  if (!circle)
    return false;
  // A tampa poligonal é a única face reta que não encosta no círculo.
  std::optional<std::vector<gp_Pnt>> polygon;
  TopoDS_Face flat_face;
  for (TopExp_Explorer it(body, TopAbs_FACE); it.More(); it.Next()) {
    const auto face = TopoDS::Face(it.Current());
    if (BRepAdaptor_Surface(face).GetType() != GeomAbs_Plane)
      continue;
    const auto points = polygon_of(BRepTools::OuterWire(face));
    if (!points)
      continue;
    bool touches = false;
    const gp_Ax3 axis(circle->Position());
    for (const auto &point : *points) {
      const gp_Vec radial(circle->Location(), point);
      const double height = radial.Dot(gp_Vec(axis.Direction()));
      const double radius = (radial - gp_Vec(axis.Direction()) * height).Magnitude();
      touches = touches || (std::abs(height) < 1e-6 &&
                            std::abs(radius - circle->Radius()) < 1e-6);
    }
    if (touches)
      continue;
    if (polygon)
      return false;
    polygon = points;
    flat_face = face;
  }
  if (!polygon)
    return false;
  const auto frame = developable_frame(*polygon, *circle);
  if (!frame)
    return false;
  bool open_flat = false, open_round = false;
  for (const auto &face : removed) {
    if (face.IsSame(flat_face))
      open_flat = true;
    else if (face.IsSame(round_face))
      open_round = true;
    else
      return false;
  }
  // O corpo precisa ser mesmo o fecho convexo: outra operação pode ter mantido as tampas.
  const auto hull = developable_body(*frame, BRepTools::OuterWire(flat_face),
                                     BRepTools::OuterWire(round_face));
  GProp_GProps measured, expected;
  BRepGProp::VolumeProperties(body, measured);
  BRepGProp::VolumeProperties(hull.solid, expected);
  if (std::abs(std::abs(measured.Mass()) - expected.Mass()) >
      1e-6 * expected.Mass())
    return false;

  const auto center = expected.CentreOfMass().XYZ();
  double reach = 0.;
  for (const auto &point : frame->polygon)
    reach = std::max(reach, (point.XYZ() - center).Modulus());
  reach = std::max(reach, (frame->circle.Location().XYZ() - center).Modulus() +
                              frame->circle.Radius());
  reach = 4. * (reach + thickness);
  const double shift = outward ? thickness : -thickness;
  std::vector<HalfSpace> spaces;
  auto add_lateral = [&](const gp_Pnt &point, gp_XYZ normal) {
    normal.Normalize();
    if (normal * (center - point.XYZ()) > 0.)
      normal.Reverse();
    spaces.push_back({normal, normal * point.XYZ() + shift});
  };
  const std::size_t n = frame->polygon.size();
  for (std::size_t i = 0; i < n; ++i) {
    const auto &a = frame->polygon[i], &b = frame->polygon[(i + 1) % n];
    const auto tangency = circle_point(frame->circle, frame->angles[i]);
    add_lateral(a, gp_Vec(a, b).XYZ() ^ gp_Vec(a, tangency).XYZ());
    // Planos tangentes do cone, densos o bastante para o desvio pedido entre dois deles.
    const double step = frame->arc(i);
    const double slant = b.Distance(tangency) + thickness;
    const double spacing = 2. * std::acos(1. - hull_deviation / slant);
    const int count = std::clamp(static_cast<int>(std::ceil(step / spacing)), 1, 96);
    for (int k = 1; k < count; ++k) {
      const double angle =
          frame->angles[i] + frame->sense * step * k / count;
      const auto point = circle_point(frame->circle, angle);
      add_lateral(b, gp_Vec(b, point).XYZ() ^
                         circle_tangent(frame->circle, angle).XYZ());
    }
  }
  auto add_cap = [&](const gp_Pnt &point, gp_XYZ normal, bool open) {
    normal.Normalize();
    if (normal * (center - point.XYZ()) > 0.)
      normal.Reverse();
    // Abertura para dentro: o poliedro atravessa a tampa, sem faces coincidentes com ela.
    const double extra = open ? (outward ? 0. : thickness + 1.) : thickness;
    const double limit = normal * point.XYZ();
    spaces.push_back({normal, limit + ((open || outward) ? extra : -extra)});
  };
  add_cap(frame->polygon[0],
          gp_Vec(frame->polygon[0], frame->polygon[1]).XYZ() ^
              gp_Vec(frame->polygon[0], frame->polygon[2]).XYZ(),
          open_flat);
  add_cap(frame->circle.Location(), gp_XYZ(frame->circle.Axis().Direction().XYZ()),
          open_round);

  const auto offset = convex_polyhedron(spaces, center, reach);
  BRepAlgoAPI_Cut cut;
  TopTools_ListOfShape arguments, tools;
  arguments.Append(outward ? offset : body);
  tools.Append(outward ? body : offset);
  cut.SetArguments(arguments);
  cut.SetTools(tools);
  cut.Build();
  if (!cut.IsDone() || cut.HasErrors())
    throw std::runtime_error("Falha ao criar casca.");
  output.value = cut.Shape();
  output.origins = history(cut, output.value, {&body});
  return true;
}

extern "C" Shape *nator_loft_developable(const Shape *first, const Shape *last,
                                         std::size_t wire) noexcept {
  return build([&](Shape &output) {
    const auto start = transition_wire(first, wire),
               end = transition_wire(last, wire);
    const auto polygon_at_start = polygon_of(start);
    const auto polygon = polygon_at_start ? polygon_at_start : polygon_of(end);
    const auto circle = circle_of(polygon_at_start ? end : start);
    const auto frame = polygon && circle
                           ? developable_frame(*polygon, *circle)
                           : std::optional<DevelopableFrame>();
    if (!frame)
      throw std::runtime_error(
          "A transição desenvolvível liga um polígono convexo a um círculo.");
    const auto body = developable_body(*frame, start, end);
    output.value = body.solid;
    OriginTable names(output.value);
    names.mark_all(body.start_cap, StartCap, 0, 0);
    names.mark_all(body.end_cap, EndCap, 0, 0);
    output.origins = names.take();
  });
}
