//! Atração em unidades do plano, com prioridade para referências exatas e guias temporárias.
use crate::*;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Endpoint(u64),
    Midpoint(u64),
    CurveMidpoint(u64),
    RegionCenter,
    Center(u64),
    OnCurve(u64),
    Intersection,
    Projected,
    /// Ponto médio de uma aresta projetada.
    ProjectedMidpoint,
    /// Sobre uma aresta projetada.
    ProjectedEdge,
    /// Tangente a um círculo ou arco a partir do início do traço.
    Tangent,
    Alignment,
    /// Múltiplo da precisão do mouse: posiciona sem criar vínculos.
    Step,
}
#[derive(Debug, Clone)]
pub struct Snap {
    pub point: Point2,
    pub kind: Kind,
    pub guides: Vec<(Point2, Point2)>,
}
#[derive(Debug, Clone, Default)]
pub struct Tracker {
    references: Vec<Point2>,
}
impl Tracker {
    pub fn capture(&mut self, p: Point2) {
        if self.references.iter().any(|q| distance(*q, p) < 1e-7) {
            return;
        }
        self.references.push(p);
        if self.references.len() > 8 {
            self.references.remove(0);
        }
    }
    pub fn clear(&mut self) {
        self.references.clear();
    }
    pub fn references(&self) -> &[Point2] {
        &self.references
    }
}

/// Leva o cursor à precisão do mouse quando não há apoio geométrico. Uma guia de
/// alinhamento conserva o eixo alinhado e aplica o passo só ao eixo livre.
pub fn with_step(exact: Option<Snap>, cursor: Point2, step: f64) -> Result<Option<Snap>> {
    if cursor.iter().any(|v| !v.is_finite()) || !step.is_finite() || step <= 0. {
        return Err(SketchError(
            "Parâmetros da precisão do mouse inválidos.".into(),
        ));
    }
    let stepped = |point: Point2| point.map(|v| crate::drafting::snap_to_step(v, step));
    Ok(Some(match exact {
        Some(mut snap) if snap.kind == Kind::Alignment => {
            let mut point = stepped(snap.point);
            for (a, b) in &snap.guides {
                if (a[0] - b[0]).abs() < 1e-9 {
                    point[0] = snap.point[0];
                }
                if (a[1] - b[1]).abs() < 1e-9 {
                    point[1] = snap.point[1];
                }
            }
            snap.point = point;
            snap
        }
        Some(snap) => snap,
        None => Snap {
            point: stepped(cursor),
            kind: Kind::Step,
            guides: vec![],
        },
    }))
}

pub fn find(
    sketch: &Sketch,
    cursor: Point2,
    tolerance: f64,
    tracker: &Tracker,
    projected: &[Point2],
) -> Result<Option<Snap>> {
    let references = crate::topology::references(sketch)?;
    find_with_references(sketch, cursor, tolerance, tracker, projected, &references)
}

/// Consulta por quadro usando encontros/centros já calculados para esta geometria.
/// Vértice existente mais próximo dentro de `radius`. Com a precisão do mouse como raio,
/// um clique quase em cima de um vértice liga nele, em vez de criar uma falsa conexão
/// a poucos décimos de distância. Retas infinitas não têm extremos.
pub fn vertex_within(sketch: &Sketch, cursor: Point2, radius: f64) -> Result<Option<Snap>> {
    if !radius.is_finite() || radius <= 0. {
        return Err(SketchError("Raio de captura inválido.".into()));
    }
    let mut best: Option<(f64, Point2, Kind)> = None;
    for entity in sketch.entities.iter().filter(|e| !e.infinite) {
        let ids = match entity.curve {
            Curve::Line { start, end } => {
                vec![(start, Kind::Endpoint(start)), (end, Kind::Endpoint(end))]
            }
            Curve::Arc {
                center, start, end, ..
            } => vec![
                (start, Kind::Endpoint(start)),
                (end, Kind::Endpoint(end)),
                (center, Kind::Center(center)),
            ],
            Curve::Circle { center, .. } => vec![(center, Kind::Center(center))],
        };
        for (id, kind) in ids {
            let point = sketch.point(id)?;
            let gap = distance(point, cursor);
            if gap <= radius && best.is_none_or(|(old, ..)| gap < old) {
                best = Some((gap, point, kind));
            }
        }
    }
    Ok(best.map(|(_, point, kind)| Snap {
        point,
        kind,
        guides: Vec::new(),
    }))
}

/// Arestas do modelo projetadas no plano do rascunho. Curvas inclinadas chegam como
/// cadeias de retas curtas, separadas para não oferecer um meio a cada trecho.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProjectedEdges<'a> {
    pub curves: &'a [profiles::Segment],
    pub chains: &'a [Vec<Point2>],
}

fn closest_on_chain(chain: &[Point2], cursor: Point2) -> Option<Point2> {
    chain
        .windows(2)
        .map(|pair| {
            let (a, b) = (pair[0], pair[1]);
            let axis = sub(b, a);
            let length = dot(axis, axis);
            if length < 1e-18 {
                return a;
            }
            let t = (dot(sub(cursor, a), axis) / length).clamp(0., 1.);
            add(a, scale(axis, t))
        })
        .min_by(|p, q| distance(*p, cursor).total_cmp(&distance(*q, cursor)))
}

/// Pontos de um círculo em que a reta vinda de `anchor` o toca. Dentro dele não há tangente.
fn circle_tangents(anchor: Point2, center: Point2, radius: f64) -> Vec<Point2> {
    let gap = distance(anchor, center);
    if gap <= radius + 1e-9 {
        return Vec::new();
    }
    let turn = (radius / gap).acos();
    let base = sub(anchor, center);
    let base = base[1].atan2(base[0]);
    [base + turn, base - turn]
        .map(|angle| add(center, [radius * angle.cos(), radius * angle.sin()]))
        .to_vec()
}

fn segment_tangents(anchor: Point2, curve: &profiles::Segment) -> Vec<Point2> {
    match *curve {
        profiles::Segment::Circle { center, radius } => circle_tangents(anchor, center, radius),
        profiles::Segment::Arc { center, a, .. } => {
            let mut points = circle_tangents(anchor, center, distance(a, center));
            points.retain(|p| crate::projection::distance_to(curve, *p) < 1e-7);
            points
        }
        profiles::Segment::Line { .. } => Vec::new(),
    }
}

/// Silhueta de uma elipse ou outra curva em cadeia, vista de `anchor`: vértices cujos
/// dois vizinhos ficam do mesmo lado da reta que passa por ele.
fn chain_tangents(anchor: Point2, chain: &[Point2]) -> Vec<Point2> {
    let closed = chain.len() > 3 && distance(chain[0], chain[chain.len() - 1]) < 1e-7;
    let count = if closed { chain.len() - 1 } else { chain.len() };
    (0..count)
        .filter(|i| closed || (*i > 0 && *i + 1 < count))
        .filter_map(|i| {
            let (before, here, after) = (
                chain[(i + count - 1) % count],
                chain[i],
                chain[(i + 1) % count],
            );
            let ray = sub(here, anchor);
            let side = |p: Point2| {
                let d = sub(p, anchor);
                ray[0] * d[1] - ray[1] * d[0]
            };
            (side(before) * side(after) > 0.).then_some(here)
        })
        .collect()
}

/// Tangente mais próxima do cursor às curvas do rascunho e às arestas projetadas, a
/// partir do início do traço. Só posiciona: projeções não recebem restrição.
pub fn tangent(
    sketch: &Sketch,
    anchor: Point2,
    edges: ProjectedEdges<'_>,
    cursor: Point2,
    tolerance: f64,
) -> Result<Option<Snap>> {
    let mut points = Vec::new();
    for entity in sketch.entities.iter().filter(|e| !e.infinite) {
        let curve = profiles::segment(sketch, entity)?;
        points.extend(segment_tangents(anchor, &curve));
    }
    for curve in edges.curves {
        points.extend(segment_tangents(anchor, curve));
    }
    for chain in edges.chains {
        points.extend(chain_tangents(anchor, chain));
    }
    Ok(points
        .into_iter()
        .map(|p| (distance(p, cursor), p))
        .filter(|(gap, _)| *gap <= tolerance)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, point)| Snap {
            point,
            kind: Kind::Tangent,
            guides: vec![(anchor, point)],
        }))
}

pub fn find_with_references(
    sketch: &Sketch,
    cursor: Point2,
    tolerance: f64,
    tracker: &Tracker,
    projected: &[Point2],
    references: &crate::topology::References,
) -> Result<Option<Snap>> {
    find_with_edges(
        sketch,
        cursor,
        tolerance,
        tracker,
        projected,
        ProjectedEdges::default(),
        references,
    )
}

pub fn find_with_edges(
    sketch: &Sketch,
    cursor: Point2,
    tolerance: f64,
    tracker: &Tracker,
    projected: &[Point2],
    edges: ProjectedEdges<'_>,
    references: &crate::topology::References,
) -> Result<Option<Snap>> {
    if !tolerance.is_finite() || tolerance <= 0. {
        return Err(SketchError("Tolerância de captura inválida.".into()));
    }
    let mut candidates = Vec::new();
    for entity in &sketch.entities {
        match entity.curve {
            Curve::Line { start, end } => {
                let (a, b) = (sketch.point(start)?, sketch.point(end)?);
                if !entity.infinite {
                    candidates.extend([
                        (a, Kind::Endpoint(start), 0),
                        (b, Kind::Endpoint(end), 0),
                        (scale(add(a, b), 0.5), Kind::Midpoint(entity.id), 1),
                    ]);
                }
                candidates.push((
                    lines::closest(sketch, entity, cursor)?,
                    Kind::OnCurve(entity.id),
                    3,
                ));
            }
            Curve::Circle { center, radius } => {
                let c = sketch.point(center)?;
                candidates.push((c, Kind::Center(center), 0));
                let d = distance(c, cursor);
                if d > 1e-10 {
                    candidates.push((
                        add(c, scale(sub(cursor, c), radius / d)),
                        Kind::OnCurve(entity.id),
                        3,
                    ));
                }
            }
            Curve::Arc {
                center, start, end, ..
            } => {
                candidates.extend([
                    (sketch.point(center)?, Kind::Center(center), 0),
                    (sketch.point(start)?, Kind::Endpoint(start), 0),
                    (sketch.point(end)?, Kind::Endpoint(end), 0),
                ]);
                let curve = profiles::segment(sketch, entity)?;
                candidates.push((
                    crate::curve_geometry::point(&curve, 0.5),
                    Kind::CurveMidpoint(entity.id),
                    1,
                ));
                let t = crate::curve_geometry::closest_parameter(&curve, cursor);
                candidates.push((
                    crate::curve_geometry::point(&curve, t),
                    Kind::OnCurve(entity.id),
                    3,
                ));
            }
        }
    }
    candidates.extend(
        references
            .vertices
            .iter()
            .map(|p| (*p, Kind::Intersection, 0)),
    );
    // Um cruzamento local tem precedência sobre a origem/apoios externos no
    // mesmo lugar; assim a intenção fraca do cruzamento não vira uma fixação.
    for p in projected {
        candidates.push((*p, Kind::Projected, 0));
    }
    for curve in edges.curves {
        if let profiles::Segment::Line { a, b } = *curve {
            candidates.push((scale(add(a, b), 0.5), Kind::ProjectedMidpoint, 1));
        }
        let t = crate::curve_geometry::closest_parameter(curve, cursor);
        candidates.push((
            crate::curve_geometry::point(curve, t),
            Kind::ProjectedEdge,
            3,
        ));
    }
    for chain in edges.chains {
        candidates.extend(closest_on_chain(chain, cursor).map(|p| (p, Kind::ProjectedEdge, 3)));
    }
    candidates.extend(
        references
            .centers
            .iter()
            .map(|p| (*p, Kind::RegionCenter, 0)),
    );
    candidates.retain(|(p, _, _)| distance(*p, cursor) <= tolerance);
    candidates.sort_by(|(a, _, pa), (b, _, pb)| {
        pa.cmp(pb)
            .then_with(|| distance(*a, cursor).total_cmp(&distance(*b, cursor)))
    });
    if let Some((point, kind, _)) = candidates.first().copied() {
        return Ok(Some(Snap {
            point,
            kind,
            guides: Vec::new(),
        }));
    }
    let x = tracker
        .references
        .iter()
        .rev()
        .find(|p| (p[0] - cursor[0]).abs() < tolerance);
    let y = tracker
        .references
        .iter()
        .rev()
        .find(|p| (p[1] - cursor[1]).abs() < tolerance);
    if x.is_none() && y.is_none() {
        return Ok(None);
    }
    let point = [x.map_or(cursor[0], |p| p[0]), y.map_or(cursor[1], |p| p[1])];
    let guides = x.into_iter().chain(y).map(|p| (*p, point)).collect();
    Ok(Some(Snap {
        point,
        kind: Kind::Alignment,
        guides,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn midpoint_and_multiple_reference_alignment_are_exact() -> Result<()> {
        let mut s = Sketch::new("Ensaio", Plane::xy());
        let id = s.add_line([0., 0.], [20., 0.], false)?;
        let snap = find(&s, [10.1, 0.2], 0.5, &Tracker::default(), &[])?
            .ok_or_else(|| SketchError("Captura ausente".into()))?;
        assert_eq!(snap.kind, Kind::Midpoint(id));
        assert_eq!(snap.point, [10., 0.]);
        let mut tracker = Tracker::default();
        tracker.capture([30., 50.]);
        tracker.capture([10., 40.]);
        let snap = find(&s, [30.1, 40.2], 0.5, &tracker, &[])?
            .ok_or_else(|| SketchError("Guia ausente".into()))?;
        assert_eq!(snap.point, [30., 40.]);
        assert_eq!(snap.guides.len(), 2);
        Ok(())
    }

    #[test]
    fn arc_midpoint_is_on_the_curve_and_has_no_line_midpoint_constraint() -> Result<()> {
        let mut sketch = Sketch::new("Metade do arco", Plane::xy());
        let center = sketch.add_point([0., 0.])?;
        let start = sketch.add_point([0., 10.])?;
        let end = sketch.add_point([0., -10.])?;
        let arc = sketch.add_curve(
            Curve::Arc {
                center,
                start,
                end,
                clockwise: true,
            },
            false,
        )?;
        let snap = find(&sketch, [10.1, 0.1], 0.5, &Tracker::default(), &[])?
            .ok_or_else(|| SketchError("Ponto médio ausente".into()))?;
        assert_eq!(snap.kind, Kind::CurveMidpoint(arc));
        assert!(distance(snap.point, [10., 0.]) < 1e-9);
        Ok(())
    }
}

#[cfg(test)]
mod step_tests {
    use super::*;
    #[test]
    fn step_rounds_free_cursors_and_never_moves_a_geometric_support()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let snap = with_step(None, [10.126, -4.984], 0.01)?.ok_or("Passo")?;
        assert_eq!(snap.kind, Kind::Step);
        assert_eq!(snap.point, [10.13, -4.98]);
        assert!(!crate::junctions::links(Some(Kind::Step)));
        let guide = Snap {
            point: [20.1, 5.5],
            kind: Kind::Alignment,
            guides: vec![([0., 5.5], [20.1, 5.5])],
        };
        let merged = with_step(Some(guide), [20.1, 5.6], 10.)?.ok_or("Guia")?;
        assert_eq!(merged.point, [20., 5.5]);
        let exact = Snap {
            point: [20.1, 5.1],
            kind: Kind::OnCurve(1),
            guides: vec![],
        };
        assert_eq!(
            with_step(Some(exact), [20.1, 5.1], 10.)?
                .ok_or("Geometria")?
                .point,
            [20.1, 5.1]
        );
        assert!(with_step(None, [0., 0.], 0.).is_err());
        assert!(with_step(None, [f64::NAN, 0.], 10.).is_err());
        Ok(())
    }
}

#[cfg(test)]
mod vertex_radius {
    use super::*;

    #[test]
    fn a_vertex_inside_the_precision_radius_wins_and_infinite_lines_have_none() -> Result<()> {
        let mut sketch = Sketch::new("Raio", Plane::xy());
        let line = sketch.add_line([0., 0.], [10., 0.], false)?;
        let [start, end] = sketch.entity(line)?.point_ids()[..] else {
            return Err(SketchError("Linha sem extremos.".into()));
        };
        let near = vertex_within(&sketch, [0.3, 0.1], 0.4)?;
        assert!(matches!(near.map(|s| s.kind), Some(Kind::Endpoint(id)) if id == start));
        let near = vertex_within(&sketch, [9.8, 0.], 0.4)?;
        assert!(matches!(near.map(|s| s.kind), Some(Kind::Endpoint(id)) if id == end));
        assert!(vertex_within(&sketch, [0.5, 0.], 0.4)?.is_none());
        assert!(vertex_within(&sketch, [0., 0.], 0.).is_err());
        let mut infinite = Sketch::new("Reta", Plane::xy());
        infinite.add_line([0., 0.], [10., 0.], false)?;
        infinite.entities[0].infinite = true;
        assert!(vertex_within(&infinite, [0.1, 0.], 0.4)?.is_none());
        Ok(())
    }
}

#[cfg(test)]
mod projected_edges {
    use super::*;
    use crate::profiles::Segment;

    fn empty() -> Sketch {
        Sketch::new("Vazio", Plane::xy())
    }
    fn snap(edges: ProjectedEdges<'_>, cursor: Point2) -> Result<Option<Snap>> {
        find_with_edges(
            &empty(),
            cursor,
            0.5,
            &Tracker::default(),
            &[],
            edges,
            &topology::References::default(),
        )
    }

    #[test]
    fn projected_edges_offer_their_midpoint_and_every_point_along_them() -> Result<()> {
        let curves = [Segment::Line {
            a: [0., 0.],
            b: [10., 0.],
        }];
        let edges = ProjectedEdges {
            curves: &curves,
            chains: &[],
        };
        let middle = snap(edges, [5.2, 0.2])?
            .ok_or("Sem captura")
            .map_err(|e| SketchError(e.into()))?;
        assert_eq!(middle.kind, Kind::ProjectedMidpoint);
        assert!(distance(middle.point, [5., 0.]) < 1e-9);
        let along = snap(edges, [8., 0.3])?
            .ok_or("Sem captura")
            .map_err(|e| SketchError(e.into()))?;
        assert_eq!(along.kind, Kind::ProjectedEdge);
        assert!(distance(along.point, [8., 0.]) < 1e-9);
        assert!(snap(edges, [8., 3.])?.is_none());
        Ok(())
    }

    #[test]
    fn tangents_reach_sketch_circles_arcs_and_projected_ellipses() -> Result<()> {
        // De (0, 0) a um círculo de raio 3 em (5, 0): tangentes em x = 3,2.
        let mut sketch = empty();
        sketch.add_circle([5., 0.], 3., false)?;
        let hit = tangent(&sketch, [0., 0.], ProjectedEdges::default(), [3.3, 2.3], 1.)?
            .ok_or("Sem tangente")
            .map_err(|e| SketchError(e.into()))?;
        assert_eq!(hit.kind, Kind::Tangent);
        assert!(distance(hit.point, [3.2, 2.4]) < 1e-9);
        assert!(tangent(&sketch, [5., 1.], ProjectedEdges::default(), [5., 3.], 1.)?.is_none());

        // Meia-volta superior: só a tangente de cima pertence ao arco.
        let arc = [Segment::Arc {
            center: [5., 0.],
            a: [8., 0.],
            b: [2., 0.],
            clockwise: false,
        }];
        let edges = ProjectedEdges {
            curves: &arc,
            chains: &[],
        };
        assert!(tangent(&empty(), [0., 0.], edges, [3.2, 2.4], 0.1)?.is_some());
        assert!(tangent(&empty(), [0., 0.], edges, [3.2, -2.4], 0.1)?.is_none());

        // Elipse amostrada: o ponto mais alto, visto de muito longe no eixo.
        let ellipse = (0..=720)
            .map(|i| {
                let t = f64::from(i) * std::f64::consts::PI / 360.;
                [10. * t.cos(), 4. * t.sin()]
            })
            .collect::<Vec<_>>();
        let chains = [ellipse];
        let edges = ProjectedEdges {
            curves: &[],
            chains: &chains,
        };
        let top = tangent(&empty(), [-1e6, 4.], edges, [0., 4.2], 1.)?
            .ok_or("Sem tangente")
            .map_err(|e| SketchError(e.into()))?;
        assert!((top.point[1] - 4.).abs() < 1e-3);
        Ok(())
    }
}
