//! Aparência por face, opcionalmente recortada por áreas de um rascunho da peça.
//! O cache de exibição não substitui o B-rep nem a malha usados para medições,
//! seleção e exportação geométrica. Material zero herda a cor do corpo/ocorrência.
use super::*;
use features::{FaceRef, RegionRef};
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaintMask {
    pub sketch: SketchId,
    pub region: RegionRef,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfacePaint {
    /// Interseções/complementos das células detectadas no clique. O campo legado
    /// mask continua legível; novos cliques não precisam escolher um rascunho.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boundaries: Vec<PaintBoundary>,
    pub face: FaceRef,
    pub mask: Option<PaintMask>,
    #[serde(with = "color_serde")]
    pub color: RgbColor,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaintBoundary {
    pub mask: PaintMask,
    pub inside: bool,
}
impl SurfacePaint {
    pub fn masks(&self) -> impl Iterator<Item = &PaintMask> {
        self.mask
            .iter()
            .chain(self.boundaries.iter().map(|b| &b.mask))
    }
}
mod color_serde {
    use nator_geometry::RgbColor;
    use serde::{Deserialize, Serialize};
    pub fn serialize<S: serde::Serializer>(
        color: &RgbColor,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        color.channels().serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<RgbColor, D::Error> {
        <[u8; 3]>::deserialize(deserializer).map(RgbColor::new)
    }
}
#[derive(Debug, Clone)]
/// Camadas ordenadas: a última cor prevalece apenas dentro da sua delimitação.
/// A malha derivada é um cache persistido para abrir o projeto sem executar OCCT.
pub struct SurfaceAppearance {
    pub body: BodyId,
    /// Cor do balde aplicada ao corpo inteiro; prevalece sobre a cor da ocorrência.
    pub base_color: Option<RgbColor>,
    pub layers: Vec<SurfacePaint>,
    pub source_signature: u64,
    pub geometry: Option<Arc<MeshAsset>>,
    /// Um byte por triângulo: zero herda a cor do corpo, N usa a camada N−1.
    pub materials: Arc<Vec<u8>>,
}
impl SurfaceAppearance {
    fn validate(&self, doc: &Document) -> Result<(), DocumentError> {
        let body = doc
            .body(self.body)
            .ok_or_else(|| DocumentError("Corpo pintado inexistente.".into()))?;
        let part = doc
            .parts
            .iter()
            .find(|p| p.bodies.iter().any(|b| b.id == self.body))
            .ok_or_else(|| DocumentError("Peça da pintura inexistente.".into()))?;
        let bytes = body
            .brep
            .as_ref()
            .ok_or_else(|| DocumentError("Pintura de superfícies exige um corpo CAD.".into()))?;
        if (self.layers.is_empty() && self.base_color.is_none()) || self.layers.len() > 255 {
            return Err(DocumentError("Quantidade de pinturas inválida.".into()));
        }
        for layer in &self.layers {
            match &layer.face {
                FaceRef::Named(name) => features::validate_face_name(name)?,
                FaceRef::Index { index, .. } if *index >= MAX_FACES_PER_BODY => {
                    return Err(DocumentError("Face pintada fora do limite.".into()));
                }
                _ => {}
            }
            if layer.boundaries.len() > 64 {
                return Err(DocumentError(
                    "Quantidade de delimitações de pintura excessiva.".into(),
                ));
            }
            for mask in layer.masks() {
                if !part.sketches.iter().any(|s| s.id == mask.sketch) {
                    return Err(DocumentError(
                        "O rascunho da pintura deve pertencer à peça do corpo.".into(),
                    ));
                }
                features::validate_region(&mask.region)?;
            }
        }
        if let Some(geometry) = &self.geometry {
            if self.source_signature != features::signature(bytes)
                || geometry.mesh().triangles().len() != self.materials.len()
                || self
                    .materials
                    .iter()
                    .any(|m| *m as usize > self.layers.len())
            {
                return Err(DocumentError(
                    "Cache de aparência incoerente com o corpo/material.".into(),
                ));
            }
        } else if !self.materials.is_empty() {
            return Err(DocumentError("Aparência sem geometria de exibição.".into()));
        }
        Ok(())
    }
}
impl Document {
    pub fn appearances(&self) -> &[SurfaceAppearance] {
        &self.appearances
    }
    pub fn body_appearance(&self, body: BodyId) -> Option<&SurfaceAppearance> {
        self.appearances.iter().find(|a| a.body == body)
    }
    pub fn set_surface_appearance(
        &mut self,
        appearance: SurfaceAppearance,
    ) -> Result<(), DocumentError> {
        appearance.validate(self)?;
        if let Some(previous) = self
            .appearances
            .iter_mut()
            .find(|a| a.body == appearance.body)
        {
            *previous = appearance;
        } else {
            self.appearances.push(appearance);
        }
        Ok(())
    }
    pub fn remove_surface_appearance(&mut self, body: BodyId) {
        self.appearances.retain(|a| a.body != body);
    }
    pub fn restore_appearances(
        &mut self,
        appearances: Vec<SurfaceAppearance>,
    ) -> Result<(), DocumentError> {
        let mut ids = HashSet::new();
        for appearance in &appearances {
            appearance.validate(self)?;
            if !ids.insert(appearance.body) {
                return Err(DocumentError("Aparência repetida no corpo.".into()));
            }
        }
        self.appearances = appearances;
        Ok(())
    }
    pub(super) fn validate_appearances(&self) -> Result<(), DocumentError> {
        if self.appearances.len() > MAX_ELEMENTS {
            return Err(DocumentError("Quantidade de aparências excessiva.".into()));
        }
        let mut ids = HashSet::new();
        for appearance in &self.appearances {
            appearance.validate(self)?;
            if !ids.insert(appearance.body) {
                return Err(DocumentError("Aparência repetida no corpo.".into()));
            }
        }
        let triangles = self
            .parts
            .iter()
            .flat_map(|p| &p.bodies)
            .map(|body| {
                self.body_appearance(body.id)
                    .and_then(|a| a.geometry.as_ref())
                    .unwrap_or(&body.geometry)
                    .mesh()
                    .triangles()
                    .len() as u64
            })
            .sum::<u64>();
        if triangles > MAX_SCENE_TRIANGLES as u64 {
            return Err(DocumentError(
                "A aparência excede o limite de triângulos da cena.".into(),
            ));
        }
        Ok(())
    }
}
