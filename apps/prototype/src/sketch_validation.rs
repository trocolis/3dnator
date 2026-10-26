//! Ciclo real de rascunho: cotar, recalcular, desfazer, salvar e reabrir.
use nator_commands::{Command, Effect, Session};
use nator_document::{Document, parameters::Dimension};
use nator_sketch::{
    ConstraintKind, Curve, Plane, Sketch,
    solver::{self, Status},
};
use std::{error::Error, path::Path};

pub fn run(directory: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = directory.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(directory)?;
    let mut sketch = Sketch::new("Base com furo", Plane::xy());
    sketch.add_rectangle([0., 0.], [40., 20.], false)?;
    let ids = sketch.points.iter().map(|p| p.id).collect::<Vec<_>>();
    sketch.add_constraint(
        ConstraintKind::Fixed {
            point: ids[0],
            position: [0., 0.],
        },
        false,
    )?;
    let width = sketch.add_constraint(
        ConstraintKind::DeltaX {
            a: ids[0],
            b: ids[1],
            value: 40.,
        },
        false,
    )?;
    sketch.add_constraint(
        ConstraintKind::DeltaY {
            a: ids[0],
            b: ids[3],
            value: 20.,
        },
        false,
    )?;
    let circle = sketch.add_circle([10., 10.], 3., false)?;
    let Curve::Circle { center, .. } = sketch.entity(circle)?.curve else {
        return Err("Círculo ausente".into());
    };
    sketch.add_constraint(
        ConstraintKind::Fixed {
            point: center,
            position: [10., 10.],
        },
        false,
    )?;
    sketch.add_constraint(ConstraintKind::Radius { circle, value: 3. }, false)?;
    let first = solver::solve(&sketch)?;
    if first.status != Status::FullyConstrained {
        return Err("O perfil deveria estar totalmente restrito.".into());
    }
    let mut session = Session::new(Document::new("Rascunhos de validação")?)?;
    let Effect::SketchCreated { sketch: id, .. } = session.execute(Command::AddSketch {
        destination: None,
        sketch: first.sketch,
    })?
    else {
        return Err("Rascunho não criado".into());
    };
    if let Some(constraint) = sketch.constraints.iter_mut().find(|c| c.id == width) {
        constraint.kind = ConstraintKind::DeltaX {
            a: ids[0],
            b: ids[1],
            value: 60.,
        };
    }
    let changed = solver::solve(&sketch)?;
    if changed.status != Status::FullyConstrained
        || (changed.sketch.point(ids[2])?[0] - 60.).abs() > 1e-6
    {
        return Err("A alteração de largura não recalculou o perfil.".into());
    }
    session.execute(Command::UpdateSketch {
        id,
        sketch: changed.sketch,
    })?;
    if !session.undo()
        || (session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch
            .point(ids[2])?[0]
            - 40.)
            .abs()
            > 1e-6
    {
        return Err("Desfazer não restaurou o perfil.".into());
    }
    if !session.redo() {
        return Err("Refazer indisponível.".into());
    }
    let part = session.document().parts()[0].id;
    session.execute(Command::SetParameter {
        part,
        name: "largura".into(),
        expression: Some("60 mm".into()),
        dimension: Dimension::Length,
    })?;
    let mut bound = session
        .document()
        .sketch(id)
        .ok_or("Rascunho ausente")?
        .sketch
        .clone();
    bound.dimension_expressions.insert(width, "largura".into());
    session.execute(Command::UpdateSketch { id, sketch: bound })?;
    session.execute(Command::SetParameter {
        part,
        name: "largura".into(),
        expression: Some("80 mm".into()),
        dimension: Dimension::Length,
    })?;
    if (session
        .document()
        .sketch(id)
        .ok_or("Rascunho ausente")?
        .sketch
        .point(ids[2])?[0]
        - 80.)
        .abs()
        > 1e-6
    {
        return Err("Parâmetro não recalculou a geometria".into());
    }
    if !session.undo()
        || (session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch
            .point(ids[2])?[0]
            - 60.)
            .abs()
            > 1e-6
    {
        return Err("Desfazer parâmetro falhou".into());
    }
    if !session.redo() {
        return Err("Refazer parâmetro falhou".into());
    }
    let file = directory.join("sketch.3dnator");
    nator_project_io::write_project(&file, session.document())?;
    let reopened = nator_project_io::read_project(&file)?;
    let sketch = &reopened
        .sketch(id)
        .ok_or("Rascunho perdido ao salvar")?
        .sketch;
    let (wires, diagnostics) = nator_sketch::profiles::analyze(sketch)?;
    if wires.len() != 2 || !diagnostics.open_points.is_empty() {
        return Err("Contornos do rascunho não foram preservados.".into());
    }
    let report = serde_json::json!({"status":"passed","app_version":env!("CARGO_PKG_VERSION"),"format_version":nator_project_io::FORMAT_VERSION,"fully_constrained":solver::solve(sketch)?.status==Status::FullyConstrained,"closed_wires":wires.len(),"width_mm":sketch.point(ids[2])?[0],"checks":["dimensional solve","fully constrained status","dimension edit and recalculation","undo/redo","save/reopen with constraints","closed outer contour and circular hole","parameter expression drives coordinates","parameter undo/redo and persistence"]});
    std::fs::write(
        directory.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
