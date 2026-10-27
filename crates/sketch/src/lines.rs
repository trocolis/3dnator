//! Referências de direção e captura perpendicular no plano local (mm/radianos).
//! A reta infinita usa a mesma parametrização das restrições lineares, sem limite
//! de parâmetro; seus dois apoios internos nunca são tratados como extremos.
use crate::*;

pub fn angle(sketch: &Sketch, line: u64) -> Result<f64> {
    let (a, b) = sketch.line(line)?;
    let d = sub(b, a);
    Ok(d[1].atan2(d[0]))
}

pub fn relative_angle(angle: f64, reference: f64) -> f64 {
    let delta = angle - reference;
    delta.sin().atan2(delta.cos())
}

pub fn closest(sketch: &Sketch, entity: &Entity, cursor: Point2) -> Result<Point2> {
    let (a, b) = sketch.line(entity.id)?;
    let d = sub(b, a);
    let t = dot(sub(cursor, a), d) / dot(d, d);
    Ok(add(
        a,
        scale(d, if entity.infinite { t } else { t.clamp(0., 1.) }),
    ))
}

/// Em encontros com mais de uma linha, a menor distância e depois o ID tornam
/// a escolha estável. `preferred` conserva a entidade indicada pela captura.
pub fn reference(
    sketch: &Sketch,
    point: Point2,
    tolerance: f64,
    preferred: Option<u64>,
) -> Option<u64> {
    sketch
        .entities
        .iter()
        .filter(|e| matches!(e.curve, Curve::Line { .. }))
        .filter_map(|e| {
            closest(sketch, e, point)
                .ok()
                .map(|p| (e.id, distance(p, point)))
        })
        .filter(|(_, d)| *d <= tolerance)
        .min_by(|(a, da), (b, db)| {
            (Some(*a) != preferred)
                .cmp(&(Some(*b) != preferred))
                .then(da.total_cmp(db))
                .then(a.cmp(b))
        })
        .map(|(id, _)| id)
}

#[derive(Debug, Clone, Copy)]
pub struct PerpendicularSnap {
    pub point: Point2,
    pub line: u64,
    /// A ponta pousou na referência: além da perpendicularidade, pode receber
    /// incidência. Ao apenas cruzar, a ponta permanece além da interseção.
    pub on_line: bool,
    pub foot: Point2,
}

pub fn perpendicular(
    sketch: &Sketch,
    start: Point2,
    cursor: Point2,
    tolerance: f64,
    source: Option<u64>,
) -> Result<Option<PerpendicularSnap>> {
    let delta = sub(cursor, start);
    let length = distance(start, cursor);
    if !tolerance.is_finite() || tolerance <= 0. || length < 1e-9 {
        return Ok(None);
    }
    let mut best: Option<(f64, PerpendicularSnap)> = None;
    for entity in sketch
        .entities
        .iter()
        .filter(|e| matches!(e.curve, Curve::Line { .. }))
    {
        let (a, b) = sketch.line(entity.id)?;
        let unit = scale(sub(b, a), 1. / distance(a, b));
        // Janela angular de 3°, limitada também por uma tolerância em tela.
        // Isso evita saltos grandes em linhas longas ou ímãs em referências distantes.
        if (dot(delta, unit) / length).abs() > 3_f64.to_radians().sin() {
            continue;
        }
        let normal = [-unit[1], unit[0]];
        let foot = add(a, scale(unit, dot(sub(start, a), unit)));
        let along = dot(sub(foot, start), normal);
        let travel = dot(delta, normal);
        let projected = add(start, scale(normal, travel));
        if entity.id != source.unwrap_or(0) {
            if travel.abs() < 1e-9
                || along / travel < 0.
                || along / travel > 1. + tolerance / length
            {
                continue;
            }
            if distance(closest(sketch, entity, foot)?, foot) > 1e-7 {
                continue;
            }
        }
        let on_line = distance(projected, foot) <= tolerance && distance(start, foot) > 1e-9;
        let point = if on_line { foot } else { projected };
        let correction = distance(point, cursor);
        if correction > tolerance {
            continue;
        }
        let snap = PerpendicularSnap {
            point,
            line: entity.id,
            on_line,
            foot,
        };
        if best.as_ref().is_none_or(|(d, _)| correction < *d) {
            best = Some((correction, snap));
        }
    }
    Ok(best.map(|(_, s)| s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infinite_references_have_no_endpoint_snaps_and_do_not_partition_areas() -> Result<()> {
        let mut sketch = Sketch::new("Retas auxiliares", Plane::xy());
        sketch.add_rectangle([-20., -10.], [20., 10.], false)?;
        let infinite = sketch.add_line_with_type([0., 0.], [1., 0.], LineType::Infinite)?;
        sketch.add_line_with_type([0., -4.], [0., 4.], LineType::Construction)?;
        let regions = areas::regions(&sketch)?;
        assert_eq!(regions.len(), 1);
        assert!((regions[0].area() - 800.).abs() < 1e-8);
        let refs = topology::references(&sketch)?;
        assert!(refs.vertices.contains(&[-20., 0.]));
        assert!(refs.vertices.contains(&[20., 0.]));
        assert!(sketch.visible_points().all(|p| {
            !sketch
                .entity(infinite)
                .is_ok_and(|e| e.point_ids().contains(&p.id))
        }));
        let snap = snapping::find(
            &sketch,
            [1000., 0.1],
            0.5,
            &snapping::Tracker::default(),
            &[],
        )?
        .ok_or_else(|| SketchError("Reta não capturada além dos apoios".into()))?;
        assert_eq!(snap.kind, snapping::Kind::OnCurve(infinite));
        assert_eq!(snap.point, [1000., 0.]);
        assert_eq!(
            tools::pick_entity(&sketch, [-1000., 0.1], 0.5)?,
            Some(infinite)
        );
        assert!(tools::trim(&mut sketch, infinite, [1000., 0.]).is_err());
        let offset = tools::offset_entity(&mut sketch, infinite, 4.)?;
        let copies = tools::mirror(&mut sketch, &[offset], [0., 0.], [0., 1.])?;
        let pattern = tools::pattern_linear(&mut sketch, &copies, [1., 0.], 3, 5.)?;
        for id in [offset].into_iter().chain(copies).chain(pattern) {
            assert!(sketch.entity(id)?.infinite);
            assert!(sketch.entity(id)?.construction);
        }
        sketch.validate()?;
        Ok(())
    }

    #[test]
    fn perpendicular_capture_distinguishes_start_end_crossing_and_distant_lines() -> Result<()> {
        let mut sketch = Sketch::new("Captura perpendicular", Plane::xy());
        let line = sketch.add_line([0., 10.], [20., 10.], false)?;
        let snap = perpendicular(&sketch, [5., 0.], [5.2, 10.1], 0.5, None)?
            .ok_or_else(|| SketchError("Término perpendicular não capturado".into()))?;
        assert_eq!(snap.line, line);
        assert!(snap.on_line);
        assert_eq!(snap.point, [5., 10.]);
        let snap = perpendicular(&sketch, [5., 0.], [5.2, 15.], 0.5, None)?
            .ok_or_else(|| SketchError("Cruzamento perpendicular não capturado".into()))?;
        assert!(!snap.on_line);
        assert_eq!(snap.point, [5., 15.]);
        assert!(perpendicular(&sketch, [35., 0.], [35.2, 15.], 0.5, None)?.is_none());
        assert!(perpendicular(&sketch, [5., 0.], [8., 10.], 0.5, None)?.is_none());
        let mut source = Sketch::new("Origem inclinada", Plane::xy());
        let line = source.add_line([0., 0.], [10., 10.], false)?;
        let snap = perpendicular(&source, [10., 10.], [-0.1, 20.], 0.5, Some(line))?
            .ok_or_else(|| SketchError("Origem perpendicular não capturada".into()))?;
        assert!(dot(sub(snap.point, [10., 10.]), [10., 10.]).abs() < 1e-8);
        assert_eq!(reference(&source, [5., 5.], 0.1, None), Some(line));
        assert_eq!(reference(&source, [5., 7.], 0.1, None), None);
        Ok(())
    }

    #[test]
    fn relative_angular_constraint_follows_the_source_direction_after_recalculation() -> Result<()>
    {
        let mut sketch = Sketch::new("Ângulo relativo", Plane::xy());
        let reference = sketch.add_line([0., 0.], [10., 0.], false)?;
        let line = sketch.add_line([10., 0.], [20., 10.], false)?;
        let Curve::Line {
            start: source_start,
            end: source_end,
        } = sketch.entity(reference)?.curve
        else {
            return Err(SketchError("Referência ausente".into()));
        };
        let Curve::Line { start, end } = sketch.entity(line)?.curve else {
            return Err(SketchError("Linha ausente".into()));
        };
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: source_start,
                position: [0., 0.],
            },
            false,
        )?;
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: source_end,
                position: [0., 10.],
            },
            false,
        )?;
        sketch.add_constraint(
            ConstraintKind::Coincident {
                a: start,
                b: source_end,
            },
            false,
        )?;
        sketch.add_constraint(
            ConstraintKind::Distance {
                a: start,
                b: end,
                value: 20.,
            },
            false,
        )?;
        sketch.add_constraint(
            ConstraintKind::LineAngle {
                line,
                reference,
                value: std::f64::consts::FRAC_PI_4,
            },
            false,
        )?;
        let report = solver::solve(&sketch)?;
        assert_ne!(report.status, solver::Status::Conflict);
        assert!(report.max_error < 1e-6);
        let expected = 3. * std::f64::consts::FRAC_PI_4;
        assert!(relative_angle(angle(&report.sketch, line)?, expected).abs() < 1e-6);
        assert!(distance(report.sketch.point(start)?, [0., 10.]) < 1e-6);
        Ok(())
    }
}
