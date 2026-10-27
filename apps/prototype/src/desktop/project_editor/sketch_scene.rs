//! Apresentação dos rascunhos e da origem no mesmo referencial do viewport CAD.
//! Conversões de cursor passam pelo raio da câmera e pelo plano local da peça,
//! incluindo posições hierárquicas; não existe uma segunda câmera de edição 2D.
use super::*;
use nator_document::assembly::Pick;
use nator_geometry::{Bounds, dot, sub};
use nator_sketch::{Plane, Point2, Sketch, profiles};

pub(super) const EMPTY_BOUNDS: Bounds = Bounds {
    min: [-50.; 3],
    max: [50.; 3],
};

pub(super) struct PlanePicker {
    pub destination: Option<PartId>,
    pub placement: Placement,
    /// Rascunho de referência da montagem: plano em coordenadas da montagem e nenhuma peça
    /// dona, para que não possa virar perfil de corpo.
    pub reference: bool,
}

/// Comando que cria o rascunho escolhido pelo seletor de plano.
fn new_sketch(destination: Option<PartId>, reference: bool, plane: Plane) -> Command {
    if reference {
        Command::AddReferenceSketch {
            sketch: Sketch::new(text(T::ReferenceSketch), plane),
        }
    } else {
        Command::AddSketch {
            destination,
            sketch: Sketch::new(text(T::Sketch), plane),
        }
    }
}
pub(super) struct CameraTransition {
    pub from: Camera,
    pub to: Camera,
    pub started: Instant,
}

#[derive(Clone)]
pub(super) struct Frame {
    pub camera: Camera,
    pub bounds: Bounds,
    pub rect: egui::Rect,
    pub placement: Placement,
    pub plane: Plane,
}
impl Frame {
    pub fn world(&self, point: Point2) -> Point3 {
        self.placement.apply(self.plane.world(point))
    }
    pub fn screen(&self, point: Point2) -> Option<egui::Pos2> {
        project(&self.camera, self.bounds, self.rect, self.world(point))
    }
    pub fn cursor(&self, point: egui::Pos2) -> Option<Point2> {
        let ray = self.camera.world_ray(
            [
                f64::from((point.x - self.rect.center().x) / self.rect.width() * 2.),
                f64::from((self.rect.center().y - point.y) / self.rect.height() * 2.),
            ],
            self.bounds,
            f64::from(self.rect.aspect_ratio()),
        );
        let origin = self.placement.inverse_apply(ray.origin);
        let direction = self.placement.inverse_vector(ray.direction);
        let denominator = dot(direction, self.plane.normal());
        if denominator.abs() < 1e-9 {
            return None;
        }
        let distance = dot(sub(self.plane.origin, origin), self.plane.normal()) / denominator;
        if distance < 0. || !distance.is_finite() {
            return None;
        }
        let hit = std::array::from_fn(|i| origin[i] + direction[i] * distance);
        Some(self.plane.project(hit))
    }
    pub fn pixels_per_mm(&self, point: Point2) -> f64 {
        let Some(a) = self.screen(point) else {
            return 1.;
        };
        let b = self.screen([point[0] + 1., point[1]]);
        let c = self.screen([point[0], point[1] + 1.]);
        b.into_iter()
            .chain(c)
            .map(|p| f64::from(a.distance(p)))
            .fold(0_f64, f64::max)
            .max(1e-6)
    }
    pub fn path(
        &self,
        painter: &egui::Painter,
        points: impl IntoIterator<Item = Point2>,
        stroke: egui::Stroke,
        dashed: bool,
    ) {
        let mut segment = Vec::new();
        let flush = |points: &mut Vec<egui::Pos2>| {
            if points.len() > 1 {
                if dashed {
                    for shape in egui::Shape::dashed_line(points, stroke, 6., 4.) {
                        painter.add(shape);
                    }
                } else {
                    painter.add(egui::Shape::line(points.clone(), stroke));
                }
            }
            points.clear();
        };
        for point in points {
            if let Some(p) = self.screen(point) {
                segment.push(p);
            } else {
                flush(&mut segment);
            }
        }
        flush(&mut segment);
    }
}
pub(super) fn project(
    camera: &Camera,
    bounds: Bounds,
    rect: egui::Rect,
    point: Point3,
) -> Option<egui::Pos2> {
    let p = camera.project_world(point, bounds, f64::from(rect.aspect_ratio()))?;
    let p = rect.center()
        + egui::vec2(
            p[0] as f32 * rect.width() * 0.5,
            -p[1] as f32 * rect.height() * 0.5,
        );
    p.is_finite().then_some(p)
}

pub(super) fn part_placement(tab: &Tab, part: PartId) -> Result<Placement, String> {
    if let Some(pick) = tab.picked.as_ref().filter(|p| p.part == part) {
        return Ok(pick.placement);
    }
    let document = tab.session.document();
    let occurrences = document.occurrences().map_err(|e| e.to_string())?;
    let selected = match tab.selection {
        Some(Selection::Instance(id)) => Some(id),
        _ => None,
    };
    occurrences
        .iter()
        .filter(|o| {
            document
                .instance(o.instance)
                .is_some_and(|i| i.part == part)
        })
        .min_by_key(|o| if Some(o.instance) == selected { 0 } else { 1 })
        .map(|o| o.placement)
        .ok_or_else(|| text(T::SketchHiddenPart).to_owned())
}
pub(super) fn transformed_plane(plane: Plane, placement: Placement) -> Plane {
    Plane {
        origin: placement.apply(plane.origin),
        u: placement.apply_vector(plane.u),
        v: placement.apply_vector(plane.v),
    }
}
pub(super) fn local_plane(plane: Plane, placement: Placement) -> Plane {
    Plane {
        origin: placement.inverse_apply(plane.origin),
        u: placement.inverse_vector(plane.u),
        v: placement.inverse_vector(plane.v),
    }
}

/// A origem do rascunho é a projeção da origem da peça sobre a face; os eixos
/// dependem da normal local, para não variar com o ponto clicado nem com a
/// orientação da ocorrência na montagem. O suporte é uma captura do plano.
fn face_work_plane(world: Plane, placement: Placement) -> Result<Plane, nator_sketch::SketchError> {
    let local = local_plane(world, placement);
    let normal = local.normal();
    let distance = dot(local.origin, normal);
    Plane::from_normal(normal.map(|v| v * distance), normal)
}

impl Editor {
    pub(super) fn display_bounds(&self) -> Bounds {
        self.current().scene_bounds.unwrap_or(EMPTY_BOUNDS)
    }
    pub(super) fn start_sketch_plane(&mut self) {
        if self.busy() || self.current().sketch_edit.is_some() {
            return;
        }
        let destination = self.current().active_part;
        let placement = match destination
            .map(|id| part_placement(self.current(), id))
            .transpose()
        {
            Ok(p) => p.unwrap_or_default(),
            Err(error) => {
                self.status.error(error);
                return;
            }
        };
        self.current_mut().plane_pick = Some(PlanePicker {
            destination,
            placement,
            reference: false,
        });
        self.status.set(text(T::ChooseSketchPlane));
    }
    /// Seletor de plano para um rascunho de referência da montagem.
    pub(super) fn start_reference_sketch_plane(&mut self) {
        if self.busy() || self.current().sketch_edit.is_some() {
            return;
        }
        // Referências pertencem à montagem: sair da construção deixa o contexto claro.
        self.current_mut().active_part = None;
        self.current_mut().plane_pick = Some(PlanePicker {
            destination: None,
            placement: Placement::default(),
            reference: true,
        });
        self.status.set(text(T::ChooseSketchPlane));
    }
    pub(super) fn choose_origin_plane(&mut self, plane: Plane) {
        if self.busy() {
            return;
        }
        let Some(picker) = self.current().plane_pick.as_ref() else {
            return;
        };
        let command = new_sketch(picker.destination, picker.reference, plane);
        self.execute(command);
    }
    pub(super) fn choose_face_plane(
        &mut self,
        context: &egui::Context,
        ray: nator_geometry::query::Ray,
    ) {
        if self.busy() {
            return;
        }
        let Some(picker) = self.current().plane_pick.as_ref() else {
            return;
        };
        let (destination, placement, reference) =
            (picker.destination, picker.placement, picker.reference);
        let document = self.current().session.document().clone();
        let (tab, revision) = (self.current().id, self.current().session.revision());
        let view = self.current().view.clone();
        self.start(context, None, move || {
            let pick = document
                .pick_with(ray, |o| view.pickable(o))
                .map_err(|e| e.to_string())?
                .ok_or_else(|| text(T::ChooseSketchPlane).to_owned())?;
            let world = nator_commands::measure::sketch_plane(&document, &pick)
                .map_err(|e| e.to_string())?;
            let plane = face_work_plane(world, placement).map_err(|e| e.to_string())?;
            let command = PreparedCommand::new(
                document,
                revision,
                new_sketch(destination, reference, plane),
            )
            .map_err(|e| e.to_string())?;
            Ok(Outcome::Prepared { tab, command })
        });
    }
    pub(super) fn choose_picked_plane(&mut self, context: &egui::Context, pick: Pick) {
        if self.busy() {
            return;
        }
        let Some(picker) = self.current().plane_pick.as_ref() else {
            return;
        };
        let (destination, placement, reference) =
            (picker.destination, picker.placement, picker.reference);
        let document = self.current().session.document().clone();
        let (tab, revision) = (self.current().id, self.current().session.revision());
        self.start(context, None, move || {
            let world = nator_commands::measure::sketch_plane(&document, &pick)
                .map_err(|e| e.to_string())?;
            let plane = face_work_plane(world, placement).map_err(|e| e.to_string())?;
            let command = PreparedCommand::new(
                document,
                revision,
                new_sketch(destination, reference, plane),
            )
            .map_err(|e| e.to_string())?;
            Ok(Outcome::Prepared { tab, command })
        });
    }
    pub(super) fn finish_sketch(&mut self) {
        if self.busy()
            || self
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.pending.is_some())
        {
            return;
        }
        if let Some(work) = self.current_mut().sketch_edit.take() {
            let mut next = self.viewport.camera.clone();
            next.copy_orientation(&work.previous_camera);
            self.current_mut().camera_transition = Some(CameraTransition {
                from: self.viewport.camera.clone(),
                to: next,
                started: Instant::now(),
            });
        }
    }
    pub(super) fn animate_camera(&mut self, context: &egui::Context) {
        if let Some(transition) = &self.current().camera_transition {
            let fraction = (transition.started.elapsed().as_secs_f64() / 0.22).min(1.);
            self.viewport.camera = transition
                .from
                .interpolated(&transition.to, fraction * fraction * (3. - 2. * fraction));
            if fraction >= 1. {
                self.current_mut().camera_transition = None;
            } else {
                context.request_repaint();
            }
        }
    }
    pub(super) fn align_sketch_camera(&mut self, fit: bool, animate: bool) {
        let Some(work) = &self.current().sketch_edit else {
            return;
        };
        let Some(sketch) = self.working_sketch() else {
            return;
        };
        let plane = transformed_plane(sketch.plane, work.placement);
        let mut next = self.viewport.camera.clone();
        let bounds = self.display_bounds();
        let mut center = [0., 0.];
        let mut extent = [40., 40.];
        if let Some((min, max)) = sketch_limits(&sketch) {
            center = std::array::from_fn(|i| min[i] * 0.5 + max[i] * 0.5);
            extent = std::array::from_fn(|i| (max[i] - min[i]).max(10.));
        }
        if let Err(error) = next.align_plane(plane.world(center), plane.u, plane.v, bounds) {
            self.status.error(error);
            return;
        }
        if fit {
            let [w, h] = self.viewport.size();
            let aspect = f64::from(w) / f64::from(h.max(1));
            let scale = bounds.size().into_iter().fold(0_f64, f64::max);
            next.span = (extent[1].max(extent[0] / aspect) * 1.35 / scale).max(1e-6);
        }
        if animate {
            self.current_mut().camera_transition = Some(CameraTransition {
                from: self.viewport.camera.clone(),
                to: next,
                started: Instant::now(),
            });
        } else {
            self.viewport.camera = next;
            self.current_mut().camera_transition = None;
        }
    }
    pub(super) fn visible_sketches(&self, ui: &egui::Ui, rect: egui::Rect) -> bool {
        let doc = self.current().session.document();
        let active = self
            .current()
            .sketch_edit
            .as_ref()
            .map(|w| (w.id, w.placement));
        let Ok(occurrences) = doc.occurrences() else {
            return false;
        };
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let mut visible = false;
        for occurrence in occurrences
            .iter()
            .filter(|o| self.current().view.in_isolation(o))
        {
            let Some(part) = doc
                .instance(occurrence.instance)
                .and_then(|i| doc.part(i.part))
            else {
                continue;
            };
            for record in part
                .sketches
                .iter()
                .filter(|s| part.shows_sketch(s) && Some((s.id, occurrence.placement)) != active)
            {
                visible |= !record.sketch.points.is_empty();
                let frame = Frame {
                    camera: self.viewport.camera.clone(),
                    bounds: self.display_bounds(),
                    rect,
                    placement: occurrence.placement,
                    plane: record.sketch.plane,
                };
                for point in &record.sketch.points {
                    if let Some(p) = frame.screen(point.position) {
                        painter.circle_filled(p, 2., theme::ACCENT);
                    }
                }
                for entity in &record.sketch.entities {
                    if let Ok(curve) = profiles::segment(&record.sketch, entity) {
                        frame.path(
                            &painter,
                            curve.sample(96),
                            egui::Stroke::new(1.5, theme::ACCENT),
                            entity.construction,
                        );
                    }
                }
            }
        }
        // Rascunhos de referência: coordenadas da montagem, cor própria e traço mais fino.
        for record in doc.reference_sketches().iter().filter(|s| {
            doc.shows_reference_sketch(s) && Some((s.id, Placement::default())) != active
        }) {
            visible |= !record.sketch.points.is_empty();
            let frame = Frame {
                camera: self.viewport.camera.clone(),
                bounds: self.display_bounds(),
                rect,
                placement: Placement::default(),
                plane: record.sketch.plane,
            };
            for point in &record.sketch.points {
                if let Some(p) = frame.screen(point.position) {
                    painter.circle_filled(p, 2.5, theme::REFERENCE);
                }
            }
            for entity in &record.sketch.entities {
                if let Ok(curve) = profiles::segment(&record.sketch, entity) {
                    frame.path(
                        &painter,
                        curve.sample(96),
                        egui::Stroke::new(1.5, theme::REFERENCE),
                        entity.construction,
                    );
                }
            }
        }
        visible
    }
    /// Eixos das origens visíveis na árvore: a da montagem e a de cada peça com a pasta
    /// Origem visível, em todas as ocorrências. Ao escolher o plano de um rascunho, os
    /// planos aparecem na origem do destino e só então podem ser clicados.
    pub(super) fn origin_overlay(&mut self, ui: &egui::Ui, response: &egui::Response) -> bool {
        let document = self.current().session.document();
        let mut origins = Vec::new();
        if document.shows_origin() {
            origins.push(Placement::default());
        }
        if let Ok(occurrences) = document.occurrences() {
            for occurrence in occurrences
                .iter()
                .filter(|o| self.current().view.in_isolation(o))
            {
                if document
                    .instance(occurrence.instance)
                    .and_then(|i| document.part(i.part))
                    .is_some_and(|p| p.folders.origin)
                {
                    origins.push(occurrence.placement);
                }
            }
        }
        let picker = self.current().plane_pick.as_ref().map(|p| p.placement);
        let picking = picker.is_some();
        if let Some(placement) = picker
            && !origins.contains(&placement)
        {
            origins.push(placement);
        }
        if origins.is_empty() {
            return false;
        }
        let placement = picker.unwrap_or_default();
        let bounds = self.display_bounds();
        let scale = bounds.size().into_iter().fold(0_f64, f64::max);
        let size = (scale * 0.3).clamp(10., 60.);
        let painter = ui
            .painter()
            .with_clip_rect(response.rect.intersect(ui.clip_rect()));
        let camera = &self.viewport.camera;
        let screen = |point| project(camera, bounds, response.rect, placement.apply(point));
        let hover = response
            .hover_pos()
            .filter(|p| !view_cube::rect(response.rect).contains(*p));
        let mut selected = None;
        if picking {
            let mut planes = Vec::new();
            for (name, plane, color) in [
                ("XY", Plane::xy(), egui::Color32::from_rgb(77, 142, 211)),
                ("XZ", Plane::xz(), egui::Color32::from_rgb(61, 156, 112)),
                ("YZ", Plane::yz(), egui::Color32::from_rgb(206, 119, 58)),
            ] {
                let polygon: Option<Vec<_>> = [[0., 0.], [size, 0.], [size, size], [0., size]]
                    .into_iter()
                    .map(|p| screen(plane.world(p)))
                    .collect();
                if let Some(points) = polygon {
                    let depth = dot(
                        placement.apply(plane.world([size * 0.5, size * 0.5])),
                        camera.direction(),
                    );
                    planes.push((depth, name, plane, color, points));
                }
            }
            planes.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (_, name, plane, color, points) in planes {
                let hit = hover.is_some_and(|p| polygon_contains(&points, p));
                // Triângulos e arestas independentes evitam junções de traço
                // instáveis quando um plano fica quase de perfil para a câmera.
                if !polygon_contains(
                    &points,
                    points
                        .iter()
                        .fold(egui::Pos2::ZERO, |a, b| a + b.to_vec2() / 4.),
                ) {
                    continue;
                }
                let mut mesh = egui::Mesh::default();
                for p in &points {
                    mesh.colored_vertex(*p, color.gamma_multiply(if hit { 0.45 } else { 0.18 }));
                }
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(0, 2, 3);
                painter.add(egui::Shape::mesh(mesh));
                for i in 0..4 {
                    painter.line_segment(
                        [points[i], points[(i + 1) % 4]],
                        egui::Stroke::new(if hit { 2. } else { 1. }, color),
                    );
                }
                if let Some(label) = screen(plane.world([size * 0.65, size * 0.65])) {
                    painter.text(
                        label,
                        egui::Align2::CENTER_CENTER,
                        name,
                        egui::FontId::proportional(13.),
                        color,
                    );
                }
                if hit {
                    selected = Some(plane);
                }
            }
        }
        for frame in origins {
            let screen = |point| project(camera, bounds, response.rect, frame.apply(point));
            let Some(origin) = screen([0.; 3]) else {
                continue;
            };
            for (axis, label, color) in [
                ([size, 0., 0.], "X", theme::DANGER),
                ([0., size, 0.], "Y", theme::GREEN),
                ([0., 0., size], "Z", theme::ACCENT),
            ] {
                if let Some(end) = screen(axis) {
                    painter.line_segment([origin, end], egui::Stroke::new(1.4, color));
                    painter.text(
                        end,
                        egui::Align2::CENTER_BOTTOM,
                        label,
                        egui::FontId::proportional(11.),
                        color,
                    );
                }
            }
            painter.circle_filled(origin, 3., theme::INK);
            painter.circle_stroke(origin, 6., egui::Stroke::new(1., theme::INK));
        }
        if picking
            && !self.busy()
            && response.clicked()
            && let Some(plane) = selected
        {
            self.choose_origin_plane(plane);
            return true;
        }
        false
    }
}

/// Sem sólidos, usar as extensões dos rascunhos permite reabrir/enquadrar
/// perfis deslocados. Havendo malha, os overlays usam exatamente seus limites,
/// pois a GPU normaliza os vértices por esse mesmo referencial.
pub(super) fn document_bounds(document: &Document, view: &ViewState) -> Bounds {
    let mut bounds: Option<Bounds> = None;
    let mut include = |point: Point3| {
        let bounds = bounds.get_or_insert(Bounds {
            min: point,
            max: point,
        });
        for (i, value) in point.into_iter().enumerate() {
            bounds.min[i] = bounds.min[i].min(value);
            bounds.max[i] = bounds.max[i].max(value);
        }
    };
    if let Ok(occurrences) = document.occurrences() {
        for occurrence in occurrences.iter().filter(|o| view.in_isolation(o)) {
            if let Some(part) = document
                .instance(occurrence.instance)
                .and_then(|i| document.part(i.part))
            {
                for record in part.sketches.iter().filter(|s| part.shows_sketch(s)) {
                    let sketch = &record.sketch;
                    for point in &sketch.points {
                        include(
                            occurrence
                                .placement
                                .apply(sketch.plane.world(point.position)),
                        );
                    }
                    for entity in &sketch.entities {
                        if let Ok(curve) = profiles::segment(sketch, entity) {
                            for point in curve.sample(96) {
                                include(occurrence.placement.apply(sketch.plane.world(point)));
                            }
                        }
                    }
                }
            }
        }
    }
    // Rascunhos de referência já estão em coordenadas da montagem.
    for record in document
        .reference_sketches()
        .iter()
        .filter(|s| document.shows_reference_sketch(s))
    {
        let sketch = &record.sketch;
        for point in &sketch.points {
            include(sketch.plane.world(point.position));
        }
        for entity in &sketch.entities {
            if let Ok(curve) = profiles::segment(sketch, entity) {
                for point in curve.sample(96) {
                    include(sketch.plane.world(point));
                }
            }
        }
    }
    let Some(mut bounds) = bounds else {
        return EMPTY_BOUNDS;
    };
    let center = bounds.center();
    for (i, value) in center.into_iter().enumerate() {
        if bounds.max[i] - bounds.min[i] < 10. {
            bounds.min[i] = value - 5.;
            bounds.max[i] = value + 5.;
        }
    }
    bounds
}

pub(super) fn sketch_limits(sketch: &Sketch) -> Option<(Point2, Point2)> {
    let first = sketch.points.first()?.position;
    let mut min = first;
    let mut max = first;
    for point in &sketch.points {
        for i in 0..2 {
            min[i] = min[i].min(point.position[i]);
            max[i] = max[i].max(point.position[i]);
        }
    }
    for entity in &sketch.entities {
        if let Ok(curve) = profiles::segment(sketch, entity) {
            for p in curve.sample(96) {
                for i in 0..2 {
                    min[i] = min[i].min(p[i]);
                    max[i] = max[i].max(p[i]);
                }
            }
        }
    }
    Some((min, max))
}
fn polygon_contains(points: &[egui::Pos2], p: egui::Pos2) -> bool {
    let mut sign = 0.;
    let mut area = 0.;
    for (a, b) in points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
    {
        let c = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        area += a.x * b.y - b.x * a.y;
        if c.abs() > 0.1 {
            if sign == 0. {
                sign = c.signum();
            } else if sign * c < 0. {
                return false;
            }
        }
    }
    area.abs() > 1.
}

#[cfg(test)]
mod tests {
    use super::*;
    use nator_renderer::Projection;

    #[test]
    fn face_origin_and_axes_are_independent_of_the_clicked_point_and_occurrence()
    -> Result<(), Box<dyn std::error::Error>> {
        let placement = Placement::new([30., -20., 70.], [0.4, -0.8, 0.6])?;
        let mut a = Plane::xy();
        a.origin = [12., 9., 8.];
        let mut b = a;
        b.origin = [-10., 15., 8.];
        for source in [a, b] {
            let local = face_work_plane(transformed_plane(source, placement), placement)?;
            assert!(nator_geometry::norm(sub(local.origin, [0., 0., 8.])) < 1e-12);
            assert!(nator_geometry::norm(sub(local.u, [1., 0., 0.])) < 1e-12);
            assert!(nator_geometry::norm(sub(local.v, [0., 1., 0.])) < 1e-12);
        }
        Ok(())
    }

    #[test]
    fn sketch_only_scene_bounds_include_displaced_circles_and_isolated_points()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = Document::new("Perfis")?;
        let mut sketch = Sketch::new("Perfil", Plane::xy());
        sketch.add_circle([400., -120.], 35., false)?;
        sketch.add_point([-80., 200.])?;
        let (_, id) = doc.add_sketch(None, sketch)?;
        let bounds = document_bounds(&doc, &ViewState::default());
        assert!(bounds.min[0] <= -80. && bounds.max[0] >= 435.);
        assert!(bounds.min[1] <= -155. && bounds.max[1] >= 200.);
        assert!(bounds.size()[2] >= 10.);
        doc.sketch_mut(id)?.visible = false;
        assert_eq!(
            document_bounds(&doc, &ViewState::default()).min,
            EMPTY_BOUNDS.min
        );
        Ok(())
    }

    #[test]
    fn cursor_and_sketch_use_the_same_world_projection() -> Result<(), Box<dyn std::error::Error>> {
        // Inclui plano oblíquo e ocorrência rotacionada: erros de referencial
        // parecem corretos em XY na origem, mas deslocam o clique na montagem.
        let placement = Placement::new([30., -25., 10.], [0.4, -0.6, 0.8])?;
        for plane in [
            Plane::xy(),
            Plane::xz(),
            Plane::yz(),
            Plane::from_normal([3., 5., -2.], [1., 2., 3.])?,
        ] {
            for projection in [Projection::Orthographic, Projection::Perspective] {
                let world = transformed_plane(plane, placement);
                let mut camera = Camera::default();
                camera.projection = projection;
                camera.align_plane(world.origin, world.u, world.v, EMPTY_BOUNDS)?;
                // Uma vista inclinada também permite desenhar enquanto o plano
                // não estiver de perfil; não há câmera 2D oculta.
                for orbit in [[0., 0.], [27., -19.]] {
                    camera.orbit(orbit);
                    let frame = Frame {
                        camera: camera.clone(),
                        bounds: EMPTY_BOUNDS,
                        rect: egui::Rect::from_min_size(
                            egui::pos2(244., 193.),
                            egui::vec2(711., 467.),
                        ),
                        placement,
                        plane,
                    };
                    for point in [[0., 0.], [12., -8.], [-21., 17.]] {
                        let screen = frame.screen(point).ok_or("Ponto recortado")?;
                        let hit = frame.cursor(screen).ok_or("Cursor sem interseção")?;
                        assert!(
                            nator_sketch::distance(hit, point) < 2e-5,
                            "{hit:?} ≠ {point:?}"
                        );
                        assert!(frame.pixels_per_mm(point) > 0.);
                    }
                    let back = local_plane(world, placement);
                    assert!(nator_geometry::norm(sub(back.origin, plane.origin)) < 1e-12);
                    assert!(nator_geometry::norm(sub(back.normal(), plane.normal())) < 1e-12);
                }
            }
        }
        Ok(())
    }

    #[test]
    fn edge_on_plane_cannot_capture_cursor_or_plane_selection() -> Result<(), String> {
        let mut camera = Camera::default();
        camera.set_direction([1., 0., 0.]);
        let frame = Frame {
            camera,
            bounds: EMPTY_BOUNDS,
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700., 450.)),
            placement: Placement::default(),
            plane: Plane::xy(),
        };
        assert!(frame.cursor(frame.rect.center()).is_none());
        let points = [[0., 0.], [20., 0.], [20., 20.], [0., 20.]]
            .into_iter()
            .map(|p| frame.screen(p).ok_or("Ponto sem projeção"))
            .collect::<Result<Vec<_>, _>>()?;
        assert!(!polygon_contains(&points, frame.rect.center()));
        Ok(())
    }
}
