//! Composição visual. Alterações do documento são encaminhadas ao editor e aos comandos.
use super::*;

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
    /// Valor com as casas decimais configuradas no documento ativo.
    pub(super) fn mm(&self, value: f64) -> String {
        nator_drawing::format_fixed(
            value,
            usize::from(
                self.current()
                    .session
                    .document()
                    .settings()
                    .display_decimals,
            ),
        )
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
        if context.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::S,
            ))
        }) {
            self.save(context, self.current().id, false, false);
            return;
        }
        if self.current().plane_pick.is_some()
            && context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.activate_selection(context);
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
        if context.egui_wants_keyboard_input()
            || self
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.is_drawing())
        {
            return;
        }
        if context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.activate_selection(context);
            return;
        }
        if self.current().sketch_edit.is_some() {
            use nator_sketch::edit::Action as Edit;
            use sketch_editor::Tool;
            if context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) {
                self.delete_sketch_selection(context);
                return;
            }
            if context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::X)) {
                self.toggle_sketch_stroke(context);
                return;
            }
            for (key, tool) in [
                (egui::Key::L, Tool::Line),
                (egui::Key::R, Tool::Rectangle),
                (egui::Key::C, Tool::Circle),
                (egui::Key::A, Tool::Arc),
                (egui::Key::P, Tool::Project),
                (egui::Key::G, Tool::Trim),
                (egui::Key::D, Tool::Dimension),
            ] {
                if context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                    self.select_sketch_tool(tool);
                    return;
                }
            }
            for (key, action) in [
                (egui::Key::S, Edit::Offset),
                (egui::Key::O, Edit::Fillet),
                (egui::Key::H, Edit::Chamfer),
                (egui::Key::E, Edit::Mirror),
            ] {
                if context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                    self.open_sketch_action(context, action);
                    return;
                }
            }
        }
        if context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::S)) {
            self.shell.search_open = true;
            self.shell.search_focus = true;
            return;
        }
        // D ativa a Cota no rascunho, como no SolidCraft; letras digitadas na caixa de
        // valor não chegam aqui, pois ela tem o teclado.
        if self.current().sketch_edit.is_some()
            && context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::D))
        {
            self.select_sketch_tool(sketch_editor::Tool::Dimension);
            return;
        }
        self.history_shortcuts(context);
    }
    fn history_shortcuts(&mut self, context: &egui::Context) {
        let redo = context.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                egui::Key::Z,
            )) || i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::Y,
            ))
        });
        let undo = context.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::Z,
            ))
        });
        let changed = if redo {
            self.current_mut().session.redo()
        } else if undo {
            self.current_mut().session.undo()
        } else {
            false
        };
        if changed {
            if let Some(work) = self.current_mut().sketch_edit.as_mut() {
                work.pending = None;
            }
            self.sanitize_sketch_selection();
            self.refresh_reference_image();
            self.current_mut().changed();
            self.invalidate_scene();
        }
    }
    pub(super) fn tree(&mut self, ui: &mut egui::Ui) {
        let menu = self.menu_context();
        let busy = self.busy();
        let mut intents = Vec::new();
        egui::Panel::left("tree")
            .default_size(270.)
            .min_size(190.)
            .max_size(420.)
            .frame(theme::panel_frame())
            .show(ui, |ui| {
                let tab = self.current();
                let document = tab.session.document();
                // Peça em edição: nome e saída rápida, além do lápis na árvore.
                if let Some(part) = tab.active_part.and_then(|p| document.part(p)) {
                    ui.horizontal(|ui| {
                        let width = (ui.available_width() - 30.).max(40.);
                        ui.allocate_ui_with_layout(
                            egui::vec2(width, 24.),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::Vec2::splat(18.),
                                    egui::Sense::hover(),
                                );
                                icons::paint(ui.painter(), rect, icons::Icon::Edit, true);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&part.name)
                                            .strong()
                                            .color(theme::ACCENT),
                                    )
                                    .truncate(),
                                );
                            },
                        );
                        if icons::tree_button(ui, icons::Icon::Close, text(T::LeavePart), false)
                            .clicked()
                        {
                            intents.push(project_tree::Intent::LeavePart);
                        }
                    });
                    ui.separator();
                }
                egui::ScrollArea::vertical()
                    .id_salt("project_browser")
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
            .frame(egui::Frame::new().fill(theme::CANVAS).inner_margin(8))
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
                let gizmo = self.extrusion_gizmo(ui, &response)
                    || self.section_gizmo(ui, &response)
                    || self.move_gizmo(ui, &response)
                    || self.offset_plane_gizmo(ui, &response);
                let assembly = !gizmo && self.assembly_pick(&response);
                let profile = !gizmo && self.profile_overlay(ui, &response);
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
                    if shift {
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
                self.viewport.render(&self.gpu.device, &self.gpu.queue);
                let visible_planes = self.work_planes_overlay(ui, response.rect);
                let origin_chosen = self.origin_overlay(ui, &response);
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
                        response.rect.center_bottom() - egui::vec2(0., 24.),
                        egui::Align2::CENTER_BOTTOM,
                        text(T::EmptyScene),
                        egui::FontId::proportional(14.),
                        theme::MUTED,
                    );
                }
                self.update_hover(ui, &response);
                self.draw_highlights(ui, response.rect);
                self.align_overlay(ui, response.rect);
                self.measure_overlay(ui, response.rect);
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
                                        if ui
                                            .add_enabled(
                                                !self.busy(),
                                                egui::Button::new(text(T::Recover)),
                                            )
                                            .clicked()
                                        {
                                            recover = Some(path.clone());
                                        }
                                    });
                                });
                            }
                        });
                });
            self.show_recovery = open;
            if let Some(path) = recover {
                self.show_recovery = false;
                self.open(context, Some(path));
            }
        }
    }
}
