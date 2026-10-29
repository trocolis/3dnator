//! Referências raster no plano do rascunho. Pixels são decodificados no worker;
//! a interface manipula somente um retângulo em milímetros e uma textura cacheada.
use super::*;
use nator_document::{ImageId, RasterAsset, ReferenceImage};
use nator_sketch::{Point2, add, sub};

pub(super) struct Draft {
    tab: u64,
    pub image: ReferenceImage,
    editing: bool,
    placing: bool,
    anchor: Option<Point2>,
    gesture: Option<Gesture>,
    model_placement: Option<Placement>,
}
struct Gesture {
    pointer: Point2,
    origin: Point2,
    size: Point2,
    corner: Option<usize>,
    changed: bool,
}
#[derive(Default)]
pub(super) struct Cache {
    textures: std::collections::HashMap<usize, (Arc<RasterAsset>, egui::TextureHandle)>,
}
impl Cache {
    fn paint(&mut self, ui: &egui::Ui, frame: &sketch_scene::Frame, image: &ReferenceImage) {
        let key = Arc::as_ptr(&image.asset) as usize;
        let (_, texture) = self.textures.entry(key).or_insert_with(|| {
            (
                image.asset.clone(),
                ui.ctx().load_texture(
                    format!("reference-image-{key}"),
                    egui::ColorImage::from_rgba_unmultiplied(
                        [image.asset.width as usize, image.asset.height as usize],
                        &image.asset.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                ),
            )
        });
        let corners = corners(image);
        let Some(points) = corners
            .iter()
            .map(|p| frame.screen(*p))
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        let mut mesh = egui::Mesh::with_texture(texture.id());
        let color =
            egui::Color32::from_white_alpha(((1. - image.transparency) * 255.).round() as u8);
        for (pos, uv) in points
            .into_iter()
            .zip([[0., 1.], [1., 1.], [1., 0.], [0., 0.]])
        {
            mesh.vertices.push(egui::epaint::Vertex {
                pos,
                uv: egui::pos2(uv[0], uv[1]),
                color,
            });
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        ui.painter()
            .with_clip_rect(frame.rect)
            .add(egui::Shape::mesh(mesh));
    }
}
pub(super) fn corners(image: &ReferenceImage) -> [Point2; 4] {
    let [x, y] = image.origin;
    let [w, h] = image.size;
    [[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
}
fn contains(image: &ReferenceImage, p: Point2) -> bool {
    p[0] >= image.origin[0]
        && p[1] >= image.origin[1]
        && p[0] <= image.origin[0] + image.size[0]
        && p[1] <= image.origin[1] + image.size[1]
}
fn read_asset(path: &std::path::Path) -> Result<Arc<RasterAsset>, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 32 * 1024 * 1024 {
        return Err(text(T::ImageFileTooLarge).into());
    }
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(nator_document::images::MAX_IMAGE_BYTES as u64);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|e| e.to_string())?.into_rgba8();
    let asset = Arc::new(RasterAsset {
        width: decoded.width(),
        height: decoded.height(),
        rgba: Arc::new(decoded.into_raw()),
    });
    asset.validate().map_err(|e| e.to_string())?;
    Ok(asset)
}

fn corner_at(
    frame: &sketch_scene::Frame,
    image: &ReferenceImage,
    pointer: egui::Pos2,
) -> Option<usize> {
    corners(image)
        .iter()
        .enumerate()
        .find(|(_, p)| {
            frame
                .screen(**p)
                .is_some_and(|at| at.distance(pointer) <= 10.)
        })
        .map(|(i, _)| i)
}
fn hit(frame: &sketch_scene::Frame, image: &ReferenceImage, pointer: egui::Pos2) -> bool {
    corner_at(frame, image, pointer).is_some()
        || frame.cursor(pointer).is_some_and(|p| contains(image, p))
}
fn handles(ui: &egui::Ui, frame: &sketch_scene::Frame, image: &ReferenceImage) {
    let painter = ui.painter().with_clip_rect(frame.rect);
    let points = corners(image);
    frame.path(
        &painter,
        points.into_iter().chain([points[0]]),
        egui::Stroke::new(1.5, theme::GREEN),
        false,
    );
    for point in points {
        if let Some(at) = frame.screen(point) {
            painter.rect_filled(
                egui::Rect::from_center_size(at, egui::Vec2::splat(8.)),
                1.,
                theme::GREEN,
            );
        }
    }
}
impl Draft {
    fn interact(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        frame: &sketch_scene::Frame,
        enabled: bool,
    ) -> bool {
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let cursor = if self.gesture.is_some() {
            pointer
        } else {
            response.hover_pos()
        }
        .and_then(|p| frame.cursor(p));
        let mut commit = false;
        if enabled {
            if self.placing {
                if let (Some(anchor), Some(cursor)) = (self.anchor, cursor) {
                    self.image.origin = [anchor[0].min(cursor[0]), anchor[1].min(cursor[1])];
                    self.image.size = [
                        (anchor[0] - cursor[0]).abs().max(1e-6),
                        (anchor[1] - cursor[1]).abs().max(1e-6),
                    ];
                }
                if response.clicked_by(egui::PointerButton::Primary)
                    && let Some(cursor) = cursor
                {
                    if self.anchor.is_none() {
                        self.anchor = Some(cursor);
                    } else if self.image.size.iter().all(|v| *v > 1e-5) {
                        self.placing = false;
                    }
                }
            } else if response.contains_pointer()
                && ui.input(|i| i.pointer.primary_pressed())
                && let Some(cursor) = cursor
            {
                let corner = corners(&self.image)
                    .iter()
                    .enumerate()
                    .find(|(_, p)| {
                        frame
                            .screen(**p)
                            .zip(pointer)
                            .is_some_and(|(a, b)| a.distance(b) < 10.)
                    })
                    .map(|(i, _)| i);
                if corner.is_some() || contains(&self.image, cursor) {
                    self.gesture = Some(Gesture {
                        pointer: cursor,
                        origin: self.image.origin,
                        size: self.image.size,
                        corner,
                        changed: false,
                    });
                }
            }
            if let Some(gesture) = &mut self.gesture {
                if let Some(cursor) = cursor {
                    if let Some(index) = gesture.corner {
                        let fixed = match index {
                            0 => add(gesture.origin, gesture.size),
                            1 => add(gesture.origin, [0., gesture.size[1]]),
                            2 => gesture.origin,
                            _ => add(gesture.origin, [gesture.size[0], 0.]),
                        };
                        self.image.origin = [fixed[0].min(cursor[0]), fixed[1].min(cursor[1])];
                        self.image.size = [
                            (fixed[0] - cursor[0]).abs().max(1e-6),
                            (fixed[1] - cursor[1]).abs().max(1e-6),
                        ];
                    } else {
                        self.image.origin = add(gesture.origin, sub(cursor, gesture.pointer));
                    }
                    gesture.changed |= nator_sketch::distance(cursor, gesture.pointer) > 1e-6;
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                }
                if !ui.input(|i| i.pointer.primary_down()) {
                    commit = gesture.changed && self.editing;
                    self.gesture = None;
                }
            }
        }
        commit
    }
}

impl Editor {
    pub(super) fn refresh_reference_image(&mut self) {
        if let Some(draft) = self.image_edit.as_mut() {
            if let Some(image) = self.tabs[self.active]
                .session
                .document()
                .image(draft.image.id)
                .cloned()
            {
                draft.image = image;
                draft.gesture = None;
            } else {
                self.image_edit = None;
            }
        }
    }
    fn model_images(&self, rect: egui::Rect) -> Vec<(sketch_scene::Frame, ReferenceImage)> {
        let document = self.current().session.document();
        let mut entries = Vec::new();
        if let Ok(occurrences) = document.occurrences() {
            for occurrence in occurrences
                .iter()
                .filter(|o| self.current().view.in_isolation(o))
            {
                if let Some(part) = document
                    .instance(occurrence.instance)
                    .and_then(|i| document.part(i.part))
                {
                    for image in document
                        .images()
                        .iter()
                        .filter(|i| i.visible && document.sketch_part(i.sketch) == Some(part.id))
                    {
                        if let Some(record) = document
                            .sketch(image.sketch)
                            .filter(|s| part.shows_sketch(s))
                        {
                            entries.push((
                                sketch_scene::Frame {
                                    camera: self.viewport.camera.clone(),
                                    bounds: self.display_bounds(),
                                    rect,
                                    placement: occurrence.placement,
                                    plane: record.sketch.plane,
                                },
                                image.clone(),
                            ));
                        }
                    }
                }
            }
        }
        for image in document
            .images()
            .iter()
            .filter(|i| i.visible && document.is_reference_sketch(i.sketch))
        {
            if let Some(record) = document
                .sketch(image.sketch)
                .filter(|s| document.shows_reference_sketch(s))
            {
                entries.push((
                    sketch_scene::Frame {
                        camera: self.viewport.camera.clone(),
                        bounds: self.display_bounds(),
                        rect,
                        placement: Placement::default(),
                        plane: record.sketch.plane,
                    },
                    image.clone(),
                ));
            }
        }
        entries
    }
    pub(super) fn reference_image_model_gesture(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
    ) -> bool {
        if self.current().sketch_edit.is_some()
            || self.busy()
            || self.current().camera_transition.is_some()
        {
            return false;
        }
        let entries = self.model_images(response.rect);
        if self.image_edit.is_none()
            && self.selection_active()
            && response.contains_pointer()
            && ui.input(|i| i.pointer.primary_pressed())
            && let Some(pointer) = ui.input(|i| i.pointer.interact_pos())
            && let Some((frame, image)) = entries
                .iter()
                .rev()
                .find(|(frame, image)| hit(frame, image, pointer))
        {
            self.image_edit = Some(Draft {
                tab: self.current().id,
                image: image.clone(),
                editing: true,
                placing: false,
                anchor: None,
                gesture: None,
                model_placement: Some(frame.placement),
            });
        }
        let Some(mut draft) = self.image_edit.take() else {
            return false;
        };
        if let Some((frame, _)) = entries.iter().find(|(frame, i)| {
            i.id == draft.image.id && draft.model_placement.is_none_or(|p| p == frame.placement)
        }) {
            if draft.interact(ui, response, frame, true) {
                self.execute(Command::UpdateReferenceImage {
                    image: draft.image.clone(),
                });
            }
            self.image_edit = Some(draft);
            true
        } else {
            false
        }
    }
    pub(super) fn reference_images_in_model(&mut self, ui: &egui::Ui, rect: egui::Rect) {
        if self.current().sketch_edit.is_some() {
            return;
        }
        let entries = self.model_images(rect);
        let active = entries
            .iter()
            .map(|(_, i)| Arc::as_ptr(&i.asset) as usize)
            .collect::<HashSet<_>>();
        self.image_cache
            .textures
            .retain(|key, _| active.contains(key));
        let pointer = ui.input(|i| i.pointer.hover_pos());
        for (frame, saved) in entries {
            let draft = self.image_edit.as_ref().filter(|d| d.image.id == saved.id);
            let image = draft.map(|d| d.image.clone()).unwrap_or(saved);
            self.image_cache.paint(ui, &frame, &image);
            if draft.is_some() || pointer.is_some_and(|p| hit(&frame, &image, p)) {
                handles(ui, &frame, &image);
            }
        }
    }
    pub(super) fn load_reference_image(&mut self, context: &egui::Context) {
        if self.busy() {
            return;
        }
        let Some(sketch) = self.current().sketch_edit.as_ref().map(|w| w.id) else {
            return;
        };
        self.activate_selection(context);
        self.image_edit = None;
        let tab = self.current().id;
        let revision = self.current().session.revision();
        self.status.set(text(T::ImageLoad));
        self.start(context, None, move || {
            let Some(path) = rfd::FileDialog::new()
                .add_filter(text(T::SketchImage), &["png", "jpg", "jpeg"])
                .pick_file()
            else {
                return Ok(Outcome::Cancelled);
            };
            let asset = read_asset(&path)?;
            Ok(Outcome::ImageLoaded {
                tab,
                revision,
                sketch,
                asset,
            })
        });
    }
    pub(super) fn receive_reference_image(
        &mut self,
        tab: u64,
        revision: u64,
        sketch: SketchId,
        asset: Arc<RasterAsset>,
    ) {
        if self.current().id != tab
            || self.current().session.revision() != revision
            || self
                .current()
                .sketch_edit
                .as_ref()
                .is_none_or(|w| w.id != sketch)
        {
            return;
        }
        self.image_edit = Some(Draft {
            tab,
            image: ReferenceImage {
                id: ImageId(0),
                sketch,
                name: text(T::SketchImage).into(),
                size: [50., 50. * f64::from(asset.height) / f64::from(asset.width)],
                asset,
                origin: [0., 0.],
                transparency: 0.5,
                visible: true,
            },
            editing: false,
            placing: true,
            anchor: None,
            gesture: None,
            model_placement: None,
        });
    }
    pub(super) fn edit_reference_image(&mut self, id: ImageId) {
        let Some(image) = self.current().session.document().image(id).cloned() else {
            return;
        };
        // Selecionar uma imagem do rascunho já aberto não reinicia câmera nem
        // captura original de cancelamento; apenas troca a ferramenta contextual.
        if self
            .current()
            .sketch_edit
            .as_ref()
            .is_none_or(|w| w.id != image.sketch)
        {
            if let Some(part) = self.current().session.document().sketch_part(image.sketch) {
                self.begin_sketch(part, image.sketch);
            } else {
                self.begin_reference_sketch(image.sketch);
            }
        }
        if let Some(work) = self.current_mut().sketch_edit.as_mut() {
            work.select_image();
        }
        self.image_edit = Some(Draft {
            tab: self.current().id,
            image,
            editing: true,
            placing: false,
            anchor: None,
            gesture: None,
            model_placement: None,
        });
    }
    pub(super) fn reference_image_panel(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.image_edit.take() else {
            return;
        };
        if draft.tab != self.current().id
            || self
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.id != draft.image.sketch)
        {
            return;
        }
        let mut transparency = draft.image.transparency * 100.;
        let outcome = command_panel::show_validated(
            context,
            command_panel::Header {
                id: "reference_image",
                icon: icons::Icon::Image,
                title: text(T::SketchImage),
            },
            self.canvas_rect,
            command_panel::Footer::None,
            !self.busy(),
            |ui| {
                ui.text_edit_singleline(&mut draft.image.name);
                if draft.placing {
                    ui.label(text(T::ImageDrawHint));
                }
                for (value, label) in draft
                    .image
                    .size
                    .iter_mut()
                    .zip([text(T::SketchDynamicWidth), text(T::SketchDynamicHeight)])
                {
                    command_panel::row(ui, label, |ui| {
                        ui.add(
                            egui::DragValue::new(value)
                                .speed(0.1)
                                .suffix(" mm")
                                .range(1e-6..=1e9),
                        );
                    });
                }
                ui.label(text(T::ImageLocalPosition));
                for (value, label) in draft.image.origin.iter_mut().zip(["X (mm)", "Y (mm)"]) {
                    command_panel::row(ui, label, |ui| {
                        ui.add(egui::DragValue::new(value).speed(0.1).range(-1e9..=1e9));
                    });
                }
                if let Some(sketch) = self.current().session.document().sketch(draft.image.sketch) {
                    let p = sketch.sketch.plane.world(draft.image.origin);
                    ui.label(format!(
                        "{}: {}, {}, {}",
                        text(T::ImagePosition),
                        nator_drawing::format_mm(p[0]),
                        nator_drawing::format_mm(p[1]),
                        nator_drawing::format_mm(p[2])
                    ));
                }
                ui.add(
                    egui::Slider::new(&mut transparency, 0. ..=100.)
                        .text(text(T::ImageTransparency)),
                );
                draft.image.transparency = transparency / 100.;
                !draft.placing && draft.image.validate().is_ok()
            },
        );
        if outcome.ok {
            let command = if draft.editing {
                Command::UpdateReferenceImage {
                    image: draft.image.clone(),
                }
            } else {
                Command::AddReferenceImage {
                    image: draft.image.clone(),
                }
            };
            let revision = self.current().session.revision();
            self.execute(command);
            if self.current().session.revision() == revision {
                self.image_edit = Some(draft);
            }
        } else if !outcome.cancel {
            self.image_edit = Some(draft);
        }
    }

    /// Desenhar a textura não interfere no perfil. Só a seleção livre pode
    /// escolher a imagem; uma curva/apoio real sob o cursor tem precedência.
    pub(super) fn reference_images_overlay(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        frame: &sketch_scene::Frame,
        source: &nator_sketch::Sketch,
        enabled: bool,
    ) -> bool {
        let Some(sketch) = self.current().sketch_edit.as_ref().map(|w| w.id) else {
            return false;
        };
        let selectable = self
            .current()
            .sketch_edit
            .as_ref()
            .is_some_and(|w| w.is_selecting());
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let dragging = self
            .image_edit
            .as_ref()
            .is_some_and(|d| d.gesture.is_some());
        let cursor = if dragging {
            pointer
        } else {
            response.hover_pos()
        }
        .and_then(|p| frame.cursor(p));
        let saved = self
            .current()
            .session
            .document()
            .images()
            .iter()
            .filter(|i| i.sketch == sketch && i.visible)
            .cloned()
            .collect::<Vec<_>>();
        if self.image_edit.is_none()
            && enabled
            && selectable
            && response.contains_pointer()
            && ui.input(|i| i.pointer.primary_pressed())
            && let Some(cursor) = cursor
            && let Some(pointer) = pointer
            && let Some(image) = saved.iter().rev().find(|i| hit(frame, i, pointer)).cloned()
            && (corner_at(frame, &image, pointer).is_some()
                || (nator_sketch::tools::pick_entity(
                    source,
                    cursor,
                    8. / frame.pixels_per_mm(cursor),
                )
                .ok()
                .flatten()
                .is_none()
                    && !source.visible_points().any(|p| {
                        nator_sketch::distance(p.position, cursor)
                            <= 8. / frame.pixels_per_mm(cursor)
                    })))
        {
            self.image_edit = Some(Draft {
                tab: self.current().id,
                image,
                editing: true,
                placing: false,
                anchor: None,
                gesture: None,
                model_placement: None,
            });
        }
        let mut draft = self
            .image_edit
            .take()
            .filter(|d| d.tab == self.current().id && d.image.sketch == sketch);
        let active_assets = saved
            .iter()
            .map(|i| Arc::as_ptr(&i.asset) as usize)
            .chain(draft.as_ref().map(|d| Arc::as_ptr(&d.image.asset) as usize))
            .collect::<HashSet<_>>();
        self.image_cache
            .textures
            .retain(|key, _| active_assets.contains(key));
        for image in &saved {
            if draft
                .as_ref()
                .is_none_or(|d| !d.editing || d.image.id != image.id)
            {
                self.image_cache.paint(ui, frame, image);
                if selectable && pointer.is_some_and(|p| hit(frame, image, p)) {
                    handles(ui, frame, image);
                }
            }
        }
        let Some(mut draft) = draft.take() else {
            return false;
        };
        let commit = draft.interact(ui, response, frame, enabled);
        self.image_cache.paint(ui, frame, &draft.image);
        handles(ui, frame, &draft.image);
        if commit {
            self.execute(Command::UpdateReferenceImage {
                image: draft.image.clone(),
            });
        }
        self.image_edit = Some(draft);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png_and_jpeg_decode_into_valid_shared_pixels() -> Result<(), Box<dyn std::error::Error>> {
        for format in [image::ImageFormat::Png, image::ImageFormat::Jpeg] {
            let file = tempfile::NamedTempFile::new()?;
            image::RgbImage::from_pixel(2, 3, image::Rgb([180, 70, 20]))
                .save_with_format(file.path(), format)?;
            let asset = read_asset(file.path())?;
            assert_eq!((asset.width, asset.height), (2, 3));
            assert_eq!(asset.rgba.len(), 24);
            assert!(asset.rgba.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
        }
        Ok(())
    }
}
