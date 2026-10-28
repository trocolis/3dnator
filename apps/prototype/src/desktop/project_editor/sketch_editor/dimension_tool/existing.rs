//! Seleção, arraste do texto e edição das cotas já publicadas. O arraste altera
//! somente a posição relativa do texto; valor, identidade e tipo são preservados.
use super::*;

#[derive(Default)]
pub(crate) struct Existing {
    selected: Option<u64>,
    drag: Option<Drag>,
    edit: Option<(u64, Draft)>,
}
impl Existing {
    pub(crate) fn selected(&self) -> Option<u64> {
        self.selected
    }
}
struct Drag {
    id: u64,
    start: Point2,
    text: Point2,
}
#[derive(Default)]
pub(crate) struct Interaction {
    pub blocked: bool,
    pub changed: Option<u64>,
    pub warning: Option<String>,
}

fn text_rect(
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    sketch: &Sketch,
    id: u64,
    gap: f64,
) -> Option<egui::Rect> {
    let constraint = sketch.constraints.iter().find(|c| c.id == id)?;
    let text = caption(&constraint.kind, constraint.reference)?;
    let layout = layout(
        sketch,
        &constraint.kind,
        sketch.dimension_labels.get(&id).copied(),
        gap,
    )?;
    let at = frame.screen(layout.text)?;
    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(13.), theme::INK);
    Some(egui::Rect::from_center_size(at, galley.size()).expand(5.))
}

/// Mesmo retângulo do texto desenhado, incluindo a margem de captura. A ordem
/// inversa permite alcançar a última cota quando dois textos se sobrepõem.
pub(super) fn hit(
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    sketch: &Sketch,
    position: egui::Pos2,
) -> Option<u64> {
    let gap = f64::from(DEFAULT_GAP) / frame.pixels_per_mm([0., 0.]);
    sketch
        .constraints
        .iter()
        .rev()
        .find(|c| {
            text_rect(frame, painter, sketch, c.id, gap).is_some_and(|r| r.contains(position))
        })
        .map(|c| c.id)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn interact_existing(
    ui: &egui::Ui,
    response: &egui::Response,
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    sketch: &mut Sketch,
    state: &mut Existing,
    enabled: bool,
    parameters: &Parameters,
) -> Interaction {
    if !enabled {
        return Interaction::default();
    }
    let mut output = Interaction::default();
    let gap = f64::from(DEFAULT_GAP) / frame.pixels_per_mm([0., 0.]);
    let pointer = response
        .hover_pos()
        .or_else(|| ui.input(|i| i.pointer.interact_pos()));
    let hovered = pointer
        .filter(|p| frame.rect.contains(*p))
        .and_then(|p| hit(frame, painter, sketch, p));
    if hovered.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        output.blocked = state.drag.is_some() || state.edit.is_some();
        state.drag = None;
        state.edit = None;
        return output;
    }
    if let Some((id, draft)) = &mut state.edit {
        output.blocked = true;
        if let Some(constraint) = sketch.constraints.iter().find(|c| c.id == *id)
            && let Some(layout) = layout(
                sketch,
                &constraint.kind,
                sketch.dimension_labels.get(id).copied(),
                gap,
            )
            && let Some(at) = frame.screen(layout.text)
            && value_box(ui, draft, at, &constraint.kind, parameters, false)
        {
            match typed_value(&constraint.kind, &draft.text, parameters) {
                Ok(value) => {
                    let id = *id;
                    if let Some(constraint) = sketch.constraints.iter_mut().find(|c| c.id == id)
                        && let Some(target) = constraint.kind.dimension_value_mut()
                    {
                        *target = value;
                    }
                    sketch
                        .dimension_expressions
                        .insert(id, draft.text.trim().into());
                    output.changed = Some(id);
                    state.edit = None;
                }
                Err(message) => output.warning = Some(message),
            }
        }
        return output;
    }
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(start_screen) = ui.input(|i| i.pointer.press_origin())
        && let Some(id) = hit(frame, painter, sketch, start_screen)
        && let Some(start) = frame.cursor(start_screen)
        && let Some(constraint) = sketch.constraints.iter().find(|c| c.id == id)
        && let Some(layout) = layout(
            sketch,
            &constraint.kind,
            sketch.dimension_labels.get(&id).copied(),
            gap,
        )
    {
        state.selected = Some(id);
        state.drag = Some(Drag {
            id,
            start,
            text: layout.text,
        });
    }
    if let Some(drag) = &state.drag {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        output.blocked = true;
        if let Some(cursor) = pointer.and_then(|p| frame.cursor(p))
            && let Some(constraint) = sketch.constraints.iter().find(|c| c.id == drag.id)
            && let Some(anchor) =
                layout(sketch, &constraint.kind, Some([0., 0.]), gap).map(|l| l.text)
        {
            let text = add(drag.text, sub(cursor, drag.start));
            sketch.dimension_labels.insert(drag.id, sub(text, anchor));
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                output.changed = Some(drag.id);
            }
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            state.drag = None;
        }
    } else if response.clicked_by(egui::PointerButton::Primary)
        || response.double_clicked_by(egui::PointerButton::Primary)
    {
        state.selected = hovered;
        output.blocked = hovered.is_some();
        if response.double_clicked_by(egui::PointerButton::Primary)
            && let Some(id) = hovered
            && let Some(constraint) = sketch.constraints.iter().find(|c| c.id == id)
        {
            if constraint.reference {
                output.warning = Some(text(T::DimensionReferenceReadOnly).into());
            } else {
                let value = constraint.kind.dimension_value().unwrap_or_default();
                state.edit = Some((
                    id,
                    Draft {
                        text: sketch
                            .dimension_expressions
                            .get(&id)
                            .cloned()
                            .unwrap_or_else(|| measured_text(&constraint.kind, value)),
                        editing: true,
                        focus: true,
                        ..Default::default()
                    },
                ));
            }
        }
    }
    if let Some(id) = state.selected.or(hovered)
        && let Some(constraint) = sketch.constraints.iter().find(|c| c.id == id)
        && let Some(layout) = layout(
            sketch,
            &constraint.kind,
            sketch.dimension_labels.get(&id).copied(),
            gap,
        )
    {
        paint(
            frame,
            painter,
            &layout,
            caption(&constraint.kind, constraint.reference).as_deref(),
            theme::ACCENT,
        );
    }
    output
}
