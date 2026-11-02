//! Furos para impressão com o núcleo real: volumes exatos, presets ligados ao perfil e a
//! prova de que a gota elimina o balanço de um furo deitado.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session, print_check};
use nator_document::{
    BodyId, Document, PartId, SketchId,
    features::{HoleHeadSpec, HoleSpec, Operation, RegionRef},
    print_profile::{HolePreset, PrintProfile},
};
use nator_geometry::printability::Issue;
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch};
use std::f64::consts::PI;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn sketch(s: &mut Session, part: Option<PartId>, sketch: Sketch) -> Result<(PartId, SketchId)> {
    let Effect::SketchCreated { part, sketch } = s.execute(Command::AddSketch {
        destination: part,
        sketch,
    })?
    else {
        return Err("Rascunho ausente".into());
    };
    Ok((part, sketch))
}
fn created(effect: Effect) -> Result<BodyId> {
    let Effect::Imported { body, .. } = effect else {
        return Err("Corpo ausente".into());
    };
    Ok(body)
}
/// Bloco de 30 × 30 × 10 mm com a base em z = 0.
fn plate() -> Result<(Session, PartId, BodyId)> {
    let mut s = Session::new(Document::new("Furos")?)?;
    let mut profile = Sketch::new("Placa", Plane::xy());
    profile.add_rectangle([0., 0.], [30., 30.], false)?;
    let (part, id) = sketch(&mut s, None, profile)?;
    let body = created(s.execute(Command::AddFeature {
        part,
        name: "Placa".into(),
        operation: Operation::Extrude {
            sketch: id,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "10 mm".into(),
            combine: None,
            until: None,
        },
    })?)?;
    Ok((s, part, body))
}
/// Ponto no centro da face superior, furando para baixo.
fn from_top(s: &mut Session, part: PartId, center: [f64; 2]) -> Result<(SketchId, u64)> {
    let mut points = Sketch::new("Ponto", Plane::xy());
    points.plane.origin = [0., 0., 10.];
    let point = points.add_point(center)?;
    let (_, id) = sketch(s, Some(part), points)?;
    Ok((id, point))
}
/// Ponto na lateral x = 30, furando na horizontal ao longo de -X.
fn from_side(s: &mut Session, part: PartId) -> Result<(SketchId, u64)> {
    let mut points = Sketch::new(
        "Lateral",
        Plane {
            origin: [30., 15., 5.],
            u: [0., 1., 0.],
            v: [0., 0., 1.],
        },
    );
    let point = points.add_point([0., 0.])?;
    let (_, id) = sketch(s, Some(part), points)?;
    Ok((id, point))
}
fn drill(
    s: &mut Session,
    part: PartId,
    base: BodyId,
    (id, point): (SketchId, u64),
    spec: HoleSpec,
) -> Result<BodyId> {
    created(s.execute(Command::AddFeature {
        part,
        name: "Furo".into(),
        operation: Operation::Hole {
            base,
            sketch: id,
            points: vec![point],
            spec,
        },
    })?)
}
fn plain(diameter: &str) -> HoleSpec {
    HoleSpec {
        diameter: diameter.into(),
        depth: None,
        head: HoleHeadSpec::Simple,
        tip_angle: None,
        reverse: false,
        thread: None,
        teardrop: None,
    }
}
fn volume(s: &Session, id: BodyId) -> Result<f64> {
    let body = s.document().body(id).ok_or("Corpo")?;
    Ok(Solid::from_brep(body.brep.as_ref().ok_or("B-rep")?, Some(&body.face_names))?.volume()?)
}
const BLOCK: f64 = 9000.;

#[test]
fn teardrop_removes_the_exact_section_and_the_overhang_of_a_lying_hole() -> Result<()> {
    const RADIUS: f64 = 2.;
    let overhangs = |s: &Session, id: BodyId| -> Result<usize> {
        Ok(print_check::inspect(s.document(), id)?
            .regions
            .iter()
            .filter(|r| r.issue == Issue::Overhang)
            .count())
    };
    let (mut s, part, base) = plate()?;
    let side = from_side(&mut s, part)?;
    let round = drill(&mut s, part, base, side, plain("4 mm"))?;
    assert!(overhangs(&s, round)? > 0);
    assert!(s.undo());
    let drop = drill(
        &mut s,
        part,
        base,
        side,
        HoleSpec {
            teardrop: Some([0., 0., 1.]),
            ..plain("4 mm")
        },
    )?;
    assert_eq!(overhangs(&s, drop)?, 0);
    // Seção: setor de 270° mais o losango até a ponta, ao longo dos 30 mm do bloco.
    let section = (0.75 * PI + 1.) * RADIUS * RADIUS;
    assert!((BLOCK - volume(&s, drop)? - section * 30.).abs() < 1e-6);
    // Fica de pé no projeto salvo.
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("gota.3dnator");
    nator_project_io::write_project(&path, s.document())?;
    let loaded = nator_project_io::read_project(&path)?;
    let feature = loaded
        .part(part)
        .ok_or("Peça")?
        .features
        .last()
        .ok_or("Furo")?;
    assert!(matches!(
        &feature.operation,
        Operation::Hole { spec, .. } if spec.teardrop == Some([0., 0., 1.])
    ));
    Ok(())
}

#[test]
fn a_vertical_hole_ignores_the_teardrop() -> Result<()> {
    let (mut s, part, base) = plate()?;
    let top = from_top(&mut s, part, [15., 15.])?;
    let body = drill(
        &mut s,
        part,
        base,
        top,
        HoleSpec {
            teardrop: Some([0., 0., 1.]),
            ..plain("4 mm")
        },
    )?;
    assert!((BLOCK - volume(&s, body)? - PI * 4. * 10.).abs() < 1e-6);
    Ok(())
}

#[test]
fn captive_nut_cuts_a_hexagon_then_the_screw_hole() -> Result<()> {
    let (mut s, part, base) = plate()?;
    let m3 = s.document().print_profile().fasteners[2].clone();
    assert_eq!(m3.name, "M3");
    let top = from_top(&mut s, part, [15., 15.])?;
    let body = drill(&mut s, part, base, top, HolePreset::CaptiveNut.spec(&m3))?;
    // Perfil genérico: porca 5,5 + 2 × 0,1, altura 2,4 + 0,2, parafuso 3 + 0,8 + 0,1.
    let flats = 5.7_f64;
    let hexagon = 3_f64.sqrt() / 2. * flats * flats;
    let expected = hexagon * 2.6 + PI * 1.95 * 1.95 * 7.4;
    assert!((BLOCK - volume(&s, body)? - expected).abs() < 1e-6);
    Ok(())
}

#[test]
fn recalibrating_the_printer_resizes_holes_made_from_presets() -> Result<()> {
    let (mut s, part, base) = plate()?;
    let m3 = s.document().print_profile().fasteners[2].clone();
    let top = from_top(&mut s, part, [15., 15.])?;
    let body = drill(&mut s, part, base, top, HolePreset::HeatInsert.spec(&m3))?;
    // Inserto M3: Ø 4,2 + 0,1, profundidade 5 + 1.
    let before = volume(&s, body)?;
    assert!((BLOCK - before - PI * 2.15 * 2.15 * 6.).abs() < 1e-6);
    s.execute(Command::SetPrintProfile {
        profile: PrintProfile {
            hole_compensation_mm: 0.3,
            ..Default::default()
        },
    })?;
    assert!((BLOCK - volume(&s, body)? - PI * 2.25 * 2.25 * 6.).abs() < 1e-6);
    Ok(())
}
