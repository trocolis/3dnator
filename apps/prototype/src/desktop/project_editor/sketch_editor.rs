//! Edição 2D no plano, com prévias locais e publicação por comandos da sessão.
use super::*;
use nator_sketch::{
    ConstraintKind, Curve, Point2, Sketch, profiles,
    snapping::{self, Kind, Tracker},
    solver::{self, Status},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Tool {
    Select,
    Line,
    Rectangle,
    Circle,
    Arc,
    Trim,
}
impl Tool {
    pub(super) fn label(self) -> T {
        match self {
            Self::Select => T::Select,
            Self::Line => T::Line,
            Self::Rectangle => T::Rectangle,
            Self::Circle => T::Circle,
            Self::Arc => T::Arc,
            Self::Trim => T::Trim,
        }
    }
    pub(super) fn icon(self) -> icons::Icon {
        match self {
            Self::Select => icons::Icon::Select,
            Self::Line => icons::Icon::Line,
            Self::Rectangle => icons::Icon::Rectangle,
            Self::Circle => icons::Icon::Circle,
            Self::Arc => icons::Icon::Arc,
            Self::Trim => icons::Icon::Trim,
        }
    }
}
pub(super) struct Workspace {
    pub id: SketchId,
    pub pending: Option<Sketch>,
    pub status: Status,
    pub redundant: bool,
    tool: Tool,
    anchors: Vec<Point2>,
    anchor_kinds: Vec<Option<Kind>>,
    selected: Vec<u64>,
    constraint: Option<u64>,
    pub(super) placement: Placement,
    pub(super) previous_camera: Camera,
    pub(super) fit_requested: bool,
    tracker: Tracker,
    projected: Vec<Point2>,
    dimension: f64,
    dimension_expression: String,
    reference: bool,
    /// Ocorrências dos padrões do rascunho (o espaçamento usa `dimension`).
    pattern_count: usize,
}
impl Workspace {
    pub(super) fn tool(&self) -> Tool {
        self.tool
    }
    pub(super) fn select_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.anchors.clear();
        self.anchor_kinds.clear();
    }
    pub fn new(id: SketchId) -> Self {
        Self {
            id,
            pending: None,
            status: Status::UnderConstrained {
                degrees_of_freedom: 0,
            },
            redundant: false,
            tool: Tool::Select,
            anchors: Vec::new(),
            anchor_kinds: Vec::new(),
            selected: Vec::new(),
            constraint: None,
            placement: Placement::default(),
            previous_camera: Camera::default(),
            fit_requested: true,
            tracker: Tracker::default(),
            projected: vec![[0., 0.]],
            dimension: 10.,
            dimension_expression: "10 mm".into(),
            pattern_count: 3,
            reference: false,
        }
    }
}
fn sketch_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
fn attach(
    sketch: &mut Sketch,
    point: u64,
    kind: Option<Kind>,
    position: Point2,
) -> Result<(), String> {
    let constraint = match kind {
        Some(Kind::Endpoint(target) | Kind::Center(target)) if target != point => {
            Some(ConstraintKind::Coincident {
                a: point,
                b: target,
            })
        }
        Some(Kind::Midpoint(line)) => Some(ConstraintKind::Midpoint { point, line }),
        Some(Kind::OnCurve(line)) if sketch.line(line).is_ok() => {
            Some(ConstraintKind::PointOnLine { point, line })
        }
        Some(Kind::Projected) if nator_sketch::distance(position, [0., 0.]) < 1e-8 => {
            Some(ConstraintKind::Fixed {
                point,
                position: [0., 0.],
            })
        }
        _ => None,
    };
    if let Some(constraint) = constraint {
        sketch
            .add_constraint(constraint, false)
            .map_err(sketch_error)?;
    }
    Ok(())
}
fn point_segment_distance(p: Point2, a: Point2, b: Point2) -> f64 {
    let d = nator_sketch::sub(b, a);
    let denominator = nator_sketch::dot(d, d);
    if denominator < 1e-18 {
        return nator_sketch::distance(p, a);
    }
    let t = (nator_sketch::dot(nator_sketch::sub(p, a), d) / denominator).clamp(0., 1.);
    nator_sketch::distance(p, nator_sketch::add(a, nator_sketch::scale(d, t)))
}
impl Editor {
    pub(super) fn begin_sketch(&mut self, part: PartId, id: SketchId) {
        if self.busy() {
            return;
        }
        if self
            .current()
            .sketch_edit
            .as_ref()
            .is_some_and(|w| w.pending.is_some())
        {
            self.status = text(T::PendingEdit).into();
            return;
        }
        let Some(record) = self.current().session.document().sketch(id).cloned() else {
            return;
        };
        let target = match self
            .current()
            .plane_pick
            .as_ref()
            .filter(|p| p.destination == Some(part))
            .map(|p| Ok(p.placement))
            .unwrap_or_else(|| sketch_scene::part_placement(self.current(), part))
        {
            Ok(p) => p,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        if !record.visible {
            self.execute(Command::SetSketchVisibility { id, visible: true });
        }
        let previous = self
            .current()
            .sketch_edit
            .as_ref()
            .map(|w| w.previous_camera.clone())
            .unwrap_or_else(|| self.viewport.camera.clone());
        let plane = record.sketch.plane;
        let mut work = Workspace::new(id);
        work.placement = target;
        work.previous_camera = previous;
        work.fit_requested = false;
        let document = self.current().session.document();
        if let Ok(occurrences) = document.occurrences() {
            for occurrence in occurrences.iter().take(8) {
                if let Some(piece) = document
                    .instance(occurrence.instance)
                    .and_then(|i| document.part(i.part))
                {
                    for body in piece.bodies.iter().filter(|b| b.visible) {
                        let vertices = body.geometry.mesh().vertices();
                        let stride = (vertices.len() / 64).max(1);
                        for p in vertices.iter().step_by(stride).take(64) {
                            let world = occurrence.placement.apply(body.placement.apply(*p));
                            work.projected
                                .push(plane.project(target.inverse_apply(world)));
                        }
                    }
                }
            }
        }
        self.current_mut().active_part = Some(part);
        self.current_mut().plane_pick = None;
        self.current_mut().sketch_edit = Some(work);
        self.align_sketch_camera(!record.sketch.points.is_empty(), true);
    }
    pub(super) fn solve_sketch(&mut self, context: &egui::Context, sketch: Sketch) {
        if self.busy() {
            return;
        }
        let Some(work) = &self.current().sketch_edit else {
            return;
        };
        let id = work.id;
        let tab = self.current().id;
        let revision = self.current().session.revision();
        let document = self.current().session.document().clone();
        let parameters = self
            .current()
            .session
            .document()
            .parts()
            .iter()
            .find(|p| p.sketches.iter().any(|s| s.id == id))
            .map(|p| p.parameters.clone())
            .unwrap_or_default();
        self.start(context, None, move || {
            let mut sketch = sketch;
            nator_document::parameters::resolve_sketch(&mut sketch, &parameters)
                .map_err(sketch_error)?;
            let report = solver::solve(&sketch).map_err(sketch_error)?;
            let prepared = if report.status == Status::Conflict {
                Err(nator_commands::CommandError {
                    message: text(T::SketchConflict).into(),
                    failure: None,
                })
            } else {
                PreparedCommand::new(
                    document,
                    revision,
                    Command::UpdateSketch {
                        id,
                        sketch: report.sketch.clone(),
                    },
                )
            };
            Ok(Outcome::SketchSolved {
                tab,
                revision,
                id,
                report,
                prepared,
            })
        });
    }
    pub(super) fn working_sketch(&self) -> Option<Sketch> {
        let work = self.current().sketch_edit.as_ref()?;
        work.pending.clone().or_else(|| {
            self.current()
                .session
                .document()
                .sketch(work.id)
                .map(|r| r.sketch.clone())
        })
    }
    pub(super) fn sketch_properties(&mut self, ui: &mut egui::Ui) {
        let Some(mut sketch) = self.working_sketch() else {
            return;
        };
        let busy = self.busy();
        let parameters = self
            .current()
            .session
            .document()
            .parts()
            .iter()
            .find(|p| {
                p.sketches
                    .iter()
                    .any(|s| Some(s.id) == self.current().sketch_edit.as_ref().map(|w| w.id))
            })
            .map(|p| p.parameters.clone())
            .unwrap_or_default();
        let mut submit = false;
        let mut issue = None;
        egui::Panel::right("properties")
            .default_size(285.)
            .min_size(250.)
            .max_size(420.)
            .frame(theme::panel_frame())
            .show(ui, |ui| {
                ui.label(egui::RichText::new(text(T::Sketch)).strong().size(14.));
                egui::ScrollArea::vertical()
                    .id_salt("sketch_property_scroll")
                    .show(ui, |ui| {
                        ui.label(&sketch.name);
                        let work = self.current_mut().sketch_edit.as_mut();
                        let Some(work) = work else {
                            return;
                        };
                        let (label, color) = match work.status {
                            Status::Conflict => (text(T::SketchConflict), theme::DANGER),
                            Status::FullyConstrained => (text(T::FullyConstrained), theme::GREEN),
                            Status::UnderConstrained { .. } => {
                                (text(T::UnderConstrained), theme::ACCENT)
                            }
                        };
                        ui.colored_label(color, label);
                        if let Status::UnderConstrained { degrees_of_freedom } = work.status {
                            ui.label(format!(
                                "{}: {degrees_of_freedom}",
                                text(T::DegreesOfFreedom)
                            ));
                        }
                        if work.redundant {
                            ui.colored_label(
                                egui::Color32::from_rgb(183, 106, 17),
                                text(T::Redundant),
                            );
                        }
                        if work.pending.is_some() && ui.button(text(T::Cancel)).clicked() {
                            work.pending = None;
                            work.anchors.clear();
                            work.status = Status::UnderConstrained {
                                degrees_of_freedom: 0,
                            };
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {}",
                                text(T::SketchTools),
                                text(work.tool.label())
                            ))
                            .small()
                            .color(theme::MUTED),
                        );
                        ui.separator();
                        ui.label(text(T::Dimension));
                        if ui
                            .add(
                                egui::DragValue::new(&mut work.dimension)
                                    .range(-1e9..=1e9)
                                    .speed(0.1),
                            )
                            .changed()
                        {
                            work.dimension_expression = format!("{} mm", work.dimension);
                        }
                        ui.text_edit_singleline(&mut work.dimension_expression);
                        let evaluated = parameters.evaluate(
                            &work.dimension_expression,
                            nator_document::parameters::Dimension::Length,
                        );
                        if let Ok(value) = &evaluated {
                            work.dimension = *value;
                        } else if let Err(error) = &evaluated {
                            ui.colored_label(theme::DANGER, error.to_string());
                        }
                        ui.checkbox(&mut work.reference, text(T::ReferenceDimension));
                        if let Some(id) = work.constraint
                            && ui
                                .add_enabled(
                                    !busy && evaluated.is_ok(),
                                    egui::Button::new(text(T::EditDimension)),
                                )
                                .clicked()
                            && let Some(c) = sketch.constraints.iter_mut().find(|c| c.id == id)
                        {
                            set_dimension_value(&mut c.kind, work.dimension);
                            c.reference = work.reference;
                            bind_dimension(&mut sketch, id, work);
                            submit = true;
                        }
                        let selected = work.selected.clone();
                        if let Some(id) = selected.first().copied() {
                            ui.label(format!("{}: {id}", text(T::Entity)));
                            ui.add_enabled_ui(!busy, |ui| {
                                if ui
                                    .add_enabled(
                                        evaluated.is_ok(),
                                        egui::Button::new(text(T::ApplyDimension)),
                                    )
                                    .clicked()
                                    && let Ok(entity) = sketch.entity(id)
                                {
                                    let kind = match entity.curve {
                                        Curve::Line { start, end } => {
                                            Some(ConstraintKind::Distance {
                                                a: start,
                                                b: end,
                                                value: work.dimension,
                                            })
                                        }
                                        Curve::Circle { .. } | Curve::Arc { .. } => {
                                            Some(ConstraintKind::Radius {
                                                circle: id,
                                                value: work.dimension,
                                            })
                                        }
                                    };
                                    if let Some(kind) = kind {
                                        let old = sketch.constraints.iter_mut().find(|c| {
                                            match (&c.kind, &kind) {
                                                (
                                                    ConstraintKind::Distance { a, b, .. },
                                                    ConstraintKind::Distance { a: x, b: y, .. },
                                                ) => a == x && b == y,
                                                (
                                                    ConstraintKind::Radius { circle, .. },
                                                    ConstraintKind::Radius { circle: x, .. },
                                                ) => circle == x,
                                                _ => false,
                                            }
                                        });
                                        let dimension_id = if let Some(old) = old {
                                            old.kind = kind;
                                            old.reference = work.reference;
                                            Ok(old.id)
                                        } else {
                                            sketch.add_constraint(kind, work.reference)
                                        };
                                        match dimension_id {
                                            Ok(id) => bind_dimension(&mut sketch, id, work),
                                            Err(error) => issue = Some(error.to_string()),
                                        }
                                        submit = true;
                                    }
                                }
                                if ui.button(text(T::FixPoint)).clicked()
                                    && let Ok(entity) = sketch.entity(id)
                                    && let Some(point) = entity.point_ids().first().copied()
                                    && let Ok(position) = sketch.point(point)
                                {
                                    if let Err(error) = sketch.add_constraint(
                                        ConstraintKind::Fixed { point, position },
                                        false,
                                    ) {
                                        issue = Some(error.to_string());
                                    }
                                    submit = true;
                                }
                                if ui.button(text(T::ConstructionGeometry)).clicked()
                                    && let Some(entity) =
                                        sketch.entities.iter_mut().find(|e| e.id == id)
                                {
                                    entity.construction = !entity.construction;
                                    submit = true;
                                }
                                if ui.button(text(T::Offset)).clicked() {
                                    match nator_sketch::tools::offset_entity(
                                        &mut sketch,
                                        id,
                                        work.dimension,
                                    ) {
                                        Ok(_) => submit = true,
                                        Err(error) => issue = Some(error.to_string()),
                                    };
                                }
                                if ui.button(text(T::DeleteEntity)).clicked() {
                                    sketch.entities.retain(|e| !selected.contains(&e.id));
                                    sketch.prune_unused_points();
                                    work.selected.clear();
                                    submit = true;
                                }
                            });
                        }
                        if selected.len() == 2
                            && sketch.line(selected[0]).is_ok()
                            && sketch.line(selected[1]).is_ok()
                        {
                            for (label, rounded) in [(T::Fillet, true), (T::Chamfer, false)] {
                                if ui
                                    .add_enabled(!busy, egui::Button::new(text(label)))
                                    .clicked()
                                {
                                    match nator_sketch::tools::refine_corner(
                                        &mut sketch,
                                        selected[0],
                                        selected[1],
                                        work.dimension,
                                        rounded,
                                    ) {
                                        Ok(_) => submit = true,
                                        Err(e) => issue = Some(e.to_string()),
                                    }
                                }
                            }
                            if ui
                                .add_enabled(!busy, egui::Button::new(text(T::Extend)))
                                .clicked()
                                && let Ok((a, _)) = sketch.line(selected[0])
                            {
                                match nator_sketch::tools::extend_line(
                                    &mut sketch,
                                    selected[0],
                                    selected[1],
                                    a,
                                ) {
                                    Ok(()) => submit = true,
                                    Err(e) => issue = Some(e.to_string()),
                                }
                            }
                            if ui
                                .add_enabled(!busy, egui::Button::new(text(T::Mirror)))
                                .clicked()
                                && let Ok((a, b)) = sketch.line(selected[1])
                            {
                                match nator_sketch::tools::mirror(&mut sketch, &selected[..1], a, b)
                                {
                                    Ok(_) => submit = true,
                                    Err(e) => issue = Some(e.to_string()),
                                }
                            }
                            for (label, kind) in [
                                (
                                    T::Parallel,
                                    ConstraintKind::Parallel {
                                        a: selected[0],
                                        b: selected[1],
                                    },
                                ),
                                (
                                    T::Perpendicular,
                                    ConstraintKind::Perpendicular {
                                        a: selected[0],
                                        b: selected[1],
                                    },
                                ),
                                (
                                    T::EqualLength,
                                    ConstraintKind::EqualLength {
                                        a: selected[0],
                                        b: selected[1],
                                    },
                                ),
                            ] {
                                if ui
                                    .add_enabled(!busy, egui::Button::new(text(label)))
                                    .clicked()
                                {
                                    if let Err(error) = sketch.add_constraint(kind, false) {
                                        issue = Some(error.to_string());
                                    }
                                    submit = true;
                                }
                            }
                        }
                        // Padrões: a primeira seleção é repetida; a segunda define direção ou centro.
                        if selected.len() == 2 {
                            ui.add(
                                egui::DragValue::new(&mut work.pattern_count)
                                    .range(2..=100)
                                    .prefix(format!("{} ", text(T::PatternCount))),
                            );
                            let linear = sketch.line(selected[1]).ok();
                            let center = sketch.radial(selected[1]).ok().map(|(c, _)| c);
                            if let Some((a, b)) = linear
                                && ui
                                    .add_enabled(
                                        !busy,
                                        egui::Button::new(text(T::SketchLinearPattern)),
                                    )
                                    .clicked()
                            {
                                match nator_sketch::tools::pattern_linear(
                                    &mut sketch,
                                    &selected[..1],
                                    nator_sketch::sub(b, a),
                                    work.pattern_count,
                                    work.dimension,
                                ) {
                                    Ok(_) => submit = true,
                                    Err(e) => issue = Some(e.to_string()),
                                }
                            }
                            if let Some(center) = center
                                && ui
                                    .add_enabled(
                                        !busy,
                                        egui::Button::new(text(T::SketchCircularPattern)),
                                    )
                                    .clicked()
                            {
                                match nator_sketch::tools::pattern_circular(
                                    &mut sketch,
                                    &selected[..1],
                                    center,
                                    work.pattern_count,
                                ) {
                                    Ok(_) => submit = true,
                                    Err(e) => issue = Some(e.to_string()),
                                }
                            }
                        }
                        ui.separator();
                        ui.label(text(T::Constraints));
                        let mut remove = None;
                        egui::ScrollArea::vertical()
                            .max_height(170.)
                            .show(ui, |ui| {
                                for constraint in &sketch.constraints {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .selectable_label(
                                                work.constraint == Some(constraint.id),
                                                constraint_label(&constraint.kind),
                                            )
                                            .clicked()
                                        {
                                            work.constraint = Some(constraint.id);
                                            if let Some(value) = dimension_value(&constraint.kind) {
                                                work.dimension = value;
                                                work.dimension_expression = sketch
                                                    .dimension_expressions
                                                    .get(&constraint.id)
                                                    .cloned()
                                                    .unwrap_or_else(|| format!("{value} mm"));
                                                work.reference = constraint.reference;
                                            }
                                        }
                                        if ui
                                            .add_enabled(!busy, egui::Button::new("×").small())
                                            .clicked()
                                        {
                                            remove = Some(constraint.id);
                                        }
                                    });
                                }
                            });
                        if let Some(id) = remove {
                            sketch.constraints.retain(|c| c.id != id);
                            sketch.dimension_expressions.remove(&id);
                            work.constraint = None;
                            submit = true;
                        }
                        ui.separator();
                        if ui
                            .add_enabled(
                                !busy && work.pending.is_none(),
                                egui::Button::new(text(T::FinishSketch)),
                            )
                            .clicked()
                        {
                            self.finish_sketch();
                        }
                    });
            });
        if let Some(issue) = issue {
            self.status = issue;
        } else if submit {
            self.solve_sketch(ui.ctx(), sketch);
        }
    }
    pub(super) fn sketch_overlay(&mut self, ui: &egui::Ui, response: &egui::Response) {
        let Some(mut sketch) = self.working_sketch() else {
            return;
        };
        let Some(work) = self.current().sketch_edit.as_ref() else {
            return;
        };
        let placement = work.placement;
        let frame = sketch_scene::Frame {
            camera: self.viewport.camera.clone(),
            bounds: self.display_bounds(),
            rect: response.rect,
            placement,
            plane: sketch.plane,
        };
        let painter = ui
            .painter()
            .with_clip_rect(response.rect.intersect(ui.clip_rect()));
        let animating = self.current().camera_transition.is_some();
        let hover = if animating {
            None
        } else {
            response
                .hover_pos()
                .filter(|p| !view_cube::rect(response.rect).contains(*p))
                .and_then(|p| frame.cursor(p))
        };
        let pixels_per_mm = frame.pixels_per_mm(hover.unwrap_or([0., 0.]));
        let busy = self.busy();
        let mut submit = false;
        let Some(work) = self.current_mut().sketch_edit.as_mut() else {
            return;
        };
        // Grade no próprio plano: o mesmo conjunto de linhas acompanha a câmera,
        // em vez de substituir a área CAD por uma superfície plana independente.
        let center = frame.cursor(response.rect.center()).unwrap_or([0., 0.]);
        let step = 10_f64.powf((60. / pixels_per_mm).log10().round());
        let radius = f64::from(response.rect.width().max(response.rect.height())) / pixels_per_mm;
        for axis in 0..2 {
            for i in -15..=15 {
                let coordinate = (center[axis] / step).round() * step + f64::from(i) * step;
                let (a, b) = if axis == 0 {
                    (
                        [coordinate, center[1] - radius],
                        [coordinate, center[1] + radius],
                    )
                } else {
                    (
                        [center[0] - radius, coordinate],
                        [center[0] + radius, coordinate],
                    )
                };
                frame.path(
                    &painter,
                    [a, b],
                    egui::Stroke::new(
                        0.5,
                        egui::Color32::from_rgba_unmultiplied(130, 150, 175, 70),
                    ),
                    false,
                );
            }
        }
        let base = match work.status {
            Status::FullyConstrained => theme::GREEN,
            Status::Conflict => theme::DANGER,
            _ => theme::ACCENT,
        };
        for point in &sketch.points {
            if let Some(p) = frame.screen(point.position) {
                painter.circle_filled(p, 2.5, base);
            }
        }
        for entity in &sketch.entities {
            if let Ok(segment) = profiles::segment(&sketch, entity) {
                let color = if work.selected.contains(&entity.id) {
                    egui::Color32::from_rgb(183, 106, 17)
                } else {
                    base
                };
                frame.path(
                    &painter,
                    segment.sample(128),
                    egui::Stroke::new(1.5, color),
                    entity.construction,
                );
                for id in entity.point_ids() {
                    if let Some(p) = sketch.point(id).ok().and_then(|p| frame.screen(p)) {
                        painter.circle_filled(p, 2.5, color);
                    }
                }
            }
        }
        for constraint in &sketch.constraints {
            let annotation = match constraint.kind {
                ConstraintKind::Distance { a, b, value }
                | ConstraintKind::DeltaX { a, b, value }
                | ConstraintKind::DeltaY { a, b, value } => sketch
                    .point(a)
                    .ok()
                    .zip(sketch.point(b).ok())
                    .map(|(a, b)| {
                        (
                            nator_sketch::scale(nator_sketch::add(a, b), 0.5),
                            nator_drawing::format_mm(value),
                        )
                    }),
                ConstraintKind::Radius { circle, value } => sketch
                    .radial(circle)
                    .ok()
                    .map(|(center, _)| (center, format!("R {}", nator_drawing::format_mm(value)))),
                _ => None,
            };
            if let Some((point, label)) = annotation
                && let Some(position) = frame.screen(point)
            {
                painter.text(
                    position + egui::vec2(0., -12.),
                    egui::Align2::CENTER_BOTTOM,
                    label,
                    egui::FontId::proportional(13.),
                    theme::INK,
                );
            }
        }

        let suspended = ui.input(|i| i.modifiers.alt);
        let snap = hover.and_then(|point| {
            if suspended {
                None
            } else {
                snapping::find(
                    &sketch,
                    point,
                    8. / pixels_per_mm,
                    &work.tracker,
                    &work.projected,
                )
                .ok()
                .flatten()
            }
        });
        if let Some(snap) = &snap {
            if matches!(
                snap.kind,
                Kind::Endpoint(_) | Kind::Center(_) | Kind::Midpoint(_) | Kind::Projected
            ) {
                work.tracker.capture(snap.point);
            }
            if let Some(position) = frame.screen(snap.point) {
                painter.circle_stroke(position, 5., egui::Stroke::new(1.5, theme::WARNING));
            }
            for (a, b) in &snap.guides {
                frame.path(
                    &painter,
                    [*a, *b],
                    egui::Stroke::new(1., theme::GREEN),
                    true,
                );
            }
        }
        let cursor = snap.as_ref().map(|s| s.point).or(hover);
        if let Some(first) = work.anchors.first().copied()
            && let Some(cursor) = cursor
        {
            let points = match work.tool {
                Tool::Rectangle => vec![
                    first,
                    [cursor[0], first[1]],
                    cursor,
                    [first[0], cursor[1]],
                    first,
                ],
                Tool::Circle => profiles::Segment::Circle {
                    center: first,
                    radius: nator_sketch::distance(first, cursor),
                }
                .sample(64),
                _ => vec![first, cursor],
            };
            frame.path(&painter, points, egui::Stroke::new(1., theme::INK), false);
        }
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if escape {
            work.anchors.clear();
            work.anchor_kinds.clear();
            work.pending = None;
            work.tracker.clear();
        }
        if !busy
            && !animating
            && response.clicked_by(egui::PointerButton::Primary)
            && let Some(cursor) = cursor
        {
            if matches!(work.tool, Tool::Select | Tool::Trim) {
                let entity = sketch
                    .entities
                    .iter()
                    .filter_map(|e| {
                        profiles::segment(&sketch, e).ok().map(|curve| {
                            (
                                e.id,
                                curve
                                    .sample(128)
                                    .windows(2)
                                    .map(|p| point_segment_distance(cursor, p[0], p[1]))
                                    .fold(f64::INFINITY, f64::min),
                            )
                        })
                    })
                    .filter(|(_, d)| *d < 8. / pixels_per_mm)
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|v| v.0);
                if let Some(id) = entity {
                    if work.tool == Tool::Trim {
                        match nator_sketch::tools::trim_line(&mut sketch, id, cursor) {
                            Ok(()) => submit = true,
                            Err(error) => {
                                painter.text(
                                    response.rect.left_top(),
                                    egui::Align2::LEFT_TOP,
                                    error.to_string(),
                                    egui::FontId::proportional(13.),
                                    theme::DANGER,
                                );
                            }
                        }
                    } else {
                        if !ui.input(|i| i.modifiers.ctrl) {
                            work.selected.clear();
                        }
                        if !work.selected.contains(&id) {
                            work.selected.push(id);
                        }
                        work.constraint = None;
                        work.dimension = sketch
                            .line(id)
                            .map(|(a, b)| nator_sketch::distance(a, b))
                            .or_else(|_| sketch.radial(id).map(|(_, r)| r))
                            .unwrap_or(10.);
                        work.dimension_expression = format!("{} mm", work.dimension);
                    }
                } else {
                    work.selected.clear();
                }
            } else {
                work.anchors.push(cursor);
                work.anchor_kinds.push(snap.as_ref().map(|s| s.kind));
                work.tracker.capture(cursor);
                let needed = if work.tool == Tool::Arc { 3 } else { 2 };
                if work.anchors.len() == needed {
                    let result = draw_entity(&mut sketch, work);
                    match result {
                        Ok(()) => submit = true,
                        Err(error) => {
                            painter.text(
                                response.rect.left_top(),
                                egui::Align2::LEFT_TOP,
                                error,
                                egui::FontId::proportional(13.),
                                theme::DANGER,
                            );
                        }
                    }
                    work.anchors.clear();
                    work.anchor_kinds.clear();
                }
            }
        }
        if submit {
            self.solve_sketch(ui.ctx(), sketch);
        }
    }
}

fn draw_entity(sketch: &mut Sketch, work: &Workspace) -> Result<(), String> {
    let a = work.anchors[0];
    let b = work.anchors[1];
    match work.tool {
        Tool::Line => {
            let id = sketch.add_line(a, b, false).map_err(sketch_error)?;
            if let Curve::Line { start, end } = sketch.entity(id).map_err(sketch_error)?.curve {
                attach(sketch, start, work.anchor_kinds[0], a)?;
                attach(sketch, end, work.anchor_kinds[1], b)?;
                if (a[1] - b[1]).abs() < 1e-6 {
                    sketch
                        .add_constraint(ConstraintKind::Horizontal { line: id }, false)
                        .map_err(sketch_error)?;
                }
                if (a[0] - b[0]).abs() < 1e-6 {
                    sketch
                        .add_constraint(ConstraintKind::Vertical { line: id }, false)
                        .map_err(sketch_error)?;
                }
            }
        }
        Tool::Rectangle => {
            let offset = sketch.points.len();
            sketch.add_rectangle(a, b, false).map_err(sketch_error)?;
            attach(sketch, sketch.points[offset].id, work.anchor_kinds[0], a)?;
            attach(
                sketch,
                sketch.points[offset + 2].id,
                work.anchor_kinds[1],
                b,
            )?;
        }
        Tool::Circle => {
            let id = sketch
                .add_circle(a, nator_sketch::distance(a, b), false)
                .map_err(sketch_error)?;
            if let Curve::Circle { center, .. } = sketch.entity(id).map_err(sketch_error)?.curve {
                attach(sketch, center, work.anchor_kinds[0], a)?;
            }
        }
        Tool::Arc => {
            let radius = nator_sketch::distance(a, b);
            let end = work.anchors[2];
            let length = nator_sketch::distance(a, end);
            if radius < 1e-9 || length < 1e-9 {
                return Err(text(T::InvalidArc).into());
            }
            let end = nator_sketch::add(
                a,
                nator_sketch::scale(nator_sketch::sub(end, a), radius / length),
            );
            let center = sketch.add_point(a).map_err(sketch_error)?;
            let start = sketch.add_point(b).map_err(sketch_error)?;
            let end = sketch.add_point(end).map_err(sketch_error)?;
            sketch
                .add_curve(
                    Curve::Arc {
                        center,
                        start,
                        end,
                        clockwise: false,
                    },
                    false,
                )
                .map_err(sketch_error)?;
        }
        _ => {}
    }
    Ok(())
}

fn dimension_value(kind: &ConstraintKind) -> Option<f64> {
    match *kind {
        ConstraintKind::Distance { value, .. }
        | ConstraintKind::DeltaX { value, .. }
        | ConstraintKind::DeltaY { value, .. }
        | ConstraintKind::Radius { value, .. } => Some(value),
        _ => None,
    }
}
fn set_dimension_value(kind: &mut ConstraintKind, next: f64) {
    match kind {
        ConstraintKind::Distance { value, .. }
        | ConstraintKind::DeltaX { value, .. }
        | ConstraintKind::DeltaY { value, .. }
        | ConstraintKind::Radius { value, .. } => *value = next,
        _ => {}
    }
}
fn constraint_label(kind: &ConstraintKind) -> String {
    let key = match kind {
        ConstraintKind::Fixed { .. } => T::Fixed,
        ConstraintKind::Coincident { .. } => T::Coincident,
        ConstraintKind::Horizontal { .. } => T::Horizontal,
        ConstraintKind::Vertical { .. } => T::Vertical,
        ConstraintKind::Distance { .. } => T::Dimension,
        ConstraintKind::DeltaX { .. } => T::HorizontalDimension,
        ConstraintKind::DeltaY { .. } => T::VerticalDimension,
        ConstraintKind::Radius { .. } => T::Radius,
        ConstraintKind::Parallel { .. } => T::Parallel,
        ConstraintKind::Perpendicular { .. } => T::Perpendicular,
        ConstraintKind::EqualLength { .. } => T::EqualLength,
        ConstraintKind::Midpoint { .. } => T::Midpoint,
        ConstraintKind::PointOnLine { .. } => T::PointOnLine,
        ConstraintKind::Concentric { .. } => T::Concentric,
        ConstraintKind::Tangent { .. } => T::Tangent,
    };
    match dimension_value(kind) {
        Some(value) => format!("{}: {value:.3}", text(key)),
        None => text(key).into(),
    }
}

// Cotas de referência medem a geometria; somente cotas dirigentes recebem expressão.
fn bind_dimension(sketch: &mut Sketch, id: u64, work: &Workspace) {
    if work.reference {
        sketch.dimension_expressions.remove(&id);
    } else {
        sketch
            .dimension_expressions
            .insert(id, work.dimension_expression.clone());
    }
}
