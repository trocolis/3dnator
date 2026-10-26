//! Composição visual. Alterações do documento são encaminhadas ao editor e aos comandos.
use super::*;

impl Editor {
    pub(super) fn shortcuts(&mut self, context: &egui::Context) {
        if self.shell.search_open {
            if context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.shell.search_open = false;
            }
            return;
        }
        if self.busy() || self.pending_close.is_some() {
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
        if context.egui_wants_keyboard_input() {
            return;
        }
        if context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::S)) {
            self.shell.search_open = true;
            self.shell.search_focus = true;
            return;
        }
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
            self.current_mut().changed();
            self.invalidate_scene();
        }
    }
    pub(super) fn tree(&mut self, ui: &mut egui::Ui) {
        let document = self.current().session.document().clone();
        let selected = self.current().selection;
        let mut selection = None;
        let mut command = None;
        let mut activate = None;
        let busy = self.busy();
        egui::Panel::left("tree")
            .default_size(235.)
            .min_size(190.)
            .max_size(390.)
            .frame(theme::panel_frame())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(text(T::Project)).strong().size(14.))
                        .on_hover_text(text(T::PrototypeNotice));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(document.parts().len().to_string())
                                .small()
                                .color(theme::MUTED),
                        );
                    });
                });
                ui.separator();
                let active = self.current().active_part.and_then(|p| document.part(p));
                if let Some(part) = active {
                    ui.label(
                        egui::RichText::new(&part.name)
                            .strong()
                            .color(theme::ACCENT),
                    );
                    if ui.button(text(T::LeavePart)).clicked() {
                        self.current_mut().active_part = None;
                        self.current_mut().sketch_edit = None;
                    }
                } else {
                    theme::section(ui, text(T::Assembly));
                }
                if document.instances().is_empty() {
                    ui.add_space(16.);
                    ui.label(egui::RichText::new(text(T::BrowserEmpty)).strong());
                    ui.label(egui::RichText::new(text(T::StartSketch)).color(theme::MUTED));
                }
                egui::ScrollArea::vertical()
                    .id_salt("project_browser")
                    .show(ui, |ui| {
                        for instance in document.instances().iter().filter(|i| i.parent.is_none()) {
                            let Some(part) = document.part(instance.part) else {
                                continue;
                            };
                            egui::collapsing_header::CollapsingState::load_with_default_open(
                                ui.ctx(),
                                egui::Id::new(("part_node", self.current().id, instance.id.0)),
                                true,
                            )
                            .show_header(ui, |ui| {
                                if ui
                                    .add_enabled_ui(!busy, |ui| {
                                        icons::button(
                                            ui,
                                            if instance.visible {
                                                icons::Icon::Eye
                                            } else {
                                                icons::Icon::EyeOff
                                            },
                                            text(T::Visibility),
                                            false,
                                            false,
                                        )
                                    })
                                    .inner
                                    .clicked()
                                {
                                    command = Some(Command::SetInstanceVisibility {
                                        id: instance.id,
                                        visible: !instance.visible,
                                    });
                                }
                                let response = icons::entry(
                                    ui,
                                    icons::Icon::Cube,
                                    &part.name,
                                    selected == Some(Selection::Instance(instance.id)),
                                );
                                if response.clicked() {
                                    selection = Some(Selection::Instance(instance.id));
                                }
                                if response.double_clicked() && !busy {
                                    activate = Some(part.id);
                                }
                            })
                            .body(|ui| {
                                self.child_tree(ui, &document, part.id, 0, &mut selection);
                                if !part.sketches.is_empty() {
                                    theme::section(
                                        ui,
                                        &format!(
                                            "{} ({})",
                                            text(T::SketchesGroup),
                                            part.sketches.len()
                                        ),
                                    );
                                }
                                for sketch in &part.sketches {
                                    let selected = self
                                        .current()
                                        .sketch_edit
                                        .as_ref()
                                        .is_some_and(|w| w.id == sketch.id);
                                    if icons::entry(
                                        ui,
                                        icons::Icon::Sketch,
                                        &sketch.sketch.name,
                                        selected,
                                    )
                                    .clicked()
                                    {
                                        self.begin_sketch(part.id, sketch.id);
                                    }
                                }
                                if !part.bodies.is_empty() {
                                    theme::section(
                                        ui,
                                        &format!(
                                            "{} ({})",
                                            text(T::SolidBodies),
                                            part.bodies.len()
                                        ),
                                    );
                                }
                                for body in &part.bodies {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add_enabled_ui(!busy, |ui| {
                                                icons::button(
                                                    ui,
                                                    if body.visible {
                                                        icons::Icon::Eye
                                                    } else {
                                                        icons::Icon::EyeOff
                                                    },
                                                    text(T::Visibility),
                                                    false,
                                                    false,
                                                )
                                            })
                                            .inner
                                            .clicked()
                                        {
                                            command = Some(Command::SetBodyVisibility {
                                                id: body.id,
                                                visible: !body.visible,
                                            });
                                        }
                                        let target = Selection::Body {
                                            part: part.id,
                                            id: body.id,
                                        };
                                        let response = icons::entry(
                                            ui,
                                            icons::Icon::Cube,
                                            &body.name,
                                            selected == Some(target),
                                        );
                                        if response.clicked() {
                                            selection = Some(target);
                                        }
                                        if response.double_clicked() && !busy {
                                            activate = Some(part.id);
                                        }
                                    });
                                }
                            });
                        }
                        if !document.sheets().is_empty() {
                            ui.separator();
                            theme::section(ui, text(T::DocumentGroup));
                        }
                        for sheet in document.sheets() {
                            if icons::entry(
                                ui,
                                icons::Icon::Drawing,
                                &sheet.specification.title,
                                false,
                            )
                            .clicked()
                            {
                                self.open_drawing(Some(sheet.clone()));
                            }
                        }
                    });
            });
        if let Some(selection) = selection {
            self.current_mut().select(Some(selection));
        }
        if let Some(part) = activate {
            self.current_mut().active_part = Some(part);
            self.current_mut().sketch_edit = None;
        }
        if let Some(command) = command {
            self.execute(command);
        }
    }
    pub(super) fn properties(&mut self, ui: &mut egui::Ui) {
        if self.current().sketch_edit.is_some() {
            self.sketch_properties(ui);
            return;
        }
        let selection = self.current().selection;
        let document = self.current().session.document().clone();
        let part = match selection {
            Some(Selection::Body { part, .. }) => Some(part),
            Some(Selection::Instance(id)) => document.instance(id).map(|i| i.part),
            None => None,
        };
        let editable_body = matches!(selection,Some(Selection::Body{part,..}) if self.current().active_part==Some(part));
        let editable_position = editable_body
            || matches!(selection,Some(Selection::Instance(_)) if self.current().active_part.is_none());
        let busy = self.busy();
        let mut command = None;
        let mut export = false;
        egui::Panel::right("properties")
            .default_size(285.)
            .min_size(250.)
            .max_size(420.)
            .frame(theme::panel_frame())
            .show(ui, |ui| {
                ui.label(egui::RichText::new(text(T::Properties)).strong().size(14.));
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("property_scroll")
                    .show(ui, |ui| {
                        egui::CollapsingHeader::new(text(T::MeasureSelection))
                            .open(self.shell.measure_requested.then_some(true))
                            .default_open(self.current().picked.is_some())
                            .show(ui, |ui| self.measure_controls(ui));
                        self.shell.measure_requested = false;
                        ui.separator();
                        if let Some(part) = part {
                            if ui
                                .add_enabled(!busy, egui::Button::new(text(T::EditPart)))
                                .clicked()
                            {
                                self.current_mut().active_part = Some(part);
                            }
                            ui.text_edit_singleline(&mut self.current_mut().name_input);
                            if ui
                                .add_enabled(
                                    !busy
                                        && (editable_body
                                            || matches!(selection, Some(Selection::Instance(_)))),
                                    egui::Button::new(text(T::Rename)),
                                )
                                .clicked()
                            {
                                let name = self.current().name_input.clone();
                                command = match selection {
                                    Some(Selection::Body { id, .. }) => {
                                        Some(Command::RenameBody { id, name })
                                    }
                                    Some(Selection::Instance(_)) => {
                                        Some(Command::RenamePart { id: part, name })
                                    }
                                    None => None,
                                };
                            }
                        } else {
                            ui.add_space(8.);
                            ui.label(
                                egui::RichText::new(text(T::SelectionHint)).color(theme::MUTED),
                            );
                            ui.add_space(8.);
                        }
                        egui::CollapsingHeader::new(text(T::VisualControls))
                            .show(ui, |ui| self.visual_controls(ui, selection));
                        if let Some(Selection::Body { id, .. }) = selection {
                            if let Some(body) = document.body(id) {
                                ui.label(
                                    egui::RichText::new(text(if body.brep.is_some() {
                                        T::SolidBody
                                    } else {
                                        T::MeshBody
                                    }))
                                    .small()
                                    .color(theme::MUTED),
                                );
                                ui.add_enabled_ui(!busy && editable_body, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(text(T::BodyColor));
                                        let mut channels = body
                                            .color
                                            .unwrap_or(nator_geometry::RgbColor::NEUTRAL)
                                            .channels();
                                        if ui.color_edit_button_srgb(&mut channels).changed() {
                                            command = Some(Command::SetBodyColor {
                                                id,
                                                color: Some(nator_geometry::RgbColor::new(
                                                    channels,
                                                )),
                                            });
                                        }
                                    });
                                    if body.color.is_some()
                                        && ui.button(text(T::NeutralColor)).clicked()
                                    {
                                        command = Some(Command::SetBodyColor { id, color: None });
                                    }
                                });
                                ui.separator();
                                ui.label(text(T::Dimensions));
                                for (axis, value) in ["X", "Y", "Z"]
                                    .into_iter()
                                    .zip(body.geometry.mesh().bounds().size())
                                {
                                    ui.monospace(format!("{axis}  {value:.3}"));
                                }
                                ui.label(format!(
                                    "{}: {}",
                                    text(T::Triangles),
                                    body.geometry.mesh().triangles().len()
                                ));
                                ui.label(format!(
                                    "{}: {}",
                                    text(T::BoundaryEdges),
                                    body.geometry.topology().boundary_edges
                                ));
                                ui.small(text(T::VolumeNotice));
                            }
                            if ui
                                .add_enabled(!busy, egui::Button::new(text(T::Export)))
                                .clicked()
                            {
                                export = true;
                            }
                            if ui
                                .add_enabled(
                                    !busy && editable_body,
                                    egui::Button::new(text(T::DeleteBody)),
                                )
                                .clicked()
                            {
                                command = Some(Command::RemoveBody { id });
                            }
                        }
                        if selection.is_some() {
                            ui.separator();
                            ui.label(text(T::Position));
                            ui.add_enabled_ui(!busy && editable_position, |ui| {
                                ui.horizontal(|ui| {
                                    for value in &mut self.current_mut().position_input {
                                        ui.add(
                                            egui::DragValue::new(value)
                                                .speed(0.1)
                                                .range(-1e9..=1e9),
                                        );
                                    }
                                });
                                ui.label(text(T::Rotation));
                                ui.horizontal(|ui| {
                                    for value in &mut self.current_mut().rotation_input {
                                        ui.add(
                                            egui::DragValue::new(value)
                                                .speed(1.)
                                                .range(-3600.0..=3600.0),
                                        );
                                    }
                                });
                                if ui.button(text(T::Apply)).clicked() {
                                    match Placement::new(
                                        self.current().position_input,
                                        self.current().rotation_input.map(f64::to_radians),
                                    ) {
                                        Ok(placement) => {
                                            command = match selection {
                                                Some(Selection::Body { id, .. }) => {
                                                    Some(Command::MoveBody { id, placement })
                                                }
                                                Some(Selection::Instance(id)) => {
                                                    Some(Command::MoveInstance { id, placement })
                                                }
                                                None => None,
                                            }
                                        }
                                        Err(error) => self.status = error.to_string(),
                                    }
                                }
                            });
                            if editable_body {
                                ui.small(text(T::BodyPosition));
                            }
                        }
                        ui.separator();
                        ui.checkbox(&mut self.viewport.show_triangles, text(T::Triangulation));
                        ui.checkbox(&mut self.viewport.show_shadows, text(T::Shadows));
                        ui.label(text(T::Scale));
                        ui.add(
                            egui::DragValue::new(&mut self.import_scale)
                                .range(0.000_001..=1_000_000.)
                                .speed(0.01),
                        );
                        ui.separator();
                        ui.label(text(T::History));
                        ui.small(
                            self.current()
                                .session
                                .undo_label()
                                .unwrap_or(text(T::NoHistory)),
                        );
                    });
            });
        if let Some(command) = command {
            self.execute(command);
        }
        if export {
            self.open_export();
        }
    }
    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        if self.current().sketch_edit.is_some() {
            self.sketch_canvas(ui);
            return;
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::CANVAS).inner_margin(8))
            .show(ui, |ui| {
                self.camera_bar(ui);
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
                let response = ui.add(
                    egui::Image::from_texture((self.texture, size))
                        .sense(egui::Sense::click_and_drag()),
                );
                response.clone().on_hover_text(text(T::Navigation));
                self.viewport_actions(ui, &response);
                let cube_rect = view_cube::rect(response.rect);
                let starts_on_cube = ui
                    .input(|i| i.pointer.press_origin())
                    .is_some_and(|p| cube_rect.contains(p));
                if response.dragged_by(egui::PointerButton::Middle) && !starts_on_cube {
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
                view_cube::show(ui, &mut self.viewport.camera, response.rect);
                self.viewport.render(&self.gpu.device, &self.gpu.queue);
                if self.scene.is_none() && !self.busy() {
                    ui.painter().text(
                        response.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        text(T::EmptyScene),
                        egui::FontId::proportional(14.),
                        theme::MUTED,
                    );
                }
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
                            for path in &self.recoveries {
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
                                    ui.small(
                                        path.file_name().unwrap_or_default().to_string_lossy(),
                                    );
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
