//! Navegação e posicionamento por coordenadas capturadas, sem vínculos entre peças.
use super::*;
use nator_document::assembly::Pick;
use nator_geometry::{dot, norm, sub};

/// Camadas de faces sob o cursor e a escolhida com ↑/↓ (0: a primeira). Mover o mouse
/// volta à primeira camada.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct FaceLayers {
    pub layer: usize,
    pub at: Option<egui::Pos2>,
    /// Camadas encontradas na última consulta, para limitar as setas.
    pub count: usize,
}
impl FaceLayers {
    /// Volta à primeira camada se o cursor andou; aplica ↑/↓. Devolve se a camada mudou.
    pub(super) fn update(&mut self, context: &egui::Context, position: egui::Pos2) -> bool {
        if self.at.is_none_or(|at| at.distance(position) > 3.) {
            let changed = self.layer != 0;
            self.layer = 0;
            self.at = Some(position);
            return changed | super::sketch_scene::step_layer(context, &mut self.layer, self.count);
        }
        super::sketch_scene::step_layer(context, &mut self.layer, self.count)
    }
    /// Sufixo do rótulo com a posição da camada, quando há mais de uma.
    pub(super) fn label(&self, label: &str) -> String {
        if self.count > 1 {
            format!("{label} ({}/{}, ↑/↓)", self.layer + 1, self.count)
        } else {
            label.to_owned()
        }
    }
}

/// Faces sob o raio, da frente para trás, inclusive as vistas por trás (a face oposta de
/// um sólido), uma vez cada. As que `eligible` aceita viram camadas; uma superfície não
/// elegível na frente ocupa a primeira camada sem face (`None`), para o clique simples não
/// atravessá-la sem querer. A parte removida pelo corte fica de fora.
pub(super) fn face_layers(
    document: &Document,
    view: &ViewState,
    section: Option<section::Section>,
    ray: nator_geometry::query::Ray,
    eligible: impl Fn(&Pick) -> bool,
) -> Vec<Option<Pick>> {
    let Ok(picks) =
        document.pick_layers_clipped_with(ray, |o| view.pickable(o), section::clip(section))
    else {
        return Vec::new();
    };
    let visible = section.filter(|s| s.visible);
    let mut seen = std::collections::HashSet::new();
    let mut layers = Vec::new();
    for pick in picks
        .into_iter()
        .filter(|p| visible.is_none_or(|s| s.keeps(p.point)))
    {
        // A mesma face atingida por triângulos vizinhos (ou duas vezes) conta uma vez.
        let face = document.face_index_at(&pick).ok();
        let key = (
            pick.instance,
            pick.body,
            face,
            face.is_none().then_some(pick.triangle),
        );
        if !seen.insert(key) {
            continue;
        }
        if eligible(&pick) {
            layers.push(Some(pick));
        } else if layers.is_empty() {
            layers.push(None);
        }
    }
    layers
}
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
    /// Oito pontos de tela em mm, compartilhados pelo candidato e pelo clique.
    pub(super) fn pick_reach(
        &self,
        position: egui::Pos2,
        rect: egui::Rect,
        bounds: nator_geometry::Bounds,
    ) -> f64 {
        let cursor = [
            f64::from((position.x - rect.center().x) / rect.width() * 2.),
            f64::from((rect.center().y - position.y) / rect.height() * 2.),
        ];
        let aspect = f64::from(rect.aspect_ratio());
        let a = self
            .viewport
            .camera
            .world_on_target_plane(cursor, bounds, aspect);
        let b = self.viewport.camera.world_on_target_plane(
            [cursor[0] + f64::from(8. / rect.width() * 2.), cursor[1]],
            bounds,
            aspect,
        );
        norm(sub(b, a))
    }

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
            // Camada escolhida com ↑/↓ quando a ferramenta espera uma face.
            let face_layer = (awaiting == Some(feature_editor::Awaiting::Face))
                .then_some(self.face_layers.layer)
                .filter(|layer| *layer > 0);
            let feature_part = self.feature_edit.as_ref().map(|d| d.part());
            // Oito pontos de tela em mm: a mesma margem da captura de vértices do rascunho.
            let vertex_reach = self.pick_reach(position, response.rect, bounds);
            let view = self.current().view.clone();
            let cut = self.current().section;
            let measurement = measure_tool::query::Query {
                camera: self.viewport.camera.clone(),
                bounds,
                rect: response.rect,
                position,
                reach: vertex_reach,
                section: cut,
            };
            self.start(ui.ctx(), None, move || {
                if measuring {
                    let target = match measurement.run(&document, &view) {
                        Ok(candidate) => candidate.map(|candidate| Ok(candidate.target)),
                        Err(error) => Some(Err(error)),
                    };
                    return Ok(Outcome::MeasureTarget {
                        tab,
                        revision,
                        target,
                    });
                }
                // A parte escondida pelo corte de seção não é selecionável.
                let pick = match face_layer {
                    // A face destacada pelas setas, entre as aptas sob o cursor.
                    Some(layer) => face_layers(&document, &view, cut, ray, |p| {
                        if extrusion {
                            Some(p.part) == feature_part
                                && document.body(p.body).is_some_and(|b| b.brep.is_some())
                                && nator_commands::measure::sketch_plane(&document, p).is_ok()
                        } else {
                            document.face_at(p).is_ok()
                        }
                    })
                    .get(layer)
                    .cloned()
                    .flatten(),
                    None => section::first_visible(
                        document
                            .pick_all_clipped_with(
                                ray,
                                |occurrence| view.pickable(occurrence),
                                section::clip(cut),
                            )
                            .map_err(|e| e.to_string())?,
                        cut,
                    ),
                };
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
