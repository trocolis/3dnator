//! Distintivos pequenos, afastados do ponto de captura e do cadeado contextual.
use super::*;
use nator_sketch::indicators::{Indicator, Kind as IndicatorKind};

/// Apoios de centro são apenas apresentação/captura: o losango vazio não cria
/// pontos nem junções no documento e desaparece ao voltar à seleção livre.
pub(super) fn centers(
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    points: impl IntoIterator<Item = Point2>,
) {
    let mut shown: Vec<Point2> = Vec::new();
    for point in points {
        if shown
            .iter()
            .any(|p| nator_sketch::distance(*p, point) < 1e-6)
        {
            continue;
        }
        shown.push(point);
        if let Some(at) = frame.screen(point) {
            painter.add(egui::Shape::closed_line(
                vec![
                    at + egui::vec2(0., -5.),
                    at + egui::vec2(5., 0.),
                    at + egui::vec2(0., 5.),
                    at + egui::vec2(-5., 0.),
                ],
                egui::Stroke::new(1.3, theme::GREEN),
            ));
            painter.circle_filled(at, 1.8, egui::Color32::BLACK);
        }
    }
}

/// Na seleção livre só o apoio selecionado revela suas relações. Durante o
/// desenho, as relações voltam a orientar as capturas e o fechamento do perfil.
pub(super) fn visible(
    markers: &[Indicator],
    drawing: bool,
    selected: Option<Point2>,
) -> Vec<Indicator> {
    markers
        .iter()
        .filter(|m| drawing || selected.is_some_and(|p| nator_sketch::distance(p, m.point) < 1e-6))
        .cloned()
        .collect()
}

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
