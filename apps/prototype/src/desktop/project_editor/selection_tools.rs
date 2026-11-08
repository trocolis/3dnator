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

/// Referência de nível mais alto na ocorrência clicada. Na construção, um modelo de
/// referência se move inteiro: nada dentro dele é editável.
pub(super) fn reference_instance(document: &Document, pick: &Pick) -> Option<InstanceId> {
    document
        .occurrence_at(pick)
        .ok()?
        .path
        .into_iter()
        .find(|id| {
            document
                .instance(*id)
                .and_then(|i| document.part(i.part))
                .is_some_and(|p| p.reference)
        })
}

/// Referência que pertence à peça em edição: filha direta dela (pasta Referências).
pub(super) fn owned_reference(document: &Document, active: Option<PartId>, id: InstanceId) -> bool {
    document.instance(id).is_some_and(|i| {
        active.is_some() && i.parent == active && document.part(i.part).is_some_and(|p| p.reference)
    })
}

/// Na construção, movem-se os corpos da peça ativa e as referências dela; referências do
/// projeto ou de outras peças só na Montagem. O erro diz por que o clique foi recusado.
pub(super) fn movable_in_construction(
    document: &Document,
    active: Option<PartId>,
    pick: &Pick,
) -> Result<(), T> {
    let Some(part) = active else {
        return Ok(());
    };
    if part == pick.part {
        return Ok(());
    }
    match reference_instance(document, pick) {
        Some(id) if owned_reference(document, active, id) => Ok(()),
        Some(_) => Err(T::PickForeignReference),
        None => Err(T::PickOtherPart),
    }
}

impl Tab {
    /// Na montagem, cliques no modelo escolhem o conjunto da raiz. Componentes
    /// internos continuam acessíveis explicitamente pela árvore.
    pub(super) fn selection_for_pick(&self, pick: &Pick) -> Option<Selection> {
        if self.active_part == Some(pick.part) {
            return Some(Selection::Body {
                part: pick.part,
                id: pick.body,
            });
        }
        if self.active_part.is_some() {
            return Some(Selection::Instance(
                reference_instance(self.session.document(), pick).unwrap_or(pick.instance),
            ));
        }
        self.session
            .document()
            .occurrence_at(pick)
            .ok()?
            .path
            .first()
            .copied()
            .map(Selection::Instance)
    }
}
impl Editor {
    /// Um clique antigo não é reaproveitado se o elemento saiu da seleção.
    pub(super) fn selected_pick(&self) -> Option<&Pick> {
        let pick = self.current().picked.as_ref()?;
        let tab = self.current();
        let selected = if tab.active_part == Some(pick.part) {
            tab.is_selected(Selection::Body {
                part: pick.part,
                id: pick.body,
            })
        } else {
            tab.session.document().occurrence_at(pick).is_ok_and(|o| {
                o.path
                    .iter()
                    .any(|id| tab.is_selected(Selection::Instance(*id)))
            })
        };
        selected.then_some(pick)
    }

    /// A seleção é o estado neutro: cancelar uma ferramenta não apaga os alvos
    /// escolhidos nem as alterações já confirmadas no documento.
    pub(super) fn selection_active(&self) -> bool {
        self.image_edit.is_none()
            && self.sketch_text.is_none()
            && self.paint_edit.is_none()
            && self.feature_edit.is_none()
            && self.assembly_panel.is_none()
            && self.section_edit.is_none()
            && self.work_plane_edit.is_none()
            && self.current().plane_pick.is_none()
            && self.sketch_repair.is_none()
            && self.insert_part.is_none()
            && self.mesh_import.is_none()
            && self.parameters_edit.is_none()
            && self.print_profile_edit.is_none()
            && self.print_check.is_none()
            && self.calibration.is_none()
            && self.drawing_edit.is_none()
            && self.export_edit.is_none()
            && !self.shell.measure_open
            && self
                .current()
                .sketch_edit
                .as_ref()
                .is_none_or(|work| work.is_selecting() && work.armed().is_none())
    }

    pub(super) fn activate_selection(&mut self, context: &egui::Context) {
        self.image_edit = None;
        if let Some(mut draft) = self.sketch_text.take() {
            draft.cancel(context);
        }
        self.cancel_plane_pick();
        self.close_offset_plane();
        self.cancel_feature_tool();
        self.assembly_panel = None;
        self.cancel_section_tool();
        self.sketch_repair = None;
        self.insert_part = None;
        self.mesh_import = None;
        self.parameters_edit = None;
        self.print_profile_edit = None;
        self.close_print_check();
        self.calibration = None;
        self.drawing_edit = None;
        self.export_edit = None;
        self.shell.measure_open = false;
        self.measure_tool.clear();
        self.shell.camera_mode = None;
        self.paint_edit = None;
        self.gestures = SelectionGestures::default();
        if let Some(work) = self.current_mut().sketch_edit.as_mut() {
            work.activate_selection(context);
        }
        // Invalidar a prévia também impede que um resultado calculado depois
        // de Esc volte a substituir a cena publicada no documento.
        self.preview_command(self.current().id, None, None, None);
        context.request_repaint();
    }

    /// Esc anda um nível por vez: primeiro desativa a ferramenta, conservando o modo de
    /// navegação; sem ferramenta ativa, Medir, Arrastar e Rotacionar voltam a Selecionar.
    pub(super) fn escape_to_selection(&mut self, context: &egui::Context) {
        // No Medir, Esc primeiro apaga a medição; sem alvos, sai da ferramenta.
        if self.shell.measure_open && !self.measure_tool.is_empty() {
            self.measure_tool.clear();
            context.request_repaint();
            return;
        }
        let camera = self.shell.camera_mode;
        let tool_active = !self.selection_active() && !self.shell.measure_open;
        self.activate_selection(context);
        if tool_active {
            self.shell.camera_mode = camera;
        }
    }

    /// Gestos disponíveis fora de rascunhos, seletores de plano, campos de painel e do
    /// arraste livre de posicionamento.
    fn gestures_enabled(&self) -> bool {
        if self.paint_edit.is_some() {
            return false;
        }
        self.current().sketch_edit.is_none()
            && self.current().plane_pick.is_none()
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
            && response.contains_pointer()
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
                    egui::Stroke::new(1.2, theme::accent()),
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
        let mut assembly = std::collections::BTreeMap::new();
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
                if active.is_none() {
                    if let Some(root) = occurrence.path.first() {
                        assembly
                            .entry(root.0)
                            .and_modify(|value: &mut bool| {
                                *value = if crossing {
                                    *value || chosen
                                } else {
                                    *value && chosen
                                }
                            })
                            .or_insert(chosen);
                    }
                    continue;
                }
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
        items.extend(
            assembly
                .into_iter()
                .filter_map(|(id, chosen)| chosen.then_some(Selection::Instance(InstanceId(id)))),
        );
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
        let Ok(mut picks) = document.pick_all_clipped_with(
            ray,
            |o| view.pickable(o),
            section::clip(self.current().section),
        ) else {
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
                            .color(theme::muted()),
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
            let Some(item) = self.current().selection_for_pick(&pick) else {
                return;
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

#[cfg(test)]
mod reference_pick_tests {
    use super::*;
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn pick(document: &Document, instance: InstanceId) -> Result<Pick> {
        let occurrence = document
            .occurrences()?
            .into_iter()
            .find(|o| o.instance == instance)
            .ok_or("Ocorrência ausente")?;
        let record = document.instance(instance).ok_or("Instância ausente")?;
        let body = document
            .part(record.part)
            .ok_or("Peça ausente")?
            .bodies
            .first()
            .ok_or("Corpo ausente")?
            .id;
        Ok(Pick {
            instance,
            part: record.part,
            body,
            triangle: 0,
            point: [0.; 3],
            normal: [0., 0., 1.],
            vertices: [[0.; 3]; 3],
            placement: occurrence.placement,
        })
    }

    fn add_other(document: &mut Document) -> Result<(PartId, InstanceId)> {
        let (part, _) = document.add_mesh(
            None,
            "Outra".into(),
            Arc::new(MeshAsset::new(reference_bracket()?)),
        )?;
        let instance = document
            .instances()
            .iter()
            .find(|i| i.part == part)
            .ok_or("Instância ausente")?
            .id;
        Ok((part, instance))
    }

    #[test]
    fn clicks_inside_a_reference_assembly_select_its_top_reference() -> Result<()> {
        let mut document = Document::new("Construção")?;
        let mut add = |name: &str| -> Result<(PartId, InstanceId)> {
            let (part, _) = document.add_mesh(
                None,
                name.into(),
                Arc::new(MeshAsset::new(reference_bracket()?)),
            )?;
            let instance = document
                .instances()
                .iter()
                .find(|i| i.part == part)
                .ok_or("Instância ausente")?
                .id;
            Ok((part, instance))
        };
        let (_, own) = add("Peça")?;
        let (motor, motor_instance) = add("Motor")?;
        let (shaft, shaft_instance) = add("Eixo")?;
        document.reparent(shaft_instance, Some(motor))?;
        document.part_mut(motor)?.reference = true;
        document.part_mut(shaft)?.reference = true;
        // O eixo dentro do motor importado: move-se o motor inteiro.
        assert_eq!(
            reference_instance(&document, &pick(&document, shaft_instance)?),
            Some(motor_instance)
        );
        assert_eq!(
            reference_instance(&document, &pick(&document, motor_instance)?),
            Some(motor_instance)
        );
        assert_eq!(reference_instance(&document, &pick(&document, own)?), None);
        // Em edição da "Peça": o motor do projeto não pertence a ela.
        let own_part = document.instance(own).ok_or("Instância ausente")?.part;
        let active = Some(own_part);
        assert!(matches!(
            movable_in_construction(&document, active, &pick(&document, shaft_instance)?),
            Err(T::PickForeignReference)
        ));
        assert!(movable_in_construction(&document, active, &pick(&document, own)?).is_ok());
        // Levado às Referências da peça, o motor (e o eixo dentro dele) passa a se mover.
        document.reparent(motor_instance, Some(own_part))?;
        assert!(owned_reference(&document, active, motor_instance));
        assert!(!owned_reference(&document, active, shaft_instance));
        assert!(
            movable_in_construction(&document, active, &pick(&document, shaft_instance)?).is_ok()
        );
        // Outra peça comum continua recusada como corpo de outra peça.
        let (_, other) = add_other(&mut document)?;
        assert!(matches!(
            movable_in_construction(&document, active, &pick(&document, other)?),
            Err(T::PickOtherPart)
        ));
        Ok(())
    }
}
