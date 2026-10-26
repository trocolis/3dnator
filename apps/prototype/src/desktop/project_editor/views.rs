//! Composição visual. Alterações do documento são encaminhadas ao editor e aos comandos.
use super::*;

impl Editor {
    pub(super) fn shortcuts(&mut self, context: &egui::Context) {
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
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("tools").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("3dnator");
                ui.add_enabled_ui(!self.busy(), |ui| {
                    if ui.button(text(T::NewProject)).clicked() {
                        let result = Document::new(text(T::Untitled))
                            .map_err(|e| e.to_string())
                            .and_then(|d| self.add_tab(d, None, None));
                        if let Err(error) = result {
                            self.status = error;
                        }
                    }
                    if ui.button(text(T::OpenProject)).clicked() {
                        self.open(ui.ctx(), None);
                    }
                    if ui.button(text(T::Save)).clicked() {
                        self.save(ui.ctx(), self.current().id, false, false);
                    }
                    if ui.button(text(T::SaveAs)).clicked() {
                        self.save(ui.ctx(), self.current().id, true, false);
                    }
                    ui.separator();
                    if ui
                        .add_enabled(
                            self.current().session.undo_label().is_some(),
                            egui::Button::new(text(T::Undo)),
                        )
                        .clicked()
                    {
                        self.current_mut().session.undo();
                        self.current_mut().changed();
                        self.invalidate_scene();
                    }
                    if ui
                        .add_enabled(
                            self.current().session.redo_label().is_some(),
                            egui::Button::new(text(T::Redo)),
                        )
                        .clicked()
                    {
                        self.current_mut().session.redo();
                        self.current_mut().changed();
                        self.invalidate_scene();
                    }
                    ui.separator();
                    if ui.button(text(T::Import)).clicked() {
                        self.import(ui.ctx(), false);
                    }
                    if ui.button(text(T::Reference)).clicked() {
                        self.import(ui.ctx(), true);
                    }
                    if ui.button(text(T::Drawing)).clicked() {
                        self.open_drawing(None);
                    }
                    if ui.button(text(T::AssemblyTools)).clicked() {
                        self.assembly_edit = Some(assembly_editor::Draft::new(self.current().id));
                    }
                    if ui
                        .add_enabled(
                            self.current().active_part.is_some(),
                            egui::Button::new(text(T::SolidTools)),
                        )
                        .clicked()
                    {
                        self.open_features();
                    }
                    ui.menu_button(text(T::NewSketch), |ui| {
                        for (label, plane) in [
                            ("XY", nator_sketch::Plane::xy()),
                            ("XZ", nator_sketch::Plane::xz()),
                            ("YZ", nator_sketch::Plane::yz()),
                        ] {
                            if ui.button(label).clicked() {
                                let destination = self.current().active_part;
                                self.execute(Command::AddSketch {
                                    destination,
                                    sketch: nator_sketch::Sketch::new(text(T::Sketch), plane),
                                });
                                ui.close();
                            }
                        }
                        // Face clicada (de qualquer peça visível): o plano é capturado em
                        // coordenadas da peça de destino, sem vínculo com a face de origem.
                        let picked = self.current().picked.clone();
                        if ui
                            .add_enabled(picked.is_some(), egui::Button::new(text(T::SketchOnFace)))
                            .clicked()
                            && let Some(pick) = picked
                        {
                            let document = self.current().session.document();
                            let destination = self.current().active_part.unwrap_or(pick.part);
                            let target = document
                                .instances()
                                .iter()
                                .find(|i| i.part == destination)
                                .map(|i| i.placement)
                                .unwrap_or_default();
                            let origin = target.inverse_apply(pick.point);
                            let normal = nator_geometry::sub(
                                target.inverse_apply(pick.normal),
                                target.inverse_apply([0.; 3]),
                            );
                            match nator_sketch::Plane::from_normal(origin, normal) {
                                Ok(plane) => self.execute(Command::AddSketch {
                                    destination: Some(destination),
                                    sketch: nator_sketch::Sketch::new(text(T::Sketch), plane),
                                }),
                                Err(error) => self.status = error.to_string(),
                            }
                            ui.close();
                        }
                    });
                    if ui
                        .add_enabled(
                            self.current().active_part.is_some(),
                            egui::Button::new(text(T::Parameters)),
                        )
                        .clicked()
                    {
                        self.open_parameters();
                    }
                });
            });
            let mut switch = None;
            let mut close = None;
            egui::ScrollArea::horizontal()
                .id_salt("tabs_scroll")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (index, tab) in self.tabs.iter().enumerate() {
                            let name = tab
                                .path
                                .as_ref()
                                .and_then(|p| p.file_name())
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| tab.session.document().name().into());
                            let label =
                                format!("{}{}", name, if tab.modified() { " *" } else { "" });
                            if ui.selectable_label(index == self.active, label).clicked() {
                                switch = Some(index);
                            }
                            if ui
                                .add_enabled(!self.busy(), egui::Button::new("×").small())
                                .on_hover_text(text(T::Close))
                                .clicked()
                            {
                                close = Some(tab.id);
                            }
                        }
                    });
                });
            if let Some(index) = switch {
                self.switch(index);
            }
            if let Some(id) = close {
                self.request_close(id);
            }
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.busy() {
                    ui.spinner();
                }
                ui.label(&self.status);
                if !self.recoveries.is_empty()
                    && ui
                        .button(format!("{} ({})", text(T::Recovery), self.recoveries.len()))
                        .clicked()
                {
                    self.show_recovery = true;
                }
            });
            if let Some(part) = self
                .current()
                .active_part
                .and_then(|id| self.current().session.document().part(id))
                .cloned()
            {
                let mut edit = None;
                ui.horizontal_wrapped(|ui| {
                    ui.label(text(T::ParametricHistory));
                    for f in &part.features {
                        if ui.button(&f.name).clicked() {
                            edit = Some(f.clone());
                        }
                    }
                });
                if let Some(f) = edit {
                    self.feature_edit =
                        Some(feature_editor::Draft::edit(self.current().id, part.id, f));
                }
            }
            ui.small(text(T::Navigation));
        });
    }
    pub(super) fn tree(&mut self, ui: &mut egui::Ui) {
        let document = self.current().session.document().clone();
        let selected = self.current().selection;
        let mut selection = None;
        let mut command = None;
        let busy = self.busy();
        egui::Panel::left("tree").default_size(230.).show(ui, |ui| {
            ui.heading(text(T::Project));
            let active = self.current().active_part.and_then(|p| document.part(p));
            if let Some(part) = active {
                ui.label(format!("{}: {}", text(T::Construction), part.name));
                if ui.button(text(T::LeavePart)).clicked() {
                    self.current_mut().active_part = None;
                    self.current_mut().sketch_edit = None;
                }
            } else {
                ui.label(text(T::Assembly));
            }
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for instance in document.instances().iter().filter(|i| i.parent.is_none()) {
                    let Some(part) = document.part(instance.part) else {
                        continue;
                    };
                    ui.horizontal(|ui| {
                        let mut visible = instance.visible;
                        if ui
                            .add_enabled(!busy, egui::Checkbox::without_text(&mut visible))
                            .changed()
                        {
                            command = Some(Command::SetInstanceVisibility {
                                id: instance.id,
                                visible,
                            });
                        }
                        if ui
                            .selectable_label(
                                selected == Some(Selection::Instance(instance.id)),
                                &part.name,
                            )
                            .clicked()
                        {
                            selection = Some(Selection::Instance(instance.id));
                        }
                    });
                    ui.indent(instance.id.0, |ui| {
                        self.child_tree(ui, &document, part.id, 0, &mut selection);
                        for sketch in &part.sketches {
                            let selected = self
                                .current()
                                .sketch_edit
                                .as_ref()
                                .is_some_and(|w| w.id == sketch.id);
                            if ui
                                .selectable_label(
                                    selected,
                                    format!("{} · {}", sketch.sketch.name, text(T::Sketch)),
                                )
                                .clicked()
                            {
                                self.begin_sketch(part.id, sketch.id);
                            }
                        }
                        for body in &part.bodies {
                            ui.horizontal(|ui| {
                                let mut visible = body.visible;
                                if ui
                                    .add_enabled(!busy, egui::Checkbox::without_text(&mut visible))
                                    .changed()
                                {
                                    command = Some(Command::SetBodyVisibility {
                                        id: body.id,
                                        visible,
                                    });
                                }
                                let target = Selection::Body {
                                    part: part.id,
                                    id: body.id,
                                };
                                if ui
                                    .selectable_label(
                                        selected == Some(target),
                                        format!(
                                            "{} · {}",
                                            body.name,
                                            text(if body.brep.is_some() {
                                                T::SolidBody
                                            } else {
                                                T::MeshBody
                                            })
                                        ),
                                    )
                                    .clicked()
                                {
                                    selection = Some(target);
                                }
                            });
                        }
                    });
                }
            });
            ui.separator();
            for sheet in document.sheets() {
                if ui
                    .button(format!(
                        "{} · {}",
                        text(T::Drawing),
                        sheet.specification.title
                    ))
                    .clicked()
                {
                    self.open_drawing(Some(sheet.clone()));
                }
            }
            ui.small(text(T::PrototypeNotice));
        });
        if let Some(selection) = selection {
            self.current_mut().select(Some(selection));
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
            .default_size(255.)
            .show(ui, |ui| {
                ui.heading(text(T::Properties));
                self.measure_controls(ui);
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
                    ui.label(text(T::SelectElement));
                }
                if let Some(Selection::Body { id, .. }) = selection {
                    if let Some(body) = document.body(id) {
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
                                ui.add(egui::DragValue::new(value).speed(0.1).range(-1e9..=1e9));
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
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button(text(T::Fit)).clicked() {
                    let [w, h] = self.viewport.size();
                    self.viewport.camera.fit(f64::from(w) / f64::from(h));
                }
                if ui.button(text(T::Iso)).clicked() {
                    self.viewport.camera = Camera::default();
                }
                if ui.button(text(T::Front)).clicked() {
                    self.viewport.camera.yaw = -std::f64::consts::FRAC_PI_2;
                    self.viewport.camera.pitch = 0.;
                }
                if ui.button(text(T::Top)).clicked() {
                    self.viewport.camera.yaw = -std::f64::consts::FRAC_PI_2;
                    self.viewport.camera.pitch = std::f64::consts::FRAC_PI_2;
                }
                if ui.button(text(T::Side)).clicked() {
                    self.viewport.camera.yaw = 0.;
                    self.viewport.camera.pitch = 0.;
                }
            });
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
            self.viewport_actions(ui, &response);
            if response.dragged_by(egui::PointerButton::Middle) {
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
            if response.hovered() {
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
            self.current_mut().camera = self.viewport.camera.clone();
            self.viewport.render(&self.gpu.device, &self.gpu.queue);
            if self.scene.is_none() && !self.busy() {
                ui.painter().text(
                    response.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text(T::EmptyScene),
                    egui::FontId::proportional(14.),
                    egui::Color32::LIGHT_GRAY,
                );
            }
            self.navigation_cube(ui, response.rect);
            self.measure_overlay(ui, response.rect);
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
