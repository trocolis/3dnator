//! Fixação entre peças com o núcleo real: ressalto e rebaixo casados, torres e presilhas com
//! volumes conferidos contra a conta exata, e a recusa do que ficaria solto.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session};
use nator_document::{
    BodyId, Document, PartId, SketchId,
    features::{
        BossSpec, FaceRef, LipKind, LipReference, LipSpec, Operation, RegionRef, SnapKind, SnapSpec,
    },
};
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
fn add(s: &mut Session, part: PartId, operation: Operation) -> Result<BodyId> {
    created(s.execute(Command::AddFeature {
        part,
        name: "Operação".into(),
        operation,
    })?)
}
fn solid(s: &Session, id: BodyId) -> Result<Solid> {
    let body = s.document().body(id).ok_or("Corpo")?;
    Ok(Solid::from_brep(
        body.brep.as_ref().ok_or("B-rep")?,
        Some(&body.face_names),
    )?)
}
fn volume(s: &Session, id: BodyId) -> Result<f64> {
    Ok(solid(s, id)?.volume()?)
}
/// Caixa de 40 × 30 × 20 mm aberta em cima, com paredes de 2 mm.
fn open_box() -> Result<(Session, PartId, BodyId, FaceRef)> {
    let mut s = Session::new(Document::new("Fixação")?)?;
    let mut profile = Sketch::new("Base", Plane::xy());
    profile.add_rectangle([0., 0.], [40., 30.], false)?;
    let (part, id) = sketch(&mut s, None, profile)?;
    let block = add(
        &mut s,
        part,
        Operation::Extrude {
            sketch: id,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "20 mm".into(),
            combine: None,
            until: None,
        },
    )?;
    let top = format!("b{}:end", block.0);
    let shell = add(
        &mut s,
        part,
        Operation::Shell {
            base: block,
            face: FaceRef::Named(top),
            thickness: "2 mm".into(),
            additional_faces: Vec::new(),
            outward: false,
            through: false,
            exit_face: None,
        },
    )?;
    let body = solid(&s, shell)?;
    let rim = (0..body.face_count()?)
        .find(|i| {
            body.face_measure(*i)
                .is_ok_and(|m| (m.area - (40. * 30. - 36. * 26.)).abs() < 1e-6)
        })
        .ok_or("Borda")?;
    let rim = FaceRef::Named(body.face_name(rim)?.to_owned());
    Ok((s, part, shell, rim))
}
fn lip(kind: LipKind, reference: LipReference) -> LipSpec {
    LipSpec {
        kind,
        reference,
        inset: "0 mm".into(),
        width: "1 mm".into(),
        height: "3 mm".into(),
        clearance: "0.2 mm".into(),
    }
}
/// Anel de largura `w` ao redor de um retângulo, com cantos vivos.
fn ring_outside(a: f64, b: f64, w: f64) -> f64 {
    (a + 2. * w) * (b + 2. * w) - a * b
}

#[test]
fn a_lip_follows_the_inner_contour_and_rises_from_the_rim() -> Result<()> {
    let (mut s, part, shell, rim) = open_box()?;
    let before = volume(&s, shell)?;
    let spec = lip(LipKind::Lip, LipReference::Inner);
    let lipped = add(
        &mut s,
        part,
        Operation::Lip {
            base: shell,
            face: rim,
            spec,
        },
    )?;
    let added = volume(&s, lipped)? - before;
    assert!(
        (added - ring_outside(36., 26., 1.) * 3.).abs() < 1e-3,
        "{added}"
    );
    Ok(())
}

#[test]
fn a_groove_is_the_lip_plus_the_clearance_in_width_and_depth() -> Result<()> {
    let (mut s, part, shell, rim) = open_box()?;
    let before = volume(&s, shell)?;
    let spec = lip(LipKind::Groove, LipReference::Inner);
    let grooved = add(
        &mut s,
        part,
        Operation::Lip {
            base: shell,
            face: rim,
            spec,
        },
    )?;
    let removed = before - volume(&s, grooved)?;
    let expected = ring_outside(36., 26., 1.2) * 3.2;
    assert!((removed - expected).abs() < 1e-3, "{removed}");
    Ok(())
}

#[test]
fn the_outer_contour_is_measured_inward() -> Result<()> {
    let (mut s, part, shell, rim) = open_box()?;
    let before = volume(&s, shell)?;
    let spec = lip(LipKind::Lip, LipReference::Outer);
    let lipped = add(
        &mut s,
        part,
        Operation::Lip {
            base: shell,
            face: rim,
            spec,
        },
    )?;
    let added = volume(&s, lipped)? - before;
    let expected = (40. * 30. - 38. * 28.) * 3.;
    assert!((added - expected).abs() < 1e-3, "{added}");
    Ok(())
}

#[test]
fn a_lip_wider_than_the_wall_is_refused() -> Result<()> {
    let (mut s, part, shell, rim) = open_box()?;
    let mut spec = lip(LipKind::Lip, LipReference::Inner);
    spec.width = "3 mm".into();
    let error = add(
        &mut s,
        part,
        Operation::Lip {
            base: shell,
            face: rim,
            spec,
        },
    )
    .err()
    .ok_or("Erro")?;
    assert!(error.to_string().contains("borda"), "{error}");
    Ok(())
}

fn tower() -> BossSpec {
    BossSpec {
        outer_diameter: "6 mm".into(),
        hole_diameter: "2.5 mm".into(),
        height: "10 mm".into(),
        hole_depth: None,
        fillet: "0 mm".into(),
        reverse: false,
    }
}
/// Dois pontos no piso interno da caixa, em z = 2.
fn floor_points(s: &mut Session, part: PartId) -> Result<(SketchId, Vec<u64>)> {
    let mut points = Sketch::new("Pontos", Plane::xy());
    points.plane.origin = [0., 0., 2.];
    let a = points.add_point([10., 15.])?;
    let b = points.add_point([30., 15.])?;
    let (_, id) = sketch(s, Some(part), points)?;
    Ok((id, vec![a, b]))
}

#[test]
fn towers_rise_from_the_floor_with_the_hole_from_the_top() -> Result<()> {
    let (mut s, part, shell, _) = open_box()?;
    let before = volume(&s, shell)?;
    let (id, points) = floor_points(&mut s, part)?;
    let towers = add(
        &mut s,
        part,
        Operation::ScrewBoss {
            base: shell,
            sketch: id,
            points,
            spec: tower(),
        },
    )?;
    let ring = PI * (3. * 3. - 1.25 * 1.25);
    let added = volume(&s, towers)? - before;
    assert!((added - 2. * ring * 10.).abs() < 1e-6, "{added}");
    Ok(())
}

#[test]
fn a_tower_with_a_shallow_hole_and_a_fillet_adds_the_exact_flare() -> Result<()> {
    let (mut s, part, shell, _) = open_box()?;
    let before = volume(&s, shell)?;
    let (id, points) = floor_points(&mut s, part)?;
    let mut spec = tower();
    spec.hole_depth = Some("6 mm".into());
    spec.fillet = "1 mm".into();
    let towers = add(
        &mut s,
        part,
        Operation::ScrewBoss {
            base: shell,
            sketch: id,
            points: vec![points[0]],
            spec,
        },
    )?;
    // Teorema de Pappus para a concordância: seção 1 - π/4, com centroide a
    // 3 + 0,2234 mm do eixo (distância do canto: (10 - 3π) / (12 - 3π) × 1 mm).
    let section = 1. - PI / 4.;
    let centroid = 3. + (10. - 3. * PI) / (12. - 3. * PI);
    let flare = 2. * PI * centroid * section;
    let body = PI * 9. * 10. - PI * 1.25 * 1.25 * 6.;
    let added = volume(&s, towers)? - before;
    assert!((added - (body + flare)).abs() < 1e-5, "{added}");
    Ok(())
}

#[test]
fn a_tower_that_floats_away_from_the_body_is_refused() -> Result<()> {
    let (mut s, part, shell, _) = open_box()?;
    let mut points = Sketch::new("Alto", Plane::xy());
    points.plane.origin = [0., 0., 40.];
    let point = points.add_point([20., 15.])?;
    let (_, id) = sketch(&mut s, Some(part), points)?;
    let error = add(
        &mut s,
        part,
        Operation::ScrewBoss {
            base: shell,
            sketch: id,
            points: vec![point],
            spec: tower(),
        },
    )
    .err()
    .ok_or("Erro")?;
    assert!(error.to_string().contains("não encosta"), "{error}");
    Ok(())
}

fn hook(kind: SnapKind) -> SnapSpec {
    SnapSpec {
        kind,
        length: "12 mm".into(),
        thickness: "2 mm".into(),
        width: "6 mm".into(),
        overhang: "1 mm".into(),
        head: "2 mm".into(),
        fillet: "0 mm".into(),
        angle: "0 deg".into(),
        clearance: "0.2 mm".into(),
        window: "3 mm".into(),
        reverse: false,
    }
}

#[test]
fn a_hook_adds_its_beam_and_head() -> Result<()> {
    let (mut s, part, shell, _) = open_box()?;
    let before = volume(&s, shell)?;
    let (id, points) = floor_points(&mut s, part)?;
    let hooked = add(
        &mut s,
        part,
        Operation::SnapFit {
            base: shell,
            sketch: id,
            points: vec![points[0]],
            spec: hook(SnapKind::Hook),
        },
    )?;
    let section = 2. * 14. + 1. * 2. / 2.;
    let added = volume(&s, hooked)? - before;
    assert!((added - section * 6.).abs() < 1e-6, "{added}");
    Ok(())
}

#[test]
fn a_root_fillet_adds_two_flares_and_changes_nothing_else() -> Result<()> {
    let (mut s, part, shell, _) = open_box()?;
    let before = volume(&s, shell)?;
    let (id, points) = floor_points(&mut s, part)?;
    let mut spec = hook(SnapKind::Hook);
    spec.fillet = "1 mm".into();
    let hooked = add(
        &mut s,
        part,
        Operation::SnapFit {
            base: shell,
            sketch: id,
            points: vec![points[0]],
            spec,
        },
    )?;
    let section = 2. * 14. + 1. + 2. * (1. - PI / 4.);
    let added = volume(&s, hooked)? - before;
    assert!((added - section * 6.).abs() < 1e-6, "{added}");
    Ok(())
}

#[test]
fn the_window_clears_the_head_of_its_hook_and_opens_the_wall() -> Result<()> {
    // Parede em x de 38 a 40: a presilha no piso sobe e a cabeça aponta para ela.
    let (mut s, part, shell, _) = open_box()?;
    let before = volume(&s, shell)?;
    let mut points = Sketch::new("Parede", Plane::xy());
    points.plane.origin = [0., 0., 2.];
    let point = points.add_point([34.8, 15.])?;
    let (_, id) = sketch(&mut s, Some(part), points)?;
    let window = add(
        &mut s,
        part,
        Operation::SnapFit {
            base: shell,
            sketch: id,
            points: vec![point],
            spec: hook(SnapKind::Window),
        },
    )?;
    // Janela em x de 36,6 a 40,8, z de 13,8 a 16,2 e 6,4 de largura. Só a parede é cortada.
    let removed = before - volume(&s, window)?;
    assert!((removed - 2. * 2.4 * 6.4).abs() < 1e-6, "{removed}");
    Ok(())
}

#[test]
fn a_window_that_misses_the_body_is_refused() -> Result<()> {
    let (mut s, part, shell, _) = open_box()?;
    let (id, points) = floor_points(&mut s, part)?;
    let error = add(
        &mut s,
        part,
        Operation::SnapFit {
            base: shell,
            sketch: id,
            points: vec![points[0]],
            spec: hook(SnapKind::Window),
        },
    )
    .err()
    .ok_or("Erro")?;
    assert!(error.to_string().contains("não alcança"), "{error}");
    Ok(())
}

#[test]
fn the_strain_estimate_follows_the_cantilever_formula() {
    use nator_document::features::SnapSpec as S;
    assert!((S::strain(2., 1., 12.) - 1.5 * 2. / 144.).abs() < 1e-12);
    // Dobrar o comprimento divide a deformação por quatro.
    assert!((S::strain(2., 1., 6.) / S::strain(2., 1., 12.) - 4.).abs() < 1e-12);
}

mod history {
    use super::*;
    /// Caixa com as quatro operações novas citando o perfil.
    fn complete() -> Result<(Session, PartId, BodyId)> {
        let (mut s, part, shell, rim) = open_box()?;
        let (id, points) = floor_points(&mut s, part)?;
        let mut spec = lip(LipKind::Groove, LipReference::Inner);
        spec.clearance = "folga_deslizante".into();
        let body = add(
            &mut s,
            part,
            Operation::Lip {
                base: shell,
                face: rim,
                spec,
            },
        )?;
        let mut boss = tower();
        boss.hole_diameter = "2.5 mm + compensacao_furo".into();
        let body = add(
            &mut s,
            part,
            Operation::ScrewBoss {
                base: body,
                sketch: id,
                points: points[..1].to_vec(),
                spec: boss,
            },
        )?;
        let body = add(
            &mut s,
            part,
            Operation::SnapFit {
                base: body,
                sketch: id,
                points: points[1..].to_vec(),
                spec: hook(SnapKind::Hook),
            },
        )?;
        Ok((s, part, body))
    }

    #[test]
    fn recalibrating_the_profile_reaches_the_new_operations() -> Result<()> {
        let (mut s, _, body) = complete()?;
        let before = volume(&s, body)?;
        let mut profile = s.document().print_profile().clone();
        profile.fit_sliding_mm = 0.3;
        s.execute(Command::SetPrintProfile { profile })?;
        let after = volume(&s, body)?;
        // Rebaixo de 1,3 mm com 3,3 mm de profundidade no lugar de 1,2 mm com 3,2 mm.
        let groove = ring_outside(36., 26., 1.3) * 3.3 - ring_outside(36., 26., 1.2) * 3.2;
        assert!(
            ((before - after) - groove).abs() < 1e-2,
            "{}",
            before - after
        );
        let mut profile = s.document().print_profile().clone();
        profile.hole_compensation_mm = 0.5;
        s.execute(Command::SetPrintProfile { profile })?;
        let bigger = |d: f64| PI * d * d / 4. * 10.;
        let bore = bigger(3.0) - bigger(2.6);
        assert!(((after - volume(&s, body)?) - bore).abs() < 1e-2);
        Ok(())
    }

    #[test]
    fn the_operations_survive_saving_and_reopening_with_the_same_volume() -> Result<()> {
        let (s, part, body) = complete()?;
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("caixa.3dnator");
        nator_project_io::write_project(&path, s.document())?;
        let read = nator_project_io::read_project(&path)?;
        let names = |doc: &Document| -> Result<Vec<String>> {
            let piece = doc.part(part).ok_or("Peça")?;
            Ok(piece
                .features
                .iter()
                .map(|f| format!("{:?}", f.operation))
                .collect())
        };
        assert_eq!(names(&read)?, names(s.document())?);
        let saved = Session::new(read)?;
        assert!((volume(&saved, body)? - volume(&s, body)?).abs() < 1e-9);
        Ok(())
    }
}
