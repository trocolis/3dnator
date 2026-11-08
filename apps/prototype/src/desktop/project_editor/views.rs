//! Composição visual. Alterações do documento são encaminhadas ao editor e aos comandos.
use super::*;

/// Largura da árvore sobre a área de edição.
const TREE_WIDTH: f32 = 270.;
/// Largura da barra de título da árvore recolhida.
const COLLAPSED_TREE_WIDTH: f32 = 200.;
/// Ícone, botão e espaçamentos que dividem o balão com o nome da peça ou do rascunho.
const BALLOON_CONTROLS_WIDTH: f32 = 96.;
/// Largura reservada ao balão dos graus de liberdade.
const FREEDOM_BALLOON_WIDTH: f32 = 190.;

/// Balão flutuante no topo da área de edição, com o estilo da barra da árvore recolhida.
/// Devolve o retângulo ocupado e o valor do conteúdo.
fn balloon<R>(
    context: &egui::Context,
    id: &str,
    at: egui::Pos2,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Rect, R) {
    let shown = egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Middle)
        .fade_in(false)
        .constrain(false)
        .fixed_pos(at)
        .show(context, |ui| {
            theme::floating_frame()
                .show(ui, |ui| {
                    egui::Frame::new()
                        .fill(theme::subtle())
                        .inner_margin(egui::Margin::symmetric(12, 6))
                        .corner_radius(8)
                        .show(ui, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                            ui.horizontal(|ui| {
                                // Altura do botão compacto: balões com e sem botão ficam iguais.
                                ui.set_min_height(30.);
                                content(ui)
                            })
                            .inner
                        })
                        .inner
                })
                .inner
        });
    (shown.response.rect, shown.inner)
}

/// Ícone, nome abreviado se preciso e um botão de ação; devolve o clique no botão.
fn named_balloon(
    ui: &mut egui::Ui,
    icon: icons::Icon,
    name: &str,
    name_width: f32,
    action: icons::Icon,
    hint: &str,
    enabled: bool,
) -> bool {
    ui.spacing_mut().item_spacing.x = 6.;
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(22.), egui::Sense::hover());
    icons::paint(ui.painter(), rect, icon, true);
    ui.allocate_ui_with_layout(
        egui::vec2(name_width, 22.),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(name)
                        .strong()
                        .size(13.)
                        .color(theme::accent()),
                )
                .truncate(),
            )
            .on_hover_text(name);
        },
    );
    ui.add_enabled_ui(enabled, |ui| icons::button(ui, action, hint, false, false))
        .inner
        .clicked()
}

/// Barra de título da árvore: mostra o nome do projeto na dica e devolve o clique no botão
/// de recolher ou mostrar.
fn tree_title(ui: &mut egui::Ui, project: &str, expanded: bool, corner: u8) -> bool {
    let mut toggled = false;
    egui::Frame::new()
        .fill(theme::subtle())
        .inner_margin(egui::Margin::symmetric(12, 6))
        .corner_radius(corner)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(22.), egui::Sense::hover());
                icons::paint(ui.painter(), rect, icons::Icon::Cube, true);
                ui.label(
                    egui::RichText::new(text(T::ProjectNode).to_uppercase())
                        .strong()
                        .size(13.)
                        .color(theme::ink()),
                )
                .on_hover_text(project);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    toggled = icons::button(
                        ui,
                        if expanded {
                            icons::Icon::CollapseUp
                        } else {
                            icons::Icon::ExpandDown
                        },
                        text(if expanded {
                            T::CollapseTree
                        } else {
                            T::ExpandTree
                        }),
                        false,
                        false,
                    )
                    .clicked();
                });
            });
        });
    toggled
}

impl Editor {
    /// Miniatura de um projeto em disco, decodificada uma vez por caminho.
    fn thumbnail(
        &mut self,
        context: &egui::Context,
        path: &std::path::Path,
    ) -> Option<egui::TextureHandle> {
        if let Some(cached) = self.thumbnails.get(path) {
            return cached.clone();
        }
        let texture = nator_project_io::read_thumbnail(path)
            .ok()
            .flatten()
            .and_then(|bytes| {
                let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
                let mut reader = decoder.read_info().ok()?;
                let mut pixels = vec![0; reader.output_buffer_size()?];
                let info = reader.next_frame(&mut pixels).ok()?;
                (info.color_type == png::ColorType::Rgb).then(|| {
                    egui::ColorImage::from_rgb(
                        [info.width as usize, info.height as usize],
                        &pixels[..info.buffer_size()],
                    )
                })
            })
            .map(|image| {
                context.load_texture(
                    format!("thumbnail:{}", path.display()),
                    image,
                    egui::TextureOptions::LINEAR,
                )
            });
        self.thumbnails.insert(path.to_path_buf(), texture.clone());
        texture
    }
    pub(super) fn settings(&self) -> nator_document::Settings {
        self.current().session.document().settings()
    }
    /// Número com as casas decimais configuradas, sem conversão de unidade.
    pub(super) fn number(&self, value: f64) -> String {
        nator_drawing::format_fixed(value, usize::from(self.settings().display_decimals))
    }
    /// Medida em milímetros convertida para a unidade exibida.
    pub(super) fn mm(&self, value: f64) -> String {
        self.settings().format_length(value)
    }
    /// Medida com o símbolo da unidade exibida.
    pub(super) fn length(&self, value: f64) -> String {
        format!("{} {}", self.mm(value), self.settings().unit.suffix())
    }
    pub(super) fn shortcuts(&mut self, context: &egui::Context) {
        if self.shell.search_open {
            if context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.shell.search_open = false;
            }
            return;
        }
        if self.busy() || self.pending_close.is_some() || self.status.has_pending() {
            return;
        }
        // Enquanto a aba Atalhos espera uma tecla nova, nenhuma ação dispara.
        if self.settings_panel.capturing() {
            return;
        }
        let save = shortcuts::effective(&self.preferences.shortcuts, &shortcuts::bindings()[0]);
        if context.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(save.0, save.1))) {
            self.save(context, self.current().id, false, false);
            return;
        }
        if self.current().plane_pick.is_some()
            && context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.escape_to_selection(context);
            return;
        }
        if self.image_edit.is_some() {
            if context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.image_edit = None;
            }
            if !context.egui_wants_keyboard_input() {
                self.history_shortcuts(context);
            }
            return;
        }
        if self
            .feature_edit
            .as_ref()
            .is_some_and(|d| d.arrow_input_active())
            || self.offset_input_active()
            || self.section_input_active()
            || self.move_input_active()
            || self
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.arrow_input_active())
            || context.egui_wants_keyboard_input()
            || self
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.is_drawing())
        {
            return;
        }
        if context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.escape_to_selection(context);
            return;
        }
        let scope = if self.current().sketch_edit.is_some() {
            shortcuts::Scope::Sketch
        } else if self.current().active_part.is_some() {
            shortcuts::Scope::Model
        } else {
            shortcuts::Scope::Assembly
        };
        let chosen = self.preferences.shortcuts.clone();
        let target = shortcuts::pressed(context, &chosen, scope).or_else(|| {
            (scope != shortcuts::Scope::Sketch)
                .then(|| shortcuts::pressed(context, &chosen, shortcuts::Scope::Workspace))
                .flatten()
        });
        if let Some(target) = target {
            self.run_shortcut(context, target);
            return;
        }
        self.history_shortcuts(context);
    }
    fn history_shortcuts(&mut self, context: &egui::Context) {
        let all = shortcuts::bindings();
        let key = |id: &str| {
            all.iter()
                .find(|b| b.id == id)
                .map(|b| shortcuts::effective(&self.preferences.shortcuts, b))
        };
        let (undo_key, redo_key) = (key("undo"), key("redo"));
        // Ctrl+Shift+Z continua como alternativa fixa de Refazer.
        let redo = context.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                egui::Key::Z,
            )) || redo_key
                .is_some_and(|(m, k)| i.consume_shortcut(&egui::KeyboardShortcut::new(m, k)))
        });
        let undo = context.input_mut(|i| {
            undo_key.is_some_and(|(m, k)| i.consume_shortcut(&egui::KeyboardShortcut::new(m, k)))
        });
        if redo || undo {
            self.step_history(context, redo);
        }
    }
    /// Árvore sobre o canvas, sem painel de fundo nem reserva de coluna lateral.
    pub(super) fn tree(&mut self, context: &egui::Context) {
        let menu = self.menu_context();
        let busy = self.busy();
        let mut intents = Vec::new();
        let canvas = self.canvas_rect;
        let shown = egui::Area::new(egui::Id::new("floating_tree"))
            .order(egui::Order::Middle)
            .fade_in(false)
            .movable(false)
            .constrain(false)
            .fixed_pos(canvas.left_top() + egui::vec2(8., 8.))
            .show(context, |ui| {
                ui.set_width(TREE_WIDTH);
                // A área não herda a altura inicial de janela flutuante do egui.
                // Cabeçalho e margens são descontados pelo layout antes da rolagem.
                ui.set_height((canvas.height() - 16.).max(0.));
                ui.spacing_mut().item_spacing.y = 0.;
                let tab = self.current();
                let document = tab.session.document();
                let collapse = theme::floating_frame()
                    .show(ui, |ui| tree_title(ui, document.name(), true, 8))
                    .inner;
                // Apenas o cabeçalho tem fundo: os nós ficam diretamente sobre a cena.
                egui::Frame::new().inner_margin(12).show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("project_browser")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut tree = project_tree::Tree::new(
                                document,
                                tab,
                                &menu,
                                busy,
                                self.section_edit.is_some(),
                            );
                            tree.show(ui);
                            intents.append(&mut tree.intents);
                        });
                });
                for intent in intents {
                    self.apply_tree_intent(ui, intent);
                }
                collapse
            });
        if shown.inner {
            self.shell.browser_visible = false;
        }
    }
    /// Árvore recolhida: resta a barra de título, no canto superior da área de edição.
    pub(super) fn collapsed_tree(&mut self, context: &egui::Context) {
        if self.shell.browser_visible {
            return;
        }
        let name = self.current().session.document().name().to_owned();
        let expand = egui::Area::new(egui::Id::new("collapsed_tree"))
            .order(egui::Order::Middle)
            .fade_in(false)
            .constrain(false)
            .fixed_pos(self.canvas_rect.left_top() + egui::vec2(8., 8.))
            .show(context, |ui| {
                theme::floating_frame()
                    .show(ui, |ui| {
                        ui.set_width(COLLAPSED_TREE_WIDTH);
                        tree_title(ui, &name, false, 8)
                    })
                    .inner
            })
            .inner;
        if expand {
            self.shell.browser_visible = true;
        }
    }
    /// Balões de edição numa fila no canto, junto da árvore: a peça em edição (Salvar
    /// conclui e volta à montagem), o rascunho em edição com Concluir e os graus de
    /// liberdade dele. Todos têm o estilo da árvore recolhida e não quebram linha.
    pub(super) fn edit_balloons(&mut self, context: &egui::Context) {
        let tab = self.current();
        // Isolamento: o nome do componente (ou a contagem) com o botão de sair, para que a
        // cena reduzida nunca pareça um projeto com peças sumidas.
        let isolation = tab.view.isolated.as_ref().map(|set| {
            let document = tab.session.document();
            let name = match set.iter().next() {
                Some(id) if set.len() == 1 => document
                    .instance(*id)
                    .map(|i| i.name.clone())
                    .unwrap_or_default(),
                _ => format!("{} {}", set.len(), text(T::IsolatedMany)),
            };
            format!("{}: {name}", text(T::IsolatedBadge))
        });
        let part = tab
            .active_part
            .and_then(|part| tab.session.document().part(part))
            .map(|part| part.name.clone());
        let sketch = tab.sketch_edit.as_ref().map(|work| {
            let name = tab
                .session
                .document()
                .sketch(work.id)
                .map(|record| record.sketch.name.clone())
                .unwrap_or_default();
            (name, work.pending.is_none(), work.freedom())
        });
        let canvas = self.canvas_rect;
        let busy = self.busy() || self.pending_close.is_some();
        // A árvore ocupa o canto nos dois estados: a fila vem depois do cabeçalho.
        let mut left = canvas.left()
            + if self.shell.browser_visible {
                TREE_WIDTH + 18.
            } else {
                COLLAPSED_TREE_WIDTH + 18.
            };
        // Os nomes dividem o espaço até o cubo; nomes longos ficam abreviados.
        let room = canvas.right() - view_cube::rect(canvas).width() - 16. - left;
        let names = usize::from(part.is_some())
            + usize::from(sketch.is_some())
            + usize::from(isolation.is_some());
        let name_width = if names == 0 {
            0.
        } else {
            ((room - FREEDOM_BALLOON_WIDTH - names as f32 * BALLOON_CONTROLS_WIDTH) / names as f32)
                .clamp(60., 240.)
        };
        let top = canvas.top() + 8.;
        if let Some(name) = isolation {
            let (rect, exit) = balloon(context, "isolation_balloon", egui::pos2(left, top), |ui| {
                named_balloon(
                    ui,
                    icons::Icon::Eye,
                    &name,
                    name_width,
                    icons::Icon::Close,
                    text(T::ExitIsolation),
                    true,
                )
            });
            left = rect.right() + 8.;
            if exit {
                self.apply_menu_action(context, browser::MenuAction::ExitIsolation);
                context.request_repaint();
            }
        }
        if let Some(name) = part {
            let (rect, save) = balloon(context, "edit_balloon", egui::pos2(left, top), |ui| {
                named_balloon(
                    ui,
                    icons::Icon::Edit,
                    &name,
                    name_width,
                    icons::Icon::Save,
                    text(T::StopEditPart),
                    !busy,
                )
            });
            left = rect.right() + 8.;
            if save {
                self.apply_menu_action(context, browser::MenuAction::Deactivate);
                context.request_repaint();
            }
        }
        let Some((name, settled, freedom)) = sketch else {
            return;
        };
        let (rect, finish) = balloon(context, "sketch_balloon", egui::pos2(left, top), |ui| {
            named_balloon(
                ui,
                icons::Icon::Sketch,
                &name,
                name_width,
                icons::Icon::Check,
                text(T::FinishSketch),
                !busy && settled && self.sketch_text.is_none(),
            )
        });
        left = rect.right() + 8.;
        if finish {
            self.finish_sketch();
            context.request_repaint();
        }
        // O cálculo do trabalhador vale para o rascunho como está, inclusive ao abrir.
        let Some(status) = freedom else {
            return;
        };
        let (label, color) = match status {
            nator_sketch::solver::Status::Conflict => {
                (text(T::SketchConflict).to_owned(), theme::danger())
            }
            nator_sketch::solver::Status::FullyConstrained => {
                (text(T::FullyConstrained).to_owned(), theme::green())
            }
            nator_sketch::solver::Status::UnderConstrained { degrees_of_freedom } => (
                format!("{}: {degrees_of_freedom}", text(T::DegreesOfFreedom)),
                theme::accent(),
            ),
        };
        balloon(context, "freedom_balloon", egui::pos2(left, top), |ui| {
            ui.label(egui::RichText::new(label).strong().size(13.).color(color));
        });
    }
    fn apply_tree_intent(&mut self, ui: &egui::Ui, intent: project_tree::Intent) {
        use project_tree::Intent;
        match intent {
            Intent::Select(selection) => {
                // Ctrl+clique alterna o elemento, como na área 3D.
                if ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                    self.current_mut().toggle(selection);
                } else {
                    self.current_mut().select(Some(selection));
                }
            }
            Intent::Command(command) => self.execute(*command),
            Intent::Menu(action) => self.apply_menu_action(ui.ctx(), action),
            Intent::EditPart(part, node) => {
                let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    node,
                    true,
                );
                state.set_open(true);
                state.store(ui.ctx());
                self.apply_menu_action(ui.ctx(), browser::MenuAction::Activate(part));
            }
            Intent::LeavePart => self.apply_menu_action(ui.ctx(), browser::MenuAction::Deactivate),
            Intent::EditSketch(part, id) => self.begin_sketch(part, id),
            Intent::PickWorkPlane(id) => self.choose_work_plane(id),
            Intent::EditWorkPlane(id) => self.edit_work_plane(id),
            Intent::MoveToReferences {
                source,
                destination,
            } => {
                if let Some(reason) = self.convert_reference_blocker() {
                    self.status.warning(text(reason));
                    return;
                }
                let Some(&instance) = source.last() else {
                    return;
                };
                match browser::move_to_references(
                    self.current().session.document(),
                    &source,
                    destination.as_deref(),
                ) {
                    Ok(Some(command)) => {
                        let tab = self.current().id;
                        self.current_mut()
                            .select(Some(Selection::Instance(instance)));
                        self.prepare_command(ui.ctx(), tab, command);
                    }
                    Ok(None) => {}
                    Err(error) => self.status.error(error.to_string()),
                }
            }
            Intent::EditReference(id) => {
                self.finish_sketch();
                self.current_mut().active_part = None;
                self.begin_reference_sketch(id);
            }
            Intent::DeleteSketch(id) => {
                self.execute(Command::RemoveSketch { id });
                if self.current().session.document().sketch(id).is_none()
                    && self
                        .current()
                        .sketch_edit
                        .as_ref()
                        .is_some_and(|w| w.id == id)
                {
                    if let Some(work) = self.current_mut().sketch_edit.as_mut() {
                        work.pending = None;
                    }
                    self.finish_sketch();
                }
            }
            Intent::NewInFolder(folder) => {
                use nator_document::Folder;
                match folder {
                    Folder::Sketches(part) => {
                        self.current_mut().active_part = Some(part);
                        self.start_sketch_plane();
                    }
                    Folder::ReferenceSketches => self.start_reference_sketch_plane(),
                    Folder::WorkPlanes(part) => {
                        self.current_mut().active_part = Some(part);
                        self.open_offset_plane();
                    }
                    Folder::Bodies(part) => {
                        self.current_mut().active_part = Some(part);
                        self.open_feature_tool(feature_editor::Tool::Extrude);
                    }
                    Folder::Parts | Folder::Subparts(_) => {
                        let parent = if let Folder::Subparts(parent) = folder {
                            Some(parent)
                        } else {
                            None
                        };
                        self.execute(Command::CreatePartIn {
                            name: format!(
                                "{} {}",
                                text(T::NewPartName),
                                self.current()
                                    .session
                                    .document()
                                    .next_name_number(nator_document::ElementKind::Part)
                            ),
                            parent,
                        });
                    }
                    _ => {}
                }
            }
            Intent::EditImage(id) => self.edit_reference_image(id),
            Intent::NewImage(owner) => {
                let current = self
                    .current()
                    .sketch_edit
                    .as_ref()
                    .map(|w| w.id)
                    .filter(|id| self.current().session.document().sketch_part(*id) == owner);
                let sketch = current
                    .or_else(|| {
                        owner.and_then(|part| {
                            self.current()
                                .session
                                .document()
                                .part(part)
                                .and_then(|p| p.sketches.first())
                                .map(|s| s.id)
                        })
                    })
                    .or_else(|| {
                        if owner.is_none() {
                            self.current()
                                .session
                                .document()
                                .reference_sketches()
                                .first()
                                .map(|s| s.id)
                        } else {
                            None
                        }
                    });
                if let Some(sketch) = sketch {
                    if let Some(part) = owner {
                        self.begin_sketch(part, sketch);
                    } else {
                        self.begin_reference_sketch(sketch);
                    }
                } else if let Some(part) = owner {
                    self.execute(Command::AddSketch {
                        destination: Some(part),
                        sketch: nator_sketch::Sketch::new(
                            text(T::Sketch),
                            nator_sketch::Plane::xy(),
                        ),
                    });
                } else {
                    self.execute(Command::AddReferenceSketch {
                        sketch: nator_sketch::Sketch::new(
                            text(T::ReferenceSketch),
                            nator_sketch::Plane::xy(),
                        ),
                    });
                }
                self.load_reference_image(ui.ctx());
            }
            Intent::ToggleSection => {
                if let Some(section) = &mut self.current_mut().section {
                    section.visible = !section.visible;
                }
            }
            Intent::OpenSection => self.open_section(),
            Intent::RemoveSection => {
                self.current_mut().section = None;
                self.section_edit = None;
            }
            Intent::OpenDrawing(sheet) => self.open_drawing(Some(*sheet)),
        }
    }
    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::canvas()).inner_margin(8))
            .show(ui, |ui| {
                self.animate_camera(ui.ctx());
                let size = ui.available_size().max(egui::vec2(1., 1.));
                let pixels = ui.ctx().pixels_per_point();
                if self.viewport.resize(
                    &self.gpu.device,
                    [(size.x * pixels) as u32, (size.y * pixels) as u32],
                ) {
                    self.gpu
                        .renderer
                        .write()
                        .update_egui_texture_from_wgpu_texture(
                            &self.gpu.device,
                            self.viewport.view(),
                            wgpu::FilterMode::Linear,
                            self.texture,
                        );
                }
                if self
                    .current()
                    .sketch_edit
                    .as_ref()
                    .is_some_and(|w| w.fit_requested)
                {
                    self.align_sketch_camera(true, false);
                    if let Some(work) = self.current_mut().sketch_edit.as_mut() {
                        work.fit_requested = false;
                    }
                }
                let response = ui.add(
                    egui::Image::from_texture((self.texture, size))
                        .sense(egui::Sense::click_and_drag()),
                );
                response.clone().on_hover_text(text(T::Navigation));
                self.canvas_rect = response.rect;
                // Com o campo Perfil ativo, o clique escolhe a região do rascunho; janela e
                // pressão longa são gestos de seleção que substituem o clique simples.
                // O manipulador e os painéis de montagem têm prioridade sobre a seleção comum.
                self.viewport.section = self.current().section_plane();
                let origin_chosen = self.origin_overlay(ui, &response);
                let gizmo = origin_chosen
                    || self.paint_interaction(ui, &response)
                    || self.reference_image_model_gesture(ui, &response)
                    || self.label_gizmo(ui, &response)
                    || self.extrusion_gizmo(ui, &response)
                    || self.feature_measure_gizmo(ui, &response)
                    || self.section_gizmo(ui, &response)
                    || self.move_gizmo(ui, &response)
                    || self.offset_plane_gizmo(ui, &response);
                let assembly = !gizmo && self.assembly_pick(&response);
                self.profile_hovered = false;
                let profile = !gizmo
                    && (self.revolve_axis_overlay(ui, &response)
                        || self.profile_overlay(ui, &response));
                let gesture = !gizmo && self.selection_gestures(ui, &response);
                if self.current().sketch_edit.is_none()
                    && self.current().plane_pick.is_none()
                    && !profile
                    && !gesture
                    && !gizmo
                    && !assembly
                    && self.assembly_panel.is_none()
                {
                    self.viewport_actions(ui, &response);
                }
                let cube_rect = view_cube::rect(response.rect);
                let starts_on_cube = ui
                    .input(|i| i.pointer.press_origin())
                    .is_some_and(|p| cube_rect.contains(p));
                if response.dragged_by(egui::PointerButton::Middle) && !starts_on_cube {
                    self.current_mut().camera_transition = None;
                    let (delta, shift) = ui.input(|i| (i.pointer.delta(), i.modifiers.shift));
                    if shift || self.shell.camera_mode == Some(view_modes::CameraMode::Orbit) {
                        self.viewport
                            .camera
                            .orbit([f64::from(delta.x), f64::from(delta.y)]);
                    } else {
                        self.viewport
                            .camera
                            .pan([f64::from(delta.x), f64::from(delta.y)], f64::from(size.y));
                    }
                }
                if response.hovered()
                    && !response.hover_pos().is_some_and(|p| cube_rect.contains(p))
                {
                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    if let Some(position) = response.hover_pos().filter(|_| scroll != 0.) {
                        self.current_mut().camera_transition = None;
                        self.viewport.camera.zoom_at(
                            f64::from(scroll),
                            [
                                f64::from((position.x - response.rect.center().x) / size.x * 2.),
                                f64::from((response.rect.center().y - position.y) / size.y * 2.),
                            ],
                            f64::from(size.x / size.y),
                        );
                    }
                }
                if ui.input(|i| i.pointer.any_down()) && starts_on_cube {
                    self.current_mut().camera_transition = None;
                }
                if view_cube::show(ui, &mut self.viewport.camera, response.rect) {
                    self.fit_view(ui.ctx());
                }
                let visible_planes = self.work_planes_overlay(ui, response.rect);
                if !origin_chosen
                    && self.current().plane_pick.is_some()
                    && response.clicked()
                    && !response.hover_pos().is_some_and(|p| cube_rect.contains(p))
                    && let Some(position) = response.hover_pos()
                {
                    let ray = self.viewport.camera.world_ray(
                        [
                            f64::from((position.x - response.rect.center().x) / size.x * 2.),
                            f64::from((response.rect.center().y - position.y) / size.y * 2.),
                        ],
                        self.display_bounds(),
                        f64::from(size.x / size.y),
                    );
                    self.choose_face_plane(ui.ctx(), ray);
                }
                let visible_sketches = self.visible_sketches(ui, response.rect);
                self.reference_images_in_model(ui, response.rect);
                self.sketch_overlay(ui, &response);
                self.sketch_repair_overlay(ui, &response);
                if self.scene.is_none()
                    && !visible_sketches
                    && !visible_planes
                    && !self.busy()
                    && self.current().sketch_edit.is_none()
                    && self.current().plane_pick.is_none()
                {
                    ui.painter().text(
                        response.rect.center_bottom() - egui::vec2(0., 64.),
                        egui::Align2::CENTER_BOTTOM,
                        text(T::EmptyScene),
                        egui::FontId::proportional(14.),
                        theme::muted(),
                    );
                }
                self.viewport.section = self.current().section_plane();
                self.update_hover(ui, &response);
                self.draw_highlights(ui, response.rect);
                self.viewport.render(&self.gpu.device, &self.gpu.queue);
                if self.viewport.section_pending() {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(16));
                }
                self.align_overlay(ui, response.rect);
                self.measure_overlay(ui, response.rect);
                if let Some(mode) = self.shell.camera_mode
                    && response.hovered()
                {
                    ui.ctx().set_cursor_icon(
                        mode.cursor(response.dragged_by(egui::PointerButton::Middle)),
                    );
                }
                if self.shell.measure_open
                    && self.shell.camera_mode.is_none()
                    && response.hovered()
                    && response
                        .hover_pos()
                        .is_some_and(|p| !view_cube::rect(response.rect).contains(p))
                {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                }
                self.tool_cursor(ui, &response);
                self.current_mut().camera = self.viewport.camera.clone();
            });
    }
    pub(super) fn dialogs(&mut self, context: &egui::Context) {
        if let Some(id) = self.pending_close {
            let name = self
                .tabs
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.session.document().name().to_owned())
                .unwrap_or_default();
            egui::Modal::new(egui::Id::new("close_project")).show(context, |ui| {
                ui.heading(text(T::Unsaved));
                ui.label(name);
                ui.label(text(T::SaveChanges));
                ui.add_enabled_ui(!self.busy(), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(text(T::Save)).clicked() {
                            self.pending_close = None;
                            self.save(context, id, false, true);
                        }
                        if ui.button(text(T::Discard)).clicked() {
                            self.close_tab(id);
                        }
                        if ui.button(text(T::Cancel)).clicked() {
                            self.pending_close = None;
                            self.want_exit = false;
                        }
                    });
                });
            });
        }
        if self.show_recovery {
            let mut open = self.show_recovery;
            let mut recover = None;
            let mut delete = None;
            egui::Window::new(text(T::Recovery))
                .open(&mut open)
                .show(context, |ui| {
                    ui.label(text(T::RecoveryNotice));
                    egui::ScrollArea::vertical()
                        .max_height(250.)
                        .show(ui, |ui| {
                            for path in self.recoveries.clone() {
                                let thumbnail = self.thumbnail(ui.ctx(), &path);
                                ui.horizontal(|ui| {
                                    // Miniatura gravada no projeto, para reconhecer a cópia.
                                    if let Some(texture) = thumbnail {
                                        ui.add(
                                            egui::Image::new((texture.id(), egui::vec2(96., 72.)))
                                                .corner_radius(4),
                                        );
                                    }
                                    ui.vertical(|ui| {
                                        ui.small(
                                            path.file_name().unwrap_or_default().to_string_lossy(),
                                        );
                                        ui.horizontal(|ui| {
                                            if ui
                                                .add_enabled(
                                                    !self.busy(),
                                                    egui::Button::new(text(T::Recover)),
                                                )
                                                .clicked()
                                            {
                                                recover = Some(path.clone());
                                            }
                                            // Excluir é definitivo: pede um segundo clique.
                                            if self.recovery_delete.as_ref() == Some(&path) {
                                                if ui
                                                    .button(
                                                        egui::RichText::new(text(
                                                            T::DeleteRecoveryConfirm,
                                                        ))
                                                        .color(theme::danger()),
                                                    )
                                                    .clicked()
                                                {
                                                    delete = Some(path.clone());
                                                }
                                                if ui.button(text(T::Cancel)).clicked() {
                                                    self.recovery_delete = None;
                                                }
                                            } else if ui.button(text(T::DeleteRecovery)).clicked() {
                                                self.recovery_delete = Some(path.clone());
                                            }
                                        });
                                    });
                                });
                            }
                        });
                });
            self.show_recovery = open;
            if let Some(path) = delete {
                self.delete_recovery(&path);
            }
            if let Some(path) = recover {
                self.show_recovery = false;
                self.open(context, Some(path));
            }
        }
    }
}

impl Editor {
    /// Exclui uma cópia de recuperação do painel. Só apaga arquivos marcados como
    /// recuperação (`remove_recovery` confere o manifesto); sem cópias, o painel fecha.
    pub(super) fn delete_recovery(&mut self, path: &std::path::Path) {
        self.recovery_delete = None;
        match nator_project_io::remove_recovery(path) {
            Ok(()) if !path.exists() => {
                self.recoveries.retain(|p| p != path);
                self.status.set(text(T::RecoveryDeleted));
            }
            Ok(()) => self.status.error(text(T::RecoveryDeleteFailed)),
            Err(error) => self
                .status
                .error(format!("{}: {error}", text(T::RecoveryDeleteFailed))),
        }
        if self.recoveries.is_empty() {
            self.show_recovery = false;
        }
    }
}
