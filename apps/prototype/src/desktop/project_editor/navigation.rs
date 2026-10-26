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
    use nator_drawing::format_mm as mm;
    let mut lines = Vec::new();
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
    pub(super) fn viewport_actions(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if response.drag_started_by(egui::PointerButton::Primary) {
            self.current_mut().drag_start = ui.input(|i| i.pointer.press_origin());
        }
        let cube_rect = egui::Rect::from_min_size(
            response.rect.right_top() + egui::vec2(-112., 10.),
            egui::vec2(105., 110.),
        );
        if response.clicked()
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
            let document = self.current().session.document().clone();
            let tab = self.current().id;
            let revision = self.current().session.revision();
            let mode = self.pick_mode;
            let awaiting = self.feature_edit.as_ref().and_then(|d| d.awaiting());
            self.start(ui.ctx(), None, move || {
                let pick = document.pick(ray).map_err(|e| e.to_string())?;
                // Com o diálogo de operações esperando, o clique vira referência por nomes.
                let topology = awaiting.zip(pick.as_ref()).map(|(kind, p)| {
                    match kind {
                        feature_editor::Awaiting::Edge => {
                            document.edge_at(p).map(feature_editor::Kept::Edge)
                        }
                        feature_editor::Awaiting::Face => {
                            document.face_at(p).map(feature_editor::Kept::Face)
                        }
                    }
                    .map_err(|e| e.to_string())
                });
                Ok(Outcome::Picked {
                    tab,
                    revision,
                    pick,
                    mode,
                    topology,
                })
            });
        }
        if self.move_freely
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
                .world_ray([0.; 2], bounds, aspect)
                .origin;
            let b = self
                .viewport
                .camera
                .world_ray(
                    [
                        f64::from(d.x / response.rect.width() * 2.),
                        f64::from(-d.y / response.rect.height() * 2.),
                    ],
                    bounds,
                    aspect,
                )
                .origin;
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
                    Err(e) => self.status = e.to_string(),
                }
            }
        }
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
                "X {:.3}  Y {:.3}  Z {:.3}",
                pick.point[0], pick.point[1], pick.point[2]
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
                    let document = self.current().session.document().clone();
                    let tab = self.current().id;
                    let revision = self.current().session.revision();
                    let mode = self.pick_mode;
                    let assembly = self.current().active_part.is_none();
                    let reference = self.current().reference.clone();
                    let pick = pick.clone();
                    // Medidas exatas consultam o núcleo: ficam fora da thread da interface.
                    self.start(ui.ctx(), None, move || {
                        let lines =
                            measure_lines(&document, &pick, mode, assembly, reference.as_ref());
                        Ok(Outcome::Measured {
                            tab,
                            revision,
                            lines,
                        })
                    });
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
                    Err(e) => self.status = e.to_string(),
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
                        Err(e) => self.status = e.to_string(),
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
            ui.monospace(format!("{:.4} mm", norm(d)));
            ui.small(format!("ΔX {:.3}  ΔY {:.3}  ΔZ {:.3}", d[0], d[1], d[2]));
        }
    }
    pub(super) fn measure_overlay(&self, ui: &egui::Ui, rect: egui::Rect) {
        let Some(bounds) = self.current().scene_bounds else {
            return;
        };
        let screen = |p| {
            let p = self
                .viewport
                .camera
                .project_world(p, bounds, f64::from(rect.aspect_ratio()));
            rect.center()
                + egui::vec2(
                    p[0] as f32 * rect.width() / 2.,
                    -p[1] as f32 * rect.height() / 2.,
                )
        };
        if let Some(p) = &self.current().picked {
            ui.painter().circle_stroke(
                screen(p.point),
                6.,
                egui::Stroke::new(2., egui::Color32::YELLOW),
            );
        }
        if let [a, b] = self.current().measurements.as_slice() {
            let p = [screen(*a), screen(*b)];
            ui.painter()
                .line_segment(p, egui::Stroke::new(1.5, egui::Color32::LIGHT_GREEN));
            ui.painter().text(
                p[0].lerp(p[1], 0.5),
                egui::Align2::CENTER_BOTTOM,
                format!("{:.3} mm", norm(sub(*b, *a))),
                egui::FontId::proportional(14.),
                egui::Color32::WHITE,
            );
        }
    }
    pub(super) fn navigation_cube(&mut self, ui: &egui::Ui, rect: egui::Rect) {
        let center = rect.right_top() + egui::vec2(-65., 65.);
        let camera = &self.viewport.camera;
        let direction = camera.direction();
        let projection = |p: Point3| {
            let v = camera.axis_projection(p);
            center + egui::vec2(v[0] as f32, v[1] as f32) * 29.
        };
        let mut faces = Vec::new();
        // Cubo truncado: seis octógonos, doze faixas de arestas e oito cantos clicáveis.
        for axis in 0..3 {
            for sign in [-1., 1.] {
                let a = (axis + 1) % 3;
                let b = (axis + 2) % 3;
                let polygon = [
                    [-0.65, -1.],
                    [0.65, -1.],
                    [1., -0.65],
                    [1., 0.65],
                    [0.65, 1.],
                    [-0.65, 1.],
                    [-1., 0.65],
                    [-1., -0.65],
                ]
                .map(|v| {
                    let mut p = [0.; 3];
                    p[axis] = sign;
                    p[a] = v[0];
                    p[b] = v[1];
                    p
                });
                let mut normal = [0.; 3];
                normal[axis] = sign;
                faces.push((normal, polygon.to_vec()));
            }
        }
        for axis in 0..3 {
            for a in [-1., 1.] {
                for b in [-1., 1.] {
                    let i = (axis + 1) % 3;
                    let j = (axis + 2) % 3;
                    let polygon = [
                        [-0.65, 1., 0.65],
                        [0.65, 1., 0.65],
                        [0.65, 0.65, 1.],
                        [-0.65, 0.65, 1.],
                    ]
                    .map(|v| {
                        let mut p = [0.; 3];
                        p[axis] = v[0];
                        p[i] = a * v[1];
                        p[j] = b * v[2];
                        p
                    });
                    let mut n = [0.; 3];
                    n[i] = a;
                    n[j] = b;
                    faces.push((n, polygon.to_vec()));
                }
            }
        }
        for x in [-1., 1.] {
            for y in [-1., 1.] {
                for z in [-1., 1.] {
                    faces.push((
                        [x, y, z],
                        vec![
                            [x, y * 0.65, z * 0.65],
                            [x * 0.65, y, z * 0.65],
                            [x * 0.65, y * 0.65, z],
                        ],
                    ));
                }
            }
        }
        faces.retain(|(n, _)| dot(*n, direction) > 0.);
        faces.sort_by(|a, b| dot(a.0, direction).total_cmp(&dot(b.0, direction)));
        let mut chosen = None;
        for (index, (normal, points)) in faces.into_iter().enumerate() {
            let polygon = points.into_iter().map(projection).collect::<Vec<_>>();
            let bounds = egui::Rect::from_points(&polygon);
            let hover = ui
                .input(|i| i.pointer.hover_pos())
                .is_some_and(|p| inside_polygon(p, &polygon));
            let _response =
                ui.interact(bounds, egui::Id::new(("cube", index)), egui::Sense::click());
            ui.painter().add(egui::Shape::convex_polygon(
                polygon,
                if hover {
                    egui::Color32::from_rgb(85, 128, 165)
                } else {
                    egui::Color32::from_gray(75)
                },
                egui::Stroke::new(1., egui::Color32::from_gray(150)),
            ));
            if hover && ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary)) {
                chosen = Some(normal);
            }
        }
        if let Some(direction) = chosen {
            self.viewport.camera.set_direction(direction);
        }
        let axis_origin = center + egui::vec2(0., 48.);
        for (axis, label, color) in [
            ([1., 0., 0.], "X", egui::Color32::LIGHT_RED),
            ([0., 1., 0.], "Y", egui::Color32::LIGHT_GREEN),
            ([0., 0., 1.], "Z", egui::Color32::LIGHT_BLUE),
        ] {
            let v = self.viewport.camera.axis_projection(axis);
            let end = axis_origin + egui::vec2(v[0] as f32, v[1] as f32) * 19.;
            ui.painter()
                .line_segment([axis_origin, end], egui::Stroke::new(1.5, color));
            ui.painter().text(
                end,
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(11.),
                color,
            );
        }
    }
}
fn inside_polygon(p: egui::Pos2, vertices: &[egui::Pos2]) -> bool {
    let mut sign = 0.;
    for i in 0..vertices.len() {
        let a = vertices[i];
        let b = vertices[(i + 1) % vertices.len()];
        let c = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        if c.abs() > 0.1 {
            if sign == 0. {
                sign = c.signum();
            } else if sign * c < 0. {
                return false;
            }
        }
    }
    true
}
