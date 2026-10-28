//! Tabela de parâmetros por definição de peça, sem ligações entre projetos ou peças.
use super::*;
use nator_document::parameters::Dimension;

pub(super) struct Draft {
    pub tab: u64,
    pub part: PartId,
    name: String,
    expression: String,
    dimension: Dimension,
}
impl Editor {
    pub(super) fn open_parameters(&mut self) {
        if let Some(part) = self.current().active_part {
            self.parameters_edit = Some(Draft {
                tab: self.current().id,
                part,
                name: String::new(),
                expression: "10 mm".into(),
                dimension: Dimension::Length,
            });
        }
    }
    pub(super) fn prepare_command(
        &mut self,
        context: &egui::Context,
        tab_id: u64,
        command: Command,
    ) {
        if self.busy() {
            return;
        }
        let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) else {
            return;
        };
        if tab
            .sketch_edit
            .as_ref()
            .is_some_and(|w| w.pending.is_some())
        {
            self.status.warning(text(T::PendingEdit));
            return;
        }
        let document = tab.session.document().clone();
        let revision = tab.session.revision();
        let retry = command.clone();
        self.start(context, None, move || {
            match PreparedCommand::new(document, revision, command) {
                Ok(command) => Ok(Outcome::Prepared {
                    tab: tab_id,
                    command,
                }),
                // Falhas atribuídas a uma operação viram decisão do usuário, não só status.
                Err(error) if error.failure.is_some() => Ok(Outcome::Rejected {
                    tab: tab_id,
                    command: retry,
                    error,
                }),
                Err(error) => Err(error.to_string()),
            }
        });
    }
    pub(super) fn parameter_dialog(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.parameters_edit.take() else {
            return;
        };
        let Some(part) = self
            .tabs
            .iter()
            .find(|t| t.id == draft.tab)
            .and_then(|t| t.session.document().part(draft.part))
            .cloned()
        else {
            return;
        };
        let mut command = None;
        let title = format!("{} · {}", text(T::Parameters), part.name);
        let outcome = command_panel::options(
            context,
            command_panel::Header {
                id: "parameters",
                icon: icons::Icon::Parameters,
                title: &title,
            },
            self.canvas_rect,
            540.,
            |ui| {
                egui::Grid::new("parameters_table")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong(text(T::ParameterName));
                        ui.strong(text(T::Expression));
                        ui.strong(text(T::CurrentValue));
                        ui.end_row();
                        for (name, parameter) in &part.parameters.entries {
                            if ui.selectable_label(draft.name == *name, name).clicked() {
                                draft.name = name.clone();
                                draft.expression = parameter.expression.clone();
                                draft.dimension = parameter.dimension;
                            }
                            ui.label(&parameter.expression);
                            match part
                                .parameters
                                .evaluate(&parameter.expression, parameter.dimension)
                            {
                                Ok(value) => {
                                    ui.monospace(match parameter.dimension {
                                        Dimension::Length => format!("{} mm", self.mm(value)),
                                        Dimension::Angle => {
                                            format!("{} graus", self.mm(value.to_degrees()))
                                        }
                                        Dimension::Scalar => self.mm(value),
                                    });
                                }
                                Err(error) => {
                                    ui.colored_label(theme::DANGER, error.to_string());
                                }
                            }
                            ui.end_row();
                        }
                    });
                ui.separator();
                ui.label(text(T::ParameterName));
                ui.text_edit_singleline(&mut draft.name);
                ui.label(text(T::Expression));
                ui.text_edit_singleline(&mut draft.expression);
                let label = |kind: Dimension| {
                    text(match kind {
                        Dimension::Length => T::LengthKind,
                        Dimension::Angle => T::AngleKind,
                        Dimension::Scalar => T::ScalarKind,
                    })
                };
                egui::ComboBox::from_id_salt("parameter_dimension")
                    .selected_text(label(draft.dimension))
                    .show_ui(ui, |ui| {
                        for kind in [Dimension::Length, Dimension::Angle, Dimension::Scalar] {
                            ui.selectable_value(&mut draft.dimension, kind, label(kind));
                        }
                    });
                ui.small(text(T::ExpressionHint));
                ui.add_enabled_ui(!self.busy(), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(text(T::ApplyParameter)).clicked() {
                            command = Some(Command::SetParameter {
                                part: draft.part,
                                name: draft.name.trim().into(),
                                expression: Some(draft.expression.clone()),
                                dimension: draft.dimension,
                            });
                        }
                        if ui
                            .add_enabled(
                                part.parameters.entries.contains_key(&draft.name),
                                egui::Button::new(text(T::DeleteParameter)),
                            )
                            .clicked()
                        {
                            command = Some(Command::SetParameter {
                                part: draft.part,
                                name: draft.name.clone(),
                                expression: None,
                                dimension: draft.dimension,
                            });
                        }
                    });
                });
            },
        );
        let open = !outcome.cancel;
        if let Some(command) = command {
            self.prepare_command(context, draft.tab, command);
        }
        if open {
            self.parameters_edit = Some(draft);
        }
    }
}
