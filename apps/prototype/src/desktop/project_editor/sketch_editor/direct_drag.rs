//! Arraste de apoios/arestas na seleção livre. A prévia usa o solver no worker;
//! somente a última posição válida solicitada pode ser publicada ao soltar.
use super::*;
use operation_preview::{Cache, CanvasUpdate, Request};

#[derive(Default)]
pub(super) struct Interaction {
    drag: Option<Drag>,
    cache: Cache,
}
struct Drag {
    scene: SceneKey,
    origin: Point2,
    delta: Point2,
    points: Vec<u64>,
    circles: Vec<u64>,
    released: bool,
}
impl Interaction {
    pub fn active(&self) -> bool {
        self.drag.is_some()
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn canvas(
    ui: &egui::Ui,
    response: &egui::Response,
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    source: &Sketch,
    work: &mut Workspace,
    scene: SceneKey,
    enabled: bool,
) -> CanvasUpdate {
    let mut result = CanvasUpdate::default();
    if !work.is_selecting()
        || work
            .dragging
            .drag
            .as_ref()
            .is_some_and(|d| d.scene != scene)
    {
        work.dragging = Interaction::default();
        return result;
    }
    if work.dragging.active()
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        result.blocked = work.dragging.active();
        work.dragging = Interaction::default();
        return result;
    }
    if enabled
        && !ui.ctx().egui_wants_keyboard_input()
        && response.drag_started_by(egui::PointerButton::Primary)
        && let Some(origin) = ui
            .input(|i| i.pointer.press_origin())
            .and_then(|p| frame.cursor(p))
    {
        let tolerance = 8. / frame.pixels_per_mm(origin);
        let vertex = source
            .visible_points()
            .filter(|p| nator_sketch::distance(p.position, origin) <= tolerance)
            .min_by(|a, b| {
                nator_sketch::distance(a.position, origin)
                    .total_cmp(&nator_sketch::distance(b.position, origin))
            });
        let entity = nator_sketch::tools::pick_entity(source, origin, tolerance)
            .ok()
            .flatten();
        let points = if let Some(point) = vertex {
            work.selected_vertex = Some(point.position);
            vec![point.id]
        } else if let Some(id) = entity {
            if !work.selected.contains(&id) {
                work.selected = vec![id];
            }
            work.selected_vertex = None;
            nator_sketch::movement::points(source, &work.selected).unwrap_or_default()
        } else {
            vec![]
        };
        if !points.is_empty() {
            let circles = source
                .entities
                .iter()
                .filter_map(|e| match e.curve {
                    Curve::Circle { center, .. } if points.contains(&center) => Some(e.id),
                    _ => None,
                })
                .collect();
            work.constraint = None;
            work.dragging.drag = Some(Drag {
                scene,
                origin,
                delta: [0., 0.],
                points,
                circles,
                released: false,
            });
        }
    }
    let Some(drag) = work.dragging.drag.as_mut() else {
        return result;
    };
    result.blocked = true;
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    if !drag.released {
        if let Some(at) = ui
            .input(|i| i.pointer.interact_pos())
            .and_then(|p| frame.cursor(p))
        {
            drag.delta = nator_sketch::sub(at, drag.origin);
        }
        drag.released = response.drag_stopped_by(egui::PointerButton::Primary)
            || !ui.input(|i| i.pointer.primary_down());
    }
    let request = Request::dragged(drag.points.clone(), drag.circles.clone(), drag.delta);
    let released = drag.released;
    if let Err(error) = work
        .dragging
        .cache
        .prepare(source, &request, scene, ui.ctx())
    {
        result.warning = Some(error);
        work.dragging = Interaction::default();
        return result;
    }
    work.dragging
        .cache
        .paint(source, &request, scene, frame, painter);
    if released
        && let Some(candidate) = work.dragging.cache.result(source, &request, scene).cloned()
    {
        match candidate {
            Ok(candidate) if nator_sketch::distance(drag.delta, [0., 0.]) > 1e-7 => {
                if let Some(point) = work.selected_vertex.as_mut() {
                    *point = nator_sketch::add(*point, drag.delta);
                }
                result.commit = Some((candidate, SolveMode::Normal));
            }
            Err(error) => result.warning = Some(error),
            _ => {}
        }
        work.dragging = Interaction::default();
    }
    result
}
