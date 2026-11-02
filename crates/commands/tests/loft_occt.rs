//! Transições entre perfis analíticos, com furos preservados e recálculo.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session};
use nator_document::{
    Document,
    features::{Operation, ProfileRef, RegionRef},
};
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[test]
fn loft_between_rectangle_and_circle_is_solid_and_recalculates_after_reopening() -> Result<()> {
    let mut a = Sketch::new("Base", Plane::xy());
    a.add_rectangle([-5., -5.], [5., 5.], false)?;
    let mut b = Sketch::new(
        "Topo",
        Plane {
            origin: [0., 0., 20.],
            ..Plane::xy()
        },
    );
    b.add_circle([0., 0.], 3., false)?;
    let mut doc = Document::new("Retângulo para círculo")?;
    let (part, start) = doc.add_sketch(None, a)?;
    let (_, end) = doc.add_sketch(Some(part), b.clone())?;
    let mut session = Session::new(doc)?;
    let op = Operation::Loft {
        start: ProfileRef {
            sketch: start,
            region: RegionRef::Index(0),
        },
        end: ProfileRef {
            sketch: end,
            region: RegionRef::Index(0),
        },
        ruled: false,
        developable: false,
        holes: None,
    };
    let Effect::Imported { body, .. } = session.execute(Command::AddFeature {
        part,
        name: "Transição".into(),
        operation: op,
    })?
    else {
        return Err("Transição".into());
    };
    let volume = |session: &Session| -> Result<f64> {
        let b = session.document().body(body).ok_or("Corpo")?;
        Ok(Solid::from_brep(b.brep.as_ref().ok_or("B-rep")?, Some(&b.face_names))?.volume()?)
    };
    let before = volume(&session)?;
    assert!(before > 500. && before < 2000.);
    let revision = session.revision();
    b.plane.origin[2] = 0.;
    assert!(
        session
            .execute(Command::UpdateSketch {
                id: end,
                sketch: b.clone()
            })
            .is_err()
    );
    assert_eq!(session.revision(), revision);
    b.plane.origin[2] = 40.;
    session.execute(Command::UpdateSketch { id: end, sketch: b })?;
    assert!((volume(&session)? - before * 2.).abs() < 1e-3);
    assert!(session.execute(Command::RemoveSketch { id: end }).is_err());
    let mut clone = session.document().clone();
    let cloned = clone.clone_part(part, "Cópia".into())?;
    assert!(
        matches!(&clone.part(cloned).ok_or("Cópia")?.features[0].operation,Operation::Loft {start:a,end:b,..} if a.sketch!=start && b.sketch!=end)
    );
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("loft.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let reopened = Session::new(nator_project_io::read_project(&path)?)?;
    assert!((volume(&reopened)? - before * 2.).abs() < 1e-3);
    let result = reopened.document().body(body).ok_or("Corpo")?;
    let solid = Solid::from_brep(
        result.brep.as_ref().ok_or("B-rep")?,
        Some(&result.face_names),
    )?;
    let step = directory.path().join("loft.step");
    solid.write_step(&step)?;
    assert!((Solid::read_step(&step)?.volume()? - before * 2.).abs() < 1e-3);
    Ok(())
}
#[test]
fn circular_loft_keeps_through_hole_and_rejects_missing_hole() -> Result<()> {
    let mut a = Sketch::new("Anel inferior", Plane::xy());
    a.add_circle([0., 0.], 4., false)?;
    a.add_circle([0., 0.], 1., false)?;
    let mut b = Sketch::new(
        "Anel superior",
        Plane {
            origin: [0., 0., 10.],
            ..Plane::xy()
        },
    );
    b.add_circle([0., 0.], 6., false)?;
    b.add_circle([0., 0.], 2., false)?;
    let region = |s: &Sketch| -> Result<_> {
        nator_sketch::profiles::regions(s)?
            .into_iter()
            .find(|r| r.holes.len() == 1)
            .ok_or_else(|| "Anel".into())
    };
    let solid = Solid::loft((&a, &region(&a)?), (&b, &region(&b)?), false, false, "loft")?;
    assert!((solid.volume()? - 230. * std::f64::consts::PI).abs() < 1e-4);
    let mut missing = region(&b)?;
    missing.holes.clear();
    assert!(Solid::loft((&a, &region(&a)?), (&b, &missing), false, false, "loft").is_err());
    Ok(())
}

#[test]
fn replacing_hole_requires_explicit_correspondence_update() -> Result<()> {
    let mut a = Sketch::new("Base", Plane::xy());
    a.add_circle([0., 0.], 8., false)?;
    a.add_circle([0., 0.], 2., false)?;
    let mut b = Sketch::new(
        "Topo",
        Plane {
            origin: [0., 0., 10.],
            ..Plane::xy()
        },
    );
    b.add_circle([0., 0.], 8., false)?;
    let old = b.add_circle([0., 0.], 2., false)?;
    let mut doc = Document::new("Correspondência")?;
    let (part, start) = doc.add_sketch(None, a)?;
    let (_, end) = doc.add_sketch(Some(part), b.clone())?;
    let region = |s: &Sketch| -> Result<RegionRef> {
        Ok(RegionRef::Boundary(
            nator_sketch::profiles::regions(s)?
                .into_iter()
                .find(|r| r.holes.len() == 1)
                .ok_or("Anel")?
                .boundary(),
        ))
    };
    let first = region(&doc.sketch(start).ok_or("Base")?.sketch)?;
    let last = region(&b)?;
    let mut session = Session::new(doc)?;
    let operation = Operation::Loft {
        start: ProfileRef {
            sketch: start,
            region: first,
        },
        end: ProfileRef {
            sketch: end,
            region: last,
        },
        ruled: false,
        developable: false,
        holes: None,
    };
    let Effect::Imported { body, .. } = session.execute(Command::AddFeature {
        part,
        name: "Duto".into(),
        operation: operation.clone(),
    })?
    else {
        return Err("Corpo".into());
    };
    b.entities.retain(|e| e.id != old);
    b.add_circle([0., 0.], 2., false)?;
    let update = Command::UpdateSketch { id: end, sketch: b };
    let revision = session.revision();
    assert!(session.execute(update.clone()).is_err());
    assert_eq!(session.revision(), revision);
    session.execute(Command::Batch {
        label: "Atualizar correspondência",
        commands: vec![
            update,
            Command::EditFeature {
                part,
                output: body,
                operation,
            },
        ],
    })?;
    Ok(())
}

#[test]
fn developable_loft_accepts_a_through_shell_and_survives_recalculation_and_reopening() -> Result<()>
{
    let mut a = Sketch::new("Base", Plane::xy());
    a.add_rectangle([-20., -15.], [20., 15.], false)?;
    let mut b = Sketch::new(
        "Topo",
        Plane {
            origin: [0., 0., 30.],
            ..Plane::xy()
        },
    );
    b.add_circle([0., 0.], 10., false)?;
    let mut doc = Document::new("Duto de transição")?;
    let (part, start) = doc.add_sketch(None, a)?;
    let (_, end) = doc.add_sketch(Some(part), b.clone())?;
    let mut session = Session::new(doc)?;
    let Effect::Imported { body, .. } = session.execute(Command::AddFeature {
        part,
        name: "Transição".into(),
        operation: Operation::Loft {
            start: ProfileRef {
                sketch: start,
                region: RegionRef::Index(0),
            },
            end: ProfileRef {
                sketch: end,
                region: RegionRef::Index(0),
            },
            ruled: true,
            developable: true,
            holes: None,
        },
    })?
    else {
        return Err("Transição".into());
    };
    let Effect::Imported { body: duct, .. } = session.execute(Command::AddFeature {
        part,
        name: "Duto".into(),
        operation: Operation::Shell {
            base: body,
            face: nator_document::features::FaceRef::Named(format!("b{}:end", body.0)),
            thickness: "2 mm".into(),
            additional_faces: vec![],
            outward: false,
            through: true,
            exit_face: None,
        },
    })?
    else {
        return Err("Duto".into());
    };
    let volume = |session: &Session| -> Result<f64> {
        let b = session.document().body(duct).ok_or("Corpo")?;
        Ok(Solid::from_brep(b.brep.as_ref().ok_or("B-rep")?, Some(&b.face_names))?.volume()?)
    };
    let before = volume(&session)?;
    assert!(before > 5000. && before < 7000., "{before}");
    // Mudar o círculo recalcula a transição e a casca que depende dela.
    b.plane.origin[2] = 40.;
    session.execute(Command::UpdateSketch { id: end, sketch: b })?;
    assert!(volume(&session)? > before);
    let directory =
        std::env::temp_dir().join(format!("nator-loft-developable-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let path = directory.join("duto.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    std::fs::remove_dir_all(&directory)?;
    let piece = reopened.part(part).ok_or("Peça")?;
    assert!(matches!(
        piece.features[0].operation,
        Operation::Loft {
            developable: true,
            ..
        }
    ));
    Ok(())
}
