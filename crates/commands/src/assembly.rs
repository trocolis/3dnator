//! Construção de versões espelhadas da montagem, sem dependências com as peças originais.
use super::add_feature;
use nator_document::{Document, DocumentError, InstanceId, MeshAsset, PartId, features::Operation};
use nator_geometry::{Placement, Point3, norm};
use std::{collections::HashSet, sync::Arc};

/// A raiz conserva sua posição e recebe a reflexão em coordenadas locais, compatível com
/// o histórico existente. Cada definição descendente é refletida uma única vez em X = 0;
/// a posição de cada ocorrência compensa essa escolha. Assim, componentes repetidos e
/// rotacionados continuam compartilhando a mesma definição dentro do clone independente.
pub(super) fn mirror_instance(
    doc: &mut Document,
    id: InstanceId,
    origin: Point3,
    normal: Point3,
    name: String,
) -> Result<(), DocumentError> {
    let source = doc
        .instance(id)
        .ok_or_else(|| DocumentError("Instância inexistente.".into()))?
        .clone();
    let length = norm(normal);
    if !length.is_finite() || length < 1e-9 || origin.iter().any(|v| !v.is_finite()) {
        return Err(DocumentError("Plano de espelhamento inválido.".into()));
    }
    let normal = normal.map(|v| v / length);
    let part = doc.clone_part(source.part, name)?;
    let (local_origin, local_normal) = local_plane(source.placement, origin, normal)?;
    let mut pending = vec![(part, local_origin, local_normal)];
    let mut visited = HashSet::new();
    while let Some((part, origin, normal)) = pending.pop() {
        if !visited.insert(part) {
            continue;
        }
        mirror_bodies(doc, part, origin, normal)?;
        // A lista é capturada antes das mutações: cada posição da definição é ajustada
        // uma só vez, mesmo quando o descendente é compartilhado por vários caminhos.
        let children = doc
            .instances()
            .iter()
            .filter(|i| i.parent == Some(part))
            .map(|i| (i.id, i.part, i.placement))
            .collect::<Vec<_>>();
        for (instance, child, placement) in children {
            doc.instance_mut(instance)?.placement = placement
                .mirrored_frame(origin, normal)
                .map_err(|e| DocumentError(e.to_string()))?;
            pending.push((child, [0.; 3], [1., 0., 0.]));
        }
    }
    let instance = doc.add_instance(part, source.parent, source.placement)?;
    doc.instance_mut(instance)?.color = source.color;
    Ok(())
}

/// Cada definição STEP é importada uma vez; instâncias só carregam referências e posições.
/// O destino de construção recebe as raízes como subpeças independentes, sem mesclar
/// definições nem criar vínculos com o arquivo de origem.
pub(super) fn import_step(
    doc: &mut Document,
    path: &std::path::Path,
    destination: Option<PartId>,
    name: String,
    unverified: &mut Vec<String>,
) -> Result<super::Effect, DocumentError> {
    nator_document::validate_name(&name)?;
    if destination.is_some_and(|id| doc.part(id).is_none()) {
        return Err(DocumentError("Peça de destino inexistente.".into()));
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = (path, unverified);
        nator_kernel::require_available().map_err(|e| DocumentError(e.to_string()))?;
        Err(DocumentError("Importação STEP indisponível.".into()))
    }
    #[cfg(feature = "occt")]
    {
        let mut scene =
            nator_kernel::StepAssembly::read(path).map_err(|e| DocumentError(e.to_string()))?;
        let mut parts = Vec::with_capacity(scene.parts.len());
        let mut first_body = None;
        let mut triangles = 0;
        for part in &mut scene.parts {
            let id = doc.create_part(part.name.clone())?;
            let automatic = doc
                .instances()
                .last()
                .ok_or_else(|| DocumentError("Instância automática ausente.".into()))?
                .id;
            doc.remove_instance(automatic)?;
            for body in &mut part.bodies {
                let b = add_step_body(doc, id, body, body.name.clone(), &mut triangles)?;
                if !body.surface && !body.verified {
                    unverified.push(format!("{} / {}", part.name, body.name));
                }
                first_body.get_or_insert((id, b));
            }
            parts.push(id);
        }
        for occurrence in scene.instances {
            let part = *parts
                .get(occurrence.part)
                .ok_or_else(|| DocumentError("Definição STEP ausente.".into()))?;
            let parent = occurrence
                .parent
                .map(|p| {
                    parts
                        .get(p)
                        .copied()
                        .ok_or_else(|| DocumentError("Pai STEP ausente.".into()))
                })
                .transpose()?
                .or(destination);
            let instance = doc.add_instance(part, parent, occurrence.placement)?;
            let record = doc.instance_mut(instance)?;
            record.name = occurrence.name;
            record.color = occurrence.color;
        }
        let (part, body) =
            first_body.ok_or_else(|| DocumentError("STEP não contém corpos sólidos.".into()))?;
        Ok(super::Effect::Imported { part, body })
    }
}

/// Corpo de um sólido STEP na peça `part`. Superfície aberta não é um sólido: vira corpo de
/// malha, que aparece e sai em STL, mas fica fora das operações CAD e da exportação STEP.
#[cfg(feature = "occt")]
fn add_step_body(
    doc: &mut Document,
    part: PartId,
    body: &mut nator_kernel::StepBody,
    name: String,
    triangles: &mut usize,
) -> Result<nator_document::BodyId, DocumentError> {
    let kernel = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let (mesh, face_indices) = body
        .solid
        .tessellate_with_faces(0.05, 0.25)
        .map_err(kernel)?;
    *triangles += mesh.triangles().len();
    if *triangles > nator_document::MAX_SCENE_TRIANGLES {
        return Err(DocumentError(
            "STEP excede dois milhões de triângulos.".into(),
        ));
    }
    if body.surface {
        let (_, b) = doc.add_mesh(Some(part), name, Arc::new(MeshAsset::new(mesh)))?;
        doc.body_mut(b)?.color = body.color;
        return Ok(b);
    }
    let faces = body.solid.face_count().map_err(kernel)?;
    let brep = body.solid.to_brep().map_err(kernel)?;
    let (_, b) = doc.add_mesh(
        Some(part),
        name,
        Arc::new(MeshAsset::with_faces(mesh, face_indices)?),
    )?;
    let record = doc.body_mut(b)?;
    record.brep = Some(Arc::new(brep));
    // A geometria importada nunca é recalculada: nomes posicionais são estáveis.
    record.face_names = Arc::new(nator_kernel::indexed_face_names(
        &nator_document::features::body_owner(b),
        faces,
    ));
    record.color = body.color;
    Ok(b)
}

/// Importa os sólidos de um STEP como corpos da peça `part`. Cada ocorrência do arquivo vira
/// um corpo por sólido da definição, na posição acumulada da hierarquia (guardada em
/// `Body::placement`); nenhuma subpeça é criada.
pub(super) fn import_step_bodies(
    doc: &mut Document,
    path: &std::path::Path,
    part: PartId,
    unverified: &mut Vec<String>,
) -> Result<super::Effect, DocumentError> {
    if doc.part(part).is_none() {
        return Err(DocumentError("Peça de destino inexistente.".into()));
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = (path, unverified);
        nator_kernel::require_available().map_err(|e| DocumentError(e.to_string()))?;
        Err(DocumentError("Importação STEP indisponível.".into()))
    }
    #[cfg(feature = "occt")]
    {
        let mut scene =
            nator_kernel::StepAssembly::read(path).map_err(|e| DocumentError(e.to_string()))?;
        // Ocorrências na ordem do arquivo, com a posição composta desde a raiz.
        let mut placed = Vec::new();
        let mut pending = scene
            .instances
            .iter()
            .enumerate()
            .filter(|(_, o)| o.parent.is_none())
            .map(|(i, o)| (i, o.placement, o.color, 0_usize))
            .collect::<Vec<_>>();
        pending.reverse();
        while let Some((index, placement, color, depth)) = pending.pop() {
            if depth > 64 || placed.len() >= nator_document::MAX_ELEMENTS {
                return Err(DocumentError(
                    "Hierarquia STEP cíclica ou excessiva.".into(),
                ));
            }
            let occurrence = &scene.instances[index];
            placed.push((occurrence.part, occurrence.name.clone(), placement, color));
            let children = scene
                .instances
                .iter()
                .enumerate()
                .filter(|(_, o)| o.parent == Some(occurrence.part))
                .map(|(i, o)| {
                    placement
                        .compose(o.placement)
                        .map(|world| (i, world, o.color.or(color), depth + 1))
                        .map_err(|e| DocumentError(e.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            pending.extend(children.into_iter().rev());
        }
        let mut first = None;
        let mut triangles = 0;
        for (definition, occurrence, placement, color) in placed {
            let definition = scene
                .parts
                .get_mut(definition)
                .ok_or_else(|| DocumentError("Definição STEP ausente.".into()))?;
            let single = definition.bodies.len() == 1;
            for body in &mut definition.bodies {
                let name = if single || body.name.is_empty() {
                    occurrence.clone()
                } else {
                    format!("{occurrence} / {}", body.name)
                };
                let name = if name.trim().is_empty() {
                    "Corpo STEP".to_string()
                } else {
                    name
                };
                let b = add_step_body(doc, part, body, name.clone(), &mut triangles)?;
                let record = doc.body_mut(b)?;
                record.placement = placement;
                // Como na montagem, a cor da ocorrência mais interna vale sobre a do sólido.
                record.color = color.or(record.color);
                if !body.surface && !body.verified {
                    unverified.push(name);
                }
                first.get_or_insert(b);
            }
        }
        let body = first.ok_or_else(|| DocumentError("STEP não contém corpos sólidos.".into()))?;
        Ok(super::Effect::Imported { part, body })
    }
}

/// Pontos incluem translação; normais incluem somente a rotação inversa. Retirar a
/// translação antes de converter a normal evita perder precisão para peças longe da origem.
fn local_plane(
    placement: Placement,
    origin: Point3,
    normal: Point3,
) -> Result<(Point3, Point3), DocumentError> {
    let rotation = Placement::new([0.; 3], placement.rotation_rad())
        .map_err(|e| DocumentError(e.to_string()))?;
    Ok((
        placement.inverse_apply(origin),
        rotation.inverse_apply(normal),
    ))
}

fn mirror_bodies(
    doc: &mut Document,
    part: PartId,
    origin: Point3,
    normal: Point3,
) -> Result<(), DocumentError> {
    let piece = doc
        .part(part)
        .ok_or_else(|| DocumentError("Peça espelhada ausente.".into()))?;
    // A reflexão entra no fim do histórico de cada definição. Operações inativas
    // continuariam usando os corpos antes da reflexão quando fossem reativadas.
    if piece.history_marker.is_some() || piece.features.iter().any(|f| f.suppressed) {
        return Err(DocumentError(format!(
            "Reative as operações e leve o marcador do histórico de “{}” ao fim antes de espelhar o conjunto.",
            piece.name
        )));
    }
    let consumed = piece
        .features
        .iter()
        .flat_map(|f| f.operation.inputs())
        .collect::<HashSet<_>>();
    // Intermediários ocultos conservam a geometria anterior à reflexão no histórico.
    // Corpos finais ocultos também precisam ser espelhados, para aparecerem corretamente
    // se o usuário os tornar visíveis depois. A visibilidade final é preservada.
    let bodies = piece
        .bodies
        .iter()
        .filter(|b| b.visible || !consumed.contains(&b.id))
        .map(|b| (b.id, b.brep.is_some(), b.placement, b.visible))
        .collect::<Vec<_>>();
    for (body, solid, placement, visible) in bodies {
        if solid {
            let output = add_feature(
                doc,
                part,
                "Espelhamento da montagem".into(),
                Operation::Mirror {
                    base: body,
                    origin,
                    normal,
                    keep_original: false,
                },
            )?;
            doc.body_mut(output)?.visible = visible;
        } else {
            let (origin, normal) = local_plane(placement, origin, normal)?;
            let target = doc.body_mut(body)?;
            let mesh = target
                .geometry
                .mesh()
                .reflected(origin, normal)
                .map_err(|e| DocumentError(e.to_string()))?;
            target.geometry = Arc::new(MeshAsset::new(mesh));
        }
    }
    Ok(())
}
