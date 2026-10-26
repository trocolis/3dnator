//! Linha do tempo do histórico paramétrico da peça ativa: controles de reprodução, marcador
//! arrastável para voltar a uma etapa, reordenação por arraste e menu de contexto por
//! operação. Toda alteração passa pelos comandos; a ordem só muda se as dependências
//! continuarem válidas, e a validação final é sempre a do documento.
use super::{
    icons::{self, Icon},
    *,
};
use nator_commands::RebuildFailure;
use nator_document::features::{Feature, FeatureState, Operation};

/// Lado de cada operação na faixa, em pontos.
const TILE: f32 = 30.;

#[derive(Default)]
pub(super) struct TimelineState {
    /// Operação destacada: aba, peça e resultado.
    pub selected: Option<(u64, PartId, BodyId)>,
    rename: Option<Rename>,
    /// Exclusão aguardando confirmação por haver dependentes.
    delete: Option<(u64, PartId, BodyId)>,
    drag: Option<Drag>,
    /// Posição de cada operação no último quadro (capturas e ensaios de eventos).
    pub tiles: Vec<(BodyId, egui::Rect)>,
}

struct Rename {
    tab: u64,
    part: PartId,
    output: BodyId,
    name: String,
    focus: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Drag {
    Marker,
    Feature(BodyId),
}

/// Edição rejeitada por uma falha de recálculo, aguardando a decisão do usuário: corrigir a
/// operação, suprimi-la junto da edição ou cancelar. Enquanto isso, o documento continua
/// com o último resultado válido.
pub(super) struct PendingFailure {
    pub tab: u64,
    pub command: Command,
    pub failure: RebuildFailure,
    /// Rascunho cuja alteração pendente é aplicada junto com a resolução.
    pub sketch: Option<SketchId>,
    /// O editor da operação está aberto para corrigi-la; o diálogo fica oculto.
    pub fixing: bool,
}

pub(super) fn operation_icon(operation: &Operation) -> Icon {
    match operation {
        Operation::Extrude { .. } => Icon::Extrude,
        Operation::Revolve { .. } => Icon::Revolve,
        Operation::Hole { .. } => Icon::Hole,
        Operation::Fillet { .. } => Icon::Fillet,
        Operation::Chamfer { .. } => Icon::Chamfer,
        Operation::Shell { .. } => Icon::Shell,
        Operation::Boolean { .. } => Icon::Boolean,
        Operation::Pattern { .. } => Icon::Pattern,
        Operation::Mirror { .. } => Icon::Mirror,
        Operation::Split { .. } => Icon::Split,
        Operation::Transform { .. } => Icon::Move,
    }
}

fn state_label(state: FeatureState) -> T {
    match state {
        FeatureState::Active => T::StateActive,
        FeatureState::Suppressed => T::StateSuppressed,
        FeatureState::Dependent => T::StateDependent,
        FeatureState::RolledBack => T::StateRolledBack,
    }
}

/// Posição de inserção para um ponteiro em `x`: quantidade de operações à esquerda dele.
fn slot(tiles: &[egui::Rect], x: f32) -> usize {
    tiles.iter().filter(|r| r.center().x < x).count()
}

/// Destaque visual de uma operação na faixa.
#[derive(Clone, Copy)]
struct Look {
    state: FeatureState,
    selected: bool,
    failed: bool,
    affected: bool,
}

fn paint_tile(ui: &egui::Ui, response: &egui::Response, operation: &Operation, look: Look) {
    let rect = response.rect;
    let painter = ui.painter();
    let fill = if look.failed {
        egui::Color32::from_rgb(253, 225, 225)
    } else if look.affected {
        egui::Color32::from_rgb(255, 238, 214)
    } else if look.selected {
        theme::BLUE_FILL
    } else if response.hovered() {
        egui::Color32::from_rgb(240, 245, 251)
    } else {
        egui::Color32::WHITE
    };
    let stroke = if look.failed {
        egui::Stroke::new(1.5, theme::DANGER)
    } else if look.affected {
        egui::Stroke::new(1.5, theme::WARNING)
    } else if look.selected {
        egui::Stroke::new(1.5, theme::ACCENT)
    } else {
        egui::Stroke::new(1., theme::BORDER)
    };
    painter.rect(rect, 4, fill, stroke, egui::StrokeKind::Inside);
    let icon = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(20.));
    icons::paint(
        painter,
        icon,
        operation_icon(operation),
        look.state.is_active(),
    );
    match look.state {
        // Supressão explícita: traço vermelho, como uma operação riscada.
        FeatureState::Suppressed => {
            painter.line_segment(
                [
                    rect.left_bottom() + egui::vec2(5., -5.),
                    rect.right_top() + egui::vec2(-5., 5.),
                ],
                egui::Stroke::new(1.5, theme::DANGER),
            );
        }
        // Posterior ao marcador: véu claro sobre a operação.
        FeatureState::RolledBack => {
            painter.rect_filled(
                rect,
                4,
                egui::Color32::from_rgba_unmultiplied(233, 238, 245, 150),
            );
        }
        FeatureState::Dependent | FeatureState::Active => {}
    }
}

impl Editor {
    /// Faixa da linha do tempo da peça ativa, dentro da barra inferior.
    pub(super) fn timeline_strip(&mut self, ui: &mut egui::Ui) {
        let tab = self.current().id;
        let Some(part) = self
            .current()
            .active_part
            .and_then(|id| self.current().session.document().part(id))
            .cloned()
            .filter(|p| !p.features.is_empty())
        else {
            ui.label(
                egui::RichText::new(text(T::TimelineEmpty))
                    .small()
                    .color(theme::MUTED),
            );
            return;
        };
        let states = nator_document::features::states(&part);
        let count = part.features.len();
        let marker = part.history_marker.unwrap_or(count);
        let editable = !self.busy() && self.pending_failure.is_none();
        let mut command = None;
        let mut edit: Option<Feature> = None;
        let mut sketch = None;
        let mut delete = None;
        ui.add_enabled_ui(editable, |ui| {
            ui.spacing_mut().item_spacing.x = 1.;
            for (icon, label, target) in [
                (Icon::First, T::TimelineFirst, Some(0)),
                (Icon::Previous, T::TimelinePrevious, marker.checked_sub(1)),
                (
                    Icon::Next,
                    T::TimelineNext,
                    (marker < count).then_some(marker + 1),
                ),
                (Icon::Last, T::TimelineLast, Some(count)),
            ] {
                let enabled = target.is_some_and(|t| t != marker);
                let clicked = ui
                    .add_enabled_ui(enabled, |ui| {
                        icons::button(ui, icon, text(label), false, false)
                    })
                    .inner
                    .clicked();
                if clicked && let Some(target) = target {
                    command = Some(Command::SetHistoryMarker {
                        part: part.id,
                        marker: (target < count).then_some(target),
                    });
                }
            }
        });
        ui.separator();
        let failure = self
            .pending_failure
            .as_ref()
            .filter(|f| f.tab == tab && f.failure.part == part.id)
            .map(|f| (f.failure.output, f.failure.dependents.clone()));
        let selected = self
            .timeline
            .selected
            .filter(|(t, p, _)| *t == tab && *p == part.id)
            .map(|(_, _, output)| output);
        let drag = self.timeline.drag;
        let mut started = None;
        let mut tiles = Vec::with_capacity(count);
        let mut gaps = Vec::with_capacity(count + 1);
        egui::ScrollArea::horizontal()
            .id_salt("feature_timeline")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.;
                    for index in 0..=count {
                        if index == marker {
                            let (rect, response) =
                                ui.allocate_exact_size(egui::vec2(9., TILE), egui::Sense::drag());
                            let response = response.on_hover_text(text(T::HistoryMarker));
                            let color = if response.hovered() || drag == Some(Drag::Marker) {
                                theme::ACCENT
                            } else {
                                egui::Color32::from_rgb(77, 141, 201)
                            };
                            ui.painter().rect_filled(
                                egui::Rect::from_center_size(rect.center(), egui::vec2(3., TILE)),
                                1,
                                color,
                            );
                            // Alça superior do marcador, como nos CADs de mercado.
                            ui.painter().add(egui::Shape::convex_polygon(
                                vec![
                                    rect.center_top(),
                                    rect.center_top() + egui::vec2(4.5, 0.),
                                    rect.center_top() + egui::vec2(4.5, 5.),
                                    rect.center_top() + egui::vec2(0., 8.),
                                    rect.center_top() + egui::vec2(-4.5, 5.),
                                    rect.center_top() + egui::vec2(-4.5, 0.),
                                ],
                                color,
                                egui::Stroke::NONE,
                            ));
                            if editable && response.drag_started() {
                                started = Some(Drag::Marker);
                            }
                        }
                        let Some((feature, state)) =
                            part.features.get(index).zip(states.get(index))
                        else {
                            continue;
                        };
                        gaps.push(ui.cursor().left());
                        let (rect, response) = ui.allocate_exact_size(
                            egui::Vec2::splat(TILE),
                            egui::Sense::click_and_drag(),
                        );
                        tiles.push(rect);
                        let look = Look {
                            state: *state,
                            selected: selected == Some(feature.output),
                            failed: failure.as_ref().is_some_and(|(f, _)| *f == feature.output),
                            affected: failure
                                .as_ref()
                                .is_some_and(|(_, d)| d.contains(&feature.output)),
                        };
                        paint_tile(ui, &response, &feature.operation, look);
                        let mut hint = format!("{} · {}", feature.name, text(state_label(*state)));
                        if look.failed {
                            hint = format!("{} · {}", feature.name, text(T::StateFailed));
                        } else if look.affected {
                            hint = format!("{} · {}", feature.name, text(T::StateAffected));
                        }
                        let response = response
                            .on_hover_text(format!("{hint}\n{}", text(T::TimelineDragHint)));
                        if response.clicked() {
                            self.timeline.selected = Some((tab, part.id, feature.output));
                        }
                        if response.double_clicked() && editable {
                            edit = Some(feature.clone());
                        }
                        if editable && response.drag_started() {
                            started = Some(Drag::Feature(feature.output));
                        }
                        response.context_menu(|ui| {
                            ui.add_enabled_ui(editable, |ui| {
                                if ui.button(text(T::EditFeature)).clicked() {
                                    edit = Some(feature.clone());
                                    ui.close();
                                }
                                if let Some(id) = feature.operation.sketch()
                                    && ui.button(text(T::EditProfileSketch)).clicked()
                                {
                                    sketch = Some(id);
                                    ui.close();
                                }
                                if ui.button(text(T::RenameFeature)).clicked() {
                                    self.timeline.rename = Some(Rename {
                                        tab,
                                        part: part.id,
                                        output: feature.output,
                                        name: feature.name.clone(),
                                        focus: true,
                                    });
                                    ui.close();
                                }
                                let label = if feature.suppressed {
                                    T::UnsuppressFeature
                                } else {
                                    T::SuppressFeature
                                };
                                if ui.button(text(label)).clicked() {
                                    command = Some(Command::SetFeatureSuppressed {
                                        part: part.id,
                                        output: feature.output,
                                        suppressed: !feature.suppressed,
                                    });
                                    ui.close();
                                }
                                if ui
                                    .add_enabled(
                                        index + 1 != marker,
                                        egui::Button::new(text(T::MarkerHere)),
                                    )
                                    .clicked()
                                {
                                    command = Some(Command::SetHistoryMarker {
                                        part: part.id,
                                        marker: (index + 1 < count).then_some(index + 1),
                                    });
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button(text(T::DeleteFeature)).clicked() {
                                    delete = Some(feature.output);
                                    ui.close();
                                }
                            });
                        });
                    }
                    gaps.push(ui.cursor().left());
                });
            });
        self.timeline.tiles = part
            .features
            .iter()
            .map(|f| f.output)
            .zip(tiles.iter().copied())
            .collect();
        if let Some(start) = started {
            self.timeline.drag = Some(start);
        }
        // Indicador da posição de soltura durante o arraste e aplicação ao soltar.
        if let Some(drag) = self.timeline.drag {
            let pointer = ui.input(|i| i.pointer.interact_pos());
            if let Some(position) = pointer {
                let target = slot(&tiles, position.x);
                if let Some(x) = gaps.get(target) {
                    let top = tiles.first().map(|r| r.top()).unwrap_or(position.y);
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(x - 2.5, top - 2.),
                            egui::vec2(3., TILE + 4.),
                        ),
                        1,
                        theme::ACCENT,
                    );
                }
            }
            if ui.input(|i| i.pointer.any_released()) {
                self.timeline.drag = None;
                if let Some(position) = pointer {
                    let target = slot(&tiles, position.x);
                    command = match drag {
                        Drag::Marker if target != marker => Some(Command::SetHistoryMarker {
                            part: part.id,
                            marker: (target < count).then_some(target),
                        }),
                        Drag::Feature(output) => part
                            .features
                            .iter()
                            .position(|f| f.output == output)
                            .and_then(|from| {
                                // Soltar à direita conta a própria operação, que sai da lista.
                                let index = if target > from { target - 1 } else { target };
                                (index != from).then_some(Command::MoveFeature {
                                    part: part.id,
                                    output,
                                    index,
                                })
                            }),
                        Drag::Marker => None,
                    };
                }
            }
        }
        // Delete exclui a operação destacada quando nenhum campo de texto tem o foco.
        if editable
            && let Some(output) = selected
            && !ui.ctx().egui_wants_keyboard_input()
            && ui.input(|i| i.key_pressed(egui::Key::Delete))
        {
            delete = Some(output);
        }
        if let Some(output) = delete {
            self.request_feature_removal(ui.ctx(), tab, part.id, output);
        }
        if let Some(feature) = edit {
            self.feature_edit = Some(feature_editor::Draft::edit(tab, part.id, feature));
        }
        if let Some(id) = sketch {
            self.begin_sketch(part.id, id);
        }
        if let Some(command) = command {
            self.prepare_command(ui.ctx(), tab, command);
        }
    }

    /// Exclui de imediato quando nada depende do resultado; caso contrário, pede confirmação
    /// listando as dependentes.
    fn request_feature_removal(
        &mut self,
        context: &egui::Context,
        tab: u64,
        part: PartId,
        output: BodyId,
    ) {
        let Some(piece) = self
            .tabs
            .iter()
            .find(|t| t.id == tab)
            .and_then(|t| t.session.document().part(part))
        else {
            return;
        };
        if nator_document::features::dependents(piece, output).is_empty() {
            self.prepare_command(
                context,
                tab,
                Command::RemoveFeature {
                    part,
                    output,
                    dependents: false,
                },
            );
        } else {
            self.timeline.delete = Some((tab, part, output));
        }
    }

    /// Renomeação, confirmação de exclusão e decisão sobre uma falha de recálculo.
    pub(super) fn timeline_dialogs(&mut self, context: &egui::Context) {
        self.rename_dialog(context);
        self.delete_dialog(context);
        self.failure_dialog(context);
    }

    fn rename_dialog(&mut self, context: &egui::Context) {
        let Some(rename) = &mut self.timeline.rename else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        egui::Window::new(text(T::RenameFeatureTitle))
            .id(egui::Id::new("rename_feature"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_BOTTOM, [0., -110.])
            .open(&mut open)
            .show(context, |ui| {
                let field = ui.text_edit_singleline(&mut rename.name);
                if rename.focus {
                    field.request_focus();
                    rename.focus = false;
                }
                if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    apply = true;
                }
                ui.horizontal(|ui| {
                    if ui.button(text(T::Ok)).clicked() {
                        apply = true;
                    }
                    if ui.button(text(T::Cancel)).clicked() {
                        cancel = true;
                    }
                });
            });
        if apply && let Some(rename) = self.timeline.rename.take() {
            self.prepare_command(
                context,
                rename.tab,
                Command::RenameFeature {
                    part: rename.part,
                    output: rename.output,
                    name: rename.name.trim().into(),
                },
            );
        } else if !open || cancel {
            self.timeline.rename = None;
        }
    }

    fn delete_dialog(&mut self, context: &egui::Context) {
        let Some((tab, part, output)) = self.timeline.delete else {
            return;
        };
        let Some(piece) = self
            .tabs
            .iter()
            .find(|t| t.id == tab)
            .and_then(|t| t.session.document().part(part))
            .cloned()
        else {
            self.timeline.delete = None;
            return;
        };
        let dependents = nator_document::features::dependents(&piece, output);
        // Prévia do resultado da exclusão atrás do diálogo, como pede o escopo.
        self.preview_command(
            tab,
            Some(Command::RemoveFeature {
                part,
                output,
                dependents: true,
            }),
            None,
            None,
        );
        let name = |id: BodyId| {
            piece
                .features
                .iter()
                .find(|f| f.output == id)
                .map(|f| f.name.clone())
                .unwrap_or_default()
        };
        let mut decision = None;
        egui::Modal::new(egui::Id::new("delete_feature")).show(context, |ui| {
            ui.set_max_width(380.);
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(22.), egui::Sense::hover());
                icons::paint(ui.painter(), rect, Icon::Delete, true);
                ui.heading(format!(
                    "{} “{}”",
                    text(T::DeleteFeatureTitle),
                    name(output)
                ));
            });
            ui.label(text(T::DeleteDependents));
            for id in &dependents {
                ui.label(format!("• {}", name(*id)));
            }
            ui.add_space(6.);
            ui.add_enabled_ui(!self.busy(), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(egui::RichText::new(text(T::DeleteAll)).color(theme::DANGER))
                        .clicked()
                    {
                        decision = Some(true);
                    }
                    if ui.button(text(T::Cancel)).clicked() {
                        decision = Some(false);
                    }
                });
            });
        });
        match decision {
            Some(true) => {
                self.timeline.delete = None;
                self.prepare_command(
                    context,
                    tab,
                    Command::RemoveFeature {
                        part,
                        output,
                        dependents: true,
                    },
                );
            }
            Some(false) => self.timeline.delete = None,
            None => {}
        }
    }

    fn failure_dialog(&mut self, context: &egui::Context) {
        let Some(pending) = self.pending_failure.as_ref().filter(|p| !p.fixing) else {
            return;
        };
        let tab = pending.tab;
        let failure = pending.failure.clone();
        let Some(piece) = self
            .tabs
            .iter()
            .find(|t| t.id == tab)
            .and_then(|t| t.session.document().part(failure.part))
            .cloned()
        else {
            self.pending_failure = None;
            return;
        };
        let names = failure
            .dependents
            .iter()
            .filter_map(|id| piece.features.iter().find(|f| f.output == *id))
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let mut decision = None;
        egui::Modal::new(egui::Id::new("rebuild_failure")).show(context, |ui| {
            ui.set_max_width(440.);
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(24.), egui::Sense::hover());
                icons::paint(ui.painter(), rect, Icon::Warning, true);
                ui.heading(text(T::RebuildFailedTitle));
            });
            ui.label(egui::RichText::new(&failure.name).strong());
            ui.colored_label(theme::DANGER, &failure.message);
            if !names.is_empty() {
                ui.label(format!("{} {names}", text(T::RebuildFailedAffected)));
            }
            ui.small(text(T::PendingEditBanner));
            ui.add_space(6.);
            ui.add_enabled_ui(!self.busy(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(text(T::FixFeature)).clicked() {
                        decision = Some(0);
                    }
                    if ui.button(text(T::SuppressAndApply)).clicked() {
                        decision = Some(1);
                    }
                    if ui.button(text(T::CancelEdit)).clicked() {
                        decision = Some(2);
                    }
                });
            });
        });
        match decision {
            Some(0) => {
                if let Some(feature) = piece.features.iter().find(|f| f.output == failure.output) {
                    self.feature_edit = Some(feature_editor::Draft::edit(
                        tab,
                        failure.part,
                        feature.clone(),
                    ));
                    if let Some(pending) = &mut self.pending_failure {
                        pending.fixing = true;
                    }
                }
            }
            Some(1) => {
                if let Some(pending) = &self.pending_failure {
                    let command = Command::Batch {
                        label: "Editar e suprimir operação",
                        commands: vec![
                            pending.command.clone(),
                            Command::SetFeatureSuppressed {
                                part: failure.part,
                                output: failure.output,
                                suppressed: true,
                            },
                        ],
                    };
                    self.resolving = Some(tab);
                    self.prepare_command(context, tab, command);
                }
            }
            Some(2) => self.pending_failure = None,
            _ => {}
        }
    }

    /// Comando que corrige a operação em edição durante uma falha pendente: a edição
    /// rejeitada entra no mesmo lote, avaliado de uma só vez.
    pub(super) fn with_pending_fix(
        &mut self,
        tab: u64,
        output: BodyId,
        command: Command,
    ) -> Command {
        match self
            .pending_failure
            .as_ref()
            .filter(|p| p.tab == tab && p.fixing && p.failure.output == output)
        {
            Some(pending) => {
                self.resolving = Some(tab);
                Command::Batch {
                    label: "Editar e corrigir operação",
                    commands: vec![pending.command.clone(), command],
                }
            }
            None => command,
        }
    }

    /// Resultado de uma resolução bem-sucedida: encerra a falha pendente e aceita a edição
    /// de rascunho que a provocou.
    pub(super) fn resolve_pending_failure(&mut self, tab: u64) {
        if self.resolving != Some(tab) {
            return;
        }
        self.resolving = None;
        let Some(pending) = self.pending_failure.take_if(|p| p.tab == tab) else {
            return;
        };
        if let Some(id) = pending.sketch
            && let Some(target) = self.find_tab(tab)
            && let Some(work) = target.sketch_edit.as_mut().filter(|w| w.id == id)
        {
            work.pending = None;
        }
    }
}
