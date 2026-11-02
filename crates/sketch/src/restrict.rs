//! Restrições aplicáveis à seleção do rascunho. A interface mostra só o que vale para os
//! vértices, arestas e origem escolhidos; as regras ficam aqui, sem depender de egui. Aplicar
//! devolve um rascunho novo, que o chamador resolve e publica como qualquer outra edição.
use crate::{dimensions, junctions, *};

const EPS: f64 = 1e-6;
pub const ORIGIN: Point2 = [0., 0.];

/// Vértices pela posição, porque cruzamentos ainda não têm apoio real; a posição `ORIGIN` sem
/// vértice nela é a própria origem do rascunho. A ordem das arestas é a da seleção: na simetria
/// entre linhas, a última é o eixo.
#[derive(Debug, Clone, Copy)]
pub struct Selection<'a> {
    pub vertices: &'a [Point2],
    pub entities: &'a [u64],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restriction {
    HorizontalVertical,
    Coincident,
    Tangent,
    Equal,
    Parallel,
    Perpendicular,
    Fix,
    Release,
    Midpoint,
    Concentric,
    Collinear,
    Symmetry,
    Polygon,
    Link,
    Detach,
    /// Tira o vértice entre duas retas, deixando uma só aresta.
    Dissolve,
}

const ALL: [Restriction; 16] = [
    Restriction::HorizontalVertical,
    Restriction::Coincident,
    Restriction::Tangent,
    Restriction::Equal,
    Restriction::Parallel,
    Restriction::Perpendicular,
    Restriction::Fix,
    Restriction::Release,
    Restriction::Midpoint,
    Restriction::Concentric,
    Restriction::Collinear,
    Restriction::Symmetry,
    Restriction::Polygon,
    Restriction::Link,
    Restriction::Detach,
    Restriction::Dissolve,
];

fn point_at(sketch: &Sketch, position: Point2) -> Option<u64> {
    sketch
        .points
        .iter()
        .find(|p| distance(p.position, position) < EPS)
        .map(|p| p.id)
}

/// O que uma posição selecionada representa para a coincidência.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Point(u64),
    Origin,
}

/// Um vértice real na posição tem prioridade: nela a origem coincide com ele, e é o vértice que
/// se quer mover. Cruzamentos sem apoio não têm o que ligar.
fn target(sketch: &Sketch, position: Point2) -> Option<Target> {
    point_at(sketch, position)
        .map(Target::Point)
        .or_else(|| (distance(position, ORIGIN) < EPS).then_some(Target::Origin))
}

/// Posições dos apoios da seleção. O centro de um círculo fixa a curva; o raio é uma cota.
fn positions(sketch: &Sketch, selection: Selection<'_>) -> Result<Vec<Point2>> {
    let mut result = selection.vertices.to_vec();
    for id in selection.entities {
        for point in sketch.entity(*id)?.point_ids() {
            let position = sketch.point(point)?;
            if !result.iter().any(|p| distance(*p, position) < EPS) {
                result.push(position);
            }
        }
    }
    Ok(result)
}

fn same(a: &ConstraintKind, b: &ConstraintKind) -> bool {
    use ConstraintKind::*;
    match (a, b) {
        (Coincident { a, b }, Coincident { a: c, b: d })
        | (Parallel { a, b }, Parallel { a: c, b: d })
        | (Perpendicular { a, b }, Perpendicular { a: c, b: d })
        | (EqualLength { a, b }, EqualLength { a: c, b: d })
        | (Concentric { a, b }, Concentric { a: c, b: d })
        | (PointsHorizontal { a, b }, PointsHorizontal { a: c, b: d })
        | (PointsVertical { a, b }, PointsVertical { a: c, b: d })
        | (Collinear { a, b }, Collinear { a: c, b: d })
        | (EqualRadius { a, b }, EqualRadius { a: c, b: d })
        | (CircleTangent { a, b, .. }, CircleTangent { a: c, b: d, .. }) => {
            (a == c && b == d) || (a == d && b == c)
        }
        (
            Symmetric { a, b, axis },
            Symmetric {
                a: c,
                b: d,
                axis: other,
            },
        ) => axis == other && ((a == c && b == d) || (a == d && b == c)),
        _ => a == b,
    }
}

fn exists(sketch: &Sketch, kind: &ConstraintKind) -> bool {
    sketch
        .constraints
        .iter()
        .any(|c| !c.reference && same(&c.kind, kind))
}

fn is_line(sketch: &Sketch, id: u64) -> bool {
    sketch.line(id).is_ok()
}

fn is_finite_line(sketch: &Sketch, id: u64) -> bool {
    is_line(sketch, id) && sketch.entity(id).is_ok_and(|e| !e.infinite)
}

fn is_radial(sketch: &Sketch, id: u64) -> bool {
    sketch.radial(id).is_ok()
}

/// A direção oposta, que `apply` retira: clicar de novo alterna entre horizontal e vertical.
fn opposite(kind: &ConstraintKind) -> Option<ConstraintKind> {
    use ConstraintKind::*;
    match *kind {
        Horizontal { line } => Some(Vertical { line }),
        Vertical { line } => Some(Horizontal { line }),
        PointsHorizontal { a, b } => Some(PointsVertical { a, b }),
        PointsVertical { a, b } => Some(PointsHorizontal { a, b }),
        _ => None,
    }
}

/// Mantém a direção já escolhida invertida; sem ela, vale a direção predominante.
fn directed(
    sketch: &Sketch,
    horizontal: ConstraintKind,
    vertical: ConstraintKind,
    delta: Point2,
) -> ConstraintKind {
    if exists(sketch, &horizontal) {
        vertical
    } else if exists(sketch, &vertical) || delta[0].abs() >= delta[1].abs() {
        horizontal
    } else {
        vertical
    }
}

fn horizontal_vertical(sketch: &Sketch, selection: Selection<'_>) -> Result<Vec<ConstraintKind>> {
    match (selection.vertices, selection.entities) {
        ([first, second], []) => Ok(
            match (point_at(sketch, *first), point_at(sketch, *second)) {
                (Some(a), Some(b)) if a != b => vec![directed(
                    sketch,
                    ConstraintKind::PointsHorizontal { a, b },
                    ConstraintKind::PointsVertical { a, b },
                    sub(*second, *first),
                )],
                _ => vec![],
            },
        ),
        ([], lines)
            if !lines.is_empty()
                && lines.iter().all(|id| {
                    is_line(sketch, *id) && sketch.entity(*id).is_ok_and(|e| !e.projected)
                }) =>
        {
            lines
                .iter()
                .map(|line| {
                    let (a, b) = sketch.line(*line)?;
                    Ok(directed(
                        sketch,
                        ConstraintKind::Horizontal { line: *line },
                        ConstraintKind::Vertical { line: *line },
                        sub(b, a),
                    ))
                })
                .collect()
        }
        _ => Ok(vec![]),
    }
}

/// Apoios de cada entidade para coincidir: extremos, ou o centro de um círculo.
fn anchors(entity: &Entity) -> Vec<u64> {
    match entity.curve {
        Curve::Line { start, end } | Curve::Arc { start, end, .. } => vec![start, end],
        Curve::Circle { center, .. } => vec![center],
    }
}

/// Dois vértices, vértice e aresta, duas arestas ou qualquer um deles com a origem.
fn coincident(sketch: &Sketch, selection: Selection<'_>) -> Result<Vec<ConstraintKind>> {
    let held_at_origin = |point| ConstraintKind::Fixed {
        point,
        position: ORIGIN,
    };
    Ok(match (selection.vertices, selection.entities) {
        ([first, second], []) => match (target(sketch, *first), target(sketch, *second)) {
            (Some(Target::Point(a)), Some(Target::Point(b))) if a != b => {
                vec![ConstraintKind::Coincident { a, b }]
            }
            (Some(Target::Point(point)), Some(Target::Origin))
            | (Some(Target::Origin), Some(Target::Point(point))) => vec![held_at_origin(point)],
            _ => vec![],
        },
        ([position], [curve]) => match target(sketch, *position) {
            Some(Target::Origin) => vec![ConstraintKind::OriginOnCurve { curve: *curve }],
            Some(Target::Point(point)) if !sketch.entity(*curve)?.point_ids().contains(&point) => {
                vec![if is_line(sketch, *curve) {
                    ConstraintKind::PointOnLine {
                        point,
                        line: *curve,
                    }
                } else {
                    ConstraintKind::PointOnCurve {
                        point,
                        curve: *curve,
                    }
                }]
            }
            _ => vec![],
        },
        ([], [first, second]) => {
            let left = anchors(sketch.entity(*first)?);
            let right = anchors(sketch.entity(*second)?);
            if left.iter().any(|a| right.contains(a)) {
                return Ok(vec![]);
            }
            let mut best = None;
            for a in &left {
                for b in &right {
                    let gap = distance(sketch.point(*a)?, sketch.point(*b)?);
                    if best.is_none_or(|(d, _, _)| gap < d) {
                        best = Some((gap, *a, *b));
                    }
                }
            }
            best.map(|(_, a, b)| ConstraintKind::Coincident { a, b })
                .into_iter()
                .collect()
        }
        _ => vec![],
    })
}

/// Apoio da tangência entre reta e curva: o extremo comum com o arco ou, sem ele, o centro.
fn tangent_point(sketch: &Sketch, line: u64, curve: u64) -> Result<Option<u64>> {
    Ok(match sketch.entity(curve)?.curve {
        Curve::Circle { center, .. } => Some(center),
        Curve::Arc {
            center, start, end, ..
        } => Some(
            sketch
                .entity(line)?
                .point_ids()
                .into_iter()
                .find(|p| *p == start || *p == end)
                .unwrap_or(center),
        ),
        Curve::Line { .. } => None,
    })
}

fn tangent(sketch: &Sketch, selection: Selection<'_>) -> Result<Vec<ConstraintKind>> {
    let ([], [a, b]) = (selection.vertices, selection.entities) else {
        return Ok(vec![]);
    };
    let (a, b) = (*a, *b);
    if is_radial(sketch, a) && is_radial(sketch, b) {
        let (first, r) = sketch.radial(a)?;
        let (second, q) = sketch.radial(b)?;
        return Ok(vec![ConstraintKind::CircleTangent {
            a,
            b,
            internal: distance(first, second) < r.max(q),
        }]);
    }
    let (line, arc) = if is_line(sketch, a) { (a, b) } else { (b, a) };
    if !is_line(sketch, line) || !is_radial(sketch, arc) {
        return Ok(vec![]);
    }
    Ok(tangent_point(sketch, line, arc)?
        .map(|point| ConstraintKind::Tangent { line, arc, point })
        .into_iter()
        .collect())
}

/// Liga a primeira entidade às demais com a mesma regra.
fn chained(
    selection: Selection<'_>,
    accepts: impl Fn(u64) -> bool,
    kind: impl Fn(u64, u64) -> ConstraintKind,
) -> Vec<ConstraintKind> {
    match (selection.vertices, selection.entities) {
        ([], [first, others @ ..])
            if !others.is_empty() && selection.entities.iter().all(|id| accepts(*id)) =>
        {
            others.iter().map(|other| kind(*first, *other)).collect()
        }
        _ => vec![],
    }
}

fn equal(sketch: &Sketch, selection: Selection<'_>) -> Vec<ConstraintKind> {
    let lines = chained(
        selection,
        |id| is_finite_line(sketch, id),
        |a, b| ConstraintKind::EqualLength { a, b },
    );
    if !lines.is_empty() {
        return lines;
    }
    chained(
        selection,
        |id| is_radial(sketch, id),
        |a, b| ConstraintKind::EqualRadius { a, b },
    )
}

fn midpoint(sketch: &Sketch, selection: Selection<'_>) -> Result<Vec<ConstraintKind>> {
    let ([position], [line]) = (selection.vertices, selection.entities) else {
        return Ok(vec![]);
    };
    Ok(match point_at(sketch, *position) {
        Some(point)
            if is_finite_line(sketch, *line)
                && !sketch.entity(*line)?.point_ids().contains(&point) =>
        {
            vec![ConstraintKind::Midpoint { point, line: *line }]
        }
        _ => vec![],
    })
}

fn reflected(point: Point2, a: Point2, b: Point2) -> Point2 {
    let d = sub(b, a);
    let t = dot(sub(point, a), d) / dot(d, d);
    let foot = add(a, scale(d, t));
    sub(scale(foot, 2.), point)
}

/// Dois vértices e um eixo, ou duas linhas e o eixo por último: cada ponta se liga à
/// ponta mais próxima do seu reflexo.
fn symmetry(sketch: &Sketch, selection: Selection<'_>) -> Result<Vec<ConstraintKind>> {
    match (selection.vertices, selection.entities) {
        ([first, second], [axis]) if is_line(sketch, *axis) => Ok(
            match (point_at(sketch, *first), point_at(sketch, *second)) {
                (Some(a), Some(b)) if a != b => {
                    vec![ConstraintKind::Symmetric { a, b, axis: *axis }]
                }
                _ => vec![],
            },
        ),
        ([], [left, right, axis])
            if [left, right, axis]
                .into_iter()
                .all(|id| is_line(sketch, *id)) =>
        {
            let (p, q) = sketch.line(*axis)?;
            let ends = |id: u64| -> Result<(u64, u64)> {
                match sketch.entity(id)?.curve {
                    Curve::Line { start, end } => Ok((start, end)),
                    _ => Err(SketchError("Linha ausente".into())),
                }
            };
            let (a, b) = ends(*left)?;
            let (c, d) = ends(*right)?;
            let target = reflected(sketch.point(a)?, p, q);
            let straight = distance(target, sketch.point(c)?) <= distance(target, sketch.point(d)?);
            let pairs = if straight {
                [(a, c), (b, d)]
            } else {
                [(a, d), (b, c)]
            };
            Ok(pairs
                .map(|(a, b)| ConstraintKind::Symmetric { a, b, axis: *axis })
                .to_vec())
        }
        _ => Ok(vec![]),
    }
}

/// Vértice do contorno do polígono e todos os apoios que chegam nele.
struct Corner {
    position: Point2,
    points: Vec<u64>,
}

/// Cantos de um contorno fechado e simples: cada vértice recebe exatamente duas pontas, e
/// percorrer as linhas passa por todas.
fn polygon_corners(sketch: &Sketch, lines: &[u64]) -> Result<Option<Vec<Corner>>> {
    if lines.len() < 3 || !lines.iter().all(|id| is_finite_line(sketch, *id)) {
        return Ok(None);
    }
    let mut corners: Vec<Corner> = Vec::new();
    let mut touches = Vec::new();
    for id in lines {
        let mut indices = [0; 2];
        for (slot, point) in sketch.entity(*id)?.point_ids().into_iter().enumerate() {
            let position = sketch.point(point)?;
            let found = corners
                .iter()
                .position(|c| distance(c.position, position) < EPS);
            let index = found.unwrap_or_else(|| {
                corners.push(Corner {
                    position,
                    points: Vec::new(),
                });
                corners.len() - 1
            });
            corners[index].points.push(point);
            indices[slot] = index;
        }
        touches.push(indices);
    }
    if corners.len() != lines.len() || corners.iter().any(|c| c.points.len() != 2) {
        return Ok(None);
    }
    let mut seen = vec![false; touches.len()];
    let mut pending = vec![0];
    while let Some(line) = pending.pop() {
        if !std::mem::replace(&mut seen[line], true) {
            pending.extend(
                touches
                    .iter()
                    .enumerate()
                    .filter(|(_, other)| other.iter().any(|c| touches[line].contains(c)))
                    .map(|(index, _)| index),
            );
        }
    }
    Ok(seen.into_iter().all(|visited| visited).then_some(corners))
}

/// Restrições comuns que `apply` gravaria; vazio quando a escolha não se aplica à seleção.
fn plan(
    sketch: &Sketch,
    selection: Selection<'_>,
    restriction: Restriction,
) -> Result<Vec<ConstraintKind>> {
    match restriction {
        Restriction::HorizontalVertical => horizontal_vertical(sketch, selection),
        Restriction::Coincident => coincident(sketch, selection),
        Restriction::Tangent => tangent(sketch, selection),
        Restriction::Equal => Ok(equal(sketch, selection)),
        Restriction::Parallel => Ok(chained(
            selection,
            |id| is_line(sketch, id),
            |a, b| ConstraintKind::Parallel { a, b },
        )),
        Restriction::Perpendicular if selection.entities.len() == 2 => Ok(chained(
            selection,
            |id| is_line(sketch, id),
            |a, b| ConstraintKind::Perpendicular { a, b },
        )),
        Restriction::Midpoint => midpoint(sketch, selection),
        Restriction::Concentric if selection.entities.len() == 2 => Ok(chained(
            selection,
            |id| is_radial(sketch, id),
            |a, b| ConstraintKind::Concentric { a, b },
        )),
        Restriction::Collinear => Ok(chained(
            selection,
            |id| is_line(sketch, id),
            |a, b| ConstraintKind::Collinear { a, b },
        )),
        Restriction::Symmetry => symmetry(sketch, selection),
        _ => Ok(vec![]),
    }
}

/// Só o que vale para a seleção e ainda não existe.
pub fn available(sketch: &Sketch, selection: Selection<'_>) -> Result<Vec<Restriction>> {
    let mut result = Vec::new();
    for restriction in ALL {
        let kinds = plan(sketch, selection, restriction)?;
        if kinds.iter().any(|kind| !exists(sketch, kind)) {
            result.push(restriction);
        }
    }
    if selection.vertices.is_empty() && polygon_corners(sketch, selection.entities)?.is_some() {
        result.push(Restriction::Polygon);
    }
    if let ([position], []) = (selection.vertices, selection.entities)
        && let Some(junction) = junctions::at(sketch, *position)?
    {
        if junction.linked {
            result.push(Restriction::Detach);
        } else if junctions::linkable(sketch, *position)? {
            result.push(Restriction::Link);
        }
        if junctions::can_dissolve(sketch, *position)? {
            result.push(Restriction::Dissolve);
        }
    }
    let places = positions(sketch, selection)?;
    let fixed = places
        .iter()
        .filter(|position| {
            sketch
                .points
                .iter()
                .any(|p| !p.projected && distance(p.position, **position) < EPS)
        })
        .map(|position| junctions::fixed(sketch, *position))
        .collect::<Result<Vec<_>>>()?;
    if fixed.iter().any(|fixed| !fixed) {
        result.push(Restriction::Fix);
    }
    if fixed.iter().any(|fixed| *fixed) {
        result.push(Restriction::Release);
    }
    Ok(result)
}

/// Fixar e liberar dividem uma ferramenta: sem nada a fixar, ela libera.
pub fn resolve(
    sketch: &Sketch,
    selection: Selection<'_>,
    restriction: Restriction,
) -> Result<Restriction> {
    Ok(
        if restriction == Restriction::Fix
            && available(sketch, selection)?
                .iter()
                .all(|r| *r != Restriction::Fix)
        {
            Restriction::Release
        } else {
            restriction
        },
    )
}

/// Apoio que o solver não pode mover: fixado no espaço ou projetado.
fn held(sketch: &Sketch, point: u64) -> bool {
    sketch.points.iter().any(|p| p.id == point && p.projected)
        || sketch.constraints.iter().any(|c| {
            !c.reference && matches!(c.kind, ConstraintKind::Fixed { point: p, .. } if p == point)
        })
}

fn line_points(sketch: &Sketch, line: u64) -> Result<(u64, u64)> {
    match sketch.entity(line)?.curve {
        Curve::Line { start, end } => Ok((start, end)),
        _ => Err(SketchError("Linha ausente".into())),
    }
}

fn movable(sketch: &Sketch, line: u64) -> Result<bool> {
    let (start, end) = line_points(sketch, line)?;
    Ok(!(held(sketch, start) && held(sketch, end)))
}

fn place(sketch: &mut Sketch, point: u64, position: Point2) {
    if let Some(found) = sketch.points.iter_mut().find(|p| p.id == point) {
        found.position = position;
    }
}

/// Reposiciona a linha com a direção e o comprimento pedidos. O giro é pela ponta presa ou,
/// sem ela, pelo meio da linha.
fn reshape_line(sketch: &mut Sketch, line: u64, direction: Point2, length: f64) -> Result<()> {
    let (start, end) = line_points(sketch, line)?;
    let (a, b) = sketch.line(line)?;
    let size = distance([0., 0.], direction);
    if size < EPS || length < EPS {
        return Ok(());
    }
    let step = scale(direction, length / size);
    // A ponta que continua à frente é a mais próxima da direção atual.
    let step = if dot(step, sub(b, a)) < 0. {
        scale(step, -1.)
    } else {
        step
    };
    let (first, second) = match (held(sketch, start), held(sketch, end)) {
        (true, true) => return Ok(()),
        (true, false) => (a, add(a, step)),
        (false, true) => (sub(b, step), b),
        (false, false) => {
            let middle = scale(add(a, b), 0.5);
            (sub(middle, scale(step, 0.5)), add(middle, scale(step, 0.5)))
        }
    };
    place(sketch, start, first);
    place(sketch, end, second);
    Ok(())
}

fn line_vector(sketch: &Sketch, line: u64) -> Result<(Point2, f64)> {
    let (a, b) = sketch.line(line)?;
    Ok((sub(b, a), distance(a, b)))
}

/// Gira a segunda linha para a direção da primeira, ou o contrário se ela estiver presa.
fn align_pair(sketch: &mut Sketch, a: u64, b: u64, turn: fn(Point2) -> Point2) -> Result<()> {
    let (target, moving) = if movable(sketch, b)? { (a, b) } else { (b, a) };
    let (direction, _) = line_vector(sketch, target)?;
    let (_, length) = line_vector(sketch, moving)?;
    reshape_line(sketch, moving, turn(direction), length)
}

/// O solver só empurra as pontas na perpendicular da linha: num giro grande, ela esticaria
/// ou encolheria. Partir da posição certa preserva o comprimento; o solver só confere.
fn preposition(sketch: &mut Sketch, kind: &ConstraintKind) -> Result<()> {
    match *kind {
        ConstraintKind::Horizontal { line } => {
            let (_, length) = line_vector(sketch, line)?;
            reshape_line(sketch, line, [1., 0.], length)
        }
        ConstraintKind::Vertical { line } => {
            let (_, length) = line_vector(sketch, line)?;
            reshape_line(sketch, line, [0., 1.], length)
        }
        ConstraintKind::Parallel { a, b } => align_pair(sketch, a, b, |d| d),
        ConstraintKind::Perpendicular { a, b } => align_pair(sketch, a, b, |d| [-d[1], d[0]]),
        ConstraintKind::EqualLength { a, b } => {
            let (target, moving) = if movable(sketch, b)? { (a, b) } else { (b, a) };
            let (_, length) = line_vector(sketch, target)?;
            let (direction, _) = line_vector(sketch, moving)?;
            reshape_line(sketch, moving, direction, length)
        }
        ConstraintKind::Collinear { a, b } => {
            align_pair(sketch, a, b, |d| d)?;
            if !movable(sketch, b)? {
                return Ok(());
            }
            let (origin, _) = sketch.line(a)?;
            let (direction, _) = line_vector(sketch, a)?;
            let (start, end) = line_points(sketch, b)?;
            let (p, q) = sketch.line(b)?;
            let foot = |point: Point2| {
                add(
                    origin,
                    scale(
                        direction,
                        dot(sub(point, origin), direction) / dot(direction, direction),
                    ),
                )
            };
            let (first, second) = (foot(p), foot(q));
            place(sketch, start, first);
            place(sketch, end, second);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn record(sketch: &mut Sketch, kind: ConstraintKind) -> Result<()> {
    if let Some(reversed) = opposite(&kind) {
        sketch.constraints.retain(|c| !same(&c.kind, &reversed));
    }
    // Levar à origem um vértice fixado em outro lugar troca a fixação, sem criar conflito.
    if let ConstraintKind::Fixed { point, .. } = kind {
        sketch.constraints.retain(
            |c| !matches!(c.kind, ConstraintKind::Fixed { point: other, .. } if other == point),
        );
    }
    if let Some(existing) = sketch.constraints.iter_mut().find(|c| same(&c.kind, &kind)) {
        existing.reference = false;
    } else {
        sketch.add_constraint(kind, false)?;
    }
    Ok(())
}

/// Polígono regular: vértices sobre um círculo de referência e lados iguais. O círculo fica
/// no rascunho como traço de referência e dá o raio e o centro para cotar.
fn add_polygon(sketch: &mut Sketch, lines: &[u64]) -> Result<()> {
    let corners = polygon_corners(sketch, lines)?
        .ok_or_else(|| SketchError("Selecione as linhas de um contorno fechado.".into()))?;
    let count = corners.len() as f64;
    let center = corners
        .iter()
        .fold([0., 0.], |sum, c| add(sum, c.position))
        .map(|value| value / count);
    let radius = corners
        .iter()
        .map(|c| distance(c.position, center))
        .sum::<f64>()
        / count;
    let circle = sketch.add_circle(center, radius, true)?;
    for corner in &corners {
        let [point, rest @ ..] = &corner.points[..] else {
            continue;
        };
        record(
            sketch,
            ConstraintKind::PointOnCurve {
                point: *point,
                curve: circle,
            },
        )?;
        for other in rest.iter().filter(|other| *other != point) {
            record(
                sketch,
                ConstraintKind::Coincident {
                    a: *point,
                    b: *other,
                },
            )?;
        }
    }
    for other in &lines[1..] {
        record(
            sketch,
            ConstraintKind::EqualLength {
                a: lines[0],
                b: *other,
            },
        )?;
    }
    Ok(())
}

pub fn apply(
    sketch: &Sketch,
    selection: Selection<'_>,
    restriction: Restriction,
) -> Result<Sketch> {
    let mut next = sketch.clone();
    match restriction {
        Restriction::Polygon => add_polygon(&mut next, selection.entities)?,
        Restriction::Link | Restriction::Detach | Restriction::Dissolve => {
            let [position] = selection.vertices else {
                return Err(SketchError("Escolha um vértice.".into()));
            };
            match restriction {
                Restriction::Link => junctions::link(&mut next, *position)?,
                Restriction::Detach => junctions::detach(&mut next, *position)?,
                _ => junctions::dissolve(&mut next, *position)?,
            };
        }
        Restriction::Fix | Restriction::Release => {
            for position in positions(sketch, selection)? {
                let movable = next
                    .points
                    .iter()
                    .any(|p| !p.projected && distance(p.position, position) < EPS);
                if movable {
                    junctions::set_fixed(&mut next, position, restriction == Restriction::Fix)?;
                }
            }
        }
        _ => {
            let kinds = plan(sketch, selection, restriction)?;
            if kinds.is_empty() {
                return Err(SketchError(
                    "A restrição não se aplica a esta seleção.".into(),
                ));
            }
            for kind in kinds {
                preposition(&mut next, &kind)?;
                record(&mut next, kind)?;
            }
        }
    }
    next.validate()?;
    Ok(next)
}

/// Restrições que tocam a seleção, para listar e remover. Ligações fracas de projetos antigos
/// ficam de fora: não têm efeito, e cotas de referência continuam aparecendo.
pub fn related(sketch: &Sketch, selection: Selection<'_>) -> Vec<u64> {
    let points = selection
        .vertices
        .iter()
        .flat_map(|position| {
            sketch
                .points
                .iter()
                .filter(|p| distance(p.position, *position) < EPS)
                .map(|p| p.id)
        })
        .chain(selection.entities.iter().flat_map(|id| {
            sketch
                .entity(*id)
                .map(Entity::point_ids)
                .unwrap_or_default()
        }))
        .collect::<HashSet<_>>();
    sketch
        .constraints
        .iter()
        .filter(|c| !c.reference || c.kind.is_dimension())
        .filter(|c| {
            let (own_points, entities) = dimensions::participants(&c.kind);
            own_points.iter().any(|p| points.contains(p))
                || entities.iter().any(|e| selection.entities.contains(e))
        })
        .map(|c| c.id)
        .collect()
}

/// Remove a restrição e o que a acompanha: expressão, texto e alvo da cota.
pub fn remove(sketch: &mut Sketch, id: u64) {
    sketch.constraints.retain(|c| c.id != id);
    sketch.dimension_expressions.remove(&id);
    sketch.dimension_labels.remove(&id);
    sketch.dimension_targets.remove(&id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{constraint_state, solver};
    use Restriction::*;

    fn entities<'a>(ids: &'a [u64]) -> Selection<'a> {
        Selection {
            vertices: &[],
            entities: ids,
        }
    }
    fn vertices<'a>(positions: &'a [Point2]) -> Selection<'a> {
        Selection {
            vertices: positions,
            entities: &[],
        }
    }
    fn mixed<'a>(positions: &'a [Point2], ids: &'a [u64]) -> Selection<'a> {
        Selection {
            vertices: positions,
            entities: ids,
        }
    }
    fn arc(sketch: &mut Sketch, center: Point2, start: Point2, end: Point2) -> Result<u64> {
        let center = sketch.add_point(center)?;
        let start = sketch.add_point(start)?;
        let end = sketch.add_point(end)?;
        sketch.add_curve(
            Curve::Arc {
                center,
                start,
                end,
                clockwise: false,
            },
            false,
        )
    }
    fn solved(sketch: &Sketch) -> Result<Sketch> {
        let report = solver::solve(sketch)?;
        assert_ne!(report.status, solver::Status::Conflict);
        Ok(report.sketch)
    }

    #[test]
    fn offers_only_what_fits_the_selection() -> Result<()> {
        let mut sketch = Sketch::new("Seleção", Plane::xy());
        let a = sketch.add_line([0., 0.], [10., 1.], false)?;
        let b = sketch.add_line([0., 5.], [10., 7.], false)?;
        let circle = sketch.add_circle([30., 0.], 5., false)?;
        let other = sketch.add_circle([30., 2.], 8., false)?;

        assert_eq!(
            available(&sketch, entities(&[a]))?,
            [HorizontalVertical, Fix]
        );
        assert_eq!(
            available(&sketch, entities(&[a, b]))?,
            [
                HorizontalVertical,
                Coincident,
                Equal,
                Parallel,
                Perpendicular,
                Collinear,
                Fix
            ]
        );
        assert_eq!(
            available(&sketch, entities(&[a, circle]))?,
            [Coincident, Tangent, Fix]
        );
        assert_eq!(
            available(&sketch, entities(&[circle, other]))?,
            [Coincident, Tangent, Equal, Concentric, Fix]
        );
        assert_eq!(available(&sketch, vertices(&[[0., 0.]]))?, [Fix]);
        assert_eq!(
            available(&sketch, vertices(&[[0., 0.], [0., 5.]]))?,
            [HorizontalVertical, Coincident, Fix]
        );
        assert_eq!(
            available(&sketch, mixed(&[[0., 5.]], &[a]))?,
            [Coincident, Midpoint, Fix]
        );
        // Ponta da própria linha não pode ser presa a ela.
        assert_eq!(available(&sketch, mixed(&[[0., 0.]], &[a]))?, [Fix]);
        assert_eq!(
            available(&sketch, mixed(&[[0., 0.], [0., 5.]], &[a]))?,
            [Symmetry, Fix]
        );
        Ok(())
    }

    #[test]
    fn horizontal_vertical_picks_the_direction_and_alternates_on_repeat() -> Result<()> {
        let mut sketch = Sketch::new("Direção", Plane::xy());
        let line = sketch.add_line([0., 0.], [10., 1.], false)?;
        let first = apply(&sketch, entities(&[line]), HorizontalVertical)?;
        assert!(
            first
                .constraints
                .iter()
                .any(|c| matches!(c.kind, ConstraintKind::Horizontal { line: l } if l == line))
        );
        let (a, b) = solved(&first)?.line(line)?;
        assert!((a[1] - b[1]).abs() < 1e-6);
        let second = apply(&first, entities(&[line]), HorizontalVertical)?;
        assert_eq!(second.constraints.len(), 1);
        assert!(
            second
                .constraints
                .iter()
                .any(|c| matches!(c.kind, ConstraintKind::Vertical { line: l } if l == line))
        );

        // Dois vértices sem linha entre eles.
        let mut loose = Sketch::new("Vértices", Plane::xy());
        loose.add_line([0., 0.], [5., 20.], false)?;
        loose.add_line([40., 3.], [45., 30.], false)?;
        let pair: &[Point2] = &[[0., 0.], [40., 3.]];
        let aligned = solved(&apply(&loose, vertices(pair), HorizontalVertical)?)?;
        let [p, q] = [aligned.points[0].position, aligned.points[2].position];
        assert!((p[1] - q[1]).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn applied_restriction_leaves_the_menu_and_can_be_removed() -> Result<()> {
        let mut sketch = Sketch::new("Aplicar", Plane::xy());
        let a = sketch.add_line([0., 0.], [10., 1.], false)?;
        let b = sketch.add_line([0., 5.], [10., 7.], false)?;
        let applied = apply(&sketch, entities(&[a, b]), Parallel)?;
        assert_eq!(sketch.constraints.len(), 0);
        assert!(!available(&applied, entities(&[a, b]))?.contains(&Parallel));
        let ids = related(&applied, entities(&[a]));
        assert_eq!(ids.len(), 1);
        let mut removed = applied;
        remove(&mut removed, ids[0]);
        assert!(removed.constraints.is_empty());
        Ok(())
    }

    #[test]
    fn fixing_an_edge_holds_both_ends_and_the_tool_releases_when_all_is_fixed() -> Result<()> {
        let mut sketch = Sketch::new("Aresta", Plane::xy());
        let line = sketch.add_line([0., 0.], [10., 0.], false)?;
        let fixed = apply(&sketch, entities(&[line]), Fix)?;
        let state = constraint_state::analyze(&fixed)?;
        assert!(state.entity_restricted(line));
        assert_eq!(resolve(&fixed, entities(&[line]), Fix)?, Release);
        assert_eq!(resolve(&sketch, entities(&[line]), Fix)?, Fix);
        let released = apply(&fixed, entities(&[line]), Release)?;
        assert!(released.constraints.is_empty());
        Ok(())
    }

    #[test]
    fn vertex_link_and_detach_are_offered_by_its_real_state() -> Result<()> {
        let mut sketch = Sketch::new("Vértice", Plane::xy());
        sketch.add_rectangle([0., 0.], [20., 10.], false)?;
        let corner: &[Point2] = &[[0., 0.]];
        assert!(available(&sketch, vertices(corner))?.contains(&Detach));
        let detached = apply(&sketch, vertices(corner), Detach)?;
        assert!(available(&detached, vertices(corner))?.contains(&Link));
        let linked = apply(&detached, vertices(corner), Link)?;
        assert!(available(&linked, vertices(corner))?.contains(&Detach));
        Ok(())
    }

    #[test]
    fn related_lists_dimensions_of_the_edge_and_hides_old_weak_links() -> Result<()> {
        let mut sketch = Sketch::new("Lista", Plane::xy());
        let line = sketch.add_line([0., 0.], [10., 0.], false)?;
        let [a, b] = sketch.entity(line)?.point_ids()[..] else {
            return Err(SketchError("Linha sem pontas".into()));
        };
        let dimension =
            sketch.add_constraint(ConstraintKind::Distance { a, b, value: 10. }, false)?;
        sketch.add_constraint(ConstraintKind::Horizontal { line }, true)?;
        assert_eq!(related(&sketch, entities(&[line])), [dimension]);
        Ok(())
    }

    fn length(sketch: &Sketch, line: u64) -> Result<f64> {
        let (a, b) = sketch.line(line)?;
        Ok(distance(a, b))
    }

    #[test]
    fn coincident_accepts_edges_vertices_and_the_origin_in_any_order() -> Result<()> {
        let mut sketch = Sketch::new("Origem", Plane::xy());
        let line = sketch.add_line([5., 7.], [20., 3.], false)?;
        let other = sketch.add_line([30., 5.], [40., 9.], false)?;
        let circle = sketch.add_circle([-20., -5.], 6., false)?;
        let (start, _) = line_points(&sketch, line)?;

        // Vértice e origem, nas duas ordens.
        for pair in [[[5., 7.], ORIGIN], [ORIGIN, [5., 7.]]] {
            assert!(available(&sketch, vertices(&pair))?.contains(&Coincident));
            let result = solved(&apply(&sketch, vertices(&pair), Coincident)?)?;
            assert!(distance(result.point(start)?, ORIGIN) < 1e-6);
        }

        // Aresta e origem: a reta suporte e a borda do círculo passam por ela.
        let origin: &[Point2] = &[ORIGIN];
        let result = solved(&apply(&sketch, mixed(origin, &[line]), Coincident)?)?;
        let (p, q) = result.line(line)?;
        assert!(cross(sub(ORIGIN, p), sub(q, p)).abs() < 1e-5);
        let result = solved(&apply(&sketch, mixed(origin, &[circle]), Coincident)?)?;
        let (center, radius) = result.radial(circle)?;
        assert!((distance(center, ORIGIN) - radius).abs() < 1e-5);

        // Aplicada, a restrição sai das opções e aparece na lista da aresta.
        let applied = apply(&sketch, mixed(origin, &[line]), Coincident)?;
        assert!(!available(&applied, mixed(origin, &[line]))?.contains(&Coincident));
        assert_eq!(related(&applied, entities(&[line])).len(), 1);

        // Vértice e aresta, e duas arestas, continuam valendo.
        let pair: &[Point2] = &[[30., 5.]];
        let result = solved(&apply(&sketch, mixed(pair, &[line]), Coincident)?)?;
        let (a, b) = result.line(line)?;
        let point = result.line(other)?.0;
        assert!(cross(sub(point, a), sub(b, a)).abs() < 1e-5);
        assert!(available(&sketch, entities(&[line, other]))?.contains(&Coincident));
        Ok(())
    }

    #[test]
    fn a_real_vertex_at_the_origin_takes_priority_and_a_fixed_vertex_moves_there() -> Result<()> {
        let mut sketch = Sketch::new("Prioridade", Plane::xy());
        let at_origin = sketch.add_line(ORIGIN, [10., 0.], false)?;
        let other = sketch.add_line([5., 7.], [20., 3.], false)?;
        let (corner, _) = line_points(&sketch, at_origin)?;
        let (start, _) = line_points(&sketch, other)?;
        // Com um vértice na origem, a seleção é dele: as pontas ficam ligadas, não presas.
        let applied = apply(&sketch, vertices(&[ORIGIN, [5., 7.]]), Coincident)?;
        assert!(applied.constraints.iter().any(|c| {
            matches!(c.kind, ConstraintKind::Coincident { a, b } if a == corner && b == start)
        }));

        let mut loose = Sketch::new("Fixado", Plane::xy());
        let line = loose.add_line([5., 7.], [20., 3.], false)?;
        let (point, _) = line_points(&loose, line)?;
        loose.add_constraint(
            ConstraintKind::Fixed {
                point,
                position: [5., 7.],
            },
            false,
        )?;
        let result = solved(&apply(&loose, vertices(&[[5., 7.], ORIGIN]), Coincident)?)?;
        assert!(distance(result.point(point)?, ORIGIN) < 1e-6);
        assert_eq!(result.constraints.len(), 1);
        Ok(())
    }

    #[test]
    fn large_turns_keep_the_length_of_the_line() -> Result<()> {
        let mut sketch = Sketch::new("Giro", Plane::xy());
        let turning = sketch.add_line([0., 0.], [10., 15.], false)?;
        let base = sketch.add_line([0., 20.], [30., 20.], false)?;
        let size = length(&sketch, turning)?;
        for restriction in [Parallel, Perpendicular] {
            let result = solved(&apply(&sketch, entities(&[base, turning]), restriction)?)?;
            assert!((length(&result, turning)? - size).abs() < 1e-5);
            assert!((length(&result, base)? - 30.).abs() < 1e-5);
        }
        let result = solved(&apply(&sketch, entities(&[turning]), HorizontalVertical)?)?;
        assert!((length(&result, turning)? - size).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn the_held_end_is_the_pivot_and_a_fully_held_line_stays_put() -> Result<()> {
        let mut sketch = Sketch::new("Pivô", Plane::xy());
        let turning = sketch.add_line([0., 0.], [10., 15.], false)?;
        let base = sketch.add_line([0., 20.], [30., 20.], false)?;
        let (start, _) = line_points(&sketch, turning)?;
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: start,
                position: [0., 0.],
            },
            false,
        )?;
        let (p, q) = line_points(&sketch, base)?;
        for (point, position) in [(p, [0., 20.]), (q, [30., 20.])] {
            sketch.add_constraint(ConstraintKind::Fixed { point, position }, false)?;
        }
        // A base está presa: gira a outra linha, em torno da ponta fixada.
        let size = length(&sketch, turning)?;
        let result = solved(&apply(&sketch, entities(&[turning, base]), Parallel)?)?;
        assert_eq!(result.point(start)?, [0., 0.]);
        assert!((length(&result, turning)? - size).abs() < 1e-5);
        assert!((result.line(turning)?.1[1]).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn collinear_keeps_both_ends_of_the_second_line_on_the_first() -> Result<()> {
        let mut sketch = Sketch::new("Colineares", Plane::xy());
        let a = sketch.add_line([0., 0.], [10., 1.], false)?;
        let b = sketch.add_line([20., 3.], [30., 3.], false)?;
        let result = solved(&apply(&sketch, entities(&[a, b]), Collinear)?)?;
        let (p, q) = result.line(a)?;
        let (r, s) = result.line(b)?;
        let d = sub(q, p);
        assert!(cross(sub(r, p), d).abs() < 1e-5);
        assert!(cross(sub(s, p), d).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn symmetry_mirrors_vertices_and_lines_about_the_axis() -> Result<()> {
        let mut sketch = Sketch::new("Simetria", Plane::xy());
        let axis = sketch.add_line([0., -20.], [0., 20.], false)?;
        let left = sketch.add_line([-10., 0.], [-12., 8.], false)?;
        let right = sketch.add_line([13., 9.], [9., 1.], false)?;
        let result = solved(&apply(&sketch, entities(&[left, right, axis]), Symmetry)?)?;
        let (a, b) = result.line(left)?;
        let (c, d) = result.line(right)?;
        let (p, q) = result.line(axis)?;
        // As pontas se casam pelo reflexo: a primeira de `left` com a segunda de `right`.
        assert!(distance(reflected(a, p, q), d) < 1e-5);
        assert!(distance(reflected(b, p, q), c) < 1e-5);

        let pair: &[Point2] = &[[-10., 0.], [9., 1.]];
        let result = solved(&apply(&sketch, mixed(pair, &[axis]), Symmetry)?)?;
        let (p, q) = result.line(axis)?;
        let (a, _) = result.line(left)?;
        let (_, d) = result.line(right)?;
        assert!(distance(reflected(a, p, q), d) < 1e-5);
        Ok(())
    }

    #[test]
    fn equal_compares_lengths_or_radii_and_chains_every_selected_element() -> Result<()> {
        let mut sketch = Sketch::new("Iguais", Plane::xy());
        let a = sketch.add_line([0., 0.], [10., 0.], false)?;
        let b = sketch.add_line([0., 5.], [14., 5.], false)?;
        let c = sketch.add_line([0., 9.], [7., 9.], false)?;
        let result = solved(&apply(&sketch, entities(&[a, b, c]), Equal)?)?;
        let length = |id| -> Result<f64> {
            let (p, q) = result.line(id)?;
            Ok(distance(p, q))
        };
        assert!((length(a)? - length(b)?).abs() < 1e-5);
        assert!((length(a)? - length(c)?).abs() < 1e-5);

        let mut circles = Sketch::new("Raios", Plane::xy());
        let a = circles.add_circle([0., 0.], 5., false)?;
        let b = circles.add_circle([30., 0.], 8., false)?;
        let result = solved(&apply(&circles, entities(&[a, b]), Equal)?)?;
        assert!((result.radial(a)?.1 - result.radial(b)?.1).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn tangent_covers_line_and_arc_away_from_the_ends_and_two_circles() -> Result<()> {
        let mut sketch = Sketch::new("Tangências", Plane::xy());
        let line = sketch.add_line([-20., 12.], [20., 12.], false)?;
        let curve = arc(&mut sketch, [0., 0.], [10., 0.], [0., 10.])?;
        let result = solved(&apply(&sketch, entities(&[line, curve]), Tangent)?)?;
        let (p, q) = result.line(line)?;
        let (center, radius) = result.radial(curve)?;
        let reach = cross(sub(center, p), sub(q, p)).abs() / distance(p, q);
        assert!((reach - radius).abs() < 1e-5);

        let mut circles = Sketch::new("Círculos", Plane::xy());
        let a = circles.add_circle([0., 0.], 5., false)?;
        let b = circles.add_circle([30., 0.], 8., false)?;
        let result = solved(&apply(&circles, entities(&[a, b]), Tangent)?)?;
        let (first, r) = result.radial(a)?;
        let (second, q) = result.radial(b)?;
        assert!((distance(first, second) - (r + q)).abs() < 1e-5);

        // Um círculo dentro do outro toca por dentro.
        let mut nested = Sketch::new("Dentro", Plane::xy());
        let a = nested.add_circle([0., 0.], 20., false)?;
        let b = nested.add_circle([3., 0.], 5., false)?;
        let result = solved(&apply(&nested, entities(&[a, b]), Tangent)?)?;
        let (first, r) = result.radial(a)?;
        let (second, q) = result.radial(b)?;
        assert!((distance(first, second) - (r - q).abs()).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn concentric_accepts_arcs_and_coincident_uses_the_closest_ends() -> Result<()> {
        let mut sketch = Sketch::new("Centros", Plane::xy());
        let circle = sketch.add_circle([0., 0.], 5., false)?;
        let curve = arc(&mut sketch, [3., 1.], [13., 1.], [3., 11.])?;
        let result = solved(&apply(&sketch, entities(&[circle, curve]), Concentric)?)?;
        assert!(distance(result.radial(circle)?.0, result.radial(curve)?.0) < 1e-5);

        let mut lines = Sketch::new("Pontas", Plane::xy());
        let a = lines.add_line([0., 0.], [10., 0.], false)?;
        let b = lines.add_line([11., 1.], [20., 8.], false)?;
        let result = solved(&apply(&lines, entities(&[a, b]), Coincident)?)?;
        assert!(distance(result.line(a)?.1, result.line(b)?.0) < 1e-5);
        Ok(())
    }

    #[test]
    fn polygon_makes_a_closed_contour_regular_and_refuses_open_ones() -> Result<()> {
        let mut sketch = Sketch::new("Polígono", Plane::xy());
        let lines = sketch.add_rectangle([0., 0.], [20., 12.], false)?;
        let mut free = Sketch::new("Quadrilátero", Plane::xy());
        let ids = [
            free.add_line([0., 0.], [20., 1.], false)?,
            free.add_line([20., 1.], [21., 13.], false)?,
            free.add_line([21., 13.], [-1., 12.], false)?,
            free.add_line([-1., 12.], [0., 0.], false)?,
        ];
        assert!(available(&sketch, entities(&lines))?.contains(&Polygon));
        assert!(!available(&sketch, entities(&lines[..3]))?.contains(&Polygon));
        let before = free.entities.len();
        let result = solved(&apply(&free, entities(&ids), Polygon)?)?;
        // O círculo de referência fica no rascunho, fora do perfil.
        assert_eq!(result.entities.len(), before + 1);
        assert!(result.entities.last().is_some_and(|e| e.construction));
        let side = |id| -> Result<f64> {
            let (p, q) = result.line(id)?;
            Ok(distance(p, q))
        };
        for id in &ids[1..] {
            assert!((side(*id)? - side(ids[0])?).abs() < 1e-5);
        }
        // Quadrado: as diagonais medem o mesmo.
        let (a, b) = result.line(ids[0])?;
        let (c, d) = result.line(ids[2])?;
        assert!((distance(a, c) - distance(b, d)).abs() < 1e-4);
        Ok(())
    }
}
