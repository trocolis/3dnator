//! Política de movimento das cotas: preservar o elemento mais conectado; no
//! empate, preservar o segundo escolhido. Ligações fracas não contam como apoio.
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Point(u64),
    Entity(u64),
}

impl Target {
    pub fn points(self, sketch: &Sketch) -> Result<Vec<u64>> {
        match self {
            Self::Point(id) => {
                sketch.point(id)?;
                Ok(vec![id])
            }
            Self::Entity(id) => Ok(sketch.entity(id)?.point_ids()),
        }
    }
    fn entities(self, sketch: &Sketch) -> Result<HashSet<u64>> {
        Ok(match self {
            Self::Entity(id) => {
                sketch.entity(id)?;
                [id].into_iter().collect()
            }
            Self::Point(id) => {
                sketch.point(id)?;
                sketch
                    .entities
                    .iter()
                    .filter(|e| e.point_ids().contains(&id))
                    .map(|e| e.id)
                    .collect()
            }
        })
    }
}

/// Apoios e entidades que participam de uma restrição, sem inferir conexões pela
/// mera proximidade. IDs locais de pontos e de entidades compartilham o contador.
fn participants(kind: &ConstraintKind) -> (Vec<u64>, Vec<u64>) {
    use ConstraintKind::*;
    match *kind {
        Fixed { point, .. } => (vec![point], vec![]),
        Coincident { a, b }
        | Distance { a, b, .. }
        | DeltaX { a, b, .. }
        | DeltaY { a, b, .. }
        | Angle { a, b, .. } => (vec![a, b], vec![]),
        Horizontal { line } | Vertical { line } => (vec![], vec![line]),
        Radius { circle, .. } | Diameter { circle, .. } => (vec![], vec![circle]),
        PointOnLine { point, line }
        | Midpoint { point, line }
        | PointLineDistance { point, line, .. } => (vec![point], vec![line]),
        PointOnCurve { point, curve } => (vec![point], vec![curve]),
        LineAngle {
            line, reference, ..
        } => (vec![], vec![line, reference]),
        LineCircleDistance { line, circle, .. } => (vec![], vec![line, circle]),
        Parallel { a, b } | Perpendicular { a, b } | EqualLength { a, b } | Concentric { a, b } => {
            (vec![], vec![a, b])
        }
        Tangent { line, arc, point } => (vec![point], vec![line, arc]),
    }
}

pub fn connections(sketch: &Sketch, target: Target, excluded: u64) -> Result<usize> {
    let points = target.points(sketch)?;
    let owned = target.entities(sketch)?;
    let mut neighbors = HashSet::new();
    // Compartilhar um ID é uma ligação forte, mesmo sem Coincident explícito.
    for entity in &sketch.entities {
        if !owned.contains(&entity.id) && entity.point_ids().iter().any(|p| points.contains(p)) {
            neighbors.insert(entity.id);
        }
    }
    let mut anchors = 0;
    for constraint in sketch
        .constraints
        .iter()
        .filter(|c| !c.reference && c.id != excluded)
    {
        let (mut linked_points, entities) = participants(&constraint.kind);
        let touches = linked_points.iter().any(|p| points.contains(p))
            || entities.iter().any(|e| owned.contains(e));
        if !touches {
            continue;
        }
        for entity in &entities {
            linked_points.extend(sketch.entity(*entity)?.point_ids());
        }
        for entity in &sketch.entities {
            if !owned.contains(&entity.id)
                && (entities.contains(&entity.id)
                    || entity.point_ids().iter().any(|p| linked_points.contains(p)))
            {
                neighbors.insert(entity.id);
            }
        }
        // Uma cota dirigente ou fixação também é apoio efetivo. As regras
        // internas Horizontal/Vertical não fabricam vizinhos nem peso adicional.
        if constraint.kind.is_dimension() || matches!(constraint.kind, ConstraintKind::Fixed { .. })
        {
            anchors += 1;
        }
    }
    let shared = if matches!(target, Target::Point(_)) {
        owned.len().saturating_sub(1)
    } else {
        0
    };
    Ok(neighbors.len() + anchors + shared)
}

pub fn preferred(sketch: &Sketch, pair: [Target; 2], excluded: u64) -> Result<Target> {
    Ok(
        if connections(sketch, pair[0], excluded)? > connections(sketch, pair[1], excluded)? {
            pair[0]
        } else {
            pair[1]
        },
    )
}

/// Projetos antigos não guardam a ordem dos cliques; os campos da restrição dão
/// uma ordem determinística. Cotas de uma única curva continuam sem dois alvos.
pub fn targets(sketch: &Sketch, id: u64) -> Option<[Target; 2]> {
    if let Some(pair) = sketch.dimension_targets.get(&id) {
        return Some(*pair);
    }
    let kind = &sketch.constraints.iter().find(|c| c.id == id)?.kind;
    use Target::*;
    Some(match *kind {
        ConstraintKind::Distance { a, b, .. }
        | ConstraintKind::DeltaX { a, b, .. }
        | ConstraintKind::DeltaY { a, b, .. } => {
            if sketch.entities.iter().any(|e| {
                let p = e.point_ids();
                p.contains(&a) && p.contains(&b)
            }) {
                return None;
            }
            [Point(a), Point(b)]
        }
        ConstraintKind::PointLineDistance { point, line, .. } => [Point(point), Entity(line)],
        ConstraintKind::LineCircleDistance { line, circle, .. } => [Entity(line), Entity(circle)],
        ConstraintKind::LineAngle {
            line, reference, ..
        } => [Entity(reference), Entity(line)],
        _ => return None,
    })
}

pub fn solve(sketch: &Sketch, id: u64) -> Result<solver::Report> {
    if let Some(pair) = targets(sketch, id) {
        let target = preferred(sketch, pair, id)?;
        let circles = match target {
            Target::Entity(id) => vec![id],
            _ => vec![],
        };
        solver::solve_preserving(sketch, &target.points(sketch)?, &circles)
    } else {
        solver::solve(sketch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ties_move_the_first_and_strong_connections_change_the_preference() -> Result<()> {
        for weak in [false, true] {
            let mut sketch = Sketch::new("Conexões", Plane::xy());
            let first = sketch.add_circle([0., 0.], 2., false)?;
            let second = sketch.add_circle([10., 0.], 2., false)?;
            let line = sketch.add_line([0., 0.], [-5., 3.], false)?;
            let a = sketch.entity(first)?.point_ids()[0];
            let b = sketch.entity(second)?.point_ids()[0];
            let start = sketch.entity(line)?.point_ids()[0];
            sketch.add_constraint(ConstraintKind::Coincident { a, b: start }, weak)?;
            let id = sketch.add_constraint(ConstraintKind::Distance { a, b, value: 20. }, false)?;
            let pair = [Target::Entity(first), Target::Entity(second)];
            sketch.dimension_targets.insert(id, pair);
            assert_eq!(
                preferred(&sketch, pair, id)?,
                if weak { pair[1] } else { pair[0] }
            );
            let report = solve(&sketch, id)?;
            assert_ne!(report.status, solver::Status::Conflict);
            let preserved = if weak { b } else { a };
            assert_eq!(report.sketch.point(preserved)?, sketch.point(preserved)?);
            assert!(
                (distance(report.sketch.point(a)?, report.sketch.point(b)?) - 20.).abs() < 1e-6
            );
        }
        Ok(())
    }

    #[test]
    fn shared_vertices_count_and_invalid_saved_targets_are_rejected() -> Result<()> {
        let mut sketch = Sketch::new("Compartilhados", Plane::xy());
        let lines = sketch.add_rectangle([0., 0.], [10., 10.], false)?;
        let point = sketch.entity(lines[0])?.point_ids()[0];
        assert!(connections(&sketch, Target::Point(point), 0)? > 0);
        let other = sketch.add_point([30., 30.])?;
        let id = sketch.add_constraint(
            ConstraintKind::Distance {
                a: point,
                b: other,
                value: distance(sketch.point(point)?, sketch.point(other)?),
            },
            false,
        )?;
        sketch
            .dimension_targets
            .insert(id, [Target::Point(point), Target::Entity(99999)]);
        assert!(sketch.validate().is_err());
        Ok(())
    }
}
