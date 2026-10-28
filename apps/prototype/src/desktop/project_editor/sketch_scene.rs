//! Apresentação dos rascunhos e da origem no mesmo referencial do viewport CAD.
//! Conversões de cursor passam pelo raio da câmera e pelo plano local da peça,
//! incluindo posições hierárquicas; não existe uma segunda câmera de edição 2D.
use super::*;
use nator_document::{WorkPlaneId, assembly::Pick};
use nator_geometry::{Bounds, dot, sub};
use nator_sketch::{Plane, Point2, Sketch, profiles};
mod plane_hover;

pub(super) const EMPTY_BOUNDS: Bounds = Bounds {
    min: [-50.; 3],
    max: [50.; 3],
};

/// Destino do plano escolhido no seletor de suporte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PlanePurpose {
    /// Novo rascunho de peça ou, com `reference`, de referência da montagem.
    Sketch,
    /// Plano-base do plano deslocado em edição; nenhum comando é publicado na escolha.
    OffsetBase,
}

pub(super) struct PlanePicker {
    pub destination: Option<PartId>,
    pub placement: Placement,
    /// Rascunho de referência da montagem: plano em coordenadas da montagem e nenhuma peça
    /// dona, para que não possa virar perfil de corpo.
    pub reference: bool,
    pub purpose: PlanePurpose,
    /// Os planos da origem e de trabalho têm prioridade sobre a face desenhada atrás deles.
    origin_hovered: bool,
    hover: plane_hover::Cache,
}
impl PlanePicker {
    pub fn new(
        destination: Option<PartId>,
        placement: Placement,
        reference: bool,
        purpose: PlanePurpose,
    ) -> Self {
        Self {
            destination,
            placement,
            reference,
            purpose,
            origin_hovered: false,
            hover: plane_hover::Cache::default(),
        }
    }
    /// Planos de trabalho só servem a rascunhos de peça: são locais à peça de destino.
    fn offers_work_planes(&self) -> bool {
        !self.reference && self.destination.is_some()
    }
}

/// Plano clicável no seletor: um dos planos da origem ou um plano de trabalho visível.
#[derive(Clone, Copy, PartialEq)]
enum Candidate {
    Origin(&'static str, Plane),
    Work(WorkPlaneId),
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
    /// A reta é projetada e recortada ao canvas, sem inventar um comprimento em
    /// mm nem alterar os limites de enquadramento do documento ao navegar.
    pub fn infinite_line(
        &self,
        painter: &egui::Painter,
        a: Point2,
        b: Point2,
        stroke: egui::Stroke,
    ) {
        let Some((a, b)) = self.screen(a).zip(self.screen(b)) else {
            return;
        };
        let d = b - a;
        if d.length_sq() < 1e-8 {
            return;
        }
        let mut low = f32::NEG_INFINITY;
        let mut high = f32::INFINITY;
        for (p, v, min, max) in [
            (a.x, d.x, self.rect.left(), self.rect.right()),
            (a.y, d.y, self.rect.top(), self.rect.bottom()),
        ] {
            if v.abs() < 1e-6 {
                if p < min || p > max {
                    return;
                }
            } else {
                let first = (min - p) / v;
                let last = (max - p) / v;
                low = low.max(first.min(last));
                high = high.min(first.max(last));
            }
        }
        if low.is_finite() && high.is_finite() && low < high {
            for shape in egui::Shape::dashed_line(&[a + d * low, a + d * high], stroke, 6., 4.) {
                painter.add(shape);
            }
        }
    }

    fn entity(
        &self,
        painter: &egui::Painter,
        sketch: &Sketch,
        entity: &nator_sketch::Entity,
        color: egui::Color32,
    ) {
        let Ok(curve) = profiles::segment(sketch, entity) else {
            return;
        };
        let stroke = egui::Stroke::new(
            1.5,
            if entity.projected {
                theme::PROJECTED
            } else if entity.construction {
                theme::CONSTRUCTION
            } else {
                color
            },
        );
        if entity.infinite {
            if let profiles::Segment::Line { a, b } = curve {
                self.infinite_line(painter, a, b, stroke);
            }
        } else {
            self.path(painter, curve.sample(96), stroke, entity.construction);
        }
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

/// Suporte exato da face escolhida, nas coordenadas do destino, e o nome com que o
/// painel do plano deslocado a apresenta. Roda no trabalhador: consulta o B-rep.
fn face_plane(
    document: &Document,
    pick: &Pick,
    placement: Placement,
) -> Result<(Plane, String), String> {
    let world = nator_commands::measure::sketch_plane(document, pick).map_err(|e| e.to_string())?;
    let plane = face_work_plane(world, placement).map_err(|e| e.to_string())?;
    let label = match document.body(pick.body) {
        Some(body) => format!("{} ({})", text(T::PickedFace), body.name),
        None => text(T::PickedFace).to_owned(),
    };
    Ok((plane, label))
}

/// Retângulo que representa um plano infinito na área 3D: centrado na projeção do centro
/// da cena sobre o plano, alinhado a U/V e com lado proporcional à cena (mm, no mundo).
pub(super) fn plane_patch(world: Plane, bounds: Bounds) -> [Point3; 4] {
    let center = world.project(bounds.center());
    let half = (bounds.size().into_iter().fold(0_f64, f64::max) * 0.35).max(10.);
    [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]]
        .map(|[a, b]| world.world([center[0] + a * half, center[1] + b * half]))
}

/// Plano translúcido com contorno, em pontos de tela; destacado sob o cursor. Triângulos e
/// arestas independentes evitam junções de traço instáveis com o plano quase de perfil.
pub(super) fn paint_patch(
    painter: &egui::Painter,
    points: &[egui::Pos2],
    color: egui::Color32,
    highlighted: bool,
) {
    let mut mesh = egui::Mesh::default();
    for p in points {
        mesh.colored_vertex(
            *p,
            color.gamma_multiply(if highlighted { 0.45 } else { 0.18 }),
        );
    }
    for i in 1..points.len().saturating_sub(1) {
        let i = i as u32;
        mesh.add_triangle(0, i, i + 1);
    }
    painter.add(egui::Shape::mesh(mesh));
    for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
        painter.line_segment(
            [*a, *b],
            egui::Stroke::new(if highlighted { 2. } else { 1. }, color),
        );
    }
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
        self.current_mut().plane_pick = Some(PlanePicker::new(
            destination,
            placement,
            false,
            PlanePurpose::Sketch,
        ));
        self.status.set(text(T::ChooseSketchPlane));
    }
    /// Seletor de plano para um rascunho de referência da montagem.
    pub(super) fn start_reference_sketch_plane(&mut self) {
        if self.busy() || self.current().sketch_edit.is_some() {
            return;
        }
        // Referências pertencem à montagem: sair da construção deixa o contexto claro.
        self.current_mut().active_part = None;
        self.current_mut().plane_pick = Some(PlanePicker::new(
            None,
            Placement::default(),
            true,
            PlanePurpose::Sketch,
        ));
        self.status.set(text(T::ChooseSketchPlane));
    }
    /// Sai da escolha de suporte. Sem plano-base, o plano deslocado também se encerra:
    /// não há o que deslocar.
    pub(super) fn cancel_plane_pick(&mut self) {
        let purpose = self.current_mut().plane_pick.take().map(|p| p.purpose);
        if purpose == Some(PlanePurpose::OffsetBase) && !self.offset_plane_has_base() {
            self.close_offset_plane();
        }
    }
    pub(super) fn choose_origin_plane(&mut self, name: &str, plane: Plane) {
        if self.busy() {
            return;
        }
        let Some(picker) = self.current().plane_pick.as_ref() else {
            return;
        };
        match picker.purpose {
            PlanePurpose::Sketch => {
                let command = new_sketch(picker.destination, picker.reference, plane);
                self.execute(command);
            }
            PlanePurpose::OffsetBase => {
                self.set_offset_base(plane, format!("{name} · {}", text(T::Origin)));
            }
        }
    }
    /// Plano de trabalho escolhido na área 3D ou na árvore. O rascunho fica na peça dona
    /// do plano; a escolha de outra peça que não a de destino é recusada, porque o plano
    /// está nas coordenadas da sua própria peça.
    pub(super) fn choose_work_plane(&mut self, id: WorkPlaneId) {
        if self.busy() {
            return;
        }
        let Some(picker) = self.current().plane_pick.as_ref() else {
            return;
        };
        let document = self.current().session.document();
        let (Some(part), Some(record)) = (document.work_plane_part(id), document.work_plane(id))
        else {
            return;
        };
        if picker.reference || self.editing_work_plane() == Some(id) {
            return;
        }
        let accepted = match picker.purpose {
            // Sem peça em construção, o rascunho vai para a peça do plano escolhido.
            PlanePurpose::Sketch => picker.destination.is_none_or(|d| d == part),
            PlanePurpose::OffsetBase => picker.destination == Some(part),
        };
        if !accepted {
            self.status.warning(text(T::WorkPlaneOtherPart));
            return;
        }
        match picker.purpose {
            PlanePurpose::Sketch => self.execute(Command::AddSketchOnWorkPlane {
                id,
                name: text(T::Sketch).into(),
            }),
            PlanePurpose::OffsetBase => {
                let (plane, name) = (record.plane.plane(), record.name.clone());
                self.set_offset_base(plane, name);
            }
        }
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
        let (destination, placement, reference, purpose) = (
            picker.destination,
            picker.placement,
            picker.reference,
            picker.purpose,
        );
        let document = self.current().session.document().clone();
        let (tab, revision) = (self.current().id, self.current().session.revision());
        let view = self.current().view.clone();
        let section = self.current().section;
        self.start(context, None, move || {
            let picks = document
                .pick_all_with(ray, |o| view.pickable(o))
                .map_err(|e| e.to_string())?;
            let pick = section::first_visible(picks, section);
            if purpose == PlanePurpose::OffsetBase {
                // Clicar fora de uma face plana só avisa: a escolha continua aberta.
                let plane = pick
                    .ok_or_else(|| text(T::ChooseOffsetBase).to_owned())
                    .and_then(|pick| face_plane(&document, &pick, placement));
                return Ok(Outcome::PlaneChosen {
                    tab,
                    revision,
                    plane,
                });
            }
            let pick = pick.ok_or_else(|| text(T::ChooseSketchPlane).to_owned())?;
            let plane = face_plane(&document, &pick, placement)?.0;
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
        let (destination, placement, reference, purpose) = (
            picker.destination,
            picker.placement,
            picker.reference,
            picker.purpose,
        );
        let document = self.current().session.document().clone();
        let (tab, revision) = (self.current().id, self.current().session.revision());
        self.start(context, None, move || {
            if purpose == PlanePurpose::OffsetBase {
                return Ok(Outcome::PlaneChosen {
                    tab,
                    revision,
                    plane: face_plane(&document, &pick, placement),
                });
            }
            let plane = face_plane(&document, &pick, placement)?.0;
            let command = PreparedCommand::new(
                document,
                revision,
                new_sketch(destination, reference, plane),
            )
            .map_err(|e| e.to_string())?;
            Ok(Outcome::Prepared { tab, command })
        });
    }
    pub(super) fn cancel_sketch(&mut self, context: &egui::Context) {
        if self.busy() {
            return;
        }
        let Some(work) = self.current().sketch_edit.as_ref() else {
            return;
        };
        let id = work.id;
        let original = work.original_sketch.clone();
        let current = self
            .current()
            .session
            .document()
            .sketch(id)
            .map(|record| &record.sketch);
        // Comparação estrutural inclui restrições e expressões, não só entidades.
        // Uma edição sem mudanças fecha sem acrescentar um comando ao histórico.
        let changed = match (original.as_ref(), current) {
            (Some(original), Some(current)) => serde_json::to_value(original)
                .and_then(|before| serde_json::to_value(current).map(|after| before != after)),
            _ => Ok(false),
        };
        let changed = match changed {
            Ok(changed) => changed,
            Err(error) => {
                self.status.error(error.to_string());
                return;
            }
        };
        if !changed {
            if let Some(work) = self.current_mut().sketch_edit.as_mut() {
                work.pending = None;
                work.reset_drawing(context);
            }
            let tab = self.current().id;
            self.pending_failure
                .take_if(|failure| failure.tab == tab && failure.sketch == Some(id));
            self.finish_sketch();
            return;
        }
        let Some(sketch) = original else {
            return;
        };
        let tab = self.current().id;
        let revision = self.current().session.revision();
        let document = self.current().session.document().clone();
        // Restaurar por comando mantém desfazer/refazer e recalcula operações
        // dependentes no trabalhador. Uma falha conserva a edição e o documento.
        self.start(context, None, move || {
            let command =
                PreparedCommand::new(document, revision, Command::UpdateSketch { id, sketch })
                    .map_err(|error| error.to_string())?;
            Ok(Outcome::SketchCanceled { tab, id, command })
        });
    }
    pub(super) fn finish_sketch(&mut self) {
        self.image_edit = None;
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
        let mut limits = sketch_limits(&sketch);
        for point in self
            .current()
            .session
            .document()
            .images()
            .iter()
            .filter(|i| i.sketch == work.id && i.visible)
            .flat_map(image_editor::corners)
        {
            if let Some((min, max)) = &mut limits {
                for axis in 0..2 {
                    min[axis] = min[axis].min(point[axis]);
                    max[axis] = max[axis].max(point[axis]);
                }
            } else {
                limits = Some((point, point));
            }
        }
        if let Some((min, max)) = limits {
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
                for point in record.sketch.visible_points() {
                    if let Some(p) = frame.screen(point.position) {
                        painter.circle_filled(p, 2., theme::ACCENT);
                    }
                }
                for entity in &record.sketch.entities {
                    frame.entity(&painter, &record.sketch, entity, theme::ACCENT);
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
            for point in record.sketch.visible_points() {
                if let Some(p) = frame.screen(point.position) {
                    painter.circle_filled(p, 2.5, theme::REFERENCE);
                }
            }
            for entity in &record.sketch.entities {
                frame.entity(&painter, &record.sketch, entity, theme::REFERENCE);
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
        let world_screen = |point| project(camera, bounds, response.rect, point);
        let screen = |point| world_screen(placement.apply(point));
        let hover = response
            .hover_pos()
            .filter(|p| !view_cube::rect(response.rect).contains(*p));
        let mut selected = None;
        if picking {
            // Profundidade, candidato, rótulo, cor, contorno na tela e posição do rótulo.
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
                    let label = screen(plane.world([size * 0.65, size * 0.65]));
                    planes.push((
                        depth,
                        Candidate::Origin(name, plane),
                        name.to_owned(),
                        color,
                        points,
                        label.map(|p| (p, egui::Align2::CENTER_CENTER)),
                    ));
                }
            }
            // Planos de trabalho visíveis da peça de destino, na mesma ocorrência.
            let destination = self
                .current()
                .plane_pick
                .as_ref()
                .filter(|p| p.offers_work_planes())
                .and_then(|p| p.destination)
                .and_then(|id| document.part(id));
            let editing = self.editing_work_plane();
            for record in destination.iter().flat_map(|part| {
                part.work_planes
                    .iter()
                    .filter(|r| part.shows_work_plane(r) && Some(r.id) != editing)
            }) {
                let corners =
                    plane_patch(transformed_plane(record.plane.plane(), placement), bounds);
                let polygon: Option<Vec<_>> = corners.iter().map(|p| world_screen(*p)).collect();
                if let Some(points) = polygon {
                    let center: Point3 =
                        std::array::from_fn(|i| corners.iter().map(|c| c[i]).sum::<f64>() / 4.);
                    let label = points[3] + (points[1] - points[3]) * 0.06;
                    planes.push((
                        dot(center, camera.direction()),
                        Candidate::Work(record.id),
                        record.name.clone(),
                        theme::WORK_PLANE,
                        points,
                        Some((label, egui::Align2::LEFT_BOTTOM)),
                    ));
                }
            }
            planes.sort_by(|a, b| a.0.total_cmp(&b.0));
            // O último plano desenhado recebe o clique quando há sobreposição;
            // apenas esse mesmo plano recebe o destaque de pré-seleção.
            selected = planes
                .iter()
                .rev()
                .find_map(|(_, candidate, _, _, points, _)| {
                    hover
                        .is_some_and(|p| polygon_contains(points, p))
                        .then_some(*candidate)
                });
            for (_, candidate, name, color, points, label) in planes {
                // Um plano de perfil para a câmera não tem área para receber cliques.
                let center = points.iter().fold(egui::Pos2::ZERO, |a, b| {
                    a + b.to_vec2() / points.len() as f32
                });
                if !polygon_contains(&points, center) {
                    continue;
                }
                paint_patch(&painter, &points, color, selected == Some(candidate));
                if let Some((at, align)) = label {
                    painter.text(at, align, name, egui::FontId::proportional(13.), color);
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
        // Como nas ferramentas de desenho, o ícone da ferramenta acompanha o cursor sobre
        // a área 3D, sem substituir o ponteiro nem cobrir a ponta usada no clique.
        if let (Some(picker), Some(position)) = (self.current().plane_pick.as_ref(), hover) {
            let icon = match picker.purpose {
                PlanePurpose::Sketch if picker.reference => icons::Icon::ReferenceSketch,
                PlanePurpose::Sketch => icons::Icon::Sketch,
                PlanePurpose::OffsetBase => icons::Icon::OffsetPlane,
            };
            icons::paint(
                &painter,
                egui::Rect::from_min_size(position + egui::vec2(16., -28.), egui::Vec2::splat(22.)),
                icon,
                true,
            );
        }
        if let Some(picker) = self.current_mut().plane_pick.as_mut() {
            picker.origin_hovered = selected.is_some();
        }
        if selected.is_some() && !self.busy() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if picking
            && !self.busy()
            && response.clicked()
            && let Some(candidate) = selected
        {
            match candidate {
                Candidate::Origin(name, plane) => self.choose_origin_plane(name, plane),
                Candidate::Work(id) => self.choose_work_plane(id),
            }
            return true;
        }
        false
    }
    /// Planos de trabalho visíveis em todas as ocorrências isoladas das suas peças. Durante
    /// a escolha de suporte, os da peça de destino ficam com o seletor, que os torna
    /// clicáveis; desenhá-los aqui também duplicaria o preenchimento.
    pub(super) fn work_planes_overlay(&self, ui: &egui::Ui, rect: egui::Rect) -> bool {
        let document = self.current().session.document();
        let Ok(occurrences) = document.occurrences() else {
            return false;
        };
        let picked = self
            .current()
            .plane_pick
            .as_ref()
            .filter(|p| p.offers_work_planes())
            .map(|p| (p.destination, p.placement));
        let bounds = self.display_bounds();
        let camera = &self.viewport.camera;
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        // O plano em edição aparece só na prévia do painel, na posição nova.
        let editing = self.editing_work_plane();
        let mut drawn = false;
        for occurrence in occurrences
            .iter()
            .filter(|o| self.current().view.in_isolation(o))
        {
            let Some(part) = document
                .instance(occurrence.instance)
                .and_then(|i| document.part(i.part))
            else {
                continue;
            };
            if picked == Some((Some(part.id), occurrence.placement)) {
                continue;
            }
            for record in part
                .work_planes
                .iter()
                .filter(|r| part.shows_work_plane(r) && Some(r.id) != editing)
            {
                let corners = plane_patch(
                    transformed_plane(record.plane.plane(), occurrence.placement),
                    bounds,
                );
                let Some(points) = corners
                    .iter()
                    .map(|p| project(camera, bounds, rect, *p))
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                paint_patch(&painter, &points, theme::WORK_PLANE, false);
                painter.text(
                    points[3] + (points[1] - points[3]) * 0.06,
                    egui::Align2::LEFT_BOTTOM,
                    &record.name,
                    egui::FontId::proportional(12.),
                    theme::WORK_PLANE,
                );
                drawn = true;
            }
        }
        drawn
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
                    for point in document
                        .images()
                        .iter()
                        .filter(|i| i.sketch == record.id && i.visible)
                        .flat_map(image_editor::corners)
                    {
                        include(occurrence.placement.apply(sketch.plane.world(point)));
                    }
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
        for point in document
            .images()
            .iter()
            .filter(|i| i.sketch == record.id && i.visible)
            .flat_map(image_editor::corners)
        {
            include(sketch.plane.world(point));
        }
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
