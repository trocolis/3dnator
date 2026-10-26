//! Fronteira do núcleo CAD: detalhes e tipos nativos não saem deste crate.

use std::{error::Error, fmt};

#[derive(Debug)]
pub struct KernelError(pub String);
impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for KernelError {}

pub fn is_available() -> bool {
    cfg!(feature = "occt")
}

// Perfil de furos e nomeação são Rust puro, testados sem o SDK; em produção, só o módulo
// nativo os usa.
#[cfg_attr(not(feature = "occt"), allow(dead_code))]
pub mod hole;
#[cfg_attr(not(feature = "occt"), allow(dead_code))]
mod naming;
#[cfg(feature = "occt")]
#[allow(unsafe_code)]
mod native;
#[cfg(feature = "occt")]
pub use native::{
    Solid, StepAssembly, StepBody, StepOccurrence, StepPart, projected_edges, version,
};

/// Propriedades de massa com densidade uniforme, em mm, mm² e mm³.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MassProperties {
    pub volume: f64,
    pub area: f64,
    pub center: [f64; 3],
}

/// Superfície de uma face do B-rep.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FaceKind {
    /// Plano com normal para fora e distância assinada à origem ao longo dela.
    Plane {
        normal: [f64; 3],
        offset: f64,
    },
    Cylinder {
        radius: f64,
    },
    Cone,
    Sphere {
        radius: f64,
    },
    Torus {
        major_radius: f64,
    },
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceMeasure {
    pub area: f64,
    pub kind: FaceKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeMeasure {
    pub length: f64,
    pub straight: bool,
    /// Raio de arestas circulares.
    pub radius: Option<f64>,
}

/// Nomes posicionais (`{dono}:f{i}`) para sólidos importados, que nunca são recalculados.
pub fn indexed_face_names(owner: &str, count: usize) -> Vec<String> {
    naming::indexed_names(owner, count)
}

/// Não existe núcleo alternativo silencioso: sem SDK, a operação é indisponível.
pub fn require_available() -> Result<(), KernelError> {
    if is_available() {
        Ok(())
    } else {
        Err(KernelError("OCCT não habilitado. Instale o SDK e compile com --features occt. Consulte docs/PROTOTYPE.md.".into()))
    }
}
