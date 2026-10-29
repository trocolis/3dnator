//! Controles e posicionamento de rótulos; fontes e B-rep são preparados no worker.
use super::*;
use nator_document::{assembly::Pick, features::LabelMode};

#[derive(Default)]
pub(in crate::desktop::project_editor) struct Fonts {
    pub names: Vec<String>,
    receiver: Option<Receiver<Vec<String>>>,
    requested: bool,
}
impl Fonts {
    pub fn poll(&mut self, context: &egui::Context) {
        if !self.requested {
            self.requested = true;
            let (sender, receiver) = mpsc::channel();
            self.receiver = Some(receiver);
            let context = context.clone();
            std::thread::spawn(move || {
                #[cfg(feature = "occt")]
                let names = nator_kernel::Solid::font_names().unwrap_or_default();
                #[cfg(not(feature = "occt"))]
                let names = Vec::new();
                let _ = sender.send(names);
                context.request_repaint();
            });
        }
        if let Some(names) = self.receiver.as_ref().and_then(|r| r.try_recv().ok()) {
            self.names = names;
            self.receiver = None;
        }
    }
}
pub(super) struct Drag {
    frame: sketch_scene::Frame,
    cursor: [f64; 2],
    original: LabelSpec,
}
impl Draft {
    pub(in crate::desktop::project_editor) fn capture_label_pick(&mut self, pick: &Pick) {
        if self.tool == Tool::Label && self.base == Some(pick.body) {
            self.label.position = pick.placement.inverse_apply(pick.point);
            self.label.normal = pick.placement.inverse_vector(pick.normal);
            self.label.uv = None;
        }
    }
}
pub(super) fn panel(ui: &mut egui::Ui, d: &mut Draft, fonts: &[String]) {
    let description = d.kept_topology.as_ref().map(describe_topology);
    let selection = panel::selection(
        ui,
        text(T::FacesField),
        usize::from(description.is_some()),
        description.as_deref(),
        d.field == Some(Field::Faces),
    );
    if selection.activate {
        d.field = Some(Field::Faces);
    }
    if selection.clear {
        d.kept_topology = None;
        d.field = Some(Field::Faces);
    }
    panel::row(ui, text(T::LabelContent), |ui| {
        ui.text_edit_singleline(&mut d.label.text)
    });
    panel::row(ui, text(T::LabelFont), |ui| {
        egui::ComboBox::from_id_salt("label_font")
            .selected_text(&d.label.font_family)
            .width(150.)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut d.label.font_family, "sans-serif".into(), "sans-serif");
                for font in fonts {
                    ui.selectable_value(&mut d.label.font_family, font.clone(), font);
                }
            })
    });
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut d.label.bold, text(T::LabelBold));
        ui.checkbox(&mut d.label.italic, text(T::LabelItalic));
        ui.checkbox(&mut d.label.underline, text(T::LabelUnderline));
    });
    panel::row(ui, text(T::LabelSize), |ui| {
        panel::expression(ui, &mut d.label.size)
    });
    panel::row(ui, text(T::LabelMode), |ui| {
        panel::choice(
            ui,
            &mut d.label.mode,
            &[
                (LabelMode::Surface, Icon::Paint, T::Surface),
                (LabelMode::Raised, Icon::Extrude, T::LabelRaised),
                (LabelMode::Engraved, Icon::Hole, T::LabelEngraved),
            ],
        )
    });
    if d.label.mode != LabelMode::Surface {
        panel::row(ui, text(T::HoleDepth), |ui| {
            panel::expression(ui, &mut d.label.depth)
        });
    }
    panel::row(ui, text(T::RotationField), |ui| {
        panel::expression(ui, &mut d.label.rotation)
    });
    panel::row(ui, text(T::PaintColor), |ui| {
        let mut rgb = d.label.color.channels();
        let response = ui.color_edit_button_srgb(&mut rgb);
        d.label.color = nator_geometry::RgbColor::new(rgb);
        response
    });
    ui.label(text(T::Position));
    ui.horizontal(|ui| {
        for value in &mut d.label.position {
            if ui.add(egui::DragValue::new(value).speed(0.1)).changed() {
                d.label.uv = None;
            }
        }
    });
    ui.label(
        egui::RichText::new(text(T::LabelHint))
            .small()
            .color(theme::MUTED),
    );
}
impl Editor {
    pub(in crate::desktop::project_editor) fn label_gizmo(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
    ) -> bool {
        let Some(mut d) = self.feature_edit.take() else {
            return false;
        };
        let consumed = self.draw_label_gizmo(ui, response, &mut d);
        self.feature_edit = Some(d);
        consumed
    }
    fn draw_label_gizmo(&self, ui: &egui::Ui, response: &egui::Response, d: &mut Draft) -> bool {
        if d.tool != Tool::Label || !d.complete() || d.field.is_some() || self.busy() {
            return false;
        }
        let Ok(placement) = sketch_scene::part_placement(self.current(), d.part) else {
            return false;
        };
        let Ok(plane) = nator_sketch::Plane::from_normal(d.label.position, d.label.normal) else {
            return false;
        };
        let frame = sketch_scene::Frame {
            camera: self.viewport.camera.clone(),
            bounds: self.display_bounds(),
            rect: response.rect,
            placement,
            plane,
        };
        let Some(center) = frame.screen([0., 0.]) else {
            return false;
        };
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let hovered =
            pointer.is_some_and(|p| response.rect.contains(p) && p.distance(center) < 12.);
        let painter = ui.painter().with_clip_rect(response.rect);
        painter.circle_filled(center, 5., theme::ACCENT);
        painter.circle_stroke(center, 8., egui::Stroke::new(1.5, theme::INK));
        if hovered || d.label_drag.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if hovered
            && ui.input(|i| i.pointer.primary_pressed())
            && let Some(cursor) = pointer.and_then(|p| frame.cursor(p))
        {
            d.label_drag = Some(Drag {
                frame,
                cursor,
                original: d.label.clone(),
            });
        }
        if d.label_drag.is_some()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            if let Some(drag) = d.label_drag.take() {
                d.label = drag.original;
            }
            return true;
        }
        if let Some(drag) = &d.label_drag {
            if let Some(cursor) = pointer.and_then(|p| drag.frame.cursor(p)) {
                let delta = [cursor[0] - drag.cursor[0], cursor[1] - drag.cursor[1]];
                d.label.position = drag.frame.plane.world(delta);
                d.label.uv = None;
            }
            if !ui.input(|i| i.pointer.primary_down()) {
                d.label_drag = None;
            }
            return true;
        }
        false
    }
}
