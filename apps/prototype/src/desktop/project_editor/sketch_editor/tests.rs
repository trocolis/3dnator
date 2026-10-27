//! Regressões dos gestos e da publicação de geometria, sem depender de GPU.
use super::*;
use nator_document::parameters::{Parameters, resolve_sketch};

fn workspace(tool: Tool, anchors: &[Point2]) -> Workspace {
    let mut work = Workspace::new(SketchId(1));
    work.select_tool(tool);
    work.anchors = anchors.to_vec();
    work.anchor_kinds = vec![None; anchors.len()];
    work
}

#[test]
fn symmetric_lines_remain_opposite_and_equal_after_dimension_changes() -> Result<(), String> {
    let mut sketch = Sketch::new("Simetria", nator_sketch::Plane::xy());
    let mut work = workspace(Tool::Line, &[[3., 4.], [15., 4.]]);
    work.line_mode = LineMode::Symmetric;
    work.drawing.locked[0] = true;
    work.drawing.fields[0] = "12 mm".into();
    draw_entity(&mut sketch, &work, &Parameters::default())?;
    assert_eq!(sketch.entities.len(), 2);
    assert_eq!(
        sketch.line(sketch.entities[1].id).map_err(sketch_error)?,
        ([-9., 4.], [3., 4.])
    );
    let expression = sketch
        .dimension_expressions
        .values_mut()
        .next()
        .ok_or("Cota ausente")?;
    *expression = "20 mm".into();
    resolve_sketch(&mut sketch, &Parameters::default()).map_err(sketch_error)?;
    let solved = solver::solve(&sketch).map_err(sketch_error)?;
    assert_ne!(solved.status, Status::Conflict);
    let (center, end) = solved
        .sketch
        .line(sketch.entities[0].id)
        .map_err(sketch_error)?;
    let (mirror, shared) = solved
        .sketch
        .line(sketch.entities[1].id)
        .map_err(sketch_error)?;
    assert!(nator_sketch::distance(center, shared) < 1e-6);
    assert!(
        nator_sketch::distance(
            nator_sketch::scale(nator_sketch::add(end, mirror), 0.5),
            center
        ) < 1e-6
    );
    assert!((nator_sketch::distance(center, end) - 20.).abs() < 1e-6);
    Ok(())
}

#[test]
fn continuous_lines_keep_the_endpoint_and_stop_only_on_area_or_escape() -> Result<(), String> {
    let context = egui::Context::default();
    let parameters = Parameters::default();
    let mut sketch = Sketch::new("Cadeia", nator_sketch::Plane::xy());
    let mut work = workspace(Tool::Line, &[[0., 0.], [20., 0.]]);
    work.chain_points = vec![[0., 0.]];
    draw_entity(&mut sketch, &work, &parameters)?;
    work.continue_line(&sketch, &context, [20., 0.]);
    assert_eq!(work.anchors, vec![[20., 0.]]);
    assert!(matches!(work.anchor_kinds[0], Some(Kind::Endpoint(_))));
    for end in [[20., 10.], [0., 0.]] {
        work.anchors.push(end);
        work.anchor_kinds.push(None);
        draw_entity(&mut sketch, &work, &parameters)?;
        work.continue_line(&sketch, &context, end);
    }
    assert!(!work.is_drawing());
    assert_eq!(
        nator_sketch::areas::regions(&sketch)
            .map_err(sketch_error)?
            .len(),
        1
    );
    work.anchors = vec![[40., 40.]];
    work.chain_points = work.anchors.clone();
    work.reset_drawing(&context);
    assert!(!work.is_drawing());
    assert_eq!(sketch.entities.len(), 3);
    Ok(())
}

#[test]
fn rectangle_modes_use_the_preview_corners_and_respect_stroke() -> Result<(), String> {
    for (mode, anchors, expected_area) in [
        (RectangleMode::Normal, vec![[0., 0.], [20., 10.]], 200.),
        (RectangleMode::Center, vec![[5., 5.], [15., 10.]], 200.),
        (
            RectangleMode::Inclined,
            vec![[0., 0.], [6., 8.], [2., 11.]],
            50.,
        ),
    ] {
        for kind in [LineType::Normal, LineType::Construction, LineType::Infinite] {
            let mut sketch = Sketch::new("Retângulos", nator_sketch::Plane::xy());
            let mut work = workspace(Tool::Rectangle, &anchors);
            work.rectangle_mode = mode;
            work.line_type = kind;
            draw_entity(&mut sketch, &work, &Parameters::default())?;
            assert_eq!(sketch.entities.len(), 4);
            assert!(
                sketch
                    .entities
                    .iter()
                    .all(|entity| entity.line_type() == kind)
            );
            sketch.validate().map_err(sketch_error)?;
            let report = solver::solve(&sketch).map_err(sketch_error)?;
            assert_ne!(report.status, Status::Conflict);
            let regions = nator_sketch::areas::regions(&report.sketch).map_err(sketch_error)?;
            if kind == LineType::Normal {
                assert_eq!(regions.len(), 1);
                assert!((regions[0].area() - expected_area).abs() < 1e-6);
            } else {
                assert!(regions.is_empty());
            }
            if mode == RectangleMode::Center {
                let first = sketch.line(sketch.entities[0].id).map_err(sketch_error)?.0;
                let opposite = sketch.line(sketch.entities[2].id).map_err(sketch_error)?.0;
                assert_eq!(
                    nator_sketch::scale(nator_sketch::add(first, opposite), 0.5),
                    anchors[0]
                );
            }
        }
    }
    Ok(())
}

#[test]
fn inclined_rectangle_preserves_typed_side_angle_and_width() -> Result<(), String> {
    let mut sketch = Sketch::new("Retângulo cotado", nator_sketch::Plane::xy());
    let mut work = workspace(Tool::Rectangle, &[[0., 0.], [6., 8.], [2., 11.]]);
    work.rectangle_mode = RectangleMode::Inclined;
    work.drawing.side_fields = Some((
        [
            "10 mm".into(),
            format!("{} deg", 8_f64.atan2(6.).to_degrees()),
        ],
        [true, true],
    ));
    work.drawing.fields[0] = "5 mm".into();
    work.drawing.locked[0] = true;
    draw_entity(&mut sketch, &work, &Parameters::default())?;
    let width = sketch
        .dimension_expressions
        .values_mut()
        .find(|expression| **expression == "5 mm")
        .ok_or("Largura ausente")?;
    *width = "8 mm".into();
    resolve_sketch(&mut sketch, &Parameters::default()).map_err(sketch_error)?;
    let report = solver::solve(&sketch).map_err(sketch_error)?;
    assert_ne!(report.status, Status::Conflict);
    let (a, b) = report
        .sketch
        .line(sketch.entities[0].id)
        .map_err(sketch_error)?;
    let (c, d) = report
        .sketch
        .line(sketch.entities[1].id)
        .map_err(sketch_error)?;
    assert!((nator_sketch::distance(a, b) - 10.).abs() < 1e-6);
    assert!((nator_sketch::distance(c, d) - 8.).abs() < 1e-6);
    assert!(
        (nator_sketch::lines::angle(&report.sketch, sketch.entities[0].id)
            .map_err(sketch_error)?
            - 8_f64.atan2(6.))
        .abs()
            < 1e-6
    );
    Ok(())
}

#[test]
fn diameter_circle_drives_half_the_typed_diameter_and_arc_uses_construction_stroke()
-> Result<(), String> {
    let mut sketch = Sketch::new("Curvas auxiliares", nator_sketch::Plane::xy());
    let mut work = workspace(Tool::Circle, &[[0., 0.], [20., 0.]]);
    work.circle_mode = CircleMode::Diameter;
    work.line_type = LineType::Construction;
    work.drawing.fields[0] = "20 mm".into();
    work.drawing.locked[0] = true;
    draw_entity(&mut sketch, &work, &Parameters::default())?;
    assert_eq!(
        sketch.circle(sketch.entities[0].id).map_err(sketch_error)?,
        ([10., 0.], 10.)
    );
    resolve_sketch(&mut sketch, &Parameters::default()).map_err(sketch_error)?;
    let report = solver::solve(&sketch).map_err(sketch_error)?;
    assert_ne!(report.status, Status::Conflict);
    assert_eq!(
        report
            .sketch
            .circle(sketch.entities[0].id)
            .map_err(sketch_error)?
            .1,
        10.
    );
    let mut work = workspace(Tool::Arc, &[[30., 0.], [35., 0.], [30., 5.]]);
    work.line_type = LineType::Construction;
    draw_entity(&mut sketch, &work, &Parameters::default())?;
    assert!(
        sketch
            .entities
            .iter()
            .all(|entity| entity.construction && !entity.infinite)
    );
    work.line_type = LineType::Infinite;
    work.select_tool(Tool::Circle);
    assert_eq!(work.line_type, LineType::Normal);
    Ok(())
}

#[test]
fn tangent_circle_survives_radius_changes_save_and_undo_redo()
-> Result<(), Box<dyn std::error::Error>> {
    let parameters = Parameters::default();
    let mut sketch = Sketch::new("Tangência", nator_sketch::Plane::xy());
    let first = sketch.add_line([-30., 0.], [30., 0.], false)?;
    let second = sketch.add_line([0., -30.], [0., 30.], false)?;
    for point in sketch.points.clone() {
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: point.id,
                position: point.position,
            },
            false,
        )?;
    }
    let mut document = Document::new("Círculo tangente")?;
    let (_, id) = document.add_sketch(None, sketch.clone())?;
    let mut session = Session::new(document)?;
    let mut work = workspace(Tool::Circle, &[[0., 0.], [1., 0.], [0., 1.], [-8., 8.]]);
    work.circle_mode = CircleMode::Tangent;
    work.tangent_lines = vec![first, second];
    work.drawing.locked[0] = true;
    work.drawing.fields[0] = "8 mm".into();
    draw_entity(&mut sketch, &work, &parameters)?;
    let circle = sketch.entities.last().ok_or("Círculo ausente")?.id;
    for expression in sketch.dimension_expressions.values_mut() {
        *expression = "12 mm".into();
    }
    resolve_sketch(&mut sketch, &parameters)?;
    let report = solver::solve(&sketch)?;
    assert_ne!(report.status, Status::Conflict);
    let (center, radius) = report.sketch.circle(circle)?;
    assert!((radius - 12.).abs() < 1e-6);
    assert!(nator_sketch::distance(center, [-12., 12.]) < 1e-6);
    session.execute(Command::UpdateSketch {
        id,
        sketch: report.sketch,
    })?;
    assert!(session.undo());
    assert_eq!(
        session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch
            .entities
            .len(),
        2
    );
    assert!(session.redo());
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("tangente.3dnator");
    write_project(&path, session.document())?;
    let reopened = read_project(&path)?;
    let loaded = &reopened.sketch(id).ok_or("Rascunho ausente")?.sketch;
    assert_eq!(
        loaded
            .constraints
            .iter()
            .filter(|constraint| matches!(constraint.kind, ConstraintKind::Tangent { .. }))
            .count(),
        2
    );
    assert!(solver::solve(loaded)?.max_error < 1e-6);
    Ok(())
}
