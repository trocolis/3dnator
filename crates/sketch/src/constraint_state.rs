//! Estado de restrição de cada elemento, para a interface marcar vértices e arestas. Combina
//! o que o solver determina (o elemento não pode se mover) com as restrições aplicadas (ainda
//! se move, mas já tem alguma regra). Nada aqui altera o rascunho.
use crate::{dimensions, solver, *};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Nenhuma restrição age sobre o apoio.
    Free,
    /// Ainda se move, mas o apoio ou sua curva têm alguma restrição.
    Partial,
    /// O solver confirma que o apoio não pode se mover.
    Restricted,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    levels: HashMap<u64, Level>,
    restricted_entities: HashSet<u64>,
    fixed: HashSet<u64>,
    conflict_points: HashSet<u64>,
    conflict_entities: HashSet<u64>,
}
impl State {
    pub fn level(&self, point: u64) -> Level {
        self.levels.get(&point).copied().unwrap_or(Level::Free)
    }
    pub fn entity_restricted(&self, entity: u64) -> bool {
        self.restricted_entities.contains(&entity)
    }
    /// Fixado pelo usuário no espaço; apoios projetados são constantes, não fixações.
    pub fn is_fixed(&self, point: u64) -> bool {
        self.fixed.contains(&point)
    }
    pub fn point_in_conflict(&self, point: u64) -> bool {
        self.conflict_points.contains(&point)
    }
    pub fn entity_in_conflict(&self, entity: u64) -> bool {
        self.conflict_entities.contains(&entity)
    }
}

/// Elementos sobre os quais a restrição age. Ligar pontas não conta: duas linhas encadeadas
/// ficam livres com apoio compartilhado ou com coincidência. Num apoio preso a uma curva, é
/// o apoio que recebe a regra, não a curva que o hospeda.
fn acting(kind: &ConstraintKind) -> (Vec<u64>, Vec<u64>) {
    use ConstraintKind::*;
    match *kind {
        Coincident { .. } => (vec![], vec![]),
        PointOnLine { point, .. } | PointOnCurve { point, .. } | Midpoint { point, .. } => {
            (vec![point], vec![])
        }
        // O eixo só serve de referência: quem recebe a regra são os apoios espelhados.
        Symmetric { a, b, .. } => (vec![a, b], vec![]),
        _ => dimensions::participants(kind),
    }
}

/// Estende o conjunto pelos apoios ligados: cada lado de uma coincidência é o mesmo vértice.
fn spread(points: &mut HashSet<u64>, links: &[(u64, u64)]) {
    loop {
        let before = points.len();
        for (a, b) in links {
            if points.contains(a) || points.contains(b) {
                points.insert(*a);
                points.insert(*b);
            }
        }
        if points.len() == before {
            return;
        }
    }
}

/// Avalia a geometria recebida; roda no trabalhador, porque deriva o jacobiano do rascunho.
pub fn analyze(sketch: &Sketch) -> Result<State> {
    let determined = solver::determined(sketch)?;
    let driving = || sketch.constraints.iter().filter(|c| !c.reference);
    let links = driving()
        .filter_map(|c| match c.kind {
            ConstraintKind::Coincident { a, b } => Some((a, b)),
            _ => None,
        })
        .collect::<Vec<_>>();

    let mut constrained = HashSet::new();
    let mut fixed = HashSet::new();
    let mut conflict_points = HashSet::new();
    let mut conflict_entities = HashSet::new();
    let owned = |id: u64| sketch.entity(id).map(Entity::point_ids).unwrap_or_default();
    for constraint in driving() {
        let (points, entities) = acting(&constraint.kind);
        constrained.extend(points);
        constrained.extend(entities.into_iter().flat_map(owned));
        if let ConstraintKind::Fixed { point, .. } = constraint.kind {
            fixed.insert(point);
        }
        let violated = solver::constraint_residuals(sketch, &constraint.kind)
            .is_ok_and(|r| r.iter().any(|value| value.abs() > 1e-6));
        if violated {
            let (points, entities) = dimensions::participants(&constraint.kind);
            conflict_points.extend(points);
            conflict_points.extend(entities.iter().copied().flat_map(owned));
            conflict_entities.extend(entities);
        }
    }
    spread(&mut constrained, &links);
    spread(&mut fixed, &links);
    for entity in &sketch.entities {
        if entity
            .point_ids()
            .iter()
            .any(|p| conflict_points.contains(p))
        {
            conflict_entities.insert(entity.id);
        }
    }
    let projected = |id: u64| sketch.points.iter().any(|p| p.id == id && p.projected);
    fixed.retain(|id| !projected(*id));

    let levels = sketch
        .points
        .iter()
        .map(|point| {
            let level = if point.projected || determined.points.contains(&point.id) {
                Level::Restricted
            } else if constrained.contains(&point.id) {
                Level::Partial
            } else {
                Level::Free
            };
            (point.id, level)
        })
        .collect::<HashMap<_, _>>();
    let restricted_entities = sketch
        .entities
        .iter()
        .filter(|entity| {
            let radius = !matches!(entity.curve, Curve::Circle { .. })
                || entity.projected
                || determined.radii.contains(&entity.id);
            radius
                && entity
                    .point_ids()
                    .iter()
                    .all(|p| levels.get(p) == Some(&Level::Restricted))
        })
        .map(|entity| entity.id)
        .collect();
    Ok(State {
        levels,
        restricted_entities,
        fixed,
        conflict_points,
        conflict_entities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ends(sketch: &Sketch, line: u64) -> Result<(u64, u64)> {
        match sketch.entity(line)?.curve {
            Curve::Line { start, end } => Ok((start, end)),
            _ => Err(SketchError("Linha ausente".into())),
        }
    }

    #[test]
    fn loose_line_is_free_and_linked_ends_do_not_count_as_restriction() -> Result<()> {
        let mut sketch = Sketch::new("Soltas", Plane::xy());
        let first = sketch.add_line([0., 0.], [10., 0.], false)?;
        let second = sketch.add_line([10., 0.], [10., 10.], false)?;
        let (_, a) = ends(&sketch, first)?;
        let (b, _) = ends(&sketch, second)?;
        sketch.add_constraint(ConstraintKind::Coincident { a, b }, false)?;
        let state = analyze(&sketch)?;
        for point in &sketch.points {
            assert_eq!(state.level(point.id), Level::Free);
        }
        assert!(!state.entity_restricted(first));
        Ok(())
    }

    #[test]
    fn loose_rectangle_is_partial_until_it_is_tied_to_something_fixed() -> Result<()> {
        let mut sketch = Sketch::new("Retângulo", Plane::xy());
        let lines = sketch.add_rectangle([0., 0.], [20., 10.], false)?;
        let (corner, opposite) = ends(&sketch, lines[0])?;
        let (_, far) = ends(&sketch, lines[1])?;
        let state = analyze(&sketch)?;
        assert!(
            sketch
                .points
                .iter()
                .all(|p| state.level(p.id) == Level::Partial)
        );
        // Totalmente cotado, mas solto no plano: ainda pode se deslocar.
        sketch.add_constraint(
            ConstraintKind::Distance {
                a: corner,
                b: opposite,
                value: 20.,
            },
            false,
        )?;
        sketch.add_constraint(
            ConstraintKind::Distance {
                a: opposite,
                b: far,
                value: 10.,
            },
            false,
        )?;
        let state = analyze(&sketch)?;
        assert!(
            sketch
                .points
                .iter()
                .all(|p| state.level(p.id) == Level::Partial)
        );
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: corner,
                position: [0., 0.],
            },
            false,
        )?;
        let state = analyze(&sketch)?;
        assert!(
            sketch
                .points
                .iter()
                .all(|p| state.level(p.id) == Level::Restricted)
        );
        assert!(lines.iter().all(|line| state.entity_restricted(*line)));
        assert!(state.is_fixed(corner));
        assert!(!state.is_fixed(far));
        Ok(())
    }

    #[test]
    fn point_held_on_a_fixed_line_is_partial_and_its_free_end_stays_free() -> Result<()> {
        let mut sketch = Sketch::new("Apoio", Plane::xy());
        let base = sketch.add_line([0., 0.], [10., 0.], false)?;
        let (a, b) = ends(&sketch, base)?;
        for (point, position) in [(a, [0., 0.]), (b, [10., 0.])] {
            sketch.add_constraint(ConstraintKind::Fixed { point, position }, false)?;
        }
        let other = sketch.add_line([5., 0.], [5., 8.], false)?;
        let (held, loose) = ends(&sketch, other)?;
        sketch.add_constraint(
            ConstraintKind::PointOnLine {
                point: held,
                line: base,
            },
            false,
        )?;
        let state = analyze(&sketch)?;
        assert_eq!(state.level(held), Level::Partial);
        assert_eq!(state.level(loose), Level::Free);
        assert_eq!(state.level(a), Level::Restricted);
        assert!(state.entity_restricted(base));
        assert!(!state.entity_restricted(other));
        Ok(())
    }

    #[test]
    fn circle_needs_center_and_radius_to_be_restricted() -> Result<()> {
        let mut sketch = Sketch::new("Círculo", Plane::xy());
        let circle = sketch.add_circle([0., 0.], 5., false)?;
        let center = sketch.entity(circle)?.point_ids()[0];
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: center,
                position: [0., 0.],
            },
            false,
        )?;
        assert!(!analyze(&sketch)?.entity_restricted(circle));
        sketch.add_constraint(ConstraintKind::Radius { circle, value: 5. }, false)?;
        assert!(analyze(&sketch)?.entity_restricted(circle));
        Ok(())
    }

    #[test]
    fn only_the_elements_of_a_violated_constraint_are_in_conflict() -> Result<()> {
        let mut sketch = Sketch::new("Conflito", Plane::xy());
        let wrong = sketch.add_line([0., 0.], [10., 3.], false)?;
        let right = sketch.add_line([20., 0.], [30., 0.], false)?;
        let (point, _) = ends(&sketch, wrong)?;
        sketch.add_constraint(ConstraintKind::Horizontal { line: wrong }, false)?;
        let state = analyze(&sketch)?;
        assert!(state.entity_in_conflict(wrong));
        assert!(state.point_in_conflict(point));
        assert!(!state.entity_in_conflict(right));
        Ok(())
    }

    #[test]
    fn old_weak_links_and_reference_dimensions_do_not_restrict() -> Result<()> {
        let mut sketch = Sketch::new("Antigo", Plane::xy());
        let line = sketch.add_line([0., 0.], [10., 0.], false)?;
        let (a, b) = ends(&sketch, line)?;
        sketch.add_constraint(ConstraintKind::Distance { a, b, value: 10. }, true)?;
        sketch.add_constraint(ConstraintKind::Horizontal { line }, true)?;
        let state = analyze(&sketch)?;
        assert_eq!(state.level(a), Level::Free);
        assert_eq!(state.level(b), Level::Free);
        Ok(())
    }
}
