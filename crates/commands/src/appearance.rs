//! Preparação precisa da aparência fora da interface. Prismas do rascunho
//! recortam as faces sem mudar volume, topologia ou referências de modelagem.
use nator_document::{BodyId, Document, DocumentError, PartId, SurfaceAppearance, SurfacePaint};
#[cfg(feature = "occt")]
use std::sync::Arc;
#[cfg(feature = "occt")]
struct Prepared {
    solid: nator_kernel::Solid,
    bytes: Arc<Vec<u8>>,
    faces: Vec<usize>,
    tools: Vec<Option<nator_kernel::Solid>>,
    layers: Vec<SurfacePaint>,
}
#[cfg(feature = "occt")]
fn prepare(
    doc: &Document,
    body: BodyId,
    layers: Vec<SurfacePaint>,
) -> Result<Prepared, DocumentError> {
    use crate::features::{face_index, region, source};
    use nator_document::features::FaceRef;
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
        if let Some(name) = solid.face_names().get(face).filter(|n| {
            !n.is_empty() && solid.face_names().iter().filter(|v| *v == *n).count() == 1
        }) {
            layer.face = FaceRef::Named(name.clone());
        }
        faces.push(face);
        let make_mask =
            |mask: &nator_document::PaintMask| -> Result<nator_kernel::Solid, DocumentError> {
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
                mask_solid.ok_or_else(|| DocumentError("Delimitação de pintura vazia.".into()))
            };
        let mut tool = layer.mask.as_ref().map(make_mask).transpose()?;
        for boundary in &layer.boundaries {
            let region = make_mask(&boundary.mask)?;
            tool = Some(match tool {
                Some(previous) => previous
                    .boolean(&region, if boundary.inside { 2 } else { 1 }, "paint")
                    .map_err(error)?,
                None if boundary.inside => region,
                None => {
                    let mut min = [f64::INFINITY; 3];
                    let mut max = [f64::NEG_INFINITY; 3];
                    for x in [corners[0][0], corners[1][0]] {
                        for y in [corners[0][1], corners[1][1]] {
                            for z in [corners[0][2], corners[1][2]] {
                                let p = record.placement.apply([x, y, z]);
                                for i in 0..3 {
                                    min[i] = min[i].min(p[i]);
                                    max[i] = max[i].max(p[i]);
                                }
                            }
                        }
                    }
                    let margin = nator_geometry::norm(nator_geometry::sub(max, min)).max(1.);
                    let envelope = nator_kernel::Solid::cuboid(
                        min.map(|v| v - margin),
                        std::array::from_fn(|i| max[i] - min[i] + 2. * margin),
                    )
                    .map_err(error)?;
                    envelope.boolean(&region, 1, "paint").map_err(error)?
                }
            });
        }
        tools.push(tool);
        normalized.push(layer);
    }
    Ok(Prepared {
        solid,
        bytes,
        faces,
        tools,
        layers: normalized,
    })
}

/// Exibição e STEP usam as mesmas máscaras exatas e a mesma ordem de sobreposição.
/// Somente o STEP costura as subdivisões numa cópia fechada do sólido de origem.
#[cfg(feature = "occt")]
pub(super) fn step_body(
    doc: &Document,
    body: BodyId,
) -> Result<nator_kernel::StepBody, DocumentError> {
    let record = doc
        .body(body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let brep = record.brep.as_ref().ok_or_else(|| DocumentError(
        "O escopo contém corpos de malha. Use STL ou selecione apenas sólidos CAD para exportar STEP.".into(),
    ))?;
    let appearance = doc.body_appearance(body);
    let base_color = appearance.and_then(|a| a.base_color);
    let error = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let (solid, face_colors) = if let Some(appearance) = appearance.filter(|a| !a.layers.is_empty())
    {
        let prepared = prepare(doc, body, appearance.layers.clone())?;
        let input = prepared
            .faces
            .iter()
            .copied()
            .zip(prepared.tools.iter().map(Option::as_ref))
            .collect::<Vec<_>>();
        let (solid, materials) = prepared.solid.painted_solid(&input).map_err(error)?;
        let colors = materials
            .into_iter()
            .map(|material| {
                if material == 0 {
                    // A pintura uniforme tem precedência sobre cores de ocorrências.
                    base_color
                } else {
                    Some(prepared.layers[material as usize - 1].color)
                }
            })
            .collect();
        (solid, colors)
    } else {
        let solid = nator_kernel::Solid::from_brep(brep, None)
            .and_then(|s| s.transformed(record.placement))
            .map_err(error)?;
        let colors = base_color
            .map(|c| vec![Some(c); solid.face_names().len()])
            .unwrap_or_default();
        (solid, colors)
    };
    Ok(nator_kernel::StepBody {
        name: record.name.clone(),
        solid,
        color: base_color.or(record.color),
        face_colors,
    })
}

#[cfg(feature = "occt")]
pub(super) fn build(
    doc: &Document,
    body: BodyId,
    layers: Vec<SurfacePaint>,
) -> Result<SurfaceAppearance, DocumentError> {
    use nator_document::{MeshAsset, features::signature};
    let Prepared {
        solid,
        bytes,
        faces,
        tools,
        layers: normalized,
    } = prepare(doc, body, layers)?;
    let record = doc
        .body(body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let input = faces
        .into_iter()
        .zip(tools.iter().map(|s| s.as_ref()))
        .collect::<Vec<_>>();
    let (mesh, materials) = solid
        .surface_appearance(&input)
        .map_err(|e| DocumentError(e.to_string()))?;
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
        base_color: doc.body_appearance(body).and_then(|a| a.base_color),
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
        if layers.is_empty() {
            continue;
        }
        let appearance = build(doc, body, layers)?;
        doc.set_surface_appearance(appearance)?;
    }
    Ok(())
}

/// Reconhece corpos inteiramente pintados, inclusive arquivos que guardavam uma
/// camada por face. A cor uniforme acompanha também as faces novas de uma extrusão.
pub(super) fn uniform_color(doc: &Document, body: BodyId) -> Option<nator_geometry::RgbColor> {
    let record = doc.body(body)?;
    let Some(appearance) = doc.body_appearance(body) else {
        return record.color;
    };
    if appearance.geometry.is_none() {
        return appearance.base_color.or(record.color);
    }
    let color = |material: &u8| {
        if *material == 0 {
            appearance
                .base_color
                .or(record.color)
                .unwrap_or(nator_geometry::RgbColor::NEUTRAL)
        } else {
            appearance.layers[usize::from(*material) - 1].color
        }
    };
    let first = color(appearance.materials.first()?);
    appearance
        .materials
        .iter()
        .all(|m| color(m) == first)
        .then_some(first)
}
#[cfg(feature = "occt")]
pub(super) fn inherit(
    doc: &mut Document,
    part: PartId,
    input: BodyId,
    output: BodyId,
    operation: &nator_document::features::Operation,
) -> Result<(), DocumentError> {
    use crate::features::{face_index, source};
    use nator_document::features::{FaceRef, Operation, body_owner, signature};
    if let Some(color) = uniform_color(doc, input) {
        if doc.body_appearance(input).is_some() {
            crate::painting::set_body_color(doc, output, color)?;
        }
        return Ok(());
    }
    let Some(appearance) = doc.body_appearance(input).cloned() else {
        return Ok(());
    };
    let (original, bytes) = source(doc, part, input)?;
    let (result, result_bytes) = source(doc, part, output)?;
    let owner = format!("{}:", body_owner(output));
    let mut layers = Vec::new();
    let guide = crate::extrusion::guide(doc, part, operation).ok();
    for layer in appearance.layers {
        let original_face = face_index(&original, &bytes, &layer.face)?;
        let carry_new = match operation {
            Operation::ExtrudeFace { base, face, .. } if *base == input => {
                face_index(&original, &bytes, face)? == original_face
            }
            Operation::Extrude { .. } => guide.is_some_and(|g| {
                original
                    .face_measure(original_face)
                    .is_ok_and(|m| match m.kind {
                        nator_kernel::FaceKind::Plane { normal, offset } => {
                            let start = std::array::from_fn(|i| g.origin[i] + g.normal[i] * g.from);
                            nator_geometry::norm(nator_geometry::cross(normal, g.normal)) < 1e-7
                                && (nator_geometry::dot(normal, start) - offset).abs() < 1e-6
                        }
                        _ => false,
                    })
            }),
            _ => false,
        };
        for index in 0..result
            .face_count()
            .map_err(|e| DocumentError(e.to_string()))?
        {
            let name = &result.face_names()[index];
            let generated = carry_new && name.starts_with(&owner);
            let inherited = original
                .face_common_area(original_face, &result, index)
                .map_err(|e| DocumentError(e.to_string()))?
                > 1e-7;
            if !generated && !inherited {
                continue;
            }
            let mut candidate = layer.clone();
            candidate.face = if !name.is_empty()
                && result.face_names().iter().filter(|n| *n == name).count() == 1
            {
                FaceRef::Named(name.clone())
            } else {
                FaceRef::Index {
                    index,
                    signature: signature(&result_bytes),
                }
            };
            // Uma parte da face pode ter sido consumida. Delimitações que não
            // alcançam mais a superfície são descartadas, sem colorir outra região.
            match build(doc, output, vec![candidate.clone()]) {
                Ok(_) => layers.push(candidate),
                Err(error) if error.0.contains("A área do rascunho não alcança") => {}
                Err(error) => return Err(error),
            }
        }
    }
    if !layers.is_empty() {
        doc.set_surface_appearance(build(doc, output, layers)?)?;
    }
    Ok(())
}
#[cfg(not(feature = "occt"))]
pub(super) fn inherit(
    _: &mut Document,
    _: PartId,
    _: BodyId,
    _: BodyId,
    _: &nator_document::features::Operation,
) -> Result<(), DocumentError> {
    Ok(())
}
