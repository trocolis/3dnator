//! Histórico explícito por peça. Referências nunca atravessam definições de peças.
//!
//! Referências a regiões, faces e arestas são estáveis: guardam a identidade de origem do
//! elemento (entidades do rascunho, nomes de faces), não sua posição em uma lista. Quando a
//! origem desaparece, o recálculo falha com uma mensagem pedindo nova seleção; nenhuma outra
//! região, face ou aresta é escolhida no lugar.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub name: String,
    pub output: BodyId,
    pub operation: Operation,
}

/// Região de um rascunho escolhida para extrusão ou revolução.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionRef {
    /// Entidades do contorno externo no momento da seleção (`profiles::find_region`).
    Boundary(Vec<u64>),
    /// Formato até a versão 6: posição na lista de regiões ordenada por área. Mantida só
    /// para abrir projetos antigos; a próxima edição da operação a converte em `Boundary`.
    Index(usize),
}

/// Aresta usada por arredondamento ou chanfro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeRef {
    /// Aresta entre duas faces nomeadas pela origem.
    Between([String; 2]),
    /// Índice válido apenas enquanto o B-rep de entrada for idêntico ao da seleção
    /// (`signature`). Usado quando as faces adjacentes não têm nome estável.
    Index { index: usize, signature: u64 },
}

/// Face usada pela casca.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaceRef {
    Named(String),
    Index { index: usize, signature: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Extrude {
        sketch: SketchId,
        region: RegionRef,
        from: String,
        /// Fim da extrusão; ignorado quando `until` define uma face limite.
        to: String,
        /// Une, corta ou intersecta o resultado com um corpo existente, que é consumido.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        combine: Option<Combine>,
        /// Termina no plano de uma face plana de outro corpo da mesma peça.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        until: Option<UntilFace>,
    },
    Revolve {
        sketch: SketchId,
        region: RegionRef,
        /// Eixo fixo em coordenadas da peça, usado quando `axis_line` é `None`.
        axis_origin: [f64; 3],
        axis_direction: [f64; 3],
        angle: String,
        /// Linha do rascunho usada como eixo (em geral de construção). Acompanha as cotas
        /// do rascunho a cada recálculo.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis_line: Option<u64>,
    },
    Boolean {
        base: BodyId,
        tool: BodyId,
        mode: BooleanMode,
    },
    /// Arredondamento de raio constante. O núcleo propaga cada aresta pelas tangentes.
    Fillet {
        base: BodyId,
        edge: EdgeRef,
        radius: String,
        /// Outras arestas arredondadas na mesma operação.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        additional_edges: Vec<EdgeRef>,
    },
    /// Chanfro simétrico, ou por distância e ângulo quando `angle` é informado.
    Chamfer {
        base: BodyId,
        edge: EdgeRef,
        distance: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        angle: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        additional_edges: Vec<EdgeRef>,
    },
    /// Casca com as faces indicadas removidas, para dentro ou para fora.
    Shell {
        base: BodyId,
        face: FaceRef,
        thickness: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        additional_faces: Vec<FaceRef>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        outward: bool,
    },
    Transform {
        base: BodyId,
        translation: [f64; 3],
        rotation: [f64; 3],
    },
    /// Furos centrados em pontos de um rascunho, furando contra a normal do plano do
    /// rascunho (a favor dela com `reverse`).
    Hole {
        base: BodyId,
        sketch: SketchId,
        /// Pontos do rascunho usados como centros; a identidade dos pontos é estável.
        points: Vec<u64>,
        spec: HoleSpec,
    },
    /// Cópias do corpo unidas em padrão linear ou circular.
    Pattern { base: BodyId, layout: PatternLayout },
    /// Espelhamento por plano. Com `keep_original`, une o corpo e sua imagem.
    Mirror {
        base: BodyId,
        origin: [f64; 3],
        normal: [f64; 3],
        keep_original: bool,
    },
    /// Parte do corpo do lado para onde `normal` aponta.
    Split {
        base: BodyId,
        origin: [f64; 3],
        normal: [f64; 3],
    },
}

/// Furo com medidas em expressões dos parâmetros da peça.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoleSpec {
    pub diameter: String,
    /// Profundidade da parede cilíndrica; `None` atravessa o corpo.
    pub depth: Option<String>,
    pub head: HoleHeadSpec,
    /// Ângulo total da ponta de broca em furos cegos; `None` deixa o fundo plano.
    pub tip_angle: Option<String>,
    pub reverse: bool,
    /// Rosca simplificada: registrada para documentação, sem hélice na geometria.
    pub thread: Option<ThreadSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HoleHeadSpec {
    Simple,
    Counterbore { diameter: String, depth: String },
    Countersink { diameter: String, angle: String },
}

/// Rosca registrada no furo: designação (por exemplo `M6`), diâmetro nominal, passo e
/// profundidade útil. O furo modelado é o de broca; a rosca não altera a geometria.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadSpec {
    pub designation: String,
    pub nominal: String,
    pub pitch: String,
    pub depth: Option<String>,
}

/// Direção de repetição de um padrão linear: `count` ocorrências espaçadas de `spacing`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinearAxis {
    pub direction: [f64; 3],
    pub count: u32,
    pub spacing: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternLayout {
    /// Uma ou duas direções; com duas, as ocorrências formam uma grade.
    Linear {
        first: LinearAxis,
        second: Option<LinearAxis>,
    },
    /// `count` ocorrências ao redor do eixo. Um ângulo total de 360° distribui as cópias
    /// por igual; outro ângulo coloca a última cópia exatamente nele.
    Circular {
        axis_origin: [f64; 3],
        axis_direction: [f64; 3],
        count: u32,
        angle: String,
    },
}

/// Limite de ocorrências por padrão e de furos por operação neste protótipo.
pub const MAX_OCCURRENCES: usize = 1000;
/// Combinação do resultado de uma extrusão com um corpo existente da peça.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Combine {
    pub target: BodyId,
    pub mode: BooleanMode,
}

/// Limite de extrusão por uma face plana de um corpo da mesma peça. A distância é recalculada
/// a cada recálculo, então a extrusão acompanha a face; faces de outras peças não são aceitas,
/// para manter as peças independentes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UntilFace {
    pub body: BodyId,
    pub face: FaceRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BooleanMode {
    Join,
    Cut,
    Intersect,
}
impl Operation {
    /// Corpos consumidos pela operação: ficam ocultos enquanto ela existir.
    pub fn inputs(&self) -> Vec<BodyId> {
        match self {
            Self::Extrude {
                combine: Some(combine),
                ..
            } => vec![combine.target],
            Self::Boolean { base, tool, .. } => vec![*base, *tool],
            Self::Fillet { base, .. }
            | Self::Chamfer { base, .. }
            | Self::Shell { base, .. }
            | Self::Transform { base, .. }
            | Self::Hole { base, .. }
            | Self::Pattern { base, .. }
            | Self::Mirror { base, .. }
            | Self::Split { base, .. } => vec![*base],
            _ => Vec::new(),
        }
    }
    /// Todos os corpos de que a operação depende (consumidos ou apenas consultados), para a
    /// ordem do histórico e para impedir a exclusão de um corpo ainda referenciado.
    pub fn references(&self) -> Vec<BodyId> {
        let mut bodies = self.inputs();
        if let Self::Extrude {
            until: Some(limit), ..
        } = self
        {
            bodies.push(limit.body);
        }
        bodies
    }
    pub fn sketch(&self) -> Option<SketchId> {
        match self {
            Self::Extrude { sketch, .. }
            | Self::Revolve { sketch, .. }
            | Self::Hole { sketch, .. } => Some(*sketch),
            _ => None,
        }
    }
}

/// Dono das faces produzidas pela operação cujo resultado é o corpo `id`.
/// Os nomes de faces têm a forma `b{id}:...` (ver `nator_kernel`).
pub fn body_owner(id: BodyId) -> String {
    format!("b{}", id.0)
}

/// Ao clonar uma peça, nomes `b{antigo}:...` passam a `b{novo}:...`, para que as
/// referências do clone apontem para os corpos do próprio clone.
pub fn remap_face_name(name: &str, bodies: &HashMap<BodyId, BodyId>) -> String {
    let Some((digits, tail)) = name.strip_prefix('b').and_then(|rest| rest.split_once(':')) else {
        return name.to_owned();
    };
    match digits
        .parse::<u64>()
        .ok()
        .and_then(|id| bodies.get(&BodyId(id)))
    {
        Some(new) => format!("{}:{tail}", body_owner(*new)),
        None => name.to_owned(),
    }
}

impl Feature {
    pub fn remap(
        &mut self,
        bodies: &HashMap<BodyId, BodyId>,
        sketches: &HashMap<SketchId, SketchId>,
    ) -> Result<(), DocumentError> {
        let body = |id: &mut BodyId| -> Result<(), DocumentError> {
            *id = *bodies
                .get(id)
                .ok_or_else(|| DocumentError("Referência de corpo ausente ao clonar.".into()))?;
            Ok(())
        };
        body(&mut self.output)?;
        let sketch = |id: &mut SketchId| -> Result<(), DocumentError> {
            *id = *sketches
                .get(id)
                .ok_or_else(|| DocumentError("Rascunho ausente ao clonar.".into()))?;
            Ok(())
        };
        match &mut self.operation {
            Operation::Extrude {
                sketch: id,
                combine,
                until,
                ..
            } => {
                sketch(id)?;
                if let Some(combine) = combine {
                    body(&mut combine.target)?;
                }
                if let Some(limit) = until {
                    body(&mut limit.body)?;
                    if let FaceRef::Named(name) = &mut limit.face {
                        *name = remap_face_name(name, bodies);
                    }
                }
            }
            Operation::Revolve { sketch: id, .. } => {
                sketch(id)?;
            }
            Operation::Hole {
                base, sketch: id, ..
            } => {
                body(base)?;
                sketch(id)?;
            }
            Operation::Pattern { base, .. }
            | Operation::Mirror { base, .. }
            | Operation::Split { base, .. } => body(base)?,
            Operation::Boolean { base, tool, .. } => {
                body(base)?;
                body(tool)?;
            }
            Operation::Fillet {
                base,
                edge,
                additional_edges,
                ..
            }
            | Operation::Chamfer {
                base,
                edge,
                additional_edges,
                ..
            } => {
                body(base)?;
                for edge in std::iter::once(edge).chain(additional_edges) {
                    if let EdgeRef::Between(faces) = edge {
                        for face in faces {
                            *face = remap_face_name(face, bodies);
                        }
                    }
                }
            }
            Operation::Shell {
                base,
                face,
                additional_faces,
                ..
            } => {
                body(base)?;
                for face in std::iter::once(face).chain(additional_faces) {
                    if let FaceRef::Named(name) = face {
                        *name = remap_face_name(name, bodies);
                    }
                }
            }
            Operation::Transform { base, .. } => body(base)?,
        }
        Ok(())
    }
}
/// Assinatura conservadora para referências por índice: se a geometria mudar, a seleção
/// precisa ser refeita. Isso evita aplicar um chanfro em outra aresta após uma mudança.
pub fn signature(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
pub fn validate(part: &Part) -> Result<(), DocumentError> {
    if part.features.len() > 256 {
        return Err(DocumentError(
            "Histórico excede 256 operações por peça.".into(),
        ));
    }
    let outputs = part
        .features
        .iter()
        .map(|f| f.output)
        .collect::<HashSet<_>>();
    let mut ready = part
        .bodies
        .iter()
        .filter(|b| !outputs.contains(&b.id))
        .map(|b| b.id)
        .collect::<HashSet<_>>();
    for f in &part.features {
        validate_name(&f.name)?;
        if !part
            .bodies
            .iter()
            .any(|b| b.id == f.output && b.brep.is_some())
            || ready.contains(&f.output)
            || f.operation.references().iter().any(|b| !ready.contains(b))
            || f.operation
                .sketch()
                .is_some_and(|id| !part.sketches.iter().any(|s| s.id == id))
        {
            return Err(DocumentError(
                "Histórico contém referência ausente, repetida, futura ou de outra peça.".into(),
            ));
        }
        use parameters::Dimension::{Angle, Length};
        match &f.operation {
            Operation::Extrude {
                from,
                to,
                region,
                until,
                ..
            } => {
                validate_region(region)?;
                let a = part.parameters.evaluate(from, Length)?;
                match until {
                    // Com face limite, o comprimento só é conhecido no recálculo.
                    Some(limit) => {
                        if let FaceRef::Named(name) = &limit.face {
                            validate_face_name(name)?;
                        }
                    }
                    None => {
                        let b = part.parameters.evaluate(to, Length)?;
                        if (a - b).abs() < 1e-7 {
                            return Err(DocumentError(
                                "Extrusão deve ter comprimento não nulo.".into(),
                            ));
                        }
                    }
                }
            }
            Operation::Revolve {
                angle,
                axis_origin,
                axis_direction,
                region,
                ..
            } => {
                validate_region(region)?;
                let a = part.parameters.evaluate(angle, Angle)?;
                if !a.is_finite()
                    || a.abs() < 1e-7
                    || a.abs() > std::f64::consts::TAU + 1e-9
                    || axis_origin
                        .iter()
                        .chain(axis_direction)
                        .any(|v| !v.is_finite())
                    || nator_geometry::norm(*axis_direction) < 1e-9
                {
                    return Err(DocumentError(
                        "Revolução com eixo ou ângulo inválido.".into(),
                    ));
                }
            }
            Operation::Fillet {
                radius,
                edge,
                additional_edges,
                ..
            }
            | Operation::Chamfer {
                distance: radius,
                edge,
                additional_edges,
                ..
            } => {
                if additional_edges.len() >= MAX_OCCURRENCES {
                    return Err(DocumentError("Arestas demais na operação.".into()));
                }
                for edge in std::iter::once(edge).chain(additional_edges) {
                    if let EdgeRef::Between(faces) = edge {
                        faces.iter().try_for_each(|f| validate_face_name(f))?;
                    }
                }
                if part.parameters.evaluate(radius, Length)? <= 0. {
                    return Err(DocumentError("Dimensão deve ser positiva.".into()));
                }
                if let Operation::Chamfer {
                    angle: Some(angle), ..
                } = &f.operation
                {
                    let value = part.parameters.evaluate(angle, Angle)?;
                    if !(value > 0. && value < std::f64::consts::FRAC_PI_2) {
                        return Err(DocumentError(
                            "O ângulo do chanfro deve ficar entre 0° e 90°.".into(),
                        ));
                    }
                }
            }
            Operation::Shell {
                thickness,
                face,
                additional_faces,
                ..
            } => {
                if additional_faces.len() >= MAX_OCCURRENCES {
                    return Err(DocumentError("Faces demais na casca.".into()));
                }
                for face in std::iter::once(face).chain(additional_faces) {
                    if let FaceRef::Named(name) = face {
                        validate_face_name(name)?;
                    }
                }
                if part.parameters.evaluate(thickness, Length)? <= 0. {
                    return Err(DocumentError("Dimensão deve ser positiva.".into()));
                }
            }
            Operation::Transform {
                translation,
                rotation,
                ..
            } => {
                Placement::new(*translation, *rotation)
                    .map_err(|e| DocumentError(e.to_string()))?;
            }
            Operation::Hole {
                points,
                spec,
                sketch,
                ..
            } => {
                if points.is_empty() || points.len() > MAX_OCCURRENCES {
                    return Err(DocumentError(
                        "A operação de furo precisa de 1 a 1000 pontos.".into(),
                    ));
                }
                let record = part.sketches.iter().find(|s| s.id == *sketch);
                if points
                    .iter()
                    .any(|p| record.is_some_and(|r| r.sketch.point(*p).is_err()))
                {
                    return Err(DocumentError(
                        "Um ponto do furo não existe mais no rascunho; selecione os pontos novamente."
                            .into(),
                    ));
                }
                validate_hole(spec, &part.parameters)?;
            }
            Operation::Pattern { layout, .. } => validate_pattern(layout, &part.parameters)?,
            Operation::Mirror { origin, normal, .. } | Operation::Split { origin, normal, .. } => {
                validate_plane(*origin, *normal)?;
            }
            Operation::Boolean { .. } => {}
        }
        ready.insert(f.output);
    }
    Ok(())
}

fn positive(
    parameters: &parameters::Parameters,
    expression: &str,
    dimension: parameters::Dimension,
) -> Result<f64, DocumentError> {
    let value = parameters.evaluate(expression, dimension)?;
    if value.is_finite() && value > 0. {
        Ok(value)
    } else {
        Err(DocumentError(format!(
            "A medida \"{expression}\" deve ser positiva."
        )))
    }
}

fn validate_hole(
    spec: &HoleSpec,
    parameters: &parameters::Parameters,
) -> Result<(), DocumentError> {
    use parameters::Dimension::{Angle, Length};
    positive(parameters, &spec.diameter, Length)?;
    if let Some(depth) = &spec.depth {
        positive(parameters, depth, Length)?;
    }
    if let Some(angle) = &spec.tip_angle {
        positive(parameters, angle, Angle)?;
    }
    match &spec.head {
        HoleHeadSpec::Simple => {}
        HoleHeadSpec::Counterbore { diameter, depth } => {
            positive(parameters, diameter, Length)?;
            positive(parameters, depth, Length)?;
        }
        HoleHeadSpec::Countersink { diameter, angle } => {
            positive(parameters, diameter, Length)?;
            positive(parameters, angle, Angle)?;
        }
    }
    if let Some(thread) = &spec.thread {
        validate_name(&thread.designation)?;
        positive(parameters, &thread.nominal, Length)?;
        positive(parameters, &thread.pitch, Length)?;
        if let Some(depth) = &thread.depth {
            positive(parameters, depth, Length)?;
        }
    }
    Ok(())
}

fn validate_direction(direction: [f64; 3], name: &str) -> Result<(), DocumentError> {
    if direction.iter().any(|v| !v.is_finite()) || nator_geometry::norm(direction) < 1e-9 {
        return Err(DocumentError(format!("{name} deve ser não nula e finita.")));
    }
    Ok(())
}

fn validate_plane(origin: [f64; 3], normal: [f64; 3]) -> Result<(), DocumentError> {
    if origin.iter().any(|v| !v.is_finite()) {
        return Err(DocumentError("Origem do plano inválida.".into()));
    }
    validate_direction(normal, "A normal do plano")
}

fn validate_pattern(
    layout: &PatternLayout,
    parameters: &parameters::Parameters,
) -> Result<(), DocumentError> {
    use parameters::Dimension::{Angle, Length};
    let count = match layout {
        PatternLayout::Linear { first, second } => {
            let mut total = 1_usize;
            for axis in std::iter::once(first).chain(second) {
                validate_direction(axis.direction, "A direção do padrão")?;
                parameters.evaluate(&axis.spacing, Length)?;
                total = total.saturating_mul(axis.count as usize);
            }
            total
        }
        PatternLayout::Circular {
            axis_origin,
            axis_direction,
            count,
            angle,
        } => {
            validate_plane(*axis_origin, *axis_direction)?;
            let total = parameters.evaluate(angle, Angle)?;
            if !total.is_finite()
                || total.abs() < 1e-9
                || total.abs() > std::f64::consts::TAU + 1e-9
            {
                return Err(DocumentError(
                    "O ângulo do padrão circular deve estar entre 0° e 360°.".into(),
                ));
            }
            *count as usize
        }
    };
    if !(2..=MAX_OCCURRENCES).contains(&count) {
        return Err(DocumentError(
            "O padrão deve ter entre 2 e 1000 ocorrências.".into(),
        ));
    }
    Ok(())
}

/// Posições de um padrão em coordenadas da peça; a primeira é sempre a identidade.
pub fn pattern_placements(
    layout: &PatternLayout,
    parameters: &parameters::Parameters,
) -> Result<Vec<Placement>, DocumentError> {
    use parameters::Dimension::{Angle, Length};
    let error = |e: nator_geometry::GeometryError| DocumentError(e.to_string());
    validate_pattern(layout, parameters)?;
    match layout {
        PatternLayout::Linear { first, second } => {
            let step = |axis: &LinearAxis| -> Result<[f64; 3], DocumentError> {
                let spacing = parameters.evaluate(&axis.spacing, Length)?;
                let length = nator_geometry::norm(axis.direction);
                Ok(axis.direction.map(|v| v / length * spacing))
            };
            let a = step(first)?;
            let (b, rows) = match second {
                Some(axis) => (step(axis)?, axis.count),
                None => ([0.; 3], 1),
            };
            let mut placements = Vec::new();
            for j in 0..rows {
                for i in 0..first.count {
                    let offset = std::array::from_fn(|k| a[k] * f64::from(i) + b[k] * f64::from(j));
                    placements.push(Placement::new(offset, [0.; 3]).map_err(error)?);
                }
            }
            Ok(placements)
        }
        PatternLayout::Circular {
            axis_origin,
            axis_direction,
            count,
            angle,
        } => {
            let total = parameters.evaluate(angle, Angle)?;
            let full = (total.abs() - std::f64::consts::TAU).abs() < 1e-9;
            let step = if full {
                total / f64::from(*count)
            } else {
                total / f64::from(count - 1)
            };
            (0..*count)
                .map(|k| {
                    Placement::default()
                        .rotated_about(*axis_origin, *axis_direction, step * f64::from(k))
                        .map_err(error)
                })
                .collect()
        }
    }
}

fn validate_region(region: &RegionRef) -> Result<(), DocumentError> {
    if let RegionRef::Boundary(ids) = region
        && (ids.is_empty() || ids.len() > nator_sketch::MAX_ENTITIES)
    {
        return Err(DocumentError(
            "Referência de região vazia ou excessiva.".into(),
        ));
    }
    Ok(())
}

/// Nomes de faces são curtos e sem NUL; vazio significa "sem nome" e não é referenciável.
pub fn validate_face_name(name: &str) -> Result<(), DocumentError> {
    if name.is_empty() || name.len() > 256 || name.contains('\0') {
        return Err(DocumentError("Nome de face inválido.".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_placements_follow_spacing_and_angles() -> Result<(), Box<dyn Error>> {
        let mut parameters = parameters::Parameters::default();
        parameters.set(
            "passo".into(),
            "12 mm".into(),
            parameters::Dimension::Length,
        )?;
        let grid = PatternLayout::Linear {
            first: LinearAxis {
                direction: [2., 0., 0.],
                count: 3,
                spacing: "passo".into(),
            },
            second: Some(LinearAxis {
                direction: [0., 1., 0.],
                count: 2,
                spacing: "20 mm".into(),
            }),
        };
        let placements = pattern_placements(&grid, &parameters)?;
        assert_eq!(placements.len(), 6);
        assert_eq!(placements[0], Placement::default());
        assert_eq!(placements[2].apply([0.; 3]), [24., 0., 0.]);
        assert_eq!(placements[5].apply([0.; 3]), [24., 20., 0.]);
        let ring = |angle: &str, count| PatternLayout::Circular {
            axis_origin: [0.; 3],
            axis_direction: [0., 0., 1.],
            count,
            angle: angle.into(),
        };
        // 360°: quatro cópias a cada 90°; 90°: três cópias com a última em 90°.
        let full = pattern_placements(&ring("360 deg", 4), &parameters)?;
        let quarter = pattern_placements(&ring("90 deg", 3), &parameters)?;
        let x = |p: &Placement| p.apply([10., 0., 0.]);
        assert!((x(&full[1])[1] - 10.).abs() < 1e-9);
        assert!((x(&quarter[2])[1] - 10.).abs() < 1e-9);
        assert!((x(&quarter[1])[0] - 10. * std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
        // Uma única ocorrência não é um padrão.
        assert!(pattern_placements(&ring("360 deg", 1), &parameters).is_err());
        Ok(())
    }

    #[test]
    fn cloning_moves_face_owners_to_the_new_bodies() -> Result<(), Box<dyn Error>> {
        let bodies = HashMap::from([(BodyId(12), BodyId(40)), (BodyId(3), BodyId(41))]);
        assert_eq!(remap_face_name("b12:e7", &bodies), "b40:e7");
        assert_eq!(remap_face_name("b3:e2:inner", &bodies), "b41:e2:inner");
        // Donos de fora da peça clonada e nomes de outro formato ficam intactos.
        assert_eq!(remap_face_name("b99:start", &bodies), "b99:start");
        assert_eq!(remap_face_name("box:f1", &bodies), "box:f1");
        let mut feature = Feature {
            name: "Arredondamento".into(),
            output: BodyId(3),
            operation: Operation::Fillet {
                base: BodyId(12),
                edge: EdgeRef::Between(["b12:e7".into(), "b12:end".into()]),
                radius: "2 mm".into(),
                additional_edges: vec![EdgeRef::Between(["b12:e8".into(), "b12:end".into()])],
            },
        };
        feature.remap(&bodies, &HashMap::new())?;
        let Operation::Fillet {
            base,
            edge,
            additional_edges,
            ..
        } = feature.operation
        else {
            return Err("A operação deixou de ser um arredondamento".into());
        };
        assert_eq!(base, BodyId(40));
        assert_eq!(edge, EdgeRef::Between(["b40:e7".into(), "b40:end".into()]));
        assert_eq!(
            additional_edges,
            vec![EdgeRef::Between(["b40:e8".into(), "b40:end".into()])]
        );
        Ok(())
    }
}
