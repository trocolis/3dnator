//! Perfis coplanares de vários rascunhos e eixo associativo independente.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session};
use nator_document::{
    Document,
    features::{Operation, ProfileRef, RegionRef},
};
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Resultado inteiro da operação: partes soltas viram corpos próprios e o conjunto fica
/// oculto como etapa intermediária.
fn whole(
    session: &Session,
    part: nator_document::PartId,
    body: nator_document::BodyId,
) -> Result<nator_document::BodyId> {
    let feature = session
        .document()
        .part(part)
        .ok_or("Peça ausente")?
        .features
        .iter()
        .find(|f| f.output == body)
        .ok_or("Operação ausente")?;
    Ok(match feature.operation {
        Operation::Piece { base, .. } => base,
        _ => body,
    })
}
#[test]
fn multiple_profiles_and_external_axis_recalculate_clone_and_roundtrip() -> Result<()> {
    let mut a = Sketch::new("Áreas", Plane::xy());
    a.add_rectangle([10., 0.], [14., 5.], false)?;
    a.add_rectangle([10., 8.], [14., 13.], false)?;
    let mut b = Sketch::new(
        "Outra área",
        Plane {
            origin: [0.; 3],
            u: [1., 0., 0.],
            v: [0., -1., 0.],
        },
    );
    b.add_rectangle([18., -5.], [22., 0.], false)?;
    let mut axis = Sketch::new("Eixo", Plane::xy());
    let line = axis.add_line([0., -5.], [0., 20.], true)?;
    let mut doc = Document::new("Revolução coplanar")?;
    let (part, a) = doc.add_sketch(None, a)?;
    let (_, b) = doc.add_sketch(Some(part), b)?;
    let (_, axis_id) = doc.add_sketch(Some(part), axis.clone())?;
    let mut session = Session::new(doc)?;
    let op = Operation::Revolve {
        sketch: a,
        region: RegionRef::Multiple(vec![RegionRef::Index(0), RegionRef::Index(1)]),
        axis_origin: [0.; 3],
        axis_direction: [0., 1., 0.],
        angle: "360 deg".into(),
        axis_line: Some(line),
        axis_sketch: Some(axis_id),
        additional_profiles: vec![ProfileRef {
            sketch: b,
            region: RegionRef::Index(0),
        }],
    };
    let Effect::Imported { body, .. } = session.execute(Command::AddFeature {
        part,
        name: "Anéis".into(),
        operation: op,
    })?
    else {
        return Err("Resultado".into());
    };
    let body = whole(&session, part, body)?;
    let volume = |session: &Session| -> Result<f64> {
        let b = session.document().body(body).ok_or("Corpo")?;
        Ok(Solid::from_brep(b.brep.as_ref().ok_or("B-rep")?, Some(&b.face_names))?.volume()?)
    };
    assert!((volume(&session)? - 1760. * std::f64::consts::PI).abs() < 1e-4);
    for p in &mut axis.points {
        p.position[0] = -2.;
    }
    session.execute(Command::UpdateSketch {
        id: axis_id,
        sketch: axis.clone(),
    })?;
    assert!((volume(&session)? - 2000. * std::f64::consts::PI).abs() < 1e-4);
    let revision = session.revision();
    axis.plane.origin[2] = 2.;
    assert!(
        session
            .execute(Command::UpdateSketch {
                id: axis_id,
                sketch: axis
            })
            .is_err()
    );
    assert_eq!(session.revision(), revision);
    assert!(session.execute(Command::RemoveSketch { id: b }).is_err());
    assert!(
        session
            .execute(Command::RemoveSketch { id: axis_id })
            .is_err()
    );
    let mut copy = session.document().clone();
    let clone = copy.clone_part(part, "Cópia".into())?;
    let cp = copy.part(clone).ok_or("Cópia")?;
    assert!(
        matches!(&cp.features[0].operation,Operation::Revolve {axis_sketch:Some(id),additional_profiles,..} if *id!=axis_id && additional_profiles[0].sketch!=b)
    );
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("revolucao.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let reopened = Session::new(nator_project_io::read_project(&path)?)?;
    assert!((volume(&reopened)? - 2000. * std::f64::consts::PI).abs() < 1e-4);
    Ok(())
}
