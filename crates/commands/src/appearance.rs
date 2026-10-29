//! Preparação precisa da aparência fora da interface. Prismas do rascunho
//! recortam as faces sem mudar volume, topologia ou referências de modelagem.
use nator_document::{BodyId, Document, DocumentError, PartId, SurfaceAppearance, SurfacePaint};
#[cfg(feature = "occt")]
use std::sync::Arc;
#[cfg(feature = "occt")]
pub(super) fn build(
    doc: &Document,
    body: BodyId,
    layers: Vec<SurfacePaint>,
) -> Result<SurfaceAppearance, DocumentError> {
    use crate::features::{face_index, region, source};
    use nator_document::{
        MeshAsset,
        features::{FaceRef, signature},
    };
    let error = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let part = doc
        .parts()
        .iter()
        .find(|p| p.bodies.iter().any(|b| b.id == body))
        .ok_or_else(|| DocumentError("Peça da pintura inexistente.".into()))?;
    let record = doc
        .body(body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let (solid, bytes) = source(doc, part.id, body)?;
    // O prisma atravessa os oito cantos do corpo na direção normal do rascunho.
    // As distâncias são assinadas, em mm, relativamente à origem do plano; a
    // margem evita coincidência das tampas com a face que será recortada.
    let bounds = record.geometry.mesh().bounds();
    let corners = [bounds.min, bounds.max];
    let mut tools = Vec::new();
    let mut normalized = Vec::new();
    let mut faces = Vec::new();
    for mut layer in layers {
        let face = face_index(&solid, &bytes, &layer.face)?;
        if let Some(name) = solid.face_names().get(face).filter(|n| !n.is_empty()) {
            layer.face = FaceRef::Named(name.clone());
        }
        faces.push(face);
        tools.push(if let Some(mask) = &layer.mask {
            let sketch = &part
                .sketches
                .iter()
                .find(|s| s.id == mask.sketch)
                .ok_or_else(|| DocumentError("Rascunho da pintura inexistente.".into()))?
                .sketch;
            let mut from = f64::INFINITY;
            let mut to = f64::NEG_INFINITY;
            for x in [corners[0][0], corners[1][0]] {
                for y in [corners[0][1], corners[1][1]] {
                    for z in [corners[0][2], corners[1][2]] {
                        let local = record.placement.apply([x, y, z]);
                        let distance = nator_geometry::dot(
                            nator_geometry::sub(local, sketch.plane.origin),
                            sketch.plane.normal(),
                        );
                        from = from.min(distance);
                        to = to.max(distance);
                    }
                }
            }
            let margin = (to - from).abs().max(1.);
            let mut mask_solid: Option<nator_kernel::Solid> = None;
            for reference in mask.region.selections() {
                let prism = nator_kernel::Solid::extrude(
                    sketch,
                    &region(sketch, reference)?,
                    from - margin,
                    to + margin,
                    "paint",
                )
                .map_err(error)?;
                mask_solid = Some(match mask_solid {
                    Some(previous) => previous.boolean(&prism, 0, "paint").map_err(error)?,
                    None => prism,
                });
            }
            mask_solid
        } else {
            None
        });
        normalized.push(layer);
    }
    let input = faces
        .into_iter()
        .zip(tools.iter().map(|s| s.as_ref()))
        .collect::<Vec<_>>();
    let (mesh, materials) = solid.surface_appearance(&input).map_err(error)?;
    // O kernel operou em coordenadas da peça. O cache volta ao referencial local
    // do corpo para receber seu posicionamento exatamente uma vez na cena.
    let vertices = mesh
        .vertices()
        .iter()
        .map(|p| record.placement.inverse_apply(*p))
        .collect();
    let mesh = nator_geometry::Mesh::new(vertices, mesh.triangles().to_vec())
        .map_err(|e| DocumentError(e.to_string()))?;
    Ok(SurfaceAppearance {
        body,
        layers: normalized,
        source_signature: signature(&bytes),
        geometry: Some(Arc::new(MeshAsset::new(mesh))),
        materials: Arc::new(materials.into_iter().map(|m| m as u8).collect()),
    })
}
#[cfg(not(feature = "occt"))]
pub(super) fn build(
    _: &Document,
    _: BodyId,
    _: Vec<SurfacePaint>,
) -> Result<SurfaceAppearance, DocumentError> {
    Err(DocumentError(
        "Pintar superfícies exige o núcleo OCCT.".into(),
    ))
}
pub(super) fn rebuild(doc: &mut Document, part: PartId) -> Result<(), DocumentError> {
    let inputs = doc
        .appearances()
        .iter()
        .filter(|a| {
            doc.part(part)
                .is_some_and(|p| p.bodies.iter().any(|b| b.id == a.body))
        })
        .map(|a| (a.body, a.layers.clone()))
        .collect::<Vec<_>>();
    for (body, layers) in inputs {
        let appearance = build(doc, body, layers)?;
        doc.set_surface_appearance(appearance)?;
    }
    Ok(())
}
