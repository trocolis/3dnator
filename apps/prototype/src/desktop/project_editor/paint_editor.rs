//! Balde de tinta: seleção de células/superfícies, paleta e confirmação única.
//! A preferência de cor personalizada vive fora do documento e do histórico.
use super::{
    command_panel::{self as panel, Footer},
    live_preview::PreviewStatus,
    *,
};
use nator_commands::painting::{Picked, Target};
use nator_geometry::RgbColor;
const DEFAULT_CUSTOM: [u8; 3] = [62, 175, 102];
const PRESETS: [[[u8; 3]; 8]; 2] = [
    [
        [255, 255, 255],
        [30, 30, 30],
        [220, 65, 65],
        [243, 180, 40],
        [62, 175, 102],
        [58, 130, 220],
        [161, 99, 208],
        [166, 168, 170],
    ],
    [
        [225, 230, 236],
        [102, 75, 58],
        [255, 150, 150],
        [255, 224, 135],
        [160, 222, 184],
        [163, 202, 250],
        [216, 181, 241],
        [238, 152, 206],
    ],
];
pub(super) struct Palette {
    pub custom: [u8; 3],
    writer: Option<(mpsc::Sender<[u8; 3]>, std::thread::JoinHandle<()>)>,
    errors: Option<Receiver<String>>,
}
impl Default for Palette {
    fn default() -> Self {
        Self {
            custom: DEFAULT_CUSTOM,
            writer: None,
            errors: None,
        }
    }
}
impl Palette {
    pub fn load(path: PathBuf) -> Self {
        let custom = nator_project_io::read_color_preference(&path).unwrap_or(DEFAULT_CUSTOM);
        let (sender, receiver) = mpsc::channel();
        let (errors, error_receiver) = mpsc::channel();
        let failures = errors.clone();
        let writer = std::thread::Builder::new()
            .name("paint-preferences".into())
            .spawn(move || {
                while let Ok(mut color) = receiver.recv() {
                    // Serializar e juntar eventos rápidos impede uma gravação antiga de
                    // sobrescrever a cor mais recente. Somente uma thread escreve o arquivo.
                    while let Ok(next) = receiver.try_recv() {
                        color = next;
                    }
                    if let Err(error) = nator_project_io::write_color_preference(&path, color) {
                        let _ = errors.send(error.to_string());
                    }
                }
            })
            .map(|worker| (sender, worker));
        let writer = match writer {
            Ok(writer) => Some(writer),
            Err(error) => {
                let _ = failures.send(error.to_string());
                None
            }
        };
        Self {
            custom,
            writer,
            errors: Some(error_receiver),
        }
    }
    fn remember(&mut self, color: [u8; 3]) {
        if self.custom == color {
            return;
        }
        self.custom = color;
        if let Some((sender, _)) = &self.writer {
            let _ = sender.send(color);
        }
    }
}
impl Drop for Palette {
    fn drop(&mut self) {
        // O aplicativo fecha somente depois de publicar a última preferência.
        if let Some((sender, worker)) = self.writer.take() {
            drop(sender);
            let _ = worker.join();
        }
    }
}
type Answer = Result<Option<(PartId, Picked)>, String>;
struct Query {
    revision: u64,
    document: Document,
    view: ViewState,
    cut: Option<section::Section>,
    ray: nator_geometry::query::Ray,
    whole: bool,
}
pub(super) struct Draft {
    pub tab: u64,
    pub part: PartId,
    pub targets: Vec<Target>,
    color: [u8; 3],
    pub(super) rgb: [u8; 3],
    pub(super) hex: String,
    custom_selected: bool,
    queue: std::collections::VecDeque<Query>,
    running: Option<(u64, Receiver<Answer>)>,
}
fn parse_hex(value: &str) -> Option<[u8; 3]> {
    let digits = value.strip_prefix('#')?;
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&digits[..2], 16).ok()?,
        u8::from_str_radix(&digits[2..4], 16).ok()?,
        u8::from_str_radix(&digits[4..], 16).ok()?,
    ])
}
fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}
impl Draft {
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn selection_ready(&self) -> bool {
        self.running.is_none() && self.queue.is_empty()
    }
    pub fn receive(&mut self, picked: Picked) {
        let body = picked.target.body();
        if picked.whole_body {
            self.targets.retain(|t| t.body() != body);
            self.targets.push(Target::Body(body));
            return;
        }
        if let Some(index) = self.targets.iter().position(|t| *t == Target::Body(body)) {
            self.targets.remove(index);
            // Retirar uma face de um corpo selecionado inteiro conserva as demais.
            let face = match &picked.target {
                Target::Face { face, .. } => Some(face),
                _ => None,
            };
            self.targets.extend(
                picked
                    .faces
                    .into_iter()
                    .filter(|t| !matches!(t,Target::Face{face:f,..} if Some(f)==face)),
            );
            if let Target::Face {
                body,
                face,
                boundaries,
            } = picked.target
            {
                // Retirar uma célula do corpo inteiro conserva o complemento na
                // mesma face, além das outras faces (De Morgan para vários desenhos).
                for mut boundary in boundaries {
                    boundary.inside = !boundary.inside;
                    self.targets.push(Target::Face {
                        body,
                        face: face.clone(),
                        boundaries: vec![boundary],
                    });
                }
            }
        } else if let Some(index) = self.targets.iter().position(|t| *t == picked.target) {
            self.targets.remove(index);
        } else {
            self.targets.push(picked.target);
        }
    }
    pub(super) fn command(&self) -> Option<Command> {
        if self.targets.is_empty() {
            return None;
        }
        let color = if self.custom_selected {
            parse_hex(&self.hex)?
        } else {
            self.color
        };
        Some(Command::PaintSurfaces {
            targets: self.targets.clone(),
            color: RgbColor::new(color),
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
        self.current_mut().select(None);
        self.pick_mode = navigation::PickMode::Face;
        self.paint_edit = Some(Draft {
            tab: self.current().id,
            part,
            targets: vec![],
            color: self.paint_palette.custom,
            rgb: self.paint_palette.custom,
            hex: hex(self.paint_palette.custom),
            custom_selected: true,
            queue: Default::default(),
            running: None,
        });
    }
    /// Cliques não disputam o worker principal: o segundo clique do duplo clique
    /// entra na fila mesmo se a consulta do primeiro ainda estiver calculando.
    pub(super) fn paint_interaction(&mut self, ui: &egui::Ui, response: &egui::Response) -> bool {
        let Some(mut draft) = self.paint_edit.take() else {
            return false;
        };
        if draft.tab != self.current().id {
            self.paint_edit = Some(draft);
            return false;
        }
        if let Some((revision, receiver)) = &draft.running {
            let result = match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err(text(T::PointQueryFailed).into()))
                }
                Err(mpsc::TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                let current = *revision == self.current().session.revision();
                draft.running = None;
                if current {
                    match result {
                        Ok(Some((part, picked))) if part == draft.part => draft.receive(picked),
                        Ok(Some(_)) => self.status.warning(text(T::PickOtherPart)),
                        Err(error) => self.status.error(error),
                        _ => {}
                    }
                }
            }
        }
        let clicked = response.clicked()
            && response
                .hover_pos()
                .is_some_and(|p| !view_cube::rect(response.rect).contains(p));
        if clicked && let Some(pointer) = response.interact_pointer_pos() {
            let cursor = [
                f64::from((pointer.x - response.rect.center().x) / response.rect.width() * 2.),
                f64::from((response.rect.center().y - pointer.y) / response.rect.height() * 2.),
            ];
            draft.queue.push_back(Query {
                revision: self.current().session.revision(),
                document: self.picking_document(),
                view: self.current().view.clone(),
                cut: self.current().section,
                ray: self.viewport.camera.world_ray(
                    cursor,
                    self.display_bounds(),
                    f64::from(response.rect.aspect_ratio()),
                ),
                whole: response.double_clicked() || response.triple_clicked(),
            });
        }
        if draft.running.is_none()
            && let Some(query) = draft.queue.pop_front()
        {
            let revision = query.revision;
            let context = ui.ctx().clone();
            let (sender, receiver) = mpsc::channel();
            match std::thread::Builder::new()
                .name("paint-pick".into())
                .spawn(move || {
                    let result = (|| {
                        let picks = query
                            .document
                            .pick_all_with(query.ray, |o| query.view.pickable(o))
                            .map_err(|e| e.to_string())?;
                        section::first_visible(picks, query.cut)
                            .map(|pick| {
                                nator_commands::painting::pick(&query.document, &pick, query.whole)
                                    .map(|selection| (pick.part, selection))
                                    .map_err(|e| e.to_string())
                            })
                            .transpose()
                    })();
                    let _ = sender.send(result);
                    context.request_repaint();
                }) {
                Ok(_) => draft.running = Some((revision, receiver)),
                Err(error) => self.status.error(error.to_string()),
            }
        }
        if draft.running.is_some() || !draft.queue.is_empty() {
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
        self.paint_edit = Some(draft);
        clicked
    }
    pub(super) fn paint_panel(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.paint_palette.errors {
            while let Ok(error) = receiver.try_recv() {
                self.status.warning(error);
            }
        }
        let Some(mut draft) = self.paint_edit.take() else {
            return;
        };
        if self.current().id != draft.tab {
            return;
        }
        let Some(part) = self.current().session.document().part(draft.part).cloned() else {
            return;
        };
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
            command.is_some() && !self.busy() && draft.running.is_none() && draft.queue.is_empty(),
            |ui| {
                ui.label(text(T::PaintSurfaces));
                ui.label(
                    egui::RichText::new(text(T::PaintPickHint))
                        .small()
                        .color(theme::MUTED),
                );
                let mut remove = None;
                egui::ScrollArea::vertical()
                    .max_height(135.)
                    .show(ui, |ui| {
                        for (index, target) in draft.targets.iter().enumerate() {
                            let name = part
                                .bodies
                                .iter()
                                .find(|b| b.id == target.body())
                                .map(|b| b.name.as_str())
                                .unwrap_or("?");
                            let label = match target {
                                Target::Body(_) => format!("{name} · {}", text(T::PaintWholeBody)),
                                Target::Face { boundaries, .. } => format!(
                                    "{name} · {} {}{}",
                                    text(T::Surface),
                                    index + 1,
                                    if boundaries.is_empty() { "" } else { " ◇" }
                                ),
                            };
                            let mut checked = true;
                            if ui.checkbox(&mut checked, label).changed() {
                                remove = Some(index);
                            }
                        }
                        if draft.targets.is_empty() {
                            ui.label(text(T::Select));
                        }
                    });
                if let Some(index) = remove {
                    draft.targets.remove(index);
                }
                ui.separator();
                ui.label(text(T::PaintColor));
                for row in PRESETS {
                    ui.horizontal(|ui| {
                        for rgb in row {
                            let color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                            let selected = !draft.custom_selected && draft.color == rgb;
                            let button = egui::Button::new(if selected { "✓" } else { " " })
                                .fill(color)
                                .min_size(egui::vec2(26., 26.));
                            if ui.add(button).on_hover_text(hex(rgb)).clicked() {
                                draft.color = rgb;
                                draft.custom_selected = false;
                            }
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut draft.custom_selected, true, text(T::PaintCustom));
                    let picker = ui.color_edit_button_srgb(&mut draft.rgb);
                    if picker.changed() {
                        draft.hex = hex(draft.rgb);
                        draft.custom_selected = true;
                        self.paint_palette.remember(draft.rgb);
                    }
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut draft.hex)
                                .desired_width(94.)
                                .char_limit(7),
                        )
                        .changed()
                    {
                        draft.custom_selected = true;
                        if let Some(rgb) = parse_hex(&draft.hex) {
                            draft.rgb = rgb;
                            self.paint_palette.remember(rgb);
                        }
                    }
                });
                if parse_hex(&draft.hex).is_none() {
                    ui.colored_label(theme::WARNING, text(T::PaintInvalidHex));
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
    #[test]
    fn custom_color_survives_reopening_and_is_independent_of_presets()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("paint.json");
        {
            let mut palette = Palette::load(path.clone());
            palette.remember([18, 72, 172]);
            palette.remember([200, 80, 10]);
        }
        assert_eq!(Palette::load(path).custom, [200, 80, 10]);
        Ok(())
    }
}
