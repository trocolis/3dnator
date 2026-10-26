//! Histórico explícito por peça. Referências nunca atravessam definições de peças.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub name: String,
    pub output: BodyId,
    pub operation: Operation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Extrude {
        sketch: SketchId,
        region: usize,
        from: String,
        to: String,
    },
    Revolve {
        sketch: SketchId,
        region: usize,
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
        edge: usize,
        signature: u64,
        radius: String,
    },
    Chamfer {
        base: BodyId,
        edge: usize,
        signature: u64,
        distance: String,
    },
    Shell {
        base: BodyId,
        face: usize,
        signature: u64,
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
            Operation::Fillet { base, .. }
            | Operation::Chamfer { base, .. }
            | Operation::Shell { base, .. }
            | Operation::Transform { base, .. } => body(base)?,
        }
        Ok(())
    }
}
/// Assinatura conservadora: se a geometria mudar, uma seleção topológica precisa ser refeita.
/// Isso evita aplicar um chanfro em outra aresta após uma mudança no perfil.
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
            Operation::Extrude { from, to, .. } => {
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
                ..
            } => {
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
            Operation::Fillet { radius, .. }
            | Operation::Chamfer {
                distance: radius, ..
            }
            | Operation::Shell {
                thickness: radius, ..
            } => {
                if part.parameters.evaluate(radius, Length)? <= 0. {
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
