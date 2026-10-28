//! Persistência das ligações e publicação pelo mesmo comando de edição do rascunho.
use nator_commands::{Command, Session};
use nator_document::Document;
use nator_sketch::{ConstraintKind, Curve, Plane, Sketch, junctions, snapping::Kind, solver};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn weak_and_strong_links_round_trip_with_circle_incidence_and_history() -> Result<()> {
    let mut sketch = Sketch::new("Ligações", Plane::xy());
    let circle = sketch.add_circle([30., 0.], 10., false)?;
    let first = sketch.add_line([0., 0.], [10., 0.], false)?;
    let second = sketch.add_line([10., 0.], [20., 0.], false)?;
    let Curve::Line { end: target, .. } = sketch.entity(first)?.curve else {
        return Err("Linha ausente".into());
    };
    let Curve::Line { start: a, end: b } = sketch.entity(second)?.curve else {
        return Err("Linha ausente".into());
    };
    junctions::attach(
        &mut sketch,
        a,
        Some(Kind::Endpoint(target)),
        [10., 0.],
        false,
    )?;
    junctions::attach(&mut sketch, b, Some(Kind::OnCurve(circle)), [20., 0.], true)?;
    let mut document = Document::new("Junções persistentes")?;
    let (_, id) = document.add_sketch(None, sketch.clone())?;
    let mut session = Session::new(document)?;
    assert!(junctions::set_strong(&mut sketch, [10., 0.], true)?);
    session.execute(Command::UpdateSketch { id, sketch })?;
    assert!(session.undo());
    assert!(
        junctions::at(
            &session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch,
            [10., 0.]
        )?
        .is_some_and(|j| !j.strong)
    );
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("juncoes.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let loaded = nator_project_io::read_project(&path)?;
    let sketch = &loaded.sketch(id).ok_or("Rascunho ausente")?.sketch;
    assert!(
        sketch
            .constraints
            .iter()
            .any(|c| c.reference && matches!(c.kind, ConstraintKind::Coincident { .. }))
    );
    assert!(
        sketch
            .constraints
            .iter()
            .any(|c| !c.reference && matches!(c.kind, ConstraintKind::PointOnCurve { .. }))
    );
    assert_ne!(solver::solve(sketch)?.status, solver::Status::Conflict);
    assert!(session.redo());
    assert!(
        junctions::at(
            &session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch,
            [10., 0.]
        )?
        .is_some_and(|j| j.strong)
    );
    Ok(())
}
