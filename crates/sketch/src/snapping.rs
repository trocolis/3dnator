//! Atração em unidades do plano, com prioridade para referências exatas e guias temporárias.
use crate::*;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Endpoint(u64),
    Midpoint(u64),
    Center(u64),
    OnCurve(u64),
    Intersection,
    Projected,
    Alignment,
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

pub fn find(
    sketch: &Sketch,
    cursor: Point2,
    tolerance: f64,
    tracker: &Tracker,
    projected: &[Point2],
) -> Result<Option<Snap>> {
    if !tolerance.is_finite() || tolerance <= 0. {
        return Err(SketchError("Tolerância de captura inválida.".into()));
    }
    let mut candidates = Vec::new();
    for entity in &sketch.entities {
        match entity.curve {
            Curve::Line { start, end } => {
                let (a, b) = (sketch.point(start)?, sketch.point(end)?);
                candidates.extend([
                    (a, Kind::Endpoint(start), 0),
                    (b, Kind::Endpoint(end), 0),
                    (scale(add(a, b), 0.5), Kind::Midpoint(entity.id), 1),
                ]);
                let direction = sub(b, a);
                let t = (dot(sub(cursor, a), direction) / dot(direction, direction)).clamp(0., 1.);
                candidates.push((add(a, scale(direction, t)), Kind::OnCurve(entity.id), 3));
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
                let points = profiles::segment(sketch, entity)?.sample(128);
                if let Some(p) = points
                    .into_iter()
                    .min_by(|a, b| distance(*a, cursor).total_cmp(&distance(*b, cursor)))
                {
                    candidates.push((p, Kind::OnCurve(entity.id), 3));
                }
            }
        }
    }
    for p in projected {
        candidates.push((*p, Kind::Projected, 0));
    }
    for (i, a) in sketch.entities.iter().enumerate() {
        let Ok((a0, a1)) = sketch.line(a.id) else {
            continue;
        };
        for b in &sketch.entities[i + 1..] {
            let Ok((b0, b1)) = sketch.line(b.id) else {
                continue;
            };
            if let Some((p, _, _)) = profiles::line_intersection(a0, a1, b0, b1)
                .filter(|(_, t, u)| *t >= 0. && *t <= 1. && *u >= 0. && *u <= 1.)
            {
                candidates.push((p, Kind::Intersection, 0));
            }
        }
    }
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
}
