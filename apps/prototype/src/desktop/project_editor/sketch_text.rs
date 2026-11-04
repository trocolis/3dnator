//! Texto transitório com captura, orientação, prévia no trabalhador e confirmação explícita.
use super::*;
use crate::desktop::project_editor::{command_panel as panel, sketch_editor::dynamic_input};
use nator_sketch::{
    Point2, Sketch,
    snapping::{Kind, Tracker},
    text::SketchText,
};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_DRAFT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, PartialEq)]
struct Properties {
    content: String,
    font: String,
    bold: bool,
    italic: bool,
    underline: bool,
    size: f64,
    rotation_deg: f64,
    anchor: Option<Point2>,
    attachment: Option<Kind>,
}
#[derive(Clone, PartialEq)]
struct FontKey {
    content: String,
    font: String,
    bold: bool,
    italic: bool,
    underline: bool,
}
impl Properties {
    fn font_key(&self) -> FontKey {
        FontKey {
            content: self.content.clone(),
            font: self.font.clone(),
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
        }
    }
    fn block(&self, id: u64, contours: Vec<Vec<Point2>>) -> SketchText {
        SketchText {
            id,
            anchor: 0,
            content: self.content.clone(),
            font: self.font.clone(),
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            size: self.size,
            rotation: self.rotation_deg.to_radians(),
            contours,
            points: vec![],
            entities: vec![],
        }
    }
}
#[derive(Clone)]
struct FontCache {
    key: FontKey,
    contours: Vec<Vec<Point2>>,
}
#[derive(Clone, PartialEq)]
struct Key {
    revision: u64,
    properties: Properties,
}
struct Answer {
    font: Option<FontCache>,
    sketch: Result<Arc<Sketch>, String>,
}

pub(super) struct Draft {
    tab: u64,
    sketch: SketchId,
    token: u64,
    editing: Option<u64>,
    properties: Properties,
    placed: bool,
    tracker: Tracker,
    direction: dynamic_input::Input,
    running: Option<(Key, Receiver<Answer>)>,
    ready: Option<(Key, Result<Arc<Sketch>, String>)>,
    font: Option<FontCache>,
    confirming: bool,
}
impl Draft {
    fn new(
        tab: u64,
        sketch: SketchId,
        editing: Option<u64>,
        properties: Properties,
        font: Option<FontCache>,
    ) -> Self {
        Self {
            tab,
            sketch,
            token: NEXT_DRAFT.fetch_add(1, Ordering::Relaxed),
            editing,
            properties,
            placed: editing.is_some(),
            tracker: Tracker::default(),
            direction: Default::default(),
            running: None,
            ready: None,
            font,
            confirming: false,
        }
    }
    pub(super) fn cancel(&mut self, context: &egui::Context) {
        self.direction.clear(context);
    }
    fn key(&self, revision: u64) -> Key {
        Key {
            revision,
            properties: self.properties.clone(),
        }
    }
    fn candidate(&self, revision: u64) -> Option<&Arc<Sketch>> {
        self.ready
            .as_ref()
            .filter(|(key, _)| *key == self.key(revision))
            .and_then(|(_, result)| result.as_ref().ok())
    }
    fn poll(
        &mut self,
        context: &egui::Context,
        source: &Sketch,
        parameters: &nator_document::parameters::Parameters,
        revision: u64,
    ) {
        let key = self.key(revision);
        let answer = self.running.as_ref().map(|(_, rx)| rx.try_recv());
        match answer {
            Some(Ok(answer)) => {
                if let Some((requested, _)) = self.running.take() {
                    if let Some(font) = answer.font {
                        self.font = Some(font);
                    }
                    if requested == key {
                        self.ready = Some((requested, answer.sketch));
                    }
                }
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                if let Some((requested, _)) = self.running.take() {
                    self.ready = Some((requested, Err(text(T::StaleResult).into())));
                }
            }
            _ => {}
        }
        if self.running.is_some()
            || self.ready.as_ref().is_some_and(|(k, _)| *k == key)
            || self.properties.anchor.is_none()
            || self.properties.content.trim().is_empty()
        {
            return;
        }
        let source = source.clone();
        let parameters = parameters.clone();
        let p = self.properties.clone();
        let editing = self.editing.unwrap_or(0);
        let cached = self.font.clone().filter(|cache| cache.key == p.font_key());
        let context = context.clone();
        let (tx, rx) = mpsc::channel();
        self.running = Some((key, rx));
        std::thread::spawn(move || {
            let font = cached.map(Ok).unwrap_or_else(|| {
                nator_commands::sketch_text::contours(
                    &p.content,
                    &p.font,
                    p.bold,
                    p.italic,
                    p.underline,
                )
                .map(|contours| FontCache {
                    key: p.font_key(),
                    contours,
                })
                .map_err(|e| e.to_string())
            });
            let answer = match font {
                Ok(font) => {
                    let result = nator_commands::sketch_text::candidate(
                        &source,
                        p.block(editing, font.contours.clone()),
                        p.anchor.unwrap_or([0.; 2]),
                        p.attachment,
                        &parameters,
                    )
                    .map(Arc::new)
                    .map_err(|e| e.to_string());
                    Answer {
                        font: Some(font),
                        sketch: result,
                    }
                }
                Err(error) => Answer {
                    font: None,
                    sketch: Err(error),
                },
            };
            let _ = tx.send(answer);
            context.request_repaint();
        });
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn diagnostic_for_capture(&self, revision: u64) -> String {
        format!(
            "ready={}, running={}, anchor={:?}, size={}, confirming={}, error={:?}",
            self.candidate(revision).is_some(),
            self.running.is_some(),
            self.properties.anchor,
            self.properties.size,
            self.confirming,
            self.ready.as_ref().and_then(|(_, r)| r.as_ref().err())
        )
    }

    #[cfg(all(test, feature = "occt"))]
    pub(super) fn token_for_capture(&self) -> u64 {
        self.token
    }

    #[cfg(all(test, feature = "occt"))]
    pub(super) fn configure_for_capture(
        &mut self,
        position: Point2,
        rotation_deg: f64,
        size: f64,
        content: &str,
    ) {
        self.properties.anchor = Some(position);
        self.properties.rotation_deg = rotation_deg;
        self.properties.size = size;
        self.properties.content = content.into();
        self.placed = true;
    }
}

impl Editor {
    pub(super) fn preset_sketch_text_for_capture(&mut self) {
        let Some(sketch) = self.current().sketch_edit.as_ref().map(|w| w.id) else {
            return;
        };
        let mut draft = Draft::new(
            self.current().id,
            sketch,
            None,
            Properties {
                content: "3Dnator".into(),
                font: "sans-serif".into(),
                bold: false,
                italic: false,
                underline: false,
                size: 3.5,
                rotation_deg: 10.,
                anchor: Some([15., 10.]),
                attachment: None,
            },
            None,
        );
        draft.placed = true;
        self.sketch_text = Some(draft);
    }

    pub(super) fn open_sketch_text(&mut self, context: &egui::Context) {
        let Some(sketch) = self.current().sketch_edit.as_ref().map(|w| w.id) else {
            return;
        };
        self.activate_selection(context);
        self.sketch_text = Some(Draft::new(
            self.current().id,
            sketch,
            None,
            Properties {
                content: text(T::SketchTextTool).into(),
                font: "sans-serif".into(),
                bold: false,
                italic: false,
                underline: false,
                size: 10.,
                rotation_deg: 0.,
                anchor: None,
                attachment: None,
            },
            None,
        ));
        self.status.set(text(T::SketchTextHint));
    }
    fn edit_sketch_text(
        &mut self,
        context: &egui::Context,
        sketch: SketchId,
        source: &Sketch,
        block: &SketchText,
    ) {
        let Ok(anchor) = source.point(block.anchor) else {
            return;
        };
        self.activate_selection(context);
        let properties = Properties {
            content: block.content.clone(),
            font: block.font.clone(),
            bold: block.bold,
            italic: block.italic,
            underline: block.underline,
            size: block.size,
            rotation_deg: block.rotation.to_degrees(),
            anchor: Some(anchor),
            attachment: None,
        };
        let font = FontCache {
            key: properties.font_key(),
            contours: block.contours.clone(),
        };
        self.sketch_text = Some(Draft::new(
            self.current().id,
            sketch,
            Some(block.id),
            properties,
            Some(font),
        ));
    }
    fn text_parameters(&self, sketch: SketchId) -> nator_document::parameters::Parameters {
        self.current()
            .session
            .document()
            .sketch_part(sketch)
            .and_then(|id| self.current().session.document().part(id))
            .map(|p| p.parameters.clone())
            .unwrap_or_default()
    }
    pub(super) fn text_preview_sketch(&self) -> Option<Sketch> {
        let draft = self.sketch_text.as_ref()?;
        if draft.tab != self.current().id
            || self.current().sketch_edit.as_ref().map(|w| w.id) != Some(draft.sketch)
        {
            return None;
        }
        draft
            .candidate(self.current().session.revision())
            .map(|s| s.as_ref().clone())
    }
    pub(super) fn sketch_text_panel(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.sketch_text.take() else {
            return;
        };
        if draft.tab != self.current().id
            || self.current().sketch_edit.as_ref().map(|w| w.id) != Some(draft.sketch)
        {
            draft.cancel(context);
            return;
        }
        let Some(source) = self.working_sketch() else {
            draft.cancel(context);
            return;
        };
        let revision = self.current().session.revision();
        let parameters = self.text_parameters(draft.sketch);
        if draft.confirming && !self.busy() {
            draft.confirming = false;
        }
        draft.poll(context, &source, &parameters, revision);
        self.label_fonts.poll(context);
        let fonts = self.label_fonts.names.clone();
        let ready = draft.candidate(revision).is_some() && !self.busy() && !draft.confirming;
        let error = draft
            .ready
            .as_ref()
            .filter(|(key, _)| *key == draft.key(revision))
            .and_then(|(_, r)| r.as_ref().err())
            .cloned();
        let footer = if let Some(error) = &error {
            panel::Footer::Failed(error)
        } else if ready {
            panel::Footer::Ready
        } else if draft.properties.anchor.is_some() {
            panel::Footer::Waiting
        } else {
            panel::Footer::None
        };
        let mut finish_direction = false;
        let keyboard_enter = context.input(|i| i.key_pressed(egui::Key::Enter));
        let outcome = panel::show(
            context,
            panel::Header {
                id: "sketch_text",
                icon: icons::Icon::Label,
                title: text(T::SketchTextTool),
            },
            self.canvas_rect,
            footer,
            ready,
            |ui| {
                ui.add_enabled_ui(!draft.confirming, |ui| {
                    panel::row(ui, text(T::LabelContent), |ui| {
                        ui.text_edit_singleline(&mut draft.properties.content)
                    });
                    panel::row(ui, text(T::LabelFont), |ui| {
                        egui::ComboBox::from_id_salt("sketch_text_font")
                            .selected_text(&draft.properties.font)
                            .width(150.)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut draft.properties.font,
                                    "sans-serif".into(),
                                    "sans-serif",
                                );
                                for font in &fonts {
                                    ui.selectable_value(
                                        &mut draft.properties.font,
                                        font.clone(),
                                        font,
                                    );
                                }
                            })
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut draft.properties.bold, text(T::LabelBold));
                        ui.checkbox(&mut draft.properties.italic, text(T::LabelItalic));
                        ui.checkbox(&mut draft.properties.underline, text(T::LabelUnderline));
                    });
                    panel::row(ui, text(T::LabelSize), |ui| {
                        let response = ui.add(panel::length_value(
                            context,
                            &mut draft.properties.size,
                            0.1..=10_000.,
                        ));
                        #[cfg(all(test, feature = "occt"))]
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("text_size_rect"), response.rect)
                        });
                        response
                    });
                    let mut position = draft.properties.anchor.unwrap_or([0.; 2]);
                    for (axis, label) in [(0, T::SketchTextX), (1, T::SketchTextY)] {
                        panel::row(ui, text(label), |ui| {
                            let response = ui.add(panel::length_value(
                                context,
                                &mut position[axis],
                                -1e9..=1e9,
                            ));
                            #[cfg(all(test, feature = "occt"))]
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(
                                    egui::Id::new(("text_position_rect", axis)),
                                    response.rect,
                                )
                            });
                            if response.changed() {
                                draft.properties.anchor = Some(position);
                                finish_direction = true;
                            }
                            response
                        });
                    }
                    panel::row(ui, text(T::RotationField), |ui| {
                        let response = ui.add(
                            panel::localized(egui::DragValue::new(
                                &mut draft.properties.rotation_deg,
                            ))
                            .range(-360.0..=360.0)
                            .suffix("°"),
                        );
                        if response.changed() {
                            draft.direction.fields[1] =
                                format!("{} deg", draft.properties.rotation_deg);
                            draft.direction.locked[1] = true;
                        }
                        response
                    });
                    ui.label(
                        egui::RichText::new(text(if draft.placed {
                            T::SketchTextAdjustHint
                        } else if draft.properties.anchor.is_none() {
                            T::SketchTextHint
                        } else {
                            T::SketchTextDirectionHint
                        }))
                        .small()
                        .color(theme::muted()),
                    );
                });
            },
        );
        if finish_direction {
            draft.placed = true;
            draft.direction.clear(context);
        }
        if outcome.cancel {
            draft.cancel(context);
            return;
        }
        draft.poll(context, &source, &parameters, revision);
        if outcome.ok
            && !keyboard_enter
            && !self.busy()
            && let Some(candidate) = draft.candidate(revision).cloned()
        {
            let document = self.current().session.document().clone();
            let tab = draft.tab;
            let id = draft.sketch;
            let token = draft.token;
            draft.confirming = true;
            self.start(context, None, move || {
                let command = Command::UpdateSketch {
                    id,
                    sketch: candidate.as_ref().clone(),
                };
                Ok(
                    match PreparedCommand::new(document, revision, command.clone()) {
                        Ok(prepared) => Outcome::SketchTextPrepared {
                            tab,
                            revision,
                            token,
                            command: prepared,
                        },
                        Err(error) => Outcome::SketchTextRejected {
                            tab,
                            revision,
                            token,
                            command,
                            error,
                        },
                    },
                )
            });
        }
        self.sketch_text = Some(draft);
    }
    pub(super) fn sketch_text_overlay(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        frame: &sketch_scene::Frame,
        source: &Sketch,
    ) -> bool {
        let Some(sketch) = self.current().sketch_edit.as_ref().map(|w| w.id) else {
            return false;
        };
        if self.shell.camera_mode.is_some() {
            return self.sketch_text.is_some();
        }
        let cursor = response
            .hover_pos()
            .filter(|p| !view_cube::rect(response.rect).contains(*p))
            .and_then(|p| frame.cursor(p));
        if self.sketch_text.is_none() {
            if (response.double_clicked() || response.triple_clicked())
                && let Some(cursor) = cursor
                && let Ok(Some(entity)) = nator_sketch::tools::pick_entity(
                    source,
                    cursor,
                    8. / frame.pixels_per_mm(cursor),
                )
                && let Some(block) = source.text_of(entity).cloned()
            {
                self.edit_sketch_text(ui.ctx(), sketch, source, &block);
                return true;
            }
            return false;
        }
        let Some(mut draft) = self.sketch_text.take() else {
            return false;
        };
        if draft.confirming {
            self.sketch_text = Some(draft);
            return true;
        }
        let snap = cursor
            .filter(|_| !ui.input(|i| i.modifiers.alt))
            .and_then(|p| {
                self.current().sketch_edit.as_ref().and_then(|w| {
                    w.text_snap(source, p, 8. / frame.pixels_per_mm(p), &draft.tracker)
                })
            });
        let point = snap.as_ref().map(|s| s.point).or(cursor);
        let painter = ui.painter().with_clip_rect(response.rect);
        if let Some(snap) = &snap {
            for (a, b) in &snap.guides {
                frame.path(
                    &painter,
                    [*a, *b],
                    egui::Stroke::new(1., theme::accent()),
                    true,
                );
            }
            if let Some(at) = frame.screen(snap.point) {
                painter.circle_stroke(at, 5., egui::Stroke::new(2., theme::accent()));
            }
        }
        let mut confirm_direction = false;
        if let Some(anchor) = draft.properties.anchor {
            if let Some(at) = frame.screen(anchor) {
                painter.circle_stroke(at, 4., egui::Stroke::new(1.5, theme::accent()));
            }
            if !draft.placed {
                if let Some(point) = point {
                    draft.direction.cursor = Some(point);
                }
                draft.direction.infinite = true;
                let parameters = self.text_parameters(sketch);
                let output = draft.direction.show(
                    ui.ctx(),
                    frame,
                    nator_sketch::drafting::Primitive::Line,
                    &[anchor],
                    &parameters,
                    None,
                    true,
                );
                if let Some(preview) = output.preview {
                    draft.properties.rotation_deg = preview.values[1].to_degrees();
                    frame.path(
                        &painter,
                        [anchor, preview.point],
                        egui::Stroke::new(1.5, theme::accent()),
                        true,
                    );
                    confirm_direction = output.confirm;
                }
                if output.cancel {
                    draft.cancel(ui.ctx());
                    return true;
                }
            }
        }
        if response.clicked()
            && let Some(point) = point
        {
            if draft.properties.anchor.is_none() {
                draft.properties.anchor = Some(point);
                draft.properties.attachment = snap.as_ref().map(|s| s.kind);
                draft.tracker.capture(point);
                draft.direction.start(point);
                draft.direction.infinite = true;
                self.status.set(text(T::SketchTextDirectionHint));
            } else if !draft.placed {
                confirm_direction = true;
            }
        }
        if confirm_direction {
            draft.placed = true;
            draft.direction.clear(ui.ctx());
            self.status.set(text(T::SketchTextAdjustHint));
        }
        self.sketch_text = Some(draft);
        true
    }
    pub(super) fn reject_sketch_text(
        &mut self,
        tab: u64,
        revision: u64,
        token: u64,
        command: Command,
        error: nator_commands::CommandError,
    ) {
        if self.current().id != tab
            || self.current().session.revision() != revision
            || !self
                .sketch_text
                .as_ref()
                .is_some_and(|d| d.token == token && d.confirming)
        {
            return;
        }
        if let Some(draft) = self.sketch_text.as_mut() {
            draft.confirming = false;
        }
        self.reject_command(tab, command, error);
    }

    pub(super) fn receive_sketch_text_prepared(
        &mut self,
        context: &egui::Context,
        tab: u64,
        revision: u64,
        token: u64,
        command: PreparedCommand,
    ) {
        if self.current().id != tab
            || self.current().session.revision() != revision
            || !self
                .sketch_text
                .as_ref()
                .is_some_and(|d| d.tab == tab && d.token == token && d.confirming)
        {
            return;
        }
        let Some(mut draft) = self.sketch_text.take() else {
            return;
        };
        self.commit_prepared(tab, command);
        if self.current().session.revision() != revision {
            draft.cancel(context);
        } else {
            draft.confirming = false;
            self.sketch_text = Some(draft);
        }
    }
}
