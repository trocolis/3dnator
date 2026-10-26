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
        to: String,
    },
    Revolve {
        sketch: SketchId,
        region: RegionRef,
        axis_origin: [f64; 3],
        axis_direction: [f64; 3],
        angle: String,
    },
    Boolean {
        base: BodyId,
        tool: BodyId,
        mode: BooleanMode,
    },
    Fillet {
        base: BodyId,
        edge: EdgeRef,
        radius: String,
    },
    Chamfer {
        base: BodyId,
        edge: EdgeRef,
        distance: String,
    },
    Shell {
        base: BodyId,
        face: FaceRef,
        thickness: String,
    },
    Transform {
        base: BodyId,
        translation: [f64; 3],
        rotation: [f64; 3],
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BooleanMode {
    Join,
    Cut,
    Intersect,
}
impl Operation {
    pub fn inputs(&self) -> Vec<BodyId> {
        match self {
            Self::Boolean { base, tool, .. } => vec![*base, *tool],
            Self::Fillet { base, .. }
            | Self::Chamfer { base, .. }
            | Self::Shell { base, .. }
            | Self::Transform { base, .. } => vec![*base],
            _ => Vec::new(),
        }
    }
    pub fn sketch(&self) -> Option<SketchId> {
        match self {
            Self::Extrude { sketch, .. } | Self::Revolve { sketch, .. } => Some(*sketch),
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
        match &mut self.operation {
            Operation::Extrude { sketch, .. } | Operation::Revolve { sketch, .. } => {
                *sketch = *sketches
                    .get(sketch)
                    .ok_or_else(|| DocumentError("Rascunho ausente ao clonar.".into()))?;
            }
            Operation::Boolean { base, tool, .. } => {
                body(base)?;
                body(tool)?;
            }
            Operation::Fillet { base, edge, .. } | Operation::Chamfer { base, edge, .. } => {
                body(base)?;
                if let EdgeRef::Between(faces) = edge {
                    for face in faces {
                        *face = remap_face_name(face, bodies);
                    }
                }
            }
            Operation::Shell { base, face, .. } => {
                body(base)?;
                if let FaceRef::Named(name) = face {
                    *name = remap_face_name(name, bodies);
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
            || f.operation.inputs().iter().any(|b| !ready.contains(b))
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
                from, to, region, ..
            } => {
                validate_region(region)?;
                let a = part.parameters.evaluate(from, Length)?;
                let b = part.parameters.evaluate(to, Length)?;
                if (a - b).abs() < 1e-7 {
                    return Err(DocumentError(
                        "Extrusão deve ter comprimento não nulo.".into(),
                    ));
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
            Operation::Fillet { radius, edge, .. }
            | Operation::Chamfer {
                distance: radius,
                edge,
                ..
            } => {
                if let EdgeRef::Between(faces) = edge {
                    faces.iter().try_for_each(|f| validate_face_name(f))?;
                }
                if part.parameters.evaluate(radius, Length)? <= 0. {
                    return Err(DocumentError("Dimensão deve ser positiva.".into()));
                }
            }
            Operation::Shell {
                thickness, face, ..
            } => {
                if let FaceRef::Named(name) = face {
                    validate_face_name(name)?;
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
            Operation::Boolean { .. } => {}
        }
        ready.insert(f.output);
    }
    Ok(())
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
            },
        };
        feature.remap(&bodies, &HashMap::new())?;
        let Operation::Fillet { base, edge, .. } = feature.operation else {
            return Err("A operação deixou de ser um arredondamento".into());
        };
        assert_eq!(base, BodyId(40));
        assert_eq!(edge, EdgeRef::Between(["b40:e7".into(), "b40:end".into()]));
        Ok(())
    }
}
