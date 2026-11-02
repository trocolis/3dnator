//! Marca de apoio do centro de círculos e arcos durante o desenho.
use super::*;

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
                egui::Stroke::new(1.3, theme::green()),
            ));
            painter.circle_filled(at, 1.8, egui::Color32::BLACK);
        }
    }
}
