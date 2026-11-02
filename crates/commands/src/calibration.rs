//! Peça de calibração: uma plaqueta com furos de medida conhecida e encaixes quadrados de
//! folga crescente, mais um pino. Medir o resultado substitui saber qual é a impressora.
use crate::export::File;
use nator_document::DocumentError;
#[cfg(feature = "occt")]
use nator_document::print_profile::{CALIBRATION_FITS, CALIBRATION_HOLES};

/// Lado do pino, em mm; os encaixes têm este lado mais duas vezes a folga.
pub const PLUG_MM: f64 = 8.;

#[cfg(feature = "occt")]
pub fn parts() -> Result<Vec<File>, DocumentError> {
    use nator_kernel::Solid;
    use nator_sketch::{Plane, Sketch, areas};
    const PITCH: f64 = 13.;
    const MARGIN: f64 = 9.;
    const WIDTH: f64 = 2. * MARGIN + 5. * PITCH;
    const HEIGHT: f64 = 32.;
    const THICKNESS: f64 = 3.;
    /// O canto cortado fica do lado dos maiores: é a referência para ler a peça.
    const CORNER: f64 = 5.;
    let failure = |e: nator_sketch::SketchError| DocumentError(e.to_string());
    let core = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let mut plate = Sketch::new("Calibração", Plane::xy());
    let outline = [
        [0., 0.],
        [WIDTH, 0.],
        [WIDTH, HEIGHT - CORNER],
        [WIDTH - CORNER, HEIGHT],
        [0., HEIGHT],
    ];
    for (index, point) in outline.iter().enumerate() {
        plate
            .add_line(*point, outline[(index + 1) % outline.len()], false)
            .map_err(failure)?;
    }
    for (index, diameter) in CALIBRATION_HOLES.iter().enumerate() {
        let center = [MARGIN + index as f64 * PITCH, 22.];
        plate
            .add_circle(center, diameter / 2., false)
            .map_err(failure)?;
    }
    for (index, fit) in CALIBRATION_FITS.iter().enumerate() {
        let half = PLUG_MM / 2. + fit;
        let center = [MARGIN + index as f64 * PITCH, 9.];
        plate
            .add_rectangle(
                [center[0] - half, center[1] - half],
                [center[0] + half, center[1] + half],
                false,
            )
            .map_err(failure)?;
    }
    let regions = areas::regions(&plate).map_err(failure)?;
    let expected = CALIBRATION_HOLES.len() + CALIBRATION_FITS.len();
    let region = regions
        .iter()
        .find(|region| region.holes.len() == expected)
        .ok_or_else(|| DocumentError("Peça de calibração sem os furos previstos.".into()))?;
    let mut solid = Solid::extrude(&plate, region, 0., THICKNESS, "b1").map_err(core)?;
    let mut pin = Sketch::new("Pino", Plane::xy());
    pin.add_rectangle([0., 0.], [PLUG_MM, PLUG_MM], false)
        .map_err(failure)?;
    let regions = areas::regions(&pin).map_err(failure)?;
    let region = regions
        .first()
        .ok_or_else(|| DocumentError("Pino sem área.".into()))?;
    let mut plug = Solid::extrude(&pin, region, 0., 12., "b2").map_err(core)?;
    let file = |name: &str, solid: &mut Solid| -> Result<File, DocumentError> {
        Ok(File {
            name: name.into(),
            mesh: solid.tessellate(0.02, 0.25).map_err(core)?,
            occurrences: 1,
            down: None,
            surface: false,
        })
    };
    Ok(vec![
        file("Calibração", &mut solid)?,
        file("Pino de encaixe", &mut plug)?,
    ])
}

#[cfg(not(feature = "occt"))]
pub fn parts() -> Result<Vec<File>, DocumentError> {
    nator_kernel::require_available().map_err(|e| DocumentError(e.to_string()))?;
    Err(DocumentError("Peça de calibração indisponível.".into()))
}

#[cfg(all(test, feature = "occt"))]
mod tests {
    use super::*;
    use std::{error::Error, f64::consts::PI};

    #[test]
    fn test_piece_is_two_closed_solids_with_every_hole_and_socket() -> Result<(), Box<dyn Error>> {
        let parts = parts()?;
        assert_eq!(parts.len(), 2);
        for part in &parts {
            assert!(part.mesh.topology().has_closed_oriented_edges());
        }
        let holes = CALIBRATION_HOLES
            .iter()
            .map(|d| PI * d * d / 4.)
            .sum::<f64>();
        let sockets = CALIBRATION_FITS
            .iter()
            .map(|fit| (PLUG_MM + 2. * fit).powi(2))
            .sum::<f64>();
        let area = 83. * 32. - 12.5 - holes - sockets;
        // A malha é poligonal: os furos ficam um pouco menores que o círculo exato.
        assert!((parts[0].mesh.signed_volume() - area * 3.).abs() < 1.);
        // A impressão usa o pino de pé: base de 8 × 8 mm e 12 mm de altura.
        assert!((parts[1].mesh.signed_volume() - 768.).abs() < 1e-6);
        Ok(())
    }
}
