//! Texto é persistido junto dos contornos unitários. Mudar tamanho/posição/cor
//! não requer a fonte; mudar texto/estilo recompila os contornos no worker.
use crate::{
    DocumentError,
    parameters::{Dimension, Parameters},
};
use nator_geometry::{Point3, RgbColor};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelMode {
    Surface,
    Raised,
    Engraved,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct LabelSpec {
    pub text: String,
    pub font_family: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// Tamanho e profundidade em mm, rotação em radianos após avaliação.
    pub size: String,
    pub depth: String,
    pub rotation: String,
    #[serde(with = "crate::appearance::color_serde")]
    pub color: RgbColor,
    pub mode: LabelMode,
    /// Ponto em coordenadas da peça; UV é capturado na primeira projeção e
    /// acompanha mudanças paramétricas da superfície. Reposicionar limpa UV.
    pub position: Point3,
    /// Normal calculada, usada apenas pelo manipulador de apresentação.
    #[serde(default = "default_normal")]
    pub normal: Point3,
    #[serde(default)]
    pub uv: Option<[f64; 2]>,
    #[serde(default)]
    pub outline: Arc<Vec<u8>>,
    #[serde(default)]
    pub outline_key: u64,
}
impl Default for LabelSpec {
    fn default() -> Self {
        Self {
            text: "3dnator".into(),
            font_family: "sans-serif".into(),
            bold: false,
            italic: false,
            underline: false,
            size: "5 mm".into(),
            depth: "0.5 mm".into(),
            rotation: "0 deg".into(),
            color: RgbColor::new([30, 30, 30]),
            mode: LabelMode::Surface,
            position: [0.; 3],
            normal: default_normal(),
            uv: None,
            outline: Arc::default(),
            outline_key: 0,
        }
    }
}
impl LabelSpec {
    pub fn shape_key(&self) -> u64 {
        let mut bytes = self.text.as_bytes().to_vec();
        bytes.push(0);
        bytes.extend_from_slice(self.font_family.as_bytes());
        bytes.extend([
            0,
            u8::from(self.bold),
            u8::from(self.italic),
            u8::from(self.underline),
        ]);
        super::signature(&bytes)
    }
    pub fn validate(&self, parameters: &Parameters) -> Result<(), DocumentError> {
        if self.text.trim().is_empty()
            || self.text.chars().count() > 128
            || self.text.contains(['\0', '\n', '\r'])
            || self.font_family.is_empty()
            || self.font_family.len() > 256
            || self.font_family.contains(['\0', '\n', '\r'])
        {
            return Err(DocumentError(
                "Use texto de uma linha com até 128 caracteres e uma fonte válida.".into(),
            ));
        }
        if self
            .position
            .iter()
            .chain(&self.normal)
            .any(|x| !x.is_finite())
            || self.uv.is_some_and(|uv| uv.iter().any(|x| !x.is_finite()))
        {
            return Err(DocumentError("Posição do rótulo inválida.".into()));
        }
        let size = parameters.evaluate(&self.size, Dimension::Length)?;
        if !(0. < size && size <= 10000.)
            || (!matches!(self.mode, LabelMode::Surface)
                && parameters.evaluate(&self.depth, Dimension::Length)? <= 0.)
        {
            return Err(DocumentError(
                "Tamanho e profundidade do rótulo devem ser positivos.".into(),
            ));
        }
        parameters.evaluate(&self.rotation, Dimension::Angle)?;
        if self.outline.is_empty()
            || self.outline.len() > 16 * 1024 * 1024
            || self.outline_key != self.shape_key()
        {
            return Err(DocumentError(
                "Os contornos do rótulo estão ausentes ou não correspondem ao texto/fonte.".into(),
            ));
        }
        Ok(())
    }
}

fn default_normal() -> Point3 {
    [0., 0., 1.]
}
// O comando de prévia é identificado por Debug; nunca formatar o B-rep inteiro.
impl std::fmt::Debug for LabelSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LabelSpec")
            .field("text", &self.text)
            .field("font_family", &self.font_family)
            .field("bold", &self.bold)
            .field("italic", &self.italic)
            .field("underline", &self.underline)
            .field("size", &self.size)
            .field("depth", &self.depth)
            .field("rotation", &self.rotation)
            .field("color", &self.color)
            .field("mode", &self.mode)
            .field("position", &self.position)
            .field("normal", &self.normal)
            .field("uv", &self.uv)
            .field("outline_key", &self.outline_key)
            .finish()
    }
}
