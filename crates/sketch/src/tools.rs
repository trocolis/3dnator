//! Edições geométricas explícitas; o chamador mantém uma prévia antes de confirmar.
use crate::*;

pub fn offset_entity(sketch: &mut Sketch, id: u64, distance_mm: f64) -> Result<u64> {
    if !distance_mm.is_finite() {
        return Err(SketchError("Deslocamento não finito.".into()));
    }
    let entity = sketch.entity(id)?.clone();
    match entity.curve {
        Curve::Line { start, end } => {
            let (a, b) = (sketch.point(start)?, sketch.point(end)?);
            let d = sub(b, a);
            let n = scale([-d[1], d[0]], distance_mm / distance(a, b));
            sketch.add_line_with_type(add(a, n), add(b, n), entity.line_type())
        }
        Curve::Circle { center, radius } => sketch.add_circle(
            sketch.point(center)?,
            radius + distance_mm,
            entity.construction,
        ),
        Curve::Arc {
            center,
            start,
            end,
            clockwise,
        } => {
            let center = sketch.point(center)?;
            let a = sketch.point(start)?;
            let b = sketch.point(end)?;
            let radius = distance(center, a);
            let offset_radius = radius + distance_mm;
            if offset_radius <= 1e-9 || !offset_radius.is_finite() {
                return Err(SketchError(
                    "O deslocamento produziria um arco sem raio positivo.".into(),
                ));
            }
            let factor = offset_radius / radius;
            let center_id = sketch.add_point(center)?;
            let start = sketch.add_point(add(center, scale(sub(a, center), factor)))?;
            let end = sketch.add_point(add(center, scale(sub(b, center), factor)))?;
            sketch.add_curve(
                Curve::Arc {
                    center: center_id,
                    start,
                    end,
                    clockwise,
                },
                entity.construction,
            )
        }
    }
}
pub fn extend_line(sketch: &mut Sketch, line: u64, target: u64, near: Point2) -> Result<()> {
    sketch.refuse_locked(&[line])?;
    if sketch.entity(line)?.infinite {
        return Err(SketchError(
            "Uma reta infinita não tem extremo para estender.".into(),
        ));
    }
    let (a, b) = sketch.line(line)?;
    let (c, d) = sketch.line(target)?;
    let (point, t, _) = profiles::line_intersection(a, b, c, d)
        .ok_or_else(|| SketchError("Linhas paralelas não se encontram.".into()))?;
    if (0.0..=1.0).contains(&t) {
        return Err(SketchError(
            "A interseção já está dentro do segmento.".into(),
        ));
    }
    let Curve::Line { start, end } = sketch.entity(line)?.curve else {
        return Err(SketchError("Linha ausente.".into()));
    };
    let id = if distance(near, a) < distance(near, b) {
        start
    } else {
        end
    };
    let p = sketch
        .points
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| SketchError("Extremo ausente.".into()))?;
    p.position = point;
    Ok(())
}
/// Curva finita mais próxima do cursor, medida analiticamente no plano em mm.
pub fn pick_entity(sketch: &Sketch, cursor: Point2, tolerance_mm: f64) -> Result<Option<u64>> {
    use crate::curve_geometry::{closest_parameter, point};
    let mut best: Option<(u64, f64)> = None;
    for entity in &sketch.entities {
        let curve = profiles::segment(sketch, entity)?;
        let nearest = if entity.infinite {
            lines::closest(sketch, entity, cursor)?
        } else {
            point(&curve, closest_parameter(&curve, cursor))
        };
        let d = distance(cursor, nearest);
        if d <= tolerance_mm && best.is_none_or(|(_, previous)| d < previous) {
            best = Some((entity.id, d));
        }
    }
    Ok(best.map(|(id, _)| id))
}

/// Trecho orientado a remover, delimitado por extremos e encontros reais das curvas.
/// Círculos são periódicos: o intervalo que cruza 0/2π continua sendo um único trecho.
/// A prévia usa o mesmo intervalo analítico da aplicação do corte.
pub fn trim_preview(sketch: &Sketch, entity: u64, pick: Point2) -> Result<profiles::Segment> {
    let (curve, interval) = trim_interval(sketch, entity, pick)?;
    if matches!(curve, profiles::Segment::Circle { .. }) && interval[1] - interval[0] >= 1. - 1e-9 {
        return Ok(curve);
    }
    Ok(crate::curve_geometry::slice(
        &curve,
        interval[0],
        interval[1],
    ))
}

fn trim_interval(
    sketch: &Sketch,
    entity: u64,
    pick: Point2,
) -> Result<(profiles::Segment, [f64; 2])> {
    use crate::curve_geometry::{closest_parameter, entity_intersections, parameter};
    if pick.iter().any(|v| !v.is_finite()) {
        return Err(SketchError(
            "Selecione um trecho válido para aparar.".into(),
        ));
    }
    let original = sketch.entity(entity)?;
    if original.infinite {
        return Err(SketchError(
            "Uma reta infinita serve de referência e não pode ser aparada.".into(),
        ));
    }
    let curve = profiles::segment(sketch, original)?;
    let closed = matches!(curve, profiles::Segment::Circle { .. });
    let at = closest_parameter(&curve, pick);
    let mut cuts = if closed { Vec::new() } else { vec![0., 1.] };
    for other in &sketch.entities {
        if other.id == entity {
            continue;
        }
        for point in entity_intersections(sketch, original, other)? {
            if let Some(t) = parameter(&curve, point) {
                cuts.push(t);
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    if closed {
        if cuts.len() < 2 {
            return Ok((curve, [0., 1.]));
        }
        let first = cuts[0];
        cuts.push(first + 1.);
        let at = if at < first { at + 1. } else { at };
        let pair = cuts
            .windows(2)
            .find(|p| at >= p[0] && at <= p[1])
            .ok_or_else(|| SketchError("Trecho circular inexistente.".into()))?;
        return Ok((curve, [pair[0], pair[1]]));
    }
    let pair = cuts
        .windows(2)
        .find(|p| at >= p[0] && at <= p[1])
        .ok_or_else(|| SketchError("Selecione um trecho da aresta.".into()))?;
    Ok((curve, [pair[0], pair[1]]))
}

/// Aparar é atômico: falhas de criação, limites ou validação preservam o rascunho.
/// As curvas de encontro não são alteradas. Os extremos novos materializam os vértices
/// virtuais de interseção; apenas vínculos que perderam entidades/pontos são retirados.
pub fn trim(sketch: &mut Sketch, entity: u64, pick: Point2) -> Result<()> {
    sketch.refuse_locked(&[entity])?;
    use crate::curve_geometry::slice;
    let (curve, [from, to]) = trim_interval(sketch, entity, pick)?;
    let original = sketch.entity(entity)?.clone();
    let mut candidate = sketch.clone();
    candidate.entities.retain(|e| e.id != entity);
    let mut intervals = Vec::new();
    if matches!(curve, profiles::Segment::Circle { .. }) {
        if to - from < 1. - 1e-9 {
            intervals.push([to, from + 1.]);
        }
    } else {
        if from > 1e-9 {
            intervals.push([0., from]);
        }
        if to < 1. - 1e-9 {
            intervals.push([to, 1.]);
        }
    }
    for [a, b] in intervals {
        match slice(&curve, a, b) {
            profiles::Segment::Line { a, b } => {
                candidate.add_line(a, b, original.construction)?;
            }
            profiles::Segment::Arc {
                a, b, clockwise, ..
            } => {
                let center = match original.curve {
                    Curve::Circle { center, .. } | Curve::Arc { center, .. } => center,
                    _ => return Err(SketchError("Centro do trecho circular ausente.".into())),
                };
                let start = candidate.add_point(a)?;
                let end = candidate.add_point(b)?;
                candidate.add_curve(
                    Curve::Arc {
                        center,
                        start,
                        end,
                        clockwise,
                    },
                    original.construction,
                )?;
            }
            _ => return Err(SketchError("Trecho inválido para aparar.".into())),
        }
    }
    candidate.prune_unused_points();
    candidate.validate()?;
    *sketch = candidate;
    Ok(())
}

pub fn mirror(
    sketch: &mut Sketch,
    ids: &[u64],
    axis_a: Point2,
    axis_b: Point2,
) -> Result<Vec<u64>> {
    let d = sub(axis_b, axis_a);
    let length = distance(axis_a, axis_b);
    if length < 1e-9 {
        return Err(SketchError("Eixo de simetria inválido.".into()));
    }
    let unit = scale(d, 1. / length);
    let reflect = |p: Point2| {
        let v = sub(p, axis_a);
        add(axis_a, sub(scale(unit, 2. * dot(v, unit)), v))
    };
    let mut created = Vec::new();
    for id in ids {
        let entity = sketch.entity(*id)?.clone();
        let new = match entity.curve {
            Curve::Line { start, end } => sketch.add_line_with_type(
                reflect(sketch.point(start)?),
                reflect(sketch.point(end)?),
                entity.line_type(),
            )?,
            Curve::Circle { center, radius } => {
                sketch.add_circle(reflect(sketch.point(center)?), radius, entity.construction)?
            }
            Curve::Arc {
                center,
                start,
                end,
                clockwise,
            } => {
                let center = sketch.add_point(reflect(sketch.point(center)?))?;
                let start = sketch.add_point(reflect(sketch.point(start)?))?;
                let end = sketch.add_point(reflect(sketch.point(end)?))?;
                sketch.add_curve(
                    Curve::Arc {
                        center,
                        start,
                        end,
                        clockwise: !clockwise,
                    },
                    entity.construction,
                )?
            }
        };
        created.push(new);
    }
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_preserves_the_unselected_pieces() -> Result<()> {
        let mut sketch = Sketch::new("Aparar", Plane::xy());
        let line = sketch.add_line([0., 0.], [30., 0.], false)?;
        sketch.add_line([10., -5.], [10., 5.], false)?;
        sketch.add_line([20., -5.], [20., 5.], false)?;
        trim(&mut sketch, line, [15., 0.])?;
        sketch.validate()?;
        assert_eq!(sketch.entities.len(), 4);
        let lengths = sketch
            .entities
            .iter()
            .filter_map(|e| sketch.line(e.id).ok())
            .map(|(a, b)| distance(a, b))
            .sum::<f64>();
        assert!((lengths - 40.).abs() < 1e-8);
        Ok(())
    }

    #[test]
    fn trim_uses_circle_crossings_and_preserves_the_other_entities() -> Result<()> {
        let mut sketch = Sketch::new("Linha e círculo", Plane::xy());
        let line = sketch.add_line([-20., 0.], [20., 0.], true)?;
        let circle = sketch.add_circle([0., 0.], 10., false)?;
        let Curve::Circle { center, .. } = sketch.entity(circle)?.curve else {
            return Err(SketchError("Centro ausente".into()));
        };
        let fixed = sketch.add_constraint(
            ConstraintKind::Fixed {
                point: center,
                position: [0., 0.],
            },
            false,
        )?;
        let profiles::Segment::Line { a, b } = trim_preview(&sketch, line, [1., 0.])? else {
            return Err(SketchError("Prévia incorreta".into()));
        };
        assert_eq!((a, b), ([-10., 0.], [10., 0.]));
        trim(&mut sketch, line, [1., 0.])?;
        assert_eq!(sketch.circle(circle)?, ([0., 0.], 10.));
        assert!(sketch.constraints.iter().any(|c| c.id == fixed));
        let pieces = sketch
            .entities
            .iter()
            .filter(|e| matches!(e.curve, Curve::Line { .. }))
            .collect::<Vec<_>>();
        assert_eq!(pieces.len(), 2);
        assert!(pieces.iter().all(|e| e.construction));
        for piece in pieces {
            let (a, b) = sketch.line(piece.id)?;
            assert!((distance(a, b) - 10.).abs() < 1e-9);
        }
        sketch.validate()?;
        let line = sketch.add_line([-20., 5.], [20., 5.], false)?;
        sketch.next_id = u64::MAX;
        let before = format!("{sketch:?}");
        assert!(trim(&mut sketch, line, [0., 5.]).is_err());
        assert_eq!(before, format!("{sketch:?}"));
        Ok(())
    }

    #[test]
    fn trimming_a_circle_across_zero_keeps_one_oriented_arc() -> Result<()> {
        let mut sketch = Sketch::new("Círculo aparado", Plane::xy());
        let circle = sketch.add_circle([0., 0.], 10., false)?;
        let line = sketch.add_line([5., -20.], [5., 20.], false)?;
        let preview = trim_preview(&sketch, circle, [10., 0.])?;
        assert!(preview.sample(128).iter().all(|p| p[0] >= 5. - 1e-9));
        trim(&mut sketch, circle, [10., 0.])?;
        assert_eq!(sketch.line(line)?, ([5., -20.], [5., 20.]));
        let arc = sketch
            .entities
            .iter()
            .find(|e| matches!(e.curve, Curve::Arc { .. }))
            .ok_or_else(|| SketchError("Arco restante ausente".into()))?;
        let points = profiles::segment(&sketch, arc)?.sample(128);
        assert!(points.iter().all(|p| p[0] <= 5. + 1e-9));
        assert!(points.iter().any(|p| p[0] < -9.9));
        sketch.validate()?;
        Ok(())
    }

    #[test]
    fn clockwise_arcs_tangencies_and_rejected_trims_keep_valid_geometry() -> Result<()> {
        let mut sketch = Sketch::new("Arco horário", Plane::xy());
        let center = sketch.add_point([0., 0.])?;
        let start = sketch.add_point([0., 10.])?;
        let end = sketch.add_point([0., -10.])?;
        let arc = sketch.add_curve(
            Curve::Arc {
                center,
                start,
                end,
                clockwise: true,
            },
            false,
        )?;
        let line = sketch.add_line([-20., 0.], [20., 0.], false)?;
        trim(&mut sketch, arc, [7., 7.])?;
        let remaining = sketch
            .entities
            .iter()
            .find(|e| matches!(e.curve, Curve::Arc { .. }))
            .ok_or_else(|| SketchError("Arco ausente".into()))?;
        assert!(matches!(
            remaining.curve,
            Curve::Arc {
                clockwise: true,
                ..
            }
        ));
        assert!(
            profiles::segment(&sketch, remaining)?
                .sample(32)
                .iter()
                .all(|p| p[0] >= -1e-9 && p[1] <= 1e-9)
        );
        let before = format!("{sketch:?}");
        assert!(trim(&mut sketch, line, [f64::NAN, 0.]).is_err());
        assert_eq!(before, format!("{sketch:?}"));
        let circle = sketch.add_circle([0., 20.], 10., false)?;
        let tangent = sketch.add_line([-20., 30.], [20., 30.], false)?;
        trim(&mut sketch, tangent, [-10., 30.])?;
        assert_eq!(sketch.circle(circle)?, ([0., 20.], 10.));
        sketch.validate()?;
        Ok(())
    }
}

/// Refina um encontro entre dois segmentos. Cotas ligadas ao vértice removido são retiradas
/// explicitamente; cotas dos segmentos sobreviventes permanecem para o solver conferir.
pub fn refine_corner(
    sketch: &mut Sketch,
    first: u64,
    second: u64,
    size: f64,
    rounded: bool,
) -> Result<u64> {
    if sketch.entity(first)?.infinite || sketch.entity(second)?.infinite {
        return Err(SketchError(
            "Retas infinitas não têm extremos para formar um canto.".into(),
        ));
    }
    if first == second || !size.is_finite() || size <= 1e-9 {
        return Err(SketchError(
            "Selecione duas linhas e uma dimensão positiva.".into(),
        ));
    }
    if sketch.line(first).is_err() || sketch.line(second).is_err() {
        return refine_curved_corner(sketch, first, second, size, rounded);
    }
    let (a, b) = sketch.line(first)?;
    let (c, d) = sketch.line(second)?;
    let corner = [(a, c), (a, d), (b, c), (b, d)]
        .into_iter()
        .find(|(p, q)| distance(*p, *q) < 1e-7)
        .map(|(p, _)| p)
        .ok_or_else(|| SketchError("As linhas devem compartilhar uma extremidade.".into()))?;
    let far_a = if distance(a, corner) < 1e-7 { b } else { a };
    let far_b = if distance(c, corner) < 1e-7 { d } else { c };
    let len_a = distance(corner, far_a);
    let len_b = distance(corner, far_b);
    let u = scale(sub(far_a, corner), 1. / len_a);
    let v = scale(sub(far_b, corner), 1. / len_b);
    let half = dot(u, v).clamp(-1., 1.).acos() / 2.;
    if half.sin().abs() < 1e-6 || half.cos().abs() < 1e-6 {
        return Err(SketchError(
            "O encontro das linhas não forma um canto refinável.".into(),
        ));
    }
    let cut = if rounded { size / half.tan() } else { size };
    if cut >= len_a - 1e-7 || cut >= len_b - 1e-7 {
        return Err(SketchError(
            "A dimensão ultrapassa os segmentos selecionados.".into(),
        ));
    }
    let p = add(corner, scale(u, cut));
    let q = add(corner, scale(v, cut));
    let pi = sketch.add_point(p)?;
    let qi = sketch.add_point(q)?;
    for (line, point) in [(first, pi), (second, qi)] {
        let entity = sketch.entity(line)?.clone();
        if let Curve::Line { start, end } = entity.curve {
            let at_start = distance(sketch.point(start)?, corner) < 1e-7;
            if let Some(e) = sketch.entities.iter_mut().find(|e| e.id == line) {
                e.curve = if at_start {
                    Curve::Line { start: point, end }
                } else {
                    Curve::Line { start, end: point }
                };
            }
        }
    }
    let id = if rounded {
        let sum = add(u, v);
        let center = add(
            corner,
            scale(sum, size / (half.sin() * distance(sum, [0.; 2]))),
        );
        let ci = sketch.add_point(center)?;
        let id = sketch.add_curve(
            Curve::Arc {
                center: ci,
                start: pi,
                end: qi,
                clockwise: cross(u, v) > 0.,
            },
            false,
        )?;
        sketch.add_constraint(
            ConstraintKind::Radius {
                circle: id,
                value: size,
            },
            false,
        )?;
        sketch.add_constraint(
            ConstraintKind::Tangent {
                line: first,
                arc: id,
                point: pi,
            },
            false,
        )?;
        sketch.add_constraint(
            ConstraintKind::Tangent {
                line: second,
                arc: id,
                point: qi,
            },
            false,
        )?;
        id
    } else {
        sketch.add_curve(Curve::Line { start: pi, end: qi }, false)?
    };
    sketch.prune_unused_points();
    Ok(id)
}
/// Direção unitária com que a linha ou o arco sai da extremidade `at` para dentro de si;
/// `None` quando `at` não é extremidade (ou a entidade é um círculo inteiro).
pub fn leaving(sketch: &Sketch, id: u64, at: Point2) -> Result<Option<Point2>> {
    let unit = |v: Point2| {
        let length = distance(v, [0.; 2]);
        (length > 1e-12).then(|| scale(v, 1. / length))
    };
    match sketch.entity(id)?.curve {
        Curve::Line { .. } => {
            let (a, b) = sketch.line(id)?;
            Ok(if distance(a, at) < 1e-7 {
                unit(sub(b, a))
            } else if distance(b, at) < 1e-7 {
                unit(sub(a, b))
            } else {
                None
            })
        }
        Curve::Arc {
            center,
            start,
            end,
            clockwise,
        } => {
            let center = sketch.point(center)?;
            let radial = sub(at, center);
            // Sentido de percurso do arco (anti-horário salvo `clockwise`), do início ao fim.
            let travel = if clockwise {
                [radial[1], -radial[0]]
            } else {
                [-radial[1], radial[0]]
            };
            Ok(if distance(sketch.point(start)?, at) < 1e-7 {
                unit(travel)
            } else if distance(sketch.point(end)?, at) < 1e-7 {
                unit(scale(travel, -1.))
            } else {
                None
            })
        }
        Curve::Circle { .. } => Ok(None),
    }
}

/// Curva deslocada para dentro do canto: reta paralela ou círculo concêntrico.
enum Offset {
    Line { point: Point2, direction: Point2 },
    Circle { center: Point2, radius: f64 },
}

/// Interseções de duas curvas deslocadas.
fn crossings(a: &Offset, b: &Offset) -> Vec<Point2> {
    let line_circle = |point: Point2, direction: Point2, center: Point2, radius: f64| {
        // |point + t·direction − center|² = radius², com direction unitária.
        let w = sub(point, center);
        let half = dot(direction, w);
        let disc = half * half - (dot(w, w) - radius * radius);
        if disc < -1e-12 {
            return Vec::new();
        }
        let root = disc.max(0.).sqrt();
        [-half - root, -half + root]
            .into_iter()
            .map(|t| add(point, scale(direction, t)))
            .collect()
    };
    match (a, b) {
        (Offset::Line { point, direction }, Offset::Circle { center, radius })
        | (Offset::Circle { center, radius }, Offset::Line { point, direction }) => {
            line_circle(*point, *direction, *center, *radius)
        }
        (
            Offset::Circle {
                center: c1,
                radius: r1,
            },
            Offset::Circle {
                center: c2,
                radius: r2,
            },
        ) => {
            let d = distance(*c1, *c2);
            if d < 1e-12 || d > r1 + r2 + 1e-9 || d < (r1 - r2).abs() - 1e-9 {
                return Vec::new();
            }
            let along = (d * d + r1 * r1 - r2 * r2) / (2. * d);
            let height = (r1 * r1 - along * along).max(0.).sqrt();
            let axis = scale(sub(*c2, *c1), 1. / d);
            let base = add(*c1, scale(axis, along));
            let normal = [-axis[1], axis[0]];
            vec![
                add(base, scale(normal, height)),
                add(base, scale(normal, -height)),
            ]
        }
        (
            Offset::Line {
                point: p1,
                direction: d1,
            },
            Offset::Line {
                point: p2,
                direction: d2,
            },
        ) => {
            let denominator = cross(*d1, *d2);
            if denominator.abs() < 1e-12 {
                return Vec::new();
            }
            let t = cross(sub(*p2, *p1), *d2) / denominator;
            vec![add(*p1, scale(*d1, t))]
        }
    }
}

/// O ponto `point`, sobre a curva, fica dentro do trecho entre a extremidade `corner` e a
/// outra (exclusive), percorrendo a partir do canto.
fn within(sketch: &Sketch, id: u64, corner: Point2, point: Point2) -> Result<bool> {
    match sketch.entity(id)?.curve {
        Curve::Line { .. } => {
            let (a, b) = sketch.line(id)?;
            let far = if distance(a, corner) < 1e-7 { b } else { a };
            let length = distance(corner, far);
            let t = dot(sub(point, corner), scale(sub(far, corner), 1. / length));
            Ok(t > 1e-7 && t < length - 1e-7)
        }
        Curve::Arc {
            center, start, end, ..
        } => {
            let center = sketch.point(center)?;
            let far = if distance(sketch.point(start)?, corner) < 1e-7 {
                sketch.point(end)?
            } else {
                sketch.point(start)?
            };
            let Some(direction) = leaving(sketch, id, corner)? else {
                return Ok(false);
            };
            // Ângulo percorrido a partir do canto, no sentido em que o arco segue.
            let sweep = |target: Point2| {
                let a = sub(corner, center);
                let b = sub(target, center);
                let mut angle = cross(a, b).atan2(dot(a, b));
                let sense = cross(a, direction).signum();
                angle *= sense;
                if angle < 0. {
                    angle += std::f64::consts::TAU;
                }
                angle
            };
            let total = sweep(far);
            let total = if total < 1e-9 {
                std::f64::consts::TAU
            } else {
                total
            };
            let at = sweep(point);
            Ok(at > 1e-9 && at < total - 1e-9)
        }
        Curve::Circle { .. } => Ok(false),
    }
}

/// Arredondamento ou chanfro de um canto em que ao menos uma das curvas é arco. O centro do
/// arredondamento é o encontro das duas curvas deslocadas de `size` para dentro do canto
/// (reta paralela; círculo com raio R − size ou R + size, conforme o lado); os pontos de
/// tangência são as projeções do centro nas curvas. O chanfro corta `size` ao longo de cada
/// curva (comprimento de arco, no arco).
fn refine_curved_corner(
    sketch: &mut Sketch,
    first: u64,
    second: u64,
    size: f64,
    rounded: bool,
) -> Result<u64> {
    let ends = |sketch: &Sketch, id: u64| -> Result<Vec<Point2>> {
        Ok(match sketch.entity(id)?.curve {
            Curve::Line { .. } => {
                let (a, b) = sketch.line(id)?;
                vec![a, b]
            }
            Curve::Arc { start, end, .. } => vec![sketch.point(start)?, sketch.point(end)?],
            Curve::Circle { .. } => Vec::new(),
        })
    };
    let first_ends = ends(sketch, first)?;
    let second_ends = ends(sketch, second)?;
    let corner = first_ends
        .iter()
        .copied()
        .find(|p| second_ends.iter().any(|q| distance(*p, *q) < 1e-7))
        .ok_or_else(|| SketchError("As curvas devem compartilhar uma extremidade.".into()))?;
    let (Some(u), Some(v)) = (
        leaving(sketch, first, corner)?,
        leaving(sketch, second, corner)?,
    ) else {
        return Err(SketchError(
            "As curvas devem compartilhar uma extremidade.".into(),
        ));
    };
    let half = dot(u, v).clamp(-1., 1.).acos() / 2.;
    if half.sin().abs() < 1e-6 || half.cos().abs() < 1e-6 {
        return Err(SketchError(
            "O encontro das curvas não forma um canto refinável.".into(),
        ));
    }
    let bisector = {
        let sum = add(u, v);
        scale(sum, 1. / distance(sum, [0.; 2]))
    };
    let too_big = || SketchError("A dimensão ultrapassa as curvas selecionadas.".into());
    // Para cada curva: deslocamento para dentro do canto e se o arredondamento fica do lado
    // de dentro do arco (tangência interna).
    let offset = |sketch: &Sketch, id: u64, direction: Point2| -> Result<(Offset, bool)> {
        match sketch.entity(id)?.curve {
            Curve::Arc { .. } => {
                let (center, radius) = sketch.radial(id)?;
                let inside = dot(bisector, sub(center, corner)) > 0.;
                let radius = if inside { radius - size } else { radius + size };
                if radius <= 1e-9 {
                    return Err(too_big());
                }
                Ok((Offset::Circle { center, radius }, inside))
            }
            _ => {
                let mut normal = [-direction[1], direction[0]];
                if dot(normal, bisector) < 0. {
                    normal = scale(normal, -1.);
                }
                Ok((
                    Offset::Line {
                        point: add(corner, scale(normal, size)),
                        direction,
                    },
                    false,
                ))
            }
        }
    };
    // Ponto de cada curva: tangência (arredondamento) ou corte (chanfro).
    let touch = |sketch: &Sketch, id: u64, direction: Point2, center: Point2| -> Result<Point2> {
        match sketch.entity(id)?.curve {
            Curve::Arc { .. } => {
                let (o, radius) = sketch.radial(id)?;
                if rounded {
                    let radial = sub(center, o);
                    Ok(add(o, scale(radial, radius / distance(radial, [0.; 2]))))
                } else {
                    // Chanfro: `size` de comprimento de arco a partir do canto.
                    let angle = size / radius * cross(sub(corner, o), direction).signum();
                    let (sin, cos) = angle.sin_cos();
                    let r = sub(corner, o);
                    Ok(add(o, [r[0] * cos - r[1] * sin, r[0] * sin + r[1] * cos]))
                }
            }
            _ => Ok(if rounded {
                add(
                    corner,
                    scale(direction, dot(sub(center, corner), direction)),
                )
            } else {
                add(corner, scale(direction, size))
            }),
        }
    };
    let (center, internal) = if rounded {
        let (a, first_internal) = offset(sketch, first, u)?;
        let (b, second_internal) = offset(sketch, second, v)?;
        let center = crossings(&a, &b)
            .into_iter()
            .min_by(|p, q| distance(*p, corner).total_cmp(&distance(*q, corner)))
            .ok_or_else(too_big)?;
        (Some(center), [first_internal, second_internal])
    } else {
        (None, [false; 2])
    };
    let p = touch(sketch, first, u, center.unwrap_or(corner))?;
    let q = touch(sketch, second, v, center.unwrap_or(corner))?;
    if !within(sketch, first, corner, p)? || !within(sketch, second, corner, q)? {
        return Err(too_big());
    }
    let pi = sketch.add_point(p)?;
    let qi = sketch.add_point(q)?;
    for (id, point) in [(first, pi), (second, qi)] {
        let entity = sketch.entity(id)?.clone();
        let curve = match entity.curve {
            Curve::Line { start, end } => {
                if distance(sketch.point(start)?, corner) < 1e-7 {
                    Curve::Line { start: point, end }
                } else {
                    Curve::Line { start, end: point }
                }
            }
            Curve::Arc {
                center,
                start,
                end,
                clockwise,
            } => {
                if distance(sketch.point(start)?, corner) < 1e-7 {
                    Curve::Arc {
                        center,
                        start: point,
                        end,
                        clockwise,
                    }
                } else {
                    Curve::Arc {
                        center,
                        start,
                        end: point,
                        clockwise,
                    }
                }
            }
            other => other,
        };
        if let Some(e) = sketch.entities.iter_mut().find(|e| e.id == id) {
            e.curve = curve;
        }
    }
    let id = if let Some(center) = center {
        let ci = sketch.add_point(center)?;
        let id = sketch.add_curve(
            Curve::Arc {
                center: ci,
                start: pi,
                end: qi,
                clockwise: cross(sub(p, center), sub(q, center)) < 0.,
            },
            false,
        )?;
        sketch.add_constraint(
            ConstraintKind::Radius {
                circle: id,
                value: size,
            },
            false,
        )?;
        for ((curve, point), internal) in [(first, pi), (second, qi)].into_iter().zip(internal) {
            let kind = if sketch.line(curve).is_ok() {
                ConstraintKind::Tangent {
                    line: curve,
                    arc: id,
                    point,
                }
            } else {
                ConstraintKind::CircleTangent {
                    a: curve,
                    b: id,
                    internal,
                }
            };
            sketch.add_constraint(kind, false)?;
        }
        id
    } else {
        sketch.add_curve(Curve::Line { start: pi, end: qi }, false)?
    };
    sketch.prune_unused_points();
    Ok(id)
}

#[cfg(test)]
mod corner_tests {
    use super::*;

    /// Linha horizontal até a origem e um quarto de círculo que sobe dela (centro à
    /// esquerda): o canto de 90° da captura do usuário.
    fn line_and_arc() -> Result<(Sketch, u64, u64)> {
        let mut sketch = Sketch::new("Linha e arco", Plane::xy());
        let line = sketch.add_line([-20., 0.], [0., 0.], false)?;
        let center = sketch.add_point([-30., 0.])?;
        let start = sketch.add_point([0., 0.])?;
        let end = sketch.add_point([-30., 30.])?;
        let arc = sketch.add_curve(
            Curve::Arc {
                center,
                start,
                end,
                clockwise: false,
            },
            false,
        )?;
        Ok((sketch, line, arc))
    }

    #[test]
    fn a_line_and_an_arc_get_a_tangent_fillet() -> Result<()> {
        let (mut sketch, line, arc) = line_and_arc()?;
        let corners = crate::corners::selected(&sketch, &[line, arc])?;
        assert_eq!(corners.len(), 1);
        let fillet = refine_corner(&mut sketch, line, arc, 2., true)?;
        sketch.validate()?;
        assert!(solver::max_error(&sketch)? < 1e-9);
        let (center, radius) = sketch.radial(fillet)?;
        assert!((radius - 2.).abs() < 1e-9);
        // Tangente à linha (centro a 2 mm acima dela) e, por dentro, ao arco de 30 mm.
        assert!((center[1] - 2.).abs() < 1e-9);
        assert!((distance(center, [-30., 0.]) - 28.).abs() < 1e-9);
        // A linha e o arco terminam onde o arredondamento começa.
        let (_, line_end) = sketch.line(line)?;
        assert!((line_end[0] - center[0]).abs() < 1e-9 && line_end[1].abs() < 1e-9);
        assert!((sketch.radial(arc)?.1 - 30.).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn two_arcs_meeting_at_an_angle_get_a_fillet_and_a_chamfer() -> Result<()> {
        for rounded in [true, false] {
            let mut sketch = Sketch::new("Dois arcos", Plane::xy());
            // Dois arcos de 20 mm que saem da origem em ângulo reto: um sobe (centro à
            // esquerda) e o outro segue para a direita descendo (centro abaixo).
            let corner = sketch.add_point([0., 0.])?;
            let left = sketch.add_point([-20., 0.])?;
            let left_end = sketch.add_point([-20., 20.])?;
            let below = sketch.add_point([0., -20.])?;
            let below_end = sketch.add_point([20., -20.])?;
            let a = sketch.add_curve(
                Curve::Arc {
                    center: left,
                    start: corner,
                    end: left_end,
                    clockwise: false,
                },
                false,
            )?;
            let b = sketch.add_curve(
                Curve::Arc {
                    center: below,
                    start: corner,
                    end: below_end,
                    clockwise: true,
                },
                false,
            )?;
            let made = refine_corner(&mut sketch, a, b, 1.5, rounded)?;
            sketch.validate()?;
            assert!(solver::max_error(&sketch)? < 1e-9, "{rounded}");
            assert!((sketch.radial(a)?.1 - 20.).abs() < 1e-9);
            assert!((sketch.radial(b)?.1 - 20.).abs() < 1e-9);
            if rounded {
                assert!((sketch.radial(made)?.1 - 1.5).abs() < 1e-9);
            } else {
                let (p, q) = sketch.line(made)?;
                assert!(distance(p, q) > 0.5);
            }
        }
        Ok(())
    }

    #[test]
    fn a_line_and_an_arc_get_a_chamfer_and_oversized_cuts_change_nothing() -> Result<()> {
        let (mut sketch, line, arc) = line_and_arc()?;
        let chamfer = refine_corner(&mut sketch, line, arc, 3., false)?;
        sketch.validate()?;
        let (p, q) = sketch.line(chamfer)?;
        // 3 mm pela linha e 3 mm de arco a partir do canto.
        let on_line = if p[1].abs() < 1e-9 { p } else { q };
        assert!((on_line[0] + 3.).abs() < 1e-9);
        let on_arc = if p[1].abs() < 1e-9 { q } else { p };
        let angle = (on_arc[1]).atan2(on_arc[0] + 30.);
        assert!((angle * 30. - 3.).abs() < 1e-9);
        let (mut other, line, arc) = line_and_arc()?;
        let before = other.clone();
        assert!(crate::corners::refine(&mut other, &[line, arc], &[], 25., true).is_err());
        assert_eq!(other.entities.len(), before.entities.len());
        assert_eq!(other.line(line)?, before.line(line)?);
        Ok(())
    }
    #[test]
    fn fillet_has_exact_radius_tangent_endpoints_and_closed_contour() -> Result<()> {
        let mut sketch = Sketch::new("Canto", Plane::xy());
        sketch.add_rectangle([0., 0.], [40., 20.], false)?;
        let a = sketch.entities[0].id;
        let b = sketch.entities[1].id;
        let arc = refine_corner(&mut sketch, a, b, 3., true)?;
        sketch.validate()?;
        assert!(solver::max_error(&sketch)? < 1e-9);
        assert!((sketch.radial(arc)?.1 - 3.).abs() < 1e-9);
        let (wires, diag) = profiles::analyze(&sketch)?;
        assert_eq!(wires.len(), 1);
        assert!(diag.open_points.is_empty());
        Ok(())
    }
}
