//! Perfil de impressão FDM: só grandezas físicas, sem marcas de impressora ou fatiador.
//! O projeto guarda sua cópia para abrir igual em outra máquina, e os comprimentos entram
//! nas expressões de todas as peças como valores herdados.
use crate::{
    DocumentError,
    features::{HoleHeadSpec, HoleSpec},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const BED_RANGE_MM: std::ops::RangeInclusive<f64> = 10.0..=2000.0;
pub const NOZZLE_RANGE_MM: std::ops::RangeInclusive<f64> = 0.05..=5.0;
pub const LAYER_RANGE_MM: std::ops::RangeInclusive<f64> = 0.01..=2.0;
pub const FIT_RANGE_MM: std::ops::RangeInclusive<f64> = 0.0..=5.0;
pub const OVERHANG_RANGE_DEG: std::ops::RangeInclusive<f64> = 10.0..=89.0;
pub const DENSITY_RANGE_G_CM3: std::ops::RangeInclusive<f64> = 0.1..=20.0;
pub const PRICE_RANGE: std::ops::RangeInclusive<f64> = 0.0..=100_000.0;
pub const INFILL_RANGE_PCT: std::ops::RangeInclusive<f64> = 0.0..=100.0;
pub const WALL_LINES_RANGE: std::ops::RangeInclusive<f64> = 1.0..=20.0;
pub const STRAIN_RANGE_PCT: std::ops::RangeInclusive<f64> = 0.1..=50.0;

/// Furos nominais da peça de calibração, em mm, na ordem em que aparecem.
pub const CALIBRATION_HOLES: [f64; 5] = [3., 4., 5., 6., 8.];
/// Folgas por lado dos encaixes da peça de calibração, em mm.
pub const CALIBRATION_FITS: [f64; 6] = [0.05, 0.1, 0.15, 0.2, 0.3, 0.4];
/// Limite de fixadores por perfil.
const MAX_FASTENERS: usize = 64;

/// Medidas de um fixador, em mm. Parafusos e porcas seguem normas; insertos variam por
/// fabricante, e por isso a tabela é editável.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fastener {
    pub name: String,
    pub diameter_mm: f64,
    /// Furo para parafuso autoatarrachante em plástico.
    pub pilot_mm: f64,
    pub insert_diameter_mm: f64,
    pub insert_length_mm: f64,
    /// Medida da porca entre faces.
    pub nut_flats_mm: f64,
    pub nut_height_mm: f64,
    pub head_diameter_mm: f64,
    pub head_height_mm: f64,
}
impl Fastener {
    /// Campos numéricos, na ordem do painel.
    pub fn sizes_mut(&mut self) -> [&mut f64; 8] {
        [
            &mut self.diameter_mm,
            &mut self.pilot_mm,
            &mut self.insert_diameter_mm,
            &mut self.insert_length_mm,
            &mut self.nut_flats_mm,
            &mut self.nut_height_mm,
            &mut self.head_diameter_mm,
            &mut self.head_height_mm,
        ]
    }
    fn validate(&self) -> Result<(), DocumentError> {
        let sizes = [
            self.diameter_mm,
            self.pilot_mm,
            self.insert_diameter_mm,
            self.insert_length_mm,
            self.nut_flats_mm,
            self.nut_height_mm,
            self.head_diameter_mm,
            self.head_height_mm,
        ];
        if self.name.trim().is_empty()
            || self.name.len() > 32
            || !sizes.iter().all(|v| (0.1..=100.).contains(v))
        {
            return Err(DocumentError(format!(
                "Medidas inválidas no fixador {}.",
                self.name
            )));
        }
        Ok(())
    }
}

fn fastener(name: &str, sizes: [f64; 8]) -> Fastener {
    let [
        diameter_mm,
        pilot_mm,
        insert_diameter_mm,
        insert_length_mm,
        nut_flats_mm,
        nut_height_mm,
        head_diameter_mm,
        head_height_mm,
    ] = sizes;
    Fastener {
        name: name.into(),
        diameter_mm,
        pilot_mm,
        insert_diameter_mm,
        insert_length_mm,
        nut_flats_mm,
        nut_height_mm,
        head_diameter_mm,
        head_height_mm,
    }
}

/// Parafusos ISO 4762, porcas ISO 4032 e insertos curtos comuns.
fn default_fasteners() -> Vec<Fastener> {
    vec![
        fastener("M2", [2., 1.7, 3.2, 4., 4., 1.6, 3.8, 2.]),
        fastener("M2.5", [2.5, 2.1, 3.6, 4., 5., 2., 4.5, 2.5]),
        fastener("M3", [3., 2.6, 4.2, 5., 5.5, 2.4, 5.5, 3.]),
        fastener("M4", [4., 3.4, 5.6, 6., 7., 3.2, 7., 4.]),
        fastener("M5", [5., 4.3, 6.4, 7., 8., 4.7, 8.5, 5.]),
    ]
}

/// Furos prontos para fixação em peças impressas. As expressões citam o perfil, então
/// recalibrar a impressora ajusta os furos já feitos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolePreset {
    Through,
    RecessedHead,
    SelfTapping,
    HeatInsert,
    CaptiveNut,
}
impl HolePreset {
    pub const ALL: [Self; 5] = [
        Self::Through,
        Self::RecessedHead,
        Self::SelfTapping,
        Self::HeatInsert,
        Self::CaptiveNut,
    ];
    pub fn spec(self, fastener: &Fastener) -> HoleSpec {
        let clearance = |size: f64| format!("{size} mm + 2 * folga_livre + compensacao_furo");
        let (diameter, depth, head) = match self {
            Self::Through => (clearance(fastener.diameter_mm), None, HoleHeadSpec::Simple),
            Self::RecessedHead => (
                clearance(fastener.diameter_mm),
                None,
                HoleHeadSpec::Counterbore {
                    diameter: clearance(fastener.head_diameter_mm),
                    depth: format!("{} mm + camada", fastener.head_height_mm),
                },
            ),
            Self::SelfTapping => (
                format!("{} mm + compensacao_furo", fastener.pilot_mm),
                Some(format!("{} mm", 2.5 * fastener.diameter_mm)),
                HoleHeadSpec::Simple,
            ),
            // O milímetro extra acolhe o plástico que o inserto empurra ao entrar.
            Self::HeatInsert => (
                format!("{} mm + compensacao_furo", fastener.insert_diameter_mm),
                Some(format!("{} mm + 1 mm", fastener.insert_length_mm)),
                HoleHeadSpec::Simple,
            ),
            Self::CaptiveNut => (
                clearance(fastener.diameter_mm),
                None,
                HoleHeadSpec::Nut {
                    across_flats: format!("{} mm + 2 * folga_justa", fastener.nut_flats_mm),
                    depth: format!("{} mm + camada", fastener.nut_height_mm),
                },
            ),
        };
        HoleSpec {
            diameter,
            depth,
            head,
            tip_angle: None,
            reverse: false,
            thread: None,
            teardrop: None,
        }
    }
}

/// Resultado medido na peça de calibração impressa.
#[derive(Debug, Clone, PartialEq)]
pub struct Calibration {
    /// Diâmetro medido de cada furo de `CALIBRATION_HOLES`.
    pub holes: [f64; 5],
    /// Posições em `CALIBRATION_FITS` do primeiro encaixe justo, deslizante e livre.
    pub fits: [usize; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintProfile {
    pub name: String,
    pub nozzle_mm: f64,
    pub layer_mm: f64,
    /// Folgas por lado entre peças: encaixe justo, deslizante e livre.
    pub fit_tight_mm: f64,
    pub fit_sliding_mm: f64,
    pub fit_free_mm: f64,
    /// Acréscimo no diâmetro dos furos, que costumam sair menores que o modelo.
    pub hole_compensation_mm: f64,
    pub max_overhang_deg: f64,
    /// Volume de impressão; `None` deixa de avaliar o tamanho da peça.
    pub bed_mm: Option<[f64; 3]>,
    pub density_g_cm3: f64,
    /// Preço do filamento por quilo, na moeda de quem preenche; zero deixa o custo de fora.
    pub price_per_kg: f64,
    /// Preenchimento e voltas de parede usados na estimativa de massa.
    pub infill_pct: f64,
    pub wall_lines: f64,
    /// Deformação admitida em presilhas: acima dela, o painel avisa.
    pub max_strain_pct: f64,
    pub fasteners: Vec<Fastener>,
}
impl Default for PrintProfile {
    /// Regras práticas para bico comum, não medidas de uma máquina: a calibração as substitui.
    fn default() -> Self {
        Self {
            name: "FDM genérico".into(),
            nozzle_mm: 0.4,
            layer_mm: 0.2,
            fit_tight_mm: 0.1,
            fit_sliding_mm: 0.2,
            fit_free_mm: 0.4,
            hole_compensation_mm: 0.1,
            max_overhang_deg: 45.,
            bed_mm: None,
            density_g_cm3: 1.24,
            price_per_kg: 0.,
            infill_pct: 20.,
            wall_lines: 2.,
            max_strain_pct: 2.5,
            fasteners: default_fasteners(),
        }
    }
}
impl PrintProfile {
    /// Perfil ajustado ao que a impressora entregou. Furos maiores que o modelo não pedem
    /// compensação negativa: nesse caso a compensação fica em zero.
    pub fn calibrated(&self, measured: &Calibration) -> Result<Self, DocumentError> {
        let mut sum = 0.;
        for (nominal, hole) in CALIBRATION_HOLES.iter().zip(measured.holes) {
            if !hole.is_finite() || hole < nominal * 0.5 || hole > nominal * 1.5 {
                return Err(DocumentError(format!(
                    "A medida do furo de {nominal} mm parece errada."
                )));
            }
            sum += nominal - hole;
        }
        let mean = sum / CALIBRATION_HOLES.len() as f64;
        let [tight, sliding, free] = measured
            .fits
            .map(|index| CALIBRATION_FITS.get(index).copied());
        let (Some(tight), Some(sliding), Some(free)) = (tight, sliding, free) else {
            return Err(DocumentError("Encaixe inexistente na calibração.".into()));
        };
        if tight > sliding || sliding > free {
            return Err(DocumentError(
                "Os encaixes devem ir do mais justo para o mais livre.".into(),
            ));
        }
        let profile = Self {
            hole_compensation_mm: (mean.max(0.) * 100.).round() / 100.,
            fit_tight_mm: tight,
            fit_sliding_mm: sliding,
            fit_free_mm: free,
            ..self.clone()
        };
        profile.validate()?;
        Ok(profile)
    }
    pub fn validate(&self) -> Result<(), DocumentError> {
        let inside = |value: f64, range: &std::ops::RangeInclusive<f64>, label: &str| {
            if range.contains(&value) {
                Ok(())
            } else {
                Err(DocumentError(format!(
                    "{label} deve ficar entre {} e {}.",
                    range.start(),
                    range.end()
                )))
            }
        };
        if self.name.trim().is_empty() || self.name.len() > 128 {
            return Err(DocumentError("Nome do perfil inválido.".into()));
        }
        inside(self.nozzle_mm, &NOZZLE_RANGE_MM, "O bico")?;
        inside(self.layer_mm, &LAYER_RANGE_MM, "A camada")?;
        inside(self.fit_tight_mm, &FIT_RANGE_MM, "A folga justa")?;
        inside(self.fit_sliding_mm, &FIT_RANGE_MM, "A folga deslizante")?;
        inside(self.fit_free_mm, &FIT_RANGE_MM, "A folga livre")?;
        inside(
            self.hole_compensation_mm,
            &FIT_RANGE_MM,
            "A compensação de furo",
        )?;
        inside(self.max_overhang_deg, &OVERHANG_RANGE_DEG, "O balanço")?;
        inside(self.density_g_cm3, &DENSITY_RANGE_G_CM3, "A densidade")?;
        inside(self.price_per_kg, &PRICE_RANGE, "O preço")?;
        inside(self.infill_pct, &INFILL_RANGE_PCT, "O preenchimento")?;
        inside(self.wall_lines, &WALL_LINES_RANGE, "As voltas de parede")?;
        inside(self.max_strain_pct, &STRAIN_RANGE_PCT, "A deformação")?;
        for side in self.bed_mm.iter().flatten() {
            inside(*side, &BED_RANGE_MM, "A mesa")?;
        }
        if self.fasteners.len() > MAX_FASTENERS {
            return Err(DocumentError("Fixadores demais no perfil.".into()));
        }
        self.fasteners.iter().try_for_each(Fastener::validate)
    }
    /// Nomes usados nas expressões, em mm. A parede mínima deriva do bico.
    pub fn parameters(&self) -> BTreeMap<String, f64> {
        [
            ("bico", self.nozzle_mm),
            ("camada", self.layer_mm),
            ("parede", 2. * self.nozzle_mm),
            ("folga_justa", self.fit_tight_mm),
            ("folga_deslizante", self.fit_sliding_mm),
            ("folga_livre", self.fit_free_mm),
            ("compensacao_furo", self.hole_compensation_mm),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect()
    }
    /// Nomes cujo valor difere entre os dois perfis.
    pub fn changed_parameters(&self, other: &Self) -> Vec<String> {
        let next = other.parameters();
        self.parameters()
            .into_iter()
            .filter(|(name, value)| next.get(name) != Some(value))
            .map(|(name, _)| name)
            .collect()
    }
}

/// Procura `name` como identificador inteiro: `parede` não casa com `parede_externa`.
pub(crate) fn mentions(text: &str, name: &str) -> bool {
    let identifier = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(name).any(|(start, _)| {
        !text[..start].chars().next_back().is_some_and(identifier)
            && !text[start + name.len()..]
                .chars()
                .next()
                .is_some_and(identifier)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parameters::{Dimension, Parameters};

    #[test]
    fn profile_values_reach_expressions_and_part_parameters_win() -> Result<(), DocumentError> {
        let mut table = Parameters {
            inherited: PrintProfile::default().parameters(),
            ..Default::default()
        };
        let value = table.evaluate("parede + folga_livre * 2", Dimension::Length)?;
        assert!((value - 1.6).abs() < 1e-12);
        table.set("parede".into(), "3 mm".into(), Dimension::Length)?;
        assert!((table.evaluate("parede", Dimension::Length)? - 3.).abs() < 1e-12);
        assert!(table.evaluate("bico", Dimension::Angle).is_err());
        assert!(
            Parameters::default()
                .evaluate("bico", Dimension::Length)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn invalid_profiles_are_refused_and_only_changed_names_are_reported() {
        assert!(PrintProfile::default().validate().is_ok());
        for broken in [
            PrintProfile {
                nozzle_mm: 0.,
                ..Default::default()
            },
            PrintProfile {
                layer_mm: f64::NAN,
                ..Default::default()
            },
            PrintProfile {
                fit_free_mm: -1.,
                ..Default::default()
            },
            PrintProfile {
                bed_mm: Some([200., 200., 0.]),
                ..Default::default()
            },
            PrintProfile {
                name: " ".into(),
                ..Default::default()
            },
        ] {
            assert!(broken.validate().is_err());
        }
        let wider = PrintProfile {
            nozzle_mm: 0.6,
            ..Default::default()
        };
        assert_eq!(
            PrintProfile::default().changed_parameters(&wider),
            vec!["bico".to_owned(), "parede".to_owned()]
        );
        let renamed = PrintProfile {
            name: "Outro".into(),
            ..Default::default()
        };
        assert!(
            PrintProfile::default()
                .changed_parameters(&renamed)
                .is_empty()
        );
    }

    #[test]
    fn calibration_turns_measurements_into_the_profile() -> Result<(), DocumentError> {
        let measured = Calibration {
            holes: [2.8, 3.8, 4.8, 5.8, 7.8],
            fits: [1, 3, 5],
        };
        let result = PrintProfile::default().calibrated(&measured)?;
        assert!((result.hole_compensation_mm - 0.2).abs() < 1e-12);
        assert_eq!(
            (
                result.fit_tight_mm,
                result.fit_sliding_mm,
                result.fit_free_mm
            ),
            (0.1, 0.2, 0.4)
        );
        // Furos maiores que o modelo não viram compensação negativa.
        let oversized = Calibration {
            holes: [3.1, 4.1, 5.1, 6.1, 8.1],
            ..measured.clone()
        };
        let result = PrintProfile::default().calibrated(&oversized)?;
        assert_eq!(result.hole_compensation_mm, 0.);
        // Erros de digitação e encaixes fora de ordem são recusados.
        for wrong in [
            Calibration {
                holes: [30., 4., 5., 6., 8.],
                ..measured.clone()
            },
            Calibration {
                fits: [3, 1, 5],
                ..measured.clone()
            },
            Calibration {
                fits: [1, 3, 9],
                ..measured
            },
        ] {
            assert!(PrintProfile::default().calibrated(&wrong).is_err());
        }
        Ok(())
    }

    #[test]
    fn presets_cite_the_profile_and_evaluate_to_printable_sizes() -> Result<(), DocumentError> {
        let profile = PrintProfile::default();
        let table = Parameters {
            inherited: profile.parameters(),
            ..Default::default()
        };
        let m3 = profile.fasteners.iter().find(|f| f.name == "M3");
        let m3 = m3.ok_or_else(|| DocumentError("M3".into()))?;
        let diameter =
            |preset: HolePreset| table.evaluate(&preset.spec(m3).diameter, Dimension::Length);
        // 3 mm + 2 × 0,4 + 0,1: o parafuso passa com folga.
        assert!((diameter(HolePreset::Through)? - 3.9).abs() < 1e-9);
        assert!((diameter(HolePreset::SelfTapping)? - 2.7).abs() < 1e-9);
        assert!((diameter(HolePreset::HeatInsert)? - 4.3).abs() < 1e-9);
        let nut = HolePreset::CaptiveNut.spec(m3);
        let HoleSpec {
            head: HoleHeadSpec::Nut { across_flats, .. },
            depth: None,
            ..
        } = nut
        else {
            return Err(DocumentError("Porca".into()));
        };
        assert!((table.evaluate(&across_flats, Dimension::Length)? - 5.7).abs() < 1e-9);
        // Recalibrar muda o furo sem refazer a operação.
        let wider = Parameters {
            inherited: PrintProfile {
                hole_compensation_mm: 0.3,
                ..Default::default()
            }
            .parameters(),
            ..Default::default()
        };
        let spec = HolePreset::Through.spec(m3);
        assert!((wider.evaluate(&spec.diameter, Dimension::Length)? - 4.1).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn fastener_table_is_validated() {
        let mut profile = PrintProfile::default();
        assert_eq!(profile.fasteners.len(), 5);
        profile.fasteners[0].pilot_mm = 0.;
        assert!(profile.validate().is_err());
    }

    #[test]
    fn mentions_match_whole_identifiers() {
        assert!(mentions("\"parede + 1 mm\"", "parede"));
        assert!(!mentions("parede_externa", "parede"));
        assert!(!mentions("minha_parede", "parede"));
    }
}
