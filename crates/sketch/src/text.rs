//! Texto no rascunho: um bloco editável (conteúdo, fonte, estilo, tamanho e rotação)
//! preso a um ponto de inserção comum, que aceita restrições e cotas. As letras viram
//! linhas fixas, como as projeções: delimitam áreas e servem de captura, mas não se
//! editam uma a uma. Mudar o ponto, o tamanho ou a rotação recoloca as mesmas linhas,
//! com os mesmos IDs, para que as áreas escolhidas continuem válidas.
//!
//! O contorno das letras vem do núcleo, em tamanho 1 e centrado na origem; o rascunho o
//! guarda para não depender do núcleo a cada recolocação.
use crate::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SketchText {
    pub id: u64,
    /// Ponto de inserção (centro do texto).
    pub anchor: u64,
    pub content: String,
    pub font: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub underline: bool,
    /// Altura da fonte, em mm.
    pub size: f64,
    /// Rotação em radianos, anti-horária a partir do eixo X do rascunho.
    pub rotation: f64,
    /// Polilinhas do contorno unitário (fonte de tamanho 1, centrada na origem).
    pub contours: Vec<Vec<Point2>>,
    /// Pontos e linhas gerados, na ordem das polilinhas.
    #[serde(default)]
    pub points: Vec<u64>,
    #[serde(default)]
    pub entities: Vec<u64>,
}

/// Limite de pontos de um contorno de texto, para manter o rascunho leve.
const MAX_TEXT_POINTS: usize = 3000;
/// Folga, na fonte de tamanho 1, para considerar a mesma a ponta de dois trechos.
const JOIN_TOLERANCE: f64 = 1e-6;

/// Junta os trechos do contorno das letras em polilinhas contínuas. O núcleo entrega uma
/// polilinha por aresta da fonte; sem a junção, cada emenda dentro de uma letra vira um
/// par de pontas soltas sobrepostas. Uma polilinha que volta ao início termina no mesmo
/// ponto em que começou, para ser lida como fechada. Pontos repetidos em sequência saem.
pub fn join_contours(pieces: &[Vec<Point2>]) -> Vec<Vec<Point2>> {
    let near = |a: Point2, b: Point2| distance(a, b) < JOIN_TOLERANCE;
    let mut used = vec![false; pieces.len()];
    let mut result = Vec::new();
    for first in 0..pieces.len() {
        if used[first] || pieces[first].is_empty() {
            continue;
        }
        used[first] = true;
        let mut chain = pieces[first].clone();
        while let (Some(&start), Some(&end)) = (chain.first(), chain.last()) {
            if chain.len() > 2 && near(start, end) {
                break;
            }
            let next = (0..pieces.len()).find_map(|i| {
                let piece = &pieces[i];
                let (a, b) = (*piece.first()?, *piece.last()?);
                (!used[i]).then_some(())?;
                if near(a, end) {
                    Some((i, false))
                } else if near(b, end) {
                    Some((i, true))
                } else {
                    None
                }
            });
            let Some((index, reversed)) = next else {
                break;
            };
            used[index] = true;
            let mut piece = pieces[index].clone();
            if reversed {
                piece.reverse();
            }
            chain.extend(piece.into_iter().skip(1));
        }
        let mut clean: Vec<Point2> = Vec::with_capacity(chain.len());
        for point in chain {
            if clean.last().is_none_or(|last| !near(*last, point)) {
                clean.push(point);
            }
        }
        // Fechada: a última ponta coincide exatamente com a primeira.
        if clean.len() > 2
            && let Some(&start) = clean.first()
            && let Some(end) = clean.last_mut()
            && near(start, *end)
        {
            *end = start;
        }
        if clean.len() >= 2 {
            result.push(clean);
        }
    }
    result
}

impl SketchText {
    fn position(&self, anchor: Point2, p: Point2) -> Point2 {
        let (sin, cos) = self.rotation.sin_cos();
        let (x, y) = (p[0] * self.size, p[1] * self.size);
        [anchor[0] + x * cos - y * sin, anchor[1] + x * sin + y * cos]
    }
    /// Pontos de cada polilinha; uma polilinha fechada reaproveita o primeiro ponto.
    fn layout(&self) -> Vec<(usize, bool)> {
        self.contours
            .iter()
            .map(|c| {
                let closed = c.len() > 2
                    && c.first()
                        .zip(c.last())
                        .is_some_and(|(a, b)| distance(*a, *b) < 1e-9);
                (if closed { c.len() - 1 } else { c.len() }, closed)
            })
            .collect()
    }
    fn validate(&self) -> Result<()> {
        let count = self.contours.iter().map(Vec::len).sum::<usize>();
        if self.content.trim().is_empty()
            || self.content.chars().count() > 128
            || self.font.len() > 256
            || !(self.size.is_finite() && self.size > 1e-6 && self.size < 1e6)
            || !self.rotation.is_finite()
            || count == 0
            || count > MAX_TEXT_POINTS
            || self.contours.iter().any(|c| c.len() < 2)
            || self
                .contours
                .iter()
                .flatten()
                .any(|p| p.iter().any(|v| !v.is_finite() || v.abs() > 1e6))
        {
            return Err(SketchError("Texto do rascunho inválido.".into()));
        }
        Ok(())
    }
}

impl Sketch {
    /// Bloco de texto dono da entidade ou do ponto, se houver.
    pub fn text_of(&self, id: u64) -> Option<&SketchText> {
        self.texts
            .iter()
            .find(|t| t.entities.contains(&id) || t.points.contains(&id))
    }
    /// Cria um bloco de texto no ponto de inserção `anchor` (um ponto comum do rascunho).
    pub fn add_text(&mut self, mut text: SketchText) -> Result<u64> {
        text.contours = join_contours(&text.contours);
        text.validate()?;
        self.point(text.anchor)?;
        text.id = self.allocate()?;
        text.points.clear();
        text.entities.clear();
        let id = text.id;
        self.texts.push(text);
        place(self, id)?;
        Ok(id)
    }
    /// Troca conteúdo, fonte, estilo, tamanho ou rotação de um bloco existente.
    pub fn update_text(&mut self, mut text: SketchText) -> Result<()> {
        text.contours = join_contours(&text.contours);
        text.validate()?;
        let index = self
            .texts
            .iter()
            .position(|t| t.id == text.id)
            .ok_or_else(|| SketchError("Texto inexistente.".into()))?;
        let old = &self.texts[index];
        text.points = old.points.clone();
        text.entities = old.entities.clone();
        self.texts[index] = text;
        place(self, self.texts[index].id)
    }
    /// Apaga o bloco inteiro e as linhas geradas; o ponto de inserção também sai, se nada
    /// mais o usa.
    pub fn remove_text(&mut self, id: u64) -> Result<()> {
        let index = self
            .texts
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| SketchError("Texto inexistente.".into()))?;
        let text = self.texts.remove(index);
        clear(self, &text);
        let anchor = text.anchor;
        let used = self
            .entities
            .iter()
            .any(|e| e.point_ids().contains(&anchor))
            || self.texts.iter().any(|t| t.anchor == anchor)
            || self.constraints.iter().any(|c| uses(&c.kind, anchor));
        if !used {
            self.points.retain(|p| p.id != anchor);
        }
        Ok(())
    }
}

fn uses(kind: &ConstraintKind, point: u64) -> bool {
    match *kind {
        ConstraintKind::Coincident { a, b }
        | ConstraintKind::Distance { a, b, .. }
        | ConstraintKind::DeltaX { a, b, .. }
        | ConstraintKind::DeltaY { a, b, .. }
        | ConstraintKind::Angle { a, b, .. }
        | ConstraintKind::PointsHorizontal { a, b }
        | ConstraintKind::PointsVertical { a, b }
        | ConstraintKind::Symmetric { a, b, .. } => a == point || b == point,
        ConstraintKind::Fixed { point: p, .. }
        | ConstraintKind::PointOnLine { point: p, .. }
        | ConstraintKind::PointOnCurve { point: p, .. }
        | ConstraintKind::Midpoint { point: p, .. }
        | ConstraintKind::OriginDistance { point: p, .. }
        | ConstraintKind::PointLineDistance { point: p, .. }
        | ConstraintKind::Tangent { point: p, .. } => p == point,
        _ => false,
    }
}

/// A restrição cita a entidade `id` (linha, arco ou círculo).
fn cites(kind: &ConstraintKind, id: u64) -> bool {
    match *kind {
        ConstraintKind::Tangent { line, arc, .. } => line == id || arc == id,
        ConstraintKind::Horizontal { line }
        | ConstraintKind::Vertical { line }
        | ConstraintKind::Midpoint { line, .. }
        | ConstraintKind::PointOnLine { line, .. }
        | ConstraintKind::OriginLineDistance { line, .. }
        | ConstraintKind::PointLineDistance { line, .. } => line == id,
        ConstraintKind::Radius { circle, .. } | ConstraintKind::Diameter { circle, .. } => {
            circle == id
        }
        ConstraintKind::LineAngle {
            line, reference, ..
        } => line == id || reference == id,
        ConstraintKind::LineCircleDistance { line, circle, .. } => line == id || circle == id,
        ConstraintKind::Parallel { a, b }
        | ConstraintKind::Perpendicular { a, b }
        | ConstraintKind::EqualLength { a, b }
        | ConstraintKind::Concentric { a, b }
        | ConstraintKind::Collinear { a, b }
        | ConstraintKind::EqualRadius { a, b }
        | ConstraintKind::CircleTangent { a, b, .. } => a == id || b == id,
        ConstraintKind::PointOnCurve { curve, .. } | ConstraintKind::OriginOnCurve { curve } => {
            curve == id
        }
        ConstraintKind::Symmetric { axis, .. } => axis == id,
        _ => false,
    }
}

fn clear(sketch: &mut Sketch, text: &SketchText) {
    sketch.entities.retain(|e| !text.entities.contains(&e.id));
    sketch.points.retain(|p| !text.points.contains(&p.id));
    sketch.constraints.retain(|c| {
        !text.points.iter().any(|p| uses(&c.kind, *p))
            && !text.entities.iter().any(|e| cites(&c.kind, *e))
    });
    let kept = sketch
        .constraints
        .iter()
        .map(|c| c.id)
        .collect::<HashSet<_>>();
    sketch
        .dimension_expressions
        .retain(|id, _| kept.contains(id));
    sketch.dimension_labels.retain(|id, _| kept.contains(id));
    sketch.dimension_targets.retain(|id, _| kept.contains(id));
}

/// Recoloca as linhas de todos os textos no seu ponto de inserção.
pub fn place_all(sketch: &mut Sketch) -> Result<()> {
    let ids = sketch.texts.iter().map(|t| t.id).collect::<Vec<_>>();
    for id in ids {
        place(sketch, id)?;
    }
    Ok(())
}

fn place(sketch: &mut Sketch, id: u64) -> Result<()> {
    let text = sketch
        .texts
        .iter()
        .find(|t| t.id == id)
        .cloned()
        .ok_or_else(|| SketchError("Texto inexistente.".into()))?;
    let anchor = sketch.point(text.anchor)?;
    let layout = text.layout();
    let points = layout.iter().map(|(n, _)| n).sum::<usize>();
    let positions = text
        .contours
        .iter()
        .zip(&layout)
        .flat_map(|(c, (n, _))| c[..*n].iter().map(|p| text.position(anchor, *p)))
        .collect::<Vec<_>>();
    // Mesma regra da criação: trechos menores que um nanômetro não viram linhas.
    let mut entities = 0;
    let mut start = 0;
    for (n, closed) in &layout {
        let run = &positions[start..start + n];
        entities += run
            .windows(2)
            .map(|w| (w[0], w[1]))
            .chain(closed.then(|| (run[n - 1], run[0])))
            .filter(|(a, b)| distance(*a, *b) >= 1e-9)
            .count();
        start += n;
    }
    // Mesmo desenho: só as posições mudam, e os IDs ficam.
    if text.points.len() == points && text.entities.len() == entities {
        for (id, position) in text.points.iter().zip(&positions) {
            if let Some(point) = sketch.points.iter_mut().find(|p| p.id == *id) {
                point.position = *position;
            }
        }
        return Ok(());
    }
    clear(sketch, &text);
    let mut point_ids = Vec::with_capacity(points);
    let mut entity_ids = Vec::with_capacity(entities);
    let mut cursor = 0;
    for (n, closed) in layout {
        let ids = (0..n)
            .map(|i| {
                let id = sketch.add_point(positions[cursor + i])?;
                if let Some(p) = sketch.points.iter_mut().find(|p| p.id == id) {
                    p.projected = true;
                }
                Ok(id)
            })
            .collect::<Result<Vec<_>>>()?;
        cursor += n;
        let pairs = ids
            .windows(2)
            .map(|w| (w[0], w[1]))
            .chain(closed.then(|| (ids[n - 1], ids[0])))
            .collect::<Vec<_>>();
        for (start, end) in pairs {
            // Trechos menores que um nanômetro não são linhas: o vizinho já cobre o ponto.
            if distance(sketch.point(start)?, sketch.point(end)?) < 1e-9 {
                continue;
            }
            let line = sketch.add_curve(Curve::Line { start, end }, false)?;
            if let Some(e) = sketch.entities.iter_mut().find(|e| e.id == line) {
                e.projected = true;
            }
            entity_ids.push(line);
        }
        point_ids.extend(ids);
    }
    if let Some(stored) = sketch.texts.iter_mut().find(|t| t.id == id) {
        stored.points = point_ids;
        stored.entities = entity_ids;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Quadrado unitário e um traço aberto, como letras simples.
    fn block(anchor: u64) -> SketchText {
        SketchText {
            id: 0,
            anchor,
            content: "IO".into(),
            font: "sans-serif".into(),
            bold: false,
            italic: false,
            underline: false,
            size: 10.,
            rotation: 0.,
            contours: vec![
                vec![
                    [-0.5, -0.5],
                    [0.5, -0.5],
                    [0.5, 0.5],
                    [-0.5, 0.5],
                    [-0.5, -0.5],
                ],
                vec![[1., -0.5], [1., 0.5]],
            ],
            points: Vec::new(),
            entities: Vec::new(),
        }
    }

    #[test]
    fn letter_pieces_join_into_closed_contours_without_open_ends() -> Result<()> {
        // Quadrado em quatro trechos, um deles invertido, como as arestas de uma fonte, e
        // um traço aberto que continua aberto.
        let pieces = vec![
            vec![[-0.5, -0.5], [0., -0.5], [0.5, -0.5]],
            vec![[0.5, 0.5], [0.5, -0.5]],
            vec![[0.5, 0.5], [-0.5, 0.5]],
            vec![[-0.5, 0.5], [-0.5, -0.5 + 1e-8]],
            vec![[1., -0.5], [1., 0.5]],
        ];
        let joined = join_contours(&pieces);
        assert_eq!(joined.len(), 2);
        assert_eq!(joined[0].len(), 6);
        assert_eq!(joined[0].first(), joined[0].last());
        assert_eq!(joined[1], pieces[4]);
        let mut sketch = Sketch::new("Texto", Plane::xy());
        let anchor = sketch.add_point([0., 0.])?;
        let mut text = block(anchor);
        text.contours = pieces;
        sketch.add_text(text)?;
        assert_eq!(areas::regions(&sketch)?.len(), 1);
        // Nenhuma emenda sobra como ponta solta para ligar.
        let report = crate::repair::suggest(&sketch, 0.)?;
        assert!(report.fixes.is_empty(), "{:?}", report.fixes);
        Ok(())
    }

    #[test]
    fn fixed_text_ends_are_not_offered_for_linking() -> Result<()> {
        // Projetos gravados antes da junção têm um trecho por aresta: as emendas são
        // pontas fixas sobrepostas e não viram alerta.
        let mut sketch = Sketch::new("Texto", Plane::xy());
        let anchor = sketch.add_point([0., 0.])?;
        sketch.add_text(block(anchor))?;
        let text = sketch.texts[0].clone();
        let mut old = text.clone();
        old.contours = vec![
            vec![[-0.5, -0.5], [0.5, -0.5]],
            vec![[0.5, -0.5], [0.5, 0.5]],
        ];
        sketch.texts[0] = old;
        place(&mut sketch, text.id)?;
        assert_eq!(sketch.texts[0].entities.len(), 2);
        let report = crate::repair::suggest(&sketch, 1.)?;
        assert!(report.fixes.is_empty(), "{:?}", report.fixes);
        Ok(())
    }

    #[test]
    fn text_lines_follow_the_anchor_keeping_their_ids_and_area() -> Result<()> {
        let mut sketch = Sketch::new("Texto", Plane::xy());
        let anchor = sketch.add_point([20., 10.])?;
        let id = sketch.add_text(block(anchor))?;
        let text = sketch.texts[0].clone();
        assert_eq!(text.entities.len(), 5);
        assert!(text.entities.iter().all(|e| sketch.locked(*e)));
        assert_eq!(areas::regions(&sketch)?.len(), 1);
        // O ponto de inserção aceita cotas; as linhas acompanham com os mesmos IDs.
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: anchor,
                position: [0., 0.],
            },
            false,
        )?;
        let solved = solver::solve(&sketch)?.sketch;
        assert_eq!(solved.texts[0].entities, text.entities);
        let corner = solved.point(solved.texts[0].points[0])?;
        assert!(distance(corner, [-5., -5.]) < 1e-9, "{corner:?}");
        // Girar e mudar o tamanho recoloca; apagar remove o bloco e o ponto.
        let mut turned = solved.clone();
        let mut edited = turned.texts[0].clone();
        edited.rotation = std::f64::consts::FRAC_PI_2;
        edited.size = 20.;
        turned.update_text(edited)?;
        let corner = turned.point(turned.texts[0].points[0])?;
        assert!(distance(corner, [10., -10.]) < 1e-9, "{corner:?}");
        // Excluir uma letra apaga o bloco inteiro.
        let mut deleted = turned.clone();
        let letter = deleted.texts[0].entities[2];
        crate::edit::apply(
            &mut deleted,
            crate::edit::Action::Delete,
            &[letter],
            crate::edit::Options {
                value_mm: 0.,
                reference: false,
            },
        )?;
        assert!(deleted.texts.is_empty() && deleted.entities.is_empty());
        turned.remove_text(id)?;
        assert!(turned.texts.is_empty());
        assert!(turned.entities.is_empty());
        Ok(())
    }
}
