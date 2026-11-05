//! Perfis heterogêneos de uma única extrusão coplanar. Referências continuam por IDs e nomes.
use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExtrusionProfile {
    Sketch(ProfileRef),
    Face {
        base: BodyId,
        face: FaceRef,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        boundaries: Vec<PaintBoundary>,
    },
}
impl ExtrusionProfile {
    pub fn operation(&self, from: String, to: String, until: Option<ExtrusionLimit>) -> Operation {
        match self {
            Self::Sketch(profile) => Operation::Extrude {
                sketch: profile.sketch,
                region: profile.region.clone(),
                from,
                to,
                until,
                combine: None,
            },
            Self::Face {
                base,
                face,
                boundaries,
            } => Operation::ExtrudeFace {
                base: *base,
                face: face.clone(),
                boundaries: boundaries.clone(),
                from,
                to,
                until,
                combine: None,
            },
        }
    }
    pub fn base(&self) -> Option<BodyId> {
        match self {
            Self::Face { base, .. } => Some(*base),
            Self::Sketch(_) => None,
        }
    }
    pub fn uses_sketch(&self, id: SketchId) -> bool {
        match self {
            Self::Sketch(p) => p.sketch == id,
            Self::Face { boundaries, .. } => boundaries.iter().any(|b| b.mask.sketch == id),
        }
    }
    pub(super) fn remap(
        &mut self,
        bodies: &HashMap<BodyId, BodyId>,
        sketches: &HashMap<SketchId, SketchId>,
    ) -> Result<(), DocumentError> {
        let sketch = |id: &mut SketchId| -> Result<(), DocumentError> {
            *id = *sketches
                .get(id)
                .ok_or_else(|| DocumentError("Rascunho de extrusão ausente ao clonar.".into()))?;
            Ok(())
        };
        match self {
            Self::Sketch(p) => sketch(&mut p.sketch),
            Self::Face {
                base,
                face,
                boundaries,
            } => {
                *base = *bodies
                    .get(base)
                    .ok_or_else(|| DocumentError("Corpo de extrusão ausente ao clonar.".into()))?;
                if let FaceRef::Named(name) = face {
                    *name = remap_face_name(name, bodies);
                }
                for boundary in boundaries {
                    sketch(&mut boundary.mask.sketch)?;
                }
                Ok(())
            }
        }
    }
    pub(super) fn validate(&self, part: &Part) -> Result<(), DocumentError> {
        let profile = |id: SketchId, region: &RegionRef| {
            if !part.sketches.iter().any(|s| s.id == id) {
                return Err(DocumentError(
                    "Rascunho do perfil de extrusão inexistente.".into(),
                ));
            }
            validate_region(region)
        };
        match self {
            Self::Sketch(p) => profile(p.sketch, &p.region),
            Self::Face {
                face, boundaries, ..
            } => {
                if let FaceRef::Named(name) = face {
                    validate_face_name(name)?;
                }
                if boundaries.len() > 64 {
                    return Err(DocumentError("Delimitações de extrusão em excesso.".into()));
                }
                for boundary in boundaries {
                    profile(boundary.mask.sketch, &boundary.mask.region)?;
                }
                Ok(())
            }
        }
    }
}
