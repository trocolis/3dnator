//! Áreas elementares do arranjo plano de linhas, círculos e arcos. Interseções
//! subdividem curvas apenas na consulta, sem aparar entidades nem alterar o rascunho.
//! Cada meia-aresta percorre a face à esquerda; ramos abertos não inventam fechamentos.
use crate::{
    profiles::{Region, Segment, Wire},
    *,
};
const EPS: f64 = 1e-6;
const MAX_EDGES: usize = 8192;

fn radial(s: &Segment) -> Option<(Point2, f64)> {
    match *s {
        Segment::Circle { center, radius } => Some((center, radius)),
        Segment::Arc { center, a, .. } => Some((center, distance(center, a))),
        _ => None,
    }
}
fn point(s: &Segment, t: f64) -> Point2 {
    match *s {
        Segment::Line { a, b } => add(a, scale(sub(b, a), t)),
        Segment::Circle { center, radius } => add(
            center,
            scale(
                [
                    (t * std::f64::consts::TAU).cos(),
                    (t * std::f64::consts::TAU).sin(),
                ],
                radius,
            ),
        ),
        Segment::Arc {
            center,
            a,
            b,
            clockwise,
        } => {
            let d = sub(a, center);
            let end = sub(b, center);
            let sign = if clockwise { -1. } else { 1. };
            let start = d[1].atan2(d[0]);
            let sweep = ((end[1].atan2(end[0]) - start) * sign).rem_euclid(std::f64::consts::TAU);
            let angle = start + sign * sweep * t;
            add(
                center,
                scale([angle.cos(), angle.sin()], distance(center, a)),
            )
        }
    }
}
fn parameter(s: &Segment, p: Point2) -> Option<f64> {
    match *s {
        Segment::Line { a, b } => {
            let d = sub(b, a);
            let t = dot(sub(p, a), d) / dot(d, d);
            ((-EPS..=1. + EPS).contains(&t) && distance(point(s, t), p) < EPS)
                .then_some(t.clamp(0., 1.))
        }
        Segment::Circle { center, radius } => {
            let d = sub(p, center);
            ((distance(center, p) - radius).abs() < EPS).then_some(
                d[1].atan2(d[0]).rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU,
            )
        }
        Segment::Arc {
            center,
            a,
            b,
            clockwise,
        } => {
            if distance(p, a) < EPS {
                return Some(0.);
            }
            if distance(p, b) < EPS {
                return Some(1.);
            }
            let sign = if clockwise { -1. } else { 1. };
            let first = sub(a, center);
            let last = sub(b, center);
            let d = sub(p, center);
            let start = first[1].atan2(first[0]);
            let sweep = ((last[1].atan2(last[0]) - start) * sign).rem_euclid(std::f64::consts::TAU);
            let travel = ((d[1].atan2(d[0]) - start) * sign).rem_euclid(std::f64::consts::TAU);
            (sweep > 1e-12
                && travel <= sweep + 1e-10
                && (distance(center, p) - distance(center, a)).abs() < EPS)
                .then_some((travel / sweep).clamp(0., 1.))
        }
    }
}
fn intersections(a: &Segment, b: &Segment) -> Vec<Point2> {
    match (a, b) {
        (Segment::Line { a, b }, Segment::Line { a: c, b: d }) => {
            if let Some((p, _, _)) = profiles::line_intersection(*a, *b, *c, *d) {
                vec![p]
            } else {
                vec![*a, *b, *c, *d]
            }
        }
        (Segment::Line { a, b }, curve) | (curve, Segment::Line { a, b }) => {
            let Some((center, radius)) = radial(curve) else {
                return Vec::new();
            };
            let d = sub(*b, *a);
            let offset = sub(*a, center);
            let aa = dot(d, d);
            let bb = 2. * dot(d, offset);
            let cc = dot(offset, offset) - radius * radius;
            let discriminant = bb * bb - 4. * aa * cc;
            if discriminant < -1e-9 * aa {
                return Vec::new();
            }
            let root = discriminant.max(0.).sqrt();
            [-bb - root, -bb + root]
                .map(|v| add(*a, scale(d, v / (2. * aa))))
                .to_vec()
        }
        _ => {
            let Some((c, r)) = radial(a) else {
                return Vec::new();
            };
            let Some((d, s)) = radial(b) else {
                return Vec::new();
            };
            let length = distance(c, d);
            if length < EPS {
                return vec![point(a, 0.), point(a, 1.), point(b, 0.), point(b, 1.)];
            }
            if length > r + s + EPS || length < (r - s).abs() - EPS {
                return Vec::new();
            }
            let x = (r * r - s * s + length * length) / (2. * length);
            let h = (r * r - x * x).max(0.).sqrt();
            let u = scale(sub(d, c), 1. / length);
            let middle = add(c, scale(u, x));
            vec![
                add(middle, [-u[1] * h, u[0] * h]),
                add(middle, [u[1] * h, -u[0] * h]),
            ]
        }
    }
}
fn slice(s: &Segment, from: f64, to: f64) -> Segment {
    match *s {
        Segment::Line { .. } => Segment::Line {
            a: point(s, from),
            b: point(s, to),
        },
        Segment::Circle { center, .. } => Segment::Arc {
            center,
            a: point(s, from),
            b: point(s, to),
            clockwise: false,
        },
        Segment::Arc {
            center, clockwise, ..
        } => Segment::Arc {
            center,
            a: point(s, from),
            b: point(s, to),
            clockwise,
        },
    }
}
struct Edge {
    a: usize,
    b: usize,
    curve: Segment,
    entity: u64,
}
fn endpoints(edge: &Edge, reverse: bool) -> (usize, usize) {
    if reverse {
        (edge.b, edge.a)
    } else {
        (edge.a, edge.b)
    }
}
fn oriented(edges: &[Edge], half: usize) -> Segment {
    let edge = &edges[half / 2];
    if half.is_multiple_of(2) {
        edge.curve.clone()
    } else {
        edge.curve.reversed()
    }
}

pub fn regions(sketch: &Sketch) -> Result<Vec<Region>> {
    sketch.validate()?;
    let mut curves = sketch
        .entities
        .iter()
        .filter(|e| !e.construction)
        .map(|e| Ok((e.id, profiles::segment(sketch, e)?, vec![0., 1.])))
        .collect::<Result<Vec<_>>>()?;
    curves.sort_by_key(|c| c.0);
    for i in 0..curves.len() {
        for j in i + 1..curves.len() {
            for p in intersections(&curves[i].1, &curves[j].1) {
                if let Some(a) = parameter(&curves[i].1, p)
                    && let Some(b) = parameter(&curves[j].1, p)
                {
                    curves[i].2.push(a);
                    curves[j].2.push(b);
                }
            }
        }
    }
    let mut nodes: Vec<Point2> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut wires: Vec<Wire> = Vec::new();
    for (entity, curve, mut cuts) in curves {
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-10);
        if cuts.len() == 2 && matches!(curve, Segment::Circle { .. }) {
            if !wires.iter().any(|w| {
                radial(&w.segments[0])
                    .zip(radial(&curve))
                    .is_some_and(|((a, r), (b, s))| distance(a, b) < EPS && (r - s).abs() < EPS)
            }) {
                wires.push(Wire {
                    segments: vec![curve],
                    entities: vec![entity],
                });
            }
            continue;
        }
        for cut in cuts.windows(2) {
            let piece = slice(&curve, cut[0], cut[1]);
            let (a, b) = (point(&piece, 0.), point(&piece, 1.));
            if distance(a, b) < EPS {
                continue;
            }
            let mut find = |p: Point2| {
                if let Some(i) = nodes.iter().position(|q| distance(*q, p) < EPS) {
                    i
                } else {
                    nodes.push(p);
                    nodes.len() - 1
                }
            };
            let (a, b) = (find(a), find(b));
            if edges.iter().any(|e| {
                ((e.a == a && e.b == b) || (e.a == b && e.b == a))
                    && distance(point(&e.curve, 0.5), point(&piece, 0.5)) < EPS
            }) {
                continue;
            }
            edges.push(Edge {
                a,
                b,
                curve: piece,
                entity,
            });
            if edges.len() > MAX_EDGES {
                return Err(SketchError(
                    "Interseções demais neste rascunho; simplifique o perfil.".into(),
                ));
            }
        }
    }
    let mut outgoing = vec![Vec::new(); nodes.len()];
    for (i, edge) in edges.iter().enumerate() {
        outgoing[edge.a].push(i * 2);
        outgoing[edge.b].push(i * 2 + 1);
    }
    for list in &mut outgoing {
        list.sort_by(|a, b| {
            let angle = |h| {
                let s = oriented(&edges, h);
                let d = sub(point(&s, 0.0001), point(&s, 0.));
                d[1].atan2(d[0])
            };
            angle(*a).total_cmp(&angle(*b))
        });
    }
    let mut visited = vec![false; edges.len() * 2];
    for first in 0..visited.len() {
        if visited[first] {
            continue;
        }
        let mut half = first;
        let mut walk = Vec::new();
        for _ in 0..=visited.len() {
            if visited[half] {
                break;
            }
            visited[half] = true;
            walk.push(half);
            let (_, end) = endpoints(&edges[half / 2], half % 2 == 1);
            let list = &outgoing[end];
            let twin = half ^ 1;
            let index = list
                .iter()
                .position(|h| *h == twin)
                .ok_or_else(|| SketchError("Arranjo de perfil inconsistente.".into()))?;
            half = list[(index + list.len() - 1) % list.len()];
        }
        if half != first {
            continue;
        }
        // Desmembrar vértices repetidos elimina espículas/ponte entre contornos.
        // Ramos percorridos nos dois sentidos viram ciclos de área zero, descartados.
        let mut stack = Vec::new();
        let mut path = vec![endpoints(&edges[first / 2], first % 2 == 1).0];
        for h in walk {
            let (_, end) = endpoints(&edges[h / 2], h % 2 == 1);
            if let Some(index) = path.iter().position(|n| *n == end) {
                let loop_edges = stack[index..]
                    .iter()
                    .copied()
                    .chain(std::iter::once(h))
                    .collect::<Vec<_>>();
                let wire = Wire {
                    segments: loop_edges.iter().map(|h| oriented(&edges, *h)).collect(),
                    entities: loop_edges.iter().map(|h| edges[*h / 2].entity).collect(),
                };
                if wire.area() > 1e-10 {
                    wires.push(wire);
                }
                stack.truncate(index);
                path.truncate(index + 1);
            } else {
                stack.push(h);
                path.push(end);
            }
        }
    }
    wires.sort_by(|a, b| b.area().abs().total_cmp(&a.area().abs()));
    let samples = wires.iter().map(|w| w.sample(48)).collect::<Vec<_>>();
    let contains = |i: usize, p: Point2| {
        let mut inside = false;
        for edge in samples[i].windows(2) {
            let (a, b) = (edge[0], edge[1]);
            if (a[1] > p[1]) != (b[1] > p[1])
                && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
            {
                inside = !inside;
            }
        }
        inside
    };
    let parents = wires
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let sample = &samples[i];
            // Uma borda compartilhada não faz da célula vizinha um furo. Um contorno
            // precisa estar inteiramente dentro de outro, com pelo menos um ponto estrito.
            (0..i).rev().find(|j| {
                sample.iter().all(|p| contains(*j, *p))
                    && sample
                        .iter()
                        .any(|p| samples[*j].iter().all(|q| distance(*p, *q) > EPS))
            })
        })
        .collect::<Vec<_>>();
    Ok((0..wires.len())
        .map(|outer| Region {
            outer: wires[outer].clone(),
            holes: (0..wires.len())
                .filter(|i| parents[*i] == Some(outer))
                .map(|i| wires[i].clone())
                .collect(),
        })
        .collect())
}

/// Identidade orientada e cíclica da célula, sem coordenadas e sem depender da área.
/// Mais de uma correspondência é erro, nunca uma escolha arbitrária no recálculo.
pub fn signature(sketch: &Sketch, region: &Region) -> Result<Vec<(u64, bool)>> {
    let mut result = Vec::new();
    for (entity, piece) in region.outer.entities.iter().zip(&region.outer.segments) {
        let source = profiles::segment(sketch, sketch.entity(*entity)?)?;
        let reversed = match (&source, piece) {
            (Segment::Line { a, b }, Segment::Line { a: c, b: d }) => {
                dot(sub(*b, *a), sub(*d, *c)) < 0.
            }
            (Segment::Arc { clockwise: a, .. }, Segment::Arc { clockwise: b, .. }) => a != b,
            (Segment::Circle { .. }, Segment::Arc { clockwise, .. }) => *clockwise,
            _ => false,
        };
        result.push((*entity, reversed));
    }
    if let Some(best) = (0..result.len()).min_by(|a, b| {
        (0..result.len())
            .map(|k| result[(a + k) % result.len()])
            .cmp((0..result.len()).map(|k| result[(b + k) % result.len()]))
    }) {
        result.rotate_left(best);
    }
    Ok(result)
}
pub fn find(sketch: &Sketch, regions: &[Region], boundary: &[(u64, bool)]) -> Result<usize> {
    let mut found = None;
    for (i, region) in regions.iter().enumerate() {
        if signature(sketch, region)? == boundary {
            if found.is_some() {
                return Err(SketchError(
                    "Área de perfil ambígua; selecione-a novamente.".into(),
                ));
            }
            found = Some(i);
        }
    }
    found.ok_or_else(|| {
        SketchError("A área fechada escolhida não existe mais; selecione-a novamente.".into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_disks_shared_edges_crossings_and_open_branches_are_selectable() -> Result<()> {
        let mut s = Sketch::new("Áreas", Plane::xy());
        s.add_rectangle([0., 0.], [20., 20.], false)?;
        let disk = s.add_circle([10., 10.], 3., false)?;
        let a = regions(&s)?;
        assert_eq!(a.len(), 2);
        assert!(
            a.iter()
                .any(|r| r.outer.entities == vec![disk] && r.holes.is_empty())
        );
        assert_eq!(a.iter().map(|r| r.holes.len()).sum::<usize>(), 1);
        s.add_line([10., 0.], [10., 20.], false)?;
        s.add_line([30., 0.], [40., 0.], false)?;
        let a = regions(&s)?;
        assert_eq!(a.len(), 4);
        assert!((a.iter().map(Region::area).sum::<f64>() - 400.).abs() < 1e-5);
        let keys = a
            .iter()
            .map(|r| signature(&s, r))
            .collect::<Result<Vec<_>>>()?;
        for key in keys {
            find(&s, &a, &key)?;
        }
        let mut shared = Sketch::new("Borda compartilhada", Plane::xy());
        shared.add_rectangle([0., 0.], [10., 10.], false)?;
        shared.add_rectangle([10., 0.], [20., 10.], false)?;
        assert_eq!(regions(&shared)?.len(), 2);
        let mut circles = Sketch::new("Lente", Plane::xy());
        circles.add_circle([0., 0.], 10., false)?;
        circles.add_circle([10., 0.], 10., false)?;
        let a = regions(&circles)?;
        assert_eq!(a.len(), 3);
        for region in &a {
            find(&circles, &a, &signature(&circles, region)?)?;
        }
        Ok(())
    }
}
