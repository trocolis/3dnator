//! Traço dos ícones: grade de 24 unidades, cantos e pontas arredondados, contorno
//! escuro e uma única cor de ação. Os desenhos evitam detalhes menores que 1,1
//! unidade para continuarem legíveis nos botões de 16 a 27 pontos.
use super::super::{egui, theme};
use super::Icon;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

const MAIN: f32 = 2.;
const DETAIL: f32 = 1.45;

/// Elementos geométricos mantêm proporção e traço em qualquer tamanho de botão.
struct Pen<'a> {
    painter: &'a egui::Painter,
    rect: egui::Rect,
    ink: egui::Color32,
    accent: egui::Color32,
    surface: egui::Color32,
    bright: egui::Color32,
    paper: egui::Color32,
    positive: egui::Color32,
    warning: egui::Color32,
    construction: egui::Color32,
    reference: egui::Color32,
    danger: egui::Color32,
    dark: bool,
    enabled: bool,
}

impl Pen<'_> {
    fn p(&self, x: f32, y: f32) -> egui::Pos2 {
        self.rect.min + egui::vec2(x, y) * (self.rect.width() / 24.)
    }

    fn px(&self, units: f32) -> f32 {
        (self.rect.width() / 24. * units).max(0.8)
    }

    /// Linha com pontas e junções arredondadas: o egui desenha junções em
    /// esquadria, então os nós recebem um pequeno disco da mesma cor.
    fn line(&self, points: &[[f32; 2]], color: egui::Color32, width: f32) {
        let mapped: Vec<egui::Pos2> = points.iter().map(|q| self.p(q[0], q[1])).collect();
        let stroke = egui::Stroke::new(self.px(width), color);
        self.painter.add(egui::Shape::line(mapped.clone(), stroke));
        let radius = self.px(width) / 2.;
        for point in &mapped {
            self.painter.circle_filled(*point, radius, color);
        }
    }

    fn stroke(&self, points: &[[f32; 2]], color: egui::Color32) {
        self.line(points, color, MAIN);
    }

    fn detail(&self, points: &[[f32; 2]], color: egui::Color32) {
        self.line(points, color, DETAIL);
    }

    /// Preenchimento de polígonos convexos. Formas côncavas são decompostas
    /// em partes convexas na própria definição do ícone.
    fn fill(&self, points: &[[f32; 2]], color: egui::Color32) {
        let mapped: Vec<egui::Pos2> = points.iter().map(|q| self.p(q[0], q[1])).collect();
        self.painter.add(egui::Shape::convex_polygon(
            mapped,
            color,
            egui::Stroke::new(0., egui::Color32::TRANSPARENT),
        ));
    }

    /// Preenchimento mais contorno fechado.
    fn polygon(&self, points: &[[f32; 2]], fill: egui::Color32, edge: egui::Color32, width: f32) {
        self.fill(points, fill);
        let mut closed = points.to_vec();
        if let Some(first) = points.first() {
            closed.push(*first);
        }
        self.line(&closed, edge, width);
    }

    fn disc(&self, x: f32, y: f32, radius: f32, color: egui::Color32) {
        self.painter
            .circle_filled(self.p(x, y), radius * self.rect.width() / 24., color);
    }

    fn ring(&self, x: f32, y: f32, radius: f32, color: egui::Color32, width: f32) {
        self.painter.circle_stroke(
            self.p(x, y),
            radius * self.rect.width() / 24.,
            egui::Stroke::new(self.px(width), color),
        );
    }

    fn arc(&self, cx: f32, cy: f32, radius: f32, from: f32, to: f32, color: egui::Color32) {
        let steps = (((to - from).abs() * 7.).ceil() as usize).max(7);
        let points: Vec<[f32; 2]> = (0..=steps)
            .map(|index| {
                let angle = from + (to - from) * index as f32 / steps as f32;
                [cx + radius * angle.cos(), cy + radius * angle.sin()]
            })
            .collect();
        self.line(&points, color, MAIN);
    }

    fn ellipse(&self, cx: f32, cy: f32, rx: f32, ry: f32, color: egui::Color32, width: f32) {
        self.line(&ellipse_points(cx, cy, rx, ry), color, width);
    }

    fn ellipse_fill(&self, cx: f32, cy: f32, rx: f32, ry: f32, color: egui::Color32) {
        self.fill(&ellipse_points(cx, cy, rx, ry), color);
    }

    fn bezier(&self, a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2], color: egui::Color32) {
        let points: Vec<[f32; 2]> = (0..=18)
            .map(|index| {
                let t = index as f32 / 18.;
                let u = 1. - t;
                [
                    u * u * u * a[0]
                        + 3. * u * u * t * b[0]
                        + 3. * u * t * t * c[0]
                        + t * t * t * d[0],
                    u * u * u * a[1]
                        + 3. * u * u * t * b[1]
                        + 3. * u * t * t * c[1]
                        + t * t * t * d[1],
                ]
            })
            .collect();
        self.stroke(&points, color);
    }

    fn dashed(&self, a: [f32; 2], b: [f32; 2], color: egui::Color32, width: f32) {
        self.dashes(a, b, color, width, 2.4, 3.8);
    }
    /// Traços longos e vão curto: o padrão fica evidente mesmo no ícone de 16 pontos.
    fn long_dashed(&self, a: [f32; 2], b: [f32; 2], color: egui::Color32, width: f32) {
        self.dashes(a, b, color, width, 4.2, 6.6);
    }
    fn dashes(
        &self,
        a: [f32; 2],
        b: [f32; 2],
        color: egui::Color32,
        width: f32,
        dash: f32,
        period: f32,
    ) {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.01 {
            return;
        }
        let (ux, uy) = (dx / len, dy / len);
        let mut start = 0.;
        while start < len {
            let end = (start + dash).min(len);
            self.painter.add(egui::Shape::line(
                vec![
                    self.p(a[0] + ux * start, a[1] + uy * start),
                    self.p(a[0] + ux * end, a[1] + uy * end),
                ],
                egui::Stroke::new(self.px(width), color),
            ));
            start += period;
        }
    }

    /// Ponta de seta preenchida; a haste termina antes dela para não vazar.
    fn head(&self, tip: [f32; 2], direction: [f32; 2], color: egui::Color32) {
        let len = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
        if len < 0.01 {
            return;
        }
        let (ux, uy) = (direction[0] / len, direction[1] / len);
        let base = [tip[0] - ux * 4.4, tip[1] - uy * 4.4];
        let normal = [-uy * 2.4, ux * 2.4];
        self.fill(
            &[
                tip,
                [base[0] + normal[0], base[1] + normal[1]],
                [base[0] - normal[0], base[1] - normal[1]],
            ],
            color,
        );
    }

    fn arrow(&self, from: [f32; 2], to: [f32; 2], color: egui::Color32) {
        let dx = to[0] - from[0];
        let dy = to[1] - from[1];
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.01 {
            return;
        }
        let (ux, uy) = (dx / len, dy / len);
        self.stroke(&[from, [to[0] - ux * 3.4, to[1] - uy * 3.4]], color);
        self.head(to, [ux, uy], color);
    }

    fn cube(&self) {
        let top = [[12., 2.8], [21., 7.6], [12., 12.4], [3., 7.6]];
        let left = [[3., 7.6], [12., 12.4], [12., 21.4], [3., 16.6]];
        let right = [[12., 12.4], [21., 7.6], [21., 16.6], [12., 21.4]];
        self.fill(&left, self.paper);
        self.fill(&right, self.surface);
        self.fill(&top, self.bright);
        self.stroke(
            &[
                [3., 7.6],
                [12., 2.8],
                [21., 7.6],
                [21., 16.6],
                [12., 21.4],
                [3., 16.6],
                [3., 7.6],
            ],
            self.ink,
        );
        self.detail(&[[3., 7.6], [12., 12.4], [21., 7.6]], self.ink);
        self.detail(&[[12., 12.4], [12., 21.4]], self.ink);
    }

    fn document(&self) {
        self.polygon(
            &[[5.5, 2.5], [14.5, 2.5], [19., 7.], [19., 21.5], [5.5, 21.5]],
            self.paper,
            self.ink,
            MAIN,
        );
        self.detail(&[[14.5, 2.5], [14.5, 7.], [19., 7.]], self.ink);
    }

    fn folder(&self) {
        let tab = [[3., 6.], [9., 6.], [11., 8.], [3., 8.]];
        let body = [[3., 8.], [21., 8.], [21., 20.5], [3., 20.5]];
        self.fill(&tab, self.surface);
        self.fill(&body, self.surface);
        self.stroke(
            &[
                [3., 6.],
                [9., 6.],
                [11., 8.],
                [21., 8.],
                [21., 20.5],
                [3., 20.5],
                [3., 6.],
            ],
            self.ink,
        );
    }

    /// Lápis genérico entre a ponta e o topo, com corpo e ponta coloridos.
    fn pencil(
        &self,
        tip: [f32; 2],
        end: [f32; 2],
        half: f32,
        body: egui::Color32,
        tip_color: egui::Color32,
    ) {
        let dx = end[0] - tip[0];
        let dy = end[1] - tip[1];
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.01 {
            return;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (-uy * half, ux * half);
        let start = [tip[0] + ux * half * 2.6, tip[1] + uy * half * 2.6];
        let a = [start[0] + nx, start[1] + ny];
        let b = [start[0] - nx, start[1] - ny];
        let c = [end[0] + nx, end[1] + ny];
        let d = [end[0] - nx, end[1] - ny];
        self.fill(&[a, c, d, b], body);
        self.stroke(&[a, c, d, b, a], self.ink);
        self.fill(&[tip, a, b], tip_color);
        self.stroke(&[a, tip, b], self.ink);
    }

    fn badge_plus(&self, x: f32, y: f32, radius: f32) {
        self.disc(x, y, radius, self.accent);
        self.line(
            &[[x - radius * 0.55, y], [x + radius * 0.55, y]],
            egui::Color32::WHITE,
            2.,
        );
        self.line(
            &[[x, y - radius * 0.55], [x, y + radius * 0.55]],
            egui::Color32::WHITE,
            2.,
        );
    }
}

fn ellipse_points(cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<[f32; 2]> {
    (0..=40)
        .map(|index| {
            let angle = index as f32 / 40. * TAU;
            [cx + rx * angle.cos(), cy + ry * angle.sin()]
        })
        .collect()
}

pub(super) fn paint(
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: Icon,
    enabled: bool,
    ink: egui::Color32,
    dark: bool,
) {
    if rect.width() < 0.5 || rect.height() < 0.5 {
        return;
    }
    // Ferramentas e indicadores do rascunho compartilham contorno preto e ação
    // verde. O mesmo desenho/paleta aparece na faixa, nos painéis e no canvas.
    let constraint = matches!(
        icon,
        Icon::ConstraintHorizontalVertical
            | Icon::ConstraintCoincident
            | Icon::ConstraintTangent
            | Icon::ConstraintEqual
            | Icon::ConstraintParallel
            | Icon::ConstraintPerpendicular
            | Icon::ConstraintFix
            | Icon::ConstraintConcentric
            | Icon::ConstraintCollinear
            | Icon::ConstraintSymmetry
            | Icon::ConstraintPolygon
            | Icon::ConstraintManager
    );
    let sketch = matches!(
        icon,
        Icon::Line
            | Icon::LineSymmetric
            | Icon::LineConstruction
            | Icon::LineInfinite
            | Icon::Rectangle
            | Icon::RectangleCenter
            | Icon::RectangleInclined
            | Icon::Circle
            | Icon::CircleDiameter
            | Icon::CircleTangent
            | Icon::Arc
            | Icon::Trim
            | Icon::SketchOffset
            | Icon::SketchProject
            | Icon::Paint
            | Icon::Image
            | Icon::SketchFillet
            | Icon::SketchChamfer
            | Icon::SketchMirror
            | Icon::CircularPattern
            | Icon::RectangularPattern
            | Icon::Dimension
            | Icon::SnapMidpoint
            | Icon::SnapCenter
            | Icon::LockOpen
    ) || constraint;
    let disabled = egui::Color32::from_gray(if dark { 112 } else { 165 });
    let accent = if !enabled {
        disabled
    } else if sketch {
        if dark {
            egui::Color32::from_rgb(110, 204, 154)
        } else {
            theme::green()
        }
    } else if dark {
        egui::Color32::from_rgb(124, 188, 248)
    } else {
        theme::accent()
    };
    let ink = if !enabled {
        disabled
    } else if sketch && !dark {
        egui::Color32::BLACK
    } else {
        ink
    };
    let surface = if !enabled {
        egui::Color32::from_gray(if dark { 72 } else { 226 })
    } else if sketch {
        if dark {
            egui::Color32::from_rgb(44, 82, 68)
        } else {
            egui::Color32::from_rgb(223, 242, 231)
        }
    } else if dark {
        egui::Color32::from_rgb(50, 68, 92)
    } else {
        egui::Color32::from_rgb(232, 240, 249)
    };
    let paper = if !enabled {
        egui::Color32::from_gray(if dark { 60 } else { 242 })
    } else if dark {
        egui::Color32::from_rgb(38, 46, 59)
    } else {
        egui::Color32::WHITE
    };
    let bright = if enabled {
        accent.gamma_multiply(if dark { 0.55 } else { 0.32 })
    } else {
        surface
    };
    let positive = if enabled {
        if dark {
            egui::Color32::from_rgb(120, 210, 160)
        } else {
            theme::green()
        }
    } else {
        disabled
    };
    let construction = if enabled {
        theme::construction()
    } else {
        disabled
    };
    let warning = if enabled {
        if dark {
            egui::Color32::from_rgb(230, 180, 90)
        } else {
            theme::warning()
        }
    } else {
        disabled
    };
    let reference = if enabled {
        if dark {
            egui::Color32::from_rgb(178, 148, 232)
        } else {
            theme::reference()
        }
    } else {
        disabled
    };
    let danger = if enabled {
        if dark {
            egui::Color32::from_rgb(226, 122, 118)
        } else {
            theme::danger()
        }
    } else {
        disabled
    };
    let pen = Pen {
        painter,
        rect,
        ink,
        accent,
        surface,
        bright,
        paper,
        positive,
        warning,
        construction,
        reference,
        danger,
        dark,
        enabled,
    };
    draw(&pen, icon);
}

fn draw(p: &Pen, icon: Icon) {
    match icon {
        // Arquivos e aplicativo.
        Icon::New => {
            p.document();
            p.badge_plus(17., 17.5, 4.1);
        }
        Icon::Open => {
            p.polygon(
                &[[7., 2.5], [17., 2.5], [17., 15.5], [7., 15.5]],
                p.paper,
                p.ink,
                DETAIL,
            );
            p.detail(&[[9., 6.5], [15., 6.5]], p.accent);
            p.detail(&[[9., 9.5], [13., 9.5]], p.accent);
            p.folder();
        }
        Icon::Save => {
            p.polygon(
                &[
                    [4., 3.],
                    [17., 3.],
                    [21., 7.],
                    [21., 21.],
                    [3., 21.],
                    [3., 3.],
                ],
                p.surface,
                p.ink,
                MAIN,
            );
            p.polygon(
                &[[10., 3.], [17., 3.], [17., 9.], [10., 9.]],
                p.bright,
                p.ink,
                DETAIL,
            );
            p.polygon(
                &[[12.5, 3.], [14.5, 3.], [14.5, 6.], [12.5, 6.]],
                p.surface,
                egui::Color32::TRANSPARENT,
                DETAIL,
            );
            p.polygon(
                &[[6., 13.], [18., 13.], [18., 21.], [6., 21.]],
                p.paper,
                p.ink,
                DETAIL,
            );
            p.detail(&[[8., 16.], [16., 16.]], p.accent);
            p.detail(&[[8., 18.5], [13., 18.5]], p.accent);
        }
        Icon::SaveAs => {
            p.document();
            p.pencil([12., 21.5], [21., 11.], 1.5, p.surface, p.accent);
        }
        Icon::Settings => {
            for tooth in 0..8 {
                let angle = tooth as f32 * TAU / 8.;
                let (s, c) = angle.sin_cos();
                p.line(
                    &[
                        [12. + 6.6 * c, 12. + 6.6 * s],
                        [12. + 9.6 * c, 12. + 9.6 * s],
                    ],
                    p.ink,
                    2.4,
                );
            }
            p.ring(12., 12., 6.2, p.ink, 2.2);
            p.disc(12., 12., 2.2, p.accent);
        }
        Icon::Undo | Icon::Redo => {
            // Seta curva: chevron na ponta, arco de 180° e cauda curta.
            let mirror = |x: f32| if icon == Icon::Redo { 24. - x } else { x };
            let mut path: Vec<[f32; 2]> = vec![[4., 9.], [14.5, 9.]];
            path.extend((0..=16).map(|index| {
                let angle = -FRAC_PI_2 + PI * index as f32 / 16.;
                [14.5 + 5.5 * angle.cos(), 14.5 + 5.5 * angle.sin()]
            }));
            path.push([11., 20.]);
            for point in &mut path {
                point[0] = mirror(point[0]);
            }
            p.stroke(&path, p.ink);
            p.line(
                &[[mirror(9.), 14.], [mirror(4.), 9.], [mirror(9.), 4.]],
                p.accent,
                MAIN,
            );
        }
        Icon::Import | Icon::Export => {
            p.stroke(&[[3., 14.], [3., 21.], [21., 21.], [21., 14.]], p.ink);
            if icon == Icon::Import {
                p.arrow([12., 2.], [12., 16.], p.accent);
            } else {
                p.arrow([12., 16.], [12., 2.], p.accent);
            }
        }
        Icon::Search => {
            p.ring(10.5, 10.5, 6.2, p.ink, 2.2);
            p.line(&[[15.2, 15.2], [21., 21.]], p.ink, 2.4);
        }
        Icon::Home => {
            p.stroke(&[[3., 11.], [12., 3.], [21., 11.]], p.ink);
            p.stroke(&[[5., 10.], [5., 21.], [19., 21.], [19., 10.]], p.ink);
            p.polygon(
                &[[10., 21.], [10., 15.], [14., 15.], [14., 21.]],
                p.bright,
                p.accent,
                DETAIL,
            );
        }
        Icon::PanCamera => {
            // Mão aberta: quatro dedos e o polegar saem de uma palma única.
            for (x, top) in [(6.5, 8.), (10., 4.5), (13.5, 4.), (17., 6.)] {
                p.stroke(&[[x, 13.], [x, top]], p.ink);
            }
            p.stroke(&[[6.5, 15.], [3.5, 12.5]], p.ink);
            p.polygon(
                &[
                    [6.5, 12.],
                    [17., 12.],
                    [18.5, 15.],
                    [16.5, 21.],
                    [9., 21.],
                    [6.5, 16.5],
                ],
                p.bright,
                p.accent,
                MAIN,
            );
        }
        Icon::OrbitCamera => {
            // Órbita inclinada em volta do alvo, com o sentido marcado na ponta.
            let points: Vec<[f32; 2]> = (0..=32)
                .map(|index| {
                    let angle = -0.5 + 5.2 * index as f32 / 32.;
                    [12. + 9.5 * angle.cos(), 12. + 4.5 * angle.sin()]
                })
                .collect();
            p.stroke(&points, p.ink);
            if let [.., before, tip] = points.as_slice() {
                p.head(*tip, [tip[0] - before[0], tip[1] - before[1]], p.accent);
            }
            p.disc(12., 12., 3.2, p.accent);
        }
        Icon::CollapseUp => p.stroke(&[[6., 15.], [12., 9.], [18., 15.]], p.ink),
        Icon::ExpandDown => p.stroke(&[[6., 9.], [12., 15.], [18., 9.]], p.ink),
        Icon::Extrude => {
            p.cube();
            p.arrow([12., 12.], [12., 1.8], p.accent);
        }
        Icon::Revolve => {
            p.dashed([12., 2.], [12., 22.], p.ink, 1.6);
            p.polygon(
                &[[4., 6.], [8., 6.], [8., 18.], [4., 18.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.arc(12., 12., 8., -FRAC_PI_2, FRAC_PI_2, p.accent);
            p.head([12., 20.], [-1., 0.], p.accent);
        }
        Icon::Loft => {
            p.ellipse_fill(12., 19., 8.5, 2.6, p.surface);
            p.ellipse(12., 19., 8.5, 2.6, p.ink, DETAIL);
            p.ellipse_fill(12., 5.5, 4.5, 2., p.bright);
            p.ellipse(12., 5.5, 4.5, 2., p.accent, MAIN);
            p.ellipse(12., 12.5, 6.5, 2.2, p.ink, 1.1);
            p.detail(&[[3.5, 19.], [7.5, 5.5]], p.ink);
            p.detail(&[[20.5, 19.], [16.5, 5.5]], p.ink);
        }
        Icon::Sweep => {
            p.polygon(
                &[[3., 13.], [8., 13.], [8., 18.], [3., 18.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.bezier([8., 15.5], [11., 17.5], [17., 14.], [20., 7.], p.accent);
            p.head([20.5, 5.6], [0.55, -0.85], p.accent);
        }
        Icon::Label => {
            p.polygon(
                &[[3., 4.], [14., 4.], [21., 12.], [14., 20.], [3., 20.]],
                p.paper,
                p.ink,
                MAIN,
            );
            p.disc(7.5, 8.5, 1.6, p.paper);
            p.ring(7.5, 8.5, 1.6, p.accent, DETAIL);
            p.detail(&[[10., 11.], [16., 11.]], p.accent);
            p.detail(&[[10., 15.], [14., 15.]], p.accent);
        }
        Icon::Hole => {
            let top = [[12., 3.], [21., 7.], [12., 11.], [3., 7.]];
            let left = [[3., 7.], [12., 11.], [12., 16.], [3., 12.]];
            let right = [[12., 11.], [21., 7.], [21., 12.], [12., 16.]];
            p.fill(&left, p.paper);
            p.fill(&right, p.surface);
            p.fill(&top, p.bright);
            p.stroke(
                &[
                    [3., 7.],
                    [12., 3.],
                    [21., 7.],
                    [21., 12.],
                    [12., 16.],
                    [3., 12.],
                    [3., 7.],
                ],
                p.ink,
            );
            p.detail(&[[3., 7.], [12., 11.], [21., 7.]], p.ink);
            p.detail(&[[12., 11.], [12., 16.]], p.ink);
            p.ellipse_fill(12., 7., 3.6, 1.8, p.paper);
            p.ellipse(12., 7., 3.6, 1.8, p.accent, DETAIL);
        }
        Icon::Fillet => {
            p.stroke(&[[5., 21.], [5., 11.]], p.ink);
            p.stroke(&[[11., 5.], [21., 5.]], p.ink);
            p.arc(11., 11., 6., PI, PI + FRAC_PI_2, p.accent);
        }
        Icon::Chamfer => {
            p.stroke(&[[5., 21.], [5., 11.]], p.ink);
            p.stroke(&[[11., 5.], [21., 5.]], p.ink);
            p.dashed([5., 5.], [5., 11.], p.ink, 1.1);
            p.dashed([5., 5.], [11., 5.], p.ink, 1.1);
            p.line(&[[5., 11.], [11., 5.]], p.accent, 2.2);
        }
        Icon::Shell => {
            p.fill(&[[3., 8.], [12., 13.], [12., 21.], [3., 16.]], p.paper);
            p.fill(&[[12., 13.], [21., 8.], [21., 16.], [12., 21.]], p.surface);
            p.fill(&[[12., 3.], [21., 8.], [12., 13.], [3., 8.]], p.bright);
            p.fill(&[[12., 5.8], [18.2, 8.8], [12., 11.8], [5.8, 8.8]], p.paper);
            p.stroke(
                &[
                    [3., 8.],
                    [12., 3.],
                    [21., 8.],
                    [21., 16.],
                    [12., 21.],
                    [3., 16.],
                    [3., 8.],
                ],
                p.ink,
            );
            p.detail(&[[3., 8.], [12., 13.], [21., 8.]], p.ink);
            p.detail(&[[12., 13.], [12., 21.]], p.ink);
            p.polygon(
                &[[12., 5.8], [18.2, 8.8], [12., 11.8], [5.8, 8.8]],
                p.paper,
                p.accent,
                DETAIL,
            );
        }
        Icon::Boolean => {
            p.polygon(
                &[[3., 4.], [13., 4.], [13., 14.], [3., 14.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.polygon(
                &[[11., 10.], [21., 10.], [21., 20.], [11., 20.]],
                p.bright,
                p.ink,
                MAIN,
            );
            p.fill(&[[11., 10.], [13., 10.], [13., 14.], [11., 14.]], p.accent);
        }
        Icon::Calibrate => {
            // Paquímetro medindo uma peça.
            p.stroke(&[[3., 6.], [21., 6.]], p.ink);
            p.stroke(&[[5., 6.], [5., 13.]], p.ink);
            p.stroke(&[[13., 6.], [13., 13.]], p.ink);
            p.polygon(
                &[[6., 15.], [12., 15.], [12., 21.], [6., 21.]],
                p.surface,
                p.accent,
                MAIN,
            );
        }
        Icon::PrintCheck => {
            // Camadas depositadas, com a verificação concluída sobre elas.
            p.stroke(&[[3., 21.], [21., 21.]], p.ink);
            p.stroke(&[[3., 17.], [21., 17.]], p.ink);
            p.stroke(&[[6., 13.], [18., 13.]], p.accent);
            p.line(&[[7., 7.], [11., 10.5], [18., 3.]], p.positive, 2.4);
        }
        Icon::FitCheck => {
            // Dois blocos encaixados, com a verificação concluída sobre eles.
            p.polygon(
                &[
                    [3., 14.],
                    [9., 14.],
                    [9., 11.],
                    [15., 11.],
                    [15., 14.],
                    [21., 14.],
                    [21., 21.],
                    [3., 21.],
                ],
                p.surface,
                p.ink,
                MAIN,
            );
            p.polygon(
                &[[10., 12.], [14., 12.], [14., 16.], [10., 16.]],
                p.bright,
                p.accent,
                MAIN,
            );
            p.line(&[[9., 6.], [12., 9.], [18., 3.]], p.positive, 2.4);
        }
        Icon::ComponentCut => {
            // Componente descendo para o recorte de mesma forma.
            p.polygon(
                &[
                    [2., 11.],
                    [7., 11.],
                    [7., 20.],
                    [17., 20.],
                    [17., 11.],
                    [22., 11.],
                    [22., 22.],
                    [2., 22.],
                ],
                p.surface,
                p.ink,
                MAIN,
            );
            p.polygon(
                &[[9., 3.], [15., 3.], [15., 10.], [9., 10.]],
                p.bright,
                p.accent,
                MAIN,
            );
            p.stroke(&[[12., 12.], [12., 17.]], p.accent);
            p.stroke(&[[10., 15.], [12., 17.], [14., 15.]], p.accent);
        }
        Icon::Lip => {
            // Corte da parede: a borda sobe em degrau para encaixar a tampa.
            p.polygon(
                &[
                    [5., 22.],
                    [5., 11.],
                    [10., 11.],
                    [10., 5.],
                    [14., 5.],
                    [14., 22.],
                ],
                p.surface,
                p.ink,
                MAIN,
            );
            p.polygon(
                &[
                    [3., 3.],
                    [21., 3.],
                    [21., 9.],
                    [17., 9.],
                    [17., 5.],
                    [3., 5.],
                ],
                p.bright,
                p.accent,
                MAIN,
            );
        }
        Icon::ScrewBoss => {
            // Torre com furo central sobre o piso.
            p.polygon(
                &[[2., 19.], [22., 19.], [22., 22.], [2., 22.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.polygon(
                &[[7., 5.], [17., 5.], [17., 19.], [7., 19.]],
                p.bright,
                p.ink,
                MAIN,
            );
            p.fill(
                &[[10.5, 5.], [13.5, 5.], [13.5, 14.], [10.5, 14.]],
                p.accent,
            );
        }
        Icon::SnapFit => {
            // Haste em balanço com a cabeça que trava.
            p.polygon(
                &[[2., 19.], [22., 19.], [22., 22.], [2., 22.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.polygon(
                &[
                    [8., 19.],
                    [8., 3.],
                    [12., 3.],
                    [16., 9.],
                    [12., 9.],
                    [12., 19.],
                ],
                p.bright,
                p.accent,
                MAIN,
            );
        }
        Icon::BedFace => {
            // Bloco descendo até apoiar a face na mesa.
            p.polygon(
                &[[8., 4.], [16., 4.], [16., 12.], [8., 12.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.fill(&[[8., 10.], [16., 10.], [16., 12.], [8., 12.]], p.accent);
            p.polygon(
                &[[2., 17.], [22., 17.], [22., 20.], [2., 20.]],
                p.bright,
                p.ink,
                MAIN,
            );
        }
        Icon::PrintProfile => {
            // Bico sobre camadas depositadas: vale para qualquer impressora FDM.
            p.polygon(
                &[
                    [8., 3.],
                    [16., 3.],
                    [16., 9.],
                    [13., 14.],
                    [11., 14.],
                    [8., 9.],
                ],
                p.surface,
                p.ink,
                MAIN,
            );
            p.stroke(&[[4., 17.], [20., 17.]], p.accent);
            p.stroke(&[[4., 21.], [20., 21.]], p.ink);
        }
        Icon::CheckSolid => {
            p.cube();
            p.line(&[[7.5, 12.5], [10.8, 15.8], [16.8, 9.2]], p.positive, 2.4);
        }
        Icon::Split => {
            p.cube();
            let plane = [[1., 15.5], [21., 3.], [22., 6.], [2., 18.5]];
            p.polygon(&plane, p.danger.gamma_multiply(0.16), p.danger, DETAIL);
            // Redesenhar as arestas mantém o sólido legível sob o plano translúcido.
            p.stroke(
                &[
                    [3., 7.6],
                    [12., 2.8],
                    [21., 7.6],
                    [21., 16.6],
                    [12., 21.4],
                    [3., 16.6],
                    [3., 7.6],
                ],
                p.ink,
            );
            p.detail(&[[3., 7.6], [12., 12.4], [21., 7.6]], p.ink);
            p.detail(&[[12., 12.4], [12., 21.4]], p.ink);
        }
        Icon::Move => {
            for target in [[12., 2.], [22., 12.], [12., 22.], [2., 12.]] {
                p.arrow([12., 12.], target, p.accent);
            }
            p.disc(12., 12., 2., p.ink);
        }
        Icon::Mirror => {
            p.dashed([12., 2.], [12., 22.], p.accent, 1.5);
            p.polygon(&[[3., 19.], [9., 5.], [9., 19.]], p.surface, p.ink, MAIN);
            p.polygon(
                &[[21., 19.], [15., 5.], [15., 19.]],
                p.bright,
                p.accent,
                MAIN,
            );
        }
        Icon::Pattern => {
            for (x, y) in [(4., 4.), (13.5, 4.), (4., 13.5), (13.5, 13.5)] {
                p.polygon(
                    &[[x, y], [x + 6., y], [x + 6., y + 6.], [x, y + 6.]],
                    p.surface,
                    p.ink,
                    DETAIL,
                );
            }
        }
        Icon::RectangularPattern => {
            for x in [3.5, 10., 16.5] {
                for y in [5., 15.] {
                    p.polygon(
                        &[[x, y], [x + 4.5, y], [x + 4.5, y + 4.5], [x, y + 4.5]],
                        p.surface,
                        p.ink,
                        DETAIL,
                    );
                }
            }
        }
        Icon::Measure => {
            p.polygon(
                &[[3., 17.], [17., 3.], [21., 7.], [7., 21.]],
                p.paper,
                p.ink,
                MAIN,
            );
            for step in 0..4 {
                let t = 3.5 + step as f32 * 3.2;
                p.detail(&[[t, 17. - (t - 3.)], [t - 1.2, 15.4 - (t - 3.)]], p.ink);
            }
            p.detail(&[[17., 3.], [21., 7.]], p.accent);
        }
        Icon::Parameters => {
            for (y, x) in [(6., 8.), (12., 16.), (18., 11.)] {
                p.detail(&[[4., y], [20., y]], p.ink);
                p.disc(x, y, 2.4, p.accent);
                p.disc(x, y, 0.9, p.paper);
            }
        }
        Icon::Drawing => {
            p.polygon(
                &[[3., 3.], [21., 3.], [21., 21.], [3., 21.]],
                p.paper,
                p.ink,
                MAIN,
            );
            p.detail(
                &[[6., 6.], [18., 6.], [18., 18.], [6., 18.], [6., 6.]],
                p.ink,
            );
            p.detail(&[[6., 14.], [18., 14.]], p.accent);
            p.detail(&[[6., 17.], [18., 17.]], p.accent);
            p.detail(&[[13., 14.], [13., 18.]], p.accent);
        }
        Icon::Fit => {
            for (x, y, dx, dy) in [
                (3., 3., 1., 1.),
                (21., 3., -1., 1.),
                (3., 21., 1., -1.),
                (21., 21., -1., -1.),
            ] {
                p.line(&[[x, y + dy * 5.], [x, y], [x + dx * 5., y]], p.ink, 2.2);
            }
            p.polygon(
                &[[9., 9.], [15., 9.], [15., 15.], [9., 15.]],
                p.bright,
                p.accent,
                DETAIL,
            );
        }
        Icon::Cube => p.cube(),
        Icon::Front | Icon::Top | Icon::Side => {
            p.cube();
            let face = match icon {
                Icon::Front => vec![[3., 7.6], [12., 12.4], [12., 21.4], [3., 16.6]],
                Icon::Top => vec![[12., 2.8], [21., 7.6], [12., 12.4], [3., 7.6]],
                _ => vec![[12., 12.4], [21., 7.6], [21., 16.6], [12., 21.4]],
            };
            p.polygon(&face, p.bright, p.accent, MAIN);
        }
        Icon::Perspective => {
            // Piso em fuga: as linhas convergem para o ponto de fuga.
            p.dashed([3., 21.], [12., 8.], p.accent, 1.6);
            p.dashed([21., 21.], [12., 8.], p.accent, 1.6);
            for (y, half) in [(12., 5.4), (16., 3.2), (19., 1.8)] {
                p.detail(&[[12. - half, y], [12. + half, y]], p.ink);
            }
            p.detail(&[[3., 8.], [21., 8.]], p.ink);
            p.disc(12., 8., 1.6, p.accent);
        }
        Icon::Eye | Icon::EyeOff => {
            p.bezier([2., 12.], [5., 6.5], [9., 5.], [12., 5.], p.ink);
            p.bezier([12., 5.], [15., 5.], [19., 6.5], [22., 12.], p.ink);
            p.bezier([22., 12.], [19., 17.5], [15., 19.], [12., 19.], p.ink);
            p.bezier([12., 19.], [9., 19.], [5., 17.5], [2., 12.], p.ink);
            p.disc(12., 12., 3.6, p.accent);
            p.disc(12., 12., 1.4, p.paper);
            if icon == Icon::EyeOff {
                p.line(&[[3., 3.], [21., 21.]], p.ink, 2.2);
            }
        }
        Icon::RotateClockwise | Icon::RotateCounterclockwise => {
            let clockwise = icon == Icon::RotateClockwise;
            let mut points: Vec<[f32; 2]> = (0..=20)
                .map(|index| {
                    let angle = -1. + 4.6 * index as f32 / 20.;
                    [12. + 8. * angle.cos(), 12. + 8. * angle.sin()]
                })
                .collect();
            if !clockwise {
                for point in &mut points {
                    point[0] = 24. - point[0];
                }
            }
            p.stroke(&points, p.ink);
            if let [.., before, tip] = points.as_slice() {
                p.head(*tip, [tip[0] - before[0], tip[1] - before[1]], p.accent);
            }
        }
        Icon::Minimize => p.line(&[[4., 12.], [20., 12.]], p.ink, 2.4),
        Icon::Maximize => p.polygon(
            &[[4.5, 4.5], [19.5, 4.5], [19.5, 19.5], [4.5, 19.5]],
            egui::Color32::TRANSPARENT,
            p.ink,
            2.2,
        ),
        Icon::Restore => {
            p.detail(&[[9., 3.5], [20.5, 3.5], [20.5, 15.], [17., 15.]], p.ink);
            p.polygon(
                &[[3.5, 8.5], [15., 8.5], [15., 20.5], [3.5, 20.5]],
                egui::Color32::TRANSPARENT,
                p.ink,
                2.2,
            );
        }
        Icon::Close => {
            p.line(&[[4.5, 4.5], [19.5, 19.5]], p.ink, 2.4);
            p.line(&[[19.5, 4.5], [4.5, 19.5]], p.ink, 2.4);
        }
        Icon::Check => p.line(&[[4., 12.5], [9.5, 18.], [20., 6.]], p.positive, 2.8),
        Icon::Select => {
            p.fill(&[[5., 2.], [5., 20.], [13., 14.]], p.surface);
            p.fill(&[[10., 16.], [14., 22.], [17., 20.], [13., 14.]], p.surface);
            p.stroke(
                &[
                    [5., 2.],
                    [5., 20.],
                    [10., 16.],
                    [14., 22.],
                    [17., 20.],
                    [13., 14.],
                    [20., 13.],
                    [5., 2.],
                ],
                p.ink,
            );
        }
        Icon::First | Icon::Last => {
            let mirror = |x: f32| if icon == Icon::Last { 24. - x } else { x };
            p.line(&[[mirror(5.), 4.], [mirror(5.), 20.]], p.ink, 2.4);
            p.fill(
                &[[mirror(19.), 4.], [mirror(7.5), 12.], [mirror(19.), 20.]],
                p.accent,
            );
        }
        Icon::Previous | Icon::Next => {
            let mirror = |x: f32| if icon == Icon::Next { 24. - x } else { x };
            p.fill(
                &[[mirror(17.), 4.], [mirror(6.), 12.], [mirror(17.), 20.]],
                p.accent,
            );
        }
        Icon::Delete => {
            p.line(&[[4., 6.], [20., 6.]], p.ink, 2.2);
            p.detail(&[[9., 6.], [9., 3.], [15., 3.], [15., 6.]], p.ink);
            p.polygon(
                &[[6., 6.], [18., 6.], [17., 21.], [7., 21.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.detail(&[[10., 10.], [10., 17.]], p.accent);
            p.detail(&[[14., 10.], [14., 17.]], p.accent);
        }
        Icon::Warning => {
            let fill = if !p.enabled {
                egui::Color32::from_gray(if p.dark { 72 } else { 226 })
            } else if p.dark {
                egui::Color32::from_rgb(96, 72, 36)
            } else {
                egui::Color32::from_rgb(255, 226, 160)
            };
            p.polygon(&[[12., 2.5], [22., 21.], [2., 21.]], fill, p.warning, 2.);
            p.line(&[[12., 8.5], [12., 15.]], p.warning, 2.4);
            p.disc(12., 18., 1.1, p.warning);
        }

        // Direções da extrusão.
        Icon::OneSide => {
            p.stroke(&[[3., 19.], [21., 19.]], p.ink);
            p.polygon(
                &[[8., 15.], [16., 15.], [16., 19.], [8., 19.]],
                p.bright,
                p.ink,
                DETAIL,
            );
            p.arrow([12., 15.], [12., 2.5], p.accent);
        }
        Icon::TwoSides => {
            p.stroke(&[[3., 9.], [21., 9.]], p.ink);
            p.polygon(
                &[[8., 7.], [16., 7.], [16., 11.], [8., 11.]],
                p.bright,
                p.ink,
                DETAIL,
            );
            p.arrow([12., 7.], [12., 2.5], p.accent);
            p.arrow([12., 11.], [12., 22.], p.accent);
        }
        Icon::Symmetric => {
            p.stroke(&[[3., 12.], [21., 12.]], p.ink);
            p.polygon(
                &[[8., 10.], [16., 10.], [16., 14.], [8., 14.]],
                p.bright,
                p.ink,
                DETAIL,
            );
            p.arrow([12., 10.], [12., 3.], p.accent);
            p.arrow([12., 14.], [12., 21.], p.accent);
        }
        Icon::ToFace => {
            p.polygon(
                &[[8., 15.], [16., 15.], [16., 19.], [8., 19.]],
                p.surface,
                p.ink,
                DETAIL,
            );
            p.polygon(
                &[[5., 3.], [19., 3.], [19., 6.], [5., 6.]],
                p.bright,
                p.accent,
                MAIN,
            );
            p.arrow([12., 14.], [12., 7.], p.accent);
        }
        Icon::Flip => {
            p.arrow([4., 8.], [20., 8.], p.accent);
            p.arrow([20., 16.], [4., 16.], p.ink);
        }

        // Montagem e organização.
        Icon::Align => {
            p.dashed([2., 18.], [22., 18.], p.ink, 1.4);
            p.polygon(
                &[[3., 12.], [9.5, 12.], [9.5, 18.], [3., 18.]],
                p.surface,
                p.ink,
                DETAIL,
            );
            p.polygon(
                &[[14.5, 12.], [21., 12.], [21., 18.], [14.5, 18.]],
                p.bright,
                p.accent,
                DETAIL,
            );
            p.arrow([10., 15.], [14., 15.], p.accent);
        }
        Icon::Group => {
            for (x, y, dx, dy) in [
                (2.5, 2.5, 1., 1.),
                (21.5, 2.5, -1., 1.),
                (2.5, 21.5, 1., -1.),
                (21.5, 21.5, -1., -1.),
            ] {
                p.line(&[[x + dx * 5., y], [x, y], [x, y + dy * 5.]], p.accent, 2.2);
            }
            p.polygon(
                &[[6., 6.], [11., 6.], [11., 11.], [6., 11.]],
                p.surface,
                p.ink,
                DETAIL,
            );
            p.polygon(
                &[[13., 13.], [18., 13.], [18., 18.], [13., 18.]],
                p.bright,
                p.accent,
                DETAIL,
            );
        }
        Icon::Section => {
            p.cube();
            let plane = [[2., 13.], [12., 6.5], [22., 11.], [12., 17.5]];
            p.polygon(&plane, p.danger.gamma_multiply(0.2), p.danger, DETAIL);
            p.stroke(
                &[
                    [3., 7.6],
                    [12., 2.8],
                    [21., 7.6],
                    [21., 16.6],
                    [12., 21.4],
                    [3., 16.6],
                    [3., 7.6],
                ],
                p.ink,
            );
            p.detail(&[[3., 7.6], [12., 12.4], [21., 7.6]], p.ink);
            p.detail(&[[12., 12.4], [12., 21.4]], p.ink);
        }
        Icon::Copy => {
            p.polygon(
                &[[8., 3.], [19., 3.], [19., 16.], [8., 16.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.polygon(
                &[[4., 8.], [15., 8.], [15., 21.], [4., 21.]],
                p.paper,
                p.ink,
                MAIN,
            );
            p.detail(&[[7., 12.], [12., 12.]], p.accent);
            p.detail(&[[7., 16.], [12., 16.]], p.accent);
        }
        Icon::FaceToFace => {
            p.polygon(
                &[[2., 6.], [9., 6.], [9., 18.], [2., 18.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.polygon(
                &[[15., 6.], [22., 6.], [22., 18.], [15., 18.]],
                p.bright,
                p.accent,
                MAIN,
            );
            p.dashed([12., 3.], [12., 21.], p.ink, 1.2);
            p.arrow([9., 12.], [15., 12.], p.accent);
        }
        Icon::EdgeToEdge => {
            p.stroke(&[[3., 20.], [8., 4.]], p.ink);
            p.stroke(&[[16., 20.], [21., 4.]], p.accent);
            p.arrow([9., 12.], [15., 12.], p.accent);
        }
        Icon::PointToPoint => {
            p.disc(5., 18., 2.4, p.ink);
            p.disc(19., 6., 2.4, p.accent);
            p.arrow([7.5, 16.5], [16.5, 7.5], p.accent);
        }
        Icon::ReferenceSketch => {
            p.polygon(
                &[[2., 19.], [6., 13.], [20., 13.], [16., 19.]],
                p.paper,
                p.reference,
                DETAIL,
            );
            p.pencil([7., 21.], [19., 6.], 1.7, p.surface, p.reference);
        }

        // Arquivos de intercâmbio e corpos.
        Icon::StepFile => {
            p.document();
            p.fill(&[[12., 9.5], [17., 12.], [12., 14.5], [7., 12.]], p.bright);
            p.stroke(
                &[
                    [7., 12.],
                    [12., 9.5],
                    [17., 12.],
                    [17., 16.5],
                    [12., 19.],
                    [7., 16.5],
                    [7., 12.],
                ],
                p.accent,
            );
            p.detail(&[[7., 12.], [12., 14.5], [17., 12.]], p.accent);
            p.detail(&[[12., 14.5], [12., 19.]], p.accent);
        }
        Icon::StlFile => {
            p.document();
            p.polygon(
                &[[12., 9.5], [17.5, 18.], [6.5, 18.]],
                p.bright,
                p.accent,
                DETAIL,
            );
            p.detail(&[[12., 9.5], [12., 18.]], p.accent);
        }
        Icon::Join | Icon::CutBody | Icon::IntersectBody => {
            let left = [[3., 4.], [13., 4.], [13., 14.], [3., 14.]];
            let right = [[11., 10.], [21., 10.], [21., 20.], [11., 20.]];
            match icon {
                Icon::Join => {
                    p.polygon(&left, p.surface, p.ink, MAIN);
                    p.polygon(&right, p.surface, p.ink, MAIN);
                    p.fill(&[[11., 10.], [13., 10.], [13., 14.], [11., 14.]], p.bright);
                }
                Icon::CutBody => {
                    p.polygon(&left, p.surface, p.ink, MAIN);
                    p.dashed([11., 10.], [21., 10.], p.accent, 1.5);
                    p.dashed([21., 10.], [21., 20.], p.accent, 1.5);
                    p.dashed([21., 20.], [11., 20.], p.accent, 1.5);
                    p.dashed([11., 20.], [11., 10.], p.accent, 1.5);
                    p.fill(&[[11., 10.], [13., 10.], [13., 14.], [11., 14.]], p.paper);
                    p.detail(&[[11., 10.], [11., 14.]], p.accent);
                }
                _ => {
                    p.polygon(&left, p.paper, p.ink, DETAIL);
                    p.polygon(&right, p.paper, p.ink, DETAIL);
                    p.polygon(
                        &[[11., 10.], [13., 10.], [13., 14.], [11., 14.]],
                        p.bright,
                        p.accent,
                        DETAIL,
                    );
                }
            }
        }
        Icon::NewBody => {
            p.cube();
            p.badge_plus(18., 18.5, 4.1);
        }
        Icon::Part => {
            p.cube();
            p.polygon(
                &[[15., 14.], [21., 14.], [22., 15.5], [22., 21.], [15., 21.]],
                p.accent,
                p.accent,
                DETAIL,
            );
            p.disc(20., 16., 1.1, p.paper);
        }
        Icon::Edit => {
            p.polygon(
                &[
                    [14., 3.5],
                    [20.5, 10.],
                    [10., 20.5],
                    [3.5, 20.5],
                    [3.5, 14.],
                ],
                p.paper,
                p.ink,
                MAIN,
            );
            p.fill(&[[3.5, 14.], [10., 20.5], [3.5, 20.5]], p.accent);
            p.line(&[[13., 4.5], [19., 10.5]], p.accent, DETAIL);
        }
        Icon::Origin => {
            p.detail(&[[5., 19.], [21., 19.]], p.ink);
            p.detail(&[[5., 19.], [5., 3.]], p.ink);
            p.detail(&[[5., 19.], [14., 10.]], p.accent);
            p.head([21., 19.], [1., 0.], p.accent);
            p.head([5., 3.], [0., -1.], p.accent);
            p.head([14., 10.], [0.7, -0.7], p.accent);
            p.disc(5., 19., 2.4, p.accent);
        }
        Icon::OffsetPlane => {
            p.detail(
                &[[2., 21.], [7., 15.], [22., 15.], [17., 21.], [2., 21.]],
                p.ink,
            );
            p.polygon(
                &[[2., 10.], [7., 4.], [22., 4.], [17., 10.]],
                p.bright,
                p.accent,
                MAIN,
            );
            p.arrow([12., 19.], [12., 7.5], p.accent);
        }
        Icon::WorkPlane => {
            p.polygon(
                &[[2., 18.], [8., 7.], [22., 7.], [16., 18.]],
                p.bright,
                p.accent,
                MAIN,
            );
            p.detail(&[[9., 12.5], [15., 12.5]], p.accent);
            p.disc(12., 12.5, 1., p.accent);
        }
        Icon::Folder => p.folder(),

        // Rascunho.
        Icon::Sketch => {
            p.polygon(
                &[[2., 19.], [6., 13.], [20., 13.], [16., 19.]],
                p.surface,
                p.ink,
                DETAIL,
            );
            p.pencil([7., 21.], [19., 6.], 1.7, p.surface, p.accent);
        }
        Icon::SketchProject => {
            p.polygon(&[[6., 2.], [12., 2.], [9., 7.]], p.paper, p.ink, DETAIL);
            p.polygon(
                &[[2., 15.], [9., 12.], [22., 18.], [15., 21.]],
                p.bright,
                p.accent,
                MAIN,
            );
            p.dashed([6., 7.], [8., 14.], p.ink, 1.2);
            p.dashed([9., 7.], [11., 15.], p.ink, 1.2);
            p.dashed([12., 7.], [14., 16.5], p.ink, 1.2);
        }
        Icon::Image => {
            p.polygon(
                &[[3., 4.], [21., 4.], [21., 20.], [3., 20.]],
                p.paper,
                p.ink,
                MAIN,
            );
            p.disc(7.5, 9., 2., p.accent);
            p.stroke(
                &[[3., 17.], [9., 11.], [13., 15.], [17., 10.], [21., 15.]],
                p.accent,
            );
        }
        Icon::Paint => {
            p.polygon(
                &[[3., 11.], [10., 4.], [19., 13.], [12., 20.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.detail(
                &[[6.5, 8.], [7., 4.], [10., 2.], [13., 3.], [16., 6.]],
                p.ink,
            );
            p.detail(&[[3., 11.], [17., 17.]], p.accent);
            p.disc(20., 17., 2.2, p.accent);
            p.fill(&[[20., 12.], [18.6, 16.], [21.4, 16.]], p.accent);
        }
        Icon::Line => {
            p.line(&[[4., 19.], [20., 5.]], p.accent, 2.4);
            p.disc(4., 19., 2.2, p.ink);
            p.disc(20., 5., 2.2, p.ink);
        }
        Icon::LineSymmetric => {
            p.line(&[[3., 18.], [21., 6.]], p.accent, 2.2);
            p.disc(12., 12., 2.4, p.ink);
            p.detail(&[[6.9, 15.3], [9.1, 17.5]], p.ink);
            p.detail(&[[14.9, 6.5], [17.1, 8.7]], p.ink);
        }
        Icon::LineConstruction | Icon::LineInfinite => {
            // Mesmo traço e ângulo do ícone Linha; só o laranja e o tracejado longo
            // distinguem a referência, e as pontas de seta distinguem a infinita.
            let color = p.construction;
            if icon == Icon::LineConstruction {
                p.long_dashed([4., 19.], [20., 5.], color, 2.4);
                p.disc(4., 19., 2.2, p.ink);
                p.disc(20., 5., 2.2, p.ink);
            } else {
                p.long_dashed([2.5, 20.3], [21.5, 3.7], color, 2.4);
                p.head([1., 21.6], [-8., 7.], color);
                p.head([23., 2.4], [8., -7.], color);
            }
        }
        Icon::Rectangle => {
            p.polygon(
                &[[3.5, 5.], [20.5, 5.], [20.5, 19.], [3.5, 19.]],
                p.surface,
                p.ink,
                MAIN,
            );
        }
        Icon::RectangleCenter => {
            p.polygon(
                &[[3.5, 5.], [20.5, 5.], [20.5, 19.], [3.5, 19.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.detail(&[[3.5, 5.], [20.5, 19.]], p.accent);
            p.detail(&[[3.5, 19.], [20.5, 5.]], p.accent);
            p.disc(12., 12., 2.2, p.ink);
        }
        Icon::RectangleInclined => {
            p.polygon(
                &[[3., 11.], [15., 3.], [21., 13.], [9., 21.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.line(&[[3., 11.], [15., 3.]], p.accent, 2.2);
            p.detail(&[[8.5, 9.5], [10.5, 11.], [13., 7.5]], p.ink);
            p.disc(3., 11., 1.5, p.ink);
            p.disc(15., 3., 1.5, p.ink);
        }
        Icon::Circle => {
            p.ring(12., 12., 8.5, p.accent, 2.2);
            p.detail(&[[9., 12.], [15., 12.]], p.ink);
            p.detail(&[[12., 9.], [12., 15.]], p.ink);
        }
        Icon::CircleDiameter => {
            p.ring(12., 12., 8.5, p.accent, 2.2);
            p.detail(&[[5.5, 18.5], [18.5, 5.5]], p.ink);
            p.disc(5.5, 18.5, 1.7, p.ink);
            p.disc(18.5, 5.5, 1.7, p.ink);
        }
        Icon::CircleTangent => {
            p.ring(12., 12., 7., p.accent, 2.2);
            p.stroke(&[[3., 19.], [21., 19.]], p.ink);
            p.disc(12., 19., 1.6, p.accent);
        }
        Icon::Arc => {
            p.arc(12., 16., 8.5, PI, TAU, p.accent);
            p.disc(3.5, 16., 1.8, p.ink);
            p.disc(20.5, 16., 1.8, p.ink);
        }
        Icon::Trim => {
            p.line(&[[7., 20.], [18., 4.]], p.ink, 2.2);
            p.line(&[[17., 20.], [6., 4.]], p.ink, 2.2);
            p.ring(6., 20., 2.2, p.accent, DETAIL);
            p.ring(18., 20., 2.2, p.accent, DETAIL);
        }
        Icon::SketchOffset => {
            p.stroke(&[[3., 20.], [3., 8.], [8., 3.], [20., 3.]], p.ink);
            p.stroke(&[[7., 21.], [7., 10.], [10., 7.], [21., 7.]], p.accent);
            p.arrow([13., 15.], [19., 15.], p.accent);
        }
        Icon::SketchFillet => {
            // Retângulo com o canto superior esquerdo arredondado; o Chanfro
            // usa a mesma silhueta com uma aresta reta no lugar do arco.
            let mut outline = vec![[10., 4.], [20., 4.], [20., 20.], [4., 20.], [4., 10.]];
            outline.extend((0..=12).map(|index| {
                let angle = PI + FRAC_PI_2 * index as f32 / 12.;
                [10. + 6. * angle.cos(), 10. + 6. * angle.sin()]
            }));
            p.polygon(&outline, p.surface, p.ink, MAIN);
            p.arc(10., 10., 6., PI, PI + FRAC_PI_2, p.accent);
        }
        Icon::SketchChamfer => {
            p.polygon(
                &[[10., 4.], [20., 4.], [20., 20.], [4., 20.], [4., 10.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.line(&[[4., 10.], [10., 4.]], p.accent, 2.2);
        }
        Icon::SketchMirror => {
            p.dashed([12., 2.], [12., 22.], p.ink, 1.4);
            p.polygon(&[[3., 18.], [8., 7.], [8., 18.]], p.surface, p.ink, DETAIL);
            p.polygon(
                &[[21., 18.], [16., 7.], [16., 18.]],
                p.bright,
                p.accent,
                DETAIL,
            );
        }
        Icon::CircularPattern => {
            p.ring(12., 12., 7.5, p.ink, 1.1);
            for step in 0..6 {
                let angle = step as f32 * TAU / 6.;
                p.disc(
                    12. + 7.5 * angle.cos(),
                    12. + 7.5 * angle.sin(),
                    1.9,
                    p.accent,
                );
            }
            p.disc(12., 12., 1.4, p.ink);
        }
        Icon::Dimension => {
            p.detail(&[[3., 3.], [3., 21.]], p.ink);
            p.detail(&[[21., 3.], [21., 21.]], p.ink);
            p.line(&[[4., 12.], [20., 12.]], p.accent, 2.2);
            p.head([4., 12.], [-1., 0.], p.accent);
            p.head([20., 12.], [1., 0.], p.accent);
        }
        Icon::ConstraintFix => {
            // Alfinete: cabeça na cor de ação e haste presa ao plano.
            p.ring(12., 8., 4.6, p.accent, 2.2);
            p.stroke(&[[12., 12.6], [12., 21.]], p.ink);
            p.detail(&[[7., 21.], [17., 21.]], p.ink);
        }
        Icon::ConstraintConcentric => {
            p.ring(12., 12., 9., p.ink, MAIN);
            p.ring(12., 12., 4.6, p.accent, 2.2);
            p.disc(12., 12., 1.3, p.accent);
        }
        Icon::ConstraintCollinear => {
            p.stroke(&[[3., 20.], [9.5, 13.5]], p.accent);
            p.stroke(&[[14.5, 9.5], [21., 3.]], p.accent);
            p.disc(12., 11.5, 1.2, p.ink);
            p.disc(3., 20., 1.7, p.ink);
            p.disc(21., 3., 1.7, p.ink);
        }
        Icon::ConstraintSymmetry => {
            p.dashed([12., 2.], [12., 22.], p.accent, 1.5);
            p.detail(&[[5., 12.], [19., 12.]], p.ink);
            p.disc(5., 12., 2.6, p.ink);
            p.disc(19., 12., 2.6, p.ink);
        }
        Icon::ConstraintPolygon => p.polygon(
            &[
                [12., 3.],
                [20., 7.5],
                [20., 16.5],
                [12., 21.],
                [4., 16.5],
                [4., 7.5],
            ],
            p.bright,
            p.accent,
            MAIN,
        ),
        Icon::ConstraintManager => {
            p.polygon(
                &[[3., 5.], [21., 5.], [21., 19.], [3., 19.]],
                p.bright,
                p.accent,
                MAIN,
            );
            p.detail(&[[7., 10.], [17., 10.]], p.ink);
            p.detail(&[[7., 14.], [14., 14.]], p.ink);
        }
        Icon::ConstraintHorizontalVertical => {
            p.stroke(&[[5., 4.], [5., 19.], [20., 19.]], p.accent);
            p.detail(&[[5., 14.], [10., 14.], [10., 19.]], p.ink);
        }
        Icon::ConstraintCoincident => {
            p.stroke(&[[3., 19.], [10., 12.], [21., 17.]], p.ink);
            p.ring(10., 12., 3.2, p.accent, 2.2);
            p.disc(10., 12., 1., p.accent);
        }
        Icon::ConstraintTangent => {
            p.ring(12., 13., 6.5, p.accent, 2.2);
            p.stroke(&[[2., 6.5], [22., 6.5]], p.ink);
            p.disc(12., 6.5, 1.6, p.ink);
        }
        Icon::ConstraintEqual => {
            p.line(&[[4., 8.], [20., 8.]], p.accent, 2.2);
            p.line(&[[4., 16.], [20., 16.]], p.accent, 2.2);
            p.detail(&[[9., 5.], [9., 11.]], p.ink);
            p.detail(&[[15., 13.], [15., 19.]], p.ink);
        }
        Icon::ConstraintParallel => {
            p.stroke(&[[4., 19.], [11., 4.]], p.accent);
            p.stroke(&[[13., 20.], [20., 5.]], p.accent);
            p.head([11., 4.], [0.55, -1.], p.accent);
            p.head([20., 5.], [0.55, -1.], p.accent);
        }
        Icon::ConstraintPerpendicular => {
            p.stroke(&[[3., 19.], [21., 19.]], p.accent);
            p.stroke(&[[12., 3.], [12., 19.]], p.accent);
            p.detail(&[[12., 14.], [17., 14.], [17., 19.]], p.ink);
        }
        Icon::SnapMidpoint => {
            p.stroke(&[[2., 18.], [22., 18.]], p.ink);
            p.polygon(
                &[[12., 18.], [6.5, 10.], [17.5, 10.]],
                p.bright,
                p.accent,
                DETAIL,
            );
            p.disc(12., 18., 1.4, p.accent);
        }
        Icon::SnapCenter => {
            p.detail(&[[2., 12.], [22., 12.]], p.ink);
            p.detail(&[[12., 2.], [12., 22.]], p.ink);
            p.ring(12., 12., 6., p.accent, 2.2);
            p.disc(12., 12., 1.6, p.accent);
        }
        Icon::LockOpen => {
            let mut shackle = vec![[11.5, 11.], [11.5, 6.5]];
            shackle.extend((0..=12).map(|index| {
                let angle = PI + PI * index as f32 / 12.;
                [16. + 4.5 * angle.cos(), 6.5 + 4.5 * angle.sin()]
            }));
            shackle.push([20.5, 10.]);
            p.stroke(&shackle, p.ink);
            p.polygon(
                &[[5., 11.], [19., 11.], [19., 21.], [5., 21.]],
                p.surface,
                p.ink,
                MAIN,
            );
            p.disc(12., 15.5, 1.4, p.accent);
            p.line(&[[12., 16.5], [12., 19.]], p.accent, 2.);
        }
    }
}

/// Folha de contato usada na revisão visual de todos os ícones.
pub(super) fn preview_sheet(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    let painter = ui.painter();
    painter.rect_filled(rect, 0, theme::panel());
    let columns = 10;
    let margin = 16.;
    let header = 30.;
    let cell_width = (rect.width() - margin * 2.) / columns as f32;
    let cell_height = (rect.height() - margin * 2. - header) / 12.;
    let icon_size = (cell_width * 0.4).min(cell_height * 0.52).clamp(24., 46.);
    let label_top = icon_size * 0.6 + 2.;
    let font_size = (icon_size * 0.25).clamp(8., 11.);
    painter.text(
        egui::pos2(rect.left() + margin, rect.top() + 10.),
        egui::Align2::LEFT_TOP,
        "Ícones do 3dnator",
        egui::FontId::proportional(15.),
        theme::ink(),
    );
    for (index, icon) in super::ALL.iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        let center = egui::pos2(
            rect.left() + margin + cell_width * (column as f32 + 0.5),
            rect.top() + margin + header + cell_height * (row as f32 + 0.5),
        );
        let icon_rect = egui::Rect::from_center_size(
            center - egui::vec2(0., label_top * 0.35),
            egui::Vec2::splat(icon_size),
        );
        paint(painter, icon_rect, *icon, true, theme::ink(), false);
        painter.text(
            center + egui::vec2(0., label_top * 0.6),
            egui::Align2::CENTER_TOP,
            format!("{icon:?}"),
            egui::FontId::proportional(font_size),
            theme::muted(),
        );
    }
    if rect.height() < 760. {
        return;
    }
    // Amostra da barra escura da janela, onde traço e preenchimento mudam.
    let strip = egui::Rect::from_min_max(
        egui::pos2(rect.left() + margin, rect.bottom() - 74.),
        egui::pos2(rect.right() - margin, rect.bottom() - 14.),
    );
    painter.rect_filled(strip, 8, theme::header());
    let dark_ink = egui::Color32::from_rgb(238, 243, 250);
    let sample = [
        Icon::New,
        Icon::Open,
        Icon::Save,
        Icon::SaveAs,
        Icon::Settings,
        Icon::Undo,
        Icon::Redo,
        Icon::Search,
        Icon::Eye,
        Icon::EyeOff,
        Icon::Close,
        Icon::Select,
    ];
    for (index, icon) in sample.iter().enumerate() {
        let step = strip.width() / sample.len() as f32;
        let icon_rect = egui::Rect::from_center_size(
            egui::pos2(strip.left() + step * (index as f32 + 0.5), strip.center().y),
            egui::Vec2::splat(34.),
        );
        paint(painter, icon_rect, *icon, true, dark_ink, true);
    }
}
