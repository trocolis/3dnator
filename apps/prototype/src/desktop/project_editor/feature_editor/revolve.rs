//! Eixo escolhido diretamente nas linhas coplanares do viewport. A captura usa
//! coordenadas do plano e a ocorrência da peça, inclusive linhas infinitas.
use super::*;
impl Editor {
    pub(in crate::desktop::project_editor) fn revolve_axis_overlay(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
    ) -> bool {
        let Some(draft) = self
            .feature_edit
            .as_ref()
            .filter(|d| d.tool == Tool::Revolve && d.field == Some(Field::RevolveAxis))
        else {
            return false;
        };
        let Some(part) = self.current().session.document().part(draft.part) else {
            return false;
        };
        let Ok(placement) = sketch_scene::part_placement(self.current(), draft.part) else {
            return false;
        };
        let plane = draft.profile_plane(part);
        let chosen = draft.revolve_axis;
        let pointer = response.hover_pos();
        let mut hovered: Option<(f32, SketchId, u64, egui::Pos2, egui::Pos2)> = None;
        for record in &part.sketches {
            if !part.shows_sketch(record)
                && chosen.is_none_or(|(id, _)| id != record.id)
                && draft.output.is_none()
            {
                continue;
            }
            if plane.is_some_and(|p| {
                nator_document::features::validate_coplanar(p, record.sketch.plane).is_err()
            }) {
                continue;
            }
            let frame = sketch_scene::Frame {
                camera: self.viewport.camera.clone(),
                bounds: self.display_bounds(),
                rect: response.rect,
                placement,
                plane: record.sketch.plane,
            };
            let Some(cursor) = pointer.and_then(|p| frame.cursor(p)) else {
                continue;
            };
            for entity in record
                .sketch
                .entities
                .iter()
                .filter(|e| matches!(e.curve, Curve::Line { .. }))
            {
                let Ok((a, b)) = record.sketch.line(entity.id) else {
                    continue;
                };
                let delta = nator_sketch::sub(b, a);
                let length = nator_sketch::dot(delta, delta);
                if length < 1e-18 {
                    continue;
                }
                let t = nator_sketch::dot(nator_sketch::sub(cursor, a), delta) / length;
                let t = if entity.infinite { t } else { t.clamp(0., 1.) };
                let q = nator_sketch::add(a, nator_sketch::scale(delta, t));
                let Some(screen) = frame.screen(q) else {
                    continue;
                };
                let distance = screen.distance(pointer.unwrap_or(screen));
                if distance > 10. || hovered.as_ref().is_some_and(|h| h.0 <= distance) {
                    continue;
                }
                let (a, b) = if entity.infinite {
                    (
                        nator_sketch::add(q, nator_sketch::scale(delta, -1000.)),
                        nator_sketch::add(q, nator_sketch::scale(delta, 1000.)),
                    )
                } else {
                    (a, b)
                };
                if let Some((a, b)) = frame.screen(a).zip(frame.screen(b)) {
                    hovered = Some((distance, record.id, entity.id, a, b));
                }
            }
        }
        if let Some((_, sketch, line, a, b)) = hovered {
            ui.painter()
                .with_clip_rect(response.rect)
                .line_segment([a, b], egui::Stroke::new(3., theme::ACCENT));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if response.clicked()
                && !self.busy()
                && let Some(draft) = self.feature_edit.as_mut()
            {
                draft.choose_revolve_axis(sketch, line);
            }
        }
        response.clicked()
    }
}
