//! Pintura transacional: a prévia usa o trabalhador de comandos; apenas OK
//! publica a aparência. O rascunho recorta a face ao longo da normal do plano.
use super::live_preview::PreviewStatus;
use super::{
    command_panel::{self as panel, Footer},
    *,
};
use nator_document::{
    PaintMask, SurfacePaint,
    features::{FaceRef, RegionRef},
};
use nator_geometry::RgbColor;

pub(super) struct Draft {
    pub tab: u64,
    pub part: PartId,
    pub face: Option<(BodyId, FaceRef)>,
    sketch: Option<SketchId>,
    regions: Vec<RegionRef>,
    pub(super) rgb: [u8; 3],
    pub(super) hex: String,
}
fn parse_hex(value: &str) -> Option<[u8; 3]> {
    let digits = value.strip_prefix('#')?;
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&digits[0..2], 16).ok()?,
        u8::from_str_radix(&digits[2..4], 16).ok()?,
        u8::from_str_radix(&digits[4..6], 16).ok()?,
    ])
}
fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}
impl Draft {
    pub(super) fn command(&self) -> Option<Command> {
        let (body, face) = self.face.as_ref()?;
        let color = RgbColor::new(parse_hex(&self.hex)?);
        let mask = match self.sketch {
            Some(sketch) if !self.regions.is_empty() => Some(PaintMask {
                sketch,
                region: if self.regions.len() == 1 {
                    self.regions[0].clone()
                } else {
                    RegionRef::Multiple(self.regions.clone())
                },
            }),
            Some(_) => return None,
            None => None,
        };
        Some(Command::PaintSurface {
            body: *body,
            paint: SurfacePaint {
                face: face.clone(),
                mask,
                color,
            },
        })
    }
}
impl Editor {
    pub(super) fn open_paint(&mut self) {
        let Some(part) = self.current().active_part else {
            return;
        };
        if self.current().sketch_edit.is_some() {
            self.finish_sketch();
        }
        let face = self
            .selected_pick()
            .filter(|p| p.part == part)
            .and_then(|pick| {
                self.current()
                    .session
                    .document()
                    .face_at(pick)
                    .ok()
                    .map(|f| (pick.body, f))
            });
        self.pick_mode = navigation::PickMode::Face;
        self.paint_edit = Some(Draft {
            tab: self.current().id,
            part,
            face,
            sketch: None,
            regions: Vec::new(),
            rgb: [62, 175, 102],
            hex: "#3EAF66".into(),
        });
    }
    pub(super) fn paint_panel(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.paint_edit.take() else {
            return;
        };
        let Some(part) = self
            .find_tab(draft.tab)
            .and_then(|t| t.session.document().part(draft.part))
            .cloned()
        else {
            return;
        };
        let areas = draft.sketch.and_then(|s| self.profile_areas(s, ctx));
        let command = draft.command();
        let error;
        let footer = match command.as_ref().map(|c| self.preview_status(c)) {
            None => Footer::None,
            Some(PreviewStatus::Waiting) => Footer::Waiting,
            Some(PreviewStatus::Ready) => Footer::Ready,
            Some(PreviewStatus::Failed(e)) => {
                error = e.to_string();
                Footer::Failed(&error)
            }
        };
        let outcome = panel::show(
            ctx,
            panel::Header {
                id: "paint",
                icon: icons::Icon::Paint,
                title: text(T::Paint),
            },
            self.canvas_rect,
            footer,
            command.is_some() && !self.busy(),
            |ui| {
                ui.label(text(T::PaintPickHint));
                panel::row(ui, text(T::FacesField), |ui| {
                    let name = draft
                        .face
                        .as_ref()
                        .and_then(|(id, _)| part.bodies.iter().find(|b| b.id == *id))
                        .map(|b| b.name.as_str())
                        .unwrap_or(text(T::Select));
                    if ui.button(name).clicked() {
                        draft.face = None;
                    }
                });
                ui.separator();
                ui.label(text(T::PaintColor));
                ui.horizontal_wrapped(|ui| {
                    for rgb in [
                        [255, 255, 255],
                        [30, 30, 30],
                        [220, 65, 65],
                        [243, 180, 40],
                        [62, 175, 102],
                        [58, 130, 220],
                        [161, 99, 208],
                        [166, 168, 170],
                    ] {
                        let color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        if ui
                            .add(
                                egui::Button::new(" ")
                                    .fill(color)
                                    .min_size(egui::vec2(25., 25.)),
                            )
                            .clicked()
                        {
                            draft.rgb = rgb;
                            draft.hex = hex(rgb);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.color_edit_button_srgb(&mut draft.rgb).changed() {
                        draft.hex = hex(draft.rgb);
                    }
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut draft.hex)
                                .desired_width(100.)
                                .char_limit(7),
                        )
                        .changed()
                        && let Some(rgb) = parse_hex(&draft.hex)
                    {
                        draft.rgb = rgb;
                    }
                });
                if parse_hex(&draft.hex).is_none() {
                    ui.colored_label(theme::WARNING, text(T::PaintInvalidHex));
                }
                ui.separator();
                ui.label(text(T::PaintBoundary));
                let old = draft.sketch;
                egui::ComboBox::from_id_salt("paint_sketch")
                    .selected_text(
                        draft
                            .sketch
                            .and_then(|id| part.sketches.iter().find(|s| s.id == id))
                            .map(|s| s.sketch.name.as_str())
                            .unwrap_or(text(T::PaintWholeFace)),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.sketch, None, text(T::PaintWholeFace));
                        for sketch in &part.sketches {
                            ui.selectable_value(
                                &mut draft.sketch,
                                Some(sketch.id),
                                &sketch.sketch.name,
                            );
                        }
                    });
                if old != draft.sketch {
                    draft.regions.clear();
                }
                if draft.sketch.is_some() {
                    match &areas {
                        Some(Ok(Some(areas))) if old == draft.sketch => {
                            for (index, reference) in areas.references.iter().enumerate() {
                                let mut selected = draft.regions.contains(reference);
                                if ui
                                    .checkbox(
                                        &mut selected,
                                        format!("{} {}", text(T::ProjectionArea), index + 1),
                                    )
                                    .changed()
                                {
                                    if selected {
                                        draft.regions.push(reference.clone());
                                    } else {
                                        draft.regions.retain(|r| r != reference);
                                    }
                                }
                            }
                            if areas.references.is_empty() {
                                ui.label(text(T::PaintNoAreas));
                            }
                        }
                        Some(Err(e)) => {
                            ui.colored_label(theme::WARNING, e);
                        }
                        _ => {
                            ui.label(text(T::FindingClosedAreas));
                        }
                    }
                }
            },
        );
        if outcome.cancel {
            self.preview_command(draft.tab, None, None, None);
            return;
        }
        if outcome.ok
            && let Some(command) = draft.command()
        {
            self.apply_previewed(ctx, draft.tab, command);
            return;
        }
        self.preview_command(draft.tab, draft.command(), None, None);
        self.paint_edit = Some(draft);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hexadecimal_requires_six_ascii_rgb_digits() {
        assert_eq!(parse_hex("#fF007a"), Some([255, 0, 122]));
        for invalid in ["FFFFFF", "#FFF", "#GG0000", "#é0000", "#FFFFFF0"] {
            assert_eq!(parse_hex(invalid), None);
        }
        assert_eq!(hex([1, 10, 255]), "#010AFF");
    }
}
