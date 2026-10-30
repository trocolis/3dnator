//! Contexto e eventos do histórico, independentes da interface. Referências são
//! obtidas antes da edição e os IDs criados são completados pelo Effect.
use super::*;
use nator_document::activity::{ActivityKind as K, ActivityTarget as T, EditContext as C};

fn owner(doc: &Document, body: BodyId) -> Option<PartId> {
    doc.parts()
        .iter()
        .find(|p| p.bodies.iter().any(|b| b.id == body))
        .map(|p| p.id)
}
fn sketch_context(doc: &Document, sketch: SketchId) -> C {
    C::Sketch {
        part: doc
            .parts()
            .iter()
            .find(|p| p.sketches.iter().any(|s| s.id == sketch))
            .map(|p| p.id),
        sketch,
    }
}
pub(super) fn context(command: &Command, doc: &Document) -> C {
    match command {
        Command::UpdateSketch { id, .. }
        | Command::RemoveSketch { id }
        | Command::RenameSketch { id, .. } => sketch_context(doc, *id),
        Command::AddReferenceImage { image } | Command::UpdateReferenceImage { image } => {
            sketch_context(doc, image.sketch)
        }
        Command::RemoveReferenceImage { id } => doc
            .images()
            .iter()
            .find(|i| i.id == *id)
            .map(|i| sketch_context(doc, i.sketch))
            .unwrap_or(C::Keep),
        Command::SplitBody { part, .. }
        | Command::AddFeature { part, .. }
        | Command::AddSmartExtrusion { part, .. }
        | Command::EditFeature { part, .. }
        | Command::RemoveFeature { part, .. }
        | Command::SetFeatureSuppressed { part, .. }
        | Command::RenameFeature { part, .. }
        | Command::MoveFeature { part, .. }
        | Command::SetHistoryMarker { part, .. }
        | Command::SetParameter { part, .. }
        | Command::AddWorkPlane { part, .. } => C::Part(*part),
        Command::MoveBody { id, .. }
        | Command::CopyBody { id, .. }
        | Command::RemoveBody { id }
        | Command::RenameBody { id, .. }
        | Command::SetBodyColor { id, .. } => owner(doc, *id).map(C::Part).unwrap_or(C::Keep),
        Command::PaintSurface { body, .. } | Command::ClearSurfacePaint { body, .. } => {
            owner(doc, *body).map(C::Part).unwrap_or(C::Keep)
        }
        Command::PaintSurfaces { targets, .. } => targets
            .first()
            .and_then(|t| {
                owner(
                    doc,
                    match t {
                        crate::painting::Target::Body(b)
                        | crate::painting::Target::Face { body: b, .. } => *b,
                    },
                )
            })
            .map(C::Part)
            .unwrap_or(C::Keep),
        Command::UpdateWorkPlane { id, .. }
        | Command::RemoveWorkPlane { id }
        | Command::AddSketchOnWorkPlane { id, .. } => doc
            .parts()
            .iter()
            .find(|p| p.work_planes.iter().any(|w| w.id == *id))
            .map(|p| C::Part(p.id))
            .unwrap_or(C::Keep),
        Command::AddSketch { destination, .. } => destination.map(C::Part).unwrap_or(C::Assembly),
        Command::RenamePart { id, .. } => C::Part(*id),
        Command::CreatePart { .. }
        | Command::CreatePartIn { .. }
        | Command::AddReferenceSketch { .. }
        | Command::MoveInstance { .. }
        | Command::MoveGroup { .. }
        | Command::CreateGroup { .. }
        | Command::RemoveGroup { .. }
        | Command::DuplicateInstance { .. }
        | Command::CopyInstanceAt { .. }
        | Command::CloneInstance { .. }
        | Command::AddInstance { .. }
        | Command::RemoveInstance { .. }
        | Command::ReparentInstance { .. }
        | Command::RenameInstance { .. }
        | Command::RenameGroup { .. }
        | Command::PatternInstance { .. }
        | Command::MirrorInstance { .. }
        | Command::InsertPart { .. } => C::Assembly,
        Command::Batch { commands, .. } => commands
            .iter()
            .map(|c| context(c, doc))
            .find(|c| *c != C::Keep)
            .unwrap_or(C::Keep),
        _ => C::Keep,
    }
}
pub(super) fn created_context(context: C, effect: Effect) -> C {
    match effect {
        Effect::SketchCreated { part, sketch } => C::Sketch {
            part: Some(part),
            sketch,
        },
        Effect::ReferenceSketchCreated { sketch } => C::Sketch { part: None, sketch },
        Effect::PartCreated { part } | Effect::Imported { part, .. } if context == C::Keep => {
            C::Part(part)
        }
        _ => context,
    }
}

pub(super) struct Intent {
    context: C,
    kind: K,
    target: Option<T>,
    remove_group: Option<nator_document::GroupId>,
}
pub(super) fn intent(command: &Command, doc: &Document) -> Option<Intent> {
    let context = context(command, doc);
    let (kind, target) = match command {
        Command::AddSketch { .. }
        | Command::AddReferenceSketch { .. }
        | Command::AddSketchOnWorkPlane { .. } => (K::Sketch, None),
        Command::CreatePart { .. } | Command::CreatePartIn { .. } => (K::Part, None),
        Command::SplitBody { .. }
        | Command::AddFeature { .. }
        | Command::AddSmartExtrusion { .. } => (K::Feature, None),
        Command::UpdateSketch { id, sketch } => {
            let old = &doc.sketch(*id)?.sketch;
            let added = sketch
                .entities
                .iter()
                .filter(|e| !old.entities.iter().any(|o| o.id == e.id))
                .map(|e| e.id)
                .collect::<Vec<_>>();
            // Entidades mantêm IDs quando apenas seus pontos são arrastados.
            // Comparar também os apoios evita perder a seleção dessas edições.
            let changed = sketch
                .entities
                .iter()
                .filter(|e| {
                    old.entities
                        .iter()
                        .find(|o| o.id == e.id)
                        .is_none_or(|o| o != *e)
                        || e.point_ids()
                            .iter()
                            .any(|id| old.point(*id).ok() != sketch.point(*id).ok())
                })
                .map(|e| e.id)
                .collect::<Vec<_>>();
            let constraints = sketch
                .constraints
                .iter()
                .filter(|c| {
                    old.constraints
                        .iter()
                        .find(|o| o.id == c.id)
                        .is_none_or(|o| o != *c)
                        || old.dimension_expressions.get(&c.id)
                            != sketch.dimension_expressions.get(&c.id)
                        || old.dimension_labels.get(&c.id) != sketch.dimension_labels.get(&c.id)
                })
                .map(|c| c.id)
                .collect::<Vec<_>>();
            let kind = if !added.is_empty()
                && sketch
                    .entities
                    .iter()
                    .any(|e| added.contains(&e.id) && e.projected)
            {
                K::Projection
            } else if !added.is_empty() {
                K::Geometry
            } else if !constraints.is_empty() {
                if sketch
                    .constraints
                    .iter()
                    .any(|c| constraints.contains(&c.id) && c.kind.is_dimension())
                {
                    K::Dimension
                } else {
                    K::Relation
                }
            } else {
                K::Modify
            };
            // Deslocar só câmera/seleção nunca chega como comando. Atualizações
            // sem diferença útil tampouco produzem um cartão vazio.
            if changed.is_empty()
                && old
                    .points
                    .iter()
                    .map(|p| (p.id, p.position, p.projected))
                    .eq(sketch
                        .points
                        .iter()
                        .map(|p| (p.id, p.position, p.projected)))
                && constraints.is_empty()
                && old.entities == sketch.entities
                && old.constraints == sketch.constraints
            {
                return None;
            }
            (
                kind,
                Some(T::Sketch {
                    id: *id,
                    entities: changed,
                    constraints,
                }),
            )
        }
        Command::MoveInstance { id, .. } => (K::Move, Some(T::Instance(*id))),
        Command::MoveBody { id, .. } => (K::Move, Some(T::Body(*id))),
        Command::CopyBody { .. } => (K::Copy, None),
        Command::DuplicateInstance { .. }
        | Command::CloneInstance { .. }
        | Command::CopyInstanceAt { .. } => (K::Copy, None),
        Command::MoveGroup { id, .. } => (K::Move, Some(T::Group(*id))),
        Command::CreateGroup { .. } => (K::Group, None),
        Command::RemoveGroup { id } => {
            return Some(Intent {
                context,
                kind: K::Group,
                target: None,
                remove_group: Some(*id),
            });
        }
        Command::PaintSurfaces { targets, .. } => (
            K::Paint,
            Some(T::Body(match targets.first()? {
                painting::Target::Body(b) | painting::Target::Face { body: b, .. } => *b,
            })),
        ),
        Command::PaintSurface { body, .. } | Command::SetBodyColor { id: body, .. } => {
            (K::Paint, Some(T::Body(*body)))
        }
        Command::AddWorkPlane { .. } => (K::WorkPlane, None),
        Command::AddReferenceImage { image } => (K::Image, Some(T::Image(image.id))),
        _ => return None,
    };
    Some(Intent {
        context,
        kind,
        target,
        remove_group: None,
    })
}
pub(super) fn record(
    doc: &mut Document,
    intent: Option<Intent>,
    effect: Effect,
) -> Result<(), DocumentError> {
    let Some(mut i) = intent else { return Ok(()) };
    if let Some(id) = i.remove_group {
        doc.remove_group_activity(id);
        return Ok(());
    }
    if i.target.is_none() {
        i.target = match effect {
            Effect::SketchCreated { part, sketch } => {
                i.context = C::Part(part);
                Some(T::Sketch {
                    id: sketch,
                    entities: vec![],
                    constraints: vec![],
                })
            }
            Effect::ReferenceSketchCreated { sketch } => {
                i.context = C::Assembly;
                Some(T::Sketch {
                    id: sketch,
                    entities: vec![],
                    constraints: vec![],
                })
            }
            Effect::PartCreated { part } => {
                i.context = C::Assembly;
                Some(T::Part(part))
            }
            Effect::Imported { part, body } => Some(if i.kind == K::Feature {
                T::Feature { part, output: body }
            } else {
                T::Body(body)
            }),
            Effect::WorkPlaneCreated { plane, .. } => Some(T::WorkPlane(plane)),
            _ if i.kind == K::Copy => doc.instances().last().map(|v| T::Instance(v.id)),
            _ if i.kind == K::Group => doc.groups().last().map(|g| T::Group(g.id)),
            _ => None,
        };
    }
    if let Some(target) = i.target {
        doc.record_activity(i.context, i.kind, target)?;
    }
    Ok(())
}
