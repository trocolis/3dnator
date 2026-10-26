//! Folhas guardam associação à peça/montagem; exportar regenera com a revisão capturada.
use super::*;
use nator_document::{SheetId, SheetRecord};
use nator_drawing::{Paper, Primitive, Section, Specification, Style};
pub(super) struct Draft {
    pub tab: u64,
    id: Option<SheetId>,
    target: Option<PartId>,
    spec: Specification,
    pub preview: Option<nator_drawing::Sheet>,
}
impl Editor {
    pub(super) fn open_drawing(&mut self, record: Option<SheetRecord>) {
        let (id, target, spec) = record
            .map(|r| (Some(r.id), r.target, r.specification))
            .unwrap_or_else(|| (None, self.current().active_part, Specification::default()));
        self.drawing_edit = Some(Draft {
            tab: self.current().id,
            id,
            target,
            spec,
            preview: None,
        });
    }
    pub(super) fn drawing_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.drawing_edit.take() else {
            return;
        };
        let Some(tab) = self.tabs.iter().find(|t| t.id == draft.tab) else {
            return;
        };
        let document = tab.session.document().clone();
        let revision = tab.session.revision();
        let mut open = true;
        let mut action = None;
        egui::Window::new(text(T::Drawing))
            .open(&mut open)
            .default_width(720.)
            .show(ctx, |ui| {
                ui.label(text(T::MeshDrawingNotice));
                ui.add_enabled_ui(!self.busy(), |ui| {
                    ui.label(text(T::DrawingTitle));
                    ui.text_edit_singleline(&mut draft.spec.title);
                    ui.horizontal(|ui| {
                        ui.label(text(T::Paper));
                        ui.selectable_value(&mut draft.spec.paper, Paper::A4, "A4");
                        ui.selectable_value(&mut draft.spec.paper, Paper::A3, "A3");
                        ui.label(text(T::Scale));
                        ui.add(
                            egui::DragValue::new(&mut draft.spec.scale)
                                .range(0.001..=1000.)
                                .speed(0.05),
                        );
                    });
                    ui.horizontal(|ui| {
                        let mut blue = draft.spec.style == Style::Blueprint;
                        if ui.checkbox(&mut blue, text(T::Blueprint)).changed() {
                            draft.spec.style = if blue {
                                Style::Blueprint
                            } else {
                                Style::Conventional
                            };
                        }
                        ui.checkbox(&mut draft.spec.hidden_lines, text(T::HiddenLines));
                        ui.checkbox(&mut draft.spec.dimensions, text(T::DrawingDimensions));
                    });
                    let mut section = draft.spec.section.is_some();
                    if ui.checkbox(&mut section, text(T::Section)).changed() {
                        draft.spec.section = section.then_some(Section {
                            origin: [0.; 3],
                            normal: [0., 0., 1.],
                        });
                    }
                    if let Some(s) = &mut draft.spec.section {
                        ui.label(text(T::SectionPosition));
                        ui.add(
                            egui::DragValue::new(&mut s.origin[2])
                                .range(-1e9..=1e9)
                                .speed(0.1),
                        );
                    }
                    ui.horizontal_wrapped(|ui| {
                        for (label, id) in [
                            (T::Preview, 0),
                            (T::ExportPdf, 1),
                            (T::ExportPng, 2),
                            (T::SaveSheet, 3),
                        ] {
                            if ui.button(text(label)).clicked() {
                                action = Some(id);
                            }
                        }
                    });
                });
                if let Some(sheet) = &draft.preview {
                    let width = ui.available_width().min(800.);
                    let scale = width / sheet.size[0] as f32;
                    let height = sheet.size[1] as f32 * scale;
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                    let painter = ui.painter_at(rect);
                    let (bg, fg) = if sheet.style == Style::Blueprint {
                        (
                            egui::Color32::from_rgb(17, 49, 85),
                            egui::Color32::from_rgb(225, 239, 250),
                        )
                    } else {
                        (egui::Color32::WHITE, egui::Color32::BLACK)
                    };
                    painter.rect_filled(rect, 0., bg);
                    let map = |p: [f64; 2]| rect.min + egui::vec2(p[0] as f32, p[1] as f32) * scale;
                    for p in &sheet.primitives {
                        match p {
                            Primitive::Line {
                                a,
                                b,
                                width,
                                dashed,
                            } => {
                                let points = [map(*a), map(*b)];
                                let stroke =
                                    egui::Stroke::new((*width as f32 * scale).max(0.4), fg);
                                if *dashed {
                                    for s in egui::Shape::dashed_line(&points, stroke, 3., 2.) {
                                        painter.add(s);
                                    }
                                } else {
                                    painter.line_segment(points, stroke);
                                }
                            }
                            Primitive::Text { at, size, value } => {
                                painter.text(
                                    map(*at),
                                    egui::Align2::LEFT_BOTTOM,
                                    value,
                                    egui::FontId::monospace((*size as f32 * scale).max(3.)),
                                    fg,
                                );
                            }
                        }
                    }
                }
            });
        if let Some(action) = action {
            if action == 3 {
                let id = draft.id.or_else(|| {
                    document
                        .sheets()
                        .iter()
                        .find(|s| {
                            s.target == draft.target && s.specification.title == draft.spec.title
                        })
                        .map(|s| s.id)
                });
                self.prepare_command(
                    ctx,
                    draft.tab,
                    Command::SetSheet {
                        id,
                        target: draft.target,
                        specification: draft.spec.clone(),
                    },
                );
            } else {
                let spec = draft.spec.clone();
                let target = draft.target;
                let tab = draft.tab;
                self.start(ctx, None, move || {
                    let mesh = document.drawing_mesh(target).map_err(|e| e.to_string())?;
                    let sheet = nator_drawing::generate(&mesh, &spec).map_err(|e| e.to_string())?;
                    if action == 0 {
                        return Ok(Outcome::Drawing {
                            tab,
                            revision,
                            sheet,
                        });
                    }
                    let extension = if action == 1 { "pdf" } else { "png" };
                    let Some(mut path) = rfd::FileDialog::new()
                        .add_filter(extension, &[extension])
                        .set_file_name(format!("drawing.{extension}"))
                        .save_file()
                    else {
                        return Ok(Outcome::Cancelled);
                    };
                    path.set_extension(extension);
                    let bytes = if action == 1 {
                        nator_drawing::pdf(&sheet)
                    } else {
                        nator_drawing::png(&sheet, 150)
                    }
                    .map_err(|e| e.to_string())?;
                    nator_project_io::write_export(&path, &bytes).map_err(|e| e.to_string())?;
                    Ok(Outcome::Exported)
                });
            }
        }
        if open {
            self.drawing_edit = Some(draft);
        }
    }
}
