//! Ponte entre a faixa do rascunho e intenções geométricas próprias. A interface
//! coleta seleção/expressões; a edição é validada pelo rascunho e o solver roda no worker.
use super::*;
use crate::desktop::project_editor::{command_panel as panel, icons::Icon};
use nator_sketch::edit::{self, Action, Options};

pub(in crate::desktop::project_editor) fn edit_label(action: Action) -> T {
    match action {
        Action::Move => T::MoveMenu,
        Action::Offset => T::OffsetField,
        Action::Fillet => T::FilletShort,
        Action::Chamfer => T::SketchChamfer,
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
        Action::Move => Icon::Move,
        Action::Offset => Icon::SketchOffset,
        Action::Fillet => Icon::SketchFillet,
        Action::Chamfer => Icon::SketchChamfer,
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
        Action::Offset => T::SketchOffsetSelectHint,
        Action::Move => T::SketchSelectOne,
        Action::Fillet | Action::Chamfer => T::SketchCornersHint,
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
            Some(c.id) == work.constraint
                && matches!(
                    c.kind,
                    ConstraintKind::Angle { .. } | ConstraintKind::LineAngle { .. }
                )
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
    if action == Action::Mirror {
        nator_sketch::modifications::mirror(
            &mut sketch,
            &work.selected,
            work.mirror_axis
                .ok_or_else(|| text(T::SketchMirrorAxisHint).to_owned())?,
            work.operation_line_type,
        )
        .map_err(sketch_error)?;
        return Ok(sketch);
    }
    if action == Action::Offset {
        let second = if work.offset_mode == nator_sketch::modifications::OffsetMode::Double {
            parameters
                .evaluate(
                    &work.offset_second_expression,
                    nator_document::parameters::Dimension::Length,
                )
                .map_err(sketch_error)?
        } else {
            value
        };
        nator_sketch::modifications::offset(
            &mut sketch,
            &work.selected,
            work.offset_mode,
            value,
            second,
            work.operation_line_type,
        )
        .map_err(sketch_error)?;
        return Ok(sketch);
    }
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
        self.sanitize_sketch_selection();
        let source = self.working_sketch();
        let Some(work) = self.current_mut().sketch_edit.as_mut() else {
            return;
        };
        work.select_tool(Tool::Select);
        work.tool_panel_open = false;
        work.active_action = Some(action);
        work.operation.selecting = work.selected.is_empty();
        work.operation.focus = action == Action::Offset && !work.selected.is_empty();
        if action == Action::Offset
            && work.selected.len() == 1
            && let Some(source) = &source
        {
            let seed = work.selected[0];
            work.operation.seed = Some(seed);
            if let Ok(chain) = nator_sketch::connected::chain(source, seed) {
                work.selected = chain;
            }
        }
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
        let scene = self.scene_key();
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
                let selection = panel::selection(
                    ui,
                    text(T::SketchEntities),
                    count,
                    None,
                    if action == Action::Offset {
                        work.operation.selecting
                    } else {
                        !work.mirror_axis_active
                    },
                );
                if selection.activate {
                    work.mirror_axis_active = false;
                    if action == Action::Offset {
                        work.operation.selecting = true;
                    }
                }
                if selection.clear {
                    work.selected.clear();
                    work.constraint = None;
                    work.operation.selecting = true;
                    work.operation.seed = None;
                }
                if action == Action::Mirror {
                    let description = work.mirror_axis.map(|id| format!("{} {id}", text(T::Line)));
                    let selection = panel::selection(
                        ui,
                        text(T::SketchMirrorAxis),
                        usize::from(work.mirror_axis.is_some()),
                        description.as_deref(),
                        work.mirror_axis_active,
                    );
                    if selection.activate {
                        work.mirror_axis_active = true;
                    }
                    if selection.clear {
                        work.mirror_axis = None;
                        work.mirror_axis_active = true;
                    }
                }
                if matches!(action, Action::Mirror | Action::Offset) {
                    ui.label(text(T::SketchLineType));
                    ui.horizontal(|ui| {
                        for (kind, label) in [
                            (LineType::Normal, T::SketchLineNormal),
                            (LineType::Construction, T::Construction),
                            (LineType::Infinite, T::SketchLineInfinite),
                        ] {
                            if icons::button(
                                ui,
                                line_tools::icon(kind),
                                text(label),
                                true,
                                work.operation_line_type == kind,
                            )
                            .clicked()
                            {
                                work.operation_line_type = kind;
                            }
                        }
                    });
                }
                if matches!(action, Action::Fillet | Action::Chamfer) {
                    ui.label(text(T::SketchOffsetMouseHint));
                }
                if action == Action::Offset {
                    ui.label(text(T::SketchOffsetMouseHint));
                    if ui
                        .checkbox(&mut work.operation.whole_chain, text(T::SketchWholeChain))
                        .changed()
                        && let Some(seed) = work.operation.seed
                    {
                        work.selected = if work.operation.whole_chain {
                            nator_sketch::connected::chain(&source, seed)
                                .unwrap_or_else(|_| vec![seed])
                        } else {
                            vec![seed]
                        };
                    }
                    use nator_sketch::modifications::OffsetMode;
                    ui.horizontal(|ui| {
                        for (mode, label) in [
                            (OffsetMode::Normal, T::SketchLineNormal),
                            (OffsetMode::Double, T::SketchOffsetDouble),
                            (OffsetMode::Symmetric, T::Symmetric),
                        ] {
                            if ui
                                .selectable_value(&mut work.offset_mode, mode, text(label))
                                .changed()
                            {
                                work.operation.second_side = false;
                                work.operation.handle_drag = None;
                            }
                        }
                    });
                }
                if action.needs_value() {
                    let angular = action == Action::Dimension
                        && source.constraints.iter().any(|c| {
                            Some(c.id) == work.constraint
                                && matches!(
                                    c.kind,
                                    ConstraintKind::Angle { .. } | ConstraintKind::LineAngle { .. }
                                )
                        });
                    panel::row(
                        ui,
                        text(if angular {
                            T::SketchAngleValue
                        } else {
                            T::SketchValue
                        }),
                        |ui| {
                            if panel::expression(ui, &mut work.dimension_expression).changed() {
                                work.operation.confirm = false;
                            }
                        },
                    );
                }
                if action == Action::Offset {
                    if work.offset_mode == nator_sketch::modifications::OffsetMode::Double {
                        panel::row(ui, text(T::SketchOffsetSecond), |ui| {
                            if panel::expression(ui, &mut work.offset_second_expression).changed() {
                                work.operation.confirm = false;
                            }
                        });
                    } else if work.offset_mode == nator_sketch::modifications::OffsetMode::Symmetric
                    {
                        ui.label(text(T::SketchOffsetPerSide));
                    }
                }
                if action.is_pattern() {
                    panel::row(ui, text(T::PatternCount), |ui| {
                        ui.add(egui::DragValue::new(&mut work.pattern_count).range(2..=100));
                    });
                }
                if action == Action::Dimension {
                    ui.checkbox(&mut work.reference, text(T::ReferenceDimension));
                }
                let result = if operation_preview::supported(action) {
                    match operation_preview::Request::from(work, action, &parameters) {
                        Ok(request) => {
                            if let Err(error) = work
                                .operation
                                .cache
                                .prepare(&source, &request, scene, context)
                            {
                                ui.label(error);
                            }
                            work.operation
                                .cache
                                .result(&source, &request, scene)
                                .cloned()
                                .unwrap_or_else(|| Err(text(T::SketchPreviewPreparing).into()))
                        }
                        Err(error) => Err(error),
                    }
                } else {
                    candidate(&source, work, action, &parameters)
                };
                match result {
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
            let next = if operation_preview::supported(action) {
                operation_preview::Request::from(work, action, &parameters).and_then(|request| {
                    work.operation
                        .cache
                        .result(&source, &request, scene)
                        .cloned()
                        .unwrap_or_else(|| Err(text(T::SketchPreviewPreparing).into()))
                })
            } else {
                candidate(&source, work, action, &parameters)
            };
            let edited_dimension = (action == Action::Dimension)
                .then_some(work.constraint)
                .flatten();
            work.active_action = None;
            match next {
                Ok(sketch) => {
                    work.selected.retain(|id| sketch.entity(*id).is_ok());
                    if let Some(id) = edited_dimension {
                        self.solve_sketch_with(context, sketch, SolveMode::Dimension(id, false));
                    } else {
                        self.solve_sketch(context, sketch);
                    }
                }
                Err(message) => self.status.error(message),
            }
        }
    }
}
