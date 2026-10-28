//! Translação solicitada pelo usuário, sem transformar cotas nem apoios fixos.
//! A resolução exige a nova posição dos pontos escolhidos; restrições externas
//! podem mover vizinhos ou impedir o movimento, sem fazê-lo voltar silenciosamente.
use crate::*;
pub fn points(sketch: &Sketch, selected: &[u64]) -> Result<Vec<u64>> {
    let mut result = HashSet::new();
    for id in selected {
        result.extend(sketch.entity(*id)?.point_ids());
    }
    Ok(sketch
        .points
        .iter()
        .filter(|p| result.contains(&p.id))
        .map(|p| p.id)
        .collect())
}
pub fn translated(sketch: &Sketch, selected: &[u64], delta: Point2) -> Result<Sketch> {
    if selected.is_empty() || delta.iter().any(|v| !v.is_finite() || v.abs() > 1e12) {
        return Err(SketchError(
            "Selecione elementos e um deslocamento finito.".into(),
        ));
    }
    let selected = points(sketch, selected)?;
    let mut next = sketch.clone();
    for point in &mut next.points {
        if selected.contains(&point.id) {
            point.position = add(point.position, delta);
        }
    }
    next.validate()?;
    Ok(next)
}

/// Arrastar impõe destinos temporários ao solver a partir de uma geometria
/// válida. Assim um extremo de arco pode mover-se sem antes produzir um arco
/// inválido; os demais apoios são resolvidos pelas relações reais do desenho.
/// As travas transitórias e seus IDs nunca entram no documento ou no histórico.
pub fn dragged(sketch: &Sketch, points: &[u64], circles: &[u64], delta: Point2) -> Result<Sketch> {
    if points.is_empty() || delta.iter().any(|v| !v.is_finite() || v.abs() > 1e12) {
        return Err(SketchError(
            "Escolha um apoio e um deslocamento finito.".into(),
        ));
    }
    let mut next = sketch.clone();
    let first_temporary = next.next_id;
    for point in points
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
    {
        let position = add(sketch.point(point)?, delta);
        next.add_constraint(ConstraintKind::Fixed { point, position }, false)?;
    }
    for circle in circles {
        if let Curve::Circle { radius, .. } = sketch.entity(*circle)?.curve {
            next.add_constraint(
                ConstraintKind::Radius {
                    circle: *circle,
                    value: radius,
                },
                false,
            )?;
        }
    }
    let report = solver::solve(&next)?;
    if report.status == solver::Status::Conflict {
        return Err(SketchError(
            "As restrições existentes impedem esse movimento.".into(),
        ));
    }
    let mut next = report.sketch;
    next.constraints.retain(|c| c.id < first_temporary);
    next.next_id = first_temporary;
    next.validate()?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vertex_drag_solves_arc_and_strong_links_without_persisting_temporary_locks() -> Result<()> {
        let mut sketch = Sketch::new("Arraste", Plane::xy());
        let center = sketch.add_point([0., 0.])?;
        let a = sketch.add_point([10., 0.])?;
        let b = sketch.add_point([0., 10.])?;
        sketch.add_curve(
            Curve::Arc {
                center,
                start: a,
                end: b,
                clockwise: false,
            },
            false,
        )?;
        let line = sketch.add_line([10., 0.], [20., 0.], false)?;
        let other = sketch.entity(line)?.point_ids()[0];
        sketch.add_constraint(ConstraintKind::Coincident { a, b: other }, false)?;
        let next = dragged(&sketch, &[a], &[], [2., 1.])?;
        assert!(distance(next.point(a)?, [12., 1.]) < 1e-6);
        assert!(distance(next.point(a)?, next.point(other)?) < 1e-6);
        assert_eq!(next.next_id, sketch.next_id);
        assert_eq!(next.constraints.len(), sketch.constraints.len());
        assert!(solver::max_error(&next)? < 1e-6);
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: a,
                position: [10., 0.],
            },
            false,
        )?;
        assert!(dragged(&sketch, &[a], &[], [2., 1.]).is_err());
        assert_eq!(sketch.point(a)?, [10., 0.]);
        Ok(())
    }
    #[test]
    fn moving_preserves_requested_points_and_moves_linked_neighbors() -> Result<()> {
        let mut sketch = Sketch::new("Mover", Plane::xy());
        let a = sketch.add_line([0., 0.], [10., 0.], false)?;
        let b = sketch.add_line([10., 0.], [20., 0.], false)?;
        let p = sketch.entity(a)?.point_ids()[1];
        let q = sketch.entity(b)?.point_ids()[0];
        sketch.add_constraint(ConstraintKind::Coincident { a: p, b: q }, false)?;
        let moved = translated(&sketch, &[a], [5., 3.])?;
        let report = solver::solve_required(&moved, &points(&moved, &[a])?, &[])?;
        assert_ne!(report.status, solver::Status::Conflict);
        assert_eq!(report.sketch.line(a)?, ([5., 3.], [15., 3.]));
        assert!(distance(report.sketch.point(q)?, [15., 3.]) < 1e-6);
        let fixed = sketch.entity(a)?.point_ids()[0];
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: fixed,
                position: [0., 0.],
            },
            false,
        )?;
        let moved = translated(&sketch, &[a], [5., 3.])?;
        assert_eq!(
            solver::solve_required(&moved, &points(&moved, &[a])?, &[])?.status,
            solver::Status::Conflict
        );
        assert_eq!(sketch.point(fixed)?, [0., 0.]);
        Ok(())
    }
}
