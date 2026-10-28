//! Plano deslocado (aba Peça): escolhe um plano-base — um plano da origem, um plano de
//! trabalho da peça ou uma face plana — e o desloca ao longo da normal, arrastando a seta
//! ou digitando a distância. O OK publica `AddWorkPlane` na peça ativa; até lá, painel,
//! prévia e arraste pertencem à interface e não alteram o documento nem o desfazer.
//! A escolha do plano-base reaproveita o seletor de suporte do Novo rascunho. O mesmo painel
//! edita um plano existente: o OK publica `UpdateWorkPlane`, preparado no trabalhador porque
//! os rascunhos apoiados no plano se movem e o histórico da peça é recalculado.
use super::{
    command_panel::{self as panel, Footer},
    icons::{self, Icon},
    sketch_scene::{PlanePicker, PlanePurpose},
    *,
};
use nator_document::{WorkPlane, WorkPlaneId, work_planes::MAX_OFFSET};
use nator_geometry::add;
use nator_sketch::Plane;

/// Distância inicial (mm): o plano novo já aparece afastado do suporte, pronto para o
/// arraste, sem se confundir com a face de onde partiu.
const DEFAULT_OFFSET: f64 = 10.;

/// Plano-base escolhido, em coordenadas da peça, e como o painel o descreve.
struct Base {
    plane: Plane,
    label: String,
}

/// Arraste da seta: ponteiro e distância no início do gesto, direção e escala da tela.
struct Drag {
    pointer: egui::Pos2,
    initial: f64,
    axis: egui::Vec2,
    pixels_per_mm: f32,
}

pub(super) struct Draft {
    tab: u64,
    part: PartId,
    /// Ocorrência da peça usada para desenhar e para levar faces escolhidas à peça.
    placement: Placement,
    base: Option<Base>,
    /// Distância ao longo da normal do plano-base, em mm.
    pub(super) offset: f64,
    input: panel::NumberInput,
    drag: Option<Drag>,
    /// Plano existente em edição e sua definição ao abrir; `None` cria um plano novo.
    editing: Option<(WorkPlaneId, WorkPlane)>,
}
impl Draft {
    fn work_plane(&self) -> Option<WorkPlane> {
        self.base.as_ref().map(|base| WorkPlane {
            base: base.plane,
            offset: self.offset,
        })
    }
}

/// "Plano N" com o primeiro número livre na peça. Entre `len + 1` candidatos sempre há um
/// nome que nenhum plano usa.
fn next_name(part: &nator_document::Part) -> String {
    let prefix = text(T::WorkPlaneName);
    (1..=part.work_planes.len() + 1)
        .map(|n| format!("{prefix} {n}"))
        .find(|name| !part.work_planes.iter().any(|w| &w.name == name))
        .unwrap_or_else(|| prefix.to_owned())
}

impl Editor {
    pub(super) fn open_offset_plane(&mut self) {
        if self.busy() || self.current().sketch_edit.is_some() {
            return;
        }
        let Some(part) = self.current().active_part else {
            self.status.warning(text(T::ChooseActivePart));
            return;
        };
        let placement = match sketch_scene::part_placement(self.current(), part) {
            Ok(placement) => placement,
            Err(error) => {
                self.status.error(error);
                return;
            }
        };
        self.feature_edit = None;
        self.assembly_panel = None;
        self.section_edit = None;
        self.work_plane_edit = Some(Draft {
            tab: self.current().id,
            part,
            placement,
            base: None,
            offset: DEFAULT_OFFSET,
            input: panel::NumberInput::default(),
            drag: None,
            editing: None,
        });
        self.pick_offset_base();
    }

    /// Reabre o painel com um plano existente, já com o plano-base e a distância gravados.
    /// A peça dona entra em construção, como ao editar um rascunho dela.
    pub(super) fn edit_work_plane(&mut self, id: WorkPlaneId) {
        if self.busy() || self.current().sketch_edit.is_some() {
            return;
        }
        let document = self.current().session.document();
        let (Some(part), Some(plane)) = (
            document.work_plane_part(id),
            document.work_plane(id).map(|w| w.plane),
        ) else {
            return;
        };
        let placement = match sketch_scene::part_placement(self.current(), part) {
            Ok(placement) => placement,
            Err(error) => {
                self.status.error(error);
                return;
            }
        };
        self.current_mut().active_part = Some(part);
        if self.shell.tab == shell::RibbonTab::Assembly {
            self.shell.tab = shell::RibbonTab::Model;
        }
        self.current_mut().plane_pick = None;
        self.feature_edit = None;
        self.assembly_panel = None;
        self.section_edit = None;
        self.work_plane_edit = Some(Draft {
            tab: self.current().id,
            part,
            placement,
            // O plano-base gravado não guarda a origem (face, XY…), só a geometria.
            base: Some(Base {
                plane: plane.base,
                label: text(T::CurrentBase).to_owned(),
            }),
            offset: plane.offset,
            input: panel::NumberInput::default(),
            drag: None,
            editing: Some((id, plane)),
        });
        self.status.set(text(self.offset_plane_hint()));
    }

    /// Dica da barra de estado com o plano-base escolhido: criar ou mover o plano.
    fn offset_plane_hint(&self) -> T {
        if self
            .work_plane_edit
            .as_ref()
            .is_some_and(|d| d.editing.is_some())
        {
            T::EditOffsetPlaneHint
        } else {
            T::OffsetPlaneHint
        }
    }

    /// Plano existente em edição na aba ativa. A prévia o substitui na área 3D e ele não
    /// pode servir de plano-base para si mesmo.
    pub(super) fn editing_work_plane(&self) -> Option<WorkPlaneId> {
        self.work_plane_edit
            .as_ref()
            .filter(|d| d.tab == self.current().id)
            .and_then(|d| d.editing.map(|(id, _)| id))
    }

    /// Ativa o campo Plano: os planos da origem e de trabalho aparecem na ocorrência da
    /// peça e as faces planas recebem realce, como no Novo rascunho.
    fn pick_offset_base(&mut self) {
        let Some(draft) = &self.work_plane_edit else {
            return;
        };
        let picker = PlanePicker::new(
            Some(draft.part),
            draft.placement,
            false,
            PlanePurpose::OffsetBase,
        );
        self.current_mut().plane_pick = Some(picker);
        self.status.set(text(T::ChooseOffsetBase));
    }

    fn picking_offset_base(&self) -> bool {
        self.current()
            .plane_pick
            .as_ref()
            .is_some_and(|p| p.purpose == PlanePurpose::OffsetBase)
    }

    pub(super) fn offset_plane_has_base(&self) -> bool {
        self.work_plane_edit
            .as_ref()
            .is_some_and(|d| d.base.is_some())
    }

    /// Encerra a ferramenta e a escolha de suporte que ela tenha aberto, em qualquer aba.
    pub(super) fn close_offset_plane(&mut self) {
        if let Some(draft) = self.work_plane_edit.take()
            && let Some(tab) = self.find_tab(draft.tab)
        {
            tab.plane_pick
                .take_if(|p| p.purpose == PlanePurpose::OffsetBase);
        }
    }

    /// Distância do plano deslocado aberto, para capturas de ensaio.
    pub(super) fn set_offset_plane_for_capture(&mut self, offset: f64) {
        if let Some(draft) = &mut self.work_plane_edit {
            draft.offset = offset;
        }
    }

    /// Plano-base escolhido no seletor, já nas coordenadas da peça.
    pub(super) fn set_offset_base(&mut self, plane: Plane, label: String) {
        let Some(draft) = self.work_plane_edit.as_mut() else {
            return;
        };
        draft.base = Some(Base { plane, label });
        self.current_mut()
            .plane_pick
            .take_if(|p| p.purpose == PlanePurpose::OffsetBase);
        self.status.set(text(self.offset_plane_hint()));
    }

    /// Entrega da consulta da face em segundo plano. Uma resposta de outra aba ou revisão,
    /// ou que chega depois de a escolha ter sido encerrada, é descartada.
    pub(super) fn receive_offset_base(
        &mut self,
        tab: u64,
        revision: u64,
        plane: Result<(Plane, String), String>,
    ) {
        let current = self.current().id == tab && self.current().session.revision() == revision;
        let waiting = self.work_plane_edit.as_ref().is_some_and(|d| d.tab == tab)
            && self.picking_offset_base();
        if !current || !waiting {
            return;
        }
        match plane {
            Ok((plane, label)) => self.set_offset_base(plane, label),
            // Faces curvas ou cliques no vazio só avisam: a escolha continua aberta.
            Err(message) => self.status.warning(message),
        }
    }

    pub(super) fn offset_plane_panel(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.work_plane_edit.take() else {
            return;
        };
        let purpose = self.current().plane_pick.as_ref().map(|p| p.purpose);
        let document = self.current().session.document();
        // Outra ferramenta assumiu a área 3D, a aba mudou ou a peça (ou o plano em edição,
        // por exemplo ao desfazer) deixou de existir.
        let replaced = draft.tab != self.current().id
            || self.feature_edit.is_some()
            || self.assembly_panel.is_some()
            || self.section_edit.is_some()
            || self.current().sketch_edit.is_some()
            || purpose == Some(PlanePurpose::Sketch)
            || document.part(draft.part).is_none()
            || draft
                .editing
                .is_some_and(|(id, _)| document.work_plane(id).is_none());
        let title = match draft.editing.and_then(|(id, _)| document.work_plane(id)) {
            Some(record) => format!("{} · {}", text(T::OffsetPlane), record.name),
            None => text(T::OffsetPlane).to_owned(),
        };
        if replaced {
            self.work_plane_edit = Some(draft);
            self.close_offset_plane();
            return;
        }
        let picking = purpose == Some(PlanePurpose::OffsetBase);
        let decimals = usize::from(
            self.current()
                .session
                .document()
                .settings()
                .display_decimals,
        );
        let mut pick = false;
        let outcome = panel::show_validated(
            context,
            panel::Header {
                id: "offset_plane",
                icon: Icon::OffsetPlane,
                title: &title,
            },
            self.canvas_rect,
            Footer::None,
            draft.base.is_some() && !self.busy(),
            |ui| {
                let field = panel::selection(
                    ui,
                    text(T::PlaneField),
                    usize::from(draft.base.is_some()),
                    draft.base.as_ref().map(|b| b.label.as_str()),
                    picking,
                );
                if field.clear {
                    draft.base = None;
                }
                pick = field.activate || field.clear;
                let quantity = panel::Quantity::Length;
                let label = format!("{} ({})", text(T::DistanceField), quantity.unit());
                panel::row(ui, &label, |ui| {
                    ui.spacing_mut().item_spacing.x = 3.;
                    draft.input.show(
                        ui,
                        "offset_plane_distance",
                        &mut draft.offset,
                        quantity,
                        decimals,
                        96.,
                    );
                    if icons::button(ui, Icon::Flip, text(T::FlipOffset), false, false).clicked() {
                        // Sem "-0": zero continua zero ao inverter.
                        draft.offset = if draft.offset == 0. {
                            0.
                        } else {
                            -draft.offset
                        };
                    }
                });
                draft.input.is_valid(quantity)
            },
        );
        if outcome.cancel {
            self.work_plane_edit = Some(draft);
            self.close_offset_plane();
            return;
        }
        if outcome.ok
            && let Some(plane) = draft.work_plane()
            && let Some((id, original)) = draft.editing
        {
            // Sem mudança não há o que recalcular nem uma ação nova no desfazer.
            if plane != original {
                self.prepare_command(context, draft.tab, Command::UpdateWorkPlane { id, plane });
            }
            self.work_plane_edit = Some(draft);
            self.close_offset_plane();
            return;
        }
        if outcome.ok
            && let Some(plane) = draft.work_plane()
            && let Some(part) = self.current().session.document().part(draft.part)
        {
            let name = next_name(part);
            let command = Command::AddWorkPlane {
                part: draft.part,
                name: name.clone(),
                plane,
            };
            let revision = self.current().session.revision();
            self.execute(command);
            // Uma recusa mantém o painel com os valores digitados para correção.
            if self.current().session.revision() != revision {
                self.work_plane_edit = Some(draft);
                self.close_offset_plane();
                self.status
                    .set(format!("{name} {}", text(T::WorkPlaneCreated)));
                return;
            }
        }
        self.work_plane_edit = Some(draft);
        if pick {
            self.pick_offset_base();
        }
    }

    /// Prévia e seta do deslocamento: o plano-base em contorno tracejado, o plano novo
    /// preenchido e uma alça no centro dele. Arrastar a alça muda a distância ao longo da
    /// normal. Devolve verdadeiro quando o ponteiro está na alça ou a arrasta.
    pub(super) fn offset_plane_gizmo(&mut self, ui: &egui::Ui, response: &egui::Response) -> bool {
        // Só a interface muda durante o arraste; um trabalhador em curso (salvamento de
        // recuperação, por exemplo) não pode prender a alça. O OK já espera o trabalhador.
        if self.current().plane_pick.is_some() {
            return false;
        }
        let tab = self.current().id;
        let bounds = self.display_bounds();
        let camera = self.viewport.camera.clone();
        let Some(draft) = self.work_plane_edit.as_mut().filter(|d| d.tab == tab) else {
            return false;
        };
        let Some(base) = &draft.base else {
            return false;
        };
        let rect = response.rect;
        let project = |p: Point3| sketch_scene::project(&camera, bounds, rect, p);
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let world = sketch_scene::transformed_plane(base.plane, draft.placement);
        let normal = world.normal();
        let target = Plane {
            origin: add(world.origin, normal.map(|v| v * draft.offset)),
            ..world
        };
        let patch = |plane| {
            sketch_scene::plane_patch(plane, bounds)
                .iter()
                .map(|p| project(*p))
                .collect::<Option<Vec<_>>>()
        };
        if let Some(mut points) = patch(world) {
            points.push(points[0]);
            for shape in
                egui::Shape::dashed_line(&points, egui::Stroke::new(1.2, theme::MUTED), 6., 4.)
            {
                painter.add(shape);
            }
        }
        let dragging = draft.drag.is_some();
        // O centro do retângulo do plano-base; o do plano novo fica a `offset` dele.
        let anchor = world.world(world.project(bounds.center()));
        let Some(start) = project(anchor) else {
            return false;
        };
        let unit = project(add(anchor, normal))
            .map(|p| p - start)
            .unwrap_or_default();
        let (axis, pixels_per_mm) = if unit.length() > 0.5 {
            (unit.normalized(), unit.length())
        } else {
            // De frente para o plano, a normal não se projeta numa reta: o arraste vertical
            // usa a escala do próprio plano, sem exigir órbita.
            let side =
                project(world.world([world.project(anchor)[0] + 1., world.project(anchor)[1]]))
                    .map_or(1., |p| p.distance(start));
            (egui::vec2(0., -1.), side.max(1e-3))
        };
        let end = add(anchor, normal.map(|v| v * draft.offset));
        let handle = if unit.length() > 0.5 {
            project(end).unwrap_or(start)
        } else {
            start + axis * (draft.offset as f32 * pixels_per_mm)
        };
        // A alça continua alcançável mesmo quando o plano sai do enquadramento.
        let handle = egui::pos2(
            handle.x.clamp(rect.left() + 18., rect.right() - 18.),
            handle.y.clamp(rect.top() + 18., rect.bottom() - 18.),
        );
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let hovered = !dragging && pointer.is_some_and(|p| p.distance(handle) < 14.);
        if let Some(points) = patch(target) {
            sketch_scene::paint_patch(&painter, &points, theme::WORK_PLANE, hovered || dragging);
        }
        painter.line_segment([start, handle], egui::Stroke::new(2., theme::ACCENT));
        painter.circle(
            start,
            4.,
            egui::Color32::WHITE,
            egui::Stroke::new(1.5, theme::ACCENT),
        );
        painter.circle_filled(
            handle,
            7.,
            if hovered || dragging {
                theme::GREEN
            } else {
                theme::ACCENT
            },
        );
        let direction = if draft.offset < 0. { -axis } else { axis };
        let side = egui::vec2(-direction.y, direction.x) * 5.;
        painter.add(egui::Shape::convex_polygon(
            vec![
                handle + direction * 13.,
                handle + direction * 5. + side,
                handle + direction * 5. - side,
            ],
            theme::ACCENT,
            egui::Stroke::NONE,
        ));
        painter.text(
            handle + egui::vec2(16., -12.),
            egui::Align2::LEFT_BOTTOM,
            format!("{} mm", nator_drawing::format_mm(draft.offset)),
            egui::FontId::proportional(13.),
            theme::INK,
        );
        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if hovered
            && ui.input(|i| i.pointer.primary_pressed())
            && let Some(pointer) = pointer
        {
            draft.drag = Some(Drag {
                pointer,
                initial: draft.offset,
                axis,
                pixels_per_mm,
            });
        }
        if let (Some(drag), Some(pointer)) = (&draft.drag, pointer) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let value = drag.initial
                + f64::from((pointer - drag.pointer).dot(drag.axis) / drag.pixels_per_mm);
            let value = (value * 100.).round() / 100.;
            if value.is_finite() && value.abs() <= MAX_OFFSET {
                draft.offset = value;
            }
            ui.ctx().request_repaint();
        }
        let active = hovered || draft.drag.is_some();
        if !ui.input(|i| i.pointer.primary_down()) {
            draft.drag = None;
        }
        active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_planes_take_the_first_free_number() -> Result<(), Box<dyn std::error::Error>> {
        let mut document = Document::new("Nomes")?;
        let part = document.create_part("Peça".into())?;
        let name = |document: &Document| document.part(part).map(next_name).ok_or("Peça ausente");
        assert_eq!(name(&document)?, "Plano 1");
        let plane = WorkPlane {
            base: Plane::xy(),
            offset: 5.,
        };
        document.add_work_plane(part, "Plano 2".into(), plane)?;
        assert_eq!(name(&document)?, "Plano 1");
        document.add_work_plane(part, "Plano 1".into(), plane)?;
        assert_eq!(name(&document)?, "Plano 3");
        Ok(())
    }
}
