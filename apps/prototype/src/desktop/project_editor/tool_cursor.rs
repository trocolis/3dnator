//! Distintivo da ferramenta ativa, junto ao ponteiro e somente sobre o canvas.
use super::*;
use icons::Icon;

impl Editor {
    fn active_tool_icon(&self) -> Option<Icon> {
        // A navegação usa somente o cursor do sistema, inclusive quando suspende
        // uma ferramenta: não mostrar também o distintivo do painel que ficou aberto.
        if self.shell.camera_mode.is_some() || self.shell.measure_open {
            return None;
        }
        if let Some(draft) = self
            .feature_edit
            .as_ref()
            .filter(|d| d.tab() == self.current().id)
        {
            return Some(draft.cursor_icon());
        }
        if self
            .paint_edit
            .as_ref()
            .is_some_and(|d| d.tab == self.current().id)
        {
            return Some(Icon::Paint);
        }
        if let Some(panel) = &self.assembly_panel {
            return Some(panel.cursor_icon());
        }
        if self.work_plane_edit.is_some() {
            return Some(Icon::OffsetPlane);
        }
        if self.section_edit.is_some() {
            return Some(Icon::Section);
        }
        if let Some(picker) = &self.current().plane_pick {
            return Some(match picker.purpose {
                sketch_scene::PlanePurpose::Sketch => Icon::Sketch,
                sketch_scene::PlanePurpose::OffsetBase => Icon::OffsetPlane,
            });
        }
        if self.sketch_text.is_some() {
            return Some(Icon::Label);
        }
        if self.image_edit.is_some() {
            return Some(Icon::Image);
        }
        if self.sketch_repair.is_some() {
            return Some(Icon::Check);
        }
        if let Some(work) = &self.current().sketch_edit {
            return work.cursor_icon();
        }
        if self.insert_part.is_some() || self.mesh_import.is_some() {
            return Some(Icon::Import);
        }
        if self.parameters_edit.is_some() {
            return Some(Icon::Parameters);
        }
        if self.print_profile_edit.is_some() {
            return Some(Icon::PrintProfile);
        }
        if self.calibration.is_some() {
            return Some(Icon::Calibrate);
        }
        if self.print_check.is_some() {
            return Some(Icon::PrintCheck);
        }
        if self.fit_check.is_some() {
            return Some(Icon::FitCheck);
        }
        if self.export_edit.is_some() {
            return Some(Icon::Export);
        }
        None
    }

    pub(super) fn tool_cursor(&self, ui: &egui::Ui, response: &egui::Response) {
        let Some(position) = response
            .hover_pos()
            .filter(|p| !view_cube::rect(response.rect).contains(*p))
        else {
            return;
        };
        let Some(icon) = self.active_tool_icon() else {
            return;
        };
        let painter = ui
            .ctx()
            .layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                ui.id().with("active_tool_cursor"),
            ))
            .with_clip_rect(response.rect.intersect(ui.clip_rect()));
        icons::paint(
            &painter,
            egui::Rect::from_min_size(position + egui::vec2(16., -28.), egui::Vec2::splat(22.)),
            icon,
            true,
        );
    }
}
