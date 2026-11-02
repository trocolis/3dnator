//! Persistência das ligações e publicação pelo mesmo comando de edição do rascunho.
use nator_commands::{Command, Session};
use nator_document::Document;
use nator_sketch::{ConstraintKind, Curve, Plane, Sketch, junctions, snapping::Kind, solver};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn links_round_trip_with_circle_incidence_and_history() -> Result<()> {
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
    // Sem ligação, as pontas só ficam no mesmo lugar; ligar é uma edição explícita.
    assert!(!junctions::links(Some(Kind::OnCurve(circle))));
    sketch.add_constraint(
        ConstraintKind::PointOnCurve {
            point: b,
            curve: circle,
        },
        false,
    )?;
    let _ = (a, target);
    let mut document = Document::new("Junções persistentes")?;
    let (_, id) = document.add_sketch(None, sketch.clone())?;
    let mut session = Session::new(document)?;
    assert!(junctions::link(&mut sketch, [10., 0.])?);
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
        .is_some_and(|j| !j.linked)
    );
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("juncoes.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let loaded = nator_project_io::read_project(&path)?;
    let sketch = &loaded.sketch(id).ok_or("Rascunho ausente")?.sketch;
    assert!(
        !sketch
            .constraints
            .iter()
            .any(|c| matches!(c.kind, ConstraintKind::Coincident { .. }))
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
        .is_some_and(|j| j.linked)
    );
    Ok(())
}

#[test]
fn new_restrictions_survive_saving_and_reopening() -> Result<()> {
    let mut sketch = Sketch::new("Restrições", Plane::xy());
    let axis = sketch.add_line([0., -20.], [0., 20.], false)?;
    let first = sketch.add_line([-10., 0.], [-20., 5.], false)?;
    let second = sketch.add_line([10., 0.], [20., 5.], false)?;
    let third = sketch.add_line([10., 30.], [20., 30.], false)?;
    let left = sketch.add_circle([-40., 0.], 5., false)?;
    let right = sketch.add_circle([-30., 0.], 5., false)?;
    let ends = |sketch: &Sketch, line| -> Result<(u64, u64)> {
        let Curve::Line { start, end } = sketch.entity(line)?.curve else {
            return Err("Linha ausente".into());
        };
        Ok((start, end))
    };
    let (a, _) = ends(&sketch, first)?;
    let (b, c) = ends(&sketch, second)?;
    let (d, _) = ends(&sketch, third)?;
    let kinds = [
        ConstraintKind::Symmetric { a, b, axis },
        ConstraintKind::Collinear {
            a: second,
            b: second,
        },
        ConstraintKind::OriginOnCurve { curve: left },
        ConstraintKind::EqualRadius { a: left, b: right },
        ConstraintKind::CircleTangent {
            a: left,
            b: right,
            internal: false,
        },
        ConstraintKind::PointsHorizontal { a: c, b: d },
        ConstraintKind::PointsVertical { a: c, b: d },
    ];
    for kind in &kinds {
        sketch.add_constraint(kind.clone(), true)?;
    }
    let mut document = Document::new("Formato 43")?;
    let (_, id) = document.add_sketch(None, sketch)?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("restricoes.3dnator");
    nator_project_io::write_project(&path, &document)?;
    let loaded = nator_project_io::read_project(&path)?;
    let sketch = &loaded.sketch(id).ok_or("Rascunho ausente")?.sketch;
    for kind in &kinds {
        assert!(sketch.constraints.iter().any(|c| c.kind == *kind));
    }
    Ok(())
}
