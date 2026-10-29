//! Seleção transacional de projeções. Coordenadas de origem são de montagem;
//! coordenadas projetadas são locais ao rascunho. Consultas e materialização
//! rodam em trabalhadores, e nenhuma escolha publica uma revisão por si só.
use super::*;
use nator_commands::sketch_projection::{self, Element, Kind};

#[derive(Default)]
pub(super) struct Cache {
    key: Option<projection::Key>,
    elements: Arc<Vec<Element>>,
    running: Option<Receiver<Result<Vec<Element>, String>>>,
    filter: Option<Kind>,
    selected: Vec<usize>,
    pick: Option<(String, Receiver<Option<usize>>)>,
    hovered: Option<(String, Option<usize>)>,
    click: Option<(String, egui::Pos2)>,
    finish: bool,
    cancel: bool,
    applying: Option<Receiver<Result<Sketch, String>>>,
}
#[derive(Default)]
pub(super) struct Update {
    pub commit: Option<Sketch>,
    pub finished: bool,
    pub warning: Option<String>,
}
fn kind_label(kind: Kind) -> T {
    match kind {
        Kind::Vertex => T::ProjectionVertex,
        Kind::Edge => T::ProjectionEdge,
        Kind::Plane => T::ProjectionArea,
    }
}
impl Cache {
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn ready(&self) -> bool {
        self.key.is_some() && self.running.is_none() && self.pick.is_none()
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn selected_count(&self) -> usize {
        self.selected.len()
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
        ui.label(format!(
            "{}: {}",
            text(T::ProjectionSelected),
            self.selected.len()
        ));
        let mut remove = None;
        egui::ScrollArea::vertical()
            .max_height(180.)
            .show(ui, |ui| {
                for index in &self.selected {
                    if let Some(element) = self.elements.get(*index) {
                        let mut checked = true;
                        if ui
                            .add_enabled(
                                self.applying.is_none(),
                                egui::Checkbox::new(
                                    &mut checked,
                                    format!(
                                        "{} {} · {}",
                                        text(kind_label(element.kind)),
                                        index + 1,
                                        element.source.name
                                    ),
                                ),
                            )
                            .changed()
                        {
                            remove = Some(*index);
                        }
                    }
                }
            });
        if let Some(index) = remove {
            self.selected.retain(|i| *i != index);
        }
        ui.label(text(T::ProjectionPickHint));
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.selected.is_empty()
                        && self.applying.is_none()
                        && self.running.is_none()
                        && self.click.is_none(),
                    egui::Button::new(text(T::ProjectionFinish)),
                )
                .clicked()
            {
                self.finish = true;
            }
            if ui.button(text(T::Cancel)).clicked() {
                self.cancel = true;
            }
        });
    }
    pub(super) fn prepare(
        &mut self,
        key: projection::Key,
        document: &Document,
        view: &ViewState,
        context: &egui::Context,
    ) -> Result<(), String> {
        if self.key != Some(key) {
            let filter = self.filter;
            *self = Self {
                filter,
                ..Default::default()
            };
            self.key = Some(key);
            let occurrences = document
                .occurrences()
                .map_err(sketch_error)?
                .into_iter()
                .filter(|o| view.pickable(o))
                .collect::<Vec<_>>();
            let document = document.clone();
            let context = context.clone();
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("sketch-project-elements".into())
                .spawn(move || {
                    let result = sketch_projection::elements_for_sketch(
                        &document,
                        &occurrences,
                        key.plane,
                        key.placement,
                        Some(key.sketch),
                    )
                    .map_err(sketch_error);
                    let _ = sender.send(result);
                    context.request_repaint();
                })
                .map_err(sketch_error)?;
            self.running = Some(receiver);
        }
        if let Some(receiver) = &self.running {
            match receiver.try_recv() {
                Ok(result) => {
                    self.running = None;
                    self.elements = Arc::new(result?);
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
        &mut self,
        ui: &egui::Ui,
        frame: &sketch_scene::Frame,
        painter: &egui::Painter,
        pointer: Option<egui::Pos2>,
        clicked: bool,
        source: &Sketch,
    ) -> Update {
        if self.cancel {
            return Update {
                finished: true,
                ..Default::default()
            };
        }
        let mut result = Update::default();
        if let Some(receiver) = &self.pick {
            match receiver.1.try_recv() {
                Ok(index) => {
                    self.hovered = Some((receiver.0.clone(), index));
                    self.pick = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pick = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        // A chave inclui câmera, área e seleção: orbitar não confunde origem e
        // projeção; uma entrega de outro ponteiro nunca realça o elemento errado.
        let key = pointer.map(|p| {
            format!(
                "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
                frame.camera, frame.bounds, frame.rect, p, self.filter, self.selected
            )
        });
        if clicked && self.applying.is_none() {
            self.click = key.clone().zip(pointer);
        }
        if let Some((key, index)) = &self.hovered
            && self.click.as_ref().map(|c| &c.0) == Some(key)
        {
            if let Some(index) = index {
                if let Some(at) = self.selected.iter().position(|i| i == index) {
                    self.selected.remove(at);
                } else {
                    self.selected.push(*index);
                }
            }
            self.click = None;
        }
        let request = self.click.clone().or_else(|| key.clone().zip(pointer));
        if self.pick.is_none()
            && let Some((key, pointer)) = request.as_ref()
            && self.hovered.as_ref().is_none_or(|(old, _)| old != key)
            && self.applying.is_none()
            && self.running.is_none()
        {
            let pointer = *pointer;
            let elements = self.elements.clone();
            let frame = frame.clone();
            let selected = self.selected.clone();
            let filter = self.filter;
            let context = ui.ctx().clone();
            let (sender, receiver) = mpsc::channel();
            match std::thread::Builder::new()
                .name("projection-pick".into())
                .spawn(move || {
                    let found = pick(&elements, &frame, pointer, filter, &selected);
                    let _ = sender.send(found);
                    context.request_repaint();
                }) {
                Ok(_) => self.pick = Some((key.clone(), receiver)),
                Err(e) => result.warning = Some(e.to_string()),
            }
        }
        for index in &self.selected {
            if let Some(element) = self.elements.get(*index) {
                paint_target(frame, painter, element);
                paint_source(frame, painter, element, false);
            }
        }
        if let Some((old, Some(index))) = &self.hovered
            && key.as_ref() == Some(old)
            && let Some(element) = self.elements.get(*index)
        {
            paint_target(frame, painter, element);
            paint_source(frame, painter, element, true);
        }
        if std::mem::take(&mut self.finish) && self.applying.is_none() {
            let chosen = self
                .selected
                .iter()
                .filter_map(|i| self.elements.get(*i))
                .cloned()
                .collect::<Vec<_>>();
            let mut next = source.clone();
            let context = ui.ctx().clone();
            let (sender, receiver) = mpsc::channel();
            match std::thread::Builder::new()
                .name("projection-apply".into())
                .spawn(move || {
                    let curves = chosen
                        .iter()
                        .flat_map(|e| e.curves.iter().cloned())
                        .collect::<Vec<_>>();
                    let vertices = chosen
                        .iter()
                        .flat_map(|e| e.vertices.iter().copied())
                        .collect::<Vec<_>>();
                    let result = nator_sketch::projection::insert(&mut next, &curves, &vertices)
                        .map(|_| next)
                        .map_err(sketch_error);
                    let _ = sender.send(result);
                    context.request_repaint();
                }) {
                Ok(_) => self.applying = Some(receiver),
                Err(e) => result.warning = Some(e.to_string()),
            }
        }
        if let Some(receiver) = &self.applying {
            match receiver.try_recv() {
                Ok(Ok(candidate)) => {
                    result.commit = Some(candidate);
                    result.finished = true;
                    self.applying = None;
                }
                Ok(Err(error)) => {
                    result.warning = Some(error);
                    self.applying = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    result.warning = Some(text(T::PointQueryFailed).into());
                    self.applying = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        result
    }
}
fn segment_distance(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_sq().max(1e-12)).clamp(0., 1.);
    p.distance(a + d * t)
}
fn inside(points: &[egui::Pos2], p: egui::Pos2) -> bool {
    let mut value = false;
    for (a, b) in points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
    {
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (b.x - a.x) * (p.y - a.y) / (b.y - a.y) {
            value = !value;
        }
    }
    value
}
/// Distância ao longo do raio: faces sobrepostas são escolhidas pela profundidade,
/// mesmo quando o plano do rascunho está inclinado em relação ao corpo.
fn triangle_depth(
    ray: nator_geometry::query::Ray,
    [a, b, c]: [nator_geometry::Point3; 3],
) -> Option<f64> {
    use nator_geometry::{cross, dot, norm, sub};
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let p = cross(ray.direction, e2);
    let d = dot(e1, p);
    if d.abs() < 1e-12 * norm(e1) * norm(e2) {
        return None;
    }
    let s = sub(ray.origin, a);
    let u = dot(s, p) / d;
    let q = cross(s, e1);
    let v = dot(ray.direction, q) / d;
    let t = dot(e2, q) / d;
    (u >= -1e-9 && v >= -1e-9 && u + v <= 1. + 1e-9 && t >= 0.).then_some(t)
}
fn pick(
    elements: &[Element],
    frame: &sketch_scene::Frame,
    pointer: egui::Pos2,
    filter: Option<Kind>,
    selected: &[usize],
) -> Option<usize> {
    let world = |p| sketch_scene::project(&frame.camera, frame.bounds, frame.rect, p);
    let cursor = [
        f64::from((pointer.x - frame.rect.center().x) / frame.rect.width() * 2.),
        f64::from((frame.rect.center().y - pointer.y) / frame.rect.height() * 2.),
    ];
    let ray = frame
        .camera
        .world_ray(cursor, frame.bounds, f64::from(frame.rect.aspect_ratio()));
    let depth = |p| nator_geometry::dot(nator_geometry::sub(p, ray.origin), ray.direction);
    let mut best: Option<(usize, u8, f32, f64)> = None;
    for (index, element) in elements
        .iter()
        .enumerate()
        .filter(|(_, e)| filter.is_none_or(|k| e.kind == k))
    {
        let distance = match element.kind {
            Kind::Vertex => element
                .source
                .vertices
                .iter()
                .filter_map(|p| world(*p))
                .map(|p| p.distance(pointer))
                .fold(f32::INFINITY, f32::min),
            Kind::Edge => element
                .source
                .curves
                .iter()
                .flat_map(|c| c.windows(2))
                .filter_map(|p| Some(segment_distance(pointer, world(p[0])?, world(p[1])?)))
                .fold(f32::INFINITY, f32::min),
            Kind::Plane => {
                let hit = element.source.triangles.iter().any(|t| {
                    t.iter()
                        .map(|p| world(*p))
                        .collect::<Option<Vec<_>>>()
                        .is_some_and(|p| inside(&p, pointer))
                }) || element
                    .source
                    .contours
                    .iter()
                    .filter(|c| {
                        c.iter()
                            .map(|p| world(*p))
                            .collect::<Option<Vec<_>>>()
                            .is_some_and(|p| inside(&p, pointer))
                    })
                    .count()
                    % 2
                    == 1;
                if hit { 0. } else { f32::INFINITY }
            }
        };
        let rank = match element.kind {
            Kind::Vertex => 0,
            Kind::Edge => 1,
            Kind::Plane => 2,
        };
        let source_depth = element
            .source
            .triangles
            .iter()
            .filter_map(|t| triangle_depth(ray, *t))
            .fold(f64::INFINITY, f64::min);
        let source_depth = if source_depth.is_finite() {
            source_depth
        } else {
            element
                .source
                .vertices
                .iter()
                .chain(element.source.curves.iter().flatten())
                .chain(element.source.contours.iter().flatten())
                .map(|p| depth(*p))
                .fold(f64::INFINITY, f64::min)
        };
        if distance <= 8.
            && best.is_none_or(|(_, r, d, z)| {
                rank < r
                    || (rank == r
                        && (distance < d - 0.001
                            || ((distance - d).abs() <= 0.001 && source_depth < z)))
            })
        {
            best = Some((index, rank, distance, source_depth));
        }
    }
    // A projeção selecionada é também um alvo para retirar a mesma origem,
    // inclusive quando o corpo está fora do plano ou fora do enquadramento.
    if let Some(cursor) = frame.cursor(pointer) {
        let choices = selected
            .iter()
            .filter_map(|i| elements.get(*i).cloned().map(|e| (*i, e)))
            .collect::<Vec<_>>();
        let projected = choices.iter().map(|(_, e)| e.clone()).collect::<Vec<_>>();
        if let Some(index) =
            sketch_projection::pick(&projected, cursor, 8. / frame.pixels_per_mm(cursor), filter)
        {
            let rank = match projected[index].kind {
                Kind::Vertex => 0,
                Kind::Edge => 1,
                Kind::Plane => 2,
            };
            if best.is_none_or(|(i, r, _, _)| rank <= r || selected.contains(&i)) {
                return Some(choices[index].0);
            }
        }
    }
    best.map(|(index, _, _, _)| index)
}
fn paint_source(
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    element: &Element,
    hovered: bool,
) {
    let stroke = egui::Stroke::new(if hovered { 2.5 } else { 1.5 }, theme::GREEN);
    let world = |p| sketch_scene::project(&frame.camera, frame.bounds, frame.rect, p);
    for curve in element.source.curves.iter().chain(&element.source.contours) {
        let points = curve.iter().filter_map(|p| world(*p)).collect::<Vec<_>>();
        if points.len() > 1 {
            painter.add(egui::Shape::line(points, stroke));
        }
    }
    for point in &element.source.vertices {
        if let Some(at) = world(*point) {
            painter.circle_stroke(at, 5., stroke);
        }
    }
}
fn paint_target(frame: &sketch_scene::Frame, painter: &egui::Painter, element: &Element) {
    let polygons = element
        .contours
        .iter()
        .map(|c| c.iter().filter_map(|p| frame.screen(*p)).collect())
        .collect::<Vec<Vec<_>>>();
    profile_pick::fill(painter, &polygons, theme::PROJECTED_FILL);
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
        if let Some(at) = frame.screen(*point) {
            painter.circle_filled(at, 5., theme::PROJECTED);
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

#[cfg(test)]
mod tests {
    use super::*;
    use nator_commands::sketch_projection::SourceGeometry;
    #[test]
    fn original_and_selected_projection_are_picked_in_their_own_planes() -> Result<(), String> {
        let mut camera = nator_renderer::Camera::default();
        camera.set_direction([1., -1., 2.]);
        let frame = sketch_scene::Frame {
            camera,
            bounds: sketch_scene::EMPTY_BOUNDS,
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.)),
            placement: Placement::default(),
            plane: nator_sketch::Plane {
                origin: [0., 0., 30.],
                ..nator_sketch::Plane::xy()
            },
        };
        let original = [5., 8., -20.];
        let point = [5., 8.];
        let element = Element {
            kind: Kind::Vertex,
            source: SourceGeometry {
                vertices: vec![original],
                ..Default::default()
            },
            vertices: vec![point],
            curves: vec![],
            triangles: vec![],
            contours: vec![],
        };
        let original = sketch_scene::project(&frame.camera, frame.bounds, frame.rect, original)
            .ok_or("Origem invisível")?;
        let projected = frame.screen(point).ok_or("Projeção invisível")?;
        assert!(original.distance(projected) > 30.);
        assert_eq!(
            pick(std::slice::from_ref(&element), &frame, original, None, &[]),
            Some(0)
        );
        assert_eq!(
            pick(std::slice::from_ref(&element), &frame, projected, None, &[]),
            None
        );
        assert_eq!(pick(&[element], &frame, projected, None, &[0]), Some(0));
        Ok(())
    }
    #[test]
    fn nearest_face_wins_when_source_faces_overlap() -> Result<(), String> {
        let mut camera = nator_renderer::Camera::default();
        camera.set_direction([0., 0., 1.]);
        let frame = sketch_scene::Frame {
            camera,
            bounds: sketch_scene::EMPTY_BOUNDS,
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.)),
            placement: Placement::default(),
            plane: nator_sketch::Plane::xy(),
        };
        let elements = [-20., 20.].map(|z| Element {
            kind: Kind::Plane,
            source: SourceGeometry {
                triangles: vec![[[-10., -10., z], [10., -10., z], [0., 10., z]]],
                ..Default::default()
            },
            curves: vec![],
            vertices: vec![],
            triangles: vec![],
            contours: vec![],
        });
        let at = frame.screen([0., 0.]).ok_or("Face invisível")?;
        assert_eq!(pick(&elements, &frame, at, Some(Kind::Plane), &[]), Some(1));
        Ok(())
    }
}
