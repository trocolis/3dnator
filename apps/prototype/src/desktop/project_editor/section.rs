//! Corte de seção na área 3D (análise de corte): um plano deslocável esconde a parte do
//! modelo para onde aponta e mostra a seção como tampa hachurada. É um controle visual da
//! aba, como isolamento e transparência: não altera geometria, histórico nem desfazer.
//! A seleção ignora o que está cortado.
use super::{
    command_panel::{self as panel, Footer},
    icons::{self, Icon},
    *,
};
use nator_document::assembly::Pick;
use nator_geometry::{dot, norm, sub};
use nator_renderer::SectionPlane;

/// Planos principais pela normal.
const PLANES: [(&str, Point3); 3] = [
    ("XY", [0., 0., 1.]),
    ("XZ", [0., 1., 0.]),
    ("YZ", [1., 0., 0.]),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Base {
    Principal(usize),
    /// Plano de uma face escolhida, capturado no momento: não acompanha edições.
    Face {
        point: Point3,
        normal: Point3,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Section {
    pub base: Base,
    /// Deslocamento ao longo da normal, em mm.
    pub offset: f64,
    pub flip: bool,
    pub visible: bool,
}
impl Section {
    /// Plano no mundo; a parte para onde a normal aponta não é desenhada.
    pub fn plane(&self) -> SectionPlane {
        let (origin, normal) = match self.base {
            Base::Principal(i) => ([0.; 3], PLANES[i].1),
            Base::Face { point, normal } => {
                let length = norm(normal).max(1e-12);
                (point, normal.map(|v| v / length))
            }
        };
        let point = std::array::from_fn(|i| origin[i] + normal[i] * self.offset);
        SectionPlane {
            point,
            normal: if self.flip {
                normal.map(|v| -v)
            } else {
                normal
            },
        }
    }
    /// Um ponto escolhido continua selecionável se estiver do lado visível.
    pub fn keeps(&self, point: Point3) -> bool {
        let plane = self.plane();
        dot(plane.normal, sub(point, plane.point)) <= 1e-6
    }
}

/// Painel aberto: a seção em edição e a anterior, restaurada ao cancelar.
pub(super) struct SectionDraft {
    tab: u64,
    previous: Option<Section>,
    dragging: Option<(egui::Pos2, f64)>,
}

/// Primeiro ponto do lado visível do corte, na ordem de distância.
pub(super) fn first_visible(picks: Vec<Pick>, section: Option<Section>) -> Option<Pick> {
    match section.filter(|s| s.visible) {
        Some(section) => picks.into_iter().find(|p| section.keeps(p.point)),
        None => picks.into_iter().next(),
    }
}

impl Tab {
    /// Plano aplicado ao viewport, se o corte estiver visível.
    pub(super) fn section_plane(&self) -> Option<SectionPlane> {
        self.section.filter(|s| s.visible).map(|s| s.plane())
    }
}

impl Editor {
    pub(super) fn open_section(&mut self) {
        let tab = self.current().id;
        let previous = self.current().section;
        let bounds = self.current().scene_bounds;
        if self.current().section.is_none() {
            // Face escolhida, se houver; senão XY pelo centro do modelo.
            let section = match self.current().picked.clone() {
                Some(pick) => Section {
                    base: Base::Face {
                        point: pick.point,
                        normal: pick.normal,
                    },
                    offset: 0.,
                    flip: false,
                    visible: true,
                },
                None => Section {
                    base: Base::Principal(0),
                    offset: bounds.map(|b| b.center()[2]).unwrap_or(0.),
                    flip: false,
                    visible: true,
                },
            };
            self.current_mut().section = Some(section);
        }
        if let Some(section) = &mut self.current_mut().section {
            section.visible = true;
        }
        self.feature_edit = None;
        self.assembly_panel = None;
        self.section_edit = Some(SectionDraft {
            tab,
            previous,
            dragging: None,
        });
    }

    pub(super) fn section_panel(&mut self, context: &egui::Context) {
        let Some(draft) = &self.section_edit else {
            return;
        };
        if draft.tab != self.current().id {
            self.section_edit = None;
            return;
        }
        let Some(mut section) = self.current().section else {
            self.section_edit = None;
            return;
        };
        let picked = self.current().picked.clone();
        let outcome = panel::show(
            context,
            panel::Header {
                id: "section",
                icon: Icon::Section,
                title: text(T::SectionAnalysis),
            },
            self.canvas_rect,
            Footer::None,
            true,
            |ui| {
                panel::row(ui, text(T::PlaneField), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.;
                    for (i, (name, _)) in PLANES.iter().enumerate() {
                        if ui
                            .selectable_label(section.base == Base::Principal(i), *name)
                            .clicked()
                        {
                            section.base = Base::Principal(i);
                        }
                    }
                    let face = matches!(section.base, Base::Face { .. });
                    if ui
                        .add_enabled(
                            picked.is_some() || face,
                            egui::Button::new(text(T::PickedFace)).selected(face),
                        )
                        .on_hover_text(text(T::SectionFaceHint))
                        .clicked()
                        && let Some(pick) = &picked
                    {
                        section.base = Base::Face {
                            point: pick.point,
                            normal: pick.normal,
                        };
                        section.offset = 0.;
                    }
                });
                panel::row(ui, text(T::OffsetField), |ui| {
                    ui.add(
                        egui::DragValue::new(&mut section.offset)
                            .speed(0.5)
                            .suffix(" mm")
                            .max_decimals(3),
                    );
                    if icons::button(ui, Icon::Flip, text(T::FlipSide), false, section.flip)
                        .clicked()
                    {
                        section.flip = !section.flip;
                    }
                });
                ui.label(
                    egui::RichText::new(text(T::SectionHint))
                        .small()
                        .color(theme::MUTED),
                );
            },
        );
        self.current_mut().section = Some(section);
        if outcome.cancel {
            let previous = self.section_edit.take().and_then(|d| d.previous);
            self.current_mut().section = previous;
        } else if outcome.ok {
            self.section_edit = None;
        }
    }

    /// Seta do deslocamento no centro do modelo projetado no plano: arrastar muda a
    /// distância ao longo da normal. Devolve verdadeiro quando o ponteiro está nela.
    pub(super) fn section_gizmo(&mut self, ui: &egui::Ui, response: &egui::Response) -> bool {
        if self
            .section_edit
            .as_ref()
            .is_none_or(|d| d.tab != self.current().id)
        {
            return false;
        }
        let (Some(section), Some(bounds)) = (self.current().section, self.current().scene_bounds)
        else {
            return false;
        };
        let plane = section.plane();
        let rect = response.rect;
        let camera = self.viewport.camera.clone();
        let center = bounds.center();
        let anchor: Point3 = {
            let distance = dot(plane.normal, sub(center, plane.point));
            std::array::from_fn(|i| center[i] - plane.normal[i] * distance)
        };
        let project = |p: Point3| sketch_scene::project(&camera, bounds, rect, p);
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        // Contorno do plano: quadrado do tamanho do modelo, no próprio plano.
        let size = bounds.size().into_iter().fold(0_f64, f64::max) * 0.65;
        let helper = if plane.normal[0].abs() < 0.9 {
            [1., 0., 0.]
        } else {
            [0., 1., 0.]
        };
        let u = nator_geometry::cross(plane.normal, helper);
        let u = u.map(|v| v / norm(u));
        let v = nator_geometry::cross(plane.normal, u);
        let corners = [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.], [-1., -1.]].map(|[a, b]| {
            project(std::array::from_fn(|i| {
                anchor[i] + size * (a * u[i] + b * v[i])
            }))
        });
        if let Some(points) = corners.into_iter().collect::<Option<Vec<_>>>() {
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(196, 82, 64)),
            ));
        }
        let Some(start) = project(anchor) else {
            return false;
        };
        // Seta de comprimento fixo na tela, ao longo da normal (sentido da parte cortada).
        let Some(probe) = project(std::array::from_fn(|i| anchor[i] + plane.normal[i])) else {
            return false;
        };
        let per_mm = probe - start;
        let direction = if per_mm.length() > 1e-3 {
            per_mm.normalized()
        } else {
            egui::vec2(0., -1.)
        };
        let tip = start + direction * 70.;
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let hovered = pointer.is_some_and(|p| {
            let d = tip - start;
            let t = ((p - start).dot(d) / d.length_sq()).clamp(0., 1.);
            p.distance(start + d * t) < 8.
        });
        let draft = self.section_edit.as_mut();
        let dragging = draft.as_ref().is_some_and(|d| d.dragging.is_some());
        let color = egui::Color32::from_rgb(196, 82, 64);
        painter.line_segment(
            [start, tip],
            egui::Stroke::new(if hovered || dragging { 4. } else { 2.5 }, color),
        );
        let n = egui::vec2(-direction.y, direction.x);
        painter.add(egui::Shape::convex_polygon(
            vec![tip + direction * 11., tip + n * 6., tip - n * 6.],
            color,
            egui::Stroke::NONE,
        ));
        painter.circle(
            start,
            5.,
            egui::Color32::WHITE,
            egui::Stroke::new(2., color),
        );
        let Some(draft) = draft else {
            return false;
        };
        if ui.input(|i| i.pointer.primary_pressed())
            && hovered
            && let Some(p) = pointer
        {
            draft.dragging = Some((p, section.offset));
        }
        if let (Some((origin, offset)), Some(p)) = (draft.dragging, pointer)
            && per_mm.length_sq() > 1e-9
        {
            // O deslocamento cresce no sentido da normal sem inversão.
            let sign = if section.flip { -1. } else { 1. };
            let delta = f64::from((p - origin).dot(per_mm) / per_mm.length_sq());
            if let Some(current) = &mut self.current_mut().section {
                current.offset = ((offset + sign * delta) * 100.).round() / 100.;
            }
            ui.ctx().request_repaint();
        }
        let draft = self.section_edit.as_mut();
        if !ui.input(|i| i.pointer.primary_down())
            && let Some(draft) = draft
        {
            draft.dragging = None;
        }
        hovered || dragging
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_keeps_only_the_side_opposite_to_the_normal() {
        let mut section = Section {
            base: Base::Principal(1),
            offset: 5.,
            flip: false,
            visible: true,
        };
        let plane = section.plane();
        assert_eq!(plane.point, [0., 5., 0.]);
        assert!(section.keeps([10., 4., 0.]));
        assert!(!section.keeps([10., 6., 0.]));
        // Inverter troca o lado escondido sem mover o plano.
        section.flip = true;
        assert_eq!(section.plane().point, [0., 5., 0.]);
        assert!(section.keeps([10., 6., 0.]));
        // Face escolhida: a normal é normalizada e o deslocamento segue por ela.
        let face = Section {
            base: Base::Face {
                point: [1., 1., 1.],
                normal: [0., 0., 2.],
            },
            offset: 3.,
            flip: false,
            visible: true,
        };
        assert_eq!(face.plane().point, [1., 1., 4.]);
        assert_eq!(face.plane().normal, [0., 0., 1.]);
    }
}
