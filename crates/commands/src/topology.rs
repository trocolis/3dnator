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
