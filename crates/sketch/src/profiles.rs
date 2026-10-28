//! Descoberta de contornos independentes e diagnóstico; nunca fecha lacunas silenciosamente.
use crate::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
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
/// Contorno fechado. `entities[i]` é a entidade do rascunho que originou `segments[i]`;
/// essa correspondência dá nomes estáveis às faces laterais de extrusões e revoluções.
#[derive(Debug, Clone)]
pub struct Wire {
    pub segments: Vec<Segment>,
    pub entities: Vec<u64>,
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
        // Integral de Green por curva: áreas estreitas entre arcos não desaparecem
        // pela amostragem usada apenas no desenho e na captura do cursor.
        self.segments
            .iter()
            .map(|segment| match *segment {
                Segment::Line { a, b } => cross(a, b) * 0.5,
                Segment::Circle { radius, .. } => std::f64::consts::PI * radius * radius,
                Segment::Arc {
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
                    (cross(center, sub(b, a)) + dot(first, first) * sweep) * 0.5
                }
            })
            .sum()
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
    // Cada aresta do grafo guarda também a entidade de origem.
    let mut edges: Vec<(usize, usize, Segment, u64)> = Vec::new();
    let mut wires = Vec::new();
    for entity in sketch.entities.iter().filter(|e| !e.construction) {
        let curve = segment(sketch, entity)?;
        if let Segment::Circle { .. } = curve {
            wires.push(Wire {
                segments: vec![curve],
                entities: vec![entity.id],
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
            .any(|(x, y, _, _)| (*x == ai && *y == bi) || (*x == bi && *y == ai))
        {
            diagnostics.duplicate_entities.push(entity.id);
        }
        edges.push((ai, bi, curve, entity.id));
    }
    let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();
    for (id, (a, b, _, _)) in edges.iter().enumerate() {
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
        let mut sources = Vec::new();
        let mut closed = false;
        for _ in 0..=edges.len() {
            if visited.contains(&current) {
                break;
            }
            visited.insert(current);
            let (a, b, curve, entity) = &edges[current];
            let next = if node == *a { *b } else { *a };
            chain.push(if node == *a {
                curve.clone()
            } else {
                curve.reversed()
            });
            sources.push(*entity);
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
            let wire = Wire {
                segments: chain,
                entities: sources,
            };
            if wire.area().abs() > 1e-10 {
                wires.push(wire);
            }
        }
    }
    for (i, (_, _, first, _)) in edges.iter().enumerate() {
        let Segment::Line { a, b } = *first else {
            continue;
        };
        for (_, _, second, _) in &edges[i + 1..] {
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

/// Região preenchível de um rascunho: um contorno externo e os furos contidos diretamente nele.
/// Um contorno dentro de um furo forma uma nova região (ilha), como em uma arruela dentro de outra.
#[derive(Debug, Clone)]
pub struct Region {
    pub outer: Wire,
    pub holes: Vec<Wire>,
}
impl Region {
    /// Identidade da região: entidades do contorno externo, ordenadas e sem repetição.
    /// Diferente de um índice, ela não muda quando uma cota altera a área das regiões.
    pub fn boundary(&self) -> Vec<u64> {
        let mut ids = self.outer.entities.clone();
        ids.sort_unstable();
        ids.dedup();
        ids
    }
    /// Área líquida em mm², descontando os furos.
    pub fn area(&self) -> f64 {
        self.outer.area().abs() - self.holes.iter().map(|h| h.area().abs()).sum::<f64>()
    }
}

/// Regiões de um perfil limpo, ordenadas por área do contorno externo (maior primeiro).
/// Pontas abertas, cruzamentos e duplicações são rejeitados: nada é fechado implicitamente.
pub fn regions(sketch: &Sketch) -> Result<Vec<Region>> {
    let (mut wires, diagnostics) = analyze(sketch)?;
    if !diagnostics.open_points.is_empty()
        || !diagnostics.crossings.is_empty()
        || !diagnostics.duplicate_entities.is_empty()
    {
        return Err(SketchError(
            "Feche os contornos e remova cruzamentos/duplicações antes de criar o sólido.".into(),
        ));
    }
    // Com a ordem decrescente de área, o contorno que envolve outro sempre aparece antes dele.
    wires.sort_by(|a, b| b.area().abs().total_cmp(&a.area().abs()));
    let parents = wires
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let first = w.sample(64).first().copied().unwrap_or([0.; 2]);
            (0..i).rev().find(|p| wires[*p].contains(first))
        })
        .collect::<Vec<_>>();
    let depth = |mut i: usize| {
        let mut d = 0;
        while let Some(p) = parents[i] {
            d += 1;
            i = p;
        }
        d
    };
    Ok((0..wires.len())
        .filter(|i| depth(*i) % 2 == 0)
        .map(|outer| Region {
            outer: wires[outer].clone(),
            holes: (0..wires.len())
                .filter(|i| parents[*i] == Some(outer))
                .map(|i| wires[i].clone())
                .collect(),
        })
        .collect())
}

/// Localiza a região escolhida anteriormente pelas entidades de seu contorno externo.
///
/// A correspondência exata é preferida. Se o contorno foi refinado (um canto arredondado,
/// uma linha aparada), a região é a mesma quando ao menos metade das entidades originais
/// continua existindo e todas elas estão no contorno externo de uma única região: cada
/// entidade pertence a um só contorno, então não há escolha entre candidatas. Em qualquer
/// outro caso a referência é considerada perdida e o erro pede uma nova seleção.
pub fn find_region(regions: &[Region], boundary: &[u64]) -> Result<usize> {
    let mut wanted = boundary.to_vec();
    wanted.sort_unstable();
    wanted.dedup();
    if wanted.is_empty() {
        return Err(SketchError("Referência de região vazia.".into()));
    }
    if let Some(index) = regions.iter().position(|r| r.boundary() == wanted) {
        return Ok(index);
    }
    let mut owner = None;
    let mut survivors = 0;
    for id in &wanted {
        if regions
            .iter()
            .any(|r| r.holes.iter().any(|h| h.entities.contains(id)))
        {
            return Err(SketchError(
                "A região escolhida virou furo de outra região; selecione-a novamente.".into(),
            ));
        }
        if let Some(index) = regions.iter().position(|r| r.outer.entities.contains(id)) {
            survivors += 1;
            if owner.is_some_and(|o| o != index) {
                return Err(SketchError(
                    "O contorno da região foi dividido; selecione a região novamente.".into(),
                ));
            }
            owner = Some(index);
        }
    }
    match owner {
        Some(index) if survivors * 2 >= wanted.len() => Ok(index),
        _ => Err(SketchError(
            "A região escolhida não existe mais no rascunho; selecione-a novamente.".into(),
        )),
    }
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

    /// Mudar cotas troca a ordem por área, mas não a identidade das regiões.
    #[test]
    fn region_identity_survives_area_reordering() -> Result<()> {
        let mut sketch = Sketch::new("Duas placas", Plane::xy());
        let small = sketch.add_rectangle([0., 0.], [10., 10.], false)?;
        sketch.add_rectangle([200., 0.], [240., 20.], false)?;
        let before = regions(&sketch)?;
        let target = before
            .iter()
            .find(|r| r.boundary().contains(&small[0]))
            .ok_or_else(|| SketchError("Região pequena ausente".into()))?
            .boundary();
        assert_eq!(find_region(&before, &target)?, 1);
        // Aumentar a placa pequena a torna a maior: o índice muda, a identidade não.
        for point in &mut sketch.points {
            if point.position[0] == 10. {
                point.position[0] = 100.;
            }
            if point.position[1] == 10. {
                point.position[1] = 50.;
            }
        }
        let after = regions(&sketch)?;
        let index = find_region(&after, &target)?;
        assert_eq!(index, 0);
        assert!((after[index].area() - 5000.).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn holes_belong_to_their_region_and_lost_regions_are_reported() -> Result<()> {
        let mut sketch = Sketch::new("Placa com furo", Plane::xy());
        let plate = sketch.add_rectangle([0., 0.], [40., 20.], false)?;
        let hole = sketch.add_circle([10., 10.], 3., false)?;
        let found = regions(&sketch)?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].holes.len(), 1);
        // A área vem do polígono amostrado (128 lados no círculo): erro de ~0,011 mm².
        assert!((found[0].area() - (800. - std::f64::consts::PI * 9.)).abs() < 0.05);
        // Um furo não é uma região selecionável.
        assert!(find_region(&found, &[hole]).is_err());
        // Remover todas as entidades do contorno torna a referência perdida, sem substituição.
        sketch.entities.retain(|e| !plate.contains(&e.id));
        sketch.prune_unused_points();
        let only_circle = regions(&sketch)?;
        assert!(find_region(&only_circle, &plate).is_err());
        Ok(())
    }

    /// Arredondar um canto acrescenta um arco e muda as linhas, mas a região é a mesma.
    #[test]
    fn refined_boundary_keeps_the_region() -> Result<()> {
        let mut sketch = Sketch::new("Canto", Plane::xy());
        let lines = sketch.add_rectangle([0., 0.], [40., 20.], false)?;
        let target = regions(&sketch)?[0].boundary();
        let arc = crate::tools::refine_corner(&mut sketch, lines[0], lines[1], 3., true)?;
        let refined = regions(&sketch)?;
        assert!(refined[0].boundary().contains(&arc));
        assert_eq!(find_region(&refined, &target)?, 0);
        Ok(())
    }
}
