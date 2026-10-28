//! Edição 2D no plano, com prévias locais e publicação por comandos da sessão.
use super::*;
use nator_sketch::drafting::{self, Primitive};
use nator_sketch::{
    ConstraintKind, Curve, LineType, Point2, Sketch, profiles,
    snapping::{self, Kind, Tracker},
    solver::{self, Status},
};
mod actions;
mod dimension_tool;
mod dynamic_input;
mod indicators;
mod junction_tool;
mod line_tools;
mod project_tool;
mod projection;
mod references;
pub(super) use actions::{edit_icon, edit_label};
use nator_sketch::edit::Action as EditAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Tool {
    Select,
    Line,
    Rectangle,
    Circle,
    Arc,
    Trim,
    Project,
    /// Cota no modelo da Dimension do SolidCraft (`dimension_tool`).
    Dimension,
}
impl Tool {
    pub(super) fn label(self) -> T {
        match self {
            Self::Select => T::Select,
            Self::Line => T::Line,
            Self::Rectangle => T::Rectangle,
            Self::Circle => T::Circle,
            Self::Arc => T::Arc,
            Self::Trim => T::Trim,
            Self::Project => T::SketchProject,
            Self::Dimension => T::SketchDimension,
        }
    }
    /// Chave estável de memória, compartilhada com os campos flutuantes para
    /// evitar que se posicionem sobre o painel de opções.
    pub(super) fn panel_key(self) -> &'static str {
        match self {
            Self::Select => "sketch_select",
            Self::Line => "sketch_line",
            Self::Rectangle => "sketch_rectangle",
            Self::Circle => "sketch_circle",
            Self::Arc => "sketch_arc",
            Self::Trim => "sketch_trim",
            Self::Project => "sketch_project",
            Self::Dimension => "sketch_dimension",
        }
    }
    pub(super) fn icon(self) -> icons::Icon {
        match self {
            Self::Select => icons::Icon::Select,
            Self::Line => icons::Icon::Line,
            Self::Rectangle => icons::Icon::Rectangle,
            Self::Circle => icons::Icon::Circle,
            Self::Arc => icons::Icon::Arc,
            Self::Trim => icons::Icon::Trim,
            Self::Project => icons::Icon::SketchProject,
            Self::Dimension => icons::Icon::Dimension,
        }
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum LineMode {
    #[default]
    Normal,
    Symmetric,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum RectangleMode {
    #[default]
    Normal,
    Center,
    Inclined,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum CircleMode {
    #[default]
    Normal,
    Diameter,
    Tangent,
}
pub(super) struct Workspace {
    pub id: SketchId,
    pub pending: Option<Sketch>,
    /// Estado anterior à edição: Cancelar restaura apenas este rascunho, sem
    /// desfazer alterações independentes feitas no restante do documento.
    pub(super) original_sketch: Option<Sketch>,
    pub status: Status,
    pub redundant: bool,
    tool: Tool,
    /// Entrar no rascunho não abre painel; somente uma escolha explícita na faixa.
    tool_panel_open: bool,
    pub(super) line_type: LineType,
    line_mode: LineMode,
    rectangle_mode: RectangleMode,
    circle_mode: CircleMode,
    tangent_lines: Vec<u64>,
    /// Ponto inicial e vértices já percorridos da cadeia, para encerrar ao fechar.
    chain_points: Vec<Point2>,
    /// Capturada uma vez no primeiro clique; não muda ao cruzar outra linha.
    line_reference: Option<u64>,
    perpendicular: Option<u64>,
    anchors: Vec<Point2>,
    anchor_kinds: Vec<Option<Kind>>,
    /// Decisão capturada em cada clique; F afeta somente o próximo apoio.
    anchor_strengths: Vec<bool>,
    junction_override: Option<bool>,
    selected_vertex: Option<Point2>,
    selected: Vec<u64>,
    constraint: Option<u64>,
    pub(super) placement: Placement,
    pub(super) previous_camera: Camera,
    pub(super) fit_requested: bool,
    tracker: Tracker,
    projection: projection::Cache,
    projector: project_tool::Cache,
    dimension: f64,
    dimension_expression: String,
    reference: bool,
    /// Ocorrências dos padrões do rascunho (o espaçamento usa `dimension`).
    pattern_count: usize,
    /// Intenção aberta pela faixa; os cliques alternam a seleção até OK/Cancelar.
    active_action: Option<EditAction>,
    operation_line_type: LineType,
    mirror_axis: Option<u64>,
    mirror_axis_active: bool,
    offset_mode: nator_sketch::modifications::OffsetMode,
    offset_second_expression: String,
    drawing: dynamic_input::Input,
    references: references::Cache,
    /// Escolhas, texto e caixa de valor da ferramenta Cota.
    dimension_draft: dimension_tool::Draft,
    dimension_existing: dimension_tool::Existing,
}
impl Workspace {
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn select_vertex_for_capture(&mut self, position: Point2) {
        self.selected_vertex = Some(position);
    }
    fn anchor_strong(&self, index: usize) -> bool {
        self.anchor_strengths
            .get(index)
            .copied()
            .unwrap_or_else(|| {
                nator_sketch::junctions::default_strong(
                    self.anchor_kinds.get(index).copied().flatten(),
                )
            })
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn projection_ready(&self) -> bool {
        self.projector.ready()
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn projected_region_count(&self) -> usize {
        self.references.projected_regions.len()
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn projected_points(&self) -> Option<&[Point2]> {
        self.projection
            .ready()
            .then_some(self.projection.points.as_slice())
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn reference_points(&self) -> &nator_sketch::topology::References {
        &self.references.points
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn drawing_fields(&self) -> [(String, bool); 2] {
        std::array::from_fn(|i| (self.drawing.fields[i].clone(), self.drawing.locked[i]))
    }
    pub(super) fn is_drawing(&self) -> bool {
        !self.anchors.is_empty() && dynamic_input::primitive(self.tool).is_some()
    }
    pub(super) fn active_action(&self) -> Option<EditAction> {
        self.active_action
    }
    pub(super) fn tool(&self) -> Tool {
        self.tool
    }
    /// Caixa de valor da Cota aberta e quantidade de escolhas, para os ensaios.
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn dimension_state(&self) -> (bool, usize) {
        (self.dimension_draft.editing(), self.dimension_draft.picks())
    }
    pub(super) fn is_selecting(&self) -> bool {
        self.tool == Tool::Select && self.active_action.is_none()
    }
    pub(super) fn activate_selection(&mut self, context: &egui::Context) {
        self.reset_drawing(context);
        self.select_tool(Tool::Select);
        self.tool_panel_open = false;
        self.pending = None;
        self.tracker.clear();
        context.memory_mut(|memory| {
            if let Some(id) = memory.focused() {
                memory.surrender_focus(id);
            }
        });
    }
    pub(super) fn select_tool(&mut self, tool: Tool) {
        self.active_action = None;
        self.mirror_axis = None;
        self.mirror_axis_active = false;
        self.tool = tool;
        self.tool_panel_open = true;
        self.anchors.clear();
        self.anchor_kinds.clear();
        self.anchor_strengths.clear();
        self.junction_override = None;
        self.selected_vertex = None;
        self.line_reference = None;
        self.perpendicular = None;
        self.chain_points.clear();
        self.tangent_lines.clear();
        if !matches!(tool, Tool::Line | Tool::Rectangle) && self.line_type == LineType::Infinite {
            self.line_type = LineType::Normal;
        }
        self.drawing = dynamic_input::Input::default();
        self.dimension_draft.clear();
        self.dimension_existing = dimension_tool::Existing::default();
    }
    fn primitive(&self) -> Option<Primitive> {
        Some(match self.tool {
            Tool::Line if self.line_mode == LineMode::Symmetric => Primitive::SymmetricLine,
            Tool::Rectangle if self.rectangle_mode == RectangleMode::Center => {
                Primitive::CenterRectangle
            }
            Tool::Rectangle if self.rectangle_mode == RectangleMode::Inclined => {
                Primitive::InclinedRectangle
            }
            Tool::Circle if self.circle_mode == CircleMode::Diameter => Primitive::DiameterCircle,
            Tool::Circle if self.circle_mode == CircleMode::Tangent => Primitive::TangentCircle,
            _ => return dynamic_input::primitive(self.tool),
        })
    }
    pub(super) fn reset_drawing(&mut self, context: &egui::Context) {
        self.anchors.clear();
        self.anchor_kinds.clear();
        self.anchor_strengths.clear();
        self.junction_override = None;
        self.chain_points.clear();
        self.tangent_lines.clear();
        self.line_reference = None;
        self.perpendicular = None;
        self.drawing.clear(context);
        self.dimension_draft.clear();
    }
    /// Cada segmento confirmado conserva o último ponto e captura o próximo
    /// ângulo a partir dele. Medidas digitadas pertencem somente ao segmento atual.
    fn continue_line(&mut self, sketch: &Sketch, context: &egui::Context, end: Point2) {
        let closed =
            self.line_type != LineType::Infinite && drafting::closes_chain(&self.chain_points, end);
        if closed {
            self.reset_drawing(context);
            return;
        }
        self.chain_points.push(end);
        let last = sketch.entities.iter().rev().find(|entity| {
            matches!(entity.curve, Curve::Line { .. })
                && entity.point_ids().into_iter().any(|id| {
                    sketch
                        .point(id)
                        .is_ok_and(|p| nator_sketch::distance(p, end) < 1e-7)
                })
        });
        let endpoint = last.and_then(|entity| {
            entity.point_ids().into_iter().find(|id| {
                sketch
                    .point(*id)
                    .is_ok_and(|p| nator_sketch::distance(p, end) < 1e-7)
            })
        });
        self.anchors = vec![end];
        self.anchor_kinds = vec![endpoint.map(Kind::Endpoint)];
        self.anchor_strengths = vec![true];
        self.drawing.clear(context);
        self.drawing.start(end);
        self.line_reference = last.map(|entity| entity.id);
        self.drawing.relative = self.line_reference.is_some();
        self.drawing.reference_angle = self
            .line_reference
            .and_then(|id| nator_sketch::lines::angle(sketch, id).ok())
            .unwrap_or(0.);
        self.perpendicular = None;
    }
    pub fn new(id: SketchId) -> Self {
        Self {
            id,
            pending: None,
            original_sketch: None,
            status: Status::UnderConstrained {
                degrees_of_freedom: 0,
            },
            redundant: false,
            tool: Tool::Select,
            tool_panel_open: false,
            line_type: LineType::Normal,
            line_mode: LineMode::Normal,
            rectangle_mode: RectangleMode::Normal,
            circle_mode: CircleMode::Normal,
            tangent_lines: Vec::new(),
            chain_points: Vec::new(),
            line_reference: None,
            perpendicular: None,
            anchors: Vec::new(),
            anchor_kinds: Vec::new(),
            anchor_strengths: Vec::new(),
            junction_override: None,
            selected_vertex: None,
            selected: Vec::new(),
            constraint: None,
            placement: Placement::default(),
            previous_camera: Camera::default(),
            fit_requested: true,
            tracker: Tracker::default(),
            projection: projection::Cache::default(),
            projector: project_tool::Cache::default(),
            dimension: 10.,
            dimension_expression: "10 mm".into(),
            pattern_count: 3,
            reference: false,
            active_action: None,
            operation_line_type: LineType::Normal,
            mirror_axis: None,
            mirror_axis_active: false,
            offset_mode: nator_sketch::modifications::OffsetMode::Normal,
            offset_second_expression: "10 mm".into(),
            drawing: dynamic_input::Input::default(),
            references: references::Cache::default(),
            dimension_draft: dimension_tool::Draft::default(),
            dimension_existing: dimension_tool::Existing::default(),
        }
    }
}
fn sketch_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
fn attach(
    sketch: &mut Sketch,
    point: u64,
    kind: Option<Kind>,
    position: Point2,
    strong: bool,
) -> Result<(), String> {
    nator_sketch::junctions::attach(sketch, point, kind, position, strong).map_err(sketch_error)
}
impl Editor {
    pub(super) fn begin_sketch(&mut self, part: PartId, id: SketchId) {
        self.begin_sketch_in(Some(part), id);
    }
    /// Edição de um rascunho de referência da montagem, em coordenadas da montagem.
    pub(super) fn begin_reference_sketch(&mut self, id: SketchId) {
        self.begin_sketch_in(None, id);
    }
    fn begin_sketch_in(&mut self, owner: Option<PartId>, id: SketchId) {
        if self.busy() {
            return;
        }
        if self
            .current()
            .sketch_edit
            .as_ref()
            .is_some_and(|w| w.pending.is_some())
        {
            self.status.warning(text(T::PendingEdit));
            return;
        }
        let Some(record) = self.current().session.document().sketch(id).cloned() else {
            return;
        };
        let target = match owner {
            None => Ok(Placement::default()),
            Some(part) => self
                .current()
                .plane_pick
                .as_ref()
                .filter(|p| p.destination == Some(part))
                .map(|p| Ok(p.placement))
                .unwrap_or_else(|| sketch_scene::part_placement(self.current(), part)),
        };
        let target = match target {
            Ok(p) => p,
            Err(e) => {
                self.status.error(e);
                return;
            }
        };
        if !record.visible {
            self.execute(Command::SetSketchVisibility { id, visible: true });
        }
        let previous = self
            .current()
            .sketch_edit
            .as_ref()
            .map(|w| w.previous_camera.clone())
            .unwrap_or_else(|| self.viewport.camera.clone());
        let original = self
            .current()
            .sketch_edit
            .as_ref()
            .filter(|work| work.id == id)
            .and_then(|work| work.original_sketch.clone())
            .unwrap_or_else(|| record.sketch.clone());
        let mut work = Workspace::new(id);
        work.original_sketch = Some(original);
        work.placement = target;
        work.previous_camera = previous;
        work.fit_requested = false;
        // Rascunhos de referência pertencem à montagem: nenhuma peça entra em construção.
        self.current_mut().active_part = owner;
        self.current_mut().plane_pick = None;
        self.current_mut().sketch_edit = Some(work);
        self.align_sketch_camera(!record.sketch.points.is_empty(), true);
    }
    /// Ferramenta escolhida na faixa, na busca ou pelo atalho D (Cota).
    pub(super) fn select_sketch_tool(&mut self, tool: Tool) {
        let sketch = (tool == Tool::Dimension)
            .then(|| self.working_sketch())
            .flatten();
        if let Some(work) = &mut self.current_mut().sketch_edit {
            work.select_tool(tool);
            if tool == Tool::Dimension {
                if let Some(sketch) = &sketch {
                    work.dimension_draft.seed(sketch, &work.selected);
                }
                self.status.set(text(T::DimensionPickHint));
            }
            if tool == Tool::Project {
                self.status.set(text(T::SketchProjectHint));
            }
        }
    }
    pub(super) fn solve_sketch(&mut self, context: &egui::Context, sketch: Sketch) {
        self.solve_sketch_with(context, sketch, None);
    }
    /// `dimension` identifica a cota recém-criada pela ferramenta Cota: antes de resolver,
    /// o trabalhador decide se ela dirige a geometria ou entra como referência.
    fn solve_sketch_with(
        &mut self,
        context: &egui::Context,
        sketch: Sketch,
        dimension: Option<(u64, bool)>,
    ) {
        if self.busy() {
            return;
        }
        let Some(work) = &self.current().sketch_edit else {
            return;
        };
        let id = work.id;
        let check_area =
            work.tool == Tool::Line && work.line_type == LineType::Normal && work.is_drawing();
        let tab = self.current().id;
        let revision = self.current().session.revision();
        let document = self.current().session.document().clone();
        let parameters = self
            .current()
            .session
            .document()
            .parts()
            .iter()
            .find(|p| p.sketches.iter().any(|s| s.id == id))
            .map(|p| p.parameters.clone())
            .unwrap_or_default();
        self.start(context, None, move || {
            let mut sketch = sketch;
            if let Some((id, true)) = dimension {
                dimension_tool::settle_driven(&mut sketch, id, &parameters)?;
            }
            nator_document::parameters::resolve_sketch(&mut sketch, &parameters)
                .map_err(sketch_error)?;
            let report = match dimension {
                Some((id, _)) => nator_sketch::dimensions::solve(&sketch, id),
                None => solver::solve(&sketch),
            }
            .map_err(sketch_error)?;
            // O fechamento com linhas já existentes só é conhecido após resolver
            // a geometria. Esta consulta fica fora da thread da interface.
            let chain_closed = check_area
                && report.status != Status::Conflict
                && document.sketch(id).is_some_and(|previous| {
                    drafting::creates_area(&previous.sketch, &report.sketch).unwrap_or(false)
                });
            let prepared = if report.status == Status::Conflict {
                Err(nator_commands::CommandError {
                    message: text(T::SketchConflict).into(),
                    failure: None,
                })
            } else {
                PreparedCommand::new(
                    document,
                    revision,
                    Command::UpdateSketch {
                        id,
                        sketch: report.sketch.clone(),
                    },
                )
            };
            Ok(Outcome::SketchSolved {
                tab,
                revision,
                id,
                report,
                prepared,
                chain_closed,
            })
        });
    }
    pub(super) fn working_sketch(&self) -> Option<Sketch> {
        let work = self.current().sketch_edit.as_ref()?;
        work.pending.clone().or_else(|| {
            self.current()
                .session
                .document()
                .sketch(work.id)
                .map(|r| r.sketch.clone())
        })
    }
    /// Painel da ferramenta escolhida. O rascunho nasce sem janela: cada escolha
    /// explícita abre seu painel, e operações com formulário usam `sketch_action_panel`.
    pub(super) fn sketch_tools_window(&mut self, context: &egui::Context) {
        let Some(work) = self.current().sketch_edit.as_ref() else {
            return;
        };
        // A Cota trabalha no próprio canvas, com a caixa de valor junto ao texto.
        if !work.tool_panel_open || work.active_action.is_some() || work.tool == Tool::Dimension {
            return;
        }
        let tool = work.tool;
        let Some(mut sketch) = self.working_sketch() else {
            return;
        };
        let busy = self.busy();
        let mut submit = false;
        let mut open_repair = false;
        let mut edit_dimension = false;
        let outcome = command_panel::options(
            context,
            command_panel::Header {
                id: tool.panel_key(),
                icon: tool.icon(),
                title: text(tool.label()),
            },
            self.canvas_rect,
            command_panel::WIDTH,
            |ui| {
                if tool != Tool::Select {
                    if let Some(work) = self.current_mut().sketch_edit.as_mut() {
                        ui.add_enabled_ui(!busy, |ui| line_tools::settings(ui, work));
                    }
                    return;
                }
                ui.label(&sketch.name);
                let Some(work) = self.current_mut().sketch_edit.as_mut() else {
                    return;
                };
                let (label, color) = match work.status {
                    Status::Conflict => (text(T::SketchConflict), theme::DANGER),
                    Status::FullyConstrained => (text(T::FullyConstrained), theme::GREEN),
                    Status::UnderConstrained { .. } => (text(T::UnderConstrained), theme::ACCENT),
                };
                ui.colored_label(color, label);
                if let Status::UnderConstrained { degrees_of_freedom } = work.status {
                    ui.label(format!(
                        "{}: {degrees_of_freedom}",
                        text(T::DegreesOfFreedom)
                    ));
                }
                if work.redundant {
                    ui.colored_label(theme::WARNING, text(T::Redundant));
                }
                if work.pending.is_some() && ui.button(text(T::Cancel)).clicked() {
                    work.pending = None;
                    work.anchors.clear();
                    work.status = Status::UnderConstrained {
                        degrees_of_freedom: 0,
                    };
                }
                if ui
                    .add_enabled(!busy, egui::Button::new(text(T::RepairSketch)))
                    .on_hover_text(text(T::RepairHint))
                    .clicked()
                {
                    open_repair = true;
                }
                ui.separator();
                ui.label(text(T::Constraints));
                let mut remove = None;
                for constraint in &sketch.constraints {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                work.constraint == Some(constraint.id),
                                if constraint.reference && !constraint.kind.is_dimension() {
                                    format!(
                                        "{} · {}",
                                        constraint_label(&constraint.kind),
                                        text(T::JunctionWeak)
                                    )
                                } else {
                                    constraint_label(&constraint.kind)
                                },
                            )
                            .clicked()
                        {
                            work.constraint = Some(constraint.id);
                            if let Some(value) = dimension_value(&constraint.kind) {
                                work.dimension = value;
                                work.dimension_expression = sketch
                                    .dimension_expressions
                                    .get(&constraint.id)
                                    .cloned()
                                    .unwrap_or_else(|| {
                                        dimension_expression(&constraint.kind, value)
                                    });
                                work.reference = constraint.reference;
                            }
                        }
                        if ui
                            .add_enabled(!busy, egui::Button::new("×").small())
                            .clicked()
                        {
                            remove = Some(constraint.id);
                        }
                    });
                }
                if let Some(id) = remove {
                    sketch.constraints.retain(|c| c.id != id);
                    sketch.dimension_expressions.remove(&id);
                    sketch.dimension_labels.remove(&id);
                    sketch.dimension_targets.remove(&id);
                    work.constraint = None;
                    submit = true;
                }
                if work.constraint.is_some_and(|id| {
                    sketch
                        .constraints
                        .iter()
                        .any(|c| c.id == id && dimension_value(&c.kind).is_some())
                }) && ui
                    .add_enabled(!busy, egui::Button::new(text(T::EditDimension)))
                    .clicked()
                {
                    edit_dimension = true;
                }
            },
        );
        if outcome.cancel
            && let Some(work) = self.current_mut().sketch_edit.as_mut()
        {
            work.drawing.clear(context);
            work.tracker.clear();
            work.select_tool(Tool::Select);
            work.tool_panel_open = false;
        }
        if submit {
            self.solve_sketch(context, sketch);
        }
        if open_repair {
            self.open_sketch_repair();
        }
        if edit_dimension {
            self.open_sketch_action(context, EditAction::Dimension);
        }
    }
    pub(super) fn sketch_overlay(&mut self, ui: &egui::Ui, response: &egui::Response) {
        let Some(mut sketch) = self.working_sketch() else {
            return;
        };
        let Some(work) = self.current().sketch_edit.as_ref() else {
            return;
        };
        let placement = work.placement;
        let projection_key = projection::Key {
            scene: self.scene_key(),
            sketch: work.id,
            plane: sketch.plane,
            placement,
        };
        let frame = sketch_scene::Frame {
            camera: self.viewport.camera.clone(),
            bounds: self.display_bounds(),
            rect: response.rect,
            placement,
            plane: sketch.plane,
        };
        let painter = ui
            .painter()
            .with_clip_rect(response.rect.intersect(ui.clip_rect()));
        let animating = self.current().camera_transition.is_some();
        let hover = if animating {
            None
        } else {
            response
                .hover_pos()
                .filter(|p| !view_cube::rect(response.rect).contains(*p))
                .and_then(|p| frame.cursor(p))
        };
        let pixels_per_mm = frame.pixels_per_mm(hover.unwrap_or([0., 0.]));
        let busy = self.busy();
        let parameters = self
            .current()
            .session
            .document()
            .sketch_part(work.id)
            .and_then(|id| self.current().session.document().part(id))
            .map(|p| p.parameters.clone())
            .unwrap_or_default();
        let mut submit = false;
        let projection_error = {
            let tab = self.current_mut();
            tab.sketch_edit.as_mut().and_then(|work| {
                work.projection
                    .prepare(projection_key, tab.session.document(), &tab.view, ui.ctx())
                    .err()
            })
        };
        if let Some(error) = projection_error {
            self.status.error(error);
        }
        let project_error = {
            let tab = self.current_mut();
            tab.sketch_edit
                .as_mut()
                .filter(|work| work.tool == Tool::Project)
                .and_then(|work| {
                    work.projector
                        .prepare(projection_key, tab.session.document(), &tab.view, ui.ctx())
                        .err()
                })
        };
        if let Some(error) = project_error {
            self.status.error(error);
        }
        if let Some(work) = self.current_mut().sketch_edit.as_mut()
            && let Err(error) = work.references.prepare(&sketch, ui.ctx())
        {
            self.status.error(error);
        }
        let Some(work) = self.current_mut().sketch_edit.as_mut() else {
            return;
        };
        // Grade no próprio plano: o mesmo conjunto de linhas acompanha a câmera,
        // em vez de substituir a área CAD por uma superfície plana independente.
        let center = frame.cursor(response.rect.center()).unwrap_or([0., 0.]);
        let step = 10_f64.powf((60. / pixels_per_mm).log10().round());
        let radius = f64::from(response.rect.width().max(response.rect.height())) / pixels_per_mm;
        for axis in 0..2 {
            for i in -15..=15 {
                let coordinate = (center[axis] / step).round() * step + f64::from(i) * step;
                let (a, b) = if axis == 0 {
                    (
                        [coordinate, center[1] - radius],
                        [coordinate, center[1] + radius],
                    )
                } else {
                    (
                        [center[0] - radius, coordinate],
                        [center[0] + radius, coordinate],
                    )
                };
                frame.path(
                    &painter,
                    [a, b],
                    egui::Stroke::new(
                        0.5,
                        egui::Color32::from_rgba_unmultiplied(130, 150, 175, 70),
                    ),
                    false,
                );
            }
        }
        project_tool::paint_regions(&frame, &painter, &work.references.projected_regions);
        let base = match work.status {
            Status::FullyConstrained => theme::GREEN,
            Status::Conflict => theme::DANGER,
            _ => theme::ACCENT,
        };
        for point in sketch.visible_points() {
            if let Some(p) = frame.screen(point.position) {
                painter.circle_filled(
                    p,
                    2.5,
                    if point.projected {
                        theme::PROJECTED
                    } else {
                        base
                    },
                );
            }
        }
        // Encontros são vértices virtuais enquanto as curvas estão inteiras. Assim
        // aparecem e atraem o desenho antes de o aparador materializar os extremos.
        for point in &work.references.points.vertices {
            if let Some(p) = frame.screen(*point) {
                painter.circle_filled(p, 2.5, base);
            }
        }
        for entity in &sketch.entities {
            if let Ok(segment) = profiles::segment(&sketch, entity) {
                let color = if work.selected.contains(&entity.id) {
                    egui::Color32::from_rgb(183, 106, 17)
                } else if entity.projected {
                    theme::PROJECTED
                } else if entity.construction {
                    theme::CONSTRUCTION
                } else {
                    base
                };
                if entity.infinite {
                    if let profiles::Segment::Line { a, b } = segment {
                        frame.infinite_line(&painter, a, b, egui::Stroke::new(1.5, color));
                    }
                } else {
                    frame.path(
                        &painter,
                        segment.sample(128),
                        egui::Stroke::new(1.5, color),
                        entity.construction,
                    );
                }
                for id in entity.point_ids().into_iter().filter(|_| !entity.infinite) {
                    if let Some(p) = sketch.point(id).ok().and_then(|p| frame.screen(p)) {
                        painter.circle_filled(p, 2.5, color);
                    }
                }
            }
        }
        // Cotas como cotas: extensões, linha com setas e valor (R/Ø, graus, parênteses).
        let edit_existing = !busy
            && !animating
            && (work.is_selecting()
                || (work.tool == Tool::Dimension && work.dimension_draft.is_empty()));
        let existing_update = dimension_tool::interact_existing(
            ui,
            response,
            &frame,
            &painter,
            &mut sketch,
            &mut work.dimension_existing,
            edit_existing,
            &parameters,
        );
        if existing_update.blocked
            && (response.clicked_by(egui::PointerButton::Primary)
                || response.drag_started_by(egui::PointerButton::Primary))
        {
            work.selected.clear();
            work.selected_vertex = None;
            work.constraint = work.dimension_existing.selected();
            if let Some(constraint) = sketch
                .constraints
                .iter()
                .find(|c| Some(c.id) == work.constraint)
                && let Some(value) = constraint.kind.dimension_value()
            {
                work.dimension = value;
                work.dimension_expression = sketch
                    .dimension_expressions
                    .get(&constraint.id)
                    .cloned()
                    .unwrap_or_else(|| dimension_expression(&constraint.kind, value));
                work.reference = constraint.reference;
            }
        }
        dimension_tool::paint_all(
            &frame,
            &painter,
            &sketch,
            work.dimension_existing.selected(),
        );
        junction_tool::paint(&frame, &painter, &work.references.junctions);
        indicators::paint(ui, &frame, &painter, &work.references.indicators);
        let junction_update = if work.is_selecting() {
            junction_tool::button(
                ui,
                &frame,
                &painter,
                &mut sketch,
                work.selected_vertex,
                !busy && !animating,
            )
        } else {
            junction_tool::Update::default()
        };
        submit |= junction_update.changed;

        let suspended = ui.input(|i| i.modifiers.alt);
        let mut snap = hover.and_then(|point| {
            if suspended {
                None
            } else {
                snapping::find_with_references(
                    &sketch,
                    point,
                    8. / pixels_per_mm,
                    &work.tracker,
                    &work.projection.points,
                    &work.references.points,
                )
                .ok()
                .flatten()
            }
        });
        if hover.is_some() || suspended || work.drawing.locked[1] || work.anchors.is_empty() {
            work.perpendicular = None;
        }
        if work.tool == Tool::Line
            && !suspended
            && !work.drawing.locked[1]
            && let (Some(start), Some(point)) = (
                work.anchors.first().copied(),
                snap.as_ref().map(|s| s.point).or(hover),
            )
            && let Ok(Some(perpendicular)) = nator_sketch::lines::perpendicular(
                &sketch,
                start,
                point,
                8. / pixels_per_mm,
                work.line_reference,
            )
        {
            work.perpendicular = Some(perpendicular.line);
            snap = Some(snapping::Snap {
                point: perpendicular.point,
                kind: if perpendicular.on_line {
                    Kind::OnCurve(perpendicular.line)
                } else {
                    Kind::Alignment
                },
                guides: vec![(start, perpendicular.foot)],
            });
        }
        if work.primitive().is_some()
            && hover.is_some()
            && !busy
            && !animating
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F))
        {
            let current = work.junction_override.unwrap_or_else(|| {
                nator_sketch::junctions::default_strong(snap.as_ref().map(|s| s.kind))
            });
            work.junction_override = Some(!current);
            // F é uma ação de desenho, mesmo com o campo numérico ativo. Seu
            // evento de texto não deve virar parte da expressão da medida.
            ui.input_mut(|i| {
                i.events.retain(
                    |event| !matches!(event, egui::Event::Text(s) if s.eq_ignore_ascii_case("f")),
                )
            });
        }
        if let Some(snap) = &snap {
            if matches!(
                snap.kind,
                Kind::Endpoint(_)
                    | Kind::Center(_)
                    | Kind::Midpoint(_)
                    | Kind::CurveMidpoint(_)
                    | Kind::RegionCenter
                    | Kind::Intersection
                    | Kind::Projected
            ) {
                work.tracker.capture(snap.point);
            }
            if let Some(position) = frame.screen(snap.point) {
                painter.circle_stroke(position, 5., egui::Stroke::new(1.5, theme::WARNING));
                if work.primitive().is_some() {
                    let strong = work.junction_override.unwrap_or_else(|| {
                        nator_sketch::junctions::default_strong(Some(snap.kind))
                    });
                    icons::paint(
                        &painter,
                        egui::Rect::from_min_size(
                            position + egui::vec2(-30., -30.),
                            egui::Vec2::splat(20.),
                        ),
                        if strong {
                            icons::Icon::LockClosed
                        } else {
                            icons::Icon::LockOpen
                        },
                        true,
                    );
                    painter.text(
                        position + egui::vec2(12., 12.),
                        egui::Align2::LEFT_TOP,
                        text(if strong {
                            T::JunctionStrong
                        } else {
                            T::JunctionWeak
                        }),
                        egui::FontId::proportional(11.),
                        theme::INK,
                    );
                }
            }
            for (a, b) in &snap.guides {
                frame.path(
                    &painter,
                    [*a, *b],
                    egui::Stroke::new(1., theme::GREEN),
                    true,
                );
            }
        }
        let mut preview_indicators = Vec::new();
        if work.primitive().is_some() {
            use nator_sketch::indicators::{Indicator, Kind as IndicatorKind};
            if let Some(snap) = &snap {
                let kind = match snap.kind {
                    Kind::Midpoint(_) | Kind::CurveMidpoint(_) => Some(IndicatorKind::Midpoint),
                    Kind::Center(_) | Kind::RegionCenter => Some(IndicatorKind::Center),
                    _ => None,
                };
                if let Some(kind) = kind {
                    preview_indicators.push(Indicator {
                        point: snap.point,
                        kind,
                    });
                }
            }
            if let Some(line) = work.perpendicular
                && let Some(start) = work.anchors.first()
                && let Ok((a, b)) = sketch.line(line)
            {
                let d = nator_sketch::sub(b, a);
                let foot = nator_sketch::add(
                    a,
                    nator_sketch::scale(
                        d,
                        nator_sketch::dot(nator_sketch::sub(*start, a), d)
                            / nator_sketch::dot(d, d),
                    ),
                );
                preview_indicators.push(Indicator {
                    point: foot,
                    kind: IndicatorKind::Perpendicular,
                });
            }
        }
        preview_indicators.retain(|i| {
            !work.references.indicators.iter().any(|saved| {
                saved.kind == i.kind && nator_sketch::distance(saved.point, i.point) < 1e-6
            })
        });
        indicators::paint(ui, &frame, &painter, &preview_indicators);
        // Selecionar/aparar usa o clique real: a atração ao centro ou ao vértice
        // vizinho não pode deslocar o clique para outro trecho do cruzamento.
        let tangent_selection = work.tool == Tool::Circle
            && work.circle_mode == CircleMode::Tangent
            && work.tangent_lines.len() < 2;
        let raw_cursor = if matches!(work.tool, Tool::Select | Tool::Trim) || tangent_selection {
            hover
        } else {
            snap.as_ref().map(|s| s.point).or(hover)
        };
        if work.tool == Tool::Trim
            && let Some(cursor) = hover
            && let Ok(Some(id)) =
                nator_sketch::tools::pick_entity(&sketch, cursor, 8. / pixels_per_mm)
            && let Ok(piece) = nator_sketch::tools::trim_preview(&sketch, id, cursor)
        {
            frame.path(
                &painter,
                piece.sample(128),
                egui::Stroke::new(3., theme::DANGER),
                false,
            );
        }
        if work.tool == Tool::Circle && work.circle_mode == CircleMode::Tangent {
            for id in &work.tangent_lines {
                if let Ok((a, b)) = sketch.line(*id) {
                    frame.infinite_line(&painter, a, b, egui::Stroke::new(2., theme::GREEN));
                }
            }
            if tangent_selection {
                painter.text(
                    response.rect.left_top() + egui::vec2(12., 12.),
                    egui::Align2::LEFT_TOP,
                    text(if work.tangent_lines.is_empty() {
                        T::SketchPickFirstTangent
                    } else {
                        T::SketchPickSecondTangent
                    }),
                    egui::FontId::proportional(13.),
                    theme::INK,
                );
            }
        }
        let mut cursor = raw_cursor;
        let mut confirm = false;
        let mut valid = true;
        let mut adjusted = false;
        let escape;
        if !work.anchors.is_empty()
            && let Some(primitive) = work.primitive()
        {
            work.drawing.infinite = work.tool == Tool::Line && work.line_type == LineType::Infinite;
            if let Some(point) = raw_cursor {
                work.drawing.cursor = Some(point);
            }
            let output = work.drawing.show(
                ui.ctx(),
                &frame,
                primitive,
                &work.anchors,
                &parameters,
                !busy && !animating,
            );
            confirm = output.confirm;
            escape = output.cancel;
            valid = output.preview.is_some();
            if let Some(preview) = output.preview {
                if primitive == Primitive::TangentCircle {
                    use nator_sketch::indicators::{Indicator, Kind as IndicatorKind};
                    let mut markers = vec![Indicator {
                        point: preview.point,
                        kind: IndicatorKind::Center,
                    }];
                    for line in &work.tangent_lines {
                        if let Ok((a, b)) = sketch.line(*line) {
                            let d = nator_sketch::sub(b, a);
                            let foot = nator_sketch::add(
                                a,
                                nator_sketch::scale(
                                    d,
                                    nator_sketch::dot(nator_sketch::sub(preview.point, a), d)
                                        / nator_sketch::dot(d, d),
                                ),
                            );
                            markers.push(Indicator {
                                point: foot,
                                kind: IndicatorKind::Tangent,
                            });
                        }
                    }
                    indicators::paint(ui, &frame, &painter, &markers);
                }
                cursor = Some(preview.point);
                adjusted =
                    raw_cursor.is_none_or(|p| nator_sketch::distance(p, preview.point) > 1e-7);
                if let Some(start) = preview.arc_start {
                    work.anchors[1] = start;
                }
                let construction = work.line_type != LineType::Normal;
                let stroke = egui::Stroke::new(
                    1.5,
                    if construction {
                        theme::CONSTRUCTION
                    } else {
                        theme::INK
                    },
                );
                if primitive == Primitive::DiameterCircle {
                    frame.path(
                        &painter,
                        [work.anchors[0], preview.point],
                        egui::Stroke::new(1., theme::INK),
                        true,
                    );
                }
                if work.tool == Tool::Line && work.line_type == LineType::Infinite {
                    frame.infinite_line(&painter, work.anchors[0], preview.point, stroke);
                } else if work.tool == Tool::Rectangle && work.line_type == LineType::Infinite {
                    for edge in preview.path.windows(2) {
                        frame.infinite_line(&painter, edge[0], edge[1], stroke);
                    }
                } else {
                    frame.path(&painter, preview.path, stroke, construction);
                }
            }
        } else {
            escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        }
        if escape {
            work.activate_selection(ui.ctx());
        }
        let dimension = (work.tool == Tool::Dimension && !existing_update.blocked).then(|| {
            dimension_tool::update(
                ui,
                response,
                &frame,
                &painter,
                &sketch,
                &mut work.dimension_draft,
                hover,
                !busy && !animating,
                &parameters,
            )
        });
        let projection_result = if work.tool == Tool::Project && !escape && !animating {
            hover.map(|cursor| {
                work.projector.update(
                    &frame,
                    &painter,
                    cursor,
                    !busy && response.clicked_by(egui::PointerButton::Primary),
                    &mut sketch,
                )
            })
        } else {
            None
        };
        if matches!(projection_result, Some(Ok(true))) {
            submit = true;
        }
        if !busy
            && !animating
            && !escape
            && !junction_update.blocked
            && !existing_update.blocked
            && valid
            && !matches!(work.tool, Tool::Dimension | Tool::Project)
            && !(work.tool == Tool::Rectangle
                && work.rectangle_mode == RectangleMode::Inclined
                && work.anchors.len() == 1
                && cursor
                    .is_some_and(|point| nator_sketch::distance(work.anchors[0], point) < 1e-9))
            && (response.clicked_by(egui::PointerButton::Primary) || confirm)
            && let Some(cursor) = cursor
        {
            if matches!(work.tool, Tool::Select | Tool::Trim) {
                if work.is_selecting() {
                    work.selected_vertex = junction_tool::vertex(
                        &sketch,
                        &work.references.points.vertices,
                        cursor,
                        8. / pixels_per_mm,
                    );
                }
                let entity = nator_sketch::tools::pick_entity(&sketch, cursor, 8. / pixels_per_mm)
                    .ok()
                    .flatten();
                if let Some(id) = entity {
                    if work.tool == Tool::Trim {
                        match nator_sketch::tools::trim(&mut sketch, id, cursor) {
                            Ok(()) => submit = true,
                            Err(error) => {
                                painter.text(
                                    response.rect.left_top(),
                                    egui::Align2::LEFT_TOP,
                                    error.to_string(),
                                    egui::FontId::proportional(13.),
                                    theme::DANGER,
                                );
                            }
                        }
                    } else {
                        if work.active_action == Some(EditAction::Mirror) && work.mirror_axis_active
                        {
                            if sketch.line(id).is_ok() {
                                work.mirror_axis = Some(id);
                                work.selected.retain(|selected| *selected != id);
                                work.mirror_axis_active = false;
                            }
                        } else if work.active_action == Some(EditAction::Mirror)
                            && work.mirror_axis == Some(id)
                        {
                            // O eixo pertence ao campo próprio e não vira origem da cópia.
                        } else if work.active_action.is_some() {
                            if let Some(index) = work.selected.iter().position(|e| *e == id) {
                                work.selected.remove(index);
                            } else {
                                work.selected.push(id);
                            }
                        } else if !ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                            work.selected.clear();
                            work.selected.push(id);
                        } else if let Some(index) =
                            work.selected.iter().position(|selected| *selected == id)
                        {
                            work.selected.remove(index);
                        } else {
                            work.selected.push(id);
                        }
                        work.constraint = None;
                        if work.active_action.is_none() {
                            work.dimension = sketch
                                .line(id)
                                .map(|(a, b)| nator_sketch::distance(a, b))
                                .or_else(|_| sketch.radial(id).map(|(_, r)| r))
                                .unwrap_or(10.);
                            work.dimension_expression = format!("{} mm", work.dimension);
                        }
                    }
                } else {
                    if work.active_action.is_none()
                        && !ui.input(|input| input.modifiers.ctrl || input.modifiers.command)
                    {
                        work.selected.clear();
                    }
                }
            } else if tangent_selection {
                let selection =
                    nator_sketch::tools::pick_entity(&sketch, cursor, 8. / pixels_per_mm)
                        .and_then(|id| {
                            id.ok_or_else(|| {
                                nator_sketch::SketchError(text(T::SketchPickLine).into())
                            })
                        })
                        .and_then(|id| {
                            sketch.line(id)?;
                            Ok(id)
                        });
                let selection = selection.and_then(|id| {
                    if let Some(first) = work.tangent_lines.first() {
                        let anchors = drafting::tangent_frame(&sketch, *first, id)?;
                        work.anchors = anchors.to_vec();
                        work.anchor_kinds = vec![None; 3];
                        work.drawing.start(cursor);
                    }
                    work.tangent_lines.push(id);
                    Ok(())
                });
                if let Err(error) = selection {
                    painter.text(
                        response.rect.left_top() + egui::vec2(12., 36.),
                        egui::Align2::LEFT_TOP,
                        error.to_string(),
                        egui::FontId::proportional(13.),
                        theme::DANGER,
                    );
                }
            } else {
                work.anchors.push(cursor);
                // Uma medida digitada pode afastar a ponta da captura. Não gravar
                // nesse caso uma coincidência que contradiria a cota escolhida.
                work.anchor_kinds.push(if adjusted {
                    None
                } else {
                    snap.as_ref().map(|s| s.kind)
                });
                let kind = work.anchor_kinds.last().copied().flatten();
                work.anchor_strengths.push(
                    work.junction_override
                        .take()
                        .unwrap_or_else(|| nator_sketch::junctions::default_strong(kind)),
                );
                work.tracker.capture(cursor);
                let needed = match work.primitive() {
                    Some(Primitive::Arc | Primitive::InclinedRectangle) => 3,
                    Some(Primitive::TangentCircle) => 4,
                    _ => 2,
                };
                if work.anchors.len() == 1 {
                    work.drawing.start(cursor);
                    if work.tool == Tool::Line {
                        work.chain_points = vec![cursor];
                        let preferred = snap.as_ref().and_then(|s| match s.kind {
                            Kind::OnCurve(id) | Kind::Midpoint(id) => Some(id),
                            _ => None,
                        });
                        work.line_reference =
                            nator_sketch::lines::reference(&sketch, cursor, 1e-6, preferred);
                        work.drawing.relative = work.line_reference.is_some();
                        work.drawing.reference_angle = work
                            .line_reference
                            .and_then(|id| nator_sketch::lines::angle(&sketch, id).ok())
                            .unwrap_or(0.);
                    }
                    ui.ctx().request_repaint();
                }
                if work.tool == Tool::Arc && work.anchors.len() == 2 {
                    work.drawing.arc_next();
                }
                if work.tool == Tool::Rectangle
                    && work.rectangle_mode == RectangleMode::Inclined
                    && work.anchors.len() == 2
                {
                    work.drawing.side_fields =
                        Some((work.drawing.fields.clone(), work.drawing.locked));
                    work.drawing.start_width(cursor);
                }
                if work.anchors.len() == needed {
                    // A falha descarta também pontos/restrições parciais. Somente
                    // a geometria completa segue para o comando de atualização.
                    let mut candidate = sketch.clone();
                    let result = draw_entity(&mut candidate, work, &parameters);
                    match result {
                        Ok(()) => {
                            sketch = candidate;
                            submit = true;
                            if work.tool == Tool::Line {
                                work.continue_line(&sketch, ui.ctx(), cursor);
                            } else {
                                work.reset_drawing(ui.ctx());
                            }
                        }
                        Err(error) => {
                            painter.text(
                                response.rect.left_top(),
                                egui::Align2::LEFT_TOP,
                                error,
                                egui::FontId::proportional(13.),
                                theme::DANGER,
                            );
                            work.anchors.pop();
                            work.anchor_kinds.pop();
                            work.anchor_strengths.pop();
                        }
                    }
                }
            }
        }
        // O distintivo usa o mesmo ícone da faixa e só existe sobre o canvas,
        // sem substituir o ponteiro nem cobrir a ponta usada para capturar apoios.
        if work.tool != Tool::Select
            && work.active_action.is_none()
            && let Some(position) = response
                .hover_pos()
                .filter(|p| !view_cube::rect(response.rect).contains(*p))
        {
            let icon = line_tools::cursor_icon(work);
            icons::paint(
                &painter,
                egui::Rect::from_min_size(position + egui::vec2(16., -28.), egui::Vec2::splat(22.)),
                icon,
                true,
            );
        }
        if let Some(id) = existing_update.changed {
            self.solve_sketch_with(ui.ctx(), sketch, Some((id, false)));
        } else if submit {
            self.solve_sketch(ui.ctx(), sketch);
        }
        if let Some(error) = existing_update.warning {
            self.status.warning(error);
        }
        if let Some(Err(error)) = projection_result {
            self.status.warning(error);
        }
        if let Some(error) = junction_update.warning {
            self.status.warning(error);
        }
        if let Some(update) = dimension {
            if let Some(hint) = update.hint {
                self.status.set(text(hint));
            }
            if let Some(warning) = update.warning {
                self.status.warning(warning);
            }
            if let Some((candidate, id)) = update.commit {
                self.solve_sketch_with(ui.ctx(), candidate, Some((id, true)));
            }
        }
    }
}

fn draw_entity(
    sketch: &mut Sketch,
    work: &Workspace,
    parameters: &nator_document::parameters::Parameters,
) -> Result<(), String> {
    let a = work.anchors[0];
    let b = work.anchors[1];
    match work.tool {
        Tool::Line => {
            let id = sketch
                .add_line_with_type(a, b, work.line_type)
                .map_err(sketch_error)?;
            if let Curve::Line { start, end } = sketch.entity(id).map_err(sketch_error)?.curve {
                attach(
                    sketch,
                    start,
                    work.anchor_kinds[0],
                    a,
                    work.anchor_strong(0),
                )?;
                attach(sketch, end, work.anchor_kinds[1], b, work.anchor_strong(1))?;
                let perpendicular = work.perpendicular.filter(|reference| {
                    sketch.line(*reference).is_ok_and(|(c, d)| {
                        let u = nator_sketch::sub(b, a);
                        let v = nator_sketch::sub(d, c);
                        nator_sketch::dot(u, v).abs()
                            < 1e-7 * nator_sketch::distance(a, b) * nator_sketch::distance(c, d)
                    })
                });
                if !work.drawing.locked[1]
                    && let Some(reference) = perpendicular
                {
                    sketch
                        .add_constraint(
                            ConstraintKind::Perpendicular {
                                a: reference,
                                b: id,
                            },
                            false,
                        )
                        .map_err(sketch_error)?;
                }
                if !work.drawing.locked[1] && perpendicular.is_none() && (a[1] - b[1]).abs() < 1e-6
                {
                    sketch
                        .add_constraint(ConstraintKind::Horizontal { line: id }, false)
                        .map_err(sketch_error)?;
                }
                if !work.drawing.locked[1] && perpendicular.is_none() && (a[0] - b[0]).abs() < 1e-6
                {
                    sketch
                        .add_constraint(ConstraintKind::Vertical { line: id }, false)
                        .map_err(sketch_error)?;
                }
                if work.drawing.locked[0] && work.line_type != LineType::Infinite {
                    drive(
                        sketch,
                        ConstraintKind::Distance {
                            a: start,
                            b: end,
                            value: nator_sketch::distance(a, b),
                        },
                        work.drawing.fields[0].clone(),
                    )?;
                }
                if work.drawing.locked[1] {
                    let d = nator_sketch::sub(b, a);
                    drive(
                        sketch,
                        if let Some(reference) = work.line_reference {
                            ConstraintKind::LineAngle {
                                line: id,
                                reference,
                                value: nator_sketch::lines::relative_angle(
                                    d[1].atan2(d[0]),
                                    work.drawing.reference_angle,
                                ),
                                flip_line: false,
                                flip_reference: false,
                            }
                        } else {
                            ConstraintKind::Angle {
                                a: start,
                                b: end,
                                value: d[1].atan2(d[0]),
                            }
                        },
                        work.drawing.fields[1].clone(),
                    )?;
                }
                if work.line_mode == LineMode::Symmetric {
                    let opposite = nator_sketch::sub(nator_sketch::scale(a, 2.), b);
                    let mirror_start = sketch.add_point(opposite).map_err(sketch_error)?;
                    // Compartilhar o ponto central evita duas metades desconectadas.
                    // Paralelismo e igualdade mantêm a simetria ao editar as medidas.
                    let mirror = sketch
                        .add_curve(
                            Curve::Line {
                                start: mirror_start,
                                end: start,
                            },
                            work.line_type != LineType::Normal,
                        )
                        .map_err(sketch_error)?;
                    if work.line_type == LineType::Infinite
                        && let Some(entity) = sketch
                            .entities
                            .iter_mut()
                            .find(|entity| entity.id == mirror)
                    {
                        entity.infinite = true;
                    }
                    sketch
                        .add_constraint(ConstraintKind::Parallel { a: id, b: mirror }, false)
                        .map_err(sketch_error)?;
                    sketch
                        .add_constraint(ConstraintKind::EqualLength { a: id, b: mirror }, false)
                        .map_err(sketch_error)?;
                }
            }
        }
        Tool::Rectangle => {
            let inclined = work.rectangle_mode == RectangleMode::Inclined;
            let corners = match work.rectangle_mode {
                RectangleMode::Normal => [a, [b[0], a[1]], b, [a[0], b[1]]],
                RectangleMode::Center => {
                    let opposite = nator_sketch::sub(nator_sketch::scale(a, 2.), b);
                    [opposite, [b[0], opposite[1]], b, [opposite[0], b[1]]]
                }
                RectangleMode::Inclined => {
                    let preview = drafting::preview(
                        Primitive::InclinedRectangle,
                        &[a, b],
                        work.anchors[2],
                        [None, None],
                    )
                    .map_err(sketch_error)?;
                    [
                        preview.path[0],
                        preview.path[1],
                        preview.path[2],
                        preview.path[3],
                    ]
                }
            };
            let lines = drafting::add_rectangle(sketch, corners, work.line_type, inclined)
                .map_err(sketch_error)?;
            let point_ids = lines
                .iter()
                .map(|id| {
                    if let Curve::Line { start, .. } =
                        sketch.entity(*id).map_err(sketch_error)?.curve
                    {
                        Ok(start)
                    } else {
                        Err(text(T::InvalidArc).to_owned())
                    }
                })
                .collect::<Result<Vec<_>, String>>()?;
            // O clique do modo central é o centro, nunca um dos cantos. Capturar
            // esse clique como canto deslocaria o retângulo ao resolver restrições.
            if work.rectangle_mode != RectangleMode::Center {
                attach(
                    sketch,
                    point_ids[0],
                    work.anchor_kinds[0],
                    a,
                    work.anchor_strong(0),
                )?;
            }
            let last = if inclined { 2 } else { 1 };
            attach(
                sketch,
                point_ids[2],
                work.anchor_kinds[last],
                corners[2],
                work.anchor_strong(last),
            )?;
            if inclined {
                attach(
                    sketch,
                    point_ids[1],
                    work.anchor_kinds[1],
                    b,
                    work.anchor_strong(1),
                )?;
                if let Some((fields, locked)) = &work.drawing.side_fields {
                    if locked[0] && work.line_type != LineType::Infinite {
                        drive(
                            sketch,
                            ConstraintKind::Distance {
                                a: point_ids[0],
                                b: point_ids[1],
                                value: nator_sketch::distance(a, b),
                            },
                            fields[0].clone(),
                        )?;
                    }
                    if locked[1] {
                        let direction = nator_sketch::sub(b, a);
                        drive(
                            sketch,
                            ConstraintKind::Angle {
                                a: point_ids[0],
                                b: point_ids[1],
                                value: direction[1].atan2(direction[0]),
                            },
                            fields[1].clone(),
                        )?;
                    }
                }
                if work.drawing.locked[0] && work.line_type != LineType::Infinite {
                    drive(
                        sketch,
                        ConstraintKind::Distance {
                            a: point_ids[1],
                            b: point_ids[2],
                            value: nator_sketch::distance(corners[1], corners[2]),
                        },
                        work.drawing.fields[0].clone(),
                    )?;
                }
            } else {
                for (index, end) in [(0, 1), (1, 3)] {
                    if work.drawing.locked[index] && work.line_type != LineType::Infinite {
                        let value = corners[2][index] - corners[0][index];
                        let expression = if value < 0. {
                            format!("-({})", work.drawing.fields[index])
                        } else {
                            work.drawing.fields[index].clone()
                        };
                        let kind = if index == 0 {
                            ConstraintKind::DeltaX {
                                a: point_ids[0],
                                b: point_ids[end],
                                value,
                            }
                        } else {
                            ConstraintKind::DeltaY {
                                a: point_ids[0],
                                b: point_ids[end],
                                value,
                            }
                        };
                        drive(sketch, kind, expression)?;
                    }
                }
            }
        }
        Tool::Circle => {
            let (center, radius) = match work.circle_mode {
                CircleMode::Normal => (a, nator_sketch::distance(a, b)),
                CircleMode::Diameter => (
                    nator_sketch::scale(nator_sketch::add(a, b), 0.5),
                    nator_sketch::distance(a, b) * 0.5,
                ),
                CircleMode::Tangent => {
                    let center = work.anchors[3];
                    let (line_start, line_end) =
                        sketch.line(work.tangent_lines[0]).map_err(sketch_error)?;
                    let radius = nator_sketch::cross(
                        nator_sketch::sub(center, line_start),
                        nator_sketch::sub(line_end, line_start),
                    )
                    .abs()
                        / nator_sketch::distance(line_start, line_end);
                    (center, radius)
                }
            };
            let id = sketch
                .add_circle(center, radius, work.line_type != LineType::Normal)
                .map_err(sketch_error)?;
            if let Curve::Circle { center: point, .. } =
                sketch.entity(id).map_err(sketch_error)?.curve
            {
                if work.circle_mode == CircleMode::Normal {
                    attach(
                        sketch,
                        point,
                        work.anchor_kinds[0],
                        a,
                        work.anchor_strong(0),
                    )?;
                } else if work.circle_mode == CircleMode::Tangent {
                    for line in &work.tangent_lines {
                        sketch
                            .add_constraint(
                                ConstraintKind::Tangent {
                                    line: *line,
                                    arc: id,
                                    point,
                                },
                                false,
                            )
                            .map_err(sketch_error)?;
                    }
                }
            }
            if work.drawing.locked[0] {
                let expression = if work.circle_mode == CircleMode::Diameter {
                    format!("({}) / 2", work.drawing.fields[0])
                } else {
                    work.drawing.fields[0].clone()
                };
                drive(
                    sketch,
                    ConstraintKind::Radius {
                        circle: id,
                        value: radius,
                    },
                    expression,
                )?;
            }
        }
        Tool::Arc => {
            let radius = nator_sketch::distance(a, b);
            let end = work.anchors[2];
            let length = nator_sketch::distance(a, end);
            if radius < 1e-9 || length < 1e-9 {
                return Err(text(T::InvalidArc).into());
            }
            let end = nator_sketch::add(
                a,
                nator_sketch::scale(nator_sketch::sub(end, a), radius / length),
            );
            let center = sketch.add_point(a).map_err(sketch_error)?;
            let start = sketch.add_point(b).map_err(sketch_error)?;
            let end = sketch.add_point(end).map_err(sketch_error)?;
            let id = sketch
                .add_curve(
                    Curve::Arc {
                        center,
                        start,
                        end,
                        clockwise: false,
                    },
                    work.line_type != LineType::Normal,
                )
                .map_err(sketch_error)?;
            attach(
                sketch,
                center,
                work.anchor_kinds[0],
                a,
                work.anchor_strong(0),
            )?;
            // A ponta inicial só mantém a captura se ainda estiver no raio original.
            attach(
                sketch,
                start,
                work.anchor_kinds[1].filter(|_| !work.drawing.locked[0]),
                b,
                work.anchor_strong(1),
            )?;
            if work.drawing.locked[0] {
                drive(
                    sketch,
                    ConstraintKind::Radius {
                        circle: id,
                        value: radius,
                    },
                    work.drawing.fields[0].clone(),
                )?;
            }
            let d = nator_sketch::sub(b, a);
            let angle = d[1].atan2(d[0]);
            let start_expression = work
                .drawing
                .arc_angle
                .as_ref()
                .map(|expression| angle_expression(expression, parameters))
                .unwrap_or_else(|| format!("{} deg", angle.to_degrees()));
            if work.drawing.arc_angle.is_some() || work.drawing.locked[1] {
                drive(
                    sketch,
                    ConstraintKind::Angle {
                        a: center,
                        b: start,
                        value: angle,
                    },
                    start_expression.clone(),
                )?;
            }
            if work.drawing.locked[1] {
                let delta = nator_sketch::sub(sketch.point(end).map_err(sketch_error)?, a);
                // Abertura é uma expressão angular; somar à orientação inicial
                // mantém o raio e o início ao recalcular os parâmetros da peça.
                drive(
                    sketch,
                    ConstraintKind::Angle {
                        a: center,
                        b: end,
                        value: delta[1].atan2(delta[0]),
                    },
                    format!(
                        "({start_expression}) + ({})",
                        angle_expression(&work.drawing.fields[1], parameters)
                    ),
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn drive(sketch: &mut Sketch, kind: ConstraintKind, expression: String) -> Result<(), String> {
    let id = sketch.add_constraint(kind, false).map_err(sketch_error)?;
    sketch.dimension_expressions.insert(id, expression);
    Ok(())
}
fn angle_expression(
    expression: &str,
    parameters: &nator_document::parameters::Parameters,
) -> String {
    // Sem unidade explícita, a entrada angular usa graus. Ao compor a abertura
    // com a orientação inicial precisamos materializar essa unidade antes da soma.
    if parameters
        .evaluate(expression, nator_document::parameters::Dimension::Scalar)
        .is_ok()
    {
        format!("({expression}) * 1 deg")
    } else {
        expression.to_owned()
    }
}
fn dimension_expression(kind: &ConstraintKind, value: f64) -> String {
    if kind.is_angular() {
        format!("{} deg", value.to_degrees())
    } else {
        format!("{value} mm")
    }
}
fn dimension_value(kind: &ConstraintKind) -> Option<f64> {
    kind.dimension_value()
}
fn set_dimension_value(kind: &mut ConstraintKind, next: f64) {
    if let Some(value) = kind.dimension_value_mut() {
        *value = next;
    }
}
fn constraint_label(kind: &ConstraintKind) -> String {
    let key = match kind {
        ConstraintKind::Fixed { .. } => T::Fixed,
        ConstraintKind::Coincident { .. } => T::Coincident,
        ConstraintKind::Horizontal { .. } => T::Horizontal,
        ConstraintKind::Vertical { .. } => T::Vertical,
        ConstraintKind::Distance { .. } => T::Dimension,
        ConstraintKind::DeltaX { .. } => T::HorizontalDimension,
        ConstraintKind::DeltaY { .. } => T::VerticalDimension,
        ConstraintKind::Radius { .. } => T::Radius,
        ConstraintKind::Diameter { .. } => T::DiameterLabel,
        ConstraintKind::PointLineDistance { .. } => T::PointLineDistance,
        ConstraintKind::LineCircleDistance { .. } => T::LineCircleDistance,
        ConstraintKind::Angle { .. } | ConstraintKind::LineAngle { .. } => T::AngleLabel,
        ConstraintKind::Parallel { .. } => T::Parallel,
        ConstraintKind::Perpendicular { .. } => T::Perpendicular,
        ConstraintKind::EqualLength { .. } => T::EqualLength,
        ConstraintKind::Midpoint { .. } => T::Midpoint,
        ConstraintKind::PointOnLine { .. } => T::PointOnLine,
        ConstraintKind::PointOnCurve { .. } => T::PointOnCurve,
        ConstraintKind::Concentric { .. } => T::Concentric,
        ConstraintKind::Tangent { .. } => T::Tangent,
    };
    match dimension_value(kind) {
        Some(value) if kind.is_angular() => {
            format!(
                "{}: {}°",
                text(key),
                nator_drawing::format_mm(value.to_degrees())
            )
        }
        Some(value) => format!("{}: {value:.3}", text(key)),
        None => text(key).into(),
    }
}

// Cotas de referência medem a geometria; somente cotas dirigentes recebem expressão.
fn bind_dimension(sketch: &mut Sketch, id: u64, work: &Workspace) {
    if work.reference {
        sketch.dimension_expressions.remove(&id);
    } else {
        sketch
            .dimension_expressions
            .insert(id, work.dimension_expression.clone());
    }
}

#[cfg(test)]
mod tests;
