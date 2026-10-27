//! Campos independentes sobre o canvas, sem janela ou caixa compartilhada.
//! A medida segue o mouse até ser digitada; Tab alterna campos, Enter confirma.
use super::*;
use nator_document::parameters::{Dimension, Parameters};
use nator_sketch::drafting::{self, Preview, Primitive};

#[derive(Default)]
pub(super) struct Input {
    pub(super) fields: [String; 2],
    pub(super) locked: [bool; 2],
    pub(super) reference_angle: f64,
    pub(super) relative: bool,
    pub(super) infinite: bool,
    active: usize,
    focus: bool,
    /// O egui pode mover foco por Tab antes de reconstruir as Areas. Conservar
    /// a propriedade anterior evita entregar essa tecla à faixa de ferramentas.
    had_focus: bool,
    pub(super) cursor: Option<Point2>,
    pub(super) arc_angle: Option<String>,
}
pub(super) struct Output {
    pub preview: Option<Preview>,
    pub confirm: bool,
    pub cancel: bool,
}
pub(super) fn primitive(tool: Tool) -> Option<Primitive> {
    match tool {
        Tool::Line => Some(Primitive::Line),
        Tool::Rectangle => Some(Primitive::Rectangle),
        Tool::Circle => Some(Primitive::Circle),
        Tool::Arc => Some(Primitive::Arc),
        _ => None,
    }
}

impl Input {
    pub(super) fn start(&mut self, cursor: Point2) {
        *self = Self {
            cursor: Some(cursor),
            focus: true,
            ..Default::default()
        };
    }
    pub(super) fn arc_next(&mut self) {
        self.arc_angle = self.locked[1].then(|| self.fields[1].clone());
        self.locked[1] = false;
        self.active = 1;
        self.focus = true;
    }
    pub(super) fn clear(&mut self, context: &egui::Context) {
        for index in 0..2 {
            context.memory_mut(|m| m.surrender_focus(field_id(index)));
        }
        *self = Self::default();
        context.request_repaint();
    }
    fn values(
        &self,
        parameters: &Parameters,
        primitive: Primitive,
    ) -> Result<[Option<f64>; 2], String> {
        let mut values = [None, None];
        for (index, value) in values.iter_mut().enumerate().take(primitive.field_count()) {
            if self.locked[index] && !(self.infinite && index == 0) {
                *value = Some(
                    parameters
                        .evaluate(
                            &self.fields[index],
                            if primitive.angular(index) {
                                Dimension::Angle
                            } else {
                                Dimension::Length
                            },
                        )
                        .map_err(sketch_error)?,
                );
            }
        }
        Ok(values)
    }
    fn preview(
        &self,
        primitive: Primitive,
        anchors: &[Point2],
        parameters: &Parameters,
    ) -> Result<Preview, String> {
        drafting::preview_with_reference(
            primitive,
            anchors,
            self.cursor.unwrap_or(anchors[0]),
            self.values(parameters, primitive)?,
            self.reference_angle,
        )
        .map_err(sketch_error)
    }

    pub(super) fn show(
        &mut self,
        context: &egui::Context,
        frame: &sketch_scene::Frame,
        primitive: Primitive,
        anchors: &[Point2],
        parameters: &Parameters,
        enabled: bool,
    ) -> Output {
        let cursor = self.cursor.unwrap_or(anchors[0]);
        let measured = drafting::preview_with_reference(
            primitive,
            anchors,
            cursor,
            [None, None],
            self.reference_angle,
        )
        .ok();
        let indices: &[usize] = if self.infinite {
            &[1]
        } else if primitive.field_count() == 1 {
            &[0]
        } else {
            &[0, 1]
        };
        if !indices.contains(&self.active) {
            self.active = indices[0];
            self.focus = true;
        }
        for &index in indices {
            if !self.locked[index]
                && let Some(measured) = &measured
            {
                let value = measured.values[index];
                self.fields[index] = if primitive.angular(index) {
                    format!("{} deg", nator_drawing::format_mm(value.to_degrees()))
                } else {
                    format!("{} mm", nator_drawing::format_mm(value))
                };
            }
        }
        let owns_keyboard = context.memory(|m| {
            m.focused()
                .is_none_or(|id| indices.iter().any(|i| field_id(*i) == id))
        }) || self.focus
            || self.had_focus;
        let cancel = enabled
            && owns_keyboard
            && context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let confirm = enabled
            && owns_keyboard
            && context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        if enabled && owns_keyboard {
            let backward =
                context.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));
            let forward =
                context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
            if backward || forward {
                context.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
                let current = indices.iter().position(|i| *i == self.active).unwrap_or(0);
                self.active = indices
                    [(current + if backward { indices.len() - 1 } else { 1 }) % indices.len()];
                self.focus = true;
            }
        }
        let next = self
            .preview(primitive, anchors, parameters)
            .ok()
            .or(measured);
        let mouse = frame.screen(cursor).unwrap_or(frame.rect.center());
        let mut positions = [mouse + egui::vec2(24., 16.), mouse + egui::vec2(24., -44.)];
        if primitive == Primitive::Line
            && let Some(preview) = &next
        {
            positions[1] = angle_position(frame, anchors[0], preview, self.reference_angle)
                + egui::vec2(8., 8.);
        } else if primitive == Primitive::Rectangle
            && let Some(preview) = &next
        {
            let a = anchors[0];
            let b = preview.point;
            positions[0] =
                frame.screen([(a[0] + b[0]) * 0.5, a[1]]).unwrap_or(mouse) + egui::vec2(8., 10.);
            positions[1] =
                frame.screen([b[0], (a[1] + b[1]) * 0.5]).unwrap_or(mouse) + egui::vec2(12., 8.);
        }
        // Transferir antes de registrar qualquer campo: o campo anterior ainda
        // teria foco ao ser desenhado primeiro e poderia desfazer a troca por Tab.
        if self.focus && enabled {
            context.memory_mut(|m| m.request_focus(field_id(self.active)));
        }
        let clamp = |position: egui::Pos2| {
            egui::pos2(
                position.x.clamp(
                    frame.rect.left() + 4.,
                    (frame.rect.right() - 126.).max(frame.rect.left() + 4.),
                ),
                position.y.clamp(
                    frame.rect.top() + 4.,
                    (frame.rect.bottom() - 32.).max(frame.rect.top() + 4.),
                ),
            )
        };
        positions = positions.map(clamp);
        if indices.len() == 2 {
            // Em linhas curtas ou próximas da borda, os dois apoios podem
            // coincidir na tela. Conservar o comprimento junto ao mouse e
            // deslocar o ângulo o mínimo necessário, deixando uma folga visível.
            let size = egui::vec2(118., 24.);
            let length = egui::Rect::from_min_size(positions[0], size).expand(6.);
            if length.intersects(egui::Rect::from_min_size(positions[1], size)) {
                let desired = positions[1];
                if let Some(next) = [
                    egui::pos2(desired.x, length.top() - size.y - 6.),
                    egui::pos2(desired.x, length.bottom() + 6.),
                ]
                .into_iter()
                .map(clamp)
                .filter(|p| !length.intersects(egui::Rect::from_min_size(*p, size)))
                .min_by(|a, b| a.distance_sq(desired).total_cmp(&b.distance_sq(desired)))
                {
                    positions[1] = next;
                }
            }
        }
        for &index in indices {
            let position = positions[index];
            egui::Area::new(egui::Id::new(("sketch_dynamic_input", index)))
                .order(egui::Order::Foreground)
                .fixed_pos(position)
                .show(context, |ui| {
                    let label = match (primitive, index) {
                        (Primitive::Rectangle, 0) => T::SketchDynamicWidth,
                        (Primitive::Rectangle, _) => T::SketchDynamicHeight,
                        (Primitive::Line, 0) => T::Length,
                        (Primitive::Arc, 1) if anchors.len() == 2 => T::SketchSweep,
                        (_, 1) => T::SketchAngleValue,
                        _ => T::Radius,
                    };
                    let label = if primitive.angular(index) && anchors.len() == 1 {
                        format!(
                            "{}\n{}",
                            text(label),
                            text(if self.relative {
                                T::SketchRelativeAngleHint
                            } else {
                                T::SketchHorizontalAngleHint
                            })
                        )
                    } else {
                        text(label).to_owned()
                    };
                    self.field(ui, index, &label, enabled);
                });
        }
        if context.memory(|m| m.has_focus(field_id(self.active))) {
            self.focus = false;
        }
        self.had_focus = context.memory(|m| indices.iter().any(|i| m.has_focus(field_id(*i))));
        let result = self.preview(primitive, anchors, parameters);
        let painter = context
            .layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                egui::Id::new("sketch_angle_preview"),
            ))
            .with_clip_rect(frame.rect);
        if primitive == Primitive::Line
            && let Ok(preview) = &result
        {
            paint_angle(&painter, frame, anchors[0], preview, self.reference_angle);
        }
        if let Err(error) = &result {
            painter.text(
                mouse + egui::vec2(24., 48.),
                egui::Align2::LEFT_TOP,
                error,
                egui::FontId::proportional(12.),
                theme::DANGER,
            );
        }
        Output {
            preview: result.ok(),
            confirm,
            cancel,
        }
    }

    fn field(&mut self, ui: &mut egui::Ui, index: usize, label: &str, enabled: bool) {
        let id = field_id(index);
        // Antes de entregar eventos ao TextEdit, selecionar a medida automática:
        // a primeira tecla deve substituir o número medido, nunca anexar texto.
        if (!self.locked[index] || self.focus) && ui.memory(|m| m.has_focus(id)) {
            select_text(ui.ctx(), id, &self.fields[index]);
        }
        let response = ui.add_enabled(
            enabled,
            egui::TextEdit::singleline(&mut self.fields[index])
                .id(id)
                .desired_width(114.)
                .hint_text(label),
        );
        if response.gained_focus() && !response.changed() {
            select_text(ui.ctx(), id, &self.fields[index]);
        }
        if response.changed() {
            self.locked[index] = !self.fields[index].trim().is_empty();
        }
        if response.has_focus() {
            self.active = index;
        }
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, enabled, label));
        if self.locked[index] {
            ui.painter().circle_filled(
                response.rect.right_top() + egui::vec2(-4., 4.),
                2.5,
                theme::ACCENT,
            );
        }
        response.on_hover_text(format!(
            "{label}\n{}\n{}",
            text(T::SketchDynamicHint),
            text(T::SketchValueLocked)
        ));
    }
}

fn field_id(index: usize) -> egui::Id {
    egui::Id::new(("sketch_dynamic_field", index))
}

fn select_text(context: &egui::Context, id: egui::Id, text: &str) {
    let mut state = egui::TextEdit::load_state(context, id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(text.chars().count()),
        )));
    state.store(context, id);
}

fn angle_radius(frame: &sketch_scene::Frame, origin: Point2, preview: &Preview) -> f64 {
    let pixels = frame.pixels_per_mm(origin);
    (nator_sketch::distance(origin, preview.point) * 0.45).clamp(18. / pixels, 52. / pixels)
}
fn angle_position(
    frame: &sketch_scene::Frame,
    origin: Point2,
    preview: &Preview,
    reference: f64,
) -> egui::Pos2 {
    let middle = reference + preview.values[1] * 0.5;
    let radius = angle_radius(frame, origin, preview);
    frame
        .screen(nator_sketch::add(
            origin,
            [radius * middle.cos(), radius * middle.sin()],
        ))
        .unwrap_or(frame.rect.center())
}
fn paint_angle(
    painter: &egui::Painter,
    frame: &sketch_scene::Frame,
    origin: Point2,
    preview: &Preview,
    reference: f64,
) {
    if nator_sketch::distance(origin, preview.point) < 1e-9 {
        return;
    }
    let radius = angle_radius(frame, origin, preview);
    let point = |angle: f64, scale: f64| {
        nator_sketch::add(
            origin,
            [radius * scale * angle.cos(), radius * scale * angle.sin()],
        )
    };
    let stroke = egui::Stroke::new(1., egui::Color32::BLACK);
    frame.path(painter, [origin, point(reference, 1.2)], stroke, true);
    frame.path(
        painter,
        (0..=48).map(|i| point(reference + preview.values[1] * f64::from(i) / 48., 1.)),
        stroke,
        true,
    );
}
