//! Prévia assíncrona de operações do rascunho. Uma consulta por vez, invalidada
//! por seleção, valores e revisão; respostas antigas nunca são aplicadas.
use super::*;
use nator_sketch::{
    edit,
    modifications::{self, OffsetMode},
    movement,
};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Request {
    pub action: EditAction,
    selected: Vec<u64>,
    axis: Option<u64>,
    stroke: LineType,
    mode: OffsetMode,
    value: f64,
    second: f64,
    delta: Point2,
    points: Option<Vec<u64>>,
}
impl Request {
    pub fn from(
        work: &Workspace,
        action: EditAction,
        parameters: &nator_document::parameters::Parameters,
    ) -> Result<Self, String> {
        let number = |s: &str| {
            parameters
                .evaluate(s, nator_document::parameters::Dimension::Length)
                .map_err(sketch_error)
        };
        Ok(Self {
            action,
            points: None,
            selected: work.selected.clone(),
            axis: work.mirror_axis,
            stroke: work.operation_line_type,
            mode: work.offset_mode,
            value: if matches!(
                action,
                EditAction::Offset | EditAction::Fillet | EditAction::Chamfer
            ) {
                number(&work.dimension_expression)?
            } else {
                0.
            },
            second: if action == EditAction::Offset && work.offset_mode == OffsetMode::Double {
                number(&work.offset_second_expression)?
            } else {
                0.
            },
            delta: [0., 0.],
        })
    }
    pub fn dragged(points: Vec<u64>, circles: Vec<u64>, delta: Point2) -> Self {
        Self {
            action: EditAction::Move,
            selected: circles,
            axis: None,
            stroke: LineType::Normal,
            mode: OffsetMode::Normal,
            value: 0.,
            second: 0.,
            delta,
            points: Some(points),
        }
    }
    fn build(&self, source: &Sketch) -> Result<Sketch, String> {
        let mut next = source.clone();
        match self.action {
            EditAction::Mirror => {
                modifications::mirror(
                    &mut next,
                    &self.selected,
                    self.axis
                        .ok_or_else(|| text(T::SketchMirrorAxisHint).to_owned())?,
                    self.stroke,
                )
                .map_err(sketch_error)?;
            }
            EditAction::Offset => {
                modifications::offset(
                    &mut next,
                    &self.selected,
                    self.mode,
                    self.value,
                    self.second,
                    self.stroke,
                )
                .map_err(sketch_error)?;
            }
            EditAction::Move => {
                if nator_sketch::distance(self.delta, [0., 0.]) < 1e-9 {
                    return Err(text(T::SketchDragHint).into());
                }
                let points = self
                    .points
                    .clone()
                    .map(Ok)
                    .unwrap_or_else(|| movement::points(source, &self.selected))
                    .map_err(sketch_error)?;
                next = movement::dragged(source, &points, &self.selected, self.delta)
                    .map_err(sketch_error)?;
            }
            EditAction::Fillet | EditAction::Chamfer => {
                edit::apply(
                    &mut next,
                    self.action,
                    &self.selected,
                    edit::Options {
                        value_mm: self.value,
                        count: 2,
                        reference: false,
                    },
                )
                .map_err(sketch_error)?;
            }
            _ => return Err(text(T::SketchSelectOne).into()),
        }
        Ok(next)
    }
}
pub(super) fn supported(action: EditAction) -> bool {
    matches!(
        action,
        EditAction::Mirror | EditAction::Offset | EditAction::Fillet | EditAction::Chamfer
    )
}

#[derive(Clone, PartialEq)]
struct Key {
    scene: SceneKey,
    geometry: Vec<u64>,
    request: Request,
}
fn key(source: &Sketch, request: &Request, scene: SceneKey) -> Key {
    Key {
        scene,
        request: request.clone(),
        geometry: std::iter::once(source.next_id)
            .chain(
                source
                    .points
                    .iter()
                    .flat_map(|p| [p.id, p.position[0].to_bits(), p.position[1].to_bits()]),
            )
            .collect(),
    }
}
#[derive(Default)]
pub(super) struct Cache {
    ready: Option<(Key, Result<Sketch, String>)>,
    running: Option<(Key, Receiver<Result<Sketch, String>>)>,
}
impl Cache {
    pub fn ready(&self) -> bool {
        self.ready.as_ref().is_some_and(|(_, r)| r.is_ok()) && self.running.is_none()
    }
    pub fn prepare(
        &mut self,
        source: &Sketch,
        request: &Request,
        scene: SceneKey,
        context: &egui::Context,
    ) -> Result<(), String> {
        let wanted = key(source, request, scene);
        if let Some((key, receiver)) = &self.running {
            let result = match receiver.try_recv() {
                Ok(value) => Some(value),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err(text(T::PointQueryFailed).into()))
                }
                Err(mpsc::TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                self.ready = Some((key.clone(), result));
                self.running = None;
            }
        }
        if self.ready.as_ref().is_some_and(|(key, _)| *key == wanted) {
            return Ok(());
        }
        self.ready = None;
        if self.running.is_none() {
            let source = source.clone();
            let request = request.clone();
            let context = context.clone();
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("sketch-operation-preview".into())
                .spawn(move || {
                    let _ = sender.send(request.build(&source));
                    context.request_repaint();
                })
                .map_err(sketch_error)?;
            self.running = Some((wanted, receiver));
        }
        Ok(())
    }
    pub fn result(
        &self,
        source: &Sketch,
        request: &Request,
        scene: SceneKey,
    ) -> Option<&Result<Sketch, String>> {
        let wanted = key(source, request, scene);
        self.ready
            .as_ref()
            .filter(|(key, _)| *key == wanted)
            .map(|(_, r)| r)
    }
    pub fn paint(
        &self,
        source: &Sketch,
        request: &Request,
        scene: SceneKey,
        frame: &sketch_scene::Frame,
        painter: &egui::Painter,
    ) {
        let Some(Ok(next)) = self.result(source, request, scene) else {
            return;
        };
        for entity in &next.entities {
            let Ok(curve) = profiles::segment(next, entity) else {
                continue;
            };
            let unchanged = source
                .entity(entity.id)
                .ok()
                .and_then(|e| profiles::segment(source, e).ok())
                .is_some_and(|old| old == curve);
            if unchanged {
                continue;
            }
            let stroke = egui::Stroke::new(
                2.5,
                egui::Color32::from_rgba_unmultiplied(75, 105, 210, 145),
            );
            if entity.infinite {
                if let profiles::Segment::Line { a, b } = curve {
                    frame.infinite_line(painter, a, b, stroke);
                }
            } else {
                frame.path(painter, curve.sample(96), stroke, entity.construction);
            }
        }
    }
}

pub(super) struct Interaction {
    pub cache: Cache,
    pub selecting: bool,
    pub whole_chain: bool,
    pub seed: Option<u64>,
    pub handle_drag: Option<operation_handles::Drag>,
    pub focus: bool,
    pub second_side: bool,
    pub confirm: bool,
    #[cfg(all(test, feature = "occt"))]
    pub handle_tips: Vec<egui::Pos2>,
}
impl Default for Interaction {
    fn default() -> Self {
        Self {
            cache: Cache::default(),
            selecting: false,
            whole_chain: true,
            seed: None,
            handle_drag: None,
            focus: false,
            second_side: false,
            confirm: false,
            #[cfg(all(test, feature = "occt"))]
            handle_tips: vec![],
        }
    }
}

#[derive(Default)]
pub(super) struct CanvasUpdate {
    pub blocked: bool,
    pub commit: Option<(Sketch, SolveMode)>,
    pub warning: Option<String>,
}

fn floating(ui: &egui::Ui, position: egui::Pos2, work: &mut Workspace) -> bool {
    let mut confirm = false;
    let focus = std::mem::take(&mut work.operation.focus);
    egui::Area::new(egui::Id::new("sketch_operation_floating"))
        .order(egui::Order::Foreground)
        .fixed_pos(position + egui::vec2(18., 18.))
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1., theme::ACCENT))
                .corner_radius(4)
                .inner_margin(4)
                .show(ui, |ui| {
                    let mut field = |ui: &mut egui::Ui,
                                     label: &str,
                                     value: &mut String,
                                     key: &'static str,
                                     request_focus: bool| {
                        ui.label(label);
                        let id = egui::Id::new(key);
                        let response = ui.add(
                            egui::TextEdit::singleline(value)
                                .id(id)
                                .desired_width(100.)
                                .font(egui::TextStyle::Monospace),
                        );
                        if response.changed() {
                            work.operation.confirm = false;
                        }
                        if request_focus {
                            response.request_focus();
                            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                                state
                                    .cursor
                                    .set_char_range(Some(egui::text::CCursorRange::two(
                                        egui::text::CCursor::new(0),
                                        egui::text::CCursor::new(value.chars().count()),
                                    )));
                                state.store(ui.ctx(), id);
                            }
                        }
                        if response.has_focus() || response.lost_focus() {
                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                confirm = true;
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::Tab)) {
                                response.surrender_focus();
                            }
                        }
                    };
                    ui.horizontal(|ui| {
                        if work.operation.second_side {
                            field(
                                ui,
                                text(T::SketchOffsetSecond),
                                &mut work.offset_second_expression,
                                "sketch_offset_second",
                                focus,
                            );
                        } else {
                            field(
                                ui,
                                text(T::SketchValue),
                                &mut work.dimension_expression,
                                "sketch_offset_distance",
                                focus,
                            );
                        }
                    });
                });
        });
    confirm
}

#[allow(clippy::too_many_arguments)]
pub(super) fn canvas(
    ui: &egui::Ui,
    response: &egui::Response,
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    source: &Sketch,
    work: &mut Workspace,
    parameters: &nator_document::parameters::Parameters,
    scene: SceneKey,
    hover: Option<Point2>,
    enabled: bool,
) -> CanvasUpdate {
    let Some(action) = work.active_action.filter(|a| supported(*a)) else {
        return CanvasUpdate::default();
    };
    let mut result = CanvasUpdate {
        blocked: action == EditAction::Offset,
        ..Default::default()
    };
    if !enabled {
        return result;
    }
    if action == EditAction::Offset
        && work.operation.selecting
        && response.clicked_by(egui::PointerButton::Primary)
        && let Some(cursor) = hover
        && let Ok(Some(id)) =
            nator_sketch::tools::pick_entity(source, cursor, 8. / frame.pixels_per_mm(cursor))
    {
        let chosen = if work.operation.whole_chain {
            nator_sketch::connected::chain(source, id).unwrap_or_else(|_| vec![id])
        } else {
            vec![id]
        };
        if ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
            for id in chosen {
                if let Some(index) = work.selected.iter().position(|p| *p == id) {
                    work.selected.remove(index);
                } else {
                    work.selected.push(id);
                }
            }
        } else {
            work.selected = chosen;
            work.operation.selecting = false;
        }
        work.operation.seed = Some(id);
        work.operation.focus = true;
    }
    if matches!(
        action,
        EditAction::Offset | EditAction::Fillet | EditAction::Chamfer
    ) {
        let handles = operation_handles::update(
            ui, response, frame, painter, source, work, parameters, action,
        );
        result.blocked |= handles.blocked;
        if let Some(position) = handles.position
            && floating(ui, position, work)
        {
            work.operation.confirm = true;
        }
    }
    match Request::from(work, action, parameters) {
        Ok(request) => {
            if let Err(error) = work
                .operation
                .cache
                .prepare(source, &request, scene, ui.ctx())
            {
                result.warning = Some(error);
            }
            work.operation
                .cache
                .paint(source, &request, scene, frame, painter);
            if work.operation.confirm
                && let Some(candidate) = work
                    .operation
                    .cache
                    .result(source, &request, scene)
                    .cloned()
            {
                work.operation.confirm = false;
                match candidate {
                    Ok(candidate) => {
                        work.active_action = None;
                        result.commit = Some((candidate, SolveMode::Normal));
                    }
                    Err(error) => result.warning = Some(error),
                }
            }
        }
        Err(error) if work.operation.confirm => {
            work.operation.confirm = false;
            result.warning = Some(error);
        }
        Err(_) => {}
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_distances_discard_older_worker_results() -> Result<(), Box<dyn std::error::Error>> {
        let mut source = Sketch::new("Prévia", nator_sketch::Plane::xy());
        let line = source.add_line([0., 0.], [10., 0.], false)?;
        let mut work = Workspace::new(SketchId(1));
        work.selected = vec![line];
        work.dimension_expression = "1 mm".into();
        let parameters = nator_document::parameters::Parameters::default();
        let first = Request::from(&work, EditAction::Offset, &parameters)?;
        work.dimension_expression = "2 mm".into();
        let second = Request::from(&work, EditAction::Offset, &parameters)?;
        let scene = SceneKey {
            tab: 1,
            revision: 0,
            view: 0,
            active: None,
            preview: 0,
        };
        let context = egui::Context::default();
        let mut cache = Cache::default();
        cache.prepare(&source, &first, scene, &context)?;
        cache.prepare(&source, &second, scene, &context)?;
        assert!(cache.result(&source, &second, scene).is_none());
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            cache.prepare(&source, &second, scene, &context)?;
            if let Some(result) = cache.result(&source, &second, scene) {
                let result = result.as_ref().map_err(|e| e.to_owned())?;
                let created = result.entities.last().ok_or("Cópia ausente")?;
                assert_eq!(result.line(created.id)?, ([0., 2.], [10., 2.]));
                break;
            }
            if Instant::now() > deadline {
                return Err("Prévia não terminou".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(cache.result(&source, &first, scene).is_none());
        assert_eq!(source.entities.len(), 1);
        Ok(())
    }
}
