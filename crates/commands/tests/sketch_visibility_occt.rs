//! Ciclo de vida do perfil: visível ao concluir, oculto ao consumir e
//! reexibível sem que um recálculo posterior apague a escolha do usuário.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session, measure};
use nator_document::{
    Document, PartId, SketchId,
    features::{Operation, RegionRef},
    parameters::Dimension,
};
use nator_geometry::{Placement, dot, norm, query::Ray, sub};
use nator_sketch::{Plane, Sketch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn profile(circle: bool) -> Result<(Session, PartId, SketchId)> {
    let mut session = Session::new(Document::new("Perfil")?)?;
    let mut sketch = Sketch::new("Base", Plane::xy());
    if circle {
        sketch.add_circle([0., 0.], 10., false)?;
    } else {
        sketch.add_rectangle([0., 0.], [20., 15.], false)?;
    }
    let Effect::SketchCreated { part, sketch } = session.execute(Command::AddSketch {
        destination: None,
        sketch,
    })?
    else {
        return Err("Rascunho ausente".into());
    };
    Ok((session, part, sketch))
}
fn extrusion(part: PartId, sketch: SketchId, to: &str) -> Command {
    Command::AddFeature {
        part,
        name: "Extrusão".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: to.into(),
            combine: None,
            until: None,
        },
    }
}

#[test]
fn consumed_sketch_visibility_is_atomic_undoable_and_persistent() -> Result<()> {
    let (mut session, part, id) = profile(false)?;
    assert!(
        session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .visible
    );
    // Uma operação inválida não pode apagar o perfil nem adicionar histórico.
    let revision = session.revision();
    assert!(session.execute(extrusion(part, id, "0 mm")).is_err());
    assert_eq!(session.revision(), revision);
    assert!(
        session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .visible
    );
    session.execute(extrusion(part, id, "8 mm"))?;
    assert!(
        !session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .visible
    );
    assert!(session.undo());
    assert!(
        session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .visible
    );
    assert!(
        session
            .document()
            .part(part)
            .ok_or("Peça ausente")?
            .bodies
            .is_empty()
    );
    assert!(session.redo());
    assert!(
        !session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .visible
    );
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("perfil.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    assert!(
        !nator_project_io::read_project(&path)?
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .visible
    );
    session.execute(Command::SetSketchVisibility { id, visible: true })?;
    // SetParameter reconstrói o histórico inteiro; a preferência deve sobreviver.
    session.execute(Command::SetParameter {
        part,
        name: "altura".into(),
        expression: Some("12 mm".into()),
        dimension: Dimension::Length,
    })?;
    assert!(
        session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .visible
    );
    nator_project_io::write_project(&path, session.document())?;
    assert!(
        nator_project_io::read_project(&path)?
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .visible
    );
    assert!(session.undo());
    assert!(session.undo());
    assert!(
        !session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .visible
    );
    assert!(session.redo());
    assert!(
        session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .visible
    );
    Ok(())
}

#[test]
fn selected_plane_follows_body_and_occurrence_placement() -> Result<()> {
    let (mut session, part, id) = profile(false)?;
    let Effect::Imported { body, .. } = session.execute(extrusion(part, id, "8 mm"))? else {
        return Err("Corpo ausente".into());
    };
    let instance = session.document().instances()[0].id;
    let body_pose = Placement::new([4., -3., 9.], [0.3, 0.2, -0.4])?;
    let occurrence = Placement::new([100., -50., 30.], [-0.5, 0.6, 0.8])?;
    session.execute(Command::MoveBody {
        id: body,
        placement: body_pose,
    })?;
    session.execute(Command::MoveInstance {
        id: instance,
        placement: occurrence,
    })?;
    let world = occurrence.compose(body_pose)?;
    let point = world.apply([9., 6., 8.]);
    let normal = world.apply_vector([0., 0., 1.]);
    let pick = session
        .document()
        .pick(Ray {
            origin: std::array::from_fn(|i| point[i] + normal[i] * 40.),
            direction: normal.map(|v| -v),
        })?
        .ok_or("Face não selecionada")?;
    let plane = measure::sketch_plane(session.document(), &pick)?;
    assert!(dot(plane.normal(), normal) > 1. - 1e-9);
    assert!(dot(sub(point, plane.origin), normal).abs() < 1e-8);
    assert!(norm(sub(plane.origin, pick.point)) < 1e-8);
    Ok(())
}

#[test]
fn curved_face_does_not_define_a_sketch_plane() -> Result<()> {
    let (mut session, part, id) = profile(true)?;
    session.execute(extrusion(part, id, "8 mm"))?;
    let pick = session
        .document()
        .pick(Ray {
            origin: [30., 0., 4.],
            direction: [-1., 0., 0.],
        })?
        .ok_or("Face cilíndrica ausente")?;
    let revision = session.revision();
    let error = measure::sketch_plane(session.document(), &pick)
        .err()
        .ok_or("Face curva foi aceita")?;
    assert!(error.to_string().contains("curva"));
    assert_eq!(session.revision(), revision);
    Ok(())
}

#[test]
fn reference_sketch_is_never_a_feature_profile() -> Result<()> {
    let (mut session, part, _) = profile(false)?;
    let mut layout = Sketch::new("Referência", Plane::xy());
    layout.add_rectangle([0., 0.], [20., 15.], false)?;
    let Effect::ReferenceSketchCreated { sketch } =
        session.execute(Command::AddReferenceSketch { sketch: layout })?
    else {
        return Err("Rascunho de referência ausente".into());
    };
    // A região é válida, mas o rascunho pertence à montagem: a operação é recusada.
    let revision = session.revision();
    assert!(session.execute(extrusion(part, sketch, "8 mm")).is_err());
    assert_eq!(session.revision(), revision);
    assert!(
        session
            .document()
            .part(part)
            .ok_or("Peça ausente")?
            .features
            .is_empty()
    );
    Ok(())
}
