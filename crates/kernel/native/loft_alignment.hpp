// Correspondência geométrica dos contornos circulares. Repartir o círculo nas
// direções dos vértices evita que sua costura arbitrária torça um Loft.
#pragma once
#include <optional>
namespace {
const double loft_pi = std::acos(-1.);
struct LoftLoop {
  std::vector<gp_Pnt> vertices;
  std::vector<double> lengths;
  std::optional<gp_Circ> circle;
  bool reversed = false;
};
LoftLoop loft_loop(const TopoDS_Wire &wire) {
  LoftLoop loop;
  for (BRepTools_WireExplorer it(wire); it.More(); it.Next()) {
    const auto edge = it.Current();
    loop.vertices.push_back(BRep_Tool::Pnt(it.CurrentVertex()));
    GProp_GProps length;
    BRepGProp::LinearProperties(edge, length);
    loop.lengths.push_back(length.Mass());
    BRepAdaptor_Curve curve(edge);
    if (loop.vertices.size() == 1 && curve.GetType() == GeomAbs_Circle &&
        std::abs(curve.LastParameter() - curve.FirstParameter() -
                 2. * loft_pi) < 1e-7) {
      loop.circle = curve.Circle();
      loop.reversed = edge.Orientation() == TopAbs_REVERSED;
    } else
      loop.circle.reset();
  }
  if (loop.circle) {
    const auto circle = *loop.circle;
    loop.vertices.clear();
    loop.lengths.assign(4, loft_pi * circle.Radius() / 2.);
    for (int i = 0; i < 4; ++i) {
      const double t = (loop.reversed ? -1. : 1.) * i * loft_pi / 2.;
      loop.vertices.push_back(circle.Location().Translated(
          gp_Vec(circle.XAxis().Direction()) * (circle.Radius() * std::cos(t)) +
          gp_Vec(circle.YAxis().Direction()) *
              (circle.Radius() * std::sin(t))));
    }
  }
  return loop;
}
TopoDS_Wire loft_circle(const LoftLoop &circular, const LoftLoop &reference) {
  const auto &circle = *circular.circle;
  const std::size_t n = reference.vertices.size();
  if (n < 2 || n > 1024)
    throw std::runtime_error(
        "Contorno de referência do Loft vazio ou excessivo.");
  gp_XYZ sum(0., 0., 0.);
  for (const auto &p : reference.vertices)
    sum += p.XYZ();
  const gp_Pnt center(sum / static_cast<double>(n));
  std::vector<double> angles;
  for (const auto &point : reference.vertices) {
    const gp_Vec d(center, point);
    angles.push_back(std::atan2(d.Dot(gp_Vec(circle.YAxis().Direction())),
                                d.Dot(gp_Vec(circle.XAxis().Direction()))));
  }
  auto wrap = [](double t) {
    t = std::fmod(t, 2. * loft_pi);
    return t < 0. ? t + 2. * loft_pi : t;
  };
  double positive = 0., negative = 0.;
  bool distinct = true;
  for (std::size_t i = 0; i < n; ++i) {
    const double step = wrap(angles[(i + 1) % n] - angles[i]);
    positive += step;
    negative += wrap(-step);
    distinct = distinct && step > 1e-8;
  }
  double sign = positive <= negative ? 1. : -1.;
  const bool ordered =
      distinct && std::abs(std::min(positive, negative) - 2. * loft_pi) < 1e-6;
  // Perfis côncavos podem voltar sobre o mesmo ângulo: usar a fração do
  // perímetro preserva a ordem, em vez de criar arcos sobrepostos.
  double perimeter = 0.;
  for (double length : reference.lengths)
    perimeter += length;
  if (!(perimeter > 1e-10))
    throw std::runtime_error("Contorno do Loft degenerado.");
  const double phase = angles[0];
  double parameter = phase;
  BRepBuilderAPI_MakeWire wire;
  for (std::size_t i = 0; i < n; ++i) {
    const double step = ordered
                            ? wrap(sign * (angles[(i + 1) % n] - angles[i]))
                            : 2. * loft_pi * reference.lengths[i] / perimeter;
    const double next = parameter + sign * step;
    TopoDS_Edge edge = BRepBuilderAPI_MakeEdge(
        circle, std::min(parameter, next), std::max(parameter, next));
    if (sign < 0.)
      edge.Reverse();
    wire.Add(edge);
    parameter = next;
  }
  if (!wire.IsDone())
    throw std::runtime_error("Não foi possível alinhar o círculo no Loft.");
  return wire.Wire();
}
std::pair<TopoDS_Wire, TopoDS_Wire> loft_aligned_wires(TopoDS_Wire first,
                                                       TopoDS_Wire last) {
  const auto a = loft_loop(first), b = loft_loop(last);
  if (a.circle && b.circle) {
    first = loft_circle(a, a);
    last = loft_circle(b, a);
  } else if (a.circle)
    first = loft_circle(a, b);
  else if (b.circle)
    last = loft_circle(b, a);
  return {first, last};
}
} // namespace
