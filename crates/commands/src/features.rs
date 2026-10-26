//! Construção ocorre no worker. Somente bytes B-rep, nomes de faces e malhas chegam ao
//! documento; objetos nativos do núcleo nunca saem da thread que os criou.
#[cfg(feature = "occt")]
use nator_document::features::{BooleanMode, EdgeRef, FaceRef, RegionRef, body_owner, signature};
#[cfg(feature = "occt")]
use nator_document::parameters::Dimension;
use nator_document::{BodyId, Document, DocumentError, MeshAsset, PartId, features::Operation};
#[cfg(feature = "occt")]
use nator_kernel::Solid;
use std::sync::Arc;

/// Resultado CAD de uma operação: malha de exibição, B-rep e nomes estáveis das faces.
pub(super) struct Built {
    pub mesh: Arc<MeshAsset>,
    pub brep: Arc<Vec<u8>>,
    pub faces: Arc<Vec<String>>,
}

/// Recalcula todo o histórico da peça na ordem. Uma falha interrompe o recálculo e, como
/// os comandos operam sobre uma cópia candidata, o documento anterior é preservado.
pub(super) fn rebuild(doc: &mut Document, part: PartId) -> Result<(), DocumentError> {
    let operations = doc
        .part(part)
        .ok_or_else(|| DocumentError("Peça inexistente.".into()))?
        .features
        .clone();
    for feature in operations {
        let built = construct(doc, part, feature.output, &feature.operation)
            .map_err(|e| DocumentError(format!("{}: {}", feature.name, e.0)))?;
        let body = doc.body_mut(feature.output)?;
        body.geometry = built.mesh;
        body.brep = Some(built.brep);
        body.face_names = built.faces;
    }
    Ok(())
}

#[cfg(feature = "occt")]
fn error(e: impl std::fmt::Display) -> DocumentError {
    DocumentError(e.to_string())
}

/// Corpo de entrada já posicionado na peça, com os nomes de faces gravados.
#[cfg(feature = "occt")]
fn source(
    doc: &Document,
    part: PartId,
    id: BodyId,
) -> Result<(Solid, Arc<Vec<u8>>), DocumentError> {
    let body = doc
        .part(part)
        .and_then(|p| p.bodies.iter().find(|b| b.id == id))
        .ok_or_else(|| DocumentError("Corpo da operação inexistente nesta peça.".into()))?;
    let bytes = body.brep.clone().ok_or_else(|| {
        DocumentError("Corpos de malha não participam de operações CAD precisas.".into())
    })?;
    let names = (!body.face_names.is_empty()).then_some(body.face_names.as_slice());
    let solid = Solid::from_brep(&bytes, names)
        .and_then(|s| s.transformed(body.placement))
        .map_err(error)?;
    Ok((solid, bytes))
}

#[cfg(feature = "occt")]
fn sketch_of(
    doc: &Document,
    part: PartId,
    id: nator_document::SketchId,
) -> Result<&nator_sketch::Sketch, DocumentError> {
    doc.part(part)
        .and_then(|p| p.sketches.iter().find(|r| r.id == id))
        .map(|r| &r.sketch)
        .ok_or_else(|| DocumentError("Rascunho inexistente nesta peça.".into()))
}

#[cfg(feature = "occt")]
fn region(
    sketch: &nator_sketch::Sketch,
    reference: &RegionRef,
) -> Result<nator_sketch::profiles::Region, DocumentError> {
    use nator_sketch::profiles;
    let mut regions = profiles::regions(sketch).map_err(error)?;
    let index = match reference {
        RegionRef::Boundary(ids) => profiles::find_region(&regions, ids).map_err(error)?,
        RegionRef::Index(index) => *index,
    };
    if index >= regions.len() {
        return Err(DocumentError("Região do rascunho inexistente.".into()));
    }
    Ok(regions.swap_remove(index))
}

/// Uma referência por índice só vale para o B-rep exato em que foi escolhida.
#[cfg(feature = "occt")]
fn same_geometry(bytes: &[u8], expected: u64) -> Result<(), DocumentError> {
    if signature(bytes) != expected {
        return Err(DocumentError(
            "A geometria mudou: selecione novamente a aresta/face desta operação no histórico."
                .into(),
        ));
    }
    Ok(())
}

#[cfg(feature = "occt")]
fn edge_index(solid: &Solid, bytes: &[u8], edge: &EdgeRef) -> Result<usize, DocumentError> {
    match edge {
        EdgeRef::Between(faces) => solid.edge_between(faces).map_err(error),
        EdgeRef::Index { index, signature } => {
            same_geometry(bytes, *signature)?;
            Ok(*index)
        }
    }
}

#[cfg(feature = "occt")]
fn face_index(solid: &Solid, bytes: &[u8], face: &FaceRef) -> Result<usize, DocumentError> {
    match face {
        FaceRef::Named(name) => solid.face_named(name).map_err(error),
        FaceRef::Index { index, signature } => {
            same_geometry(bytes, *signature)?;
            Ok(*index)
        }
    }
}

/// Converte seleções por índice em referências estáveis antes de gravá-las no histórico.
/// A interface escolhe regiões, arestas e faces pela posição atual; aqui essa posição é
/// traduzida para a identidade de origem. Quando não há nome estável (por exemplo, face sem
/// origem conhecida), a referência por índice é mantida com sua assinatura de geometria.
#[cfg(feature = "occt")]
pub(super) fn normalize(
    doc: &Document,
    part: PartId,
    operation: Operation,
) -> Result<Operation, DocumentError> {
    let stable_region = |sketch, reference: RegionRef| -> Result<RegionRef, DocumentError> {
        Ok(match reference {
            RegionRef::Index(_) => {
                RegionRef::Boundary(region(sketch_of(doc, part, sketch)?, &reference)?.boundary())
            }
            stable => stable,
        })
    };
    Ok(match operation {
        Operation::Extrude {
            sketch,
            region,
            from,
            to,
        } => Operation::Extrude {
            region: stable_region(sketch, region)?,
            sketch,
            from,
            to,
        },
        Operation::Revolve {
            sketch,
            region,
            axis_origin,
            axis_direction,
            angle,
        } => Operation::Revolve {
            region: stable_region(sketch, region)?,
            sketch,
            axis_origin,
            axis_direction,
            angle,
        },
        Operation::Fillet { base, edge, radius } => Operation::Fillet {
            edge: stable_edge(doc, part, base, edge)?,
            base,
            radius,
        },
        Operation::Chamfer {
            base,
            edge,
            distance,
        } => Operation::Chamfer {
            edge: stable_edge(doc, part, base, edge)?,
            base,
            distance,
        },
        Operation::Shell {
            base,
            face,
            thickness,
        } => {
            let face = match face {
                FaceRef::Index { index, signature } => {
                    let (solid, bytes) = source(doc, part, base)?;
                    same_geometry(&bytes, signature)?;
                    match solid.face_name(index) {
                        Ok(name) => FaceRef::Named(name.to_owned()),
                        Err(_) => FaceRef::Index { index, signature },
                    }
                }
                named => named,
            };
            Operation::Shell {
                base,
                face,
                thickness,
            }
        }
        other => other,
    })
}

#[cfg(feature = "occt")]
fn stable_edge(
    doc: &Document,
    part: PartId,
    base: BodyId,
    edge: EdgeRef,
) -> Result<EdgeRef, DocumentError> {
    let EdgeRef::Index { index, signature } = edge else {
        return Ok(edge);
    };
    let (solid, bytes) = source(doc, part, base)?;
    same_geometry(&bytes, signature)?;
    Ok(match solid.edge_face_names(index) {
        Ok(faces) if solid.edge_between(&faces).is_ok() => EdgeRef::Between(faces),
        // Par sem nome ou repetido: só o índice identifica a aresta nesta geometria.
        _ => EdgeRef::Index { index, signature },
    })
}

#[cfg(not(feature = "occt"))]
pub(super) fn normalize(
    _doc: &Document,
    _part: PartId,
    operation: Operation,
) -> Result<Operation, DocumentError> {
    Ok(operation)
}

#[cfg(feature = "occt")]
pub(super) fn construct(
    doc: &Document,
    part: PartId,
    output: BodyId,
    operation: &Operation,
) -> Result<Built, DocumentError> {
    let piece = doc
        .part(part)
        .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
    let value = |expression: &str| piece.parameters.evaluate(expression, Dimension::Length);
    let owner = body_owner(output);
    let mut solid = match operation {
        Operation::Extrude {
            sketch,
            region: reference,
            from,
            to,
        } => {
            let sketch = sketch_of(doc, part, *sketch)?;
            Solid::extrude(
                sketch,
                &region(sketch, reference)?,
                value(from)?,
                value(to)?,
                &owner,
            )
            .map_err(error)?
        }
        Operation::Revolve {
            sketch,
            region: reference,
            axis_origin,
            axis_direction,
            angle,
        } => {
            let sketch = sketch_of(doc, part, *sketch)?;
            Solid::revolve(
                sketch,
                &region(sketch, reference)?,
                *axis_origin,
                *axis_direction,
                piece.parameters.evaluate(angle, Dimension::Angle)?,
                &owner,
            )
            .map_err(error)?
        }
        Operation::Boolean { base, tool, mode } => source(doc, part, *base)?
            .0
            .boolean(
                &source(doc, part, *tool)?.0,
                match mode {
                    BooleanMode::Join => 0,
                    BooleanMode::Cut => 1,
                    BooleanMode::Intersect => 2,
                },
                &owner,
            )
            .map_err(error)?,
        Operation::Fillet { base, edge, radius } => {
            let (solid, bytes) = source(doc, part, *base)?;
            solid
                .fillet(edge_index(&solid, &bytes, edge)?, value(radius)?, &owner)
                .map_err(error)?
        }
        Operation::Chamfer {
            base,
            edge,
            distance,
        } => {
            let (solid, bytes) = source(doc, part, *base)?;
            solid
                .chamfer(edge_index(&solid, &bytes, edge)?, value(distance)?, &owner)
                .map_err(error)?
        }
        Operation::Shell {
            base,
            face,
            thickness,
        } => {
            let (solid, bytes) = source(doc, part, *base)?;
            solid
                .shell(face_index(&solid, &bytes, face)?, value(thickness)?, &owner)
                .map_err(error)?
        }
        Operation::Transform {
            base,
            translation,
            rotation,
        } => source(doc, part, *base)?
            .0
            .transformed(nator_geometry::Placement::new(*translation, *rotation).map_err(error)?)
            .map_err(error)?,
    };
    solid.volume().map_err(error)?;
    let brep = Arc::new(solid.to_brep().map_err(error)?);
    let faces = Arc::new(solid.face_names().to_vec());
    let (mesh, triangle_faces) = solid.tessellate_with_faces(0.05, 0.25).map_err(error)?;
    let mesh = Arc::new(MeshAsset::with_faces(mesh, triangle_faces)?);
    Ok(Built { mesh, brep, faces })
}
#[cfg(not(feature = "occt"))]
pub(super) fn construct(
    _doc: &Document,
    _part: PartId,
    _output: BodyId,
    _operation: &Operation,
) -> Result<Built, DocumentError> {
    nator_kernel::require_available().map_err(|e| DocumentError(e.to_string()))?;
    Err(DocumentError("Construção CAD indisponível.".into()))
}

/// Excluir uma operação nunca deixa referências silenciosamente apontando para outra geometria.
pub(super) fn remove(doc: &mut Document, part: PartId, body: BodyId) -> Result<(), DocumentError> {
    let piece = doc.part_mut(part)?;
    if piece
        .features
        .iter()
        .any(|f| f.operation.inputs().contains(&body))
    {
        return Err(DocumentError(
            "Remova primeiro as operações que dependem deste resultado.".into(),
        ));
    }
    let inputs = piece
        .features
        .iter()
        .find(|f| f.output == body)
        .map(|f| f.operation.inputs())
        .ok_or_else(|| DocumentError("Operação inexistente.".into()))?;
    piece.features.retain(|f| f.output != body);
    piece.bodies.retain(|b| b.id != body);
    for id in inputs {
        if !piece
            .features
            .iter()
            .any(|f| f.operation.inputs().contains(&id))
            && let Some(b) = piece.bodies.iter_mut().find(|b| b.id == id)
        {
            b.visible = true;
        }
    }
    Ok(())
}
