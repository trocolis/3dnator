//! Opções de criação de linhas. O tipo selecionado afeta apenas as próximas
//! entidades; escolher um tipo não converte geometria já publicada no documento.
use super::*;
use crate::desktop::project_editor::command_panel as panel;

pub(super) fn icon(kind: LineType) -> icons::Icon {
    match kind {
        LineType::Normal => icons::Icon::Line,
        LineType::Construction => icons::Icon::LineConstruction,
        LineType::Infinite => icons::Icon::LineInfinite,
    }
}

pub(super) fn cursor_icon(work: &Workspace) -> icons::Icon {
    match work.primitive() {
        Some(Primitive::SymmetricLine) => icons::Icon::LineSymmetric,
        Some(Primitive::CenterRectangle) => icons::Icon::RectangleCenter,
        Some(Primitive::InclinedRectangle) => icons::Icon::RectangleInclined,
        Some(Primitive::DiameterCircle) => icons::Icon::CircleDiameter,
        Some(Primitive::TangentCircle) => icons::Icon::CircleTangent,
        Some(Primitive::Line) => icon(work.line_type),
        _ => work.tool.icon(),
    }
}

/// Traço é compartilhado pelas primitivas; modo descreve o gesto de construção.
/// Trocar qualquer opção cancela a prévia para não misturar etapas incompatíveis.
pub(super) fn settings(ui: &mut egui::Ui, work: &mut Workspace) {
    let previous = (
        work.line_type,
        work.line_mode,
        work.rectangle_mode,
        work.circle_mode,
    );
    if matches!(
        work.tool,
        Tool::Line | Tool::Rectangle | Tool::Circle | Tool::Arc
    ) {
        line_type_row(
            ui,
            &mut work.line_type,
            matches!(work.tool, Tool::Line | Tool::Rectangle),
        );
    }
    match work.tool {
        Tool::Project => work.projector.settings(ui),
        Tool::Line => {
            panel::row(ui, text(T::SketchLineMode), |ui| {
                panel::choice_hints(
                    ui,
                    &mut work.line_mode,
                    &[
                        (
                            LineMode::Normal,
                            icons::Icon::Line,
                            T::SketchLineNormal,
                            T::SketchContinuousLineHint,
                        ),
                        (
                            LineMode::Symmetric,
                            icons::Icon::LineSymmetric,
                            T::Symmetric,
                            T::SketchSymmetricLineHint,
                        ),
                    ],
                );
            });
        }
        Tool::Rectangle => {
            panel::row(ui, text(T::SketchRectangleMode), |ui| {
                panel::choice_hints(
                    ui,
                    &mut work.rectangle_mode,
                    &[
                        (
                            RectangleMode::Normal,
                            icons::Icon::Rectangle,
                            T::SketchLineNormal,
                            T::SketchRectangleNormalHint,
                        ),
                        (
                            RectangleMode::Center,
                            icons::Icon::RectangleCenter,
                            T::SketchRectangleCenter,
                            T::SketchRectangleCenterHint,
                        ),
                        (
                            RectangleMode::Inclined,
                            icons::Icon::RectangleInclined,
                            T::SketchRectangleInclined,
                            T::SketchRectangleInclinedHint,
                        ),
                    ],
                );
            });
        }
        Tool::Circle => {
            panel::row(ui, text(T::SketchCircleMode), |ui| {
                panel::choice_hints(
                    ui,
                    &mut work.circle_mode,
                    &[
                        (
                            CircleMode::Normal,
                            icons::Icon::Circle,
                            T::SketchLineNormal,
                            T::SketchCircleNormalHint,
                        ),
                        (
                            CircleMode::Diameter,
                            icons::Icon::CircleDiameter,
                            T::SketchCircleDiameter,
                            T::SketchCircleDiameterHint,
                        ),
                        (
                            CircleMode::Tangent,
                            icons::Icon::CircleTangent,
                            T::SketchCircleTangent,
                            T::SketchCircleTangentHint,
                        ),
                    ],
                );
            });
        }
        Tool::Trim => panel::hint(ui, text(T::SketchTrimHint)),
        _ => {}
    }
    if previous
        != (
            work.line_type,
            work.line_mode,
            work.rectangle_mode,
            work.circle_mode,
        )
    {
        work.reset_drawing(ui.ctx());
    }
    if let Some(primitive) = work.primitive() {
        panel::hint(ui, text(T::JunctionToggleHint));
        work.drawing.infinite = work.tool == Tool::Line && work.line_type == LineType::Infinite;
        work.drawing.panel(ui, primitive, work.anchors.len());
    }
}

/// Linha de traço compartilhada pela criação e pelas operações de rascunho.
pub(super) fn line_type_row(ui: &mut egui::Ui, value: &mut LineType, allow_infinite: bool) {
    let mut options = vec![
        (
            LineType::Normal,
            icon(LineType::Normal),
            T::SketchLineNormal,
            T::SketchLineNormalHint,
        ),
        (
            LineType::Construction,
            icon(LineType::Construction),
            T::Construction,
            T::SketchLineConstructionHint,
        ),
    ];
    if allow_infinite {
        options.push((
            LineType::Infinite,
            icon(LineType::Infinite),
            T::SketchLineInfinite,
            T::SketchLineInfiniteHint,
        ));
    }
    panel::row(ui, text(T::SketchLineType), |ui| {
        panel::choice_hints(ui, value, &options);
    });
}
