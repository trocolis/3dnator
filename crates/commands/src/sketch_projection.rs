//! Apoios do modelo projetados ortogonalmente no plano local de um rascunho.
//! Consultar no trabalhador: os apoios CAD são exatos e não uma amostra da tesselação.
use crate::point_snap::{body_anchors, reference_points};
use nator_document::{Document, DocumentError, SketchId, assembly::Occurrence};
use nator_geometry::Placement;
use nator_sketch::{Plane, Point2};
use std::collections::HashMap;

/// O chamador fornece as ocorrências permitidas pelo isolamento/bloqueio da interface.
/// Visibilidade de corpos e rascunhos continua sendo consultada no documento. O plano
/// está no referencial da peça editada; `target` posiciona esse referencial na montagem.
/// Nenhum desses apoios cria um vínculo paramétrico nem é persistido como índice CAD.
pub fn points(
    document: &Document,
    occurrences: &[Occurrence],
    edited: SketchId,
    plane: Plane,
    target: Placement,
) -> Result<Vec<Point2>, DocumentError> {
    plane.validate().map_err(|e| DocumentError(e.to_string()))?;
    let mut result = vec![[0., 0.]];
    let mut bodies = HashMap::new();
    let mut sketches = HashMap::new();
    let project =
        |point, source: Placement| plane.project(target.inverse_apply(source.apply(point)));
    for occurrence in occurrences {
        let part = document
            .instance(occurrence.instance)
            .and_then(|i| document.part(i.part))
            .ok_or_else(|| DocumentError("Ocorrência de projeção inexistente.".into()))?;
        for body in part.shown_bodies() {
            // Uma definição pode aparecer muitas vezes na montagem. Consultar a
            // geometria uma vez e compor cada ocorrência mantém os apoios exatos.
            if let std::collections::hash_map::Entry::Vacant(entry) = bodies.entry(body.id) {
                entry.insert(body_anchors(body)?);
            }
            let source = occurrence
                .placement
                .compose(body.placement)
                .map_err(|e| DocumentError(e.to_string()))?;
            if let Some(anchors) = bodies.get(&body.id) {
                result.extend(anchors.iter().map(|a| project(a.point, source)));
            }
        }
        for record in part
            .sketches
            .iter()
            .filter(|s| s.id != edited && part.shows_sketch(s))
        {
            if let std::collections::hash_map::Entry::Vacant(entry) = sketches.entry(record.id) {
                entry.insert(reference_points(&record.sketch)?);
            }
            if let Some(anchors) = sketches.get(&record.id) {
                result.extend(
                    anchors
                        .iter()
                        .map(|p| project(record.sketch.plane.world(*p), occurrence.placement)),
                );
            }
        }
    }
    for record in document
        .reference_sketches()
        .iter()
        .filter(|s| s.id != edited && document.shows_reference_sketch(s))
    {
        result.extend(
            reference_points(&record.sketch)?
                .into_iter()
                .map(|p| project(record.sketch.plane.world(p), Placement::default())),
        );
    }
    if result.iter().flatten().any(|v| !v.is_finite()) {
        return Err(DocumentError(
            "Ponto projetado fora do intervalo numérico.".into(),
        ));
    }
    // Faces em alturas distintas podem produzir o mesmo apoio em um plano paralelo.
    // A ordem não é identidade topológica: só elimina duplicações numéricas na captura.
    result.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    result.dedup_by(|a, b| nator_sketch::distance(*a, *b) < 1e-9);
    Ok(result)
}
