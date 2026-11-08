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
        // Os rótulos são posicionados no fim, sem cobrir outro já colocado nem as linhas.
        let labels = std::cell::RefCell::new(Vec::<Label>::new());
        let label = |at: egui::Pos2, text: String| {
            labels.borrow_mut().push(Label {
                at,
                text,
                color: None,
                outward: None,
            })
        };
        let lines = std::cell::RefCell::new(Vec::<[egui::Pos2; 2]>::new());
        // Decomposição nos eixos: o contorno (origem, cantos, destino) define o "dentro".
        let outline = |from: Point3, to: Point3| -> Vec<egui::Pos2> {
            axis_corners(from, to)
                .into_iter()
                .filter_map(&screen)
                .collect()
        };
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
                        if self.measure_tool.axes {
                            lines.borrow_mut().push([a, b]);
                            labels.borrow_mut().push(Label {
                                at: a.lerp(b, 0.5),
                                text: self.length(mm),
                                color: Some(theme::green()),
                                outward: Some(outward(a, b, &outline(from, to))),
                            });
                        } else {
                            label(a.lerp(b, 0.5), self.length(mm));
                        }
                    }
                }
                Reading::Axes { from, to } => {
                    // Um trecho por eixo, em X, Y e Z, com o valor absoluto de cada um. A cota
                    // fica no meio do trecho, do lado de fora do contorno, na cor do eixo.
                    let contour = outline(from, to);
                    let mut corner = from;
                    for (axis, color) in AXES.iter().enumerate() {
                        let mut next = corner;
                        next[axis] = to[axis];
                        let delta = (to[axis] - from[axis]).abs();
                        if let (Some(a), Some(b)) = (screen(corner), screen(next))
                            && norm(sub(next, corner)) > 1e-9
                        {
                            painter.line_segment([a, b], egui::Stroke::new(1.5, *color));
                            lines.borrow_mut().push([a, b]);
                            labels.borrow_mut().push(Label {
                                at: a.lerp(b, 0.5),
                                text: format!(
                                    "{} {}",
                                    ["ΔX", "ΔY", "ΔZ"][axis],
                                    self.length(delta)
                                ),
                                color: Some(*color),
                                outward: Some(outward(a, b, &contour)),
                            });
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
        let lines = lines.into_inner();
        let mut placed: Vec<egui::Rect> = Vec::new();
        for label in labels.into_inner() {
            let galley =
                painter.layout_no_wrap(label.text, egui::FontId::proportional(13.), theme::ink());
            let size = galley.size() + egui::vec2(10., 6.);
            let place = match label.outward {
                Some(n) => place_outside(label.at, n, size, &placed, &lines),
                None => {
                    let mut place =
                        egui::Rect::from_center_size(label.at - egui::vec2(0., 14.), size);
                    while placed.iter().any(|r| r.expand(2.).intersects(place)) {
                        place = place.translate(egui::vec2(0., size.y + 3.));
                    }
                    place
                }
            };
            placed.push(place);
            let border = match label.color {
                Some(color) => {
                    // Linha de chamada do meio do trecho até a cota.
                    let end = nearest_on_rect(place, label.at);
                    painter.line_segment([label.at, end], egui::Stroke::new(1., color));
                    painter.circle_filled(label.at, 2.5, color);
                    egui::Stroke::new(2., color)
                }
                None => egui::Stroke::new(1., theme::border()),
            };
            painter.rect_filled(place, 4., theme::panel());
            painter.rect_stroke(place, 4., border, egui::StrokeKind::Inside);
            painter.galley(place.center() - galley.size() / 2., galley, theme::ink());
        }
    }
}

/// Cota a desenhar: ponto de referência, texto e, nas decomposições, a cor do trecho e o
/// lado de fora (vetor unitário na tela) em que ela deve ficar.
struct Label {
    at: egui::Pos2,
    text: String,
    color: Option<egui::Color32>,
    outward: Option<egui::Vec2>,
}

/// Origem, cantos intermediários (X, depois Y) e destino da decomposição nos eixos.
fn axis_corners(from: Point3, to: Point3) -> Vec<Point3> {
    let mut corner = from;
    let mut points = vec![from];
    for axis in 0..3 {
        corner[axis] = to[axis];
        points.push(corner);
    }
    points
}

/// Normal do trecho `a`–`b` na tela, apontando para longe do centro do contorno.
fn outward(a: egui::Pos2, b: egui::Pos2, contour: &[egui::Pos2]) -> egui::Vec2 {
    let center = (!contour.is_empty()).then(|| {
        contour
            .iter()
            .fold(egui::Vec2::ZERO, |sum, p| sum + p.to_vec2())
            / contour.len() as f32
    });
    let away = center.map(|c| a.lerp(b, 0.5).to_vec2() - c);
    // Trecho visto de topo (quase um ponto na tela): só resta fugir do centro, ou subir.
    if (b - a).length() < 1e-3 {
        return away
            .filter(|v| v.length() > 1e-3)
            .map_or(egui::vec2(0., -1.), egui::Vec2::normalized);
    }
    let direction = (b - a).normalized();
    let normal = egui::vec2(-direction.y, direction.x);
    match away {
        Some(v) if normal.dot(v) < 0. => -normal,
        _ => normal,
    }
}

/// Retângulo da cota encostado no lado `normal` do ponto, afastado até não cobrir outra cota
/// nem atravessar nenhuma das linhas medidas.
fn place_outside(
    at: egui::Pos2,
    normal: egui::Vec2,
    size: egui::Vec2,
    placed: &[egui::Rect],
    lines: &[[egui::Pos2; 2]],
) -> egui::Rect {
    // Distância do centro até a borda do retângulo na direção da normal, mais a folga.
    let extent = normal.x.abs() * size.x / 2. + normal.y.abs() * size.y / 2.;
    let step = extent * 2. + 3.;
    let mut place = egui::Rect::from_center_size(at + normal * (extent + 10.), size);
    for _ in 0..12 {
        let blocked = placed.iter().any(|r| r.expand(2.).intersects(place))
            || lines.iter().any(|line| crosses(place.expand(2.), *line));
        if !blocked {
            break;
        }
        place = place.translate(normal * step);
    }
    place
}

/// O trecho passa por dentro do retângulo (amostrado ao longo do comprimento).
fn crosses(rect: egui::Rect, [a, b]: [egui::Pos2; 2]) -> bool {
    let samples = ((b - a).length() / 2.).clamp(2., 400.) as usize;
    (0..=samples).any(|i| rect.contains(a.lerp(b, i as f32 / samples as f32)))
}

/// Ponto da borda do retângulo mais próximo de `p`.
fn nearest_on_rect(rect: egui::Rect, p: egui::Pos2) -> egui::Pos2 {
    egui::pos2(
        p.x.clamp(rect.left(), rect.right()),
        p.y.clamp(rect.top(), rect.bottom()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Triângulo retângulo na tela: catetos horizontal e vertical e a hipotenusa.
    fn triangle() -> [egui::Pos2; 3] {
        [
            egui::pos2(100., 300.),
            egui::pos2(300., 300.),
            egui::pos2(300., 100.),
        ]
    }

    #[test]
    fn each_leg_label_goes_outside_the_triangle_without_crossing_any_line() {
        let [a, b, c] = triangle();
        let lines = [[a, b], [b, c], [a, c]];
        let size = egui::vec2(90., 20.);
        let mut placed = Vec::new();
        for [p, q] in lines {
            let n = outward(p, q, &[a, b, c]);
            let rect = place_outside(p.lerp(q, 0.5), n, size, &placed, &lines);
            // Fora: o centro da cota está mais longe do centroide que o meio do trecho.
            let centroid = (a.to_vec2() + b.to_vec2() + c.to_vec2()) / 3.;
            let middle = p.lerp(q, 0.5).to_vec2();
            assert!((rect.center().to_vec2() - centroid).length() > (middle - centroid).length());
            assert!(lines.iter().all(|line| !crosses(rect, *line)), "{rect:?}");
            assert!(placed.iter().all(|r: &egui::Rect| !r.intersects(rect)));
            placed.push(rect);
        }
        // Cateto de baixo: cota abaixo dele; cateto da direita: cota à direita.
        assert!(placed[0].top() > 300.);
        assert!(placed[1].left() > 300.);
    }

    #[test]
    fn a_leg_seen_end_on_still_gets_a_finite_direction() {
        let [a, _, c] = triangle();
        let n = outward(a, a, &[a, c]);
        assert!(n.x.is_finite() && n.y.is_finite());
        assert!((n.length() - 1.).abs() < 1e-4);
    }
}
