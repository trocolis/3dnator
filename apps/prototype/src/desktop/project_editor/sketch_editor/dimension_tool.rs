//! Ferramenta Cota, no modelo da Dimension do SolidCraft: clique no que medir. Assim que as
//! escolhas determinam a cota (uma linha, um círculo ou arco, ou o segundo ponto ou linha de
//! uma distância ou ângulo), a caixa de valor recebe o teclado com a medida atual
//! selecionada, e digitar a substitui. Enquanto se digita, o texto acompanha o cursor; Enter
//! posiciona e confirma em uma ação de desfazer; o editor volta à seleção com Esc. Antes de digitar, um clique no
//! vazio fixa o texto e um clique em outro ponto ou linha transforma a cota em distância ou
//! ângulo; Tab devolve as escolhas para um clique de posicionamento. Entre dois pontos (ou
//! numa linha) fora dos eixos, o lugar do texto escolhe o que se mede: a distância entre
//! eles, a horizontal ou a vertical. Uma cota que a geometria já fixada não comporta entra
//! como cota de referência, entre parênteses.
//!
//! Também desenha todas as cotas do rascunho como cotas: linhas de extensão, linha de cota
//! com setas e o valor; raios e diâmetros com R/Ø; ângulos como arco entre as linhas.
use super::*;
use nator_document::parameters::{Dimension, Parameters};
use nator_sketch::{add, cross, distance, dot, scale, sub};

/// Folga entre a geometria e o lugar padrão do texto, em pontos de tela.
const DEFAULT_GAP: f32 = 24.;
/// Alcance do clique em pontos e curvas, em pontos de tela.
const PICK_RADIUS: f32 = 8.;
/// Abaixo deste seno, duas linhas são tratadas como paralelas (cerca de 0,5°).
const PARALLEL: f64 = 0.0087;

/// Elemento escolhido: um ponto, uma linha ou um círculo/arco.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Pick {
    Point(u64),
    Line(u64),
    Radial(u64),
}

/// O que as escolhas medem; o lugar do texto completa a variante (alinhada, horizontal,
/// vertical) e, nos ângulos, o setor entre as linhas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Measure {
    /// Dois pontos: os extremos de uma linha ou dois pontos escolhidos.
    Points(u64, u64),
    Radial {
        circle: u64,
        diameter: bool,
    },
    PointLine {
        point: u64,
        line: u64,
    },
    LineCircle {
        line: u64,
        circle: u64,
    },
    Angle {
        first: u64,
        second: u64,
    },
}

/// Cota pronta para entrar no rascunho, com o valor medido na geometria atual.
#[derive(Debug, Clone)]
pub(super) struct Built {
    pub kind: ConstraintKind,
    pub measured: f64,
    /// Referência da posição do texto gravada em `dimension_labels`.
    pub anchor: Point2,
}

/// Rascunho da ferramenta, pertencente à interface até o Enter.
#[derive(Default)]
pub(super) struct Draft {
    picks: Vec<Pick>,
    text: String,
    typed: bool,
    /// Lugar do texto fixado por um clique, em mm no plano.
    pinned: Option<Point2>,
    /// Última posição do cursor no plano, para o texto que o acompanha.
    follow: Option<Point2>,
    /// A caixa de valor tem o teclado (não foi devolvida com Tab).
    editing: bool,
    focus: bool,
    /// Ponteiro na tela quando a cota ficou determinada: sem movê-lo, o texto fica no
    /// lugar padrão.
    settled_at: Option<egui::Pos2>,
    moved: bool,
}
impl Draft {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn editing(&self) -> bool {
        self.editing
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn picks(&self) -> usize {
        self.picks.len()
    }
    /// Uma curva selecionada, ou duas linhas compatíveis, já determina a cota.
    /// Seleções ambíguas deixam a ferramenta aguardando os próximos cliques.
    pub(super) fn seed(&mut self, sketch: &Sketch, selected: &[u64]) {
        if !(1..=2).contains(&selected.len()) {
            return;
        }
        let picks = selected
            .iter()
            .filter_map(|id| {
                sketch.entity(*id).ok().map(|entity| match entity.curve {
                    Curve::Line { .. } => Pick::Line(*id),
                    Curve::Circle { .. } | Curve::Arc { .. } => Pick::Radial(*id),
                })
            })
            .collect::<Vec<_>>();
        if picks.len() == selected.len()
            && settle(sketch, &picks).is_ok_and(|measure| measure.is_some())
        {
            self.picks = picks;
            self.editing = true;
            self.focus = true;
        }
    }
    fn label(&self) -> Option<Point2> {
        self.pinned.or(if self.moved { self.follow } else { None })
    }
}

/// O que a ferramenta pede ao editor neste quadro.
#[derive(Default)]
pub(super) struct Update {
    /// Rascunho com a cota nova e o identificador dela, para decidir no trabalhador se ela
    /// dirige a geometria ou entra como referência.
    pub commit: Option<(Sketch, u64)>,
    pub warning: Option<String>,
    /// Orientação da barra de estado, emitida só quando a etapa muda.
    pub hint: Option<T>,
}

/// Pontos primeiro: perto de um extremo ou centro, o clique escolhe o ponto, como no
/// SolidCraft; longe deles, a curva sob o cursor.
pub(super) fn pick(sketch: &Sketch, cursor: Point2, tolerance: f64) -> Option<Pick> {
    let point = sketch
        .visible_points()
        .map(|p| (p.id, distance(p.position, cursor)))
        .filter(|(_, d)| *d <= tolerance)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((id, _)) = point {
        return Some(Pick::Point(id));
    }
    let id = nator_sketch::tools::pick_entity(sketch, cursor, tolerance)
        .ok()
        .flatten()?;
    Some(match sketch.entity(id).ok()?.curve {
        Curve::Line { .. } => Pick::Line(id),
        _ => Pick::Radial(id),
    })
}

fn unit(v: Point2) -> Option<Point2> {
    let length = distance(v, [0., 0.]);
    (length > 1e-12).then(|| scale(v, 1. / length))
}
fn line_direction(sketch: &Sketch, line: u64) -> Option<Point2> {
    let (a, b) = sketch.line(line).ok()?;
    unit(sub(b, a))
}
/// Pé da perpendicular de `point` sobre a reta suporte da linha.
fn foot(sketch: &Sketch, point: Point2, line: u64) -> Option<Point2> {
    let (a, _) = sketch.line(line).ok()?;
    let d = line_direction(sketch, line)?;
    Some(add(a, scale(d, dot(sub(point, a), d))))
}
/// Encontro das retas suporte das duas linhas, se não forem paralelas.
fn vertex(sketch: &Sketch, first: u64, second: u64) -> Option<Point2> {
    let (a, _) = sketch.line(first).ok()?;
    let (b, _) = sketch.line(second).ok()?;
    let (d1, d2) = (
        line_direction(sketch, first)?,
        line_direction(sketch, second)?,
    );
    let det = cross(d1, d2);
    (det.abs() > 1e-12).then(|| add(a, scale(d1, cross(sub(b, a), d2) / det)))
}
/// Ponto da borda do círculo mais próximo da reta (ou o que passa dela, se a corta).
fn edge_toward(center: Point2, radius: f64, foot: Point2) -> Option<Point2> {
    Some(add(center, scale(unit(sub(foot, center))?, radius)))
}

/// Combina as escolhas: `Ok(None)` espera outra escolha e `Err` recusa a combinação.
pub(super) fn settle(sketch: &Sketch, picks: &[Pick]) -> Result<Option<Measure>, ()> {
    use Pick::*;
    let entity = |id| sketch.entity(id).ok();
    let endpoints = |id: u64| match entity(id).map(|e| (e.curve.clone(), e.infinite)) {
        Some((Curve::Line { start, end }, false)) => Some((start, end)),
        _ => None,
    };
    let own = |line: u64, point: u64| entity(line).is_some_and(|e| e.point_ids().contains(&point));
    Ok(match *picks {
        [] | [Point(_)] => None,
        // Reta infinita não tem comprimento: espera o segundo ponto ou linha.
        [Line(line)] => endpoints(line).map(|(a, b)| Measure::Points(a, b)),
        [Radial(circle)] => Some(Measure::Radial {
            circle,
            diameter: matches!(entity(circle).map(|e| &e.curve), Some(Curve::Circle { .. })),
        }),
        [Point(a), Point(b)] if a != b => Some(Measure::Points(a, b)),
        [Point(point), Line(line)] | [Line(line), Point(point)] if !own(line, point) => {
            Some(Measure::PointLine { point, line })
        }
        [Line(first), Line(second)] if first != second => {
            let (Some(d1), Some(d2)) = (
                line_direction(sketch, first),
                line_direction(sketch, second),
            ) else {
                return Err(());
            };
            if cross(d1, d2).abs() < PARALLEL {
                // Linhas paralelas: a distância entre elas, de um extremo da segunda.
                let (point, _) = endpoints(second)
                    .or_else(|| match entity(second).map(|e| e.curve.clone()) {
                        Some(Curve::Line { start, end }) => Some((start, end)),
                        _ => None,
                    })
                    .ok_or(())?;
                Some(Measure::PointLine { point, line: first })
            } else {
                Some(Measure::Angle { first, second })
            }
        }
        [Line(line), Radial(circle)] | [Radial(circle), Line(line)] => {
            Some(Measure::LineCircle { line, circle })
        }
        _ => return Err(()),
    })
}

/// Monta a cota no lugar do texto (`label`, em mm no plano; `None` usa o padrão).
pub(super) fn build(
    sketch: &Sketch,
    measure: Measure,
    label: Option<Point2>,
) -> Result<Built, String> {
    let missing = || text(T::DimensionUnsupported).to_owned();
    let (kind, anchor) = match measure {
        Measure::Points(a, b) => {
            let (pa, pb) = (
                sketch.point(a).map_err(sketch_error)?,
                sketch.point(b).map_err(sketch_error)?,
            );
            let mid = scale(add(pa, pb), 0.5);
            let delta = sub(pb, pa);
            let length = distance(pa, pb);
            let aligned = ConstraintKind::Distance { a, b, value: 0. };
            let axis_aligned =
                delta[0].abs() < 1e-9 * length.max(1.) || delta[1].abs() < 1e-9 * length.max(1.);
            let kind = match label {
                Some(t) if !axis_aligned => {
                    let d = scale(delta, 1. / length);
                    let along = dot(sub(t, mid), d);
                    let within = |i: usize| {
                        t[i] >= pa[i].min(pb[i]) - 1e-9 && t[i] <= pa[i].max(pb[i]) + 1e-9
                    };
                    // De frente para os pontos, a distância entre eles; acima ou abaixo, a
                    // horizontal; ao lado, a vertical. Pontos em ordem crescente na direção
                    // medida mantêm o valor positivo.
                    let ordered = |i: usize| if pa[i] <= pb[i] { (a, b) } else { (b, a) };
                    if along.abs() <= length * 0.5 {
                        aligned
                    } else if within(0) {
                        let (a, b) = ordered(0);
                        ConstraintKind::DeltaX { a, b, value: 0. }
                    } else if within(1) {
                        let (a, b) = ordered(1);
                        ConstraintKind::DeltaY { a, b, value: 0. }
                    } else {
                        aligned
                    }
                }
                _ => aligned,
            };
            (kind, mid)
        }
        Measure::Radial { circle, diameter } => {
            let (center, _) = sketch.radial(circle).map_err(sketch_error)?;
            let kind = if diameter {
                ConstraintKind::Diameter { circle, value: 0. }
            } else {
                ConstraintKind::Radius { circle, value: 0. }
            };
            (kind, center)
        }
        Measure::PointLine { point, line } => {
            let p = sketch.point(point).map_err(sketch_error)?;
            let f = foot(sketch, p, line).ok_or_else(missing)?;
            (
                ConstraintKind::PointLineDistance {
                    point,
                    line,
                    value: 0.,
                },
                scale(add(p, f), 0.5),
            )
        }
        Measure::LineCircle { line, circle } => {
            let (center, radius) = sketch.radial(circle).map_err(sketch_error)?;
            let f = foot(sketch, center, line).ok_or_else(missing)?;
            let crossing = distance(center, f) < radius;
            let edge = edge_toward(center, radius, f).ok_or_else(missing)?;
            (
                ConstraintKind::LineCircleDistance {
                    line,
                    circle,
                    value: 0.,
                    crossing,
                },
                scale(add(f, edge), 0.5),
            )
        }
        Measure::Angle { first, second } => {
            let at = vertex(sketch, first, second).ok_or_else(missing)?;
            let (d1, d2) = (
                line_direction(sketch, first).ok_or_else(missing)?,
                line_direction(sketch, second).ok_or_else(missing)?,
            );
            let side = |value: f64| if value < 0. { -1. } else { 1. };
            // O setor que contém o texto define os lados; sem texto, os lados apontam
            // para o corpo de cada linha.
            let (s1, s2) = match label {
                Some(t) => {
                    let w = sub(t, at);
                    let det = cross(d1, d2);
                    (side(cross(w, d2) / det), side(cross(d1, w) / det))
                }
                None => {
                    let body = |line| {
                        sketch
                            .line(line)
                            .map(|(a, b)| scale(add(a, b), 0.5))
                            .map_err(sketch_error)
                    };
                    (
                        side(dot(sub(body(first)?, at), d1)),
                        side(dot(sub(body(second)?, at), d2)),
                    )
                }
            };
            let (u, v) = (scale(d1, s1), scale(d2, s2));
            // O valor guardado é o ângulo positivo do setor: a referência é o lado de
            // onde o setor parte no sentido anti-horário.
            let kind = if cross(u, v) >= 0. {
                ConstraintKind::LineAngle {
                    line: second,
                    reference: first,
                    value: 0.,
                    flip_line: s2 < 0.,
                    flip_reference: s1 < 0.,
                }
            } else {
                ConstraintKind::LineAngle {
                    line: first,
                    reference: second,
                    value: 0.,
                    flip_line: s1 < 0.,
                    flip_reference: s2 < 0.,
                }
            };
            (kind, at)
        }
    };
    let measured = sketch
        .measure(&kind)
        .map_err(sketch_error)?
        .ok_or_else(missing)?;
    let mut kind = kind;
    if let Some(value) = kind.dimension_value_mut() {
        *value = measured;
    }
    Ok(Built {
        kind,
        measured,
        anchor,
    })
}

/// Texto da caixa com a medida atual, na formatação da interface.
fn measured_text(kind: &ConstraintKind, measured: f64) -> String {
    if kind.is_angular() {
        format!("{} deg", nator_drawing::format_mm(measured.to_degrees()))
    } else {
        format!("{} mm", nator_drawing::format_mm(measured))
    }
}

/// Valor digitado, nas unidades internas, ou o motivo da recusa.
fn typed_value(kind: &ConstraintKind, input: &str, parameters: &Parameters) -> Result<f64, String> {
    let angular = kind.is_angular();
    let value = parameters
        .evaluate(
            input.trim(),
            if angular {
                Dimension::Angle
            } else {
                Dimension::Length
            },
        )
        .map_err(|e| e.to_string())?;
    let valid = if angular {
        value > 0. && value < std::f64::consts::PI
    } else if matches!(
        kind,
        ConstraintKind::Radius { .. } | ConstraintKind::Diameter { .. }
    ) {
        value > 0.
    } else {
        value >= 0.
    };
    if valid {
        Ok(value)
    } else {
        Err(text(T::DimensionInvalidValue).to_owned())
    }
}

/// Rascunho com a cota confirmada: o valor digitado (ou o medido, exato, sem
/// arredondamento da caixa), a expressão e a posição do texto.
pub(super) fn commit(
    sketch: &Sketch,
    built: &Built,
    typed: Option<&str>,
    label: Option<Point2>,
    parameters: &Parameters,
) -> Result<(Sketch, u64), String> {
    let mut kind = built.kind.clone();
    let (value, expression) = match typed {
        Some(text) => (
            typed_value(&kind, text, parameters)?,
            text.trim().to_owned(),
        ),
        None => (
            built.measured,
            if kind.is_angular() {
                format!("{} deg", built.measured.to_degrees())
            } else {
                format!("{} mm", built.measured)
            },
        ),
    };
    if let Some(target) = kind.dimension_value_mut() {
        *target = value;
    }
    let mut next = sketch.clone();
    let id = next.add_constraint(kind, false).map_err(sketch_error)?;
    next.dimension_expressions.insert(id, expression);
    if let Some(label) = label {
        next.dimension_labels.insert(id, sub(label, built.anchor));
    }
    next.validate().map_err(sketch_error)?;
    Ok((next, id))
}

/// Decide, no trabalhador, se a cota nova dirige a geometria: medida no lugar, ela precisa
/// tirar um grau de liberdade. Se a geometria já está fixada por outras restrições, a cota
/// passa a apenas medir (referência, entre parênteses), sem valor nem expressão.
pub(super) fn settle_driven(
    sketch: &mut Sketch,
    id: u64,
    parameters: &Parameters,
) -> Result<(), String> {
    let freedom = |status: &Status| match status {
        Status::UnderConstrained { degrees_of_freedom } => Some(*degrees_of_freedom),
        Status::FullyConstrained => Some(0),
        Status::Conflict => None,
    };
    let mut base = sketch.clone();
    base.constraints.retain(|c| c.id != id);
    base.dimension_expressions.remove(&id);
    base.dimension_labels.remove(&id);
    nator_document::parameters::resolve_sketch(&mut base, parameters).map_err(sketch_error)?;
    let before = solver::solve(&base).map_err(sketch_error)?.status;
    let mut probe = sketch.clone();
    probe.dimension_expressions.remove(&id);
    let measured = {
        let constraint = probe
            .constraints
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| text(T::SketchSelectDimension).to_owned())?;
        probe
            .measure(&constraint.kind)
            .map_err(sketch_error)?
            .ok_or_else(|| text(T::SketchSelectDimension).to_owned())?
    };
    for constraint in probe.constraints.iter_mut().filter(|c| c.id == id) {
        if let Some(value) = constraint.kind.dimension_value_mut() {
            *value = measured;
        }
    }
    nator_document::parameters::resolve_sketch(&mut probe, parameters).map_err(sketch_error)?;
    let after = solver::solve(&probe).map_err(sketch_error)?.status;
    let driven = match (freedom(&before), freedom(&after)) {
        (Some(before), Some(after)) => after >= before,
        (_, None) => true,
        (None, Some(_)) => false,
    };
    if driven {
        sketch.dimension_expressions.remove(&id);
        for constraint in sketch.constraints.iter_mut().filter(|c| c.id == id) {
            constraint.reference = true;
            if let Some(value) = constraint.kind.dimension_value_mut() {
                *value = measured;
            }
        }
    }
    Ok(())
}

/// Desenho de uma cota, em mm no plano.
#[derive(Default)]
struct Layout {
    segments: Vec<[Point2; 2]>,
    /// Ponta e direção (unitária, para fora) de cada seta.
    arrows: Vec<(Point2, Point2)>,
    arc: Vec<Point2>,
    text: Point2,
}

fn linear(a: Point2, b: Point2, direction: Point2, text: Point2) -> Layout {
    let normal = [-direction[1], direction[0]];
    let pa = add(a, scale(normal, dot(sub(text, a), normal)));
    let pb = add(b, scale(normal, dot(sub(text, b), normal)));
    let mut layout = Layout {
        segments: vec![[a, pa], [b, pb], [pa, pb]],
        text,
        ..Default::default()
    };
    if let (Some(out_a), Some(out_b)) = (unit(sub(pa, pb)), unit(sub(pb, pa))) {
        layout.arrows = vec![(pa, out_a), (pb, out_b)];
    }
    // Texto além das extensões: a linha de cota se prolonga até ele.
    let along = |p: Point2| dot(p, direction);
    let (low, high) = (along(pa).min(along(pb)), along(pa).max(along(pb)));
    let projected = add(pa, scale(direction, along(text) - along(pa)));
    if along(text) < low || along(text) > high {
        let nearest = if (along(pa) - along(text)).abs() < (along(pb) - along(text)).abs() {
            pa
        } else {
            pb
        };
        layout.segments.push([nearest, projected]);
    }
    layout
}

fn rotate(v: Point2, angle: f64) -> Point2 {
    let (sin, cos) = angle.sin_cos();
    [v[0] * cos - v[1] * sin, v[0] * sin + v[1] * cos]
}

fn angular(at: Point2, u: Point2, sweep: f64, text: Option<Point2>, gap: f64) -> Layout {
    let radius = text
        .map(|t| distance(t, at))
        .filter(|r| *r > 1e-9)
        .unwrap_or(gap * 2.);
    let start = u[1].atan2(u[0]);
    let arc = (0..=48)
        .map(|i| {
            let angle = start + sweep * f64::from(i) / 48.;
            add(at, [radius * angle.cos(), radius * angle.sin()])
        })
        .collect::<Vec<_>>();
    let turn = if sweep < 0. { -1. } else { 1. };
    let end = rotate(u, sweep);
    let mut layout = Layout {
        text: text.unwrap_or_else(|| add(at, scale(rotate(u, sweep * 0.5), radius))),
        ..Default::default()
    };
    if let (Some(first), Some(last)) = (arc.first(), arc.last()) {
        layout.arrows = vec![
            (*first, rotate(u, -turn * std::f64::consts::FRAC_PI_2)),
            (*last, rotate(end, turn * std::f64::consts::FRAC_PI_2)),
        ];
    }
    layout.arc = arc;
    layout
}

/// Desenho da cota com o texto em `offset` (relativo à âncora) ou no lugar padrão.
fn layout(
    sketch: &Sketch,
    kind: &ConstraintKind,
    offset: Option<Point2>,
    gap: f64,
) -> Option<Layout> {
    let place = |anchor: Point2| offset.map(|o| add(anchor, o));
    Some(match *kind {
        ConstraintKind::Distance { a, b, .. }
        | ConstraintKind::DeltaX { a, b, .. }
        | ConstraintKind::DeltaY { a, b, .. } => {
            let (pa, pb) = (sketch.point(a).ok()?, sketch.point(b).ok()?);
            let direction = match kind {
                ConstraintKind::DeltaX { .. } => [1., 0.],
                ConstraintKind::DeltaY { .. } => [0., 1.],
                _ => unit(sub(pb, pa))?,
            };
            let anchor = scale(add(pa, pb), 0.5);
            let normal = [-direction[1], direction[0]];
            linear(
                pa,
                pb,
                direction,
                place(anchor).unwrap_or_else(|| add(anchor, scale(normal, gap))),
            )
        }
        ConstraintKind::PointLineDistance { point, line, .. } => {
            let p = sketch.point(point).ok()?;
            let f = foot(sketch, p, line)?;
            let direction =
                unit(sub(f, p)).or_else(|| line_direction(sketch, line).map(|d| [-d[1], d[0]]))?;
            let anchor = scale(add(p, f), 0.5);
            let normal = [-direction[1], direction[0]];
            linear(
                p,
                f,
                direction,
                place(anchor).unwrap_or_else(|| add(anchor, scale(normal, gap))),
            )
        }
        ConstraintKind::LineCircleDistance { line, circle, .. } => {
            let (center, radius) = sketch.radial(circle).ok()?;
            let f = foot(sketch, center, line)?;
            let edge = edge_toward(center, radius, f)?;
            let direction = unit(sub(edge, f)).or_else(|| unit(sub(f, center)))?;
            let anchor = scale(add(f, edge), 0.5);
            let normal = [-direction[1], direction[0]];
            linear(
                f,
                edge,
                direction,
                place(anchor).unwrap_or_else(|| add(anchor, scale(normal, gap))),
            )
        }
        ConstraintKind::Radius { circle, .. } | ConstraintKind::Diameter { circle, .. } => {
            let (center, radius) = sketch.radial(circle).ok()?;
            let diagonal = std::f64::consts::FRAC_1_SQRT_2;
            let t = place(center)
                .unwrap_or_else(|| add(center, scale([diagonal, diagonal], radius + gap)));
            let w = unit(sub(t, center)).unwrap_or([1., 0.]);
            let edge = add(center, scale(w, radius));
            let far = if distance(t, center) > radius {
                t
            } else {
                edge
            };
            let mut layout = Layout {
                text: t,
                ..Default::default()
            };
            if matches!(kind, ConstraintKind::Diameter { .. }) {
                let opposite = sub(center, scale(w, radius));
                layout.segments.push([opposite, far]);
                layout.arrows = vec![(edge, w), (opposite, scale(w, -1.))];
            } else {
                layout.segments.push([center, far]);
                layout.arrows = vec![(edge, w)];
            }
            layout
        }
        ConstraintKind::LineAngle {
            line,
            reference,
            flip_line,
            flip_reference,
            value,
        } => {
            let at = vertex(sketch, reference, line)?;
            let sign = |flip: bool| if flip { -1. } else { 1. };
            let u = scale(line_direction(sketch, reference)?, sign(flip_reference));
            let v = scale(line_direction(sketch, line)?, sign(flip_line));
            let sweep = cross(u, v).atan2(dot(u, v));
            let sweep = if sweep.is_finite() { sweep } else { value };
            angular(at, u, sweep, place(at), gap)
        }
        ConstraintKind::Angle { a, b, .. } => {
            let at = sketch.point(a).ok()?;
            let direction = unit(sub(sketch.point(b).ok()?, at))?;
            angular(
                at,
                [1., 0.],
                direction[1].atan2(direction[0]),
                place(at),
                gap,
            )
        }
        _ => return None,
    })
}

/// Valor exibido: R/Ø nos raios e diâmetros, graus nos ângulos e parênteses nas cotas de
/// referência, como no SolidCraft.
fn caption(kind: &ConstraintKind, reference: bool) -> Option<String> {
    let value = kind.dimension_value()?;
    let text = match kind {
        ConstraintKind::Radius { .. } => format!("R {}", nator_drawing::format_mm(value)),
        ConstraintKind::Diameter { .. } => format!("Ø {}", nator_drawing::format_mm(value)),
        _ if kind.is_angular() => {
            format!("{}°", nator_drawing::format_mm(value.to_degrees().abs()))
        }
        _ => nator_drawing::format_mm(value.abs()),
    };
    Some(if reference { format!("({text})") } else { text })
}

fn paint(
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    layout: &Layout,
    caption: Option<&str>,
    color: egui::Color32,
) {
    let stroke = egui::Stroke::new(1., color);
    for [a, b] in &layout.segments {
        frame.path(painter, [*a, *b], stroke, false);
    }
    if layout.arc.len() > 1 {
        frame.path(painter, layout.arc.iter().copied(), stroke, false);
    }
    for (tip, direction) in &layout.arrows {
        let (Some(head), Some(back)) = (
            frame.screen(*tip),
            frame.screen(sub(*tip, scale(*direction, 1e-3))),
        ) else {
            continue;
        };
        let d = (head - back).normalized();
        if !d.is_finite() || d.length_sq() < 0.5 {
            continue;
        }
        let side = egui::vec2(-d.y, d.x) * 3.;
        painter.add(egui::Shape::convex_polygon(
            vec![head, head - d * 9. + side, head - d * 9. - side],
            color,
            egui::Stroke::NONE,
        ));
    }
    if let (Some(caption), Some(at)) = (caption, frame.screen(layout.text)) {
        let galley =
            painter.layout_no_wrap(caption.to_owned(), egui::FontId::proportional(13.), color);
        let rect = egui::Rect::from_center_size(at, galley.size()).expand2(egui::vec2(4., 1.));
        painter.rect_filled(
            rect,
            3,
            egui::Color32::from_rgba_unmultiplied(233, 238, 245, 225),
        );
        painter.galley(rect.min + egui::vec2(4., 1.), galley, color);
    }
}

/// Cotas do rascunho em edição, exceto a que está sendo criada.
pub(super) fn paint_all(frame: &sketch_scene::Frame, painter: &egui::Painter, sketch: &Sketch) {
    let gap = f64::from(DEFAULT_GAP) / frame.pixels_per_mm([0., 0.]);
    for constraint in &sketch.constraints {
        let offset = sketch.dimension_labels.get(&constraint.id).copied();
        if let Some(layout) = layout(sketch, &constraint.kind, offset, gap) {
            let caption = caption(&constraint.kind, constraint.reference);
            let color = if constraint.reference {
                theme::MUTED
            } else {
                theme::INK
            };
            paint(frame, painter, &layout, caption.as_deref(), color);
        }
    }
}

fn highlight(
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    sketch: &Sketch,
    pick: Pick,
    color: egui::Color32,
) {
    match pick {
        Pick::Point(id) => {
            if let Some(p) = sketch.point(id).ok().and_then(|p| frame.screen(p)) {
                painter.circle_stroke(p, 6., egui::Stroke::new(2., color));
            }
        }
        Pick::Line(id) | Pick::Radial(id) => {
            let Some(entity) = sketch.entity(id).ok() else {
                return;
            };
            let Ok(segment) = profiles::segment(sketch, entity) else {
                return;
            };
            let stroke = egui::Stroke::new(3., color);
            match segment {
                profiles::Segment::Line { a, b } if entity.infinite => {
                    frame.infinite_line(painter, a, b, stroke);
                }
                _ => frame.path(painter, segment.sample(128), stroke, false),
            }
        }
    }
}

/// Um quadro da ferramenta: realce, prévia, caixa de valor e cliques no canvas.
#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    ui: &egui::Ui,
    response: &egui::Response,
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    sketch: &Sketch,
    draft: &mut Draft,
    hover: Option<Point2>,
    interactive: bool,
    parameters: &Parameters,
) -> Update {
    let mut update = Update::default();
    let pixels_per_mm = frame.pixels_per_mm(hover.unwrap_or([0., 0.]));
    let tolerance = f64::from(PICK_RADIUS) / pixels_per_mm;
    let gap = f64::from(DEFAULT_GAP) / pixels_per_mm;
    let candidate = hover.and_then(|cursor| pick(sketch, cursor, tolerance));
    for chosen in &draft.picks {
        highlight(frame, painter, sketch, *chosen, theme::ACCENT);
    }
    if let Some(candidate) = candidate.filter(|c| !draft.picks.contains(c)) {
        highlight(frame, painter, sketch, candidate, theme::WARNING);
    }
    // O texto acompanha o ponteiro mesmo sobre a caixa de valor, que fica ao lado dele.
    let pointer = ui
        .input(|i| i.pointer.latest_pos())
        .filter(|p| response.rect.contains(*p) && !view_cube::rect(response.rect).contains(*p));
    if let Some(position) = pointer {
        if let Some(point) = frame.cursor(position) {
            draft.follow = Some(point);
        }
        if draft
            .settled_at
            .is_some_and(|at| at.distance(position) > 4.)
        {
            draft.moved = true;
        }
    }
    let measure = match settle(sketch, &draft.picks) {
        Ok(measure) => measure,
        Err(()) => {
            draft.clear();
            None
        }
    };
    let label = draft.label();
    let built = measure.and_then(|m| build(sketch, m, label).ok());
    let mut commit = false;
    if let Some(built) = &built {
        if !draft.typed {
            draft.text = measured_text(&built.kind, built.measured);
        }
        let preview_kind = match draft
            .typed
            .then(|| typed_value(&built.kind, &draft.text, parameters).ok())
            .flatten()
        {
            Some(value) => {
                let mut kind = built.kind.clone();
                if let Some(target) = kind.dimension_value_mut() {
                    *target = value;
                }
                kind
            }
            None => built.kind.clone(),
        };
        let offset = label.map(|l| sub(l, built.anchor));
        if let Some(layout) = layout(sketch, &built.kind, offset, gap) {
            // Com a caixa aberta, o valor aparece nela; a prévia mostra as linhas.
            let caption = (!draft.editing)
                .then(|| caption(&preview_kind, false))
                .flatten();
            paint(frame, painter, &layout, caption.as_deref(), theme::ACCENT);
            if draft.editing
                && let Some(at) = frame.screen(layout.text)
            {
                commit = value_box(
                    ui,
                    draft,
                    at,
                    &built.kind,
                    parameters,
                    label.is_some() && draft.pinned.is_none(),
                );
            }
        }
    }
    if commit && let Some(built) = &built {
        let typed = draft.typed.then_some(draft.text.as_str());
        match self::commit(sketch, built, typed, label, parameters) {
            Ok(result) => {
                update.commit = Some(result);
                update.hint = Some(T::DimensionPickHint);
                draft.clear();
            }
            Err(message) => {
                draft.focus = true;
                update.warning = Some(message);
            }
        }
        return update;
    }
    if !interactive || !response.clicked_by(egui::PointerButton::Primary) {
        return update;
    }
    let Some(cursor) = hover else {
        return update;
    };
    let open = |draft: &mut Draft, picks: Vec<Pick>| -> bool {
        match settle(sketch, &picks) {
            Ok(Some(_)) => {
                draft.picks = picks;
                draft.typed = false;
                draft.pinned = None;
                draft.editing = true;
                draft.focus = true;
                draft.settled_at = response.interact_pointer_pos();
                draft.moved = false;
                true
            }
            Ok(None) => {
                draft.picks = picks;
                draft.editing = false;
                true
            }
            Err(()) => false,
        }
    };
    let unsupported = || Some(text(T::DimensionUnsupported).to_owned());
    match candidate {
        Some(chosen) if measure.is_some() => {
            // Outro ponto ou linha transforma a cota; uma combinação sem sentido recomeça
            // com o novo elemento.
            let mut extended = draft.picks.clone();
            extended.push(chosen);
            if !open(draft, extended) && !open(draft, vec![chosen]) {
                update.warning = unsupported();
                draft.clear();
            }
        }
        Some(chosen) => {
            let mut extended = draft.picks.clone();
            extended.push(chosen);
            if !open(draft, extended) {
                update.warning = unsupported();
                if !open(draft, vec![chosen]) {
                    draft.clear();
                }
            }
        }
        // Clique no vazio: fixa o texto da cota determinada e devolve o teclado à caixa;
        // com escolhas incompletas, desiste delas.
        None if measure.is_some() => {
            draft.pinned = Some(cursor);
            draft.editing = true;
            draft.focus = true;
        }
        None => draft.clear(),
    }
    if settle(sketch, &draft.picks) == Ok(None) && !draft.picks.is_empty() {
        update.hint = Some(T::DimensionPickSecond);
    }
    update
}

/// Caixa de valor ao lado do texto da cota. Devolve verdadeiro no Enter com valor válido.
fn value_box(
    ui: &egui::Ui,
    draft: &mut Draft,
    at: egui::Pos2,
    kind: &ConstraintKind,
    parameters: &Parameters,
    following: bool,
) -> bool {
    let id = egui::Id::new("sketch_dimension_value");
    // Seguindo o ponteiro, a caixa fica ao lado dele para não cobrir o clique que fixa o
    // texto; parada, ela ocupa o lugar do texto.
    let (pivot, position) = if following {
        (egui::Align2::LEFT_TOP, at + egui::vec2(14., 12.))
    } else {
        (egui::Align2::CENTER_CENTER, at)
    };
    let valid = typed_value(kind, &draft.text, parameters).is_ok();
    let mut confirm = false;
    egui::Area::new(id.with("area"))
        .order(egui::Order::Foreground)
        .pivot(pivot)
        .fixed_pos(position)
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(
                    1.5,
                    if valid { theme::ACCENT } else { theme::DANGER },
                ))
                .corner_radius(4)
                .inner_margin(egui::Margin::symmetric(4, 2))
                .show(ui, |ui| {
                    let response = ui
                        .add(
                            egui::TextEdit::singleline(&mut draft.text)
                                .id(id)
                                .desired_width(96.)
                                .horizontal_align(egui::Align::Center)
                                .font(egui::TextStyle::Monospace),
                        )
                        .on_hover_text(text(T::DimensionValueTitle));
                    if response.changed() {
                        draft.typed = true;
                    }
                    if std::mem::take(&mut draft.focus) {
                        response.request_focus();
                    }
                    // Até a primeira tecla, a medida fica selecionada: digitar a substitui.
                    if !draft.typed
                        && (response.has_focus() || response.gained_focus())
                        && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id)
                    {
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(0),
                                egui::text::CCursor::new(draft.text.chars().count()),
                            )));
                        state.store(ui.ctx(), id);
                    }
                    if response.lost_focus() {
                        let (enter, escape, tab) = ui.input(|i| {
                            (
                                i.key_pressed(egui::Key::Enter),
                                i.key_pressed(egui::Key::Escape),
                                i.key_pressed(egui::Key::Tab),
                            )
                        });
                        if enter {
                            if typed_value(kind, &draft.text, parameters).is_ok() {
                                confirm = true;
                            } else {
                                draft.focus = true;
                            }
                        } else if escape {
                            draft.clear();
                        } else if tab {
                            // Tab devolve as escolhas: um clique posiciona o texto.
                            draft.editing = false;
                            draft.pinned = None;
                        }
                    }
                });
        });
    confirm
}

#[cfg(test)]
mod tests {
    use super::*;
    use nator_sketch::Plane;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn endpoints(sketch: &Sketch, line: u64) -> Result<(u64, u64), Box<dyn std::error::Error>> {
        match sketch.entity(line)?.curve {
            Curve::Line { start, end } => Ok((start, end)),
            _ => Err("Linha ausente".into()),
        }
    }

    #[test]
    fn picks_settle_like_the_solidcraft_dimension() -> TestResult {
        let mut sketch = Sketch::new("Escolhas", Plane::xy());
        let base = sketch.add_line([0., 0.], [20., 0.], false)?;
        let wall = sketch.add_line([0., 0.], [0., 20.], false)?;
        let parallel = sketch.add_line([0., 8.], [20., 8.], false)?;
        let circle = sketch.add_circle([10., 30.], 4., false)?;
        let point = sketch.add_point([5., 12.])?;
        let (start, end) = endpoints(&sketch, base)?;
        use Pick::*;
        let settle = |picks: &[Pick]| settle(&sketch, picks);
        assert_eq!(settle(&[Line(base)]), Ok(Some(Measure::Points(start, end))));
        assert_eq!(
            settle(&[Radial(circle)]),
            Ok(Some(Measure::Radial {
                circle,
                diameter: true
            }))
        );
        assert_eq!(settle(&[Point(point)]), Ok(None));
        assert_eq!(
            settle(&[Point(point), Point(end)]),
            Ok(Some(Measure::Points(point, end)))
        );
        assert_eq!(
            settle(&[Line(base), Point(point)]),
            Ok(Some(Measure::PointLine { point, line: base }))
        );
        // O extremo da própria linha não define distância até ela.
        assert_eq!(settle(&[Line(base), Point(end)]), Err(()));
        assert_eq!(
            settle(&[Line(base), Line(wall)]),
            Ok(Some(Measure::Angle {
                first: base,
                second: wall
            }))
        );
        let (parallel_start, _) = endpoints(&sketch, parallel)?;
        assert_eq!(
            settle(&[Line(base), Line(parallel)]),
            Ok(Some(Measure::PointLine {
                point: parallel_start,
                line: base
            }))
        );
        assert_eq!(
            settle(&[Radial(circle), Line(base)]),
            Ok(Some(Measure::LineCircle { line: base, circle }))
        );
        assert_eq!(settle(&[Radial(circle), Point(point)]), Err(()));
        Ok(())
    }

    #[test]
    fn placement_chooses_aligned_horizontal_or_vertical() -> TestResult {
        // Os mesmos lugares do ensaio do SolidCraft para a linha (0,0)–(12,16).
        let mut sketch = Sketch::new("Lugares", Plane::xy());
        let line = sketch.add_line([0., 0.], [12., 16.], false)?;
        let (a, b) = endpoints(&sketch, line)?;
        let kind = |label: Option<Point2>| -> Result<ConstraintKind, String> {
            Ok(build(&sketch, Measure::Points(a, b), label)?.kind)
        };
        assert!(
            matches!(kind(None)?, ConstraintKind::Distance { value, .. } if (value - 20.).abs() < 1e-9)
        );
        assert!(matches!(
            kind(Some([-5., 12.]))?,
            ConstraintKind::Distance { .. }
        ));
        assert!(
            matches!(kind(Some([6., 25.]))?, ConstraintKind::DeltaX { value, .. } if (value - 12.).abs() < 1e-9)
        );
        assert!(
            matches!(kind(Some([-10., 1.]))?, ConstraintKind::DeltaY { value, .. } if (value - 16.).abs() < 1e-9)
        );
        // Pontos alinhados a um eixo sempre medem a distância entre eles.
        let horizontal = sketch.add_line([0., -5.], [10., -5.], false)?;
        let (c, d) = endpoints(&sketch, horizontal)?;
        let built = build(&sketch, Measure::Points(c, d), Some([30., -2.]))?;
        assert!(matches!(built.kind, ConstraintKind::Distance { .. }));
        Ok(())
    }

    #[test]
    fn angle_side_follows_the_wedge_that_holds_the_text() -> TestResult {
        let mut sketch = Sketch::new("Setores", Plane::xy());
        let base = sketch.add_line([0., 0.], [20., 0.], false)?;
        let inclined = sketch.add_line([0., 0.], [10., 10.], false)?;
        let measure = Measure::Angle {
            first: base,
            second: inclined,
        };
        let degrees = |label| -> Result<f64, String> {
            Ok(build(&sketch, measure, label)?.measured.to_degrees())
        };
        assert!((degrees(None)? - 45.).abs() < 1e-9);
        assert!((degrees(Some([8., 3.]))? - 45.).abs() < 1e-9);
        // Do outro lado da linha inclinada, o setor suplementar.
        assert!((degrees(Some([-6., 3.]))? - 135.).abs() < 1e-9);
        // O valor guardado é o do setor, positivo, e a solução o mantém.
        let built = build(&sketch, measure, Some([-6., 3.]))?;
        let parameters = Parameters::default();
        let (next, id) = commit(&sketch, &built, Some("120"), Some([-6., 3.]), &parameters)?;
        let stored = next
            .constraints
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.kind.dimension_value())
            .ok_or("Cota ausente")?;
        assert!((stored.to_degrees() - 120.).abs() < 1e-9);
        assert_eq!(
            next.dimension_expressions.get(&id).map(String::as_str),
            Some("120")
        );
        assert!(next.dimension_labels.contains_key(&id));
        Ok(())
    }

    #[test]
    fn untyped_commits_keep_the_geometry_and_fixed_geometry_gets_a_reference() -> TestResult {
        let mut sketch = Sketch::new("Referência", Plane::xy());
        let line = sketch.add_line([0., 0.], [12.3456789, 0.], false)?;
        let (a, b) = endpoints(&sketch, line)?;
        let parameters = Parameters::default();
        let built = build(&sketch, Measure::Points(a, b), None)?;
        // Sem digitar, a medida exata entra na expressão, sem o arredondamento da caixa.
        let (mut next, id) = commit(&sketch, &built, None, None, &parameters)?;
        settle_driven(&mut next, id, &parameters)?;
        let constraint = next.constraints.iter().find(|c| c.id == id).ok_or("Cota")?;
        assert!(!constraint.reference);
        let report = solver::solve(&next)?;
        assert!(
            (distance(report.sketch.point(a)?, report.sketch.point(b)?) - 12.3456789).abs() < 1e-9
        );
        assert!(next.dimension_labels.is_empty());
        // Com os dois extremos fixos, a mesma cota só mede.
        for point in [a, b] {
            let position = sketch.point(point)?;
            sketch.add_constraint(ConstraintKind::Fixed { point, position }, false)?;
        }
        let built = build(&sketch, Measure::Points(a, b), None)?;
        let (mut fixed, id) = commit(&sketch, &built, Some("30"), None, &parameters)?;
        settle_driven(&mut fixed, id, &parameters)?;
        let constraint = fixed
            .constraints
            .iter()
            .find(|c| c.id == id)
            .ok_or("Cota")?;
        assert!(constraint.reference);
        assert!(!fixed.dimension_expressions.contains_key(&id));
        assert_eq!(caption(&constraint.kind, true).as_deref(), Some("(12,346)"));
        // Valores fora do domínio são recusados antes de chegar ao rascunho.
        assert!(commit(&sketch, &built, Some("-3"), None, &parameters).is_err());
        assert!(commit(&sketch, &built, Some("3 deg"), None, &parameters).is_err());
        Ok(())
    }
}
