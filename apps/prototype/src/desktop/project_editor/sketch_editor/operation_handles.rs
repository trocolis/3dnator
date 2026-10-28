//! Setas de distância do rascunho. Mover o cursor livremente não edita valores:
//! o usuário precisa pressionar a ponta e arrastar, como na extrusão.
use super::*;
use crate::desktop::project_editor::distance_arrow;
use nator_sketch::modifications::OffsetMode;

pub(super) struct Drag {
    motion: distance_arrow::Drag,
    second: bool,
}
#[derive(Default)]
pub(super) struct Update {
    pub blocked: bool,
    pub position: Option<egui::Pos2>,
}

/// Apoio, bissetriz/normal e conversão da medida para distância sobre a seta.
fn guide(source: &Sketch, work: &Workspace, action: EditAction) -> Option<(Point2, Point2, f64)> {
    if action == EditAction::Offset {
        let (at, direction) =
            nator_sketch::modifications::offset_guide(source, &work.selected).ok()?;
        Some((at, direction, 1.))
    } else {
        let corner = nator_sketch::corners::selected(source, &work.selected)
            .ok()?
            .into_iter()
            .next()?;
        let factor = if action == EditAction::Fillet {
            1. / corner.half_angle.sin()
        } else {
            corner.half_angle.cos()
        };
        Some((corner.point, corner.direction, factor))
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    ui: &egui::Ui,
    response: &egui::Response,
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    source: &Sketch,
    work: &mut Workspace,
    parameters: &nator_document::parameters::Parameters,
    action: EditAction,
) -> Update {
    let mut result = Update::default();
    #[cfg(all(test, feature = "occt"))]
    work.operation.handle_tips.clear();
    let Some((base, direction, factor)) = guide(source, work, action) else {
        work.operation.handle_drag = None;
        return result;
    };
    let Some(origin) = frame.screen(base) else {
        return result;
    };
    let Some(unit) = frame.screen(nator_sketch::add(base, direction)) else {
        return result;
    };
    let vector = unit - origin;
    if vector.length() < 1e-5 {
        return result;
    }
    let axis = vector.normalized();
    let scale = vector.length() * factor as f32;
    let length = |s: &str| {
        parameters
            .evaluate(s, nator_document::parameters::Dimension::Length)
            .ok()
    };
    let value = length(&work.dimension_expression).unwrap_or(1.);
    let second = length(&work.offset_second_expression).unwrap_or(1.);
    let sides = if action == EditAction::Offset && work.offset_mode != OffsetMode::Normal {
        2
    } else {
        1
    };
    let pointer = ui.input(|i| i.pointer.interact_pos());
    for index in 0..sides {
        let is_second = index == 1;
        let value = if is_second && work.offset_mode == OffsetMode::Double {
            second
        } else {
            value
        };
        let sign = if is_second { -1. } else { 1. };
        // Mesmo com distância zero, a ponta continua acessível e arrastável.
        let visual =
            (value.abs() as f32 * scale).max(26.) * if value < 0. { -1. } else { 1. } * sign;
        let mut tip = origin + axis * visual;
        tip.x = tip
            .x
            .clamp(frame.rect.left() + 18., frame.rect.right() - 18.);
        tip.y = tip
            .y
            .clamp(frame.rect.top() + 18., frame.rect.bottom() - 18.);
        let arrow = distance_arrow::Arrow {
            origin,
            tip,
            direction: axis * visual.signum(),
        };
        #[cfg(all(test, feature = "occt"))]
        work.operation.handle_tips.push(tip);
        let over = arrow.hovered(ui);
        let active = work
            .operation
            .handle_drag
            .as_ref()
            .is_some_and(|d| d.second == is_second);
        arrow.paint(painter, over || active);
        if over {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        result.blocked |= over || active;
        if is_second == work.operation.second_side {
            result.position = Some(tip);
        }
        if arrow.started(ui, response)
            && let Some(pointer) = ui.input(|i| i.pointer.press_origin())
        {
            // Uma caixa numérica pode estar focada: o arraste é explícito e
            // devolve o teclado à área CAD sem inserir texto na expressão.
            ui.ctx().memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
            work.operation.focus = false;
            work.operation.second_side = is_second && work.offset_mode == OffsetMode::Double;
            work.operation.handle_drag = Some(Drag {
                motion: distance_arrow::Drag {
                    pointer,
                    axis: axis * sign,
                    pixels_per_unit: scale,
                    initial: value,
                },
                second: is_second,
            });
        }
    }
    if let Some(drag) = &work.operation.handle_drag {
        result.blocked = true;
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        if let Some(pointer) = pointer {
            let mut value = drag.motion.value(pointer);
            if action != EditAction::Offset || work.offset_mode != OffsetMode::Normal {
                value = value.max(1e-6);
            }
            if value.is_finite() && value.abs() < 1e7 {
                if drag.second && work.offset_mode == OffsetMode::Double {
                    work.offset_second_expression = format!("{value} mm");
                } else {
                    work.dimension_expression = format!("{value} mm");
                }
                work.operation.confirm = false;
                ui.ctx().request_repaint();
            }
        }
        if response.drag_stopped_by(egui::PointerButton::Primary)
            || !ui.input(|i| i.pointer.primary_down())
        {
            work.operation.handle_drag = None;
            work.operation.focus = true;
        }
    }
    result
}
