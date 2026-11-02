//! Consultas de vértices para a seleção e as ferramentas de canto.
use super::*;
use nator_sketch::junctions;

pub(super) fn vertex(
    sketch: &Sketch,
    virtual_vertices: &[Point2],
    cursor: Point2,
    tolerance: f64,
) -> Option<Point2> {
    sketch
        .visible_points()
        .map(|p| p.position)
        .chain(virtual_vertices.iter().copied())
        .filter(|p| nator_sketch::distance(*p, cursor) <= tolerance)
        .min_by(|a, b| {
            nator_sketch::distance(*a, cursor).total_cmp(&nator_sketch::distance(*b, cursor))
        })
}

/// Linhas que terminam no vértice, na ordem em que aparecem no rascunho.
pub(super) fn corner_lines(sketch: &Sketch, position: Point2) -> Vec<u64> {
    let Ok(Some(junction)) = junctions::at(sketch, position) else {
        return Vec::new();
    };
    junction
        .curves()
        .iter()
        .copied()
        .filter(|id| {
            sketch.line(*id).is_ok_and(|(a, b)| {
                nator_sketch::distance(a, position) < 1e-6
                    || nator_sketch::distance(b, position) < 1e-6
            })
        })
        .collect()
}
