//! Raio, chanfro e espessura usam a seleção realçada como apoio visual.
//! A malha só posiciona a seta; o valor é aplicado pelo comando CAD exato.
use super::*;
#[derive(Default)]
pub(super) struct Interaction {
    pub input: distance_arrow::Input,
    drag: Option<distance_arrow::Drag>,
}
impl Editor {
    pub(in crate::desktop::project_editor) fn feature_measure_gizmo(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
    ) -> bool {
        let Some(mut d) = self.feature_edit.take() else {
            return false;
        };
        let consumed = self.draw_feature_measure(ui, response, &mut d);
        self.feature_edit = Some(d);
        consumed
    }
    fn draw_feature_measure(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        d: &mut Draft,
    ) -> bool {
        if !matches!(d.tool, Tool::Fillet | Tool::Chamfer | Tool::Shell) {
            return false;
        }
        let document = self.picking_document();
        let Some(base) = d.base.and_then(|id| document.body(id)).cloned() else {
            return false;
        };
        let Some(reference) = d.kept_topology.as_ref() else {
            return false;
        };
        let Ok(placement) = sketch_scene::part_placement(self.current(), d.part) else {
            return false;
        };
        let pose = placement.compose(base.placement).unwrap_or(placement);
        self.highlight_cache.native_edges.prepare(
            &base,
            d.references().filter_map(|r| {
                if let Kept::Edge(EdgeRef::Index { index, signature }) = r {
                    Some((*index, *signature))
                } else {
                    None
                }
            }),
            ui.ctx(),
        );
        let overlay = highlight::reference(&mut self.highlight_cache, &base, pose, reference);
        let origin = if let Some((edge, _)) = overlay.segments.get(overlay.segments.len() / 2) {
            std::array::from_fn(|i| (edge[0][i] + edge[1][i]) * 0.5)
        } else if let Some(triangle) = overlay.triangles.get(overlay.triangles.len() / 2) {
            std::array::from_fn(|i| (triangle[0][i] + triangle[1][i] + triangle[2][i]) / 3.)
        } else {
            return false;
        };
        let center = pose.apply(base.geometry.mesh().bounds().center());
        let delta = nator_geometry::sub(center, origin);
        let normal = if nator_geometry::norm(delta) > 1e-7 {
            delta.map(|v| v / nator_geometry::norm(delta))
        } else {
            pose.apply_vector([0., 0., 1.])
        };
        let camera = &self.viewport.camera;
        let bounds = self.display_bounds();
        let project = |point| sketch_scene::project(camera, bounds, response.rect, point);
        let Some(start) = project(origin) else {
            return false;
        };
        let probe = project(std::array::from_fn(|i| origin[i] + normal[i]))
            .map(|p| p - start)
            .unwrap_or_default();
        let (axis, scale) = if probe.length() > 0.5 {
            (probe.normalized(), probe.length())
        } else {
            (
                egui::vec2(0., -1.),
                response.rect.height() / bounds.size().into_iter().fold(1_f64, f64::max) as f32,
            )
        };
        let Some(part) = document.part(d.part) else {
            return false;
        };
        let length = |s: &str| {
            part.parameters
                .evaluate(s, nator_document::parameters::Dimension::Length)
                .ok()
                .filter(|v| *v > 0. && *v <= 1e7)
        };
        let value = length(&d.size).unwrap_or(1.);
        let tip = start + axis * (value as f32 * scale).max(36.);
        let tip = egui::pos2(
            tip.x
                .clamp(response.rect.left() + 18., response.rect.right() - 18.),
            tip.y
                .clamp(response.rect.top() + 18., response.rect.bottom() - 18.),
        );
        let arrow = distance_arrow::Arrow {
            origin: start,
            tip,
            direction: axis,
        };
        let over = arrow.hovered(ui);
        arrow.paint(
            &ui.painter().with_clip_rect(response.rect),
            over || d.measure.drag.is_some(),
        );
        #[cfg(all(test, feature = "occt"))]
        ui.ctx()
            .data_mut(|data| data.insert_temp(egui::Id::new("feature_measure_tip"), tip));
        if over {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if arrow.started(ui, response)
            && let Some(pointer) = ui.input(|i| i.pointer.press_origin())
        {
            d.measure.input.begin(value, pointer);
            d.measure.drag = Some(distance_arrow::Drag {
                pointer,
                axis,
                pixels_per_unit: scale,
                initial: value,
            });
        }
        let dragging = d.measure.drag.is_some();
        if let Some(drag) = &d.measure.drag
            && !d.measure.input.typing
            && let Some(pointer) = ui.input(|i| i.pointer.interact_pos())
        {
            let value = drag.value(pointer).clamp(1e-6, 1e7);
            d.size = format!("{value} mm");
        }
        let value = length(&d.size).unwrap_or(value);
        let label = match d.tool {
            Tool::Fillet => T::RadiusField,
            Tool::Shell => T::ThicknessField,
            _ => T::DistanceField,
        };
        let edit = d.measure.input.show(
            ui,
            "feature_measure",
            response.rect,
            value,
            dragging,
            label,
            length,
        );
        if let Some(value) = edit.value {
            d.size = format!("{value} mm");
        }
        if edit.finished || !ui.input(|i| i.pointer.primary_down()) {
            d.measure.drag = None;
        }
        over || dragging || edit.blocked
    }
}
