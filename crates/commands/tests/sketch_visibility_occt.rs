//! Ciclo de vida do perfil: visível ao concluir, oculto ao consumir e
//! reexibível sem que um recálculo posterior apague a escolha do usuário.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session, measure};
use nator_document::{
    Document, PartId, SketchId,
    features::{EdgeRef, Operation, RegionRef},
    parameters::Dimension,
};
use nator_geometry::{Placement, cross, dot, norm, query::Ray, sub};
use nator_kernel::Solid;
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

/// Regressão: as faces triangulares das quinas ficam sem nome no histórico OCCT;
/// as doze faces dos chanfros compartilham o mesmo papel. Nenhum desses casos pode
/// impedir uma captura fixa de plano, inclusive ao abrir um projeto já gravado.
#[test]
fn sketches_on_chamfer_corners_work_without_stable_names() -> Result<()> {
    let mut session = Session::new(Document::new("Cubo chanfrado")?)?;
    let mut outline = Sketch::new("Cubo", Plane::xy());
    outline.add_rectangle([0.; 2], [20.; 2], false)?;
    let Effect::SketchCreated { part, sketch } = session.execute(Command::AddSketch {
        destination: None,
        sketch: outline,
    })?
    else {
        return Err("Perfil ausente".into());
    };
    let Effect::Imported { body: base, .. } = session.execute(extrusion(part, sketch, "20 mm"))?
    else {
        return Err("Cubo ausente".into());
    };
    let record = session.document().body(base).ok_or("Cubo ausente")?;
    let solid = Solid::from_brep(
        record.brep.as_ref().ok_or("B-rep ausente")?,
        Some(&record.face_names),
    )?;
    let mut edges = (0..solid.edge_count()?)
        .map(|i| solid.edge_face_names(i).map(EdgeRef::Between))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    session.execute(Command::SetParameter {
        part,
        name: "chanfro".into(),
        expression: Some("2 mm".into()),
        dimension: Dimension::Length,
    })?;
    let Effect::Imported { body, .. } = session.execute(Command::AddFeature {
        part,
        name: "Chanfro".into(),
        operation: Operation::Chamfer {
            base,
            edge: edges.remove(0),
            distance: "chanfro".into(),
            angle: None,
            additional_edges: edges,
        },
    })?
    else {
        return Err("Chanfro ausente".into());
    };
    // Composição não trivial também nas quinas, em vez de depender dos eixos do cubo.
    let body_pose = Placement::new([4., -3., 9.], [0.3, 0.2, -0.4])?;
    let occurrence = Placement::new([100., -50., 30.], [-0.5, 0.6, 0.8])?;
    let instance = session.document().instances()[0].id;
    session.execute(Command::MoveBody {
        id: body,
        placement: body_pose,
    })?;
    session.execute(Command::MoveInstance {
        id: instance,
        placement: occurrence,
    })?;
    let world = occurrence.compose(body_pose)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("cubo.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    session = Session::new(nator_project_io::read_project(&path)?)?;
    let record = session.document().body(body).ok_or("Chanfro ausente")?;
    let mesh = record.geometry.clone();
    let names = record.face_names.clone();
    assert_eq!(names.len(), 26);
    assert_eq!(names.iter().filter(|n| n.is_empty()).count(), 8);
    let mut sketches = Vec::new();
    for (face, name) in names.iter().enumerate() {
        let triangle = mesh
            .triangle_faces()
            .ok_or("Faces ausentes")?
            .iter()
            .position(|f| *f as usize == face)
            .ok_or("Face sem triângulo")?;
        let [a, b, c] = mesh
            .mesh()
            .triangle_points(mesh.mesh().triangles()[triangle]);
        let center = world.apply(std::array::from_fn(|i| (a[i] + b[i] + c[i]) / 3.));
        let normal = cross(sub(b, a), sub(c, a));
        let normal = world.apply_vector(normal.map(|v| v / norm(normal)));
        let pick = session
            .document()
            .pick(Ray {
                origin: std::array::from_fn(|i| center[i] + normal[i] * 40.),
                direction: normal.map(|v| -v),
            })?
            .ok_or("Face do chanfro não selecionada")?;
        assert_eq!(pick.body, body);
        assert_eq!(session.document().face_index_at(&pick)?, face);
        let revision = session.revision();
        let plane = measure::sketch_plane(session.document(), &pick)?;
        assert_eq!(session.revision(), revision);
        assert!(dot(plane.normal(), normal) > 1. - 1e-9);
        assert!(norm(sub(plane.origin, center)) < 1e-8);
        if !name.is_empty() {
            continue;
        }
        // A regra de referências estáveis permanece: só o plano fixo é permitido.
        assert!(session.document().face_at(&pick).is_err());
        let local = Plane::from_normal(
            occurrence.inverse_apply(plane.origin),
            occurrence.inverse_vector(plane.normal()),
        )?;
        let Effect::SketchCreated { sketch, .. } = session.execute(Command::AddSketch {
            destination: Some(part),
            sketch: Sketch::new("Quina", local),
        })?
        else {
            return Err("Rascunho na quina ausente".into());
        };
        assert!(session.undo());
        assert!(session.document().sketch(sketch).is_none());
        assert!(session.redo());
        sketches.push((sketch, local));
    }
    assert_eq!(sketches.len(), 8);
    // O recálculo não move os suportes: seguem sendo capturas fixas, sem associação.
    session.execute(Command::SetParameter {
        part,
        name: "chanfro".into(),
        expression: Some("3 mm".into()),
        dimension: Dimension::Length,
    })?;
    nator_project_io::write_project(&path, session.document())?;
    let restored = nator_project_io::read_project(&path)?;
    for (id, plane) in sketches {
        let saved = &restored.sketch(id).ok_or("Rascunho perdido")?.sketch.plane;
        assert!(norm(sub(saved.origin, plane.origin)) < 1e-9);
        assert!(norm(sub(saved.u, plane.u)) < 1e-9);
        assert!(norm(sub(saved.v, plane.v)) < 1e-9);
    }
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
