//! Navegação e posicionamento por coordenadas capturadas, sem vínculos entre peças.
use super::*;
use nator_document::assembly::Pick;
use nator_geometry::{dot, norm, sub};
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
impl Editor {
    pub(super) fn viewport_actions(&mut self, ui: &egui::Ui, response: &egui::Response) {
        let cube_rect = view_cube::rect(response.rect);
        let starts_on_cube = ui
            .input(|i| i.pointer.press_origin())
            .is_some_and(|p| cube_rect.contains(p));
        if response.drag_started_by(egui::PointerButton::Primary) && !starts_on_cube {
            self.current_mut().drag_start = ui.input(|i| i.pointer.press_origin());
        }
        // No Medir, o duplo clique no segundo alvo decompõe a distância nos eixos; o
        // clique que o acompanha não escolhe outro alvo.
        // Um clique logo depois de outro alvo chega como triplo: vale como duplo.
        let measuring = self.shell.measure_open;
        let repeated = response.double_clicked() || response.triple_clicked();
        if measuring && repeated {
            self.measure_tool.expand(self.busy());
        }
        if response.clicked()
            && !(measuring && repeated)
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
            let additive = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
            let awaiting = self.feature_edit.as_ref().and_then(|d| d.awaiting());
            let extrusion = self.feature_edit.as_ref().is_some_and(|d| d.planar_pick());
            let area = self
                .feature_edit
                .as_ref()
                .is_some_and(|d| d.choosing_area());
            // Oito pontos de tela em mm: a mesma margem da captura de vértices do rascunho.
            let vertex_reach = {
                let aspect = f64::from(response.rect.aspect_ratio());
                let step = f64::from(8. / response.rect.width() * 2.);
                let a = self
                    .viewport
                    .camera
                    .world_on_target_plane(cursor, bounds, aspect);
                let b = self.viewport.camera.world_on_target_plane(
                    [cursor[0] + step, cursor[1]],
                    bounds,
                    aspect,
                );
                nator_geometry::norm(sub(b, a))
            };
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
                if measuring {
                    let target = pick.as_ref().map(|p| {
                        nator_commands::measure::target_at(&document, p, vertex_reach)
                            .map_err(|e| e.to_string())
                    });
                    return Ok(Outcome::MeasureTarget {
                        tab,
                        revision,
                        target,
                    });
                }
                // Com o diálogo de operações esperando, o clique vira referência por nomes.
                let topology = awaiting.zip(pick.as_ref()).map(|(kind, p)| {
                    match kind {
                        feature_editor::Awaiting::Edge => {
                            match nator_commands::topology::edges_at_vertex(
                                &document,
                                p,
                                vertex_reach,
                            ) {
                                Ok(edges) if !edges.is_empty() => {
                                    Ok(feature_editor::Kept::Edges(edges))
                                }
                                _ => nator_commands::topology::edge_at(&document, p)
                                    .map(feature_editor::Kept::Edge),
                            }
                        }
                        feature_editor::Awaiting::Face if extrusion => {
                            feature_editor::planar_area(&document, p, area)
                                .map_err(nator_document::DocumentError)
                        }
                        feature_editor::Awaiting::Face => {
                            document.face_at(p).map(feature_editor::Kept::Face)
                        }
                        feature_editor::Awaiting::Component => {
                            Ok(feature_editor::Kept::Component(p.body, p.placement))
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
    }
}
