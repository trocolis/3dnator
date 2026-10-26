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
pub fn trim_line(sketch: &mut Sketch, line: u64, pick: Point2) -> Result<()> {
    let (a, b) = sketch.line(line)?;
    let direction = sub(b, a);
    let at = dot(sub(pick, a), direction) / dot(direction, direction);
    let mut cuts = vec![0., 1.];
    for target in &sketch.entities {
        if target.id == line {
            continue;
        }
        let Ok((c, d)) = sketch.line(target.id) else {
            continue;
        };
        if let Some((_, t, u)) = profiles::line_intersection(a, b, c, d)
            .filter(|(_, t, u)| *t > 1e-9 && *t < 1. - 1e-9 && *u >= 0. && *u <= 1.)
        {
            let _ = u;
            cuts.push(t);
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let interval = cuts
        .windows(2)
        .find(|p| at >= p[0] && at <= p[1])
        .map(|p| [p[0], p[1]])
        .ok_or_else(|| SketchError("Selecione um trecho da linha.".into()))?;
    let construction = sketch.entity(line)?.construction;
    sketch.entities.retain(|e| e.id != line);
    if interval[0] > 1e-9 {
        sketch.add_line(a, add(a, scale(direction, interval[0])), construction)?;
    }
    if interval[1] < 1. - 1e-9 {
        sketch.add_line(add(a, scale(direction, interval[1])), b, construction)?;
    }
    sketch.prune_unused_points();
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

#[cfg(test)]
mod tests {
    use super::*;
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
