//! Seleção exata de arestas. A camada de documento fornece um par transitório
//! de faces; somente o núcleo CAD conhece a ordem real das arestas do B-rep.
use nator_document::{Document, DocumentError, assembly::Pick, features::EdgeRef};
#[cfg(feature = "occt")]
pub fn edge_at(doc: &Document, pick: &Pick) -> Result<EdgeRef, DocumentError> {
    let faces = doc.edge_faces_at(pick)?;
    let (solid, bytes) = crate::features::source(doc, pick.part, pick.body)?;
    let index = solid
        .closest_edge(faces, pick.placement.inverse_apply(pick.point))
        .map_err(|e| DocumentError(e.to_string()))?;
    Ok(match solid.edge_face_names(index) {
        Ok(names) if solid.edge_between(&names).is_ok_and(|found| found == index) => {
            EdgeRef::Between(names)
        }
        _ => EdgeRef::Index {
            index,
            signature: nator_document::features::signature(&bytes),
        },
    })
}
#[cfg(not(feature = "occt"))]
pub fn edge_at(doc: &Document, pick: &Pick) -> Result<EdgeRef, DocumentError> {
    doc.edge_at(pick)
}

/// Arestas que chegam ao vértice mais próximo do clique, dentro de `tolerance` mm. Vazio
/// quando o clique está longe de qualquer vértice: aí ele vale pela aresta sob o cursor.
#[cfg(feature = "occt")]
pub fn edges_at_vertex(
    doc: &Document,
    pick: &Pick,
    tolerance: f64,
) -> Result<Vec<EdgeRef>, DocumentError> {
    use nator_kernel::EdgeGeometry;
    let error = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let (solid, bytes) = crate::features::source(doc, pick.part, pick.body)?;
    let click = pick.placement.inverse_apply(pick.point);
    let mut ends = Vec::new();
    for index in 0..solid.edge_count().map_err(error)? {
        let pair = match solid.edge_geometry(index).map_err(error)? {
            EdgeGeometry::Line(a, b) => [a, b],
            EdgeGeometry::Circle {
                start, end, closed, ..
            } => {
                if closed {
                    continue;
                }
                [start, end]
            }
            EdgeGeometry::Curve(points) => match (points.first(), points.last()) {
                (Some(a), Some(b)) => [*a, *b],
                _ => continue,
            },
        };
        ends.push((index, pair));
    }
    let gap = |p: nator_geometry::Point3| nator_geometry::norm(nator_geometry::sub(p, click));
    let Some(vertex) = ends
        .iter()
        .flat_map(|(_, pair)| pair.iter().copied())
        .min_by(|a, b| gap(*a).total_cmp(&gap(*b)))
        .filter(|p| gap(*p) <= tolerance)
    else {
        return Ok(Vec::new());
    };
    let same =
        |p: nator_geometry::Point3| nator_geometry::norm(nator_geometry::sub(p, vertex)) < 1e-6;
    Ok(ends
        .iter()
        .filter(|(_, pair)| pair.iter().any(|p| same(*p)))
        .map(|(index, _)| match solid.edge_face_names(*index) {
            Ok(names)
                if solid
                    .edge_between(&names)
                    .is_ok_and(|found| found == *index) =>
            {
                EdgeRef::Between(names)
            }
            _ => EdgeRef::Index {
                index: *index,
                signature: nator_document::features::signature(&bytes),
            },
        })
        .collect())
}
#[cfg(not(feature = "occt"))]
pub fn edges_at_vertex(_: &Document, _: &Pick, _: f64) -> Result<Vec<EdgeRef>, DocumentError> {
    Ok(Vec::new())
}
