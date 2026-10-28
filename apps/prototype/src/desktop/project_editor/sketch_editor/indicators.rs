//! Distintivos pequenos, afastados do ponto de captura e do cadeado contextual.
use super::*;
use nator_sketch::indicators::{Indicator, Kind as IndicatorKind};

pub(super) fn paint(
    ui: &egui::Ui,
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    markers: &[Indicator],
) {
    for (index, marker) in markers.iter().enumerate() {
        let Some(at) = frame.screen(marker.point) else {
            continue;
        };
        let column = markers[..index]
            .iter()
            .filter(|m| nator_sketch::distance(m.point, marker.point) < 1e-6)
            .count();
        let rect = egui::Rect::from_min_size(
            at + egui::vec2(12. + column as f32 * 23., 9.),
            egui::Vec2::splat(21.),
        );
        let (icon, label) = match marker.kind {
            IndicatorKind::Perpendicular => {
                (icons::Icon::ConstraintPerpendicular, T::Perpendicular)
            }
            IndicatorKind::Tangent => (icons::Icon::ConstraintTangent, T::Tangent),
            IndicatorKind::Midpoint => (icons::Icon::SnapMidpoint, T::Midpoint),
            IndicatorKind::Center => (icons::Icon::SnapCenter, T::SketchCenterIndicator),
        };
        painter.rect_filled(rect, 3., theme::PANEL);
        icons::paint(painter, rect.shrink(2.), icon, true);
        ui.interact(
            rect,
            egui::Id::new("sketch_relation_indicator").with((
                marker.kind as u8,
                marker.point[0].to_bits(),
                marker.point[1].to_bits(),
            )),
            egui::Sense::hover(),
        )
        .on_hover_text(text(label));
    }
}
