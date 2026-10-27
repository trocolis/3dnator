//! Navegação e posicionamento por coordenadas capturadas, sem vínculos entre peças.
use super::*;
use nator_document::assembly::Pick;
use nator_geometry::{cross, dot, norm, sub};
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum PickMode {
    Vertex,
    Edge,
    Face,
}
pub(super) fn snap_pick(p: &mut Pick, mode: PickMode) {
    match mode {
        PickMode::Vertex => {
            if let Some(v) = p
                .vertices
                .iter()
                .min_by(|a, b| norm(sub(**a, p.point)).total_cmp(&norm(sub(**b, p.point))))
            {
                p.point = *v;
            }
        }
        PickMode::Edge => {
            if let Some((a, b, q)) = [
                (p.vertices[0], p.vertices[1]),
                (p.vertices[1], p.vertices[2]),
                (p.vertices[2], p.vertices[0]),
            ]
            .into_iter()
            .map(|(a, b)| {
                let d = sub(b, a);
                let t = (dot(sub(p.point, a), d) / dot(d, d)).clamp(0., 1.);
                let q = std::array::from_fn(|i| a[i] + t * d[i]);
                (a, b, q)
            })
            .min_by(|a, b| norm(sub(a.2, p.point)).total_cmp(&norm(sub(b.2, p.point))))
            {
                p.vertices[0] = a;
                p.vertices[1] = b;
                p.point = q;
            }
        }
        PickMode::Face => {}
    }
}
/// Relatório de medição da seleção: propriedades do corpo, face/aresta (sólidos) e, com uma
/// referência, ângulo entre faces e distância mínima entre corpos.
fn measure_lines(
    doc: &Document,
    pick: &Pick,
    mode: PickMode,
    assembly: bool,
    reference: Option<&Pick>,
) -> Vec<String> {
    use nator_commands::measure::{self, Topology};
    // Casas decimais configuradas no documento; a precisão interna não muda.
    let decimals = usize::from(doc.settings().display_decimals);
    let mm = |value: f64| nator_drawing::format_fixed(value, decimals);
    let mut lines = Vec::new();
    if let Some(body) = doc.body(pick.body) {
        let size = body.geometry.mesh().bounds().size().map(mm);
        lines.push(format!(
            "{}: {} × {} × {}",
            text(T::Dimensions),
            size[0],
            size[1],
            size[2]
        ));
        lines.push(format!(
            "{}: {}",
            text(T::Triangles),
            body.geometry.mesh().triangles().len()
        ));
        lines.push(format!(
            "{}: {}",
            text(T::BoundaryEdges),
            body.geometry.topology().boundary_edges
        ));
        lines.push(text(T::VolumeNotice).into());
    }
    match measure::body_properties(doc, pick.body) {
        Ok(p) => {
            // Na montagem, o centro vai para a origem da montagem; na construção, da peça.
            let c = if assembly {
                pick.placement.apply(p.center_mm)
            } else {
                p.center_mm
            };
            lines.push(format!("{}: {} mm³", text(T::Volume), mm(p.volume_mm3)));
            lines.push(format!("{}: {} mm²", text(T::Area), mm(p.area_mm2)));
            lines.push(format!(
                "{} ({}): {} | {} | {}",
                text(T::CenterOfMass),
                text(T::UniformDensity),
                mm(c[0]),
                mm(c[1]),
                mm(c[2])
            ));
            lines.push(
                text(if p.exact {
                    T::ExactMeasure
                } else {
                    T::ApproximateMeasure
                })
                .into(),
            );
        }
        Err(error) => lines.push(error.to_string()),
    }
    if mode != PickMode::Vertex {
        let radius = |r: f64| {
            format!(
                "{}: {} mm | {}: {} mm",
                text(T::RadiusLabel),
                mm(r),
                text(T::DiameterLabel),
                mm(2. * r)
            )
        };
        match measure::topology(doc, pick, mode == PickMode::Edge) {
            Ok(Topology::Face {
                name,
                area_mm2,
                radius_mm,
                ..
            }) => {
                lines.push(format!(
                    "{} {name}: {} mm²",
                    text(T::FaceLabel),
                    mm(area_mm2)
                ));
                lines.extend(radius_mm.map(radius));
            }
            Ok(Topology::Edge {
                faces,
                length_mm,
                radius_mm,
            }) => {
                lines.push(format!(
                    "{} {} × {}: {} {} mm",
                    text(T::EdgeLabel),
                    faces[0],
                    faces[1],
                    text(T::Length),
                    mm(length_mm)
                ));
                lines.extend(radius_mm.map(radius));
            }
            Err(error) => lines.push(error.to_string()),
        }
    }
    if let Some(reference) = reference {
        if mode == PickMode::Face
            && let Ok(pair) = measure::face_pair(reference, pick)
        {
            lines.push(format!(
                "{}: {}°",
                text(T::AngleLabel),
                mm(pair.angle_rad.to_degrees())
            ));
            if let Some(distance) = pair.parallel_distance_mm {
                lines.push(format!(
                    "{}: {} mm",
                    text(T::ParallelDistance),
                    mm(distance)
                ));
            }
        }
        if reference.body != pick.body || reference.instance != pick.instance {
            match measure::body_distance(doc, reference, pick) {
                Ok(distance) => {
                    lines.push(format!("{}: {} mm", text(T::MinDistance), mm(distance)));
                }
                Err(error) => lines.push(error.to_string()),
            }
        }
    }
    lines
}

impl Editor {
    /// Medições têm uma janela própria, aberta pela ferramenta Inspecionar → Medir.
    /// O viewport conserva toda a largura e os controles existentes continuam disponíveis.
    pub(super) fn measure_window(&mut self, context: &egui::Context) {
        if !self.shell.measure_open {
            return;
        }
        let mut open = true;
        egui::Window::new(text(T::MeasureSelection))
            .id(egui::Id::new("measure_selection"))
            .default_width(330.)
            .default_pos(command_panel::default_position(self.canvas_rect))
            .open(&mut open)
            .show(context, |ui| {
                egui::ScrollArea::vertical()
                    .max_height((self.canvas_rect.height() - 120.).max(180.))
                    .show(ui, |ui| self.measure_controls(ui));
            });
        self.shell.measure_open = open;
    }

    pub(super) fn viewport_actions(&mut self, ui: &egui::Ui, response: &egui::Response) {
        let cube_rect = view_cube::rect(response.rect);
        let starts_on_cube = ui
            .input(|i| i.pointer.press_origin())
            .is_some_and(|p| cube_rect.contains(p));
        if response.drag_started_by(egui::PointerButton::Primary) && !starts_on_cube {
            self.current_mut().drag_start = ui.input(|i| i.pointer.press_origin());
        }
        if response.clicked()
            && !starts_on_cube
            && !response
                .interact_pointer_pos()
                .is_some_and(|p| cube_rect.contains(p))
            && !self.busy()
            && let Some(position) = response.interact_pointer_pos()
            && let Some(bounds) = self.current().scene_bounds
        {
            let cursor = [
                f64::from((position.x - response.rect.center().x) / response.rect.width() * 2.),
                f64::from((response.rect.center().y - position.y) / response.rect.height() * 2.),
            ];
            let ray = self.viewport.camera.world_ray(
                cursor,
                bounds,
                f64::from(response.rect.aspect_ratio()),
            );
            let document = self.picking_document();
            let tab = self.current().id;
            let revision = self.current().session.revision();
            let mode = self.pick_mode;
            let additive = ui.input(|i| i.modifiers.command);
            let awaiting = self.feature_edit.as_ref().and_then(|d| d.awaiting());
            let view = self.current().view.clone();
            let cut = self.current().section;
            self.start(ui.ctx(), None, move || {
                // A parte escondida pelo corte de seção não é selecionável.
                let pick = section::first_visible(
                    document
                        .pick_all_with(ray, |occurrence| view.pickable(occurrence))
                        .map_err(|e| e.to_string())?,
                    cut,
                );
                // Com o diálogo de operações esperando, o clique vira referência por nomes.
                let topology = awaiting.zip(pick.as_ref()).map(|(kind, p)| {
                    match kind {
                        feature_editor::Awaiting::Edge => {
                            document.edge_at(p).map(feature_editor::Kept::Edge)
                        }
                        feature_editor::Awaiting::Face => {
                            document.face_at(p).map(feature_editor::Kept::Face)
                        }
                        feature_editor::Awaiting::Body => {
                            if document.body(p.body).is_some_and(|b| b.brep.is_some()) {
                                Ok(feature_editor::Kept::Body(p.body))
                            } else {
                                Err(nator_document::DocumentError(
                                    text(T::SolidBodyRequired).into(),
                                ))
                            }
                        }
                    }
                    .map_err(|e| e.to_string())
                });
                Ok(Outcome::Picked {
                    tab,
                    revision,
                    pick,
                    mode,
                    additive,
                    topology,
                })
            });
        }
        if self.move_freely
            && !starts_on_cube
            && !self.busy()
            && response.drag_stopped_by(egui::PointerButton::Primary)
            && let Some(bounds) = self.current().scene_bounds
        {
            let d = response
                .interact_pointer_pos()
                .zip(self.current_mut().drag_start.take())
                .map(|(end, start)| end - start)
                .unwrap_or_default();
            let aspect = f64::from(response.rect.aspect_ratio());
            let a = self
                .viewport
                .camera
                .world_on_target_plane([0.; 2], bounds, aspect);
            let b = self.viewport.camera.world_on_target_plane(
                [
                    f64::from(d.x / response.rect.width() * 2.),
                    f64::from(-d.y / response.rect.height() * 2.),
                ],
                bounds,
                aspect,
            );
            let delta = sub(b, a);
            let result = match self.current().selection {
                Some(Selection::Instance(id)) if self.current().active_part.is_none() => {
                    self.current().session.document().instance(id).map(|i| {
                        let local = if let Some(pick) = &self.current().picked {
                            pick.placement
                                .inverse()
                                .and_then(|world_inverse| i.placement.compose(world_inverse))
                                .map(|parent_inverse| {
                                    sub(parent_inverse.apply(delta), parent_inverse.apply([0.; 3]))
                                })
                        } else {
                            Ok(delta)
                        };
                        local
                            .and_then(|d| i.placement.translated(d))
                            .map(|placement| Command::MoveInstance { id, placement })
                    })
                }
                Some(Selection::Body { part, id }) if self.current().active_part == Some(part) => {
                    self.current().session.document().body(id).map(|body| {
                        let local = if let Some(p) = &self.current().picked {
                            sub(
                                p.placement.inverse_apply(delta),
                                p.placement.inverse_apply([0.; 3]),
                            )
                        } else {
                            delta
                        };
                        body.placement
                            .translated(local)
                            .map(|placement| Command::MoveBody { id, placement })
                    })
                }
                _ => None,
            };
            if let Some(result) = result {
                match result {
                    Ok(command) => self.prepare_command(ui.ctx(), self.current().id, command),
                    Err(e) => self.status.error(e.to_string()),
                }
            }
        }
    }
    pub(super) fn measure_current_selection(&mut self, context: &egui::Context) {
        if self.busy() {
            return;
        }
        let Some(pick) = self.current().picked.clone() else {
            return;
        };
        let document = self.current().session.document().clone();
        let tab = self.current().id;
        let revision = self.current().session.revision();
        let mode = self.pick_mode;
        let assembly = self.current().active_part.is_none();
        let reference = self.current().reference.clone();
        // As medidas do B-rep continuam sendo calculadas fora da interface.
        self.start(context, None, move || {
            let lines = measure_lines(&document, &pick, mode, assembly, reference.as_ref());
            Ok(Outcome::Measured {
                tab,
                revision,
                lines,
            })
        });
    }
    pub(super) fn measure_controls(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("pick_mode")
            .selected_text(text(match self.pick_mode {
                PickMode::Vertex => T::PickVertex,
                PickMode::Edge => T::PickEdge,
                PickMode::Face => T::PickFace,
            }))
            .show_ui(ui, |ui| {
                for (mode, label) in [
                    (PickMode::Vertex, T::PickVertex),
                    (PickMode::Edge, T::PickEdge),
                    (PickMode::Face, T::PickFace),
                ] {
                    ui.selectable_value(&mut self.pick_mode, mode, text(label));
                }
            });
        ui.checkbox(&mut self.move_freely, text(T::FreeMove));
        if let Some(pick) = self.current().picked.clone() {
            ui.small(text(T::MeshSelectionNotice));
            ui.monospace(format!(
                "X {}  Y {}  Z {}",
                self.mm(pick.point[0]),
                self.mm(pick.point[1]),
                self.mm(pick.point[2])
            ));
            if ui.button(text(T::Measure)).clicked() {
                let points = &mut self.current_mut().measurements;
                if points.len() >= 2 {
                    points.clear();
                }
                points.push(pick.point);
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.busy(), egui::Button::new(text(T::MeasureSelection)))
                    .clicked()
                {
                    self.measure_current_selection(ui.ctx());
                }
                if ui.button(text(T::SetReference)).clicked() {
                    self.current_mut().reference = Some(pick.clone());
                }
            });
            for line in &self.measure_report {
                ui.small(line);
            }
            if ui.button(text(T::AlignmentSource)).clicked() {
                self.alignment = Some((
                    self.current().id,
                    self.current().session.revision(),
                    pick.clone(),
                ));
            }
            if let Some(source) = self
                .alignment
                .clone()
                .filter(|(tab, revision, _)| {
                    *tab == self.current().id && *revision == self.current().session.revision()
                })
                .map(|(_, _, source)| source)
                && ui
                    .add_enabled(
                        !self.busy() && self.current().active_part.is_none(),
                        egui::Button::new(text(T::AlignSelection)),
                    )
                    .clicked()
                && source.instance != pick.instance
            {
                let doc = self.current().session.document();
                let result = (|| -> Result<Command, nator_geometry::GeometryError> {
                    let Some(instance) = doc.instance(source.instance) else {
                        return Err(nator_geometry::GeometryError(
                            "Instância de origem ausente.".into(),
                        ));
                    };
                    let mut world = source.placement;
                    if self.pick_mode != PickMode::Vertex {
                        let from = if self.pick_mode == PickMode::Face {
                            source.normal
                        } else {
                            sub(source.vertices[1], source.vertices[0])
                        };
                        let to = if self.pick_mode == PickMode::Face {
                            pick.normal.map(|v| -v)
                        } else {
                            sub(pick.vertices[1], pick.vertices[0])
                        };
                        let angle = nator_geometry::query::angle_between(from, to)?;
                        let mut axis = cross(from, to);
                        if norm(axis) < 1e-9 {
                            axis = cross(
                                from,
                                if from[0].abs() < from[1].abs() {
                                    [1., 0., 0.]
                                } else {
                                    [0., 1., 0.]
                                },
                            );
                        }
                        if angle.abs() > 1e-9 {
                            world = world.rotated_about(source.point, axis, angle)?;
                        }
                    }
                    world = world.translated(sub(pick.point, source.point))?;
                    let parent_inverse = instance.placement.compose(source.placement.inverse()?)?;
                    Ok(Command::MoveInstance {
                        id: source.instance,
                        placement: parent_inverse.compose(world)?,
                    })
                })();
                match result {
                    Ok(command) => {
                        self.prepare_command(ui.ctx(), self.current().id, command);
                        self.alignment = None;
                    }
                    Err(e) => self.status.error(e.to_string()),
                }
            }
            ui.label(text(T::RotateAngle));
            ui.add(egui::DragValue::new(&mut self.rotate_angle).range(-3600.0..=3600.0));
            if ui
                .add_enabled(
                    !self.busy() && self.current().active_part.is_none(),
                    egui::Button::new(text(T::RotatePivot)),
                )
                .clicked()
            {
                let doc = self.current().session.document();
                if let Some(instance) = doc.instance(pick.instance) {
                    let pivot = if self.pick_mode == PickMode::Face {
                        doc.instance_center(instance.id)
                            .ok()
                            .and_then(|p| {
                                instance
                                    .placement
                                    .inverse()
                                    .and_then(|i| pick.placement.compose(i))
                                    .map(|parent| parent.apply(p))
                                    .ok()
                            })
                            .unwrap_or(pick.point)
                    } else {
                        pick.point
                    };
                    let axis = if self.pick_mode == PickMode::Edge {
                        sub(pick.vertices[1], pick.vertices[0])
                    } else {
                        [0., 0., 1.]
                    };
                    let result = pick
                        .placement
                        .rotated_about(pivot, axis, self.rotate_angle.to_radians())
                        .and_then(|world| {
                            instance
                                .placement
                                .compose(pick.placement.inverse()?)?
                                .compose(world)
                        });
                    match result {
                        Ok(placement) => self.prepare_command(
                            ui.ctx(),
                            self.current().id,
                            Command::MoveInstance {
                                id: pick.instance,
                                placement,
                            },
                        ),
                        Err(e) => self.status.error(e.to_string()),
                    }
                }
            }
        }
        if !self.current().measurements.is_empty()
            && ui.button(text(T::ClearMeasurements)).clicked()
        {
            self.current_mut().measurements.clear();
        }
        if self.current().measurements.len() == 2 {
            let d = sub(
                self.current().measurements[1],
                self.current().measurements[0],
            );
            ui.monospace(format!("{} mm", self.mm(norm(d))));
            ui.small(format!(
                "ΔX {}  ΔY {}  ΔZ {}",
                self.mm(d[0]),
                self.mm(d[1]),
                self.mm(d[2])
            ));
        }
    }
    pub(super) fn measure_overlay(&self, ui: &egui::Ui, rect: egui::Rect) {
        let Some(bounds) = self.current().scene_bounds else {
            return;
        };
        let screen = |p| {
            let p =
                self.viewport
                    .camera
                    .project_world(p, bounds, f64::from(rect.aspect_ratio()))?;
            Some(
                rect.center()
                    + egui::vec2(
                        p[0] as f32 * rect.width() / 2.,
                        -p[1] as f32 * rect.height() / 2.,
                    ),
            )
        };
        if let Some(p) = &self.current().picked
            && let Some(position) = screen(p.point)
        {
            ui.painter().circle_stroke(
                position,
                6.,
                egui::Stroke::new(2., egui::Color32::from_rgb(190, 113, 21)),
            );
        }
        if let [a, b] = self.current().measurements.as_slice()
            && let (Some(start), Some(end)) = (screen(*a), screen(*b))
        {
            let p = [start, end];
            ui.painter()
                .line_segment(p, egui::Stroke::new(1.5, theme::GREEN));
            ui.painter().text(
                p[0].lerp(p[1], 0.5),
                egui::Align2::CENTER_BOTTOM,
                format!("{} mm", self.mm(norm(sub(*b, *a)))),
                egui::FontId::proportional(14.),
                theme::INK,
            );
        }
    }
}
