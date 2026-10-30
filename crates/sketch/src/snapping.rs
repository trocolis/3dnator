//! Atração em unidades do plano, com prioridade para referências exatas e guias temporárias.
use crate::*;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Endpoint(u64),
    Midpoint(u64),
    CurveMidpoint(u64),
    RegionCenter,
    Center(u64),
    OnCurve(u64),
    Intersection,
    Projected,
    Alignment,
    GridIntersection,
    GridMidpoint,
    GridCenter,
    GridLine,
}
#[derive(Debug, Clone)]
pub struct Snap {
    pub point: Point2,
    pub kind: Kind,
    pub guides: Vec<(Point2, Point2)>,
}
#[derive(Debug, Clone, Default)]
pub struct Tracker {
    references: Vec<Point2>,
}
impl Tracker {
    pub fn capture(&mut self, p: Point2) {
        if self.references.iter().any(|q| distance(*q, p) < 1e-7) {
            return;
        }
        self.references.push(p);
        if self.references.len() > 8 {
            self.references.remove(0);
        }
    }
    pub fn clear(&mut self) {
        self.references.clear();
    }
    pub fn references(&self) -> &[Point2] {
        &self.references
    }
}

/// Captura a grade visível no plano. Usada somente quando não há geometria
/// ou guia exata: a grade posiciona e não cria vínculos dirigentes.
pub fn grid(cursor: Point2, tolerance: f64, step: f64) -> Result<Option<Snap>> {
    if cursor.iter().any(|v| !v.is_finite())
        || !tolerance.is_finite()
        || tolerance <= 0.
        || !step.is_finite()
        || step <= 0.
    {
        return Err(SketchError(
            "Parâmetros de captura da grade inválidos.".into(),
        ));
    }
    let near = cursor.map(|v| (v / step).round() * step);
    let middle = cursor.map(|v| ((v / step).floor() + 0.5) * step);
    let points = [
        (near, Kind::GridIntersection),
        ([near[0], middle[1]], Kind::GridMidpoint),
        ([middle[0], near[1]], Kind::GridMidpoint),
        (middle, Kind::GridCenter),
    ];
    let tolerance = tolerance.min(step * 0.22);
    if let Some((point, kind)) = points
        .into_iter()
        .filter(|(p, _)| distance(*p, cursor) <= tolerance)
        .min_by(|(a, _), (b, _)| distance(*a, cursor).total_cmp(&distance(*b, cursor)))
    {
        return Ok(Some(Snap {
            point,
            kind,
            guides: vec![],
        }));
    }
    let lines = [
        (
            [near[0], cursor[1]],
            vec![(
                [near[0], cursor[1] - step / 2.],
                [near[0], cursor[1] + step / 2.],
            )],
        ),
        (
            [cursor[0], near[1]],
            vec![(
                [cursor[0] - step / 2., near[1]],
                [cursor[0] + step / 2., near[1]],
            )],
        ),
    ];
    Ok(lines
        .into_iter()
        .filter(|(p, _)| distance(*p, cursor) <= tolerance)
        .min_by(|(a, _), (b, _)| distance(*a, cursor).total_cmp(&distance(*b, cursor)))
        .map(|(point, guides)| Snap {
            point,
            kind: Kind::GridLine,
            guides,
        }))
}

/// Combina a grade com guias de alinhamento sem deslocar um apoio geométrico.
pub fn with_grid(
    exact: Option<Snap>,
    cursor: Point2,
    tolerance: f64,
    step: f64,
) -> Result<Option<Snap>> {
    match exact {
        Some(mut snap) if snap.kind == Kind::Alignment => {
            if let Some(grid) = grid(snap.point, tolerance, step)? {
                let mut point = grid.point;
                for (a, b) in &snap.guides {
                    if (a[0] - b[0]).abs() < 1e-9 {
                        point[0] = snap.point[0];
                    }
                    if (a[1] - b[1]).abs() < 1e-9 {
                        point[1] = snap.point[1];
                    }
                }
                snap.point = point;
                snap.guides.extend(grid.guides);
            }
            Ok(Some(snap))
        }
        Some(snap) => Ok(Some(snap)),
        None => grid(cursor, tolerance, step),
    }
}

pub fn find(
    sketch: &Sketch,
    cursor: Point2,
    tolerance: f64,
    tracker: &Tracker,
    projected: &[Point2],
) -> Result<Option<Snap>> {
    let references = crate::topology::references(sketch)?;
    find_with_references(sketch, cursor, tolerance, tracker, projected, &references)
}

/// Consulta por quadro usando encontros/centros já calculados para esta geometria.
pub fn find_with_references(
    sketch: &Sketch,
    cursor: Point2,
    tolerance: f64,
    tracker: &Tracker,
    projected: &[Point2],
    references: &crate::topology::References,
) -> Result<Option<Snap>> {
    if !tolerance.is_finite() || tolerance <= 0. {
        return Err(SketchError("Tolerância de captura inválida.".into()));
    }
    let mut candidates = Vec::new();
    for entity in &sketch.entities {
        match entity.curve {
            Curve::Line { start, end } => {
                let (a, b) = (sketch.point(start)?, sketch.point(end)?);
                if !entity.infinite {
                    candidates.extend([
                        (a, Kind::Endpoint(start), 0),
                        (b, Kind::Endpoint(end), 0),
                        (scale(add(a, b), 0.5), Kind::Midpoint(entity.id), 1),
                    ]);
                }
                candidates.push((
                    lines::closest(sketch, entity, cursor)?,
                    Kind::OnCurve(entity.id),
                    3,
                ));
            }
            Curve::Circle { center, radius } => {
                let c = sketch.point(center)?;
                candidates.push((c, Kind::Center(center), 0));
                let d = distance(c, cursor);
                if d > 1e-10 {
                    candidates.push((
                        add(c, scale(sub(cursor, c), radius / d)),
                        Kind::OnCurve(entity.id),
                        3,
                    ));
                }
            }
            Curve::Arc {
                center, start, end, ..
            } => {
                candidates.extend([
                    (sketch.point(center)?, Kind::Center(center), 0),
                    (sketch.point(start)?, Kind::Endpoint(start), 0),
                    (sketch.point(end)?, Kind::Endpoint(end), 0),
                ]);
                let curve = profiles::segment(sketch, entity)?;
                candidates.push((
                    crate::curve_geometry::point(&curve, 0.5),
                    Kind::CurveMidpoint(entity.id),
                    1,
                ));
                let t = crate::curve_geometry::closest_parameter(&curve, cursor);
                candidates.push((
                    crate::curve_geometry::point(&curve, t),
                    Kind::OnCurve(entity.id),
                    3,
                ));
            }
        }
    }
    candidates.extend(
        references
            .vertices
            .iter()
            .map(|p| (*p, Kind::Intersection, 0)),
    );
    // Um cruzamento local tem precedência sobre a origem/apoios externos no
    // mesmo lugar; assim a intenção fraca do cruzamento não vira uma fixação.
    for p in projected {
        candidates.push((*p, Kind::Projected, 0));
    }
    candidates.extend(
        references
            .centers
            .iter()
            .map(|p| (*p, Kind::RegionCenter, 0)),
    );
    candidates.retain(|(p, _, _)| distance(*p, cursor) <= tolerance);
    candidates.sort_by(|(a, _, pa), (b, _, pb)| {
        pa.cmp(pb)
            .then_with(|| distance(*a, cursor).total_cmp(&distance(*b, cursor)))
    });
    if let Some((point, kind, _)) = candidates.first().copied() {
        return Ok(Some(Snap {
            point,
            kind,
            guides: Vec::new(),
        }));
    }
    let x = tracker
        .references
        .iter()
        .rev()
        .find(|p| (p[0] - cursor[0]).abs() < tolerance);
    let y = tracker
        .references
        .iter()
        .rev()
        .find(|p| (p[1] - cursor[1]).abs() < tolerance);
    if x.is_none() && y.is_none() {
        return Ok(None);
    }
    let point = [x.map_or(cursor[0], |p| p[0]), y.map_or(cursor[1], |p| p[1])];
    let guides = x.into_iter().chain(y).map(|p| (*p, point)).collect();
    Ok(Some(Snap {
        point,
        kind: Kind::Alignment,
        guides,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn midpoint_and_multiple_reference_alignment_are_exact() -> Result<()> {
        let mut s = Sketch::new("Ensaio", Plane::xy());
        let id = s.add_line([0., 0.], [20., 0.], false)?;
        let snap = find(&s, [10.1, 0.2], 0.5, &Tracker::default(), &[])?
            .ok_or_else(|| SketchError("Captura ausente".into()))?;
        assert_eq!(snap.kind, Kind::Midpoint(id));
        assert_eq!(snap.point, [10., 0.]);
        let mut tracker = Tracker::default();
        tracker.capture([30., 50.]);
        tracker.capture([10., 40.]);
        let snap = find(&s, [30.1, 40.2], 0.5, &tracker, &[])?
            .ok_or_else(|| SketchError("Guia ausente".into()))?;
        assert_eq!(snap.point, [30., 40.]);
        assert_eq!(snap.guides.len(), 2);
        Ok(())
    }

    #[test]
    fn arc_midpoint_is_on_the_curve_and_has_no_line_midpoint_constraint() -> Result<()> {
        let mut sketch = Sketch::new("Metade do arco", Plane::xy());
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
        let snap = find(&sketch, [10.1, 0.1], 0.5, &Tracker::default(), &[])?
            .ok_or_else(|| SketchError("Ponto médio ausente".into()))?;
        assert_eq!(snap.kind, Kind::CurveMidpoint(arc));
        assert!(distance(snap.point, [10., 0.]) < 1e-9);
        Ok(())
    }
}

#[cfg(test)]
mod grid_tests {
    use super::*;
    #[test]
    fn grid_captures_crossings_segment_midpoints_cells_and_lines_without_constraints()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        for (cursor, point, kind) in [
            ([10.12, 20.05], [10., 20.], Kind::GridIntersection),
            ([10.09, 15.1], [10., 15.], Kind::GridMidpoint),
            ([15.1, 20.09], [15., 20.], Kind::GridMidpoint),
            ([15.1, 15.1], [15., 15.], Kind::GridCenter),
            ([10.07, 12.4], [10., 12.4], Kind::GridLine),
            ([-4.95, -4.95], [-5., -5.], Kind::GridCenter),
        ] {
            let snap = grid(cursor, 0.2, 10.)?.ok_or("Grade não capturada")?;
            assert_eq!(snap.kind, kind);
            assert!(distance(snap.point, point) < 1e-9);
            assert!(!crate::junctions::default_strong(Some(kind)));
        }
        assert!(grid([12.4, 12.4], 0.2, 10.)?.is_none());
        let guide = Snap {
            point: [20.1, 5.5],
            kind: Kind::Alignment,
            guides: vec![([0., 5.5], [20.1, 5.5])],
        };
        let merged = with_grid(Some(guide), [20.1, 5.6], 0.8, 10.)?.ok_or("Guia")?;
        assert_eq!(merged.point, [20., 5.5]);
        let exact = Snap {
            point: [20.1, 5.1],
            kind: Kind::OnCurve(1),
            guides: vec![],
        };
        assert_eq!(
            with_grid(Some(exact), [20.1, 5.1], 0.8, 10.)?
                .ok_or("Geometria")?
                .point,
            [20.1, 5.1]
        );
        assert!(grid([0., 0.], 0.2, 0.).is_err());
        assert!(grid([f64::NAN, 0.], 0.2, 10.).is_err());
        Ok(())
    }
}
