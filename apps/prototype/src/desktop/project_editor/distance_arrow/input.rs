//! Campo transitório comum aos manipuladores. Enter fixa só a medida;
//! cancelar restaura o início do gesto, sem publicar comandos no documento.
use super::*;

#[derive(Default)]
pub(in crate::desktop::project_editor) struct Input {
    active: bool,
    pub typing: bool,
    text: String,
    initial: f64,
    position: Option<egui::Pos2>,
    focus: bool,
    invalid: bool,
    held: bool,
}
#[derive(Default)]
pub(in crate::desktop::project_editor) struct Edit {
    pub value: Option<f64>,
    pub finished: bool,
    pub blocked: bool,
}
impl Input {
    pub fn begin(&mut self, value: f64, position: egui::Pos2) {
        *self = Self {
            active: true,
            text: nator_drawing::format_mm(value),
            initial: value,
            position: Some(position),
            focus: true,
            ..Default::default()
        };
    }
    pub fn focus(&mut self) {
        self.focus = true;
    }
    pub fn valid(&self) -> bool {
        !self.active || !self.invalid
    }
    pub fn active(&self) -> bool {
        self.active
    }
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &egui::Ui,
        id: &'static str,
        canvas: egui::Rect,
        value: f64,
        dragging: bool,
        label: T,
        parse: impl Fn(&str) -> Option<f64>,
    ) -> Edit {
        let down = ui.input(|i| i.pointer.primary_down());
        if !down {
            self.held = false;
        }
        let mut edit = Edit {
            blocked: self.held,
            ..Default::default()
        };
        if !self.active {
            return edit;
        }
        if dragging && !self.typing {
            self.position = ui.input(|i| i.pointer.interact_pos()).or(self.position);
            self.text = nator_drawing::format_mm(value);
        }
        let at = self.position.unwrap_or(canvas.center()) + egui::vec2(18., 18.);
        let at = egui::pos2(
            at.x.clamp(canvas.left(), (canvas.right() - 230.).max(canvas.left())),
            at.y.clamp(canvas.top(), (canvas.bottom() - 64.).max(canvas.top())),
        );
        if dragging && !down {
            self.focus = true;
        }
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let mut rect = egui::Rect::NOTHING;
        egui::Area::new(egui::Id::new(("arrow_input", id)))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .stroke(egui::Stroke::new(
                        1.,
                        if self.invalid {
                            theme::DANGER
                        } else {
                            theme::ACCENT
                        },
                    ))
                    .corner_radius(4)
                    .inner_margin(5)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(text(label));
                            let field_id = egui::Id::new(("arrow_value", id));
                            if !self.typing
                                && let Some(mut state) =
                                    egui::TextEdit::load_state(ui.ctx(), field_id)
                            {
                                state
                                    .cursor
                                    .set_char_range(Some(egui::text::CCursorRange::two(
                                        egui::text::CCursor::new(0),
                                        egui::text::CCursor::new(self.text.chars().count()),
                                    )));
                                state.store(ui.ctx(), field_id);
                            }
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.text)
                                    .id(egui::Id::new(("arrow_value", id)))
                                    .desired_width(108.)
                                    .font(egui::TextStyle::Monospace),
                            );
                            rect = response.rect;
                            if response.changed() {
                                self.typing = true;
                            }
                            if std::mem::take(&mut self.focus) {
                                response.request_focus();
                                if let Some(mut state) =
                                    egui::TextEdit::load_state(ui.ctx(), response.id)
                                {
                                    state.cursor.set_char_range(Some(
                                        egui::text::CCursorRange::two(
                                            egui::text::CCursor::new(0),
                                            egui::text::CCursor::new(self.text.chars().count()),
                                        ),
                                    ));
                                    state.store(ui.ctx(), response.id);
                                }
                            }
                        });
                        self.invalid = parse(&self.text).is_none();
                        if self.invalid {
                            ui.colored_label(theme::DANGER, text(T::ArrowInvalidValue));
                        } else {
                            ui.label(
                                egui::RichText::new(text(T::ArrowValueHint))
                                    .small()
                                    .color(theme::MUTED),
                            );
                        }
                    });
            });
        #[cfg(all(test, feature = "occt"))]
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new(("arrow_rect", id)), rect));
        if enter || escape {
            ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
            });
            if escape {
                edit.value = Some(self.initial);
                edit.finished = true;
            } else if let Some(value) = parse(&self.text) {
                edit.value = Some(value);
                edit.finished = true;
            } else {
                ui.ctx()
                    .memory_mut(|m| m.request_focus(egui::Id::new(("arrow_value", id))));
            }
            if edit.finished {
                self.active = false;
                self.held = down;
                ui.ctx()
                    .memory_mut(|m| m.surrender_focus(egui::Id::new(("arrow_value", id))));
            }
        }
        // Clicar outro controle deixa a medida atual e permite editar pelo painel.
        if !dragging
            && ui.input(|i| i.pointer.any_pressed())
            && !ui
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| rect.expand(6.).contains(p))
        {
            self.active = false;
        }
        edit.blocked |= self.active || self.held || edit.finished;
        edit
    }
}
