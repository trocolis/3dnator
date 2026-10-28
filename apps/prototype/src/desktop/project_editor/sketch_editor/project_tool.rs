//! Seleção das referências de Projetar. A consulta geométrica roda no trabalhador;
//! a interface apenas captura entre os elementos já projetados e desenha a prévia.
use super::*;
use nator_commands::sketch_projection::{self, Element, Kind};

#[derive(Default)]
pub(super) struct Cache {
    key: Option<projection::Key>,
    elements: Vec<Element>,
    running: Option<Receiver<Result<Vec<Element>, String>>>,
    filter: Option<Kind>,
}

impl Cache {
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn ready(&self) -> bool {
        self.key.is_some() && self.running.is_none()
    }
    pub(super) fn settings(&mut self, ui: &mut egui::Ui) {
        for (filter, label) in [
            (None, T::SketchProjectAll),
            (Some(Kind::Plane), T::SketchProjectPlanes),
            (Some(Kind::Edge), T::SketchProjectEdges),
            (Some(Kind::Vertex), T::SketchProjectVertices),
        ] {
            ui.radio_value(&mut self.filter, filter, text(label));
        }
        ui.separator();
        ui.label(text(T::SketchProjectSnapshot));
    }

    pub(super) fn prepare(
        &mut self,
        key: projection::Key,
        document: &Document,
        view: &ViewState,
        context: &egui::Context,
    ) -> Result<(), String> {
        if self.key != Some(key) {
            self.elements.clear();
            self.running = None;
            self.key = Some(key);
            let occurrences = document
                .occurrences()
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|o| view.pickable(o))
                .collect::<Vec<_>>();
            let document = document.clone();
            let context = context.clone();
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("sketch-project-elements".into())
                .spawn(move || {
                    let result = sketch_projection::elements(
                        &document,
                        &occurrences,
                        key.plane,
                        key.placement,
                    )
                    .map_err(|e| e.to_string());
                    let _ = sender.send(result);
                    context.request_repaint();
                })
                .map_err(|e| e.to_string())?;
            self.running = Some(receiver);
        }
        if let Some(receiver) = &self.running {
            match receiver.try_recv() {
                Ok(result) => {
                    self.running = None;
                    self.elements = result?;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.running = None;
                    return Err(text(T::PointQueryFailed).into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        Ok(())
    }

    pub(super) fn update(
        &self,
        frame: &sketch_scene::Frame,
        painter: &egui::Painter,
        cursor: Point2,
        clicked: bool,
        sketch: &mut Sketch,
    ) -> Result<bool, String> {
        let Some(index) = sketch_projection::pick(
            &self.elements,
            cursor,
            8. / frame.pixels_per_mm(cursor),
            self.filter,
        ) else {
            return Ok(false);
        };
        let element = &self.elements[index];
        for triangle in &element.triangles {
            let points = triangle
                .iter()
                .filter_map(|p| frame.screen(*p))
                .collect::<Vec<_>>();
            if points.len() == 3 {
                painter.add(egui::Shape::convex_polygon(
                    points,
                    theme::PROJECTED_FILL,
                    egui::Stroke::NONE,
                ));
            }
        }
        for curve in &element.curves {
            frame.path(
                painter,
                curve.sample(128),
                egui::Stroke::new(2.5, theme::PROJECTED),
                false,
            );
        }
        for point in &element.vertices {
            if let Some(screen) = frame.screen(*point) {
                painter.circle_filled(screen, 5., theme::PROJECTED);
            }
        }
        if clicked {
            nator_sketch::projection::insert(sketch, &element.curves, &element.vertices)
                .map_err(sketch_error)
        } else {
            Ok(false)
        }
    }
}

pub(super) fn paint_regions(
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    regions: &[profiles::Region],
) {
    for region in regions {
        let polygons = std::iter::once(&region.outer)
            .chain(&region.holes)
            .map(|wire| {
                wire.sample(128)
                    .into_iter()
                    .filter_map(|p| frame.screen(p))
                    .collect()
            })
            .collect::<Vec<Vec<_>>>();
        profile_pick::fill(painter, &polygons, theme::PROJECTED_FILL);
    }
}
