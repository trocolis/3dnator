//! Seleção de componentes ligados por junções fortes. Cruzamentos virtuais e
//! coincidências fracas não unem componentes só porque ocupam o mesmo lugar.
use crate::*;

pub fn entities(sketch: &Sketch, seeds: &[u64]) -> Result<Vec<u64>> {
    for id in seeds {
        sketch.entity(*id)?;
    }
    let mut chosen = seeds.iter().copied().collect::<HashSet<_>>();
    loop {
        let before = chosen.len();
        let mut points = sketch
            .entities
            .iter()
            .filter(|e| chosen.contains(&e.id))
            .flat_map(Entity::point_ids)
            .collect::<HashSet<_>>();
        loop {
            let previous = points.len();
            // Cotas e relações dirigentes também ligam elementos separados,
            // como um círculo mantido a uma distância de uma reta.
            for constraint in sketch.constraints.iter().filter(|c| !c.reference) {
                let (linked_points, linked_entities) = dimensions::participants(&constraint.kind);
                if linked_points.iter().any(|p| points.contains(p))
                    || linked_entities.iter().any(|e| chosen.contains(e))
                {
                    points.extend(linked_points);
                    chosen.extend(linked_entities);
                }
            }
            if points.len() == previous {
                break;
            }
        }
        for entity in &sketch.entities {
            if entity.point_ids().iter().any(|p| points.contains(p)) {
                chosen.insert(entity.id);
            }
        }
        if chosen.len() == before {
            break;
        }
    }
    Ok(sketch
        .entities
        .iter()
        .filter(|e| chosen.contains(&e.id))
        .map(|e| e.id)
        .collect())
}

pub fn center_is_linked(sketch: &Sketch, circle: u64) -> Result<bool> {
    let Curve::Circle { center, .. } = sketch.entity(circle)?.curve else {
        return Ok(true);
    };
    if sketch
        .entities
        .iter()
        .any(|e| e.id != circle && e.point_ids().contains(&center))
    {
        return Ok(true);
    }
    Ok(sketch
        .constraints
        .iter()
        .filter(|c| !c.reference)
        .any(|c| match c.kind {
            ConstraintKind::Coincident { a, b }
            | ConstraintKind::Distance { a, b, .. }
            | ConstraintKind::DeltaX { a, b, .. }
            | ConstraintKind::DeltaY { a, b, .. }
            | ConstraintKind::Angle { a, b, .. } => a == center || b == center,
            ConstraintKind::Midpoint { point, .. }
            | ConstraintKind::PointOnLine { point, .. }
            | ConstraintKind::PointOnCurve { point, .. }
            | ConstraintKind::PointLineDistance { point, .. } => point == center,
            ConstraintKind::Concentric { a, b } => a == circle || b == circle,
            _ => false,
        }))
}

/// Cadeia geométrica contígua, interrompida nas ramificações. Serve para escolher
/// um trecho inteiro no deslocamento sem atravessar cruzamentos interiores.
pub fn chain(sketch: &Sketch, seed: u64) -> Result<Vec<u64>> {
    let entity = sketch.entity(seed)?;
    if entity.infinite || matches!(entity.curve, Curve::Circle { .. }) {
        return Ok(vec![seed]);
    }
    let ends = |e: &Entity| -> Result<Option<[Point2; 2]>> {
        Ok(match e.curve {
            Curve::Line { start, end } | Curve::Arc { start, end, .. } if !e.infinite => {
                Some([sketch.point(start)?, sketch.point(end)?])
            }
            _ => None,
        })
    };
    let candidates = sketch
        .entities
        .iter()
        .filter(|e| e.construction == entity.construction)
        .map(|e| Ok((e.id, ends(e)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut chosen = HashSet::from([seed]);
    let mut pending = ends(entity)?.into_iter().flatten().collect::<Vec<_>>();
    while let Some(point) = pending.pop() {
        let touching = candidates
            .iter()
            .filter(|(_, ends)| {
                ends.is_some_and(|ends| ends.iter().any(|p| distance(*p, point) < 1e-6))
            })
            .collect::<Vec<_>>();
        if touching.len() != 2 {
            continue;
        }
        for (id, ends) in touching {
            if chosen.insert(*id) {
                pending.extend(ends.iter().flatten().copied());
            }
        }
    }
    Ok(sketch
        .entities
        .iter()
        .filter(|e| chosen.contains(&e.id))
        .map(|e| e.id)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strong_components_ignore_weak_links_and_virtual_crossings() -> Result<()> {
        let mut sketch = Sketch::new("Conectados", Plane::xy());
        let a = sketch.add_line([0., 0.], [10., 0.], false)?;
        let b = sketch.add_line([10., 0.], [10., 10.], false)?;
        let c = sketch.add_line([10., 10.], [20., 10.], false)?;
        sketch.add_line([5., -5.], [5., 5.], false)?;
        let pa = sketch.entity(a)?.point_ids()[1];
        let pb = sketch.entity(b)?.point_ids()[0];
        let pc = sketch.entity(b)?.point_ids()[1];
        let pd = sketch.entity(c)?.point_ids()[0];
        sketch.add_constraint(ConstraintKind::Coincident { a: pa, b: pb }, false)?;
        sketch.add_constraint(ConstraintKind::Coincident { a: pc, b: pd }, true)?;
        assert_eq!(entities(&sketch, &[a])?, vec![a, b]);
        assert_eq!(chain(&sketch, a)?, vec![a, b, c]);
        sketch.add_constraint(
            ConstraintKind::Distance {
                a: pa,
                b: pd,
                value: distance(sketch.point(pa)?, sketch.point(pd)?),
            },
            false,
        )?;
        assert_eq!(entities(&sketch, &[a])?, vec![a, b, c]);
        Ok(())
    }
    #[test]
    fn circle_centers_require_a_real_connection() -> Result<()> {
        let mut sketch = Sketch::new("Centro", Plane::xy());
        let circle = sketch.add_circle([0., 0.], 5., false)?;
        assert!(!center_is_linked(&sketch, circle)?);
        let center = sketch.entity(circle)?.point_ids()[0];
        let point = sketch.add_point([0., 0.])?;
        let link = sketch.add_constraint(
            ConstraintKind::Coincident {
                a: center,
                b: point,
            },
            true,
        )?;
        assert!(!center_is_linked(&sketch, circle)?);
        if let Some(c) = sketch.constraints.iter_mut().find(|c| c.id == link) {
            c.reference = false;
        }
        assert!(center_is_linked(&sketch, circle)?);
        Ok(())
    }
}
