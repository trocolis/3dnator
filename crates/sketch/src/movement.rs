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

#[cfg(test)]
mod tests {
    use super::*;
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
