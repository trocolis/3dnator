//! Entrada junto ao desenho. Cada campo acompanha o cursor até ser digitado;
//! Tab muda o foco sem descartar valores travados, Enter aceita a etapa atual.
use super::*;
use nator_document::parameters::{Dimension, Parameters};
use nator_sketch::drafting::{self, Preview, Primitive};

#[derive(Default)]
pub(super) struct Input {
    pub(super) fields: [String; 2],
    pub(super) locked: [bool; 2],
    active: usize,
    focus: bool,
    /// O egui pode transferir o foco por Tab antes de reconstruir esta Area.
    /// Guardar a propriedade anterior permite consumir a tecla e restituir o campo correto.
    had_focus: bool,
    pub(super) cursor: Option<Point2>,
    /// Orientação escolhida ao confirmar a segunda etapa do arco.
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
        let mut result = [None, None];
        for (index, value) in result.iter_mut().enumerate().take(primitive.field_count()) {
            if self.locked[index] {
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
        Ok(result)
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
        let measured = drafting::preview(primitive, anchors, cursor, [None, None]).ok();
        for index in 0..primitive.field_count() {
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
                .is_none_or(|id| (0..2).any(|i| field_id(i) == id))
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
                // Consumir o evento não desfaz a navegação já agendada no começo
                // do passe pelo egui. Impedir uma segunda troca ao registrar o campo.
                context.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
                let count = primitive.field_count();
                self.active = (self.active + if backward { count - 1 } else { 1 }) % count;
                self.focus = true;
            }
        }
        let position =
            frame.screen(anchors[0]).unwrap_or(frame.rect.center()) + egui::vec2(26., 26.);
        let position = egui::pos2(
            position.x.clamp(
                frame.rect.left() + 8.,
                (frame.rect.right() - 278.).max(frame.rect.left() + 8.),
            ),
            position.y.clamp(
                frame.rect.top() + 8.,
                (frame.rect.bottom() - 144.).max(frame.rect.top() + 8.),
            ),
        );
        let mut preview = None;
        egui::Area::new(egui::Id::new("sketch_dynamic_input"))
            .order(egui::Order::Foreground)
            .fixed_pos(position)
            .show(context, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(250.);
                    if self.focus && enabled {
                        ui.memory_mut(|m| m.request_focus(field_id(self.active)));
                    }
                    ui.add_enabled_ui(enabled, |ui| {
                        for index in 0..primitive.field_count() {
                            ui.horizontal(|ui| {
                                let label = match (primitive, index) {
                                    (Primitive::Rectangle, 0) => T::SketchDynamicWidth,
                                    (Primitive::Rectangle, _) => T::SketchDynamicHeight,
                                    (Primitive::Line, 0) => T::Length,
                                    (Primitive::Arc, 1) if anchors.len() == 2 => T::SketchSweep,
                                    (_, 1) => T::SketchAngleValue,
                                    _ => T::Radius,
                                };
                                let label_response =
                                    ui.add_sized([91., 24.], egui::Label::new(text(label)));
                                if primitive.angular(index) && anchors.len() == 1 {
                                    label_response
                                        .on_hover_text(text(T::SketchHorizontalAngleHint));
                                }
                                let id = field_id(index);
                                // O texto medido é selecionado antes do evento de digitação:
                                // a primeira tecla substitui a medida, em vez de anexar a ela.
                                if (!self.locked[index] || self.focus)
                                    && ui.memory(|m| m.has_focus(id))
                                {
                                    let mut state =
                                        egui::TextEdit::load_state(context, id).unwrap_or_default();
                                    state.cursor.set_char_range(Some(
                                        egui::text::CCursorRange::two(
                                            egui::text::CCursor::new(0),
                                            egui::text::CCursor::new(
                                                self.fields[index].chars().count(),
                                            ),
                                        ),
                                    ));
                                    state.store(context, id);
                                }
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut self.fields[index])
                                        .id(id)
                                        .desired_width(123.),
                                );
                                if response.gained_focus()
                                    && !response.changed()
                                    && let Some(mut state) = egui::TextEdit::load_state(context, id)
                                {
                                    state.cursor.set_char_range(Some(
                                        egui::text::CCursorRange::two(
                                            egui::text::CCursor::new(0),
                                            egui::text::CCursor::new(
                                                self.fields[index].chars().count(),
                                            ),
                                        ),
                                    ));
                                    state.store(context, id);
                                }
                                if response.changed() {
                                    self.locked[index] = !self.fields[index].trim().is_empty();
                                }
                                if response.has_focus() {
                                    self.active = index;
                                }
                                if self.locked[index] {
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(10., 24.),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().circle_filled(rect.center(), 3., theme::ACCENT);
                                    response.on_hover_text(text(T::SketchValueLocked));
                                }
                            });
                        }
                    });
                    // Uma Area nova mede o conteúdo num passe invisível, sem foco.
                    // Manter o pedido até o campo participar de um passe interativo.
                    if ui.memory(|m| m.has_focus(field_id(self.active))) {
                        self.focus = false;
                    }
                    self.had_focus = ui
                        .memory(|m| (0..primitive.field_count()).any(|i| m.has_focus(field_id(i))));
                    match self.values(parameters, primitive).and_then(|values| {
                        drafting::preview(primitive, anchors, cursor, values).map_err(sketch_error)
                    }) {
                        Ok(next) => preview = Some(next),
                        Err(error) => {
                            ui.label(egui::RichText::new(error).small().color(theme::DANGER));
                        }
                    }
                    ui.label(
                        egui::RichText::new(text(T::SketchDynamicHint))
                            .small()
                            .color(theme::MUTED),
                    );
                });
            });
        Output {
            preview,
            confirm,
            cancel,
        }
    }
}
fn field_id(index: usize) -> egui::Id {
    egui::Id::new(("sketch_dynamic_field", index))
}
