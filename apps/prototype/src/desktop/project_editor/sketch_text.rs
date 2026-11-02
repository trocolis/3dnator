//! Ferramenta Texto do rascunho: painel com as opções do Rótulo (sem cor) e dois cliques
//! no canvas, o ponto de inserção e a direção. O contorno vem do núcleo no trabalhador;
//! o bloco é editável depois com duplo clique numa letra.
use super::*;
use crate::desktop::project_editor::command_panel as panel;
use nator_sketch::{Point2, text::SketchText};

pub(super) struct Draft {
    tab: u64,
    sketch: SketchId,
    /// Bloco existente em edição; `None` cria um novo.
    editing: Option<u64>,
    content: String,
    font: String,
    bold: bool,
    italic: bool,
    underline: bool,
    size: f64,
    rotation_deg: f64,
    /// Primeiro clique: ponto de inserção, em coordenadas do rascunho.
    anchor: Option<Point2>,
}

impl Draft {
    fn ready(&self) -> bool {
        !self.content.trim().is_empty() && (self.anchor.is_some() || self.editing.is_some())
    }
}

impl Editor {
    pub(super) fn open_sketch_text(&mut self, context: &egui::Context) {
        let Some(sketch) = self.current().sketch_edit.as_ref().map(|w| w.id) else {
            return;
        };
        self.activate_selection(context);
        self.sketch_text = Some(Draft {
            tab: self.current().id,
            sketch,
            editing: None,
            content: text(T::SketchTextTool).into(),
            font: "sans-serif".into(),
            bold: false,
            italic: false,
            underline: false,
            size: 10.,
            rotation_deg: 0.,
            anchor: None,
        });
        self.status.set(text(T::SketchTextHint));
    }

    /// Duplo clique numa letra: reabre o bloco com os seus valores.
    fn edit_sketch_text(&mut self, sketch: SketchId, block: &SketchText) {
        self.sketch_text = Some(Draft {
            tab: self.current().id,
            sketch,
            editing: Some(block.id),
            content: block.content.clone(),
            font: block.font.clone(),
            bold: block.bold,
            italic: block.italic,
            underline: block.underline,
            size: block.size,
            rotation_deg: block.rotation.to_degrees(),
            anchor: None,
        });
    }

    pub(super) fn sketch_text_panel(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.sketch_text.take() else {
            return;
        };
        if draft.tab != self.current().id
            || self.current().sketch_edit.as_ref().map(|w| w.id) != Some(draft.sketch)
        {
            return;
        }
        self.label_fonts.poll(context);
        let fonts = self.label_fonts.names.clone();
        let busy = self.busy();
        let ready = draft.ready() && !busy;
        let outcome = panel::show(
            context,
            panel::Header {
                id: "sketch_text",
                icon: icons::Icon::Label,
                title: text(T::SketchTextTool),
            },
            self.canvas_rect,
            panel::Footer::None,
            ready,
            |ui| {
                panel::row(ui, text(T::LabelContent), |ui| {
                    ui.text_edit_singleline(&mut draft.content)
                });
                panel::row(ui, text(T::LabelFont), |ui| {
                    egui::ComboBox::from_id_salt("sketch_text_font")
                        .selected_text(&draft.font)
                        .width(150.)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut draft.font, "sans-serif".into(), "sans-serif");
                            for font in &fonts {
                                ui.selectable_value(&mut draft.font, font.clone(), font);
                            }
                        })
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut draft.bold, text(T::LabelBold));
                    ui.checkbox(&mut draft.italic, text(T::LabelItalic));
                    ui.checkbox(&mut draft.underline, text(T::LabelUnderline));
                });
                panel::row(ui, text(T::LabelSize), |ui| {
                    let context = ui.ctx().clone();
                    ui.add(panel::length_value(
                        &context,
                        &mut draft.size,
                        0.1..=10_000.,
                    ))
                });
                panel::row(ui, text(T::RotationField), |ui| {
                    ui.add(
                        panel::localized(egui::DragValue::new(&mut draft.rotation_deg))
                            .range(-360.0..=360.0)
                            .suffix("°"),
                    )
                });
                if draft.editing.is_none() {
                    ui.label(
                        egui::RichText::new(text(if draft.anchor.is_none() {
                            T::SketchTextHint
                        } else {
                            T::SketchTextDirectionHint
                        }))
                        .small()
                        .color(theme::muted()),
                    );
                }
            },
        );
        if outcome.cancel {
            return;
        }
        if outcome.ok && draft.ready() {
            self.commit_sketch_text(context, draft);
            return;
        }
        self.sketch_text = Some(draft);
    }

    /// Cliques no canvas enquanto o painel está aberto. Devolve `true` quando o clique
    /// pertence ao texto e não deve chegar às outras ferramentas.
    pub(super) fn sketch_text_overlay(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        frame: &sketch_scene::Frame,
        source: &nator_sketch::Sketch,
    ) -> bool {
        let Some(sketch) = self.current().sketch_edit.as_ref().map(|w| w.id) else {
            return false;
        };
        let cursor = response.hover_pos().and_then(|p| frame.cursor(p));
        // Duplo clique numa letra abre o bloco para edição.
        if self.sketch_text.is_none() {
            if response.double_clicked()
                && let Some(cursor) = cursor
                && let Ok(Some(entity)) = nator_sketch::tools::pick_entity(
                    source,
                    cursor,
                    8. / frame.pixels_per_mm(cursor),
                )
                && let Some(block) = source.text_of(entity).cloned()
            {
                self.edit_sketch_text(sketch, &block);
                return true;
            }
            return false;
        }
        let Some(draft) = self.sketch_text.as_mut() else {
            return false;
        };
        if draft.editing.is_some() {
            return false;
        }
        let painter = ui.painter().with_clip_rect(response.rect);
        let stroke = egui::Stroke::new(1.5, theme::accent());
        if let Some(anchor) = draft.anchor
            && let Some(at) = frame.screen(anchor)
        {
            painter.circle_stroke(at, 5., stroke);
            if let Some(to) = cursor.and_then(|c| frame.screen(c)) {
                painter.line_segment([at, to], stroke);
            }
        } else if let Some(at) = cursor.and_then(|c| frame.screen(c)) {
            painter.circle_stroke(at, 5., stroke);
        }
        if !response.clicked() {
            return draft.anchor.is_some();
        }
        let Some(cursor) = cursor else {
            return true;
        };
        match draft.anchor {
            None => {
                draft.anchor = Some(cursor);
                self.status.set(text(T::SketchTextDirectionHint));
            }
            Some(anchor) => {
                let d = [cursor[0] - anchor[0], cursor[1] - anchor[1]];
                if d[0].hypot(d[1]) > 1e-9 {
                    draft.rotation_deg = d[1].atan2(d[0]).to_degrees();
                }
                if let Some(draft) = self.sketch_text.take()
                    && draft.ready()
                {
                    self.commit_sketch_text(ui.ctx(), draft);
                }
            }
        }
        true
    }

    /// O contorno sai do núcleo no trabalhador; a volta cria ou troca o bloco e resolve.
    fn commit_sketch_text(&mut self, context: &egui::Context, draft: Draft) {
        let tab = draft.tab;
        let sketch = draft.sketch;
        self.start(context, None, move || {
            let contours = nator_commands::sketch_text::contours(
                &draft.content,
                &draft.font,
                draft.bold,
                draft.italic,
                draft.underline,
            )
            .map_err(|e| e.to_string())?;
            Ok(Outcome::SketchTextOutline {
                tab,
                sketch,
                block: Box::new(SketchText {
                    id: draft.editing.unwrap_or(0),
                    anchor: 0,
                    content: draft.content,
                    font: draft.font,
                    bold: draft.bold,
                    italic: draft.italic,
                    underline: draft.underline,
                    size: draft.size,
                    rotation: draft.rotation_deg.to_radians(),
                    contours,
                    points: Vec::new(),
                    entities: Vec::new(),
                }),
                anchor: draft.anchor,
            })
        });
    }

    pub(super) fn receive_sketch_text(
        &mut self,
        context: &egui::Context,
        tab: u64,
        sketch: SketchId,
        mut block: SketchText,
        anchor: Option<Point2>,
    ) {
        if self.current().id != tab
            || self.current().sketch_edit.as_ref().map(|w| w.id) != Some(sketch)
        {
            self.status.warning(text(T::StaleResult));
            return;
        }
        let Some(mut candidate) = self.working_sketch() else {
            return;
        };
        let result = if let Some(existing) = candidate
            .texts
            .iter()
            .find(|t| t.id == block.id && block.id != 0)
        {
            block.anchor = existing.anchor;
            candidate.update_text(block)
        } else {
            match anchor.map(|p| candidate.add_point(p)) {
                Some(Ok(point)) => {
                    block.anchor = point;
                    candidate.add_text(block).map(|_| ())
                }
                Some(Err(error)) => Err(error),
                None => Ok(()),
            }
        };
        match result {
            Ok(()) => self.solve_sketch(context, candidate),
            Err(error) => self.status.error(error.to_string()),
        }
    }
}
