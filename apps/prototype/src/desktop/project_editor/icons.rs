//! Ícones vetoriais originais do 3dnator, desenhados em uma grade lógica de 24 unidades.
//! Não dependem de fontes de símbolos nem de bitmaps: o mesmo desenho permanece nítido
//! com escala de interface e DPI diferentes. A cor distingue o sólido da ação em azul.
use super::{egui, theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Icon {
    New,
    Open,
    Save,
    SaveAs,
    Undo,
    Redo,
    Import,
    Export,
    Sketch,
    Extrude,
    Revolve,
    Hole,
    Fillet,
    Chamfer,
    Shell,
    Boolean,
    Split,
    Move,
    Mirror,
    Pattern,
    Measure,
    Parameters,
    Drawing,
    Fit,
    Cube,
    Front,
    Top,
    Side,
    Perspective,
    Eye,
    EyeOff,
    Close,
    Minimize,
    Maximize,
    Restore,
    RotateClockwise,
    RotateCounterclockwise,
    Search,
    Check,
    Select,
    Line,
    LineSymmetric,
    LineConstruction,
    LineInfinite,
    Rectangle,
    RectangleCenter,
    RectangleInclined,
    Circle,
    CircleDiameter,
    CircleTangent,
    Arc,
    Trim,
    SketchOffset,
    SketchFillet,
    SketchMirror,
    CircularPattern,
    RectangularPattern,
    Dimension,
    ConstraintHorizontalVertical,
    ConstraintCoincident,
    ConstraintTangent,
    ConstraintEqual,
    ConstraintParallel,
    ConstraintPerpendicular,
    Folder,
    First,
    Previous,
    Next,
    Last,
    Delete,
    Warning,
    OneSide,
    TwoSides,
    Symmetric,
    ToFace,
    Flip,
    Align,
    Group,
    Section,
    Copy,
    FaceToFace,
    EdgeToEdge,
    PointToPoint,
    ReferenceSketch,
    StepFile,
    StlFile,
    Join,
    CutBody,
    IntersectBody,
    NewBody,
    Part,
    Edit,
    Origin,
    #[allow(dead_code)]
    Project,
    Home,
}

/// Elementos geométricos mantêm proporção e traço em qualquer tamanho de botão.
struct Pen<'a> {
    painter: &'a egui::Painter,
    rect: egui::Rect,
    ink: egui::Color32,
    accent: egui::Color32,
    fill: egui::Color32,
}
impl Pen<'_> {
    fn p(&self, x: f32, y: f32) -> egui::Pos2 {
        self.rect.min + egui::vec2(x, y) * (self.rect.width() / 24.)
    }
    fn line(&self, points: &[[f32; 2]], color: egui::Color32) {
        self.painter.add(egui::Shape::line(
            points.iter().map(|p| self.p(p[0], p[1])).collect(),
            egui::Stroke::new((self.rect.width() / 24. * 1.5).max(1.), color),
        ));
    }
    fn face(&self, points: &[[f32; 2]], fill: egui::Color32) {
        self.painter.add(egui::Shape::convex_polygon(
            points.iter().map(|p| self.p(p[0], p[1])).collect(),
            fill,
            egui::Stroke::new((self.rect.width() / 24. * 1.3).max(1.), self.ink),
        ));
    }
    fn circle(&self, x: f32, y: f32, radius: f32, color: egui::Color32) {
        self.painter.circle_stroke(
            self.p(x, y),
            radius * self.rect.width() / 24.,
            egui::Stroke::new(1.5, color),
        );
    }
    fn arrow(&self, from: [f32; 2], to: [f32; 2]) {
        self.line(&[from, to], self.accent);
        let d = egui::vec2(to[0] - from[0], to[1] - from[1]).normalized();
        let side = egui::vec2(-d.y, d.x);
        let end = egui::vec2(to[0], to[1]);
        let a = end - d * 3. + side * 2.;
        let b = end - d * 3. - side * 2.;
        self.line(&[[a.x, a.y], to, [b.x, b.y]], self.accent);
    }
    fn cube(&self) {
        self.face(&[[3., 8.], [12., 3.], [21., 8.], [12., 13.]], self.fill);
        self.face(
            &[[3., 8.], [12., 13.], [12., 22.], [3., 17.]],
            egui::Color32::from_rgb(167, 192, 214),
        );
        self.face(
            &[[12., 13.], [21., 8.], [21., 17.], [12., 22.]],
            egui::Color32::from_rgb(107, 162, 209),
        );
    }
}

pub(super) fn paint(painter: &egui::Painter, rect: egui::Rect, icon: Icon, enabled: bool) {
    paint_with_ink(painter, rect, icon, enabled, theme::INK);
}
pub(super) fn paint_with_ink(
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: Icon,
    enabled: bool,
    ink: egui::Color32,
) {
    // Geometria de rascunho em verde e restrições em vermelho permitem distinguir
    // rapidamente criação e intenção geométrica, inclusive na galeria compacta.
    let constraint = matches!(
        icon,
        Icon::ConstraintHorizontalVertical
            | Icon::ConstraintCoincident
            | Icon::ConstraintTangent
            | Icon::ConstraintEqual
            | Icon::ConstraintParallel
            | Icon::ConstraintPerpendicular
    );
    let sketch = matches!(
        icon,
        Icon::Line
            | Icon::LineSymmetric
            | Icon::Rectangle
            | Icon::RectangleCenter
            | Icon::RectangleInclined
            | Icon::Circle
            | Icon::CircleDiameter
            | Icon::CircleTangent
            | Icon::Arc
            | Icon::Trim
            | Icon::SketchOffset
            | Icon::SketchFillet
            | Icon::SketchMirror
            | Icon::CircularPattern
            | Icon::RectangularPattern
            | Icon::Dimension
    );
    let accent = if constraint {
        theme::DANGER
    } else if sketch {
        theme::GREEN
    } else {
        theme::ACCENT
    };
    let pen = Pen {
        painter,
        rect,
        ink: if enabled {
            if constraint { accent } else { ink }
        } else {
            egui::Color32::from_gray(165)
        },
        accent: if enabled {
            accent
        } else {
            egui::Color32::from_gray(165)
        },
        fill: if enabled {
            egui::Color32::from_rgb(204, 225, 243)
        } else {
            egui::Color32::from_gray(226)
        },
    };
    let p = &pen;
    match icon {
        Icon::Home => {
            p.line(&[[3., 11.], [12., 3.], [21., 11.]], p.ink);
            p.line(
                &[
                    [5., 10.],
                    [5., 21.],
                    [10., 21.],
                    [10., 14.],
                    [14., 14.],
                    [14., 21.],
                    [19., 21.],
                    [19., 10.],
                ],
                p.ink,
            );
        }
        Icon::Cube
        | Icon::Extrude
        | Icon::Hole
        | Icon::Fillet
        | Icon::Chamfer
        | Icon::Shell
        | Icon::Boolean
        | Icon::Split => {
            p.cube();
            match icon {
                Icon::Extrude => p.arrow([12., 13.], [12., 1.]),
                Icon::Hole | Icon::Shell => {
                    p.circle(12., 8., if icon == Icon::Hole { 2.5 } else { 4. }, p.ink);
                }
                Icon::Fillet => {
                    p.line(&[[12., 13.], [16., 12.5], [19., 14.], [21., 17.]], p.accent)
                }
                Icon::Chamfer => p.line(&[[12., 16.], [17., 10.]], p.accent),
                Icon::Boolean => p.face(&[[1., 15.], [7., 12.], [13., 15.], [7., 19.]], p.fill),
                Icon::Split => p.line(&[[2., 20.], [21., 2.]], p.accent),
                _ => {}
            }
        }
        Icon::New | Icon::Drawing | Icon::SaveAs => {
            p.line(
                &[
                    [16., 3.],
                    [5., 3.],
                    [5., 21.],
                    [20., 21.],
                    [20., 7.],
                    [16., 3.],
                    [16., 8.],
                    [20., 8.],
                ],
                p.ink,
            );
            if icon == Icon::Drawing {
                p.line(
                    &[[8., 11.], [16., 11.], [16., 16.], [8., 16.], [8., 11.]],
                    p.accent,
                );
                p.line(&[[8., 18.], [17., 18.]], p.ink);
            } else if icon == Icon::New {
                p.line(&[[9., 14.], [16., 14.]], p.accent);
                p.line(&[[12.5, 10.5], [12.5, 17.5]], p.accent);
            } else {
                p.line(&[[10., 17.], [17., 10.]], p.accent);
            }
        }
        Icon::Open | Icon::Folder => {
            p.face(
                &[
                    [3., 6.],
                    [9., 6.],
                    [11., 8.],
                    [21., 8.],
                    [21., 19.],
                    [3., 19.],
                ],
                p.fill,
            );
            p.line(&[[3., 11.], [21., 11.]], p.ink);
        }
        Icon::Save => {
            p.face(
                &[[4., 3.], [18., 3.], [21., 6.], [21., 21.], [4., 21.]],
                p.fill,
            );
            p.face(
                &[[8., 3.], [16., 3.], [16., 9.], [8., 9.]],
                egui::Color32::WHITE,
            );
            p.face(
                &[[8., 14.], [17., 14.], [17., 21.], [8., 21.]],
                egui::Color32::WHITE,
            );
        }
        Icon::Undo | Icon::Redo => {
            let mirror = |x| if icon == Icon::Redo { 24. - x } else { x };
            let points: Vec<_> = [[5., 9.], [13., 9.], [18., 11.], [20., 16.]]
                .map(|v| [mirror(v[0]), v[1]])
                .to_vec();
            p.line(&points, p.ink);
            p.line(
                &[[mirror(10.), 4.], [mirror(5.), 9.], [mirror(10.), 14.]],
                p.accent,
            );
        }
        Icon::Import | Icon::Export => {
            p.line(&[[3., 15.], [3., 21.], [21., 21.], [21., 15.]], p.ink);
            if icon == Icon::Import {
                p.arrow([12., 2.], [12., 16.]);
            } else {
                p.arrow([12., 16.], [12., 2.]);
            }
        }
        Icon::Sketch => {
            p.line(&[[3., 5.], [3., 21.], [19., 21.]], p.ink);
            p.face(
                &[[7., 16.], [9., 10.], [18., 1.], [22., 5.], [13., 14.]],
                p.fill,
            );
            p.line(&[[9., 10.], [13., 14.]], p.accent);
        }
        Icon::Revolve => {
            p.line(&[[12., 2.], [12., 22.]], p.ink);
            p.line(
                &[
                    [14., 5.],
                    [19., 8.],
                    [21., 12.],
                    [19., 17.],
                    [14., 19.],
                    [7., 18.],
                    [4., 15.],
                ],
                p.accent,
            );
            p.arrow([4., 15.], [4., 9.]);
            p.face(&[[7., 7.], [11., 7.], [11., 17.], [7., 17.]], p.fill);
        }
        Icon::Move => {
            for point in [[12., 2.], [22., 12.], [12., 22.], [2., 12.]] {
                p.arrow([12., 12.], point);
            }
        }
        Icon::Mirror => {
            p.line(&[[12., 2.], [12., 22.]], p.accent);
            p.face(&[[3., 19.], [9., 6.], [9., 19.]], p.fill);
            p.face(&[[15., 6.], [21., 19.], [15., 19.]], p.fill);
        }
        Icon::Pattern | Icon::RectangularPattern => {
            for x in [3., 14.] {
                for y in [3., 14.] {
                    p.face(
                        &[[x, y], [x + 6., y], [x + 6., y + 6.], [x, y + 6.]],
                        if icon == Icon::RectangularPattern {
                            p.accent.gamma_multiply(0.35)
                        } else {
                            p.fill
                        },
                    );
                }
            }
        }
        Icon::Measure => {
            p.face(&[[3., 17.], [17., 3.], [22., 8.], [8., 22.]], p.fill);
            for i in 0..4 {
                let x = 7. + i as f32 * 3.;
                p.line(
                    &[[x, 17. - i as f32 * 3.], [x - 2., 15. - i as f32 * 3.]],
                    p.ink,
                );
            }
        }
        Icon::Parameters => {
            for (y, x) in [(5., 8.), (12., 16.), (19., 10.)] {
                p.line(&[[3., y], [21., y]], p.ink);
                p.face(
                    &[
                        [x - 2., y - 2.],
                        [x + 2., y - 2.],
                        [x + 2., y + 2.],
                        [x - 2., y + 2.],
                    ],
                    p.fill,
                );
            }
        }
        Icon::Fit => {
            for (x, y, dx, dy) in [
                (3., 3., 1., 1.),
                (21., 3., -1., 1.),
                (3., 21., 1., -1.),
                (21., 21., -1., -1.),
            ] {
                p.line(&[[x, y + dy * 5.], [x, y], [x + dx * 5., y]], p.ink);
            }
            p.face(&[[9., 9.], [15., 9.], [15., 15.], [9., 15.]], p.fill);
        }
        Icon::Front | Icon::Top | Icon::Side => {
            p.cube();
            let face = match icon {
                Icon::Front => vec![[3., 8.], [12., 13.], [12., 22.], [3., 17.]],
                Icon::Top => vec![[3., 8.], [12., 3.], [21., 8.], [12., 13.]],
                _ => vec![[12., 13.], [21., 8.], [21., 17.], [12., 22.]],
            };
            p.face(&face, p.accent);
        }
        Icon::Perspective => {
            p.face(&[[3., 5.], [21., 2.], [21., 22.], [3., 19.]], p.fill);
            p.line(&[[3., 5.], [13., 10.], [21., 2.]], p.ink);
            p.line(&[[13., 10.], [13., 20.]], p.ink);
        }
        Icon::Eye | Icon::EyeOff => {
            p.line(
                &[
                    [2., 12.],
                    [6., 7.],
                    [12., 5.],
                    [18., 7.],
                    [22., 12.],
                    [18., 17.],
                    [12., 19.],
                    [6., 17.],
                    [2., 12.],
                ],
                p.ink,
            );
            p.circle(12., 12., 3., p.accent);
            if icon == Icon::EyeOff {
                p.line(&[[3., 3.], [21., 21.]], p.ink);
            }
        }
        Icon::RotateClockwise | Icon::RotateCounterclockwise => {
            let mirror = if icon == Icon::RotateClockwise {
                1.
            } else {
                -1.
            };
            let points: Vec<_> = (0..=24)
                .map(|i| {
                    let angle = (-0.8 + i as f32 / 24. * 1.6) * std::f32::consts::PI;
                    [12. + mirror * 8. * angle.cos(), 12. + 8. * angle.sin()]
                })
                .collect();
            p.line(&points, p.ink);
            if let [.., a, b] = points.as_slice() {
                p.arrow(*a, *b);
            }
        }
        Icon::Minimize => p.line(&[[5., 17.], [19., 17.]], p.ink),
        Icon::Maximize => p.line(
            &[[5., 5.], [19., 5.], [19., 19.], [5., 19.], [5., 5.]],
            p.ink,
        ),
        Icon::Restore => {
            p.line(&[[8., 4.], [20., 4.], [20., 16.], [17., 16.]], p.ink);
            p.line(
                &[[4., 8.], [16., 8.], [16., 20.], [4., 20.], [4., 8.]],
                p.ink,
            );
        }
        Icon::Close => {
            p.line(&[[6., 6.], [18., 18.]], p.ink);
            p.line(&[[18., 6.], [6., 18.]], p.ink);
        }
        Icon::Search => {
            p.circle(10., 10., 6., p.ink);
            p.line(&[[15., 15.], [21., 21.]], p.ink);
        }
        Icon::Check => p.line(&[[4., 12.], [10., 18.], [21., 5.]], theme::GREEN),
        Icon::Select => p.line(
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
        ),
        Icon::Line => {
            p.line(&[[4., 19.], [20., 4.]], p.accent);
            p.circle(4., 19., 2., p.ink);
            p.circle(20., 4., 2., p.ink);
        }
        Icon::LineSymmetric => {
            // O apoio central separa duas metades iguais, com setas opostas.
            p.arrow([12., 12.], [3., 20.]);
            p.arrow([12., 12.], [21., 4.]);
            p.circle(12., 12., 2., p.ink);
            p.line(&[[8., 4.], [16., 20.]], p.ink);
        }
        Icon::LineConstruction | Icon::LineInfinite => {
            let color = if enabled { theme::CONSTRUCTION } else { p.ink };
            for (a, b) in [(3., 7.), (10., 14.), (17., 21.)] {
                p.line(&[[a, 24. - a], [b, 24. - b]], color);
            }
            if icon == Icon::LineConstruction {
                p.circle(3., 21., 1.5, color);
                p.circle(21., 3., 1.5, color);
            } else {
                p.line(&[[1., 18.], [1., 23.], [6., 23.]], color);
                p.line(&[[18., 1.], [23., 1.], [23., 6.]], color);
            }
        }
        Icon::Rectangle => p.face(&[[3., 5.], [21., 5.], [21., 19.], [3., 19.]], p.fill),
        Icon::RectangleCenter => {
            p.face(&[[3., 5.], [21., 5.], [21., 19.], [3., 19.]], p.fill);
            p.line(&[[3., 5.], [21., 19.]], p.accent);
            p.line(&[[3., 19.], [21., 5.]], p.accent);
            p.circle(12., 12., 2., p.ink);
        }
        Icon::RectangleInclined => {
            p.face(&[[2., 11.], [16., 3.], [22., 13.], [8., 21.]], p.fill);
            // O primeiro lado e o marcador de ângulo reto revelam a construção.
            p.line(&[[2., 11.], [16., 3.]], p.accent);
            p.line(&[[12.5, 5.], [14.5, 8.5], [18., 6.5]], p.ink);
            p.circle(2., 11., 1.5, p.ink);
            p.circle(16., 3., 1.5, p.ink);
        }
        Icon::Circle => {
            p.circle(12., 12., 9., p.accent);
            p.line(&[[10., 12.], [14., 12.]], p.ink);
            p.line(&[[12., 10.], [12., 14.]], p.ink);
        }
        Icon::CircleDiameter => {
            p.circle(12., 12., 9., p.accent);
            p.line(&[[5.6, 18.4], [18.4, 5.6]], p.ink);
            p.circle(5.6, 18.4, 1.7, p.ink);
            p.circle(18.4, 5.6, 1.7, p.ink);
        }
        Icon::CircleTangent => {
            // Duas retas perpendiculares e os dois pontos de tangência.
            p.line(&[[3., 2.], [3., 21.], [22., 21.]], p.ink);
            p.circle(11., 13., 8., p.accent);
            p.circle(3., 13., 1.5, p.ink);
            p.circle(11., 21., 1.5, p.ink);
        }
        Icon::Arc => {
            let points: Vec<_> = (0..=20)
                .map(|i| {
                    let a = i as f32 / 20. * std::f32::consts::PI;
                    [12. + 9. * a.cos(), 16. - 9. * a.sin()]
                })
                .collect();
            p.line(&points, p.accent);
            p.circle(3., 16., 1.5, p.ink);
            p.circle(21., 16., 1.5, p.ink);
        }
        Icon::Trim => {
            p.circle(6., 18., 3., p.ink);
            p.circle(18., 18., 3., p.ink);
            p.line(&[[7., 15.], [19., 3.]], p.accent);
            p.line(&[[17., 15.], [5., 3.]], p.accent);
        }
        Icon::SketchOffset => {
            p.line(&[[3., 19.], [3., 8.], [8., 3.], [20., 3.]], p.ink);
            p.line(&[[8., 21.], [8., 11.], [11., 8.], [22., 8.]], p.accent);
            p.arrow([13., 16.], [20., 16.]);
        }
        Icon::SketchFillet => {
            p.line(
                &[[3., 22.], [3., 3.], [22., 3.]],
                egui::Color32::from_gray(175),
            );
            p.line(&[[7., 22.], [7., 14.]], p.accent);
            let points: Vec<_> = (0..=12)
                .map(|i| {
                    let angle = std::f32::consts::PI + i as f32 / 12. * std::f32::consts::FRAC_PI_2;
                    [14. + 7. * angle.cos(), 14. + 7. * angle.sin()]
                })
                .collect();
            p.line(&points, p.accent);
            p.line(&[[14., 7.], [22., 7.]], p.accent);
        }
        Icon::SketchMirror => {
            p.line(&[[12., 2.], [12., 22.]], p.ink);
            p.face(&[[3., 18.], [8., 7.], [8., 18.]], p.accent);
            p.line(&[[21., 18.], [16., 7.], [16., 18.], [21., 18.]], p.accent);
        }
        Icon::CircularPattern => {
            for i in 0..6 {
                let angle = i as f32 * std::f32::consts::TAU / 6.;
                p.circle(12. + 8. * angle.cos(), 12. + 8. * angle.sin(), 2., p.accent);
            }
            p.circle(12., 12., 1., p.ink);
        }
        Icon::Dimension => {
            p.line(&[[3., 3.], [3., 21.]], p.ink);
            p.line(&[[21., 3.], [21., 21.]], p.ink);
            p.line(&[[4., 12.], [20., 12.]], p.accent);
            p.arrow([9., 12.], [4., 12.]);
            p.arrow([15., 12.], [20., 12.]);
        }
        Icon::ConstraintHorizontalVertical => p.line(&[[5., 3.], [5., 19.], [21., 19.]], p.accent),
        Icon::ConstraintCoincident => {
            p.line(&[[3., 19.], [10., 12.], [21., 17.]], p.ink);
            p.circle(10., 12., 3., p.accent);
        }
        Icon::ConstraintTangent => {
            p.circle(12., 13., 6., p.accent);
            p.line(&[[2., 7.], [22., 7.]], p.ink);
        }
        Icon::ConstraintEqual => {
            p.line(&[[4., 8.], [20., 8.]], p.accent);
            p.line(&[[4., 16.], [20., 16.]], p.accent);
        }
        Icon::ConstraintParallel => {
            p.line(&[[4., 19.], [12., 4.]], p.accent);
            p.line(&[[12., 20.], [20., 5.]], p.accent);
        }
        Icon::ConstraintPerpendicular => {
            p.line(&[[3., 19.], [21., 19.]], p.accent);
            p.line(&[[12., 3.], [12., 19.]], p.accent);
            p.line(&[[12., 14.], [17., 14.], [17., 19.]], p.ink);
        }
        // Controles de reprodução da linha do tempo: barra indica o limite do histórico.
        Icon::First | Icon::Last => {
            let mirror = |x: f32| if icon == Icon::Last { 24. - x } else { x };
            p.line(&[[mirror(5.), 5.], [mirror(5.), 19.]], p.ink);
            p.face(
                &[[mirror(19.), 5.], [mirror(8.), 12.], [mirror(19.), 19.]],
                p.fill,
            );
        }
        Icon::Previous | Icon::Next => {
            let mirror = |x: f32| if icon == Icon::Next { 24. - x } else { x };
            p.face(
                &[[mirror(17.), 5.], [mirror(6.), 12.], [mirror(17.), 19.]],
                p.fill,
            );
        }
        Icon::Delete => {
            p.line(&[[4., 6.], [20., 6.]], p.ink);
            p.line(&[[9., 6.], [9., 3.], [15., 3.], [15., 6.]], p.ink);
            p.face(&[[6., 6.], [18., 6.], [17., 21.], [7., 21.]], p.fill);
            p.line(&[[10., 10.], [10., 17.]], p.accent);
            p.line(&[[14., 10.], [14., 17.]], p.accent);
        }
        Icon::Warning => {
            p.face(
                &[[12., 2.], [22., 21.], [2., 21.]],
                egui::Color32::from_rgb(255, 226, 160),
            );
            p.line(&[[12., 8.], [12., 15.]], theme::WARNING);
            p.circle(12., 18., 0.8, theme::WARNING);
        }
        // Direções da extrusão: o plano do perfil é a linha horizontal.
        Icon::OneSide | Icon::TwoSides | Icon::Symmetric => {
            let base = if icon == Icon::OneSide { 19. } else { 14. };
            p.line(&[[3., base], [21., base]], p.ink);
            p.arrow([12., base], [12., 3.]);
            match icon {
                Icon::TwoSides => p.arrow([12., base], [12., 19.]),
                Icon::Symmetric => p.arrow([12., base], [12., 24.]),
                _ => {}
            }
        }
        Icon::ToFace => {
            p.line(&[[3., 20.], [21., 20.]], p.ink);
            p.face(&[[3., 3.], [21., 3.], [21., 6.], [3., 6.]], p.fill);
            p.arrow([12., 20.], [12., 7.]);
        }
        Icon::Flip => {
            p.arrow([8., 18.], [8., 4.]);
            p.arrow([16., 6.], [16., 20.]);
        }
        // Resultado de operações entre corpos.
        Icon::Join | Icon::CutBody | Icon::IntersectBody => {
            let left = [[3., 6.], [14., 6.], [14., 17.], [3., 17.]];
            match icon {
                Icon::Join => {
                    p.face(
                        &[
                            [3., 6.],
                            [14., 6.],
                            [14., 10.],
                            [21., 10.],
                            [21., 21.],
                            [10., 21.],
                            [10., 17.],
                            [3., 17.],
                        ],
                        p.fill,
                    );
                    p.line(&[[15., 15.5], [19., 15.5]], p.accent);
                    p.line(&[[17., 13.5], [17., 17.5]], p.accent);
                }
                Icon::CutBody => {
                    p.face(&left, p.fill);
                    p.line(
                        &[[10., 10.], [21., 10.], [21., 21.], [10., 21.], [10., 10.]],
                        p.accent,
                    );
                    p.face(
                        &[[10., 10.], [14., 10.], [14., 17.], [10., 17.]],
                        egui::Color32::WHITE,
                    );
                }
                _ => {
                    p.line(
                        &[[3., 6.], [14., 6.], [14., 17.], [3., 17.], [3., 6.]],
                        p.ink,
                    );
                    p.line(
                        &[[10., 10.], [21., 10.], [21., 21.], [10., 21.], [10., 10.]],
                        p.ink,
                    );
                    p.face(&[[10., 10.], [14., 10.], [14., 17.], [10., 17.]], p.accent);
                }
            }
        }
        Icon::Align => {
            p.face(&[[3., 3.], [10., 3.], [10., 10.], [3., 10.]], p.fill);
            p.face(&[[14., 14.], [21., 14.], [21., 21.], [14., 21.]], p.fill);
            p.arrow([10., 10.], [14., 14.]);
            p.line(&[[3., 12.], [21., 12.]], p.ink);
        }
        Icon::Group => {
            p.line(&[[2., 6.], [2., 2.], [6., 2.]], p.accent);
            p.line(&[[18., 2.], [22., 2.], [22., 6.]], p.accent);
            p.line(&[[22., 18.], [22., 22.], [18., 22.]], p.accent);
            p.line(&[[6., 22.], [2., 22.], [2., 18.]], p.accent);
            p.face(&[[5., 6.], [12., 6.], [12., 12.], [5., 12.]], p.fill);
            p.face(&[[12., 12.], [19., 12.], [19., 18.], [12., 18.]], p.fill);
        }
        Icon::Section => {
            p.cube();
            p.face(
                &[[1., 13.], [14., 6.], [23., 11.], [10., 18.]],
                egui::Color32::from_rgba_unmultiplied(196, 82, 64, 120),
            );
            p.line(
                &[[1., 13.], [14., 6.], [23., 11.], [10., 18.], [1., 13.]],
                egui::Color32::from_rgb(160, 60, 48),
            );
        }
        Icon::Copy => {
            p.face(&[[8., 3.], [20., 3.], [20., 16.], [8., 16.]], p.fill);
            p.face(
                &[[4., 8.], [16., 8.], [16., 21.], [4., 21.]],
                egui::Color32::WHITE,
            );
            p.line(&[[7., 12.], [13., 12.]], p.accent);
            p.line(&[[7., 16.], [13., 16.]], p.accent);
        }
        // Mover por alinhamento: o elemento móvel (preenchido) encontra o destino.
        Icon::FaceToFace => {
            p.face(&[[2., 6.], [10., 6.], [10., 18.], [2., 18.]], p.fill);
            p.face(
                &[[14., 6.], [22., 6.], [22., 18.], [14., 18.]],
                egui::Color32::WHITE,
            );
            p.line(&[[10., 12.], [14., 12.]], p.accent);
            p.line(&[[12., 3.], [12., 21.]], p.accent);
        }
        Icon::EdgeToEdge => {
            p.line(&[[3., 20.], [11., 4.]], p.ink);
            p.line(&[[13., 20.], [21., 4.]], p.accent);
            p.arrow([7., 12.], [15., 12.]);
        }
        Icon::PointToPoint => {
            p.circle(5., 17., 2.5, p.ink);
            p.circle(19., 7., 2.5, p.accent);
            p.arrow([7., 15.], [16., 9.]);
        }
        // Rascunho de referência: lápis com plano tracejado na cor das referências.
        Icon::ReferenceSketch => {
            let violet = if enabled {
                theme::REFERENCE
            } else {
                egui::Color32::from_gray(165)
            };
            p.line(&[[3., 21.], [21., 21.]], violet);
            p.line(&[[3., 21.], [3., 5.]], violet);
            p.face(
                &[[7., 16.], [9., 10.], [18., 1.], [22., 5.], [13., 14.]],
                p.fill,
            );
            p.line(&[[9., 10.], [13., 14.]], violet);
        }
        Icon::StepFile | Icon::StlFile => {
            p.line(
                &[
                    [15., 3.],
                    [5., 3.],
                    [5., 21.],
                    [19., 21.],
                    [19., 7.],
                    [15., 3.],
                    [15., 7.],
                    [19., 7.],
                ],
                p.ink,
            );
            if icon == Icon::StepFile {
                p.face(&[[8., 11.], [12., 9.], [16., 11.], [12., 13.]], p.fill);
                p.face(&[[8., 11.], [12., 13.], [12., 18.], [8., 16.]], p.accent);
            } else {
                p.face(&[[8., 17.], [12., 9.], [16., 17.]], p.fill);
                p.line(&[[12., 9.], [12., 17.]], p.accent);
            }
        }
        Icon::Part => {
            // Componente: bloco com o marcador de peça no canto.
            p.cube();
            p.face(&[[15., 15.], [22., 15.], [22., 22.], [15., 22.]], p.fill);
            p.line(&[[15., 18.5], [22., 18.5]], p.accent);
        }
        Icon::Edit => {
            p.face(
                &[[15., 4.], [20., 9.], [9., 20.], [4., 20.], [4., 15.]],
                p.fill,
            );
            p.line(&[[13., 6.], [18., 11.]], p.ink);
            p.line(&[[4., 15.], [9., 20.]], p.accent);
        }
        Icon::Origin => {
            p.line(&[[5., 19.], [21., 19.]], p.ink);
            p.line(&[[5., 19.], [5., 3.]], p.ink);
            p.line(&[[5., 19.], [15., 9.]], p.accent);
            p.circle(5., 19., 2.5, p.accent);
        }
        Icon::Project => {
            p.line(
                &[
                    [16., 3.],
                    [5., 3.],
                    [5., 21.],
                    [20., 21.],
                    [20., 7.],
                    [16., 3.],
                ],
                p.ink,
            );
            p.face(&[[8., 12.], [12.5, 10.], [17., 12.], [12.5, 14.]], p.fill);
            p.face(&[[8., 12.], [12.5, 14.], [12.5, 18.], [8., 16.]], p.fill);
            p.face(&[[12.5, 14.], [17., 12.], [17., 16.], [12.5, 18.]], p.fill);
        }
        Icon::NewBody => {
            p.cube();
            p.face(
                &[[14., 14.], [22., 14.], [22., 22.], [14., 22.]],
                egui::Color32::WHITE,
            );
            p.line(&[[18., 15.5], [18., 20.5]], p.accent);
            p.line(&[[15.5, 18.], [20.5, 18.]], p.accent);
        }
    }
}

/// Um botão desenhado continua sendo um botão para teclado e acessibilidade.
/// O tooltip mantém o nome completo mesmo nos atalhos compactos sem texto.
pub(super) fn button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    large: bool,
    selected: bool,
) -> egui::Response {
    let size = if large {
        egui::vec2(76., 64.)
    } else {
        egui::vec2(30., 30.)
    };
    let response = ui.add(
        egui::Button::new("")
            .min_size(size)
            .selected(selected)
            .frame(selected),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if ui.is_rect_visible(response.rect) {
        if response.hovered() && ui.is_enabled() {
            ui.painter().rect_filled(response.rect, 5, theme::BLUE_FILL);
        }
        let icon_size = if large { 27. } else { 18. };
        let center = response.rect.center() - egui::vec2(0., if large { 10. } else { 0. });
        paint_with_ink(
            ui.painter(),
            egui::Rect::from_center_size(center, egui::Vec2::splat(icon_size)),
            icon,
            ui.is_enabled(),
            if ui.visuals().dark_mode {
                egui::Color32::from_rgb(220, 230, 242)
            } else {
                theme::INK
            },
        );
        if large {
            // O rótulo pode ocupar duas linhas sem invadir o grupo vizinho.
            let galley = ui.painter().layout(
                label.to_owned(),
                egui::FontId::proportional(11.),
                if ui.is_enabled() {
                    theme::INK
                } else {
                    theme::MUTED
                },
                72.,
            );
            ui.painter().galley(
                egui::pos2(
                    response.rect.center().x - galley.size().x / 2.,
                    response.rect.top() + 40.,
                ),
                galley,
                theme::INK,
            );
        }
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            5,
            egui::Stroke::new(1.5, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    response.on_hover_text(label)
}

/// Botão compacto das linhas da árvore (visibilidade, edição), na altura de uma linha.
pub(super) fn tree_button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    selected: bool,
) -> egui::Response {
    let response = ui.add(
        egui::Button::new("")
            .min_size(egui::Vec2::splat(20.))
            .selected(selected)
            .frame(selected),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if ui.is_rect_visible(response.rect) {
        if response.hovered() && ui.is_enabled() {
            ui.painter().rect_filled(response.rect, 4, theme::BLUE_FILL);
        }
        paint(
            ui.painter(),
            egui::Rect::from_center_size(response.rect.center(), egui::Vec2::splat(16.)),
            icon,
            ui.is_enabled(),
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            4,
            egui::Stroke::new(1.5, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    response.on_hover_text(label)
}

/// Linha de árvore com ícone, foco de teclado e elipse para nomes longos.
pub(super) fn entry(ui: &mut egui::Ui, icon: Icon, label: &str, selected: bool) -> egui::Response {
    entry_styled(ui, icon, label, selected, false)
}

/// Como `entry`; `dimmed` esmaece o rótulo de um elemento oculto (por ele ou por um nó
/// acima dele na árvore).
pub(super) fn entry_styled(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    selected: bool,
    dimmed: bool,
) -> egui::Response {
    let ink = if dimmed { theme::MUTED } else { theme::INK };
    let response = ui.add(
        egui::Button::new("")
            .min_size(egui::vec2(ui.available_width().max(40.), 22.))
            .selected(selected)
            .frame(selected),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if response.hovered() && ui.is_enabled() {
        ui.painter().rect_filled(response.rect, 4, theme::BLUE_FILL);
    }
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(response.rect.left() + 13., response.rect.center().y),
        egui::Vec2::splat(17.),
    );
    paint(ui.painter(), icon_rect, icon, ui.is_enabled());
    let mut job = egui::text::LayoutJob::simple(
        label.to_owned(),
        egui::FontId::proportional(13.),
        ink,
        (response.rect.width() - 34.).max(1.),
    );
    job.wrap.max_rows = 1;
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(
        egui::pos2(
            response.rect.left() + 28.,
            response.rect.center().y - galley.size().y / 2.,
        ),
        galley,
        ink,
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            5,
            egui::Stroke::new(1.5, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    response.on_hover_text(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

    fn frame(
        context: &egui::Context,
        events: Vec<egui::Event>,
        enabled: bool,
    ) -> TestResult<(egui::Response, egui::FullOutput)> {
        let mut response = None;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(420., 240.),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                response = Some(
                    ui.add_enabled_ui(enabled, |ui| {
                        button(ui, Icon::Extrude, "Extrusão", true, false)
                    })
                    .inner,
                );
            },
        );
        // O ensaio valida eventos e semântica, sem renderizador. A captura wgpu
        // separada verifica os pixels; descartar explicitamente suas texturas aqui.
        output.textures_delta.clear();
        Ok((response.ok_or("O botão não foi composto.")?, output))
    }
    fn pointer(position: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }
    #[test]
    fn icon_button_preserves_clicks_disabled_state_and_accessible_label() -> TestResult<()> {
        for enabled in [true, false] {
            let context = egui::Context::default();
            theme::apply(&context);
            let (response, _) = frame(&context, vec![], enabled)?;
            let center = response.rect.center();
            frame(&context, pointer(center, true), enabled)?;
            let (released, output) = frame(&context, pointer(center, false), enabled)?;
            assert_eq!(released.clicked(), enabled);
            if enabled {
                assert!(
                    output.platform_output.events.iter().any(|event| event
                        .widget_info()
                        .label
                        .as_deref()
                        == Some("Extrusão"))
                );
            }
        }
        Ok(())
    }
    #[test]
    fn icon_button_can_be_activated_with_the_keyboard() -> TestResult<()> {
        let context = egui::Context::default();
        let (response, _) = frame(&context, vec![], true)?;
        context.memory_mut(|memory| memory.request_focus(response.id));
        let (activated, _) = frame(
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            true,
        )?;
        assert!(activated.clicked());
        Ok(())
    }
}
