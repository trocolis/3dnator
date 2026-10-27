//! Ponte entre a faixa do rascunho e intenções geométricas próprias. A interface
//! coleta seleção/expressões; a edição é validada pelo rascunho e o solver roda no worker.
use super::*;
use crate::desktop::project_editor::{command_panel as panel, icons::Icon};
use nator_sketch::edit::{self, Action, Options};

pub(in crate::desktop::project_editor) fn edit_label(action: Action) -> T {
    match action {
        Action::Offset => T::OffsetField,
        Action::Fillet => T::FilletShort,
        Action::Chamfer => T::Chamfer,
        Action::Extend => T::SketchExtendShort,
        Action::Mirror => T::MirrorShort,
        Action::CircularPattern => T::CircularPattern,
        Action::RectangularPattern => T::RectangularPattern,
        Action::Dimension => T::SketchDimension,
        Action::HorizontalVertical => T::HorizontalVertical,
        Action::Coincident => T::Coincident,
        Action::Tangent => T::Tangent,
        Action::Equal => T::EqualLength,
        Action::Parallel => T::Parallel,
        Action::Perpendicular => T::Perpendicular,
        Action::Construction => T::ConstructionGeometry,
        Action::FixPoint => T::FixPoint,
        Action::Delete => T::DeleteEntity,
    }
}
pub(in crate::desktop::project_editor) fn edit_icon(action: Action) -> Icon {
    match action {
        Action::Offset => Icon::SketchOffset,
        Action::Fillet => Icon::SketchFillet,
        Action::Chamfer => Icon::Chamfer,
        Action::Extend => Icon::Line,
        Action::Mirror => Icon::SketchMirror,
        Action::CircularPattern => Icon::CircularPattern,
        Action::RectangularPattern => Icon::RectangularPattern,
        Action::Dimension => Icon::Dimension,
        Action::HorizontalVertical => Icon::ConstraintHorizontalVertical,
        Action::Coincident => Icon::ConstraintCoincident,
        Action::Tangent => Icon::ConstraintTangent,
        Action::Equal => Icon::ConstraintEqual,
        Action::Parallel => Icon::ConstraintParallel,
        Action::Perpendicular => Icon::ConstraintPerpendicular,
        Action::Construction => Icon::ReferenceSketch,
        Action::FixPoint => Icon::Origin,
        Action::Delete => Icon::Delete,
    }
}
fn hint(action: Action) -> T {
    match action {
        Action::Mirror => T::SketchMirrorSelectionHint,
        Action::CircularPattern => T::SketchCircularSelectionHint,
        Action::RectangularPattern => T::SketchRectangularSelectionHint,
        Action::Tangent => T::SketchTangentSelectionHint,
        Action::Coincident => T::SketchCoincidentSelectionHint,
        _ if action.selection_count() == 1 => T::SketchSelectOne,
        _ => T::SketchSelectTwo,
    }
}

/// Cópia transitória: nem validar um formulário nem cancelá-lo modifica o documento.
fn candidate(
    source: &Sketch,
    work: &Workspace,
    action: Action,
    parameters: &nator_document::parameters::Parameters,
) -> Result<Sketch, String> {
    let angular = action == Action::Dimension
        && source.constraints.iter().any(|c| {
            Some(c.id) == work.constraint && matches!(c.kind, ConstraintKind::Angle { .. })
        });
    let value = if action.needs_value() {
        parameters
            .evaluate(
                &work.dimension_expression,
                if angular {
                    nator_document::parameters::Dimension::Angle
                } else {
                    nator_document::parameters::Dimension::Length
                },
            )
            .map_err(sketch_error)?
    } else {
        work.dimension
    };
    let mut sketch = source.clone();
    // Uma cota escolhida na lista mantém a identidade ao mudar valor ou referência.
    if action == Action::Dimension
        && let Some(id) = work.constraint
    {
        let constraint = sketch
            .constraints
            .iter_mut()
            .find(|c| c.id == id)
            .filter(|c| dimension_value(&c.kind).is_some())
            .ok_or_else(|| text(T::SketchSelectDimension).to_owned())?;
        set_dimension_value(&mut constraint.kind, value);
        constraint.reference = work.reference;
        bind_dimension(&mut sketch, id, work);
        sketch.validate().map_err(sketch_error)?;
        return Ok(sketch);
    }
    let outcome = edit::apply(
        &mut sketch,
        action,
        &work.selected,
        Options {
            value_mm: value,
            count: work.pattern_count,
            reference: work.reference,
        },
    )
    .map_err(sketch_error)?;
    if let Some(id) = outcome.dimension {
        bind_dimension(&mut sketch, id, work);
    }
    Ok(sketch)
}

impl Editor {
    pub(in crate::desktop::project_editor) fn open_sketch_action(
        &mut self,
        context: &egui::Context,
        action: Action,
    ) {
        let Some(work) = self.current_mut().sketch_edit.as_mut() else {
            return;
        };
        work.select_tool(Tool::Select);
        work.active_action = Some(action);
        // O Enter que ativou um botão por teclado não confirma o painel recém-aberto.
        context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
    }

    pub(in crate::desktop::project_editor) fn sketch_action_panel(
        &mut self,
        context: &egui::Context,
    ) {
        let Some(work) = self.current().sketch_edit.as_ref() else {
            return;
        };
        let Some(action) = work.active_action else {
            return;
        };
        let Some(source) = self.working_sketch() else {
            return;
        };
        let parameters = self
            .current()
            .session
            .document()
            .sketch_part(work.id)
            .and_then(|id| self.current().session.document().part(id))
            .map(|p| p.parameters.clone())
            .unwrap_or_default();
        let canvas = self.canvas_rect;
        let busy = self.busy();
        let Some(work) = self.current_mut().sketch_edit.as_mut() else {
            return;
        };
        let outcome = panel::show_validated(
            context,
            panel::Header {
                id: "sketch_action",
                icon: edit_icon(action),
                title: text(edit_label(action)),
            },
            canvas,
            panel::Footer::None,
            !busy,
            |ui| {
                ui.label(
                    egui::RichText::new(text(hint(action)))
                        .small()
                        .color(theme::MUTED),
                );
                let count = if action == Action::Dimension && work.constraint.is_some() {
                    1
                } else {
                    work.selected.len()
                };
                let selection = panel::selection(ui, text(T::SketchEntities), count, None, true);
                if selection.clear {
                    work.selected.clear();
                    work.constraint = None;
                }
                if action.needs_value() {
                    let angular = action == Action::Dimension
                        && source.constraints.iter().any(|c| {
                            Some(c.id) == work.constraint
                                && matches!(c.kind, ConstraintKind::Angle { .. })
                        });
                    panel::row(
                        ui,
                        text(if angular {
                            T::SketchAngleValue
                        } else {
                            T::SketchValue
                        }),
                        |ui| {
                            panel::expression(ui, &mut work.dimension_expression);
                        },
                    );
                }
                if action.is_pattern() {
                    panel::row(ui, text(T::PatternCount), |ui| {
                        ui.add(egui::DragValue::new(&mut work.pattern_count).range(2..=100));
                    });
                }
                if action == Action::Dimension {
                    ui.checkbox(&mut work.reference, text(T::ReferenceDimension));
                }
                match candidate(&source, work, action, &parameters) {
                    Ok(_) => true,
                    Err(message) => {
                        ui.label(egui::RichText::new(message).small().color(theme::DANGER));
                        false
                    }
                }
            },
        );
        if outcome.cancel {
            work.active_action = None;
        } else if outcome.ok {
            let next = candidate(&source, work, action, &parameters);
            work.active_action = None;
            match next {
                Ok(sketch) => {
                    work.selected.retain(|id| sketch.entity(*id).is_ok());
                    self.solve_sketch(context, sketch);
                }
                Err(message) => self.status = message,
            }
        }
    }
}
