//! Candidato do Medir: a mesma classificação exata usada pelo clique, no trabalhador.
use super::*;
use nator_document::assembly::Pick;
use nator_geometry::query::Ray;

#[derive(Clone, PartialEq)]
struct Key {
    scene: SceneKey,
    origin: Point3,
    direction: Point3,
    reach: f64,
    section: Option<section::Section>,
    view_rays: [(Point3, Point3); 2],
    rect: egui::Rect,
}
impl Key {
    fn same_context(&self, other: &Self) -> bool {
        self.scene == other.scene
            && self.section == other.section
            && self.view_rays == other.view_rays
            && self.rect == other.rect
            && (self.reach - other.reach).abs() <= self.reach.max(1.) * 1e-9
    }
}
#[derive(Default)]
pub(super) struct Cache {
    ready: Option<(Key, highlight::Overlay)>,
    running: Option<(Key, Receiver<highlight::Overlay>)>,
}
fn overlay(document: &Document, pick: &Pick, target: Target) -> highlight::Overlay {
    match target {
        Target::Vertex { point } => highlight::Overlay {
            points: vec![point],
            ..Default::default()
        },
        Target::Edge { points, .. } => highlight::Overlay {
            segments: points.windows(2).map(|s| ([s[0], s[1]], None)).collect(),
            ..Default::default()
        },
        Target::Face { .. } => {
            if let Some(body) = document.body(pick.body)
                && let Some(face) = body
                    .geometry
                    .triangle_faces()
                    .and_then(|f| f.get(pick.triangle))
                && let Ok(pose) = pick.placement.compose(body.placement)
            {
                return highlight::face(body, pose, *face);
            }
            highlight::Overlay {
                triangles: vec![pick.vertices],
                ..Default::default()
            }
        }
    }
}
impl Cache {
    fn query(
        &mut self,
        key: Key,
        document: Document,
        view: ViewState,
        context: &egui::Context,
    ) -> Option<highlight::Overlay> {
        if self
            .ready
            .as_ref()
            .is_some_and(|(ready, _)| !ready.same_context(&key))
        {
            self.ready = None;
        }
        if let Some((requested, receiver)) = &self.running {
            match receiver.try_recv() {
                Ok(overlay) => {
                    if requested.same_context(&key)
                        && (*requested == key
                            || self.ready.as_ref().is_none_or(|(ready, _)| *ready != key))
                    {
                        self.ready = Some((requested.clone(), overlay));
                    }
                    self.running = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => self.running = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some((ready, overlay)) = &self.ready
            && *ready == key
        {
            return Some(overlay.clone());
        }
        if self.running.is_none() {
            let query = key.clone();
            let context = context.clone();
            let (sender, receiver) = mpsc::channel();
            if std::thread::Builder::new()
                .name("measure-hover".into())
                .spawn(move || {
                    let ray = Ray {
                        origin: query.origin,
                        direction: query.direction,
                    };
                    let result = document
                        .pick_all_clipped_with(
                            ray,
                            |o| view.pickable(o),
                            section::clip(query.section),
                        )
                        .ok()
                        .and_then(|picks| section::first_visible(picks, query.section))
                        .and_then(|pick| {
                            nator_commands::measure::target_at(&document, &pick, query.reach)
                                .ok()
                                .map(|target| overlay(&document, &pick, target))
                        })
                        .unwrap_or_default();
                    let _ = sender.send(result);
                    context.request_repaint();
                })
                .is_ok()
            {
                self.running = Some((key, receiver));
            }
        }
        context.request_repaint_after(Duration::from_millis(16));
        self.ready.as_ref().map(|(_, overlay)| overlay.clone())
    }
}
impl Editor {
    pub(in crate::desktop::project_editor) fn update_measure_hover(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
    ) {
        self.hover = None;
        let Some(position) = response.hover_pos().filter(|p| {
            !view_cube::rect(response.rect).contains(*p) && !ui.input(|i| i.pointer.any_down())
        }) else {
            return;
        };
        let Some(bounds) = self.current().scene_bounds else {
            return;
        };
        let rect = response.rect;
        let ray = self.viewport.camera.world_ray(
            [
                f64::from((position.x - rect.center().x) / rect.width() * 2.),
                f64::from((rect.center().y - position.y) / rect.height() * 2.),
            ],
            bounds,
            f64::from(rect.aspect_ratio()),
        );
        let key = Key {
            scene: self.scene_key(),
            origin: ray.origin,
            direction: ray.direction,
            reach: self.pick_reach(position, rect, bounds),
            section: self.current().section,
            view_rays: [[0., 0.], [1., 1.]].map(|p| {
                let ray = self
                    .viewport
                    .camera
                    .world_ray(p, bounds, f64::from(rect.aspect_ratio()));
                (ray.origin, ray.direction)
            }),
            rect,
        };
        let document = self.picking_document();
        let view = self.current().view.clone();
        if let Some(overlay) = self
            .measure_tool
            .hover
            .query(key.clone(), document, view, ui.ctx())
        {
            self.hover = Some(highlight::Hover::fixed(
                key.scene.tab,
                key.scene.revision,
                position,
                overlay,
                String::new(),
            ));
        }
    }
}
