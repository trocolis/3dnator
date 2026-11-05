//! Ferramenta Medir, sem painel: cada clique escolhe um vértice, uma aresta ou uma face,
//! e a medida aparece desenhada na cena. Um alvo mostra a própria medida; dois mostram a
//! distância (ou o ângulo, se não forem paralelos); o duplo clique no segundo alvo
//! decompõe a distância nos eixos X, Y e Z. Esc limpa os alvos e, sem alvos, sai do Medir.
use super::*;
mod hover;
pub(super) mod query;
use nator_commands::measure::{Reading, Target, readings};
use nator_geometry::{Point3, norm, sub};

const TARGET: egui::Color32 = egui::Color32::from_rgb(214, 120, 24);
const AXES: [egui::Color32; 3] = [
    egui::Color32::from_rgb(205, 64, 64),
    egui::Color32::from_rgb(52, 150, 74),
    egui::Color32::from_rgb(52, 104, 205),
];

#[derive(Default)]
pub(super) struct State {
    /// Aba e revisão em que os alvos foram escolhidos: outra revisão os descarta.
    origin: Option<(u64, u64)>,
    targets: Vec<Target>,
    axes: bool,
    /// Duplo clique recebido enquanto o segundo alvo ainda era consultado.
    deferred: bool,
    hover: hover::Cache,
}

impl State {
    /// Um terceiro clique recomeça a medição a partir dele.
    pub(super) fn add(&mut self, tab: u64, revision: u64, target: Target) {
        if self.origin != Some((tab, revision)) || self.targets.len() >= 2 {
            self.targets.clear();
        }
        self.origin = Some((tab, revision));
        self.targets.push(target);
        self.axes = self.deferred && self.targets.len() == 2;
        self.deferred = false;
    }
    /// `pending`: o clique anterior ainda está sendo consultado no trabalhador.
    pub(super) fn expand(&mut self, pending: bool) {
        match self.targets.len() {
            2 if !pending => self.axes = true,
            1 if pending => self.deferred = true,
            _ => {}
        }
    }
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(super) fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }
    fn current(&self, tab: u64, revision: u64) -> Option<(&Target, Option<&Target>)> {
        (self.origin == Some((tab, revision)))
            .then(|| self.targets.first().map(|t| (t, self.targets.get(1))))
            .flatten()
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn readings(&self) -> Vec<Reading> {
        self.targets
            .first()
            .map(|first| readings(first, self.targets.get(1), self.axes))
            .unwrap_or_default()
    }
}

impl Editor {
    /// Alvos e cotas do Medir sobre a cena.
    pub(super) fn measure_overlay(&self, ui: &egui::Ui, rect: egui::Rect) {
        if !self.shell.measure_open {
            return;
        }
        let Some(bounds) = self.current().scene_bounds else {
            return;
        };
        let (tab, revision) = (self.current().id, self.current().session.revision());
        let Some((first, second)) = self.measure_tool.current(tab, revision) else {
            return;
        };
        let screen = |p: Point3| {
            let p =
                self.viewport
                    .camera
                    .project_world(p, bounds, f64::from(rect.aspect_ratio()))?;
            Some(
                rect.center()
                    + egui::vec2(
                        p[0] as f32 * rect.width() / 2.,
                        -p[1] as f32 * rect.height() / 2.,
                    ),
            )
        };
        let painter = ui.painter().with_clip_rect(rect);
        for target in [Some(first), second].into_iter().flatten() {
            match target {
                Target::Vertex { point } => {
                    if let Some(p) = screen(*point) {
                        painter.circle_filled(p, 4.5, TARGET);
                    }
                }
                Target::Edge { points, .. } => {
                    let line = points.iter().filter_map(|p| screen(*p)).collect::<Vec<_>>();
                    painter.add(egui::Shape::line(line, egui::Stroke::new(3., TARGET)));
                }
                Target::Face { point, .. } => {
                    if let Some(p) = screen(*point) {
                        painter.circle_stroke(p, 6., egui::Stroke::new(2., TARGET));
                    }
                }
            }
        }
        let settings = self.settings();
        // Os rótulos são posicionados no fim, descendo até não cobrir outro já colocado.
        let labels = std::cell::RefCell::new(Vec::<(egui::Pos2, String)>::new());
        let label = |at: egui::Pos2, text: String| labels.borrow_mut().push((at, text));
        for reading in readings(first, second, self.measure_tool.axes) {
            match reading {
                Reading::Point { at } => {
                    if let Some(p) = screen(at) {
                        label(
                            p,
                            format!(
                                "X {}  Y {}  Z {}",
                                self.mm(at[0]),
                                self.mm(at[1]),
                                self.mm(at[2])
                            ),
                        );
                    }
                }
                Reading::Length { mm, path } => {
                    let middle = path.len() / 2;
                    let at = if path.len() == 2 {
                        path.first()
                            .zip(path.last())
                            .map(|(a, b)| std::array::from_fn(|i| (a[i] + b[i]) / 2.))
                    } else {
                        path.get(middle).copied()
                    };
                    if let Some(p) = at.and_then(screen) {
                        label(p, self.length(mm));
                    }
                }
                Reading::Radius { mm, at, .. } => {
                    if let Some(p) = screen(at) {
                        label(
                            p + egui::vec2(0., 30.),
                            format!("R {}  Ø {}", self.length(mm), self.length(2. * mm)),
                        );
                    }
                }
                Reading::Area { mm2, at } => {
                    if let Some(p) = screen(at) {
                        label(p, settings.area_label(mm2));
                    }
                }
                Reading::Distance { mm, from, to } => {
                    if let (Some(a), Some(b)) = (screen(from), screen(to)) {
                        painter.line_segment([a, b], egui::Stroke::new(1.5, theme::green()));
                        painter.circle_filled(a, 3., theme::green());
                        painter.circle_filled(b, 3., theme::green());
                        label(a.lerp(b, 0.5), self.length(mm));
                    }
                }
                Reading::Axes { from, to } => {
                    // Um trecho por eixo, em X, Y e Z, com o valor absoluto de cada um.
                    let mut corner = from;
                    for (axis, color) in AXES.iter().enumerate() {
                        let mut next = corner;
                        next[axis] = to[axis];
                        let delta = (to[axis] - from[axis]).abs();
                        if let (Some(a), Some(b)) = (screen(corner), screen(next))
                            && norm(sub(next, corner)) > 1e-9
                        {
                            painter.line_segment([a, b], egui::Stroke::new(1.5, *color));
                            label(
                                a.lerp(b, 0.5) + egui::vec2(0., 28.),
                                format!("{} {}", ["ΔX", "ΔY", "ΔZ"][axis], self.length(delta)),
                            );
                        }
                        corner = next;
                    }
                }
                Reading::Angle { rad, at } => {
                    if let Some(p) = screen(at) {
                        let decimals = settings.angle_step_decimals().max(2);
                        label(
                            p,
                            format!(
                                "{}°",
                                nator_drawing::format_fixed(rad.to_degrees(), decimals)
                            ),
                        );
                    }
                }
            }
        }
        let mut placed: Vec<egui::Rect> = Vec::new();
        for (at, text) in labels.into_inner() {
            let galley =
                painter.layout_no_wrap(text, egui::FontId::proportional(13.), theme::ink());
            let size = galley.size() + egui::vec2(10., 6.);
            let mut place = egui::Rect::from_center_size(at - egui::vec2(0., 14.), size);
            while placed.iter().any(|r| r.expand(2.).intersects(place)) {
                place = place.translate(egui::vec2(0., size.y + 3.));
            }
            placed.push(place);
            painter.rect_filled(place, 4., theme::panel());
            painter.rect_stroke(
                place,
                4.,
                egui::Stroke::new(1., theme::border()),
                egui::StrokeKind::Inside,
            );
            painter.galley(place.center() - galley.size() / 2., galley, theme::ink());
        }
    }
}
