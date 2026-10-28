//! Planos de trabalho no histórico paramétrico: editar a distância move o rascunho apoiado
//! e reconstrói a extrusão feita a partir dele, em uma ação de desfazer.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session};
use nator_document::{
    BodyId, Document, WorkPlane,
    features::{Operation, RegionRef},
};
use nator_sketch::Plane;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Limites em Z do corpo, em mm.
fn heights(session: &Session, body: BodyId) -> Result<[f64; 2]> {
    let bounds = session
        .document()
        .body(body)
        .ok_or("Corpo ausente")?
        .geometry
        .mesh()
        .bounds();
    Ok([bounds.min[2], bounds.max[2]])
}

#[test]
fn editing_the_offset_moves_the_sketch_and_rebuilds_its_extrusion() -> Result<()> {
    let mut session = Session::new(Document::new("Plano no histórico")?)?;
    let Effect::PartCreated { part } = session.execute(Command::CreatePart {
        name: "Suporte".into(),
    })?
    else {
        return Err("Peça ausente".into());
    };
    let Effect::WorkPlaneCreated { plane, .. } = session.execute(Command::AddWorkPlane {
        part,
        name: "Plano 1".into(),
        plane: WorkPlane {
            base: Plane::xy(),
            offset: 20.,
        },
    })?
    else {
        return Err("Plano ausente".into());
    };
    let Effect::SketchCreated { sketch, .. } = session.execute(Command::AddSketchOnWorkPlane {
        id: plane,
        name: "Perfil".into(),
    })?
    else {
        return Err("Rascunho ausente".into());
    };
    let mut profile = session
        .document()
        .sketch(sketch)
        .ok_or("Rascunho ausente")?
        .sketch
        .clone();
    profile.add_rectangle([0., 0.], [20., 10.], false)?;
    session.execute(Command::UpdateSketch {
        id: sketch,
        sketch: profile,
    })?;
    let Effect::Imported { body, .. } = session.execute(Command::AddFeature {
        part,
        name: "Extrusão".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "5 mm".into(),
            combine: None,
            until: None,
        },
    })?
    else {
        return Err("Extrusão ausente".into());
    };
    let close = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6;
    assert!(close(heights(&session, body)?, [20., 25.]));
    session.execute(Command::UpdateWorkPlane {
        id: plane,
        plane: WorkPlane {
            base: Plane::xy(),
            offset: 32.5,
        },
    })?;
    assert!(close(heights(&session, body)?, [32.5, 37.5]));
    let support = session
        .document()
        .sketch(sketch)
        .ok_or("Rascunho ausente")?
        .sketch
        .plane;
    assert_eq!(support.origin, [0., 0., 32.5]);
    // O plano continua oculto: editar não reverte o efeito de tê-lo usado.
    assert!(
        session
            .document()
            .work_plane(plane)
            .is_some_and(|w| !w.visible)
    );
    assert!(session.undo());
    assert!(close(heights(&session, body)?, [20., 25.]));
    assert!(session.redo());
    assert!(close(heights(&session, body)?, [32.5, 37.5]));
    Ok(())
}
