//! Padrão visual dos manipuladores de medida: haste fina e ponta de seta.
//! O alvo de arraste é a ponta, sem esfera que possa sugerir um vértice real.
use super::*;

pub(super) struct Arrow {
    pub origin: egui::Pos2,
    pub tip: egui::Pos2,
    pub direction: egui::Vec2,
}
impl Arrow {
    pub fn hovered(&self, ui: &egui::Ui) -> bool {
        ui.input(|i| i.pointer.hover_pos())
            .is_some_and(|p| p.distance(self.tip) < 16.)
    }
    pub fn started(&self, ui: &egui::Ui, response: &egui::Response) -> bool {
        // A ponta é um alvo explícito: iniciar ao pressionar permite ajustes
        // menores que o limiar de arraste usado para distinguir clique em arestas.
        response.contains_pointer()
            && ui.input(|i| i.pointer.primary_pressed())
            && ui
                .input(|i| i.pointer.press_origin())
                .is_some_and(|p| p.distance(self.tip) < 16.)
    }
    pub fn paint(&self, painter: &egui::Painter, active: bool) {
        let color = if active { theme::GREEN } else { theme::ACCENT };
        let axis = self.direction.normalized();
        let side = egui::vec2(-axis.y, axis.x) * 5.;
        painter.line_segment([self.origin, self.tip], egui::Stroke::new(1.5, color));
        painter.add(egui::Shape::convex_polygon(
            vec![
                self.tip,
                self.tip - axis * 12. + side,
                self.tip - axis * 12. - side,
            ],
            color,
            egui::Stroke::NONE,
        ));
    }
}
pub(super) struct Drag {
    pub pointer: egui::Pos2,
    pub axis: egui::Vec2,
    pub pixels_per_unit: f32,
    pub initial: f64,
}
impl Drag {
    pub fn value(&self, pointer: egui::Pos2) -> f64 {
        self.initial
            + f64::from((pointer - self.pointer).dot(self.axis) / self.pixels_per_unit.max(1e-5))
    }
}
