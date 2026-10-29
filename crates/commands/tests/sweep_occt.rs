//! Transições curvas por rascunho, eixo e guia automática, com furos.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session};
use nator_document::{
    Document,
    features::{Operation, ProfileRef, RegionRef, SweepRoute},
};
use nator_kernel::Solid;
use nator_sketch::{Curve, Plane, Sketch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[test]
fn curved_sketch_and_axis_routes_keep_holes_and_persist() -> Result<()> {
    let first = Plane {
        origin: [10., 0., 0.],
        u: [1., 0., 0.],
        v: [0., 0., 1.],
    };
    let last = Plane {
        origin: [0., 10., 0.],
        u: [0., 1., 0.],
        v: [0., 0., 1.],
    };
    let mut a = Sketch::new("Entrada", first);
    a.add_circle([0., 0.], 2., false)?;
    a.add_circle([0., 0.], 1., false)?;
    let axis = a.add_line([-10., -5.], [-10., 5.], true)?;
    let mut b = Sketch::new("Saída", last);
    b.add_circle([0., 0.], 2., false)?;
    b.add_circle([0., 0.], 1., false)?;
    let mut path = Sketch::new("Arco de guia", Plane::xy());
    let center = path.add_point([0., 0.])?;
    let start = path.add_point([10., 0.])?;
    let end = path.add_point([0., 10.])?;
    let arc = path.add_curve(
        Curve::Arc {
            center,
            start,
            end,
            clockwise: false,
        },
        false,
    )?;
    let key = |s: &Sketch| -> Result<RegionRef> {
        Ok(RegionRef::Boundary(
            nator_sketch::profiles::regions(s)?
                .into_iter()
                .find(|r| r.holes.len() == 1)
                .ok_or("Anel")?
                .boundary(),
        ))
    };
    let ka = key(&a)?;
    let kb = key(&b)?;
    let mut doc = Document::new("Duto curvo")?;
    let (part, a) = doc.add_sketch(None, a)?;
    let (_, b) = doc.add_sketch(Some(part), b)?;
    let (_, path) = doc.add_sketch(Some(part), path)?;
    let mut session = Session::new(doc)?;
    let make = |route| Operation::Sweep {
        start: ProfileRef {
            sketch: a,
            region: ka.clone(),
        },
        end: ProfileRef {
            sketch: b,
            region: kb.clone(),
        },
        route,
        holes: None,
    };
    let Effect::Imported { body, .. } = session.execute(Command::AddFeature {
        part,
        name: "Duto guiado".into(),
        operation: make(SweepRoute::Sketch {
            sketch: path,
            entities: vec![arc],
        }),
    })?
    else {
        return Err("Corpo".into());
    };
    let volume = |s: &Session| -> Result<f64> {
        let b = s.document().body(body).ok_or("Corpo")?;
        Ok(Solid::from_brep(b.brep.as_ref().ok_or("B-rep")?, Some(&b.face_names))?.volume()?)
    };
    assert!(session.execute(Command::RemoveSketch { id: path }).is_err());
    let mut remapped = session.document().clone();
    let copy = remapped.clone_part(part, "Guia copiada".into())?;
    assert!(
        matches!(&remapped.part(copy).ok_or("Cópia")?.features[0].operation,Operation::Sweep {route:SweepRoute::Sketch {sketch,..},..} if *sketch!=path)
    );
    let expected = 15. * std::f64::consts::PI.powi(2);
    assert!((volume(&session)? - expected).abs() < 1e-3);
    session.execute(Command::EditFeature {
        part,
        output: body,
        operation: make(SweepRoute::Axis {
            sketch: a,
            line: axis,
            reverse: false,
        }),
    })?;
    assert!((volume(&session)? - expected).abs() < 1e-3);
    session.execute(Command::EditFeature {
        part,
        output: body,
        operation: make(SweepRoute::Automatic),
    })?;
    assert!(volume(&session)? > expected * 0.8 && volume(&session)? < expected * 1.4);
    assert!(session.undo());
    assert!((volume(&session)? - expected).abs() < 1e-3);
    assert!(session.redo());
    let mut copy = session.document().clone();
    let cloned = copy.clone_part(part, "Cópia".into())?;
    assert!(
        matches!(&copy.part(cloned).ok_or("Cópia")?.features[0].operation,Operation::Sweep {start,end,..} if start.sketch!=a && end.sketch!=b)
    );
    let directory = tempfile::tempdir()?;
    let file = directory.path().join("sweep.3dnator");
    nator_project_io::write_project(&file, session.document())?;
    let loaded = Session::new(nator_project_io::read_project(&file)?)?;
    assert!((volume(&loaded)? - volume(&session)?).abs() < 1e-5);
    Ok(())
}

#[test]
fn connected_line_arc_line_path_is_expanded_and_invalid_endpoints_are_atomic() -> Result<()> {
    let first = Plane {
        origin: [10., -5., 0.],
        u: [1., 0., 0.],
        v: [0., 0., 1.],
    };
    let last = Plane {
        origin: [-5., 10., 0.],
        u: [0., 1., 0.],
        v: [0., 0., 1.],
    };
    let mut a = Sketch::new("Entrada", first);
    a.add_circle([0., 0.], 2., false)?;
    a.add_circle([0., 0.], 1., false)?;
    let mut b = Sketch::new("Saída", last);
    b.add_circle([0., 0.], 2., false)?;
    b.add_circle([0., 0.], 1., false)?;
    let key = |s: &Sketch| -> Result<RegionRef> {
        Ok(RegionRef::Boundary(
            nator_sketch::profiles::regions(s)?
                .into_iter()
                .find(|r| r.holes.len() == 1)
                .ok_or("Anel")?
                .boundary(),
        ))
    };
    let ka = key(&a)?;
    let kb = key(&b)?;
    let mut guide = Sketch::new("Guia", Plane::xy());
    guide.add_line([10., -5.], [10., 0.], false)?;
    let center = guide.add_point([0., 0.])?;
    let start = guide.add_point([10., 0.])?;
    let end = guide.add_point([0., 10.])?;
    let arc = guide.add_curve(
        Curve::Arc {
            center,
            start,
            end,
            clockwise: false,
        },
        false,
    )?;
    guide.add_line([0., 10.], [-5., 10.], false)?;
    let mut doc = Document::new("Cadeia curva")?;
    let (part, a) = doc.add_sketch(None, a)?;
    let (_, b) = doc.add_sketch(Some(part), b)?;
    let (_, path) = doc.add_sketch(Some(part), guide.clone())?;
    let mut session = Session::new(doc)?;
    let Effect::Imported { body, .. } = session.execute(Command::AddFeature {
        part,
        name: "Duto".into(),
        operation: Operation::Sweep {
            start: ProfileRef {
                sketch: a,
                region: ka,
            },
            end: ProfileRef {
                sketch: b,
                region: kb,
            },
            route: SweepRoute::Sketch {
                sketch: path,
                entities: vec![arc],
            },
            holes: None,
        },
    })?
    else {
        return Err("Corpo".into());
    };
    assert!(
        matches!(&session.document().part(part).ok_or("Peça")?.features[0].operation,Operation::Sweep {route:SweepRoute::Sketch {entities,..},..} if entities.len()==3)
    );
    let body = session.document().body(body).ok_or("Corpo")?;
    let volume =
        Solid::from_brep(body.brep.as_ref().ok_or("B-rep")?, Some(&body.face_names))?.volume()?;
    assert!(
        (volume - (30. * std::f64::consts::PI + 15. * std::f64::consts::PI.powi(2))).abs() < 1e-3
    );
    guide.plane.origin[0] = 1.;
    let revision = session.revision();
    assert!(
        session
            .execute(Command::UpdateSketch {
                id: path,
                sketch: guide
            })
            .is_err()
    );
    assert_eq!(session.revision(), revision);
    Ok(())
}
