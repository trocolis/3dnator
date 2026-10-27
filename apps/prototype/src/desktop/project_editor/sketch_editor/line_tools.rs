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

pub(super) fn settings(ui: &mut egui::Ui, kind: &mut LineType) {
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
            let selected = *kind == option;
            if icons::button(ui, icon(option), text(label), true, selected)
                .on_hover_text(text(hint))
                .clicked()
            {
                *kind = option;
            }
        }
    });
}
