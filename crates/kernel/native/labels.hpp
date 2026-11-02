// Rótulos: fontes do sistema viram contornos CAD unitários, persistíveis sem
// depender da fonte instalada na reabertura. A projeção é local na face
// escolhida.
#pragma once
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepLib.hxx>
#include <BRepPrimAPI_MakeHalfSpace.hxx>
#include <Font_FontMgr.hxx>
#include <GeomAPI_ProjectPointOnSurf.hxx>
#include <StdPrs_BRepFont.hxx>
#include <StdPrs_BRepTextBuilder.hxx>
#include <TColStd_SequenceOfHAsciiString.hxx>
#include <mutex>
namespace {
std::mutex label_font_mutex;
thread_local std::string label_fonts;
struct LabelFrame {
  gp_Pnt point;
  gp_Dir u, v, normal;
  double pu, pv;
};
LabelFrame label_frame(const Shape *body, std::size_t index,
                       const double *point, const double *uv, bool fixed) {
  const auto& faces = faces_of(body);
  if (index >= static_cast<std::size_t>(faces.Extent()))
    throw std::runtime_error("Face do rótulo inexistente.");
  const auto face = TopoDS::Face(faces(static_cast<int>(index) + 1));
  double u = uv[0], v = uv[1];
  if (!fixed) {
    double u0, u1, v0, v1;
    BRepTools::UVBounds(face, u0, u1, v0, v1);
    TopLoc_Location location;
    const auto surface = BRep_Tool::Surface(face, location);
    const auto query = gp_Pnt(point[0], point[1], point[2])
                           .Transformed(location.Transformation().Inverted());
    GeomAPI_ProjectPointOnSurf projection(query, surface, u0, u1, v0, v1, 1e-7);
    if (!projection.NbPoints())
      throw std::runtime_error(
          "Não foi possível localizar o ponto do rótulo na superfície.");
    projection.LowerDistanceParameters(u, v);
  }
  BRepClass_FaceClassifier inside(face, gp_Pnt2d(u, v), 1e-6);
  if (inside.State() != TopAbs_IN && inside.State() != TopAbs_ON)
    throw std::runtime_error(
        "Posicione o rótulo dentro da face escolhida, fora de seus furos.");
  gp_Pnt position;
  gp_Vec du, dv;
  BRepAdaptor_Surface(face).D1(u, v, position, du, dv);
  auto normal = du.Crossed(dv);
  if (normal.SquareMagnitude() < 1e-20 || du.SquareMagnitude() < 1e-20)
    throw std::runtime_error(
        "Escolha um ponto regular da superfície, afastado dos polos.");
  if (face.Orientation() == TopAbs_REVERSED)
    normal.Reverse();
  const gp_Dir n(normal), x(du);
  const gp_Dir y(gp_Vec(n).Crossed(gp_Vec(x)));
  return {position, x, y, n, u, v};
}
TopoDS_Shape label_positive(const TopoDS_Shape &shape) {
  TopoDS_Compound compound;
  BRep_Builder builder;
  builder.MakeCompound(compound);
  TopoDS_Solid only;
  int count = 0;
  for (TopExp_Explorer it(shape, TopAbs_SOLID); it.More(); it.Next()) {
    auto solid = TopoDS::Solid(it.Current());
    if (!BRepLib::OrientClosedSolid(solid))
      throw std::runtime_error(
          "As letras não formam um volume fechado orientável.");
    builder.Add(compound, solid);
    only = solid;
    ++count;
  }
  if (!count)
    throw std::runtime_error("As letras não formam um volume fechado.");
  return count == 1 ? TopoDS_Shape(only) : TopoDS_Shape(compound);
}
TopoDS_Shape label_union(const TopoDS_Shape &a, const TopoDS_Shape &b) {
  if (a.IsNull())
    return b;
  BRepAlgoAPI_Fuse operation(a, b);
  if (!operation.IsDone())
    throw std::runtime_error("Não foi possível unir os contornos do rótulo.");
  return operation.Shape();
}
} // namespace
extern "C" {
std::size_t nator_font_names_size() noexcept {
  std::size_t size = 0;
  guarded([&] {
    std::lock_guard<std::mutex> guard(label_font_mutex);
    TColStd_SequenceOfHAsciiString names;
    Font_FontMgr::GetInstance()->GetAvailableFontsNames(names);
    label_fonts.clear();
    for (int i = 1; i <= names.Length(); ++i) {
      label_fonts += names(i)->String().ToCString();
      label_fonts += '\n';
    }
    if (label_fonts.size() > 1024 * 1024)
      throw std::runtime_error("Lista de fontes excessiva.");
    size = label_fonts.size();
  });
  return size;
}
bool nator_font_names_copy(char *output, std::size_t length) noexcept {
  return guarded([&] {
    if (length != label_fonts.size())
      throw std::runtime_error("Lista de fontes mudou.");
    std::copy(label_fonts.begin(), label_fonts.end(), output);
  });
}
Shape *nator_text_outline(const char *text, const char *family, bool bold,
                          bool italic, bool underline) noexcept {
  return build([&](Shape &output) {
    std::lock_guard<std::mutex> guard(label_font_mutex);
    const auto aspect =
        bold ? (italic ? Font_FontAspect_BoldItalic : Font_FontAspect_Bold)
             : (italic ? Font_FontAspect_Italic : Font_FontAspect_Regular);
    const auto strict = std::string(family) == "sans-serif"
                            ? Font_StrictLevel_Any
                            : Font_StrictLevel_Strict;
    const auto font = StdPrs_BRepFont::FindAndCreate(
        TCollection_AsciiString(family), aspect, 1., strict);
    if (font.IsNull())
      throw std::runtime_error(
          "A fonte ou o estilo escolhido não está disponível neste sistema.");
    StdPrs_BRepTextBuilder builder;
    const auto letters =
        builder.Perform(*font, NCollection_String(text), gp_Ax3(),
                        Graphic3d_HTA_CENTER, Graphic3d_VTA_CENTER);
    if (letters.IsNull() || map_of(letters, TopAbs_FACE).IsEmpty())
      throw std::runtime_error("O texto não produziu contornos nesta fonte.");
    output.value = letters;
    if (underline) {
      Bnd_Box bounds;
      BRepBndLib::Add(letters, bounds);
      double x0, y0, z0, x1, y1, z1;
      bounds.Get(x0, y0, z0, x1, y1, z1);
      BRepBuilderAPI_MakePolygon line;
      for (const auto &p :
           {gp_Pnt(x0, y0 - 0.12, 0.), gp_Pnt(x1, y0 - 0.12, 0.),
            gp_Pnt(x1, y0 - 0.06, 0.), gp_Pnt(x0, y0 - 0.06, 0.)})
        line.Add(p);
      line.Close();
      TopoDS_Compound compound;
      BRep_Builder combine;
      combine.MakeCompound(compound);
      combine.Add(compound, letters);
      combine.Add(compound, BRepBuilderAPI_MakeFace(line.Wire()).Face());
      output.value = compound;
    }
  });
}
bool nator_surface_frame(const Shape *body, std::size_t index,
                         const double *point, const double *uv, bool fixed,
                         double *output) noexcept {
  return guarded([&] {
    const auto f = label_frame(body, index, point, uv, fixed);
    const double values[] = {f.point.X(), f.point.Y(), f.point.Z(), f.u.X(),
                             f.u.Y(),     f.u.Z(),     f.v.X(),     f.v.Y(),
                             f.v.Z(),     f.pu,        f.pv};
    std::copy(std::begin(values), std::end(values), output);
  });
}
Shape *nator_label(const Shape *body, std::size_t index, const Shape *outline,
                   const double *point, const double *uv, bool fixed,
                   double size, double angle, double depth, int mode) noexcept {
  return build([&](Shape &output) {
    const auto& all = faces_of(body);
    const auto f = label_frame(body, index, point, uv, fixed);
    const auto host = TopoDS::Face(all(static_cast<int>(index) + 1));
    const gp_Vec x =
        gp_Vec(f.u) * std::cos(angle) + gp_Vec(f.v) * std::sin(angle);
    const gp_Vec y =
        gp_Vec(f.v) * std::cos(angle) - gp_Vec(f.u) * std::sin(angle);
    gp_Trsf placement;
    placement.SetValues(x.X() * size, y.X() * size, f.normal.X() * size,
                        f.point.X(), x.Y() * size, y.Y() * size,
                        f.normal.Y() * size, f.point.Y(), x.Z() * size,
                        y.Z() * size, f.normal.Z() * size, f.point.Z());
    const auto flat = map_of(outline->value, TopAbs_FACE);
    if (flat.IsEmpty() || flat.Extent() > 4096)
      throw std::runtime_error("Contornos do texto vazios ou excessivos.");
    Bnd_Box bounds;
    BRepBndLib::Add(body->value, bounds);
    const double span = std::sqrt(bounds.SquareExtent()) * 2. + size * 2. + 1.;
    // Superfícies periódicas podem ser atingidas em dois lados pelo prisma.
    // Limitamos a projeção à vizinhança do apoio, separada pelas meias
    // distâncias às próximas interseções da mesma face ao longo da normal.
    std::vector<TopoDS_Shape> clips;
    IntCurvesFace_ShapeIntersector ray;
    ray.Load(host, 1e-7);
    ray.Perform(gp_Lin(f.point, f.normal), -span, span);
    double before = -span * 2., after = span * 2.;
    for (int hit = 1; hit <= ray.NbPnt(); ++hit) {
      const double t = ray.WParameter(hit);
      if (t < -1e-5)
        before = std::max(before, t);
      if (t > 1e-5)
        after = std::min(after, t);
    }
    for (const double t : {before, after})
      if (std::abs(t) < span * 1.5) {
        const auto middle = f.point.Translated(gp_Vec(f.normal) * (t * 0.5));
        const auto plane =
            BRepBuilderAPI_MakeFace(gp_Pln(middle, f.normal)).Face();
        clips.push_back(BRepPrimAPI_MakeHalfSpace(plane, f.point).Solid());
      }
    TopoDS_Shape mask;
    for (int i = 1; i <= flat.Extent(); ++i) {
      const auto face = TopoDS::Face(flat(i));
      BRepAdaptor_Surface surface(face);
      if (surface.GetType() != GeomAbs_Plane ||
          std::abs(surface.Plane().Axis().Direction().Z()) < 1. - 1e-8 ||
          surface.Plane().Distance(gp_Pnt()) > 1e-6)
        throw std::runtime_error("Contorno de fonte inválido: o texto deve "
                                 "estar no plano XY unitário.");
      auto placed = BRepBuilderAPI_Transform(face, placement, true).Shape();
      gp_Trsf shift;
      shift.SetTranslation(gp_Vec(f.normal) * (-span));
      placed = BRepBuilderAPI_Transform(placed, shift, true).Shape();
      BRepPrimAPI_MakePrism prism(placed, gp_Vec(f.normal) * (2. * span),
                                  false);
      if (!prism.IsDone())
        throw std::runtime_error("Não foi possível projetar o texto na face.");
      mask = label_union(mask, label_positive(prism.Shape()));
    }
    for (const auto &clip : clips) {
      BRepAlgoAPI_Common common(mask, clip);
      if (!common.IsDone())
        throw std::runtime_error(
            "Não foi possível delimitar a projeção do rótulo.");
      mask = common.Shape();
    }
    if (mode == 0) {
      Shape tool;
      tool.value = mask;
      const Shape *tools[] = {&tool};
      auto result =
          std::unique_ptr<Shape>(nator_painted_solid(body, &index, tools, 1));
      if (!result)
        throw std::runtime_error(last_error);
      output.value = result->value;
      output.origins = result->origins;
      return;
    }
    BRepAlgoAPI_Common projection(host, mask);
    if (!projection.IsDone())
      throw std::runtime_error(
          "Não foi possível recortar as letras na superfície.");
    TopoDS_Shape tools;
    for (TopExp_Explorer it(projection.Shape(), TopAbs_FACE); it.More();
         it.Next()) {
      auto patch = TopoDS::Face(it.Current());
      GProp_GProps area;
      BRepGProp::SurfaceProperties(patch, area);
      if (area.Mass() < 1e-12)
        continue;
      patch.Orientation(host.Orientation());
      BRepOffsetAPI_MakeThickSolid thick;
      thick.MakeThickSolidBySimple(patch, mode == 1 ? depth : -depth);
      if (!thick.IsDone())
        throw std::runtime_error(
            "Não foi possível dar espessura às letras nesta superfície.");
      tools = label_union(tools, label_positive(thick.Shape()));
    }
    if (tools.IsNull())
      throw std::runtime_error("O texto não alcança a face escolhida.");
    Shape tool;
    tool.value = tools;
    if (mode == 1) {
      BRepAlgoAPI_Fuse operation(body->value, tools);
      if (!operation.IsDone())
        throw std::runtime_error("Não foi possível criar o relevo do rótulo.");
      output.value = operation.Shape();
      output.origins = history(operation, output.value, {&body->value, &tools});
    } else {
      BRepAlgoAPI_Cut operation(body->value, tools);
      if (!operation.IsDone())
        throw std::runtime_error("Não foi possível gravar o rótulo no corpo.");
      output.value = operation.Shape();
      output.origins = history(operation, output.value, {&body->value, &tools});
    }
    // Mesmo contrato da partição de aparência: origem do corpo em [1] e
    // marcador de texto em [2]. Os nomes finais são atribuídos em Rust.
    for (std::size_t i = 0; i < output.origins.size(); i += 3) {
      const int kind = output.origins[i], input = output.origins[i + 1],
                source = output.origins[i + 2];
      const bool original = (kind == Same || kind == Modified) && input == 0;
      output.origins[i] = 0;
      output.origins[i + 1] = original ? source : -1;
      output.origins[i + 2] = original ? 0 : 1;
    }
  });
}
}
