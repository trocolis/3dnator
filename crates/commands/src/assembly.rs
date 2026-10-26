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
) -> Result<super::Effect, DocumentError> {
    nator_document::validate_name(&name)?;
    if destination.is_some_and(|id| doc.part(id).is_none()) {
        return Err(DocumentError("Peça de destino inexistente.".into()));
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = path;
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
                let faces = body
                    .solid
                    .face_count()
                    .map_err(|e| DocumentError(e.to_string()))?;
                let brep = body
                    .solid
                    .to_brep()
                    .map_err(|e| DocumentError(e.to_string()))?;
                let (mesh, face_indices) = body
                    .solid
                    .tessellate_with_faces(0.05, 0.25)
                    .map_err(|e| DocumentError(e.to_string()))?;
                triangles += mesh.triangles().len();
                if triangles > nator_document::MAX_SCENE_TRIANGLES {
                    return Err(DocumentError(
                        "STEP excede dois milhões de triângulos.".into(),
                    ));
                }
                let (_, b) = doc.add_mesh(
                    Some(id),
                    body.name.clone(),
                    Arc::new(MeshAsset::with_faces(mesh, face_indices)?),
                )?;
                let record = doc.body_mut(b)?;
                record.brep = Some(Arc::new(brep));
                record.face_names = Arc::new(nator_kernel::indexed_face_names(
                    &nator_document::features::body_owner(b),
                    faces,
                ));
                record.color = body.color;
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
