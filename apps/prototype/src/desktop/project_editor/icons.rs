//! Ícones vetoriais originais do 3dnator, desenhados em uma grade lógica de 24 unidades.
//! Não dependem de fontes de símbolos nem de bitmaps: o mesmo desenho permanece nítido
//! com escala de interface e DPI diferentes. O traço escuro estrutura o objeto e uma
//! única cor de ação distingue sólido, rascunho e referência.
use super::{egui, theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Icon {
    New,
    Open,
    Save,
    SaveAs,
    Settings,
    Undo,
    Redo,
    Import,
    Export,
    Sketch,
    SketchProject,
    Image,
    Paint,
    Extrude,
    Revolve,
    Loft,
    Sweep,
    Label,
    Hole,
    Fillet,
    Chamfer,
    Shell,
    Boolean,
    Split,
    CheckSolid,
    PrintProfile,
    BedFace,
    PrintCheck,
    FitCheck,
    ComponentCut,
    Lip,
    ScrewBoss,
    SnapFit,
    Calibrate,
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
    LockOpen,
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
    SketchChamfer,
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
    SnapMidpoint,
    SnapCenter,
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
    OffsetPlane,
    WorkPlane,
    Home,
    PanCamera,
    OrbitCamera,
    CollapseUp,
    ExpandDown,
    ConstraintFix,
    ConstraintConcentric,
    ConstraintCollinear,
    ConstraintSymmetry,
    ConstraintPolygon,
    ConstraintManager,
}
mod draw;

/// Todos os ícones, na ordem do enum, para a folha de revisão visual.
const ALL: &[Icon] = &[
    Icon::New,
    Icon::Open,
    Icon::Save,
    Icon::SaveAs,
    Icon::Settings,
    Icon::Undo,
    Icon::Redo,
    Icon::Import,
    Icon::Export,
    Icon::Sketch,
    Icon::SketchProject,
    Icon::Image,
    Icon::Paint,
    Icon::Extrude,
    Icon::Revolve,
    Icon::Loft,
    Icon::Sweep,
    Icon::Label,
    Icon::Hole,
    Icon::Fillet,
    Icon::Chamfer,
    Icon::Shell,
    Icon::Boolean,
    Icon::Split,
    Icon::CheckSolid,
    Icon::PrintProfile,
    Icon::BedFace,
    Icon::PrintCheck,
    Icon::FitCheck,
    Icon::ComponentCut,
    Icon::Lip,
    Icon::ScrewBoss,
    Icon::SnapFit,
    Icon::Calibrate,
    Icon::Move,
    Icon::Mirror,
    Icon::Pattern,
    Icon::Measure,
    Icon::Parameters,
    Icon::Drawing,
    Icon::Fit,
    Icon::Cube,
    Icon::Front,
    Icon::Top,
    Icon::Side,
    Icon::Perspective,
    Icon::Eye,
    Icon::EyeOff,
    Icon::Close,
    Icon::Minimize,
    Icon::Maximize,
    Icon::Restore,
    Icon::RotateClockwise,
    Icon::RotateCounterclockwise,
    Icon::Search,
    Icon::Check,
    Icon::LockOpen,
    Icon::Select,
    Icon::Line,
    Icon::LineSymmetric,
    Icon::LineConstruction,
    Icon::LineInfinite,
    Icon::Rectangle,
    Icon::RectangleCenter,
    Icon::RectangleInclined,
    Icon::Circle,
    Icon::CircleDiameter,
    Icon::CircleTangent,
    Icon::Arc,
    Icon::Trim,
    Icon::SketchOffset,
    Icon::SketchFillet,
    Icon::SketchChamfer,
    Icon::SketchMirror,
    Icon::CircularPattern,
    Icon::RectangularPattern,
    Icon::Dimension,
    Icon::ConstraintHorizontalVertical,
    Icon::ConstraintCoincident,
    Icon::ConstraintTangent,
    Icon::ConstraintEqual,
    Icon::ConstraintParallel,
    Icon::ConstraintPerpendicular,
    Icon::SnapMidpoint,
    Icon::SnapCenter,
    Icon::Folder,
    Icon::First,
    Icon::Previous,
    Icon::Next,
    Icon::Last,
    Icon::Delete,
    Icon::Warning,
    Icon::OneSide,
    Icon::TwoSides,
    Icon::Symmetric,
    Icon::ToFace,
    Icon::Flip,
    Icon::Align,
    Icon::Group,
    Icon::Section,
    Icon::Copy,
    Icon::FaceToFace,
    Icon::EdgeToEdge,
    Icon::PointToPoint,
    Icon::ReferenceSketch,
    Icon::StepFile,
    Icon::StlFile,
    Icon::Join,
    Icon::CutBody,
    Icon::IntersectBody,
    Icon::NewBody,
    Icon::Part,
    Icon::Edit,
    Icon::Origin,
    Icon::OffsetPlane,
    Icon::WorkPlane,
    Icon::Home,
    Icon::PanCamera,
    Icon::OrbitCamera,
    Icon::CollapseUp,
    Icon::ExpandDown,
    Icon::ConstraintFix,
    Icon::ConstraintConcentric,
    Icon::ConstraintCollinear,
    Icon::ConstraintSymmetry,
    Icon::ConstraintPolygon,
    Icon::ConstraintManager,
];

pub(super) fn paint(painter: &egui::Painter, rect: egui::Rect, icon: Icon, enabled: bool) {
    draw::paint(painter, rect, icon, enabled, theme::ink(), false);
}
pub(super) fn paint_with_ink(
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: Icon,
    enabled: bool,
    ink: egui::Color32,
) {
    draw::paint(painter, rect, icon, enabled, ink, false);
}
/// `dark` adapta destaque, preenchimento e estado desabilitado à barra escura da janela,
/// onde os tons pensados para o fundo claro perdem contraste.
fn paint_styled(
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: Icon,
    enabled: bool,
    ink: egui::Color32,
    dark: bool,
) {
    draw::paint(painter, rect, icon, enabled, ink, dark);
}

/// Folha de contato dos ícones, usada pela captura de revisão visual.
pub(super) fn preview_sheet(ui: &mut egui::Ui) {
    draw::preview_sheet(ui);
}
/// A faixa usa medidas próprias; os botões maiores dos painéis de opções
/// conservam sua dimensão para não alterar as ferramentas de desenho.
pub(super) const RIBBON_BUTTON_SIZE: egui::Vec2 = egui::vec2(67., 54.);

enum ButtonLayout {
    Compact,
    Large,
    Ribbon,
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
    draw_button(
        ui,
        icon,
        label,
        if large {
            ButtonLayout::Large
        } else {
            ButtonLayout::Compact
        },
        selected,
    )
}

pub(super) fn ribbon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    selected: bool,
) -> egui::Response {
    draw_button(ui, icon, label, ButtonLayout::Ribbon, selected)
}

fn draw_button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    layout: ButtonLayout,
    selected: bool,
) -> egui::Response {
    let (size, icon_size, icon_shift, label_top, font_size) = match layout {
        ButtonLayout::Compact => (egui::vec2(30., 30.), 18., 0., None, 11.),
        ButtonLayout::Large => (egui::vec2(76., 64.), 27., 10., Some(40.), 11.),
        ButtonLayout::Ribbon => (RIBBON_BUTTON_SIZE, 24., 12., Some(29.), 10.5),
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
        let dark = ui.visuals().dark_mode;
        if response.hovered() && ui.is_enabled() {
            // O azul-claro do fundo claro apagaria o traço claro da barra escura.
            let hover = if dark {
                egui::Color32::from_rgb(69, 82, 101)
            } else {
                theme::blue_fill()
            };
            ui.painter().rect_filled(response.rect, 5, hover);
        }
        let center = response.rect.center() - egui::vec2(0., icon_shift);
        paint_styled(
            ui.painter(),
            egui::Rect::from_center_size(center, egui::Vec2::splat(icon_size)),
            icon,
            ui.is_enabled(),
            if dark {
                egui::Color32::from_rgb(238, 243, 250)
            } else {
                theme::ink()
            },
            dark,
        );
        if let Some(top) = label_top {
            // Limitar a duas linhas impede que legendas extensas invadam o
            // rodapé do grupo. O nome completo continua disponível na dica.
            let mut job = egui::text::LayoutJob::simple(
                label.to_owned(),
                egui::FontId::proportional(font_size),
                if ui.is_enabled() {
                    theme::ink()
                } else {
                    theme::muted()
                },
                size.x - 4.,
            );
            job.wrap.max_rows = 2;
            let galley = ui.painter().layout_job(job);
            ui.painter().galley(
                egui::pos2(
                    response.rect.center().x - galley.size().x / 2.,
                    response.rect.top() + top,
                ),
                galley,
                theme::ink(),
            );
        }
    }

    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            5,
            egui::Stroke::new(1.5, theme::accent()),
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
            ui.painter()
                .rect_filled(response.rect, 4, theme::blue_fill());
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
            egui::Stroke::new(1.5, theme::accent()),
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
    let ink = if dimmed { theme::muted() } else { theme::ink() };
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
        ui.painter()
            .rect_filled(response.rect, 4, theme::blue_fill());
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
            egui::Stroke::new(1.5, theme::accent()),
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
            theme::apply(&context, theme::Choice::Light);
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
