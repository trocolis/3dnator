//! Pipe circular preciso: histórico, guias associativas e falhas atômicas.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session, export};
use nator_document::{
    BodyId, Document, PartId, SketchId, features::Operation, parameters::Dimension,
};
use nator_kernel::Solid;
use nator_sketch::{Curve, Plane, Sketch};
use std::f64::consts::PI;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn solid(session: &Session, body: BodyId) -> Result<Solid> {
    let body = session.document().body(body).ok_or("Corpo ausente")?;
    Ok(Solid::from_brep(
        body.brep.as_ref().ok_or("B-rep")?,
        Some(&body.face_names),
    )?)
}
fn add(session: &mut Session, part: PartId, operation: Operation) -> Result<BodyId> {
    match session.execute(Command::AddFeature {
        part,
        name: "Pipe".into(),
        operation,
    })? {
        Effect::Imported { body, .. } => Ok(body),
        _ => Err("Pipe sem corpo".into()),
    }
}
fn operation(sketch: SketchId, entities: Vec<u64>, hollow: bool) -> Operation {
    Operation::Pipe {
        sketch,
        entities,
        diameter: "4 mm".into(),
        thickness: hollow.then(|| "0.5 mm".into()),
    }
}

#[test]
fn straight_pipe_is_solid_or_hollow_and_tracks_parameters_and_path() -> Result<()> {
    let mut path = Sketch::new("Guia", Plane::yz());
    let line = path.add_line([0., 0.], [30., 0.], false)?;
    let mut doc = Document::new("Pipe reto")?;
    let (part, sketch) = doc.add_sketch(None, path)?;
    let mut session = Session::new(doc)?;
    let body = add(&mut session, part, operation(sketch, vec![line], false))?;
    assert!((solid(&session, body)?.volume()? - 120. * PI).abs() < 1e-4);
    session.execute(Command::EditFeature {
        part,
        output: body,
        operation: operation(sketch, vec![line], true),
    })?;
    assert!((solid(&session, body)?.volume()? - 52.5 * PI).abs() < 1e-4);
    assert!(!session.document().sketch(sketch).ok_or("Guia")?.visible);
    session.execute(Command::SetParameter {
        part,
        name: "diametro".into(),
        expression: Some("6 mm".into()),
        dimension: Dimension::Length,
    })?;
    let Operation::Pipe {
        sketch,
        entities,
        thickness,
        ..
    } = operation(sketch, vec![line], true)
    else {
        return Err("Operação".into());
    };
    session.execute(Command::EditFeature {
        part,
        output: body,
        operation: Operation::Pipe {
            sketch,
            entities,
            diameter: "diametro".into(),
            thickness,
        },
    })?;
    assert!((solid(&session, body)?.volume()? - 82.5 * PI).abs() < 1e-4);
    session.execute(Command::SetParameter {
        part,
        name: "diametro".into(),
        expression: Some("8 mm".into()),
        dimension: Dimension::Length,
    })?;
    assert!((solid(&session, body)?.volume()? - 112.5 * PI).abs() < 1e-4);
    let mut changed = session
        .document()
        .sketch(sketch)
        .ok_or("Guia")?
        .sketch
        .clone();
    let Curve::Line { end, .. } = changed.entity(line)?.curve else {
        return Err("Linha".into());
    };
    changed
        .points
        .iter_mut()
        .find(|p| p.id == end)
        .ok_or("Ponta")?
        .position = [40., 0.];
    session.execute(Command::UpdateSketch {
        id: sketch,
        sketch: changed,
    })?;
    assert!((solid(&session, body)?.volume()? - 150. * PI).abs() < 1e-4);
    assert!(session.undo());
    assert!((solid(&session, body)?.volume()? - 112.5 * PI).abs() < 1e-4);
    assert!(session.redo());
    assert!((solid(&session, body)?.volume()? - 150. * PI).abs() < 1e-4);
    assert!(
        session
            .execute(Command::RemoveSketch { id: sketch })
            .is_err()
    );
    Ok(())
}

#[test]
fn tangent_chain_expands_from_a_seed_and_survives_clone_save_and_export() -> Result<()> {
    let mut path = Sketch::new(
        "Curva",
        Plane {
            origin: [8., -6., 3.],
            ..Plane::xy()
        },
    );
    let first = path.add_line([20., -10.], [20., 0.], false)?;
    let center = path.add_point([0., 0.])?;
    let start = path.add_point([20., 0.])?;
    let end = path.add_point([0., 20.])?;
    path.add_curve(
        Curve::Arc {
            center,
            start,
            end,
            clockwise: false,
        },
        false,
    )?;
    path.add_line([0., 20.], [-10., 20.], false)?;
    let mut doc = Document::new("Pipe curvo")?;
    let (part, sketch) = doc.add_sketch(None, path)?;
    let mut session = Session::new(doc)?;
    let body = add(&mut session, part, operation(sketch, vec![first], true))?;
    let expected = 1.75 * PI * (20. + 10. * PI);
    assert!((solid(&session, body)?.volume()? - expected).abs() < 1e-3);
    assert!(
        matches!(&session.document().part(part).ok_or("Peça")?.features[0].operation, Operation::Pipe { entities, .. } if entities.len() == 3)
    );
    let mut copy = session.document().clone();
    let cloned = copy.clone_part(part, "Cópia".into())?;
    let feature = &copy.part(cloned).ok_or("Cópia")?.features[0];
    assert!(matches!(feature.operation, Operation::Pipe {sketch: id, ..} if id != sketch));
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("pipe.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let mut loaded = Session::new(nator_project_io::read_project(&path)?)?;
    let operation = loaded.document().part(part).ok_or("Peça")?.features[0]
        .operation
        .clone();
    loaded.execute(Command::EditFeature {
        part,
        output: body,
        operation,
    })?;
    assert!((solid(&loaded, body)?.volume()? - expected).abs() < 1e-3);
    let step = directory.path().join("pipe.step");
    solid(&loaded, body)?.write_step(&step)?;
    assert!((Solid::read_step(&step)?.volume()? - expected).abs() < 1e-3);
    let files = export::plan(
        loaded.document(),
        export::Scope::Body(body),
        export::Quality::High,
    )?;
    assert_eq!(files.len(), 1);
    assert!(!files[0].mesh.triangles().is_empty());
    Ok(())
}

#[test]
fn invalid_dimensions_paths_and_lost_references_preserve_document_and_redo() -> Result<()> {
    let mut path = Sketch::new("Guia", Plane::xy());
    let line = path.add_line([0., 0.], [30., 0.], false)?;
    let corner_start = path.add_line([40., 0.], [50., 0.], false)?;
    let corner = path.add_line([50., 0.], [50., 10.], false)?;
    let circle = path.add_circle([60., 0.], 10., false)?;
    let infinite = path.add_line([80., 0.], [90., 0.], true)?;
    path.entities
        .iter_mut()
        .find(|e| e.id == infinite)
        .ok_or("Reta")?
        .infinite = true;
    let mut doc = Document::new("Falhas")?;
    let (part, sketch) = doc.add_sketch(None, path)?;
    let mut session = Session::new(doc)?;
    // Duas cópias do mesmo ID não devem ser aceitas como uma trajetória.
    let mut invalid = vec![
        operation(sketch, vec![corner_start, corner], true),
        operation(sketch, vec![line, corner], true),
        operation(sketch, vec![circle, line], true),
        operation(sketch, vec![infinite], true),
        operation(sketch, vec![line, line], true),
        operation(sketch, vec![u64::MAX], true),
        operation(sketch, vec![], true),
    ];
    for (diameter, thickness) in [
        ("0 mm", None),
        ("4 mm", Some("2 mm")),
        ("4 mm", Some("-1 mm")),
        ("4 deg", None),
    ] {
        invalid.push(Operation::Pipe {
            sketch,
            entities: vec![line],
            diameter: diameter.into(),
            thickness: thickness.map(Into::into),
        });
    }
    let body = add(&mut session, part, operation(sketch, vec![line], true))?;
    session.execute(Command::EditFeature {
        part,
        output: body,
        operation: operation(sketch, vec![line], false),
    })?;
    assert!(session.undo());
    let redo = session.redo_label().map(str::to_owned);
    assert!(redo.is_some());
    let revision = session.revision();
    let next = session.document().next_id();
    for operation in invalid {
        assert!(
            session
                .execute(Command::AddFeature {
                    part,
                    name: "Recusado".into(),
                    operation
                })
                .is_err()
        );
        assert_eq!(session.revision(), revision);
        assert_eq!(session.document().next_id(), next);
        assert_eq!(
            session.document().part(part).ok_or("Peça")?.features.len(),
            1
        );
        assert!(!session.document().sketch(sketch).ok_or("Guia")?.visible);
        assert_eq!(session.redo_label().map(str::to_owned), redo);
        assert!((solid(&session, body)?.volume()? - 52.5 * PI).abs() < 1e-4);
    }
    let mut lost = session
        .document()
        .sketch(sketch)
        .ok_or("Guia")?
        .sketch
        .clone();
    lost.entities.retain(|entity| entity.id != line);
    assert!(
        session
            .execute(Command::UpdateSketch {
                id: sketch,
                sketch: lost
            })
            .is_err()
    );
    assert_eq!(session.revision(), revision);
    assert_eq!(session.redo_label().map(str::to_owned), redo);
    assert!((solid(&session, body)?.volume()? - 52.5 * PI).abs() < 1e-4);
    Ok(())
}

#[test]
fn a_bend_too_tight_for_the_section_is_rejected_atomically() -> Result<()> {
    let mut path = Sketch::new("Curva apertada", Plane::xy());
    let center = path.add_point([0., 0.])?;
    let start = path.add_point([1., 0.])?;
    let end = path.add_point([0., 1.])?;
    let arc = path.add_curve(
        Curve::Arc {
            center,
            start,
            end,
            clockwise: false,
        },
        false,
    )?;
    let mut document = Document::new("Raio inválido")?;
    let (part, sketch) = document.add_sketch(None, path)?;
    let mut session = Session::new(document)?;
    let revision = session.revision();
    assert!(add(&mut session, part, operation(sketch, vec![arc], true)).is_err());
    assert_eq!(session.revision(), revision);
    assert!(
        session
            .document()
            .part(part)
            .ok_or("Peça")?
            .features
            .is_empty()
    );
    Ok(())
}

#[test]
fn circular_paths_create_exact_solid_and_hollow_rings_and_recalculate() -> Result<()> {
    let mut path = Sketch::new(
        "Círculo",
        Plane {
            origin: [5., 7., 11.],
            ..Plane::yz()
        },
    );
    let circle = path.add_circle([0., 0.], 10., false)?;
    let mut document = Document::new("Tubo circular")?;
    let (part, sketch) = document.add_sketch(None, path)?;
    let mut session = Session::new(document)?;
    let body = add(&mut session, part, operation(sketch, vec![circle], false))?;
    assert!((solid(&session, body)?.volume()? - 80. * PI * PI).abs() < 1e-3);
    session.execute(Command::EditFeature {
        part,
        output: body,
        operation: operation(sketch, vec![circle], true),
    })?;
    assert!((solid(&session, body)?.volume()? - 35. * PI * PI).abs() < 1e-3);
    let mut changed = session
        .document()
        .sketch(sketch)
        .ok_or("Guia")?
        .sketch
        .clone();
    if let Curve::Circle { radius, .. } = &mut changed
        .entities
        .iter_mut()
        .find(|e| e.id == circle)
        .ok_or("Círculo")?
        .curve
    {
        *radius = 12.;
    }
    session.execute(Command::UpdateSketch {
        id: sketch,
        sketch: changed,
    })?;
    assert!((solid(&session, body)?.volume()? - 42. * PI * PI).abs() < 1e-3);
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("circular.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let mut reopened = Session::new(nator_project_io::read_project(&path)?)?;
    reopened.execute(Command::EditFeature {
        part,
        output: body,
        operation: operation(sketch, vec![circle], true),
    })?;
    assert!((solid(&reopened, body)?.volume()? - 42. * PI * PI).abs() < 1e-3);
    let step = temp.path().join("ring.step");
    solid(&reopened, body)?.write_step(&step)?;
    assert!((Solid::read_step(&step)?.volume()? - 42. * PI * PI).abs() < 1e-3);
    assert!(session.undo());
    assert!((solid(&session, body)?.volume()? - 35. * PI * PI).abs() < 1e-3);
    assert!(session.redo());
    assert!((solid(&session, body)?.volume()? - 42. * PI * PI).abs() < 1e-3);
    Ok(())
}

#[test]
fn a_closed_line_arc_chain_has_no_caps_and_matches_its_analytic_volume() -> Result<()> {
    let mut path = Sketch::new("Oblongo", Plane::xy());
    let bottom = path.add_line([0., -10.], [30., -10.], false)?;
    for (center, a, b) in [
        ([30., 0.], [30., -10.], [30., 10.]),
        ([0., 0.], [0., 10.], [0., -10.]),
    ] {
        let center = path.add_point(center)?;
        let start = path.add_point(a)?;
        let end = path.add_point(b)?;
        path.add_curve(
            Curve::Arc {
                center,
                start,
                end,
                clockwise: false,
            },
            false,
        )?;
    }
    path.add_line([30., 10.], [0., 10.], false)?;
    let mut document = Document::new("Tubo oblongo")?;
    let (part, sketch) = document.add_sketch(None, path)?;
    let mut session = Session::new(document)?;
    let body = add(&mut session, part, operation(sketch, vec![bottom], true))?;
    assert!((solid(&session, body)?.volume()? - 1.75 * PI * (60. + 20. * PI)).abs() < 1e-3);
    let swept = solid(&session, body)?;
    for face in 0..swept.face_count()? {
        assert!(!matches!(
            swept.face_measure(face)?.kind,
            nator_kernel::FaceKind::Plane { .. }
        ));
    }
    assert!(
        matches!(&session.document().part(part).ok_or("Peça")?.features[0].operation, Operation::Pipe {entities,..} if entities.len() == 4)
    );
    assert!(
        session
            .document()
            .body(body)
            .ok_or("Corpo")?
            .face_names
            .iter()
            .all(|name| !name.ends_with(":start") && !name.ends_with(":end"))
    );
    Ok(())
}

#[test]
fn a_corner_at_the_closed_chain_seam_is_rejected_without_changes() -> Result<()> {
    let mut path = Sketch::new("Quina na emenda", Plane::xy());
    let line = path.add_line([0., 0.], [10., 0.], false)?;
    let center = path.add_point([10., 10.])?;
    let start = path.add_point([10., 0.])?;
    let end = path.add_point([0., 10.])?;
    path.add_curve(
        Curve::Arc {
            center,
            start,
            end,
            clockwise: false,
        },
        false,
    )?;
    path.add_line([0., 10.], [0., 0.], false)?;
    let mut document = Document::new("Emenda inválida")?;
    let (part, sketch) = document.add_sketch(None, path)?;
    let mut session = Session::new(document)?;
    let revision = session.revision();
    let error = add(&mut session, part, operation(sketch, vec![line], true))
        .err()
        .ok_or("Quina aceita")?;
    assert!(error.to_string().contains("quina"));
    assert_eq!(session.revision(), revision);
    assert!(
        session
            .document()
            .part(part)
            .ok_or("Peça")?
            .features
            .is_empty()
    );
    Ok(())
}
