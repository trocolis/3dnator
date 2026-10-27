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
            sketch.add_line(add(a, n), add(b, n), entity.construction)
        }
        Curve::Circle { center, radius } => sketch.add_circle(
            sketch.point(center)?,
            radius + distance_mm,
            entity.construction,
        ),
        Curve::Arc { .. } => Err(SketchError("Offset de arco ainda não disponível.".into())),
    }
}
pub fn extend_line(sketch: &mut Sketch, line: u64, target: u64, near: Point2) -> Result<()> {
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
        let d = distance(cursor, point(&curve, closest_parameter(&curve, cursor)));
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
    use crate::curve_geometry::{closest_parameter, intersections, parameter};
    if pick.iter().any(|v| !v.is_finite()) {
        return Err(SketchError(
            "Selecione um trecho válido para aparar.".into(),
        ));
    }
    let curve = profiles::segment(sketch, sketch.entity(entity)?)?;
    let closed = matches!(curve, profiles::Segment::Circle { .. });
    let at = closest_parameter(&curve, pick);
    let mut cuts = if closed { Vec::new() } else { vec![0., 1.] };
    for other in &sketch.entities {
        if other.id == entity {
            continue;
        }
        let target = profiles::segment(sketch, other)?;
        for point in intersections(&curve, &target) {
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

/// Compatibilidade com os chamadores de linhas; o aparador agora aceita também curvas.
pub fn trim_line(sketch: &mut Sketch, line: u64, pick: Point2) -> Result<()> {
    trim(sketch, line, pick)
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
            Curve::Line { start, end } => sketch.add_line(
                reflect(sketch.point(start)?),
                reflect(sketch.point(end)?),
                entity.construction,
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

/// Copia uma entidade aplicando uma transformação rígida que preserva a orientação
/// (translação ou rotação). Pontos novos são criados; a cópia não fica vinculada ao original.
fn copy_rigid(sketch: &mut Sketch, id: u64, transform: &dyn Fn(Point2) -> Point2) -> Result<u64> {
    let entity = sketch.entity(id)?.clone();
    match entity.curve {
        Curve::Line { start, end } => sketch.add_line(
            transform(sketch.point(start)?),
            transform(sketch.point(end)?),
            entity.construction,
        ),
        Curve::Circle { center, radius } => sketch.add_circle(
            transform(sketch.point(center)?),
            radius,
            entity.construction,
        ),
        Curve::Arc {
            center,
            start,
            end,
            clockwise,
        } => {
            let center = sketch.add_point(transform(sketch.point(center)?))?;
            let start = sketch.add_point(transform(sketch.point(start)?))?;
            let end = sketch.add_point(transform(sketch.point(end)?))?;
            sketch.add_curve(
                Curve::Arc {
                    center,
                    start,
                    end,
                    clockwise,
                },
                entity.construction,
            )
        }
    }
}

fn pattern_count(count: usize) -> Result<()> {
    if !(2..=100).contains(&count) {
        return Err(SketchError(
            "O padrão do rascunho deve ter entre 2 e 100 ocorrências.".into(),
        ));
    }
    Ok(())
}

/// Padrão linear: `count` ocorrências (incluindo o original) ao longo de `direction`,
/// espaçadas de `spacing` mm. Devolve as entidades criadas.
pub fn pattern_linear(
    sketch: &mut Sketch,
    ids: &[u64],
    direction: Point2,
    count: usize,
    spacing: f64,
) -> Result<Vec<u64>> {
    pattern_count(count)?;
    let length = distance(direction, [0.; 2]);
    if length < 1e-9 || !spacing.is_finite() || spacing.abs() < 1e-9 {
        return Err(SketchError(
            "Direção e espaçamento do padrão devem ser não nulos.".into(),
        ));
    }
    let step = scale(direction, spacing / length);
    let mut created = Vec::new();
    for k in 1..count {
        let offset = scale(step, k as f64);
        for id in ids {
            created.push(copy_rigid(sketch, *id, &|p| add(p, offset))?);
        }
    }
    Ok(created)
}

/// Padrão circular: `count` ocorrências distribuídas por igual em volta de `center`.
pub fn pattern_circular(
    sketch: &mut Sketch,
    ids: &[u64],
    center: Point2,
    count: usize,
) -> Result<Vec<u64>> {
    pattern_count(count)?;
    let mut created = Vec::new();
    for k in 1..count {
        let (s, c) = (std::f64::consts::TAU * k as f64 / count as f64).sin_cos();
        let rotate = |p: Point2| {
            let v = sub(p, center);
            add(center, [c * v[0] - s * v[1], s * v[0] + c * v[1]])
        };
        for id in ids {
            created.push(copy_rigid(sketch, *id, &rotate)?);
        }
    }
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_and_circular_patterns_copy_entities_rigidly() -> Result<()> {
        let mut sketch = Sketch::new("Padrões", Plane::xy());
        let hole = sketch.add_circle([10., 5.], 2., false)?;
        let copies = pattern_linear(&mut sketch, &[hole], [1., 0.], 3, 15.)?;
        assert_eq!(copies.len(), 2);
        assert_eq!(sketch.circle(copies[1])?, ([40., 5.], 2.));
        let spoke = sketch.add_line([1., 0.], [4., 0.], false)?;
        let ring = pattern_circular(&mut sketch, &[spoke], [0., 0.], 4)?;
        let (a, b) = sketch.line(ring[0])?;
        assert!(distance(a, [0., 1.]) < 1e-12 && distance(b, [0., 4.]) < 1e-12);
        sketch.validate()?;
        assert!(pattern_linear(&mut sketch, &[hole], [0., 0.], 3, 5.).is_err());
        assert!(pattern_circular(&mut sketch, &[hole], [0., 0.], 1).is_err());
        Ok(())
    }

    #[test]
    fn trim_preserves_the_unselected_pieces() -> Result<()> {
        let mut sketch = Sketch::new("Aparar", Plane::xy());
        let line = sketch.add_line([0., 0.], [30., 0.], false)?;
        sketch.add_line([10., -5.], [10., 5.], false)?;
        sketch.add_line([20., -5.], [20., 5.], false)?;
        trim_line(&mut sketch, line, [15., 0.])?;
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
    if first == second || !size.is_finite() || size <= 1e-9 {
        return Err(SketchError(
            "Selecione duas linhas e uma dimensão positiva.".into(),
        ));
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
#[cfg(test)]
mod corner_tests {
    use super::*;
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
