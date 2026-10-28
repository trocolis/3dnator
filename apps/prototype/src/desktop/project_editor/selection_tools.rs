//! Gestos de seleção da área 3D, no padrão dos CADs de mercado:
//! - janela da esquerda para a direita seleciona os elementos inteiramente contidos;
//! - da direita para a esquerda seleciona os elementos tocados;
//! - Ctrl acrescenta à seleção atual;
//! - segurar o botão esquerdo abre a lista dos elementos sobrepostos sob o cursor.
//!
//! Na montagem a unidade é a instância; na construção, o corpo da peça ativa.
use super::*;
use nator_document::assembly::Pick;

/// Tempo de pressão para abrir a lista de sobrepostos.
const LONG_PRESS: Duration = Duration::from_millis(550);
/// Deslocamento máximo, em pontos, para ainda contar como pressão parada.
const STILL: f32 = 4.;

pub(super) struct WindowSelect {
    tab: u64,
    start: egui::Pos2,
}

pub(super) struct SelectOther {
    tab: u64,
    revision: u64,
    position: egui::Pos2,
    picks: Vec<Pick>,
}

pub(super) struct Press {
    origin: egui::Pos2,
    started: Instant,
    /// A pressão abriu a lista: o clique ao soltar não seleciona o mais próximo.
    consumed: bool,
}

#[derive(Default)]
pub(super) struct SelectionGestures {
    pub window: Option<WindowSelect>,
    pub other: Option<SelectOther>,
    pub press: Option<Press>,
}

impl Editor {
    /// Um clique antigo não é reaproveitado se o elemento saiu da seleção.
    pub(super) fn selected_pick(&self) -> Option<&Pick> {
        let pick = self.current().picked.as_ref()?;
        let item = if self.current().active_part == Some(pick.part) {
            Selection::Body {
                part: pick.part,
                id: pick.body,
            }
        } else {
            Selection::Instance(pick.instance)
        };
        self.current().is_selected(item).then_some(pick)
    }

    /// A seleção é o estado neutro: cancelar uma ferramenta não apaga os alvos
    /// escolhidos nem as alterações já confirmadas no documento.
    pub(super) fn selection_active(&self) -> bool {
        self.feature_edit.is_none()
            && self.assembly_panel.is_none()
            && self.section_edit.is_none()
            && self.work_plane_edit.is_none()
            && self.current().plane_pick.is_none()
            && self.sketch_repair.is_none()
            && self.insert_part.is_none()
            && self.mesh_import.is_none()
            && self.parameters_edit.is_none()
            && self.drawing_edit.is_none()
            && self.export_edit.is_none()
            && !self.shell.measure_open
            && !self.move_freely
            && self
                .current()
                .sketch_edit
                .as_ref()
                .is_none_or(|work| work.is_selecting())
    }

    pub(super) fn activate_selection(&mut self, context: &egui::Context) {
        self.cancel_plane_pick();
        self.close_offset_plane();
        self.cancel_feature_tool();
        self.assembly_panel = None;
        self.cancel_section_tool();
        self.sketch_repair = None;
        self.insert_part = None;
        self.mesh_import = None;
        self.parameters_edit = None;
        self.drawing_edit = None;
        self.export_edit = None;
        self.shell.measure_open = false;
        self.move_freely = false;
        self.alignment = None;
        self.gestures = SelectionGestures::default();
        if let Some(work) = self.current_mut().sketch_edit.as_mut() {
            work.activate_selection(context);
        }
        // Invalidar a prévia também impede que um resultado calculado depois
        // de Esc volte a substituir a cena publicada no documento.
        self.preview_command(self.current().id, None, None, None);
        context.request_repaint();
    }

    /// Gestos disponíveis fora de rascunhos, seletores de plano, campos de painel e do
    /// arraste livre de posicionamento.
    fn gestures_enabled(&self) -> bool {
        self.current().sketch_edit.is_none()
            && self.current().plane_pick.is_none()
            && !self.move_freely
            && self
                .feature_edit
                .as_ref()
                .is_none_or(|d| d.field().is_none())
            && self.assembly_panel.is_none()
            && !self.busy()
    }

    fn cursor(&self, rect: egui::Rect, position: egui::Pos2) -> [f64; 2] {
        [
            f64::from((position.x - rect.center().x) / rect.width() * 2.),
            f64::from((rect.center().y - position.y) / rect.height() * 2.),
        ]
    }

    /// Trata janela e pressão longa; devolve verdadeiro quando o clique deste quadro não
    /// deve selecionar o elemento mais próximo.
    pub(super) fn selection_gestures(&mut self, ui: &egui::Ui, response: &egui::Response) -> bool {
        let tab = self.current().id;
        let on_cube = |p: egui::Pos2| view_cube::rect(response.rect).contains(p);
        let enabled = self.gestures_enabled();
        // Pressão longa sem movimento: lista de sobrepostos.
        let (down, pointer) = ui.input(|i| (i.pointer.primary_down(), i.pointer.interact_pos()));
        if ui.input(|i| i.pointer.primary_pressed())
            && let Some(origin) = pointer.filter(|p| response.rect.contains(*p) && !on_cube(*p))
        {
            self.gestures.press = Some(Press {
                origin,
                started: Instant::now(),
                consumed: false,
            });
        }
        let mut consumed = false;
        if let Some(press) = &mut self.gestures.press {
            let moved = pointer.is_some_and(|p| p.distance(press.origin) > STILL);
            if !down || moved {
                consumed = press.consumed;
                if !down {
                    self.gestures.press = None;
                }
            } else if !press.consumed && enabled {
                if press.started.elapsed() >= LONG_PRESS {
                    press.consumed = true;
                    let origin = press.origin;
                    self.open_select_other(response.rect, origin);
                } else {
                    ui.ctx().request_repaint_after(LONG_PRESS);
                }
            }
        }
        // Janela de seleção arrastada pelo botão esquerdo a partir de um ponto livre.
        if enabled
            && response.drag_started_by(egui::PointerButton::Primary)
            && let Some(start) = ui.input(|i| i.pointer.press_origin())
            && !on_cube(start)
        {
            self.gestures.window = Some(WindowSelect { tab, start });
        }
        if let Some(window) = self.gestures.window.as_ref().filter(|w| w.tab == tab)
            && let Some(current) = pointer
        {
            let rect = egui::Rect::from_two_pos(window.start, current);
            let crossing = current.x < window.start.x;
            let painter = ui.painter().with_clip_rect(response.rect);
            if crossing {
                painter.rect_filled(
                    rect,
                    0,
                    egui::Color32::from_rgba_unmultiplied(46, 160, 92, 28),
                );
                let corners = [
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                    rect.left_top(),
                ];
                for shape in egui::Shape::dashed_line(
                    &corners,
                    egui::Stroke::new(1.2, egui::Color32::from_rgb(46, 140, 82)),
                    6.,
                    4.,
                ) {
                    painter.add(shape);
                }
            } else {
                painter.rect(
                    rect,
                    0,
                    egui::Color32::from_rgba_unmultiplied(35, 112, 180, 28),
                    egui::Stroke::new(1.2, theme::ACCENT),
                    egui::StrokeKind::Inside,
                );
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) || !down {
                let additive = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                let items = self.window_items(response.rect, rect, crossing);
                self.current_mut().select_many(items, additive);
                self.gestures.window = None;
                consumed = true;
            }
        } else if self.gestures.window.is_some() && !down {
            self.gestures.window = None;
        }
        consumed
    }

    /// Elementos dentro (ou tocados pela) janela, pela caixa envolvente projetada.
    fn window_items(
        &self,
        canvas: egui::Rect,
        window: egui::Rect,
        crossing: bool,
    ) -> Vec<Selection> {
        let Some(bounds) = self.current().scene_bounds else {
            return Vec::new();
        };
        let document = self.current().session.document();
        let Ok(occurrences) = document.occurrences() else {
            return Vec::new();
        };
        let active = self.current().active_part;
        let mut items = Vec::new();
        for occurrence in occurrences
            .iter()
            .filter(|o| self.current().view.pickable(o))
        {
            let Some(part) = document
                .instance(occurrence.instance)
                .and_then(|i| document.part(i.part))
            else {
                continue;
            };
            for body in part.shown_bodies() {
                let pose = occurrence
                    .placement
                    .compose(body.placement)
                    .unwrap_or(occurrence.placement);
                let b = body.geometry.mesh().bounds();
                let corners = (0..8)
                    .map(|k| {
                        pose.apply([
                            if k & 1 == 0 { b.min[0] } else { b.max[0] },
                            if k & 2 == 0 { b.min[1] } else { b.max[1] },
                            if k & 4 == 0 { b.min[2] } else { b.max[2] },
                        ])
                    })
                    .filter_map(|p| sketch_scene::project(&self.viewport.camera, bounds, canvas, p))
                    .collect::<Vec<_>>();
                if corners.len() < 8 {
                    continue;
                }
                let projected = egui::Rect::from_points(&corners);
                let chosen = if crossing {
                    window.intersects(projected)
                } else {
                    window.contains_rect(projected)
                };
                if !chosen {
                    continue;
                }
                let item = if active == Some(part.id) {
                    Selection::Body {
                        part: part.id,
                        id: body.id,
                    }
                } else if active.is_none() {
                    Selection::Instance(occurrence.instance)
                } else {
                    continue;
                };
                if !items.contains(&item) {
                    items.push(item);
                }
            }
        }
        items
    }

    fn open_select_other(&mut self, rect: egui::Rect, origin: egui::Pos2) {
        let Some(bounds) = self.current().scene_bounds else {
            return;
        };
        let ray = self.viewport.camera.world_ray(
            self.cursor(rect, origin),
            bounds,
            f64::from(rect.aspect_ratio()),
        );
        let document = self.current().session.document();
        let view = self.current().view.clone();
        let Ok(mut picks) = document.pick_all_with(ray, |o| view.pickable(o)) else {
            return;
        };
        if let Some(cut) = self.current().section.filter(|s| s.visible) {
            picks.retain(|p| cut.keeps(p.point));
        }
        if picks.is_empty() {
            return;
        }
        self.gestures.other = Some(SelectOther {
            tab: self.current().id,
            revision: self.current().session.revision(),
            position: origin,
            picks,
        });
    }

    /// Lista de sobrepostos: percorrer as entradas realça cada elemento; o clique o seleciona.
    pub(super) fn select_other_popup(&mut self, context: &egui::Context) {
        let Some(other) = &self.gestures.other else {
            return;
        };
        if other.tab != self.current().id || other.revision != self.current().session.revision() {
            self.gestures.other = None;
            return;
        }
        let document = self.current().session.document().clone();
        let active = self.current().active_part;
        let entries = other
            .picks
            .iter()
            .map(|pick| {
                let instance = document
                    .instance(pick.instance)
                    .map(|i| i.name.clone())
                    .unwrap_or_default();
                let body = document
                    .body(pick.body)
                    .map(|b| b.name.clone())
                    .unwrap_or_default();
                if active == Some(pick.part) {
                    format!("{body} · {}", text(T::SolidBody))
                } else {
                    format!("{instance} · {body}")
                }
            })
            .collect::<Vec<_>>();
        let position = other.position;
        let mut chosen = None;
        let mut hovered = None;
        let area = egui::Area::new(egui::Id::new("select_other"))
            .order(egui::Order::Foreground)
            .fixed_pos(position + egui::vec2(8., 8.))
            .show(context, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(text(T::SelectOtherTitle))
                            .small()
                            .strong()
                            .color(theme::MUTED),
                    );
                    for (index, entry) in entries.iter().enumerate() {
                        let response = ui.selectable_label(false, entry);
                        if response.hovered() {
                            hovered = Some(index);
                        }
                        if response.clicked() {
                            chosen = Some(index);
                        }
                    }
                });
            });
        // Realce do elemento sob o cursor na lista.
        if let Some(index) = hovered
            && let Some(pick) = self
                .gestures
                .other
                .as_ref()
                .and_then(|o| o.picks.get(index))
                .cloned()
        {
            let (overlay, label) = self.pick_overlay(&document, &pick);
            self.hover = Some(highlight::Hover::fixed(
                self.current().id,
                self.current().session.revision(),
                position,
                overlay,
                label,
            ));
        }
        let outside = context.input(|i| i.pointer.any_click())
            && !area.response.contains_pointer()
            && chosen.is_none();
        if let Some(index) = chosen
            && let Some(pick) = self
                .gestures
                .other
                .take()
                .and_then(|o| o.picks.get(index).cloned())
        {
            let item = if self.current().active_part == Some(pick.part) {
                Selection::Body {
                    part: pick.part,
                    id: pick.body,
                }
            } else {
                Selection::Instance(pick.instance)
            };
            if context.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                self.current_mut().toggle(item);
            } else {
                self.current_mut().select(Some(item));
            }
            self.current_mut().picked = Some(pick);
            self.hover = None;
        } else if outside || context.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.gestures.other = None;
        }
    }
}
