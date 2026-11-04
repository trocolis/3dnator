//! Contorno das letras para o Texto do rascunho: o núcleo desenha o texto em tamanho 1,
//! centrado na origem, e cada aresta vira uma polilinha no plano. O rascunho guarda essas
//! polilinhas e as recoloca sozinho, sem voltar ao núcleo.
use nator_document::DocumentError;

/// Desvio máximo da polilinha, em unidades da fonte de tamanho 1 (0,02 mm num texto de
/// 10 mm): fino para a impressão, com poucos pontos por letra.
#[cfg(feature = "occt")]
const TOLERANCE: f64 = 0.002;

#[cfg(feature = "occt")]
pub fn contours(
    content: &str,
    font: &str,
    bold: bool,
    italic: bool,
    underline: bool,
) -> Result<Vec<Vec<[f64; 2]>>, DocumentError> {
    let error = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let bytes =
        nator_kernel::Solid::text_outline(content, font, bold, italic, underline).map_err(error)?;
    let outline = nator_kernel::Solid::from_brep(&bytes, None).map_err(error)?;
    let mut result = Vec::new();
    for edge in 0..outline.edge_count().map_err(error)? {
        let points = outline
            .edge_polyline(edge, TOLERANCE)
            .map_err(error)?
            .into_iter()
            .map(|p| [p[0], p[1]])
            .collect::<Vec<_>>();
        if points.len() >= 2 {
            result.push(points);
        }
    }
    if result.is_empty() {
        return Err(DocumentError(
            "O texto não produziu contornos nesta fonte.".into(),
        ));
    }
    Ok(result)
}

#[cfg(not(feature = "occt"))]
pub fn contours(
    _: &str,
    _: &str,
    _: bool,
    _: bool,
    _: bool,
) -> Result<Vec<Vec<[f64; 2]>>, DocumentError> {
    nator_kernel::require_available().map_err(|e| DocumentError(e.to_string()))?;
    Err(DocumentError(
        "Texto no rascunho exige o núcleo OCCT.".into(),
    ))
}

/// Prepara o bloco completo sem publicar no documento; posição solicitada não pode recuar.
pub fn candidate(
    source: &nator_sketch::Sketch,
    mut block: nator_sketch::text::SketchText,
    position: [f64; 2],
    attachment: Option<nator_sketch::snapping::Kind>,
    parameters: &nator_document::parameters::Parameters,
) -> Result<nator_sketch::Sketch, DocumentError> {
    let error = |e: nator_sketch::SketchError| DocumentError(e.to_string());
    let mut sketch = source.clone();
    let anchor = if block.id != 0 {
        let anchor = sketch
            .texts
            .iter()
            .find(|t| t.id == block.id)
            .ok_or_else(|| DocumentError("O texto em edição não existe mais.".into()))?
            .anchor;
        sketch
            .points
            .iter_mut()
            .find(|p| p.id == anchor)
            .ok_or_else(|| DocumentError("O ponto de inserção do texto não existe mais.".into()))?
            .position = position;
        block.anchor = anchor;
        sketch.update_text(block).map_err(error)?;
        anchor
    } else {
        let anchor = sketch.add_point(position).map_err(error)?;
        block.anchor = anchor;
        sketch.add_text(block).map_err(error)?;
        nator_sketch::junctions::attach(&mut sketch, anchor, attachment, position)
            .map_err(error)?;
        anchor
    };
    nator_document::parameters::resolve_sketch(&mut sketch, parameters)?;
    let report = nator_sketch::solver::solve_required(&sketch, &[anchor], &[]).map_err(error)?;
    if report.status == nator_sketch::solver::Status::Conflict {
        return Err(DocumentError(
            "A posição do texto conflita com as restrições do rascunho.".into(),
        ));
    }
    Ok(report.sketch)
}
