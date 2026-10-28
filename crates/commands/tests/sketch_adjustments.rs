//! Cotas conservam identidade, posição do texto e ordem dos elementos ao editar,
//! desfazer/refazer e reabrir o projeto. As novas cópias usam o mesmo comando.
use nator_commands::{Command, Session};
use nator_document::Document;
use nator_sketch::{
    ConstraintKind, LineType, Plane, Sketch,
    dimensions::{self, Target},
    modifications::{self, OffsetMode},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn dimension_order_and_label_survive_edit_history_and_reopening() -> Result<()> {
    let mut sketch = Sketch::new("Medida", Plane::xy());
    let a = sketch.add_point([0., 0.])?;
    let b = sketch.add_point([10., 0.])?;
    let dimension = sketch.add_constraint(ConstraintKind::Distance { a, b, value: 10. }, false)?;
    let pair = [Target::Point(a), Target::Point(b)];
    sketch.dimension_targets.insert(dimension, pair);
    sketch.dimension_labels.insert(dimension, [3., 7.]);
    let mut document = Document::new("Persistir cotas")?;
    let (_, id) = document.add_sketch(None, sketch.clone())?;
    let mut session = Session::new(document)?;
    if let Some(value) = sketch
        .constraints
        .iter_mut()
        .find(|c| c.id == dimension)
        .and_then(|c| c.kind.dimension_value_mut())
    {
        *value = 20.;
    }
    let report = dimensions::solve(&sketch, dimension)?;
    assert_eq!(report.sketch.point(b)?, [10., 0.]);
    session.execute(Command::UpdateSketch {
        id,
        sketch: report.sketch,
    })?;
    assert!(session.undo());
    assert!(session.redo());
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("medida.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let loaded = nator_project_io::read_project(&path)?;
    let sketch = &loaded.sketch(id).ok_or("Rascunho ausente")?.sketch;
    assert_eq!(sketch.dimension_targets.get(&dimension), Some(&pair));
    assert_eq!(sketch.dimension_labels.get(&dimension), Some(&[3., 7.]));
    assert_eq!(sketch.constraints.len(), 1);
    assert!(sketch.point(a)?[0] < -9.999);
    Ok(())
}

#[test]
fn multiple_mirror_and_double_offset_are_atomic_sketch_commands() -> Result<()> {
    let mut sketch = Sketch::new("Cópias", Plane::xy());
    let sources = sketch.add_rectangle([2., 0.], [12., 10.], false)?;
    let axis = sketch.add_line([0., -10.], [0., 20.], true)?;
    let mut document = Document::new("Histórico de cópias")?;
    let (_, id) = document.add_sketch(None, sketch.clone())?;
    let mut session = Session::new(document)?;
    modifications::mirror(&mut sketch, &sources, axis, LineType::Construction)?;
    session.execute(Command::UpdateSketch {
        id,
        sketch: sketch.clone(),
    })?;
    modifications::offset(
        &mut sketch,
        &sources[..1],
        OffsetMode::Double,
        2.,
        4.,
        LineType::Normal,
    )?;
    session.execute(Command::UpdateSketch { id, sketch })?;
    assert!(session.undo());
    assert_eq!(
        session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch
            .entities
            .len(),
        9
    );
    assert!(session.undo());
    assert_eq!(
        session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch
            .entities
            .len(),
        5
    );
    assert!(session.redo());
    assert!(session.redo());
    assert_eq!(
        session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch
            .entities
            .len(),
        11
    );
    Ok(())
}
