//! Ligações entre apoios. Uma ligação é uma restrição dirigente que mantém os elementos
//! juntos; capturar um cruzamento ou o meio de uma curva só posiciona, sem criar nada. Ligar
//! não equivale a fixar a posição no espaço: isso é `Fixed`, e o quanto cada elemento ainda
//! pode se mover é informado por `constraint_state`.
//!
//! Projetos antigos podem trazer ligações "fracas" (`reference` em restrições geométricas).
//! Elas não têm efeito no solver e, para a interface, equivalem a não haver ligação.
use crate::{snapping::Kind, *};
const EPS: f64 = 1e-6;

/// Vértices, centros e pontos médios ligam o apoio capturado; cruzamentos e capturas no
/// interior das curvas somente posicionam.
pub fn links(kind: Option<Kind>) -> bool {
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

/// Acrescenta a ligação, ou reativa a mesma ligação guardada como fraca num projeto antigo.
fn add_link(sketch: &mut Sketch, kind: ConstraintKind) -> Result<()> {
    let existing = sketch.constraints.iter_mut().find(|constraint| {
        constraint.kind == kind || matches!((&constraint.kind, &kind),
            (ConstraintKind::Coincident { a,b }, ConstraintKind::Coincident { a: c,b: d }) if a == d && b == c)
    });
    if let Some(existing) = existing {
        existing.reference = false;
    } else {
        sketch.add_constraint(kind, false)?;
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

fn link_curve(sketch: &mut Sketch, point: u64, curve: u64) -> Result<()> {
    if sketch.entity(curve)?.point_ids().contains(&point) {
        return Ok(());
    }
    let kind = if sketch.line(curve).is_ok() {
        ConstraintKind::PointOnLine { point, line: curve }
    } else {
        ConstraintKind::PointOnCurve { point, curve }
    };
    add_link(sketch, kind)
}

/// Liga o apoio recém-criado ao alvo capturado. Capturas que só posicionam não gravam nada:
/// a posição do clique já está no apoio.
pub fn attach(sketch: &mut Sketch, point: u64, kind: Option<Kind>, position: Point2) -> Result<()> {
    if !links(kind) {
        return Ok(());
    }
    match kind {
        Some(Kind::Endpoint(target) | Kind::Center(target)) if target != point => add_link(
            sketch,
            ConstraintKind::Coincident {
                a: point,
                b: target,
            },
        )?,
        Some(Kind::Midpoint(line)) => add_link(sketch, ConstraintKind::Midpoint { point, line })?,
        Some(Kind::CurveMidpoint(curve)) => link_curve(sketch, point, curve)?,
        Some(Kind::Projected) if distance(position, [0., 0.]) < EPS => {
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
    /// Verdadeiro quando há ligação de fato: apoio compartilhado ou restrição dirigente.
    pub linked: bool,
    points: Vec<u64>,
    curves: Vec<u64>,
}
impl Junction {
    /// Curvas que participam do vértice; usado por ferramentas de canto.
    pub fn curves(&self) -> &[u64] {
        &self.curves
    }
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
        | ConstraintKind::OriginDistance { point, .. }
        | ConstraintKind::PointLineDistance { point, .. } => point == target,
        _ => false,
    }
}

/// Consulta um vértice geométrico exato, já escolhido na apresentação. Pontos
/// isolados não têm junção para ligar ou soltar; são necessários dois participantes.
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
    let linked = shared
        || sketch
            .constraints
            .iter()
            .any(|c| !c.reference && local(&c.kind, &points));
    Ok(Some(Junction {
        position,
        linked,
        points,
        curves,
    }))
}

/// Um cruzamento só vira apoio entre curvas normais. Com uma linha de referência o vértice
/// é imaginário: serve de captura e não prende nem muda a geometria do corpo.
fn can_link(sketch: &Sketch, junction: &Junction) -> Result<bool> {
    if !junction.points.is_empty() {
        return Ok(true);
    }
    let mut normal = 0;
    for curve in &junction.curves {
        if !sketch.entity(*curve)?.construction {
            normal += 1;
        }
    }
    Ok(normal >= 2)
}
/// Verdadeiro quando `link` faria alguma coisa neste vértice.
pub fn linkable(sketch: &Sketch, position: Point2) -> Result<bool> {
    Ok(match at(sketch, position)? {
        Some(junction) => !junction.linked && can_link(sketch, &junction)?,
        None => false,
    })
}

/// Linhas que terminam no vértice, com o extremo oposto, quando ele une só duas retas.
fn dissolvable(sketch: &Sketch, position: Point2) -> Result<Option<[(u64, u64); 2]>> {
    let Some(junction) = at(sketch, position)? else {
        return Ok(None);
    };
    let mut ends = Vec::new();
    for curve in &junction.curves {
        let entity = sketch.entity(*curve)?;
        let Curve::Line { start, end } = entity.curve else {
            return Ok(None);
        };
        let far = if distance(sketch.point(start)?, position) < EPS {
            end
        } else if distance(sketch.point(end)?, position) < EPS {
            start
        } else {
            // Uma curva apenas passa aqui: o vértice também é apoio dela.
            return Ok(None);
        };
        if entity.infinite {
            return Ok(None);
        }
        ends.push((*curve, far));
    }
    let [first, second] = ends[..] else {
        return Ok(None);
    };
    if sketch.locked(first.0) || sketch.locked(second.0) {
        return Ok(None);
    }
    if sketch.entity(first.0)?.construction != sketch.entity(second.0)?.construction
        || first.1 == second.1
    {
        return Ok(None);
    }
    Ok(Some([first, second]))
}
pub fn can_dissolve(sketch: &Sketch, position: Point2) -> Result<bool> {
    Ok(dissolvable(sketch, position)?.is_some())
}
/// Remove o vértice entre duas retas: resta uma só aresta entre os extremos opostos.
/// Regras do vértice e da reta retirada somem com ele; as da reta mantida só continuam
/// se as duas eram colineares, porque do contrário a nova aresta tem outra direção.
pub fn dissolve(sketch: &mut Sketch, position: Point2) -> Result<bool> {
    let Some([keep, drop]) = dissolvable(sketch, position)? else {
        return Ok(false);
    };
    let mut next = sketch.clone();
    let vertices = next
        .points
        .iter()
        .filter(|p| distance(p.position, position) < EPS)
        .map(|p| p.id)
        .collect::<Vec<_>>();
    let (far_keep, far_drop) = (next.point(keep.1)?, next.point(drop.1)?);
    let u = sub(far_keep, position);
    let v = sub(far_drop, position);
    let collinear = (u[0] * v[1] - u[1] * v[0]).abs()
        <= 1e-7 * distance(u, [0., 0.]) * distance(v, [0., 0.])
        && dot(u, v) < 0.;
    next.constraints.retain(|c| {
        !vertices.iter().any(|p| uses_point(&c.kind, *p))
            && !(!collinear && crate::dimensions::participants(&c.kind).1.contains(&keep.0))
    });
    next.entities.retain(|e| e.id != drop.0);
    let replaced = drop.1;
    if let Some(Curve::Line { start, end }) = next
        .entities
        .iter_mut()
        .find(|e| e.id == keep.0)
        .map(|e| &mut e.curve)
    {
        if *start == keep.1 {
            *end = replaced;
        } else {
            *start = replaced;
        }
    }
    next.prune_unused_points();
    next.validate()?;
    *sketch = next;
    Ok(true)
}

/// Liga tudo o que passa pelo vértice de forma atômica, inclusive um cruzamento, que vira
/// um apoio real. Não desloca a geometria.
pub fn link(sketch: &mut Sketch, position: Point2) -> Result<bool> {
    let mut next = sketch.clone();
    let Some(junction) = at(&next, position)? else {
        return Ok(false);
    };
    if junction.linked || !can_link(&next, &junction)? {
        return Ok(false);
    }
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
            )?;
        }
    }
    for constraint in &mut next.constraints {
        if local(&constraint.kind, &junction.points) {
            constraint.reference = false;
        }
    }
    for curve in junction.curves {
        // Uma linha de referência que só passa pelo vértice não participa da ligação.
        if next.entity(curve)?.construction {
            continue;
        }
        // Extremos já ligados por coincidência não precisam de uma segunda
        // equação de incidência sobre a mesma curva.
        if !next
            .entity(curve)?
            .point_ids()
            .iter()
            .any(|p| junction.points.contains(p))
        {
            link_curve(&mut next, point, curve)?;
        }
    }
    next.validate()?;
    *sketch = next;
    Ok(true)
}

/// Solta o vértice de forma atômica: cada entidade passa a ter seu próprio apoio, sem deslocar
/// a geometria. Cotas, horizontalidade e demais regras das entidades continuam no documento.
pub fn detach(sketch: &mut Sketch, position: Point2) -> Result<bool> {
    let mut next = sketch.clone();
    let Some(junction) = at(&next, position)? else {
        return Ok(false);
    };
    // Separar daria à referência fixa um apoio novo, livre: o vértice dela não se solta.
    if !junction.linked
        || junction
            .curves
            .iter()
            .chain(
                next.entities
                    .iter()
                    .filter(|e| e.point_ids().iter().any(|p| junction.points.contains(p)))
                    .map(|e| &e.id),
            )
            .any(|id| next.locked(*id))
    {
        return Ok(false);
    }
    next.constraints
        .retain(|constraint| !local(&constraint.kind, &junction.points));
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
        next.constraints
            .retain(|c| !matches!(c.kind, ConstraintKind::Tangent { point: p, .. } if p == point));
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
        }
    }
    // Um cruzamento puro volta a ser virtual quando solto. Manter um apoio sem
    // curva acrescentaria dois graus de liberdade artificiais. Se o usuário
    // cotou esse apoio, ele é preservado como ponto real do rascunho.
    for point in junction.points {
        if !next.entities.iter().any(|e| e.point_ids().contains(&point))
            && next.constraints.iter().all(|c| !uses_point(&c.kind, point))
        {
            next.points.retain(|p| p.id != point);
        }
    }
    next.validate()?;
    *sketch = next;
    Ok(true)
}

/// Pontos geométricos exatamente na posição informada.
fn points_at(sketch: &Sketch, position: Point2) -> Vec<u64> {
    sketch
        .points
        .iter()
        .filter(|p| distance(p.position, position) < EPS)
        .map(|p| p.id)
        .collect()
}

/// Um elemento fixado mantém a posição durante o dimensionamento. A consulta usa as
/// restrições `Fixed` dos pontos que formam o vértice.
pub fn fixed(sketch: &Sketch, position: Point2) -> Result<bool> {
    let points = points_at(sketch, position);
    Ok(sketch.constraints.iter().any(|constraint| {
        matches!(
            constraint.kind,
            ConstraintKind::Fixed { point, .. } if points.contains(&point)
        )
    }))
}

/// Fixa ou libera o vértice de forma atômica. Fixar cria uma restrição `Fixed`
/// no apoio compartilhado; liberar remove as restrições dos pontos do vértice.
pub fn set_fixed(sketch: &mut Sketch, position: Point2, fixed: bool) -> Result<bool> {
    let points = points_at(sketch, position);
    if points.is_empty() {
        return Ok(false);
    }
    let already = sketch.constraints.iter().any(|constraint| {
        matches!(
            constraint.kind,
            ConstraintKind::Fixed { point, .. } if points.contains(&point)
        )
    });
    if already == fixed {
        return Ok(false);
    }
    let mut next = sketch.clone();
    if fixed {
        let point = *points
            .first()
            .ok_or_else(|| SketchError("Vértice ausente".into()))?;
        next.add_constraint(
            ConstraintKind::Fixed {
                point,
                position: next.point(point)?,
            },
            false,
        )?;
    } else {
        next.constraints.retain(|constraint| {
            !matches!(
                constraint.kind,
                ConstraintKind::Fixed { point, .. } if points.contains(&point)
            )
        });
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
    fn endpoint_capture_links_and_the_link_follows_the_solver() -> Result<()> {
        let mut sketch = Sketch::new("Ligação", Plane::xy());
        let first = sketch.add_line([-10., 0.], [0., 0.], false)?;
        let second = sketch.add_line([0., 0.], [10., 0.], false)?;
        let (_, target) = endpoints(&sketch, first)?;
        let (point, _) = endpoints(&sketch, second)?;
        assert!(links(Some(Kind::Endpoint(target))));
        attach(&mut sketch, point, Some(Kind::Endpoint(target)), [0., 0.])?;
        assert!(
            at(&sketch, [0., 0.])?
                .ok_or_else(|| SketchError("Junção ausente".into()))?
                .linked
        );
        // O deslocamento de um apoio representa uma edição já solicitada;
        // o apoio da segunda linha acompanha porque a ligação dirige.
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
        assert!(distance(report.sketch.point(point)?, [0., 5.]) < 1e-6);
        Ok(())
    }

    #[test]
    fn captures_that_only_position_record_nothing() -> Result<()> {
        let mut sketch = Sketch::new("Cruzamento", Plane::xy());
        let line = sketch.add_line([-10., 0.], [10., 0.], false)?;
        let other = sketch.add_line([0., 0.], [0., 10.], false)?;
        let (point, _) = endpoints(&sketch, other)?;
        assert!(!links(Some(Kind::OnCurve(line))));
        assert!(!links(Some(Kind::Intersection)));
        attach(&mut sketch, point, Some(Kind::OnCurve(line)), [0., 0.])?;
        attach(&mut sketch, point, Some(Kind::Intersection), [0., 0.])?;
        assert!(sketch.constraints.is_empty());
        Ok(())
    }

    #[test]
    fn shared_rectangle_corner_detaches_without_motion_and_links_again() -> Result<()> {
        let mut sketch = Sketch::new("Canto compartilhado", Plane::xy());
        let lines = sketch.add_rectangle([0., 0.], [20., 10.], false)?;
        let before = sketch.points.iter().map(|p| p.position).collect::<Vec<_>>();
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| j.linked));
        assert!(detach(&mut sketch, [0., 0.])?);
        assert_eq!(sketch.points.len(), 5);
        assert!(sketch.points.iter().all(|p| before.contains(&p.position)));
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| !j.linked));
        assert!(!sketch.constraints.iter().any(|c| c.reference));
        let (first, _) = endpoints(&sketch, lines[0])?;
        let (_, last) = endpoints(&sketch, lines[3])?;
        assert_ne!(first, last);
        let free = solver::solve(&sketch)?;
        assert_ne!(free.status, Status::Conflict);
        assert!(link(&mut sketch, [0., 0.])?);
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| j.linked));
        let linked = solver::solve(&sketch)?;
        assert_ne!(linked.status, Status::Conflict);
        assert_ne!(free.status, linked.status);
        assert!(!link(&mut sketch, [0., 0.])?);
        Ok(())
    }

    #[test]
    fn crossing_links_on_request_and_returns_to_virtual_when_detached() -> Result<()> {
        let mut sketch = Sketch::new("Cruzamento", Plane::xy());
        sketch.add_line([-10., 0.], [10., 0.], false)?;
        sketch.add_line([0., -10.], [0., 10.], false)?;
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| !j.linked));
        assert!(link(&mut sketch, [0., 0.])?);
        assert_eq!(sketch.points.len(), 5);
        sketch.prune_unused_points();
        assert_eq!(sketch.points.len(), 5);
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| j.linked));
        assert!(detach(&mut sketch, [0., 0.])?);
        assert_eq!(sketch.points.len(), 4);
        assert!(sketch.constraints.is_empty());
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| !j.linked));
        Ok(())
    }

    #[test]
    fn old_weak_link_counts_as_unlinked_and_is_reactivated_on_request() -> Result<()> {
        let mut sketch = Sketch::new("Projeto antigo", Plane::xy());
        let first = sketch.add_line([-10., 0.], [0., 0.], false)?;
        let second = sketch.add_line([1., 0.], [10., 0.], false)?;
        let (_, a) = endpoints(&sketch, first)?;
        let (b, _) = endpoints(&sketch, second)?;
        sketch
            .points
            .iter_mut()
            .find(|p| p.id == b)
            .ok_or_else(|| SketchError("Ponto ausente".into()))?
            .position = [0., 0.];
        let id = sketch.add_constraint(ConstraintKind::Coincident { a, b }, true)?;
        assert!(at(&sketch, [0., 0.])?.is_some_and(|j| !j.linked));
        assert!(link(&mut sketch, [0., 0.])?);
        assert!(
            sketch
                .constraints
                .iter()
                .any(|c| c.id == id && !c.reference)
        );
        assert_eq!(sketch.constraints.len(), 1);
        Ok(())
    }

    #[test]
    fn circular_incidence_follows_radius_instead_of_freezing_distance() -> Result<()> {
        let mut sketch = Sketch::new("Borda circular", Plane::xy());
        let circle = sketch.add_circle([0., 0.], 10., false)?;
        let line = sketch.add_line([10., 0.], [20., 0.], false)?;
        let (point, _) = endpoints(&sketch, line)?;
        assert!(link(&mut sketch, [10., 0.])?);
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

#[cfg(test)]
mod vertex_rules {
    use super::*;

    #[test]
    fn a_reference_crossing_stays_imaginary_and_does_not_change_the_area() -> Result<()> {
        let mut sketch = Sketch::new("Referência", Plane::xy());
        sketch.add_rectangle([0., 0.], [20., 10.], false)?;
        sketch.add_line([-5., 5.], [25., 5.], true)?;
        let crossing = [0., 5.];
        assert!(at(&sketch, crossing)?.is_some());
        assert!(!linkable(&sketch, crossing)?);
        let before = (sketch.points.len(), sketch.constraints.len());
        assert!(!link(&mut sketch, crossing)?);
        assert_eq!((sketch.points.len(), sketch.constraints.len()), before);
        assert_eq!(areas::regions(&sketch)?.len(), 1);
        // A mesma linha como traço normal divide a área e permite ligar.
        let mut normal = Sketch::new("Normal", Plane::xy());
        normal.add_rectangle([0., 0.], [20., 10.], false)?;
        normal.add_line([-5., 5.], [25., 5.], false)?;
        assert!(linkable(&normal, crossing)?);
        assert_eq!(areas::regions(&normal)?.len(), 2);
        Ok(())
    }

    #[test]
    fn removing_a_vertex_between_collinear_lines_keeps_their_rules() -> Result<()> {
        let mut sketch = Sketch::new("Colineares", Plane::xy());
        let first = sketch.add_line([0., 0.], [10., 0.], false)?;
        let second = sketch.add_line([10., 0.], [30., 0.], false)?;
        let [_, joint] = sketch.entity(first)?.point_ids()[..] else {
            return Err(SketchError("Linha sem extremos.".into()));
        };
        let [start, _] = sketch.entity(second)?.point_ids()[..] else {
            return Err(SketchError("Linha sem extremos.".into()));
        };
        sketch.add_constraint(ConstraintKind::Coincident { a: joint, b: start }, false)?;
        sketch.add_constraint(ConstraintKind::Horizontal { line: first }, false)?;
        assert!(can_dissolve(&sketch, [10., 0.])?);
        assert!(dissolve(&mut sketch, [10., 0.])?);
        assert_eq!(sketch.entities.len(), 1);
        assert_eq!(sketch.line(first)?, ([0., 0.], [30., 0.]));
        assert!(
            sketch
                .constraints
                .iter()
                .any(|c| matches!(c.kind, ConstraintKind::Horizontal { line } if line == first))
        );
        Ok(())
    }

    #[test]
    fn a_corner_becomes_a_straight_edge_and_branches_or_arcs_are_refused() -> Result<()> {
        let mut sketch = Sketch::new("Canto", Plane::xy());
        let first = sketch.add_line([0., 0.], [10., 0.], false)?;
        let [_, joint] = sketch.entity(first)?.point_ids()[..] else {
            return Err(SketchError("Linha sem extremos.".into()));
        };
        let second = sketch.add_line([10., 0.], [10., 10.], false)?;
        let [start, _] = sketch.entity(second)?.point_ids()[..] else {
            return Err(SketchError("Linha sem extremos.".into()));
        };
        sketch.add_constraint(ConstraintKind::Coincident { a: joint, b: start }, false)?;
        sketch.add_constraint(ConstraintKind::Horizontal { line: first }, false)?;
        assert!(dissolve(&mut sketch, [10., 0.])?);
        assert_eq!(sketch.line(first)?, ([0., 0.], [10., 10.]));
        assert!(sketch.constraints.is_empty());

        let mut branch = Sketch::new("Ramo", Plane::xy());
        branch.add_line([0., 0.], [10., 0.], false)?;
        branch.add_line([10., 0.], [20., 0.], false)?;
        branch.add_line([10., 0.], [10., 10.], false)?;
        assert!(!can_dissolve(&branch, [10., 0.])?);
        assert!(!dissolve(&mut branch, [10., 0.])?);
        assert_eq!(branch.entities.len(), 3);
        Ok(())
    }
}
