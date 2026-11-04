//! Candidatas de texto preservam identidade, respeitam restrições e só publicam por comando.
use nator_commands::{Command, Session, sketch_text::candidate};
use nator_document::{Document, parameters::Parameters};
use nator_sketch::{ConstraintKind, Plane, Sketch, text::SketchText};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn block() -> SketchText {
    SketchText {
        id: 0,
        anchor: 0,
        content: "A".into(),
        font: "sans-serif".into(),
        bold: false,
        italic: false,
        underline: false,
        size: 10.,
        rotation: 0.,
        contours: vec![vec![[0., 0.], [1., 0.], [0., 1.], [0., 0.]]],
        points: vec![],
        entities: vec![],
    }
}
#[test]
fn text_candidates_preserve_ids_transform_live_and_commit_as_one_transaction() -> Result<()> {
    let source = Sketch::new("Texto", Plane::xy());
    let first = candidate(&source, block(), [10., 5.], None, &Parameters::default())?;
    assert!(source.texts.is_empty());
    let original = first.texts[0].clone();
    let mut modified = original.clone();
    modified.size = 20.;
    modified.rotation = std::f64::consts::FRAC_PI_2;
    let next = candidate(&first, modified, [15., 8.], None, &Parameters::default())?;
    assert_eq!(next.texts[0].entities, original.entities);
    assert_eq!(next.texts[0].points, original.points);
    assert_eq!(next.point(next.texts[0].anchor)?, [15., 8.]);
    assert_eq!(first.point(original.anchor)?, [10., 5.]);
    let mut document = Document::new("Texto")?;
    let (_, id) = document.add_sketch(None, first)?;
    let mut session = Session::new(document)?;
    session.execute(Command::UpdateSketch { id, sketch: next })?;
    assert!(session.undo());
    assert_eq!(
        session
            .document()
            .sketch(id)
            .ok_or("Rascunho")?
            .sketch
            .point(original.anchor)?,
        [10., 5.]
    );
    assert!(session.redo());
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("text.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    assert_eq!(
        reopened.sketch(id).ok_or("Rascunho")?.sketch.texts[0].entities,
        original.entities
    );
    Ok(())
}
#[test]
fn conflicting_text_positions_are_rejected_without_moving_the_source() -> Result<()> {
    let source = Sketch::new("Texto", Plane::xy());
    let mut fixed = candidate(&source, block(), [10., 5.], None, &Parameters::default())?;
    let block = fixed.texts[0].clone();
    fixed.add_constraint(
        ConstraintKind::Fixed {
            point: block.anchor,
            position: [10., 5.],
        },
        false,
    )?;
    assert!(candidate(&fixed, block, [20., 5.], None, &Parameters::default()).is_err());
    assert_eq!(fixed.point(fixed.texts[0].anchor)?, [10., 5.]);
    Ok(())
}
