//! Uma divisão conserva ambos os lados como resultados vinculados e reversíveis.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session};
use nator_document::{
    BodyId, Document, WorkPlane,
    features::{ExtrusionLimit, Operation, RegionRef},
};
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn volume(session: &Session, id: BodyId) -> Result<f64> {
    let b = session.document().body(id).ok_or("Corpo")?;
    Ok(Solid::from_brep(b.brep.as_ref().ok_or("B-rep")?, Some(&b.face_names))?.volume()?)
}
#[test]
fn split_keeps_both_bodies_and_follows_work_plane_through_history_and_reopening() -> Result<()> {
    let mut sketch = Sketch::new("Perfil", Plane::xy());
    sketch.add_rectangle([0., 0.], [20., 20.], false)?;
    let mut doc = Document::new("Dois lados")?;
    let (part, sketch) = doc.add_sketch(None, sketch)?;
    let mut session = Session::new(doc)?;
    let Effect::Imported { body: base, .. } = session.execute(Command::AddFeature {
        part,
        name: "Prisma".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "10 mm".into(),
            combine: None,
            until: None,
        },
    })?
    else {
        return Err("Base".into());
    };
    let Effect::WorkPlaneCreated { plane, .. } = session.execute(Command::AddWorkPlane {
        part,
        name: "Corte".into(),
        plane: WorkPlane {
            base: Plane::xy(),
            offset: 4.,
        },
    })?
    else {
        return Err("Plano".into());
    };
    let original = session.document().clone();
    let Effect::Imported { body: first, .. } = session.execute(Command::SplitBody {
        part,
        name: "Divisão".into(),
        operation: Operation::Split {
            base,
            origin: [0.; 3],
            normal: [0., 0., 1.],
            cutting_plane: Some(ExtrusionLimit::WorkPlane { work_plane: plane }),
            reverse: false,
        },
    })?
    else {
        return Err("Divisão".into());
    };
    let p = session.document().part(part).ok_or("Peça")?;
    let second = p.features.last().ok_or("Complemento")?.output;
    assert!(
        matches!(p.features.last().map(|f|&f.operation),Some(Operation::SplitRemainder {division,..}) if *division==first)
    );
    assert!(!session.document().body(base).ok_or("Base")?.visible);
    assert!(session.document().body(first).ok_or("Lado")?.visible);
    assert!(session.document().body(second).ok_or("Lado")?.visible);
    assert!((volume(&session, first)? - 2400.).abs() < 1e-5);
    assert!((volume(&session, second)? - 1600.).abs() < 1e-5);
    assert!(session.undo());
    assert_eq!(
        session.document().part(part).ok_or("Peça")?.bodies.len(),
        original.part(part).ok_or("Peça")?.bodies.len()
    );
    assert!(session.redo());
    session.execute(Command::UpdateWorkPlane {
        id: plane,
        plane: WorkPlane {
            base: Plane::xy(),
            offset: 6.,
        },
    })?;
    assert!((volume(&session, first)? - 1600.).abs() < 1e-5);
    assert!((volume(&session, second)? - 2400.).abs() < 1e-5);
    assert!(
        session
            .execute(Command::RemoveWorkPlane { id: plane })
            .is_err()
    );
    // Suprimir a etapa primária também oculta o complemento, devolvendo a base.
    session.execute(Command::SetFeatureSuppressed {
        part,
        output: first,
        suppressed: true,
    })?;
    assert!(session.document().body(base).ok_or("Base")?.visible);
    assert!(!session.document().body(second).ok_or("Lado")?.visible);
    session.execute(Command::SetFeatureSuppressed {
        part,
        output: first,
        suppressed: false,
    })?;
    let mut copy = session.document().clone();
    let cloned = copy.clone_part(part, "Clone".into())?;
    let cp = copy.part(cloned).ok_or("Clone")?;
    let clone_plane = cp.work_planes[0].id;
    assert_ne!(clone_plane, plane);
    assert!(
        matches!(&cp.features[1].operation,Operation::Split {cutting_plane:Some(ExtrusionLimit::WorkPlane {work_plane}),..} if *work_plane==clone_plane)
    );
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("divisao.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let mut reopened = Session::new(nator_project_io::read_project(&path)?)?;
    reopened.execute(Command::UpdateWorkPlane {
        id: plane,
        plane: WorkPlane {
            base: Plane::xy(),
            offset: 3.,
        },
    })?;
    assert!((volume(&reopened, first)? + volume(&reopened, second)? - 4000.).abs() < 1e-5);
    let before = reopened.revision();
    assert!(
        reopened
            .execute(Command::UpdateWorkPlane {
                id: plane,
                plane: WorkPlane {
                    base: Plane::xy(),
                    offset: 30.
                }
            })
            .is_err()
    );
    assert_eq!(reopened.revision(), before);
    assert!((volume(&reopened, second)? - 1200.).abs() < 1e-5);
    Ok(())
}
