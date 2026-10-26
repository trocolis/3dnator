//! Associação das cotas por identidade topológica e caminho de ocorrência. Referências
//! perdidas permanecem como avisos na folha; nenhuma busca por proximidade as substitui.
use nator_document::{
    Document, DocumentError, PartId,
    assembly::{Occurrence, Pick},
    drawing::*,
};
use nator_drawing::{AnnotationGeometry, ResolvedAnnotation, Sheet, Specification};
#[cfg(feature = "occt")]
use nator_geometry::Placement;

pub fn generate_with_annotations(
    document: &Document,
    target: Option<PartId>,
    spec: &Specification,
    annotations: &[DrawingAnnotation],
) -> Result<Sheet, DocumentError> {
    nator_document::drawing::validate(annotations, document.next_id())?;
    let mut sheet = super::generate(document, target, spec)?;
    let resolved = annotations
        .iter()
        .map(|a| ResolvedAnnotation {
            id: a.id,
            view: a.view,
            offset_mm: a.offset_mm,
            geometry: resolve(document, target, &a.measurement).unwrap_or_else(|e| {
                AnnotationGeometry::Missing {
                    message: e.to_string(),
                }
            }),
        })
        .collect();
    sheet
        .set_annotations(resolved)
        .map_err(|e| DocumentError(e.to_string()))?;
    Ok(sheet)
}

/// A seleção fornece a identidade visual; o caminho distingue ocorrências de uma mesma
/// definição. Só uma ocorrência compatível e selecionável pode originar a referência.
pub fn capture_reference(
    document: &Document,
    target: Option<PartId>,
    pick: &Pick,
    edge: bool,
    allowed: impl Fn(&Occurrence) -> bool,
) -> Result<DrawingTopologyRef, DocumentError> {
    let mut paths = document.occurrences()?.into_iter().filter(|o| {
        o.instance == pick.instance
            && allowed(o)
            && o.placement
                .affine_rows()
                .iter()
                .zip(pick.placement.affine_rows())
                .all(|(a, b)| (a - b).abs() < 1e-8)
    });
    let mut path = paths
        .next()
        .ok_or_else(|| {
            DocumentError("A seleção da cota ficou desatualizada; selecione novamente.".into())
        })?
        .path;
    if paths.next().is_some() {
        return Err(DocumentError(
            "A seleção é ambígua entre ocorrências. Isole uma ocorrência e selecione novamente."
                .into(),
        ));
    }
    if let Some(target) = target {
        let index = path
            .iter()
            .position(|id| document.instance(*id).is_some_and(|i| i.part == target))
            .ok_or_else(|| {
                DocumentError("Selecione geometria dentro da peça associada à folha.".into())
            })?;
        path = path[index + 1..].to_vec();
    }
    let target = DrawingBodyRef {
        body: pick.body,
        path,
    };
    if edge {
        Ok(DrawingTopologyRef::Edge(DrawingEdgeRef {
            target,
            edge: document.edge_at(pick)?,
        }))
    } else {
        Ok(DrawingTopologyRef::Face(DrawingFaceRef {
            target,
            face: document.face_at(pick)?,
        }))
    }
}

#[cfg(not(feature = "occt"))]
fn resolve(
    _doc: &Document,
    _target: Option<PartId>,
    _measurement: &DrawingMeasure,
) -> Result<AnnotationGeometry, DocumentError> {
    nator_kernel::require_available().map_err(|e| DocumentError(e.to_string()))?;
    Err(DocumentError("A cota precisa requer OCCT.".into()))
}
#[cfg(feature = "occt")]
fn resolve(
    doc: &Document,
    target: Option<PartId>,
    measurement: &DrawingMeasure,
) -> Result<AnnotationGeometry, DocumentError> {
    use nator_document::features::{EdgeRef, FaceRef};
    use nator_geometry::{Point3, cross, dot, norm, sub};
    use nator_kernel::{FaceKind, Solid};
    let error = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let load = |reference: &DrawingBodyRef| -> Result<Solid, DocumentError> {
        let mut parent = target;
        let mut placement = Placement::default();
        for id in &reference.path {
            let i = doc
                .instance(*id)
                .filter(|i| i.parent == parent && i.visible)
                .ok_or_else(|| {
                    DocumentError("Referência perdida ou oculta no caminho da cota.".into())
                })?;
            placement = placement
                .compose(i.placement)
                .map_err(|e| DocumentError(e.to_string()))?;
            parent = Some(i.part);
        }
        let body=parent.and_then(|p|doc.part(p)).and_then(|p|p.bodies.iter().find(|b|b.id==reference.body && b.visible)).ok_or_else(||DocumentError("O corpo da cota foi removido, ocultado ou não pertence mais ao caminho escolhido.".into()))?;
        let brep = body.brep.as_ref().ok_or_else(|| {
            DocumentError("A cota associativa requer um corpo sólido CAD.".into())
        })?;
        let placement = placement
            .compose(body.placement)
            .map_err(|e| DocumentError(e.to_string()))?;
        Solid::from_brep(brep, Some(&body.face_names))
            .and_then(|s| s.transformed(placement))
            .map_err(error)
    };
    let face = |r: &DrawingFaceRef| -> Result<(Solid, usize), DocumentError> {
        let s = load(&r.target)?;
        let FaceRef::Named(name) = &r.face else {
            return Err(DocumentError(
                "A cota não possui uma referência estável de face.".into(),
            ));
        };
        let index = s.face_named(name).map_err(error)?;
        Ok((s, index))
    };
    let edge = |r: &DrawingEdgeRef| -> Result<(Solid, usize), DocumentError> {
        let s = load(&r.target)?;
        let EdgeRef::Between(names) = &r.edge else {
            return Err(DocumentError(
                "A cota não possui uma referência estável de aresta.".into(),
            ));
        };
        let index = s.edge_between(names).map_err(error)?;
        Ok((s, index))
    };
    match measurement {
        DrawingMeasure::EdgeLength(r) => {
            let (s, i) = edge(r)?;
            let measure = s.edge_measure(i).map_err(error)?;
            let anchors = s.edge_anchors(i).map_err(error)?;
            if measure.straight {
                Ok(AnnotationGeometry::Linear {
                    a: anchors.start,
                    b: anchors.end,
                    value_mm: measure.length,
                })
            } else {
                Ok(AnnotationGeometry::Leader {
                    at: anchors.start,
                    label: format!("L {}", nator_drawing::format_mm(measure.length)),
                })
            }
        }
        DrawingMeasure::Radius { topology, diameter } => {
            let (center, point, radius) = match topology {
                DrawingTopologyRef::Face(r) => {
                    let (s, i) = face(r)?;
                    let measure = s.face_measure(i).map_err(error)?;
                    let radius = match measure.kind {
                        FaceKind::Cylinder { radius } | FaceKind::Sphere { radius } => radius,
                        _ => {
                            return Err(DocumentError(
                                "Escolha uma face cilíndrica ou esférica para a cota de raio."
                                    .into(),
                            ));
                        }
                    };
                    let [center, point] =
                        s.face_anchors(i).map_err(error)?.radial.ok_or_else(|| {
                            DocumentError("A face não fornece apoio radial.".into())
                        })?;
                    (center, point, radius)
                }
                DrawingTopologyRef::Edge(r) => {
                    let (s, i) = edge(r)?;
                    let measure = s.edge_measure(i).map_err(error)?;
                    let anchors = s.edge_anchors(i).map_err(error)?;
                    (
                        anchors.circle_center.ok_or_else(|| {
                            DocumentError("Escolha uma aresta circular para a cota de raio.".into())
                        })?,
                        anchors.start,
                        measure.radius.ok_or_else(|| {
                            DocumentError("A aresta não possui raio circular.".into())
                        })?,
                    )
                }
            };
            Ok(AnnotationGeometry::Radius {
                center,
                point,
                radius_mm: radius,
                diameter: *diameter,
            })
        }
        DrawingMeasure::FaceDistance { first, second }
        | DrawingMeasure::FaceAngle { first, second, .. } => {
            let (a, ia) = face(first)?;
            let (b, ib) = face(second)?;
            let FaceKind::Plane {
                normal: na,
                offset: da,
            } = a.face_measure(ia).map_err(error)?.kind
            else {
                return Err(DocumentError(
                    "A cota entre faces requer faces planas.".into(),
                ));
            };
            let FaceKind::Plane {
                normal: nb,
                offset: db,
            } = b.face_measure(ib).map_err(error)?.kind
            else {
                return Err(DocumentError(
                    "A cota entre faces requer faces planas.".into(),
                ));
            };
            let pa = a.face_anchors(ia).map_err(error)?.centroid;
            let pb = b.face_anchors(ib).map_err(error)?.centroid;
            let cosine = dot(na, nb).clamp(-1., 1.);
            if matches!(measurement, DrawingMeasure::FaceDistance { .. }) {
                if 1. - cosine.abs() > 1e-8 {
                    return Err(DocumentError(
                        "As faces não são paralelas; use uma cota angular.".into(),
                    ));
                }
                let projected = std::array::from_fn(|i| pa[i] + nb[i] * (db - dot(pa, nb)));
                return Ok(AnnotationGeometry::Linear {
                    a: pa,
                    b: projected,
                    value_mm: norm(sub(projected, pa)),
                });
            }
            let DrawingMeasure::FaceAngle { supplementary, .. } = measurement else {
                return Err(DocumentError("Tipo de cota angular inválido.".into()));
            };
            let angle = cosine.abs().acos();
            let angle = if *supplementary {
                std::f64::consts::PI - angle
            } else {
                angle
            };
            let middle: Point3 = std::array::from_fn(|i| pa[i] * 0.5 + pb[i] * 0.5);
            let determinant = 1. - cosine * cosine;
            if determinant < 1e-12 {
                return Ok(AnnotationGeometry::Leader {
                    at: middle,
                    label: format!("{}°", nator_drawing::format_mm(angle.to_degrees())),
                });
            }
            // Interseção dos planos mais próxima do ponto médio dos apoios. A solução
            // fica no espaço gerado pelas normais e preserva posições de montagem.
            let ra = da - dot(middle, na);
            let rb = db - dot(middle, nb);
            let lambda = (ra - cosine * rb) / determinant;
            let mu = (rb - cosine * ra) / determinant;
            let center: Point3 = std::array::from_fn(|i| middle[i] + lambda * na[i] + mu * nb[i]);
            // Planos quase paralelos podem se encontrar muito longe das peças.
            // Nesse caso o valor permanece útil, mas um arco fora da folha não.
            if norm(sub(center, middle)) > norm(sub(pa, pb)).max(1.) * 100. {
                return Ok(AnnotationGeometry::Leader {
                    at: middle,
                    label: format!("{}°", nator_drawing::format_mm(angle.to_degrees())),
                });
            }
            let axis = cross(na, nb);
            let axis = axis.map(|v| v / norm(axis));
            let mut first = cross(axis, na);
            let mut second = cross(axis, nb);
            if dot(first, sub(pa, center)) < 0. {
                first = first.map(|v| -v);
            }
            if (dot(first, second) < 0.) != *supplementary {
                second = second.map(|v| -v);
            }
            Ok(AnnotationGeometry::Angle {
                center,
                first,
                second,
                angle_rad: angle,
            })
        }
    }
}
