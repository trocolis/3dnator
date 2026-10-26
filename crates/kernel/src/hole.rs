//! Perfil de furos simples, passantes, com rebaixo e escareados.
//!
//! O furo é a revolução de um meio-corte em torno do seu eixo. Coordenadas do perfil:
//! `r` (raio, ≥ 0) e `d` (profundidade ao longo da direção de furação, a partir da
//! superfície). O perfil começa 1 mm acima da superfície, para que a ferramenta atravesse a
//! face de entrada sem coincidir com ela, o que tornaria a operação booleana sensível a
//! tolerâncias. Cada segmento tem um papel, que dá nome à face gerada.
use super::KernelError;
use std::f64::consts::PI;

/// Folga acima da superfície de entrada, em mm.
const CLEARANCE_MM: f64 = 1.;

/// Cabeça do furo, na superfície de entrada.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HoleHead {
    Simple,
    /// Rebaixo cilíndrico: diâmetro e profundidade em mm.
    Counterbore {
        diameter: f64,
        depth: f64,
    },
    /// Escareado cônico: diâmetro na superfície (mm) e ângulo total do cone (rad).
    Countersink {
        diameter: f64,
        angle: f64,
    },
}

/// Geometria avaliada de um furo, em mm e radianos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoleGeometry {
    pub diameter: f64,
    /// Profundidade da parede cilíndrica, sem a ponta.
    pub depth: f64,
    pub head: HoleHead,
    /// Ângulo total da ponta de broca (por exemplo 118°); `None` deixa o fundo plano.
    pub tip_angle: Option<f64>,
}

fn positive(value: f64, name: &str) -> Result<(), KernelError> {
    if value.is_finite() && value > 0. {
        Ok(())
    } else {
        Err(KernelError(format!("{name} do furo deve ser positivo.")))
    }
}

fn cone_angle(angle: f64, name: &str) -> Result<(), KernelError> {
    if angle.is_finite() && angle > 0.01 && angle < PI - 0.01 {
        Ok(())
    } else {
        Err(KernelError(format!(
            "{name} do furo deve ficar entre 0° e 180°."
        )))
    }
}

/// Vértices do meio-corte e o papel do segmento que parte de cada vértice até o seguinte
/// (o último fecha no primeiro, ao longo do eixo, sem gerar face).
pub(crate) fn profile(hole: &HoleGeometry) -> Result<Vec<([f64; 2], &'static str)>, KernelError> {
    positive(hole.diameter, "Diâmetro")?;
    positive(hole.depth, "Profundidade")?;
    let radius = hole.diameter / 2.;
    let mut points = Vec::new();
    let top = match hole.head {
        HoleHead::Simple => radius,
        HoleHead::Counterbore { diameter, depth } => {
            positive(diameter, "Diâmetro do rebaixo")?;
            positive(depth, "Profundidade do rebaixo")?;
            if diameter <= hole.diameter || depth >= hole.depth {
                return Err(KernelError(
                    "O rebaixo deve ser mais largo e mais raso que o furo.".into(),
                ));
            }
            diameter / 2.
        }
        HoleHead::Countersink { diameter, angle } => {
            positive(diameter, "Diâmetro do escareado")?;
            cone_angle(angle, "O ângulo do escareado")?;
            if diameter <= hole.diameter {
                return Err(KernelError(
                    "O escareado deve ser mais largo que o furo.".into(),
                ));
            }
            if (diameter - hole.diameter) / 2. / (angle / 2.).tan() >= hole.depth {
                return Err(KernelError(
                    "O escareado é mais profundo que o furo.".into(),
                ));
            }
            diameter / 2.
        }
    };
    points.push(([0., -CLEARANCE_MM], "top"));
    match hole.head {
        HoleHead::Simple => points.push(([radius, -CLEARANCE_MM], "wall")),
        HoleHead::Counterbore { depth, .. } => {
            points.push(([top, -CLEARANCE_MM], "cbore_wall"));
            points.push(([top, depth], "cbore_floor"));
            points.push(([radius, depth], "wall"));
        }
        HoleHead::Countersink { angle, .. } => {
            // A borda do cone continua acima da superfície para cortar a aresta de entrada.
            let slope = (angle / 2.).tan();
            points.push(([top + CLEARANCE_MM * slope, -CLEARANCE_MM], "csink"));
            points.push(([radius, (top - radius) / slope], "wall"));
        }
    }
    match hole.tip_angle {
        Some(angle) => {
            cone_angle(angle, "O ângulo da ponta")?;
            points.push(([radius, hole.depth], "tip"));
            points.push(([0., hole.depth + radius / (angle / 2.).tan()], "axis"));
        }
        None => {
            points.push(([radius, hole.depth], "bottom"));
            points.push(([0., hole.depth], "axis"));
        }
    }
    Ok(points)
}

/// Volume analítico do furo abaixo da superfície (sem a folga), para conferência.
pub fn removed_volume(hole: &HoleGeometry) -> f64 {
    let r = hole.diameter / 2.;
    let cylinder = |radius: f64, height: f64| PI * radius * radius * height;
    let cone = |r1: f64, r2: f64, height: f64| PI * height * (r1 * r1 + r1 * r2 + r2 * r2) / 3.;
    let tip = hole
        .tip_angle
        .map(|a| cone(r, 0., r / (a / 2.).tan()))
        .unwrap_or(0.);
    match hole.head {
        HoleHead::Simple => cylinder(r, hole.depth) + tip,
        HoleHead::Counterbore { diameter, depth } => {
            cylinder(diameter / 2., depth) + cylinder(r, hole.depth - depth) + tip
        }
        HoleHead::Countersink { diameter, angle } => {
            let height = (diameter / 2. - r) / (angle / 2.).tan();
            cone(diameter / 2., r, height) + cylinder(r, hole.depth - height) + tip
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles(hole: &HoleGeometry) -> Result<Vec<&'static str>, KernelError> {
        Ok(profile(hole)?.into_iter().map(|(_, role)| role).collect())
    }

    #[test]
    fn profiles_have_one_role_per_segment() -> Result<(), KernelError> {
        let simple = HoleGeometry {
            diameter: 6.,
            depth: 10.,
            head: HoleHead::Simple,
            tip_angle: Some(118_f64.to_radians()),
        };
        assert_eq!(roles(&simple)?, ["top", "wall", "tip", "axis"]);
        let counterbore = HoleGeometry {
            head: HoleHead::Counterbore {
                diameter: 11.,
                depth: 6.,
            },
            tip_angle: None,
            ..simple
        };
        assert_eq!(
            roles(&counterbore)?,
            ["top", "cbore_wall", "cbore_floor", "wall", "bottom", "axis"]
        );
        let countersink = HoleGeometry {
            head: HoleHead::Countersink {
                diameter: 12.,
                angle: 90_f64.to_radians(),
            },
            tip_angle: None,
            ..simple
        };
        let points = profile(&countersink)?;
        // Cone de 90°: o raio cai 3 mm em 3 mm de profundidade.
        assert!((points[2].0[1] - 3.).abs() < 1e-12);
        assert!((points[2].0[0] - 3.).abs() < 1e-12);
        Ok(())
    }

    #[test]
    fn invalid_heads_are_rejected() {
        let base = HoleGeometry {
            diameter: 6.,
            depth: 10.,
            head: HoleHead::Simple,
            tip_angle: None,
        };
        for head in [
            HoleHead::Counterbore {
                diameter: 5.,
                depth: 2.,
            },
            HoleHead::Counterbore {
                diameter: 10.,
                depth: 12.,
            },
            HoleHead::Countersink {
                diameter: 40.,
                angle: 0.2,
            },
        ] {
            assert!(profile(&HoleGeometry { head, ..base }).is_err());
        }
        assert!(
            profile(&HoleGeometry {
                diameter: -1.,
                ..base
            })
            .is_err()
        );
    }
}
