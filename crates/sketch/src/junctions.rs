//! Ligações geométricas fortes e fracas. Uma ligação forte é uma restrição
//! dirigente; a fraca conserva a intenção como restrição de referência, sem
//! influenciar o solver. Nenhuma das duas equivale a fixar a posição no espaço.
use crate::{snapping::Kind, *};
const EPS: f64 = 1e-6;

/// Vértices são ligações explícitas por padrão; cruzamentos e capturas no
/// interior das curvas somente posicionam. A interface pode inverter com F.
pub fn default_strong(kind: Option<Kind>) -> bool {
    matches!(
        kind,
        Some(
            Kind::Endpoint(_)
                | Kind::Center(_)
                | Kind::Midpoint(_)
                | Kind::CurveMidpoint(_)
                | Kind::Projected
        )
    )
}

fn add_link(sketch: &mut Sketch, kind: ConstraintKind, strong: bool) -> Result<()> {
    let existing = sketch.constraints.iter_mut().find(|constraint| {
        constraint.kind == kind || matches!((&constraint.kind, &kind),
            (ConstraintKind::Coincident { a,b }, ConstraintKind::Coincident { a: c,b: d }) if a == d && b == c)
    });
    if let Some(existing) = existing {
        existing.reference = !strong;
    } else {
        sketch.add_constraint(kind, !strong)?;
    }
    Ok(())
}

fn on_curve(sketch: &Sketch, entity: &Entity, position: Point2) -> Result<bool> {
    let segment = profiles::segment(sketch, entity)?;
    Ok(if entity.infinite {
        distance(lines::closest(sketch, entity, position)?, position) < EPS
    } else {
        projection::distance_to(&segment, position) < EPS
    })
}

fn link_curve(sketch: &mut Sketch, point: u64, curve: u64, strong: bool) -> Result<()> {
    if sketch.entity(curve)?.point_ids().contains(&point) {
        return Ok(());
    }
    let kind = if sketch.line(curve).is_ok() {
        ConstraintKind::PointOnLine { point, line: curve }
    } else {
        ConstraintKind::PointOnCurve { point, curve }
    };
    add_link(sketch, kind, strong)
}

/// Liga o apoio recém-criado ao alvo capturado. Mesmo na ligação fraca, a posição
/// do clique já está capturada; somente a obrigação de mantê-la é desativada.
pub fn attach(
    sketch: &mut Sketch,
    point: u64,
    kind: Option<Kind>,
    position: Point2,
    strong: bool,
) -> Result<()> {
    match kind {
        Some(Kind::Endpoint(target) | Kind::Center(target)) if target != point => add_link(
            sketch,
            ConstraintKind::Coincident {
                a: point,
                b: target,
            },
            strong,
        )?,
        Some(Kind::Midpoint(line)) => {
            add_link(sketch, ConstraintKind::Midpoint { point, line }, strong)?
        }
        Some(Kind::OnCurve(curve) | Kind::CurveMidpoint(curve)) => {
            link_curve(sketch, point, curve, strong)?
        }
        Some(Kind::Intersection) => {
            // Não depende de IDs de amostras da tela: encontra as curvas exatas
            // no ponto escolhido, excluindo a própria entidade em construção.
            let targets = sketch
                .entities
                .iter()
                .filter(|e| !e.point_ids().contains(&point))
                .map(|e| Ok((e.id, on_curve(sketch, e, position)?)))
                .collect::<Result<Vec<_>>>()?;
            for (curve, touches) in targets {
                if touches {
                    link_curve(sketch, point, curve, strong)?;
                }
            }
        }
        Some(Kind::Projected) if strong && distance(position, [0., 0.]) < EPS => {
            // Origem continua sendo um apoio absoluto, distinto de uma junção.
            sketch.add_constraint(
                ConstraintKind::Fixed {
                    point,
                    position: [0., 0.],
                },
                false,
            )?;
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Junction {
    pub position: Point2,
    pub strong: bool,
    points: Vec<u64>,
    curves: Vec<u64>,
}

fn local(kind: &ConstraintKind, points: &[u64]) -> bool {
    match *kind {
        ConstraintKind::Coincident { a, b } => points.contains(&a) && points.contains(&b),
        ConstraintKind::PointOnLine { point, .. }
        | ConstraintKind::PointOnCurve { point, .. }
        | ConstraintKind::Midpoint { point, .. } => points.contains(&point),
        _ => false,
    }
}

fn uses_point(kind: &ConstraintKind, target: u64) -> bool {
    match *kind {
        ConstraintKind::Coincident { a, b }
        | ConstraintKind::Distance { a, b, .. }
        | ConstraintKind::DeltaX { a, b, .. }
        | ConstraintKind::DeltaY { a, b, .. }
        | ConstraintKind::Angle { a, b, .. } => a == target || b == target,
        ConstraintKind::PointOnLine { point, .. }
        | ConstraintKind::PointOnCurve { point, .. }
        | ConstraintKind::Midpoint { point, .. }
        | ConstraintKind::Fixed { point, .. }
        | ConstraintKind::Tangent { point, .. }
        | ConstraintKind::PointLineDistance { point, .. } => point == target,
        _ => false,
    }
}

/// Consulta um vértice geométrico exato, já escolhido na apresentação. Pontos
/// isolados não têm junção para abrir/fechar; são necessários dois participantes.
pub fn at(sketch: &Sketch, position: Point2) -> Result<Option<Junction>> {
    let points = sketch
        .points
        .iter()
        .filter(|p| distance(p.position, position) < EPS)
        .map(|p| p.id)
        .collect::<Vec<_>>();
    let mut participants = HashSet::new();
    let mut curves = Vec::new();
    let mut shared = false;
    for point in &points {
        let owners = sketch
            .entities
            .iter()
            .filter(|e| e.point_ids().contains(point))
            .collect::<Vec<_>>();
        shared |= owners.len() > 1;
        participants.extend(owners.iter().map(|e| e.id));
    }
    for entity in &sketch.entities {
        if on_curve(sketch, entity, position)? {
            curves.push(entity.id);
            participants.insert(entity.id);
        }
    }
    if participants.len() < 2 {
        return Ok(None);
    }
    let strong = shared
        || sketch
            .constraints
            .iter()
            .any(|c| !c.reference && local(&c.kind, &points));
    Ok(Some(Junction {
        position,
        strong,
        points,
        curves,
    }))
}

/// Indicadores calculados junto ao arranjo de curvas no trabalhador, nunca por
/// varredura quadrática a cada quadro. Coincidências no mesmo lugar são únicas.
pub fn all(sketch: &Sketch, virtual_vertices: &[Point2]) -> Result<Vec<Junction>> {
    let mut positions = Vec::new();
    for position in sketch
        .visible_points()
        .map(|p| p.position)
        .chain(virtual_vertices.iter().copied())
    {
        if !positions.iter().any(|p| distance(*p, position) < EPS) {
            positions.push(position);
        }
    }
    let mut result = Vec::new();
    for position in positions {
        if let Some(junction) = at(sketch, position)? {
            result.push(junction);
        }
    }
    Ok(result)
}

/// Altera a junção inteira de forma atômica. Abrir um vértice que compartilha ID
/// cria um apoio independente para cada entidade, sem deslocar a geometria.
/// Cotas, horizontalidade e demais regras das entidades continuam no documento.
pub fn set_strong(sketch: &mut Sketch, position: Point2, strong: bool) -> Result<bool> {
    let mut next = sketch.clone();
    let Some(junction) = at(&next, position)? else {
        return Ok(false);
    };
    if junction.strong == strong {
        return Ok(false);
    }
    if strong {
        let point = match junction.points.first() {
            Some(point) => *point,
            None => next.add_point(position)?,
        };
        for other in &junction.points {
            if *other != point {
                add_link(
                    &mut next,
                    ConstraintKind::Coincident {
                        a: point,
                        b: *other,
                    },
                    true,
                )?;
            }
        }
        for constraint in &mut next.constraints {
            if local(&constraint.kind, &junction.points) {
                constraint.reference = false;
            }
        }
        for curve in junction.curves {
            // Extremos já ligados por coincidência não precisam de uma segunda
            // equação de incidência sobre a mesma curva.
            if !next
                .entity(curve)?
                .point_ids()
                .iter()
                .any(|p| junction.points.contains(p))
            {
                link_curve(&mut next, point, curve, true)?;
            }
        }
    } else {
        for constraint in &mut next.constraints {
            if local(&constraint.kind, &junction.points) {
                constraint.reference = true;
            }
        }
        for point in &junction.points {
            let point = *point;
            let owners = next
                .entities
                .iter()
                .filter(|e| e.point_ids().contains(&point))
                .map(|e| e.id)
                .collect::<Vec<_>>();
            if owners.len() < 2 {
                continue;
            }
            // Uma tangência em extremo compartilhado também é liberada ao
            // separar esse extremo: não pode continuar exigindo um ID comum.
            next.constraints.retain(
                |c| !matches!(c.kind, ConstraintKind::Tangent { point: p, .. } if p == point),
            );
            for owner in owners.into_iter().skip(1) {
                let copy = next.add_point(next.point(point)?)?;
                let entity = next
                    .entities
                    .iter_mut()
                    .find(|e| e.id == owner)
                    .ok_or_else(|| SketchError("Entidade da junção inexistente.".into()))?;
                match &mut entity.curve {
                    Curve::Line { start, end } => {
                        if *start == point {
                            *start = copy;
                        }
                        if *end == point {
                            *end = copy;
                        }
                    }
                    Curve::Circle { center, .. } => {
                        if *center == point {
                            *center = copy;
                        }
                    }
                    Curve::Arc {
                        center, start, end, ..
                    } => {
                        if *center == point {
                            *center = copy;
                        }
                        if *start == point {
                            *start = copy;
                        }
                        if *end == point {
                            *end = copy;
                        }
                    }
                }
                add_link(
                    &mut next,
                    ConstraintKind::Coincident { a: point, b: copy },
                    false,
                )?;
            }
        }
        // Um cruzamento puro volta a ser virtual quando aberto. Manter um apoio
        // solto acrescentaria dois graus de liberdade artificiais. Se o usuário
        // cotou esse apoio, ele é preservado como ponto real do rascunho.
        for point in junction.points {
            if !next.entities.iter().any(|e| e.point_ids().contains(&point))
                && next
                    .constraints
                    .iter()
                    .all(|c| !uses_point(&c.kind, point) || local(&c.kind, &[point]))
            {
                next.constraints.retain(|c| !uses_point(&c.kind, point));
                next.points.retain(|p| p.id != point);
            }
        }
    }
    next.validate()?;
    *sketch = next;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{self, Status};

    fn endpoints(sketch: &Sketch, id: u64) -> Result<(u64, u64)> {
        match sketch.entity(id)?.curve {
            Curve::Line { start, end } => Ok((start, end)),
            _ => Err(SketchError("Linha ausente".into())),
        }
    }

    #[test]
    fn endpoint_defaults_strong_and_override_controls_actual_solver_motion() -> Result<()> {
        for strong in [true, false] {
            let mut sketch = Sketch::new("Ligação", Plane::xy());
            let first = sketch.add_line([-10., 0.], [0., 0.], false)?;
            let second = sketch.add_line([0., 0.], [10., 0.], false)?;
            let (_, target) = endpoints(&sketch, first)?;
            let (point, _) = endpoints(&sketch, second)?;
            assert!(default_strong(Some(Kind::Endpoint(target))));
            attach(
                &mut sketch,
                point,
                Some(Kind::Endpoint(target)),
                [0., 0.],
                strong,
            )?;
            assert_eq!(
                at(&sketch, [0., 0.])?
                    .ok_or_else(|| SketchError("Junção ausente".into()))?
                    .strong,
                strong
            );
            // O deslocamento de um apoio representa uma edição já solicitada;
            // só o apoio da segunda linha precisa acompanhar quando a ligação dirige.
            sketch
                .points
                .iter_mut()
                .find(|p| p.id == target)
                .ok_or_else(|| SketchError("Ponto ausente".into()))?
                .position = [0., 5.];
            sketch.add_constraint(
                ConstraintKind::Fixed {
                    point: target,
                    position: [0., 5.],
                },
                false,
            )?;
            let report = solver::solve(&sketch)?;
            assert_ne!(report.status, Status::Conflict);
            assert!(
                distance(
                    report.sketch.point(point)?,
                    if strong { [0., 5.] } else { [0., 0.] }
                ) < 1e-6
            );
        }
        Ok(())
    }

    #[test]
    fn shared_rectangle_corner_opens_without_motion_and_closes_again() -> Result<()> {
        let mut sketch = Sketch::new("Canto compartilhado", Plane::xy());
        let lines = sketch.add_rectangle([0., 0.], [20., 10.], false)?;
        let before = sketch.points.iter().map(|p| p.position).collect::<Vec<_>>();
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| j.strong));
        assert!(set_strong(&mut sketch, [0., 0.], false)?);
        assert_eq!(sketch.points.len(), 5);
        assert!(sketch.points.iter().all(|p| before.contains(&p.position)));
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| !j.strong));
        let (first, _) = endpoints(&sketch, lines[0])?;
        let (_, last) = endpoints(&sketch, lines[3])?;
        assert_ne!(first, last);
        let free = solver::solve(&sketch)?;
        assert_ne!(free.status, Status::Conflict);
        assert!(set_strong(&mut sketch, [0., 0.], true)?);
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| j.strong));
        let linked = solver::solve(&sketch)?;
        assert_ne!(linked.status, Status::Conflict);
        assert_ne!(free.status, linked.status);
        assert!(!set_strong(&mut sketch, [0., 0.], true)?);
        Ok(())
    }

    #[test]
    fn crossing_defaults_weak_and_explicit_lock_survives_pruning() -> Result<()> {
        let mut sketch = Sketch::new("Cruzamento", Plane::xy());
        sketch.add_line([-10., 0.], [10., 0.], false)?;
        sketch.add_line([0., -10.], [0., 10.], false)?;
        assert!(!default_strong(Some(Kind::Intersection)));
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| !j.strong));
        assert!(set_strong(&mut sketch, [0., 0.], true)?);
        assert_eq!(sketch.points.len(), 5);
        sketch.prune_unused_points();
        assert_eq!(sketch.points.len(), 5);
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| j.strong));
        assert!(set_strong(&mut sketch, [0., 0.], false)?);
        assert_eq!(sketch.points.len(), 4);
        assert!(sketch.constraints.is_empty());
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| !j.strong));
        Ok(())
    }

    #[test]
    fn circular_incidence_follows_radius_instead_of_freezing_distance() -> Result<()> {
        let mut sketch = Sketch::new("Borda circular", Plane::xy());
        let circle = sketch.add_circle([0., 0.], 10., false)?;
        let line = sketch.add_line([10., 0.], [20., 0.], false)?;
        let (point, _) = endpoints(&sketch, line)?;
        attach(
            &mut sketch,
            point,
            Some(Kind::OnCurve(circle)),
            [10., 0.],
            true,
        )?;
        sketch.add_constraint(ConstraintKind::Radius { circle, value: 15. }, false)?;
        let Curve::Circle { center, .. } = sketch.entity(circle)?.curve else {
            return Err(SketchError("Círculo ausente".into()));
        };
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: center,
                position: [0., 0.],
            },
            false,
        )?;
        let report = solver::solve(&sketch)?;
        assert_ne!(report.status, Status::Conflict);
        assert!(
            (distance(report.sketch.point(point)?, report.sketch.point(center)?) - 15.).abs()
                < 1e-6
        );
        Ok(())
    }
}
