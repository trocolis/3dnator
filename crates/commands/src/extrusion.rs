//! Consultas e decisões de extrusão, sem dependência da interface. O modo automático
//! considera contato no início do prisma e volume realmente intersectado, não caixas
//! de malha nem o sinal global de Z. Tudo é calculado no referencial local da peça.
use nator_document::{
    BodyId, Document, DocumentError, PartId,
    features::{Combine, Operation},
};
use nator_geometry::Point3;

/// Apoio do manipulador no referencial da peça. Origem e limites estão em mm;
/// a normal é unitária e orienta o sentido positivo, independente da câmera.
#[derive(Debug, Clone, Copy)]
pub struct Guide {
    pub origin: Point3,
    pub normal: Point3,
    pub from: f64,
    pub to: f64,
}

#[cfg(feature = "occt")]
mod precise {
    use super::*;
    use crate::features::{face_index, region, source};
    use nator_document::{
        features::{BooleanMode, UntilFace},
        parameters::Dimension,
    };
    use nator_geometry::{dot, norm, sub};
    use nator_kernel::Solid;
    fn error(e: impl std::fmt::Display) -> DocumentError {
        DocumentError(e.to_string())
    }

    pub fn guide(
        doc: &Document,
        part: PartId,
        operation: &Operation,
    ) -> Result<Guide, DocumentError> {
        let piece = doc
            .part(part)
            .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
        let (origin, normal, from, to, until) = match operation {
            Operation::Extrude {
                sketch,
                region: reference,
                from,
                to,
                until,
                ..
            } => {
                let sketch = &piece
                    .sketches
                    .iter()
                    .find(|s| s.id == *sketch)
                    .ok_or_else(|| DocumentError("Rascunho inexistente.".into()))?
                    .sketch;
                let reference =
                    if let nator_document::features::RegionRef::Multiple(regions) = reference {
                        regions
                            .first()
                            .ok_or_else(|| error("Selecione uma área."))?
                    } else {
                        reference
                    };
                let region = region(sketch, reference)?;
                let points = region.outer.sample(48);
                let center = points.iter().fold([0.; 2], |a, p| nator_sketch::add(a, *p));
                let center = nator_sketch::scale(center, 1. / points.len().max(1) as f64);
                (
                    sketch.plane.world(center),
                    sketch.plane.normal(),
                    from,
                    to,
                    until,
                )
            }
            Operation::ExtrudeFace {
                base,
                face,
                from,
                to,
                until,
                ..
            } => {
                let (solid, bytes) = source(doc, part, *base)?;
                let index = face_index(&solid, &bytes, face)?;
                let nator_kernel::FaceKind::Plane { normal, .. } =
                    solid.face_measure(index).map_err(error)?.kind
                else {
                    return Err(DocumentError(
                        "Extrusão direta exige uma face plana.".into(),
                    ));
                };
                (
                    solid.face_anchors(index).map_err(error)?.centroid,
                    normal,
                    from,
                    to,
                    until,
                )
            }
            _ => return Err(DocumentError("A operação não é uma extrusão.".into())),
        };
        let from = piece.parameters.evaluate(from, Dimension::Length)?;
        let to = match until {
            Some(limit) => distance_to_face(doc, part, origin, normal, limit)?,
            None => piece.parameters.evaluate(to, Dimension::Length)?,
        };
        Ok(Guide {
            origin,
            normal,
            from,
            to,
        })
    }

    fn limit_plane(
        doc: &Document,
        part: PartId,
        limit: &UntilFace,
    ) -> Result<(Point3, Point3), DocumentError> {
        let (solid, bytes) = source(doc, part, limit.body)?;
        let index = face_index(&solid, &bytes, &limit.face)?;
        match solid.face_measure(index).map_err(error)?.kind {
            nator_kernel::FaceKind::Plane { normal, offset } => {
                Ok((normal.map(|v| v * offset), normal))
            }
            _ => Err(DocumentError(
                "A face limite da extrusão deve ser plana.".into(),
            )),
        }
    }
    fn distance_to_face(
        doc: &Document,
        part: PartId,
        origin: Point3,
        normal: Point3,
        limit: &UntilFace,
    ) -> Result<f64, DocumentError> {
        let (point, n) = limit_plane(doc, part, limit)?;
        let denominator = dot(n, normal);
        if denominator.abs() < 1e-9 {
            return Err(DocumentError(
                "A face limite é paralela à direção da extrusão.".into(),
            ));
        }
        Ok(dot(n, sub(point, origin)) / denominator)
    }

    /// Prisma isolado: usado na construção, na classificação e na prévia de remoção.
    pub(crate) fn tool(
        doc: &Document,
        part: PartId,
        operation: &Operation,
        owner: &str,
        span: Option<(f64, f64)>,
    ) -> Result<Solid, DocumentError> {
        let g = guide(doc, part, operation)?;
        let (from, mut to) = span.unwrap_or((g.from, g.to));
        let until = match operation {
            Operation::Extrude { until, .. } | Operation::ExtrudeFace { until, .. } => {
                until.as_ref()
            }
            _ => None,
        };
        let limit = if span.is_none() {
            until.map(|l| limit_plane(doc, part, l)).transpose()?
        } else {
            None
        };
        let piece = doc
            .part(part)
            .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
        if let Some((_, normal)) = limit {
            // Ultrapassar conservadoramente a face e recortar pelo plano exato evita
            // terminar uma face inclinada na distância de um único ponto do perfil.
            let extent = match operation {
                Operation::Extrude { sketch, .. } => {
                    let sketch = &piece
                        .sketches
                        .iter()
                        .find(|s| s.id == *sketch)
                        .ok_or_else(|| DocumentError("Rascunho inexistente.".into()))?
                        .sketch;
                    sketch
                        .entities
                        .iter()
                        .filter_map(|e| nator_sketch::profiles::segment(sketch, e).ok())
                        .flat_map(|s| s.sample(16))
                        .map(|p| norm(sub(sketch.plane.world(p), g.origin)))
                        .fold(1_f64, f64::max)
                }
                Operation::ExtrudeFace { base, .. } => doc
                    .body(*base)
                    .map(|b| norm(b.geometry.mesh().bounds().size()))
                    .unwrap_or(1.),
                _ => 1.,
            };
            to += (to - from).signum() * (4. * extent / dot(normal, g.normal).abs() + 1.);
        }
        let mut prism = match operation {
            Operation::Extrude {
                sketch,
                region: reference,
                ..
            } => {
                let sketch = &piece
                    .sketches
                    .iter()
                    .find(|s| s.id == *sketch)
                    .ok_or_else(|| DocumentError("Rascunho inexistente.".into()))?
                    .sketch;
                // O início do prisma de rascunho é relativo ao plano, e o centro da
                // guia pertence a esse mesmo plano, independentemente da origem local.
                let references =
                    if let nator_document::features::RegionRef::Multiple(regions) = reference {
                        regions.as_slice()
                    } else {
                        std::slice::from_ref(reference)
                    };
                let mut result: Option<Solid> = None;
                for reference in references {
                    let area = region(sketch, reference)?;
                    let mut prism =
                        Solid::extrude(sketch, &area, from, to, owner).map_err(error)?;
                    // Todas as células pertencem ao mesmo corpo; o sufixo evita
                    // tampas homônimas e mantém faces selecionáveis no histórico.
                    if references.len() > 1 {
                        let signature =
                            nator_sketch::areas::signature(sketch, &area).map_err(error)?;
                        let bytes = signature
                            .iter()
                            .flat_map(|(id, reverse)| {
                                id.to_le_bytes().into_iter().chain([u8::from(*reverse)])
                            })
                            .collect::<Vec<_>>();
                        let key = nator_document::features::signature(&bytes);
                        // A identidade vem do contorno, nunca da ordem dos
                        // cliques: reordenar a seleção não troca a face referida.
                        prism.map_face_names(|name| format!("{name}:area{key:x}"));
                    }
                    result = Some(match result {
                        None => prism,
                        Some(previous) => previous
                            .boolean(
                                &prism,
                                crate::features::boolean_mode(BooleanMode::Join),
                                owner,
                            )
                            .map_err(error)?,
                    });
                }
                result.ok_or_else(|| error("Selecione pelo menos uma área."))?
            }
            Operation::ExtrudeFace { base, face, .. } => {
                let (solid, bytes) = source(doc, part, *base)?;
                solid
                    .extrude_face(face_index(&solid, &bytes, face)?, from, to, owner)
                    .map_err(error)?
            }
            _ => return Err(DocumentError("Perfil de extrusão inválido.".into())),
        };
        if let Some((point, normal)) = limit {
            let start = std::array::from_fn(|i| g.origin[i] + g.normal[i] * from);
            let side = dot(sub(start, point), normal).signum();
            if side == 0. {
                return Err(DocumentError(
                    "A face limite coincide com o início da extrusão.".into(),
                ));
            }
            prism = prism
                .split(point, normal.map(|v| v * side), owner)
                .map_err(error)?;
        }
        Ok(prism)
    }

    pub fn automatic(
        doc: &Document,
        part: PartId,
        mut operation: Operation,
        preferred: Option<BodyId>,
    ) -> Result<Operation, DocumentError> {
        let g = guide(doc, part, &operation)?;
        let step = (g.to - g.from).abs().clamp(0.00001, 0.001) * (g.to - g.from).signum();
        if step == 0. {
            return Err(DocumentError(
                "Extrusão deve ter comprimento não nulo.".into(),
            ));
        }
        let leading = tool(doc, part, &operation, "auto", Some((g.from, g.from + step)))?;
        // Consulta uma lâmina de 0,002 mm ao redor do início. Ela detecta contato
        // tanto saindo de uma face como entrando, sem confundir corpos distantes.
        let probe = tool(
            doc,
            part,
            &operation,
            "probe",
            Some((g.from - 0.001, g.from + 0.001)),
        )?;
        let piece = doc
            .part(part)
            .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
        let source_body = match operation {
            Operation::ExtrudeFace { base, .. } => Some(base),
            _ => None,
        };
        let mut candidates = Vec::new();
        for body in piece.shown_bodies().filter(|b| b.brep.is_some()) {
            let (solid, _) = source(doc, part, body.id)?;
            let contact = solid.common_volume(&probe).map_err(error)?;
            if contact > 1e-9 {
                // A direção inicial distingue entrar de sair. Cruzar outro trecho
                // do mesmo corpo mais adiante não transforma uma saliência em corte.
                let overlap = solid.common_volume(&leading).map_err(error)?;
                let mode = if overlap > 1e-9 {
                    BooleanMode::Cut
                } else {
                    BooleanMode::Join
                };
                candidates.push((
                    source_body == Some(body.id),
                    preferred == Some(body.id),
                    contact,
                    body.id,
                    mode,
                ));
            }
        }
        if candidates.is_empty() && piece.shown_bodies().any(|b| b.brep.is_some()) {
            // Um rascunho afastado também pode entrar no material depois de
            // atravessar espaço livre. Nesse caso o volume alcançado define o corte.
            let prism = tool(doc, part, &operation, "auto", None)?;
            for body in piece.shown_bodies().filter(|b| b.brep.is_some()) {
                let (solid, _) = source(doc, part, body.id)?;
                let overlap = solid.common_volume(&prism).map_err(error)?;
                if overlap > 1e-7 {
                    candidates.push((
                        false,
                        preferred == Some(body.id),
                        overlap,
                        body.id,
                        BooleanMode::Cut,
                    ));
                }
            }
        }
        candidates.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then(b.1.cmp(&a.1))
                .then(b.2.total_cmp(&a.2))
                .then(a.3.0.cmp(&b.3.0))
        });
        let choice = candidates.first().map(|c| Combine {
            target: c.3,
            mode: c.4,
        });
        match &mut operation {
            Operation::Extrude { combine, .. } | Operation::ExtrudeFace { combine, .. } => {
                *combine = choice
            }
            _ => {}
        }
        Ok(operation)
    }

    /// Volume efetivamente retirado, não o prisma inteiro que pode atravessar o vazio.
    pub fn removed(
        doc: &Document,
        part: PartId,
        operation: &Operation,
    ) -> Result<Option<nator_geometry::Mesh>, DocumentError> {
        let Some(combine) = combination(operation).filter(|c| c.mode == BooleanMode::Cut) else {
            return Ok(None);
        };
        let (base, _) = source(doc, part, combine.target)?;
        let prism = tool(doc, part, operation, "removal", None)?;
        if base.common_volume(&prism).map_err(error)? <= 1e-9 {
            return Ok(None);
        }
        let mut common = base.boolean(&prism, 2, "removal").map_err(error)?;
        Ok(Some(common.tessellate(0.05, 0.25).map_err(error)?))
    }
}
pub fn combination(operation: &Operation) -> Option<Combine> {
    match operation {
        Operation::Extrude { combine, .. } | Operation::ExtrudeFace { combine, .. } => *combine,
        _ => None,
    }
}
#[cfg(feature = "occt")]
pub(super) use precise::tool;
#[cfg(feature = "occt")]
pub use precise::{automatic, guide, removed};
#[cfg(not(feature = "occt"))]
pub fn automatic(
    _doc: &Document,
    _part: PartId,
    _operation: Operation,
    _preferred: Option<BodyId>,
) -> Result<Operation, DocumentError> {
    Err(DocumentError("Extrusão requer OCCT.".into()))
}
#[cfg(not(feature = "occt"))]
pub fn guide(
    _doc: &Document,
    _part: PartId,
    _operation: &Operation,
) -> Result<Guide, DocumentError> {
    Err(DocumentError("Extrusão requer OCCT.".into()))
}
#[cfg(not(feature = "occt"))]
pub fn removed(
    _doc: &Document,
    _part: PartId,
    _operation: &Operation,
) -> Result<Option<nator_geometry::Mesh>, DocumentError> {
    Ok(None)
}
