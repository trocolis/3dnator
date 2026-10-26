//! Descoberta de contornos independentes e diagnóstico; nunca fecha lacunas silenciosamente.
use crate::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum Segment {
    Line {
        a: Point2,
        b: Point2,
    },
    Circle {
        center: Point2,
        radius: f64,
    },
    Arc {
        center: Point2,
        a: Point2,
        b: Point2,
        clockwise: bool,
    },
}
impl Segment {
    pub fn reversed(&self) -> Self {
        match *self {
            Self::Line { a, b } => Self::Line { a: b, b: a },
            Self::Circle { center, radius } => Self::Circle { center, radius },
            Self::Arc {
                center,
                a,
                b,
                clockwise,
            } => Self::Arc {
                center,
                a: b,
                b: a,
                clockwise: !clockwise,
            },
        }
    }
    pub fn sample(&self, steps: usize) -> Vec<Point2> {
        match *self {
            Self::Line { a, b } => vec![a, b],
            Self::Circle { center, radius } => (0..=steps.max(16))
                .map(|i| {
                    let angle = std::f64::consts::TAU * i as f64 / steps.max(16) as f64;
                    add(center, [radius * angle.cos(), radius * angle.sin()])
                })
                .collect(),
            Self::Arc {
                center,
                a,
                b,
                clockwise,
            } => {
                let first = sub(a, center);
                let last = sub(b, center);
                let start = first[1].atan2(first[0]);
                let end = last[1].atan2(last[0]);
                let sweep = if clockwise {
                    -((start - end).rem_euclid(std::f64::consts::TAU))
                } else {
                    (end - start).rem_euclid(std::f64::consts::TAU)
                };
                let radius = distance(a, center);
                let count =
                    ((sweep.abs() / std::f64::consts::TAU * steps as f64).ceil() as usize).max(2);
                (0..=count)
                    .map(|i| {
                        let angle = start + sweep * i as f64 / count as f64;
                        add(center, [radius * angle.cos(), radius * angle.sin()])
                    })
                    .collect()
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct Wire {
    pub segments: Vec<Segment>,
}
impl Wire {
    pub fn sample(&self, steps: usize) -> Vec<Point2> {
        let mut result = Vec::new();
        for segment in &self.segments {
            let mut points = segment.sample(steps);
            if !result.is_empty() {
                points.remove(0);
            }
            result.extend(points);
        }
        result
    }
    pub fn area(&self) -> f64 {
        let points = self.sample(128);
        points.windows(2).map(|p| cross(p[0], p[1]) * 0.5).sum()
    }
    pub fn contains(&self, p: Point2) -> bool {
        let points = self.sample(128);
        let mut inside = false;
        for edge in points.windows(2) {
            let (a, b) = (edge[0], edge[1]);
            if (a[1] > p[1]) != (b[1] > p[1])
                && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
            {
                inside = !inside;
            }
        }
        inside
    }
}
#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    pub open_points: Vec<Point2>,
    pub crossings: Vec<Point2>,
    pub duplicate_entities: Vec<u64>,
}

pub fn segment(sketch: &Sketch, entity: &Entity) -> Result<Segment> {
    Ok(match entity.curve {
        Curve::Line { start, end } => Segment::Line {
            a: sketch.point(start)?,
            b: sketch.point(end)?,
        },
        Curve::Circle { center, radius } => Segment::Circle {
            center: sketch.point(center)?,
            radius,
        },
        Curve::Arc {
            center,
            start,
            end,
            clockwise,
        } => Segment::Arc {
            center: sketch.point(center)?,
            a: sketch.point(start)?,
            b: sketch.point(end)?,
            clockwise,
        },
    })
}

pub fn line_intersection(a: Point2, b: Point2, c: Point2, d: Point2) -> Option<(Point2, f64, f64)> {
    let ab = sub(b, a);
    let cd = sub(d, c);
    let divisor = cross(ab, cd);
    if divisor.abs() < 1e-12 {
        return None;
    }
    let ac = sub(c, a);
    let t = cross(ac, cd) / divisor;
    let u = cross(ac, ab) / divisor;
    Some((add(a, scale(ab, t)), t, u))
}

pub fn analyze(sketch: &Sketch) -> Result<(Vec<Wire>, Diagnostics)> {
    sketch.validate()?;
    let mut diagnostics = Diagnostics::default();
    let mut nodes: Vec<Point2> = Vec::new();
    let mut edges: Vec<(usize, usize, Segment)> = Vec::new();
    let mut wires = Vec::new();
    for entity in sketch.entities.iter().filter(|e| !e.construction) {
        let curve = segment(sketch, entity)?;
        if let Segment::Circle { .. } = curve {
            wires.push(Wire {
                segments: vec![curve],
            });
            continue;
        }
        let (a, b) = match curve {
            Segment::Line { a, b } | Segment::Arc { a, b, .. } => (a, b),
            Segment::Circle { .. } => continue,
        };
        let mut find = |p: Point2| {
            if let Some(index) = nodes.iter().position(|q| distance(*q, p) < 1e-6) {
                index
            } else {
                nodes.push(p);
                nodes.len() - 1
            }
        };
        let ai = find(a);
        let bi = find(b);
        if edges
            .iter()
            .any(|(x, y, _)| (*x == ai && *y == bi) || (*x == bi && *y == ai))
        {
            diagnostics.duplicate_entities.push(entity.id);
        }
        edges.push((ai, bi, curve));
    }
    let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();
    for (id, (a, b, _)) in edges.iter().enumerate() {
        adjacency.entry(*a).or_default().push(id);
        adjacency.entry(*b).or_default().push(id);
    }
    diagnostics.open_points = adjacency
        .iter()
        .filter(|(_, edges)| edges.len() != 2)
        .map(|(node, _)| nodes[*node])
        .collect();
    let mut visited = HashSet::new();
    for first in 0..edges.len() {
        if visited.contains(&first) {
            continue;
        }
        let start = edges[first].0;
        let mut node = start;
        let mut current = first;
        let mut chain = Vec::new();
        let mut closed = false;
        for _ in 0..=edges.len() {
            if visited.contains(&current) {
                break;
            }
            visited.insert(current);
            let (a, b, curve) = &edges[current];
            let next = if node == *a { *b } else { *a };
            chain.push(if node == *a {
                curve.clone()
            } else {
                curve.reversed()
            });
            node = next;
            if node == start {
                closed = true;
                break;
            }
            let Some(candidates) = adjacency
                .get(&node)
                .filter(|neighbors| neighbors.len() == 2)
            else {
                break;
            };
            let Some(next) = candidates.iter().copied().find(|id| !visited.contains(id)) else {
                break;
            };
            current = next;
        }
        if closed {
            let wire = Wire { segments: chain };
            if wire.area().abs() > 1e-10 {
                wires.push(wire);
            }
        }
    }
    for (i, (_, _, first)) in edges.iter().enumerate() {
        let Segment::Line { a, b } = *first else {
            continue;
        };
        for (_, _, second) in &edges[i + 1..] {
            let Segment::Line { a: c, b: d } = *second else {
                continue;
            };
            if let Some((p, t, u)) = line_intersection(a, b, c, d)
                .filter(|(_, t, u)| *t > 1e-7 && *t < 1. - 1e-7 && *u > 1e-7 && *u < 1. - 1e-7)
            {
                let _ = (t, u);
                diagnostics.crossings.push(p);
            }
        }
    }
    Ok((wires, diagnostics))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_rectangle_and_circle_are_found_without_closing_a_gap() -> Result<()> {
        let mut sketch = Sketch::new("Perfil", Plane::xy());
        sketch.add_rectangle([0., 0.], [60., 20.], false)?;
        sketch.add_circle([10., 10.], 3., false)?;
        let (wires, d) = analyze(&sketch)?;
        assert_eq!(wires.len(), 2);
        assert!(d.open_points.is_empty());
        assert!(wires.iter().any(|w| (w.area().abs() - 1200.).abs() < 1e-6));
        sketch.entities.remove(1);
        sketch.prune_unused_points();
        let (wires, d) = analyze(&sketch)?;
        assert_eq!(wires.len(), 1);
        assert_eq!(d.open_points.len(), 2);
        Ok(())
    }
}
