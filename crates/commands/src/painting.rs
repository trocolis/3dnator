//! Seleção do balde de tinta. Células são consultadas no trabalhador a partir
//! dos rascunhos visíveis sobre a face; o clique captura referências, não cores.
use nator_document::{
    BodyId, Document, DocumentError, PaintBoundary, SurfacePaint, features::FaceRef,
};
use nator_geometry::RgbColor;
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Body(BodyId),
    Face {
        body: BodyId,
        face: FaceRef,
        boundaries: Vec<PaintBoundary>,
    },
}
impl Target {
    pub fn body(&self) -> BodyId {
        match self {
            Self::Body(body) | Self::Face { body, .. } => *body,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Picked {
    pub target: Target,
    /// Faces inteiras para retirar uma face de uma seleção feita por duplo clique.
    pub faces: Vec<Target>,
    pub whole_body: bool,
}
#[cfg(feature = "occt")]
fn reference(solid: &nator_kernel::Solid, bytes: &[u8], index: usize) -> FaceRef {
    match solid
        .face_names()
        .get(index)
        .filter(|n| !n.is_empty() && solid.face_names().iter().filter(|v| *v == *n).count() == 1)
    {
        Some(name) => FaceRef::Named(name.clone()),
        // Booleanas podem produzir faces distintas com o mesmo papel/nome. A
        // pintura usa o índice clicado com assinatura, sem adivinhar outra face.
        None => FaceRef::Index {
            index,
            signature: nator_document::features::signature(bytes),
        },
    }
}
/// Condições que isolam a célula clicada de uma face plana: uma por rascunho visível
/// no suporte. Face curva ou sem desenho não tem condições.
#[cfg(feature = "occt")]
fn boundaries_at(
    doc: &Document,
    solid: &nator_kernel::Solid,
    index: usize,
    pick: &nator_document::assembly::Pick,
) -> Result<Vec<PaintBoundary>, DocumentError> {
    use nator_document::{PaintMask, features::RegionRef};
    let mut boundaries = Vec::new();
    if let nator_kernel::FaceKind::Plane { normal, offset } = solid
        .face_measure(index)
        .map_err(|e| DocumentError(e.to_string()))?
        .kind
    {
        let part = doc
            .part(pick.part)
            .ok_or_else(|| DocumentError("Peça da pintura inexistente.".into()))?;
        let point = pick.placement.inverse_apply(pick.point);
        for record in part.sketches.iter().filter(|s| part.shows_sketch(s)) {
            let sketch = &record.sketch;
            // Somente o desenho visível no suporte clicado delimita a célula.
            // Um perfil oculto da extrusão ou um desenho em outro plano não pinta
            // acidentalmente uma superfície situada atrás dele.
            if nator_geometry::norm(nator_geometry::cross(normal, sketch.plane.normal())) > 1e-7
                || (nator_geometry::dot(normal, sketch.plane.origin) - offset).abs() > 1e-6
            {
                continue;
            }
            let regions =
                nator_sketch::areas::regions(sketch).map_err(|e| DocumentError(e.to_string()))?;
            let local = sketch.plane.project(point);
            // Traços abertos dividem a face junto com o contorno dela.
            let closed = regions
                .iter()
                .flat_map(|r| std::iter::once(&r.outer).chain(&r.holes))
                .flat_map(|wire| wire.entities.iter().copied())
                .collect::<std::collections::HashSet<_>>();
            let open = sketch
                .entities
                .iter()
                .filter(|e| !e.construction && !closed.contains(&e.id))
                .map(|e| e.id)
                .collect::<Vec<_>>();
            if !open.is_empty()
                && let Ok(cell) = crate::features::face_cell(sketch, solid, index, local)
                && cell.drawn.iter().any(|(id, _)| open.contains(id))
            {
                boundaries.push(PaintBoundary {
                    mask: PaintMask {
                        sketch: record.id,
                        region: RegionRef::Cell {
                            seed: local.map(|v| (v * 1e6).round() as i64),
                            entities: cell.drawn,
                        },
                    },
                    inside: true,
                });
                continue;
            }
            if regions.is_empty() {
                continue;
            }
            let matched = regions
                .iter()
                .filter(|r| r.outer.contains(local) && !r.holes.iter().any(|h| h.contains(local)))
                .min_by(|a, b| a.area().total_cmp(&b.area()));
            let chosen = matched.map(std::slice::from_ref).unwrap_or(&regions);
            let mut references = chosen
                .iter()
                .map(|r| nator_sketch::areas::signature(sketch, r).map(RegionRef::Area))
                .collect::<nator_sketch::Result<Vec<_>>>()
                .map_err(|e| DocumentError(e.to_string()))?;
            let region = if references.len() == 1 {
                references.remove(0)
            } else {
                RegionRef::Multiple(references)
            };
            boundaries.push(PaintBoundary {
                mask: PaintMask {
                    sketch: record.id,
                    region,
                },
                inside: matched.is_some(),
            });
        }
    }
    Ok(boundaries)
}
#[cfg(feature = "occt")]
pub fn pick(
    doc: &Document,
    pick: &nator_document::assembly::Pick,
    whole_body: bool,
) -> Result<Picked, DocumentError> {
    let error = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let (solid, bytes) = crate::features::source(doc, pick.part, pick.body)?;
    let count = solid.face_count().map_err(error)?;
    let faces = (0..count)
        .map(|index| Target::Face {
            body: pick.body,
            face: reference(&solid, &bytes, index),
            boundaries: vec![],
        })
        .collect::<Vec<_>>();
    if whole_body {
        return Ok(Picked {
            target: Target::Body(pick.body),
            faces,
            whole_body,
        });
    }
    let index = doc.face_index_at(pick)?;
    let face = reference(&solid, &bytes, index);
    let boundaries = boundaries_at(doc, &solid, index, pick)?;
    Ok(Picked {
        target: Target::Face {
            body: pick.body,
            face,
            boundaries,
        },
        faces,
        whole_body,
    })
}
#[cfg(not(feature = "occt"))]
pub fn pick(
    _: &Document,
    _: &nator_document::assembly::Pick,
    _: bool,
) -> Result<Picked, DocumentError> {
    Err(DocumentError(
        "Pintar superfícies exige o núcleo OCCT.".into(),
    ))
}
pub(super) fn apply(
    doc: &mut Document,
    targets: Vec<Target>,
    color: RgbColor,
) -> Result<(), DocumentError> {
    if targets.is_empty() || targets.len() > nator_document::MAX_ELEMENTS {
        return Err(DocumentError("Seleção de superfícies inválida.".into()));
    }
    let mut bodies = targets.iter().map(Target::body).collect::<Vec<_>>();
    bodies.sort_by_key(|b| b.0);
    bodies.dedup();
    for body in bodies {
        if doc.body(body).is_none_or(|b| b.brep.is_none()) {
            return Err(DocumentError("Selecione um corpo CAD para pintar.".into()));
        }
        if targets.contains(&Target::Body(body)) {
            // Corpo inteiro é uma cor uniforme, independentemente de quantas faces
            // tenha. Não consumir uma camada de recorte para cada face do corpo.
            set_body_color(doc, body, color)?;
            continue;
        }
        let mut layers = doc
            .body_appearance(body)
            .map(|a| a.layers.clone())
            .unwrap_or_default();
        for target in targets.iter().filter(|t| t.body() == body) {
            if let Target::Face {
                face, boundaries, ..
            } = target
            {
                layers
                    .retain(|l| l.face != *face || l.mask.is_some() || l.boundaries != *boundaries);
                layers.push(SurfacePaint {
                    face: face.clone(),
                    mask: None,
                    boundaries: boundaries.clone(),
                    color,
                });
            }
        }
        if layers.len() > 255 {
            return Err(DocumentError("Limite de pinturas atingido.".into()));
        }
        let appearance = crate::appearance::build(doc, body, layers)?;
        doc.set_surface_appearance(appearance)?;
    }
    Ok(())
}

/// Um corpo inteiro usa uma cor de base de aparência, inclusive para sobrepor a
/// cor importada de uma ocorrência. Isso não depende do número de faces do CAD.
pub(super) fn set_body_color(
    doc: &mut Document,
    body: BodyId,
    color: RgbColor,
) -> Result<(), DocumentError> {
    let signature = doc
        .body(body)
        .and_then(|b| b.brep.as_ref())
        .map(|b| nator_document::features::signature(b))
        .ok_or_else(|| DocumentError("Selecione um corpo CAD para pintar.".into()))?;
    doc.body_mut(body)?.color = Some(color);
    doc.set_surface_appearance(nator_document::SurfaceAppearance {
        body,
        base_color: Some(color),
        layers: vec![],
        source_signature: signature,
        geometry: None,
        materials: std::sync::Arc::new(vec![]),
    })
}

/// Condições da célula sob o clique, sem listar as faces do corpo como `pick`.
#[cfg(feature = "occt")]
pub fn boundaries(
    doc: &Document,
    pick: &nator_document::assembly::Pick,
) -> Result<Vec<PaintBoundary>, DocumentError> {
    let (solid, _) = crate::features::source(doc, pick.part, pick.body)?;
    boundaries_at(doc, &solid, doc.face_index_at(pick)?, pick)
}
#[cfg(not(feature = "occt"))]
pub fn boundaries(
    _: &Document,
    _: &nator_document::assembly::Pick,
) -> Result<Vec<PaintBoundary>, DocumentError> {
    Ok(Vec::new())
}

/// Malha da célula no referencial local do corpo. Usa o recorte de Pintar, sem
/// publicar comandos nem alterar a aparência.
#[cfg(feature = "occt")]
pub fn cell_mesh(
    doc: &Document,
    body: BodyId,
    face: &FaceRef,
    boundaries: &[PaintBoundary],
) -> Result<nator_geometry::Mesh, DocumentError> {
    let appearance = crate::appearance::build(
        doc,
        body,
        vec![SurfacePaint {
            face: face.clone(),
            boundaries: boundaries.to_vec(),
            mask: None,
            color: RgbColor::NEUTRAL,
        }],
    )?;
    let geometry = appearance
        .geometry
        .ok_or_else(|| DocumentError("Geometria do realce indisponível.".into()))?;
    let mesh = geometry.mesh();
    let triangles = mesh
        .triangles()
        .iter()
        .zip(appearance.materials.iter())
        .filter(|(_, m)| **m == 1)
        .map(|(t, _)| *t)
        .collect();
    nator_geometry::Mesh::new(mesh.vertices().to_vec(), triangles)
        .map_err(|e| DocumentError(e.to_string()))
}
#[cfg(not(feature = "occt"))]
pub fn cell_mesh(
    _: &Document,
    _: BodyId,
    _: &FaceRef,
    _: &[PaintBoundary],
) -> Result<nator_geometry::Mesh, DocumentError> {
    Err(DocumentError("Recortar áreas exige o núcleo OCCT.".into()))
}

/// Malha mundial somente da célula sob o cursor, ou `None` sem delimitação na face.
#[cfg(feature = "occt")]
pub fn highlight(
    doc: &Document,
    pick: &nator_document::assembly::Pick,
) -> Result<Option<nator_geometry::Mesh>, DocumentError> {
    let Target::Face {
        body,
        face,
        boundaries,
    } = self::pick(doc, pick, false)?.target
    else {
        return Ok(None);
    };
    if boundaries.is_empty() {
        return Ok(None);
    }
    let mesh = cell_mesh(doc, body, &face, &boundaries)?;
    let record = doc
        .body(body)
        .ok_or_else(|| DocumentError("Corpo do realce inexistente.".into()))?;
    let pose = pick
        .placement
        .compose(record.placement)
        .map_err(|e| DocumentError(e.to_string()))?;
    let vertices = mesh.vertices().iter().map(|p| pose.apply(*p)).collect();
    nator_geometry::Mesh::new(vertices, mesh.triangles().to_vec())
        .map(Some)
        .map_err(|e| DocumentError(e.to_string()))
}
#[cfg(not(feature = "occt"))]
pub fn highlight(
    _: &Document,
    _: &nator_document::assembly::Pick,
) -> Result<Option<nator_geometry::Mesh>, DocumentError> {
    Ok(None)
}
