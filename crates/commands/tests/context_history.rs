//! Contexto, atividades e persistência: navegar não faz parte da transação.
use nator_commands::{Command, Effect, Session};
use nator_document::{
    Document,
    activity::{ActivityKind as K, ActivityTarget as T, EditContext as C},
};
use nator_sketch::{ConstraintKind, Plane, Sketch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn sketch_gestures_keep_context_targets_and_persist_after_undo_redo() -> Result<()> {
    let mut session = Session::new(Document::new("Contextos")?)?;
    let Effect::SketchCreated { part, sketch: id } = session.execute(Command::AddSketch {
        destination: None,
        sketch: Sketch::new("Rascunho", Plane::xy()),
    })?
    else {
        return Err("Rascunho ausente".into());
    };
    let scope = C::Sketch {
        part: Some(part),
        sketch: id,
    };
    assert_eq!(session.undo_context(), Some(scope));
    let mut sketch = session
        .document()
        .sketch(id)
        .ok_or("Rascunho")?
        .sketch
        .clone();
    let line = sketch.add_line([0., 0.], [10., 0.], false)?;
    session.execute(Command::UpdateSketch {
        id,
        sketch: sketch.clone(),
    })?;
    assert_eq!(
        session
            .document()
            .activities()
            .last()
            .ok_or("Atividade")?
            .kind,
        K::Geometry
    );
    let point = sketch.entities[0].point_ids()[1];
    sketch
        .points
        .iter_mut()
        .find(|p| p.id == point)
        .ok_or("Apoio")?
        .position = [20., 0.];
    session.execute(Command::UpdateSketch {
        id,
        sketch: sketch.clone(),
    })?;
    assert!(
        matches!(&session.document().activities().last().ok_or("Atividade")?.target,T::Sketch { entities,.. } if entities==&vec![line])
    );
    let relation = sketch.add_constraint(ConstraintKind::Horizontal { line }, false)?;
    session.execute(Command::UpdateSketch { id, sketch })?;
    let activity = session
        .document()
        .activities()
        .last()
        .ok_or("Relação")?
        .clone();
    assert_eq!(activity.kind, K::Relation);
    assert!(matches!(&activity.target,T::Sketch {constraints,..} if constraints==&vec![relation]));
    assert!(session.undo());
    assert_eq!(session.redo_context(), Some(scope));
    assert!(session.redo());
    assert_eq!(session.document().activities().last(), Some(&activity));
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("atividades.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let loaded = nator_project_io::read_project(&path)?;
    assert_eq!(loaded.activities(), session.document().activities());
    Ok(())
}

#[test]
fn ungroup_removes_only_group_card_and_remains_undoable() -> Result<()> {
    let mut session = Session::new(Document::new("Montagem")?)?;
    session.execute(Command::CreatePart {
        name: "Primeira".into(),
    })?;
    session.execute(Command::CreatePart {
        name: "Segunda".into(),
    })?;
    let members = session
        .document()
        .instances()
        .iter()
        .map(|i| i.id)
        .collect();
    session.execute(Command::CreateGroup {
        name: "Par".into(),
        members,
    })?;
    let group = session.document().groups()[0].id;
    let before = session.document().activities().to_vec();
    session.execute(Command::RemoveGroup { id: group })?;
    assert_eq!(session.undo_context(), Some(C::Assembly));
    assert!(
        session
            .document()
            .activities()
            .iter()
            .all(|a| a.target != T::Group(group))
    );
    assert_eq!(session.document().activities().len(), before.len() - 1);
    assert!(session.undo());
    assert_eq!(session.document().activities(), before);
    assert_eq!(session.document().parts().len(), 2);
    Ok(())
}

#[test]
fn failed_batch_does_not_leak_activities() -> Result<()> {
    let mut session = Session::new(Document::new("Atômico")?)?;
    let result = session.execute(Command::Batch {
        label: "Falha",
        commands: vec![
            Command::CreatePart {
                name: "Válida".into(),
            },
            Command::CreatePart { name: "".into() },
        ],
    });
    assert!(result.is_err());
    assert!(session.document().activities().is_empty());
    assert!(session.document().parts().is_empty());
    assert_eq!(session.undo_context(), None);
    Ok(())
}
