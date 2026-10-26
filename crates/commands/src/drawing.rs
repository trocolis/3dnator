//! Coordenação da folha técnica: sólidos usam HLR no núcleo; contextos com malhas
//! conservam o método aproximado identificado na legenda. O cálculo ocorre no worker.
use nator_document::{Document, DocumentError, PartId};
use nator_drawing::{Sheet, Specification};
pub fn generate(
    document: &Document,
    target: Option<PartId>,
    spec: &Specification,
) -> Result<Sheet, DocumentError> {
    spec.validate().map_err(|e| DocumentError(e.to_string()))?;
    let context = document.drawing_context(target)?;
    let mesh = context
        .scene_mesh()?
        .ok_or_else(|| DocumentError("Não há corpos visíveis para gerar a folha.".into()))?;
    let notes = document.drawing_notes(target);
    #[cfg(feature = "occt")]
    {
        use nator_drawing::{ProjectedView, View};
        let mut solids = Vec::new();
        let mut mesh_body = false;
        let mut faces = 0;
        for occurrence in context.occurrences()? {
            let instance = context
                .instance(occurrence.instance)
                .ok_or_else(|| DocumentError("Instância ausente.".into()))?;
            let part = context
                .part(instance.part)
                .ok_or_else(|| DocumentError("Peça ausente.".into()))?;
            for body in part.bodies.iter().filter(|b| b.visible) {
                if let Some(bytes) = &body.brep {
                    let placement = occurrence
                        .placement
                        .compose(body.placement)
                        .map_err(|e| DocumentError(e.to_string()))?;
                    let solid = nator_kernel::Solid::from_brep(bytes, None)
                        .and_then(|s| s.transformed(placement))
                        .map_err(|e| DocumentError(e.to_string()))?;
                    faces += solid
                        .face_count()
                        .map_err(|e| DocumentError(e.to_string()))?;
                    if faces > 20000 {
                        return Err(DocumentError(
                            "A folha excede vinte mil faces para projeção precisa.".into(),
                        ));
                    }
                    solids.push(solid);
                } else {
                    mesh_body = true;
                }
            }
        }
        if !mesh_body && !solids.is_empty() {
            let views = if spec.section.is_some() {
                &[View::Front, View::Right, View::Top][..]
            } else {
                &[View::Front, View::Right, View::Top, View::Isometric][..]
            };
            let mut projections = Vec::new();
            for view in views {
                let (right, _, direction) = view.basis();
                let segments = nator_kernel::projected_edges(&solids, right, direction, 0.01)
                    .map_err(|e| DocumentError(e.to_string()))?;
                projections.push(ProjectedView {
                    view: *view,
                    segments,
                });
            }
            return nator_drawing::generate_with_projections(
                &mesh,
                spec,
                &notes,
                Some(&projections),
            )
            .map_err(|e| DocumentError(e.to_string()));
        }
    }
    nator_drawing::generate_with_notes(&mesh, spec, &notes)
        .map_err(|e| DocumentError(e.to_string()))
}
