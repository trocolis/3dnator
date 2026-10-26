//! Construção ocorre no worker. Somente bytes B-rep e malhas chegam ao documento.
#[cfg(feature = "occt")]
use nator_document::features::BooleanMode;
use nator_document::{BodyId, Document, DocumentError, PartId, features::Operation};
#[cfg(feature = "occt")]
use nator_document::{MeshAsset, features::signature, parameters::Dimension};
#[cfg(feature = "occt")]
use nator_kernel::Solid;
#[cfg(feature = "occt")]
use std::sync::Arc;

pub(super) fn rebuild(doc: &mut Document, part: PartId) -> Result<(), DocumentError> {
    let operations = doc
        .part(part)
        .ok_or_else(|| DocumentError("Peça inexistente.".into()))?
        .features
        .clone();
    for feature in operations {
        let (mesh, brep) = construct(doc, part, &feature.operation)?;
        let body = doc.body_mut(feature.output)?;
        body.geometry = mesh;
        body.brep = Some(brep);
    }
    Ok(())
}
#[cfg(feature = "occt")]
fn source(doc: &Document, part: PartId, id: BodyId) -> Result<Solid, DocumentError> {
    let body = doc
        .part(part)
        .and_then(|p| p.bodies.iter().find(|b| b.id == id))
        .ok_or_else(|| DocumentError("Corpo da operação inexistente nesta peça.".into()))?;
    let bytes = body.brep.as_ref().ok_or_else(|| {
        DocumentError("Corpos de malha não participam de operações CAD precisas.".into())
    })?;
    Solid::from_brep(bytes)
        .and_then(|s| s.transformed(body.placement))
        .map_err(error)
}
#[cfg(feature = "occt")]
fn error(e: impl std::fmt::Display) -> DocumentError {
    DocumentError(e.to_string())
}
#[cfg(feature = "occt")]
pub(super) fn construct(
    doc: &Document,
    part: PartId,
    operation: &Operation,
) -> Result<(Arc<MeshAsset>, Arc<Vec<u8>>), DocumentError> {
    let piece = doc
        .part(part)
        .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
    let value = |expression: &str| piece.parameters.evaluate(expression, Dimension::Length);
    let sketch = |id| {
        piece
            .sketches
            .iter()
            .find(|r| r.id == id)
            .map(|r| &r.sketch)
            .ok_or_else(|| DocumentError("Rascunho inexistente nesta peça.".into()))
    };
    let guarded_source = |id, expected| -> Result<Solid, DocumentError> {
        let bytes = piece
            .bodies
            .iter()
            .find(|b| b.id == id)
            .and_then(|b| b.brep.as_ref())
            .ok_or_else(|| DocumentError("Operação exige sólido CAD.".into()))?;
        if signature(bytes) != expected {
            return Err(DocumentError(
                "A geometria mudou: selecione novamente a aresta/face desta operação no histórico."
                    .into(),
            ));
        }
        source(doc, part, id)
    };
    let mut solid = match operation {
        Operation::Extrude {
            sketch: id,
            region,
            from,
            to,
        } => Solid::extrude(sketch(*id)?, *region, value(from)?, value(to)?).map_err(error)?,
        Operation::Revolve {
            sketch: id,
            region,
            axis_origin,
            axis_direction,
            angle,
        } => Solid::revolve(
            sketch(*id)?,
            *region,
            *axis_origin,
            *axis_direction,
            piece.parameters.evaluate(angle, Dimension::Angle)?,
        )
        .map_err(error)?,
        Operation::Boolean { base, tool, mode } => source(doc, part, *base)?
            .boolean(
                &source(doc, part, *tool)?,
                match mode {
                    BooleanMode::Join => 0,
                    BooleanMode::Cut => 1,
                    BooleanMode::Intersect => 2,
                },
            )
            .map_err(error)?,
        Operation::Fillet {
            base,
            edge,
            signature,
            radius,
        } => guarded_source(*base, *signature)?
            .fillet(*edge, value(radius)?)
            .map_err(error)?,
        Operation::Chamfer {
            base,
            edge,
            signature,
            distance,
        } => guarded_source(*base, *signature)?
            .chamfer(*edge, value(distance)?)
            .map_err(error)?,
        Operation::Shell {
            base,
            face,
            signature,
            thickness,
        } => guarded_source(*base, *signature)?
            .shell(*face, value(thickness)?)
            .map_err(error)?,
        Operation::Transform {
            base,
            translation,
            rotation,
        } => source(doc, part, *base)?
            .transformed(nator_geometry::Placement::new(*translation, *rotation).map_err(error)?)
            .map_err(error)?,
    };
    solid.volume().map_err(error)?;
    let brep = Arc::new(solid.to_brep().map_err(error)?);
    let mesh = Arc::new(MeshAsset::new(solid.tessellate(0.05, 0.25).map_err(error)?));
    Ok((mesh, brep))
}
#[cfg(not(feature = "occt"))]
pub(super) fn construct(
    _doc: &Document,
    _part: PartId,
    _operation: &Operation,
) -> Result<
    (
        std::sync::Arc<nator_document::MeshAsset>,
        std::sync::Arc<Vec<u8>>,
    ),
    DocumentError,
> {
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
