//! Dutos preservam aberturas e referências durante recálculo, clone e reabertura.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session};
use nator_document::{
    BodyId, Document,
    features::{FaceRef, Operation, RegionRef},
    parameters::Dimension,
};
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn volume(session: &Session, id: BodyId) -> Result<f64> {
    let body = session.document().body(id).ok_or("Corpo")?;
    Ok(Solid::from_brep(body.brep.as_ref().ok_or("B-rep")?, Some(&body.face_names))?.volume()?)
}
#[test]
fn through_shell_uses_opposite_exit_and_tracks_thickness_and_project_roundtrip() -> Result<()> {
    let mut sketch = Sketch::new("Caixa", Plane::xy());
    sketch.add_rectangle([0., 0.], [20., 20.], false)?;
    let mut doc = Document::new("Duto")?;
    let (part, sketch) = doc.add_sketch(None, sketch)?;
    let mut session = Session::new(doc)?;
    let Effect::Imported { body: base, .. } = session.execute(Command::AddFeature {
        part,
        name: "Caixa".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "20 mm".into(),
            combine: None,
            until: None,
        },
    })?
    else {
        return Err("Base".into());
    };
    session.execute(Command::SetParameter {
        part,
        name: "parede".into(),
        expression: Some("2 mm".into()),
        dimension: Dimension::Length,
    })?;
    let entry = FaceRef::Named(format!("b{}:end", base.0));
    let operation = Operation::Shell {
        base,
        face: entry.clone(),
        thickness: "parede".into(),
        additional_faces: vec![],
        outward: false,
        through: true,
        exit_face: None,
    };
    let Effect::Imported { body: duct, .. } = session.execute(Command::AddFeature {
        part,
        name: "Duto".into(),
        operation: operation.clone(),
    })?
    else {
        return Err("Duto".into());
    };
    assert!((volume(&session, duct)? - 2880.).abs() < 1e-5);
    session.execute(Command::SetParameter {
        part,
        name: "parede".into(),
        expression: Some("3 mm".into()),
        dimension: Dimension::Length,
    })?;
    assert!((volume(&session, duct)? - 4080.).abs() < 1e-5);
    assert!(session.undo());
    assert!((volume(&session, duct)? - 2880.).abs() < 1e-5);
    assert!(session.redo());
    // Saída lateral explícita prevalece sobre a tampa oposta inferida.
    let base_body = session.document().body(base).ok_or("Base")?;
    let solid = Solid::from_brep(
        base_body.brep.as_ref().ok_or("B-rep")?,
        Some(&base_body.face_names),
    )?;
    let side=(0..solid.face_count()?).find(|i|matches!(solid.face_measure(*i).map(|m|m.kind),Ok(nator_kernel::FaceKind::Plane {normal,..}) if normal[0]>0.9)).ok_or("Lateral")?;
    let mut elbow = operation.clone();
    if let Operation::Shell { exit_face, .. } = &mut elbow {
        *exit_face = Some(FaceRef::Named(solid.face_name(side)?.into()));
    }
    session.execute(Command::EditFeature {
        part,
        output: duct,
        operation: elbow,
    })?;
    assert!((volume(&session, duct)? - (8000. - 17. * 14. * 17.)).abs() < 1e-5);
    // Saída escolhida explicitamente prevalece, mas não pode ser a entrada.
    let mut invalid = operation.clone();
    if let Operation::Shell { exit_face, .. } = &mut invalid {
        *exit_face = Some(entry);
    }
    let revision = session.revision();
    assert!(
        session
            .execute(Command::EditFeature {
                part,
                output: duct,
                operation: invalid
            })
            .is_err()
    );
    assert_eq!(session.revision(), revision);
    let mut explicit = operation;
    if let Operation::Shell { exit_face, .. } = &mut explicit {
        *exit_face = Some(FaceRef::Named(format!("b{}:start", base.0)));
    }
    session.execute(Command::EditFeature {
        part,
        output: duct,
        operation: explicit,
    })?;
    assert!((volume(&session, duct)? - 4080.).abs() < 1e-5);
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("duto.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let mut loaded = Session::new(nator_project_io::read_project(&path)?)?;
    loaded.execute(Command::SetParameter {
        part,
        name: "parede".into(),
        expression: Some("2 mm".into()),
        dimension: Dimension::Length,
    })?;
    assert!((volume(&loaded, duct)? - 2880.).abs() < 1e-5);
    let mut cloned = loaded.document().clone();
    let copy = cloned.clone_part(part, "Cópia".into())?;
    let feature = cloned
        .part(copy)
        .ok_or("Cópia")?
        .features
        .last()
        .ok_or("Casca")?;
    assert!(
        matches!(&feature.operation,Operation::Shell {exit_face:Some(FaceRef::Named(name)),..} if !name.starts_with(&format!("b{}:",base.0)))
    );
    Ok(())
}
#[test]
fn automatic_exit_rejects_stepped_ambiguity_and_handles_round_openings() -> Result<()> {
    let cylinder = Solid::cylinder([0., 0., 0.], 10., 20.)?;
    let entry=(0..cylinder.face_count()?).find(|i| matches!(cylinder.face_measure(*i).map(|m|m.kind),Ok(nator_kernel::FaceKind::Plane {normal,..}) if normal[2]>0.9)).ok_or("Tampa")?;
    let exit = cylinder.opposite_face(entry)?;
    let duct = cylinder.shell(&[entry, exit], 2., false, "duct")?;
    assert!((duct.volume()? - std::f64::consts::PI * (100. - 64.) * 20.).abs() < 1e-5);
    let a = Solid::cuboid([0.; 3], [20., 20., 10.])?;
    let b = Solid::cuboid([0., 0., 10.], [10., 20., 10.])?;
    let step = a.boolean(&b, 0, "step")?;
    let bottom=(0..step.face_count()?).find(|i| matches!(step.face_measure(*i).map(|m|m.kind),Ok(nator_kernel::FaceKind::Plane {normal,offset}) if normal[2]< -0.9 && offset.abs()<1e-6)).ok_or("Base")?;
    assert!(step.opposite_face(bottom).is_err());
    Ok(())
}
