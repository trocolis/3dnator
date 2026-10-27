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
#[cfg(any(feature = "occt", test))]
mod tessellation;
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

/// Pontos de apoio de cotas, em mm no referencial do sólido consultado. Superfícies
/// cilíndricas/esféricas têm centro e ponto radial; outras fornecem somente o centroide.
#[derive(Debug, Clone, Copy)]
pub struct FaceAnchors {
    pub centroid: [f64; 3],
    pub radial: Option<[[f64; 3]; 2]>,
}
#[derive(Debug, Clone, Copy)]
pub struct EdgeAnchors {
    pub start: [f64; 3],
    pub end: [f64; 3],
    /// Metade do comprimento percorrido sobre a curva, em mm; não é o centro da corda.
    pub midpoint: [f64; 3],
    pub circle: Option<CircleAnchor>,
    /// Costura da parametrização ou polo degenerado, sem canto capturável.
    pub seam: bool,
}

/// Círculo de apoio no referencial local do sólido, em mm. O centro fica no
/// plano da borda, inclusive quando esse plano atravessa o vazio de um furo.
#[derive(Debug, Clone, Copy)]
pub struct CircleAnchor {
    pub center: [f64; 3],
    pub normal: [f64; 3],
    /// Direção unitária do primeiro quadrante, no referencial da própria geometria.
    pub x_direction: [f64; 3],
    pub radius_mm: f64,
    /// Intervalo angular da curva, em radianos na base X/Y do círculo de apoio.
    pub start_angle_rad: f64,
    pub sweep_rad: f64,
    /// Círculos completos oferecem quatro quadrantes; arcos conservam seus extremos.
    pub closed: bool,
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
