//! Opções de criação de linhas. O tipo selecionado afeta apenas as próximas
//! entidades; escolher um tipo não converte geometria já publicada no documento.
use super::*;

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
        ui.label(text(T::SketchLineType));
        ui.horizontal(|ui| {
            for (option, label, hint) in [
                (
                    LineType::Normal,
                    T::SketchLineNormal,
                    T::SketchLineNormalHint,
                ),
                (
                    LineType::Construction,
                    T::Construction,
                    T::SketchLineConstructionHint,
                ),
                (
                    LineType::Infinite,
                    T::SketchLineInfinite,
                    T::SketchLineInfiniteHint,
                ),
            ] {
                if option == LineType::Infinite
                    && !matches!(work.tool, Tool::Line | Tool::Rectangle)
                {
                    continue;
                }
                if icons::button(
                    ui,
                    icon(option),
                    text(label),
                    true,
                    work.line_type == option,
                )
                .on_hover_text(text(hint))
                .clicked()
                {
                    work.line_type = option;
                }
            }
        });
        ui.separator();
    }
    match work.tool {
        Tool::Project => work.projector.settings(ui),
        Tool::Line => {
            ui.label(text(T::SketchLineMode));
            ui.horizontal(|ui| {
                option(
                    ui,
                    &mut work.line_mode,
                    LineMode::Normal,
                    icons::Icon::Line,
                    T::SketchLineNormal,
                    T::SketchContinuousLineHint,
                );
                option(
                    ui,
                    &mut work.line_mode,
                    LineMode::Symmetric,
                    icons::Icon::LineSymmetric,
                    T::Symmetric,
                    T::SketchSymmetricLineHint,
                );
            });
        }
        Tool::Rectangle => {
            ui.label(text(T::SketchRectangleMode));
            ui.horizontal(|ui| {
                option(
                    ui,
                    &mut work.rectangle_mode,
                    RectangleMode::Normal,
                    icons::Icon::Rectangle,
                    T::SketchLineNormal,
                    T::SketchRectangleNormalHint,
                );
                option(
                    ui,
                    &mut work.rectangle_mode,
                    RectangleMode::Center,
                    icons::Icon::RectangleCenter,
                    T::SketchRectangleCenter,
                    T::SketchRectangleCenterHint,
                );
                option(
                    ui,
                    &mut work.rectangle_mode,
                    RectangleMode::Inclined,
                    icons::Icon::RectangleInclined,
                    T::SketchRectangleInclined,
                    T::SketchRectangleInclinedHint,
                );
            });
        }
        Tool::Circle => {
            ui.label(text(T::SketchCircleMode));
            ui.horizontal(|ui| {
                option(
                    ui,
                    &mut work.circle_mode,
                    CircleMode::Normal,
                    icons::Icon::Circle,
                    T::SketchLineNormal,
                    T::SketchCircleNormalHint,
                );
                option(
                    ui,
                    &mut work.circle_mode,
                    CircleMode::Diameter,
                    icons::Icon::CircleDiameter,
                    T::SketchCircleDiameter,
                    T::SketchCircleDiameterHint,
                );
                option(
                    ui,
                    &mut work.circle_mode,
                    CircleMode::Tangent,
                    icons::Icon::CircleTangent,
                    T::SketchCircleTangent,
                    T::SketchCircleTangentHint,
                );
            });
        }
        Tool::Trim => {
            icons::button(ui, icons::Icon::Trim, text(T::Trim), true, true)
                .on_hover_text(text(T::SketchTrimHint));
        }
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
        ui.separator();
        ui.label(text(T::SketchMeasurements));
        work.drawing.infinite = work.tool == Tool::Line && work.line_type == LineType::Infinite;
        work.drawing.panel(ui, primitive, work.anchors.len());
    }
}

fn option<M: Copy + PartialEq>(
    ui: &mut egui::Ui,
    mode: &mut M,
    value: M,
    icon: icons::Icon,
    label: T,
    hint: T,
) {
    if icons::button(ui, icon, text(label), true, *mode == value)
        .on_hover_text(text(hint))
        .clicked()
    {
        *mode = value;
    }
}
