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
mod direct_drag;
pub(super) mod dynamic_input;
mod indicators;
mod junction_tool;
mod line_tools;
mod operation_handles;
mod operation_preview;
mod project_tool;
mod projection;
mod references;
mod restrictions;
pub(super) use actions::{edit_icon, edit_label};
use nator_sketch::edit::Action as EditAction;
pub(super) use restrictions::{Choice, GALLERY, presentation};

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
            Self::Select => T::SketchDiagnostics,
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
            Self::Select => icons::Icon::ConstraintManager,
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
enum SolveMode {
    Normal,
    Dimension(u64, bool),
}
pub(super) struct Workspace {
    pub id: SketchId,
    pub pending: Option<Sketch>,
    /// Estado anterior à edição: Cancelar restaura apenas este rascunho, sem
    /// desfazer alterações independentes feitas no restante do documento.
    pub(super) original_sketch: Option<Sketch>,
    /// Visibilidade antes de abrir a edição. Confirmar ou cancelar sem mudanças
    /// devolve o rascunho oculto ao estado anterior.
    pub(super) original_visible: bool,
    pub status: Status,
    pub redundant: bool,
    tool: Tool,
    /// Entrar no rascunho não abre painel; somente uma escolha explícita na faixa.
    tool_panel_open: bool,
    /// Pontos clicados com o Aparar enquanto o corte anterior era calculado.
    pending_trims: std::collections::VecDeque<Point2>,
    pub(super) line_type: LineType,
    line_mode: LineMode,
    rectangle_mode: RectangleMode,
    circle_mode: CircleMode,
    tangent_lines: Vec<u64>,
    /// Círculo ou arco onde começou a linha; com Shift, a origem desliza nele e a linha
    /// sai tangente.
    tangent_origin: Option<u64>,
    /// Shift estava pressionado no último quadro: a linha confirmada recebe a tangência.
    tangent_held: bool,
    /// Ponto inicial e vértices já percorridos da cadeia, para encerrar ao fechar.
    chain_points: Vec<Point2>,
    /// Capturada uma vez no primeiro clique; não muda ao cruzar outra linha.
    line_reference: Option<u64>,
    perpendicular: Option<u64>,
    anchors: Vec<Point2>,
    anchor_kinds: Vec<Option<Kind>>,
    /// Vértices escolhidos pela posição: cruzamentos ainda não têm apoio real.
    selected_vertices: Vec<Point2>,
    selected: Vec<u64>,
    /// Menu de restrições aberto pelo botão direito sobre um elemento.
    restriction_menu: bool,
    /// Restrição esperando a seleção: cada clique acumula até ela valer.
    armed: Option<nator_sketch::restrict::Restriction>,
    constraint: Option<u64>,
    pub(super) placement: Placement,
    /// Unidade e precisão do mouse, copiadas do documento a cada quadro.
    pub(super) settings: nator_document::Settings,
    /// Precisão aplicada às medidas livres. Sem o mouse sobre o canvas, conserva a última
    /// decisão: confirmar por Enter não pode tirar a ponta de um apoio exato.
    drawing_steps: Option<drafting::Steps>,
    pub(super) previous_camera: Camera,
    pub(super) fit_requested: bool,
    tracker: Tracker,
    projection: projection::Cache,
    projector: project_tool::Cache,
    dimension: f64,
    dimension_expression: String,
    reference: bool,
    /// Ocorrências dos padrões do rascunho (o espaçamento usa `dimension`).
    /// Intenção aberta pela faixa; os cliques alternam a seleção até OK/Cancelar.
    active_action: Option<EditAction>,
    operation_line_type: LineType,
    mirror_axis: Option<u64>,
    mirror_axis_active: bool,
    offset_mode: nator_sketch::modifications::OffsetMode,
    offset_second_expression: String,
    operation: operation_preview::Interaction,
    dragging: direct_drag::Interaction,
    drawing: dynamic_input::Input,
    references: references::Cache,
    /// Escolhas, texto e caixa de valor da ferramenta Cota.
    dimension_draft: dimension_tool::Draft,
    dimension_existing: dimension_tool::Existing,
}
impl Workspace {
    pub(super) fn text_snap(
        &self,
        sketch: &Sketch,
        point: Point2,
        tolerance: f64,
        tracker: &Tracker,
    ) -> Option<snapping::Snap> {
        snapping::find_with_edges(
            sketch,
            point,
            tolerance,
            tracker,
            &self.projection.points,
            snapping::ProjectedEdges {
                curves: &self.projection.curves,
                chains: &self.projection.chains,
            },
            &self.references.points,
        )
        .ok()
        .flatten()
    }
}

impl Workspace {
    pub(super) fn select_image(&mut self) {
        self.select_tool(Tool::Select);
        self.tool_panel_open = false;
        self.selected.clear();
        self.constraint = None;
    }
    pub(super) fn rename(&mut self, name: &str) {
        if let Some(sketch) = &mut self.pending {
            sketch.name = name.into();
        }
        if let Some(sketch) = &mut self.original_sketch {
            sketch.name = name.into();
        }
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn selected_entities(&self) -> &[u64] {
        &self.selected
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn operation_handle_tips(&self) -> &[egui::Pos2] {
        &self.operation.handle_tips
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn drag_pending(&self) -> bool {
        self.dragging.active()
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn references_pending(&self) -> bool {
        self.references.pending()
    }
    pub(super) fn operation_ready(&self) -> bool {
        self.operation.cache.ready()
    }
    pub(super) fn configure_operation_capture(
        &mut self,
        action: EditAction,
        selected: Vec<u64>,
        axis: Option<u64>,
    ) {
        self.select_tool(Tool::Select);
        self.tool_panel_open = false;
        self.active_action = Some(action);
        self.selected = selected;
        self.mirror_axis = axis;
        self.dimension_expression = "2 mm".into();
        self.offset_second_expression = "3 mm".into();
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn operation_debug(&self) -> String {
        format!(
            "seleção {:?}; valor {}; segundo {}",
            self.selected, self.dimension_expression, self.offset_second_expression
        )
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn selected_vertices(&self) -> &[Point2] {
        &self.selected_vertices
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn constraint_state(&self) -> &nator_sketch::constraint_state::State {
        &self.references.state
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn unlinked_ends(&self) -> usize {
        self.references.unlinked.len()
    }
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn projection_selected(&self) -> usize {
        self.projector.selected_count()
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
    /// Expressão de um campo de comprimento do desenho, com a unidade explícita.
    fn length_field(&self, index: usize) -> String {
        self.settings
            .unit
            .normalize_expression(&self.drawing.fields[index])
    }
    pub(super) fn active_action(&self) -> Option<EditAction> {
        self.active_action
    }
    /// Situação de restrição do rascunho mostrado, quando já calculada.
    pub(super) fn freedom(&self) -> Option<Status> {
        self.references.status.clone()
    }
    /// Painel de diagnóstico aberto: Selecionar com o painel da ferramenta visível.
    pub(super) fn diagnostics_open(&self) -> bool {
        self.tool == Tool::Select && self.tool_panel_open
    }
    pub(super) fn tool(&self) -> Tool {
        self.tool
    }
    pub(super) fn armed(&self) -> Option<nator_sketch::restrict::Restriction> {
        self.armed
    }
    /// Caixa de valor da Cota aberta e quantidade de escolhas, para os ensaios.
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn dimension_state(&self) -> (bool, usize) {
        (self.dimension_draft.editing(), self.dimension_draft.picks())
    }
    /// Alterna um canto de Arredondar/Chanfrar. A seleção de linhas acompanha os
    /// vértices escolhidos, para a ferramenta só ver as linhas desses cantos.
    fn toggle_corner(&mut self, sketch: &Sketch, position: Point2) {
        let corners = &mut self.operation.corners;
        if let Some(index) = corners
            .iter()
            .position(|p| nator_sketch::distance(*p, position) < 1e-7)
        {
            corners.remove(index);
        } else {
            corners.push(position);
        }
        self.selected.clear();
        for corner in &self.operation.corners {
            for id in junction_tool::corner_lines(sketch, *corner) {
                if !self.selected.contains(&id) {
                    self.selected.push(id);
                }
            }
        }
        self.operation.selecting = self.selected.is_empty();
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
        if self.tool == Tool::Project || tool == Tool::Project {
            self.projector = project_tool::Cache::default();
        }
        self.active_action = None;
        self.operation = operation_preview::Interaction::default();
        self.dragging = direct_drag::Interaction::default();
        self.mirror_axis = None;
        self.mirror_axis_active = false;
        self.tool = tool;
        self.tool_panel_open = true;
        self.anchors.clear();
        self.anchor_kinds.clear();
        self.selected_vertices.clear();
        self.armed = None;
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
        self.chain_points.clear();
        self.tangent_lines.clear();
        self.tangent_origin = None;
        self.tangent_held = false;
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
        let last = sketch.entities.iter().rev().find(|entity| {
            matches!(entity.curve, Curve::Line { .. })
                && entity.point_ids().into_iter().any(|id| {
                    sketch
                        .point(id)
                        .is_ok_and(|p| nator_sketch::distance(p, end) < 1e-7)
                })
        });
        // A sequência só continua numa ponta solta: reta infinita, ponta capturada num
        // elemento ou num vértice de outra entidade encerram o traço.
        let touches = self
            .anchor_kinds
            .last()
            .copied()
            .flatten()
            .is_some_and(ends_on_element)
            || sketch.entities.iter().any(|entity| {
                Some(entity.id) != last.map(|l| l.id)
                    && entity.point_ids().into_iter().any(|id| {
                        sketch
                            .point(id)
                            .is_ok_and(|p| nator_sketch::distance(p, end) < 1e-7)
                    })
            });
        if closed || touches || self.line_type == LineType::Infinite {
            self.reset_drawing(context);
            return;
        }
        self.chain_points.push(end);
        self.tangent_origin = None;
        self.tangent_held = false;
        let endpoint = last.and_then(|entity| {
            entity.point_ids().into_iter().find(|id| {
                sketch
                    .point(*id)
                    .is_ok_and(|p| nator_sketch::distance(p, end) < 1e-7)
            })
        });
        self.anchors = vec![end];
        self.anchor_kinds = vec![endpoint.map(Kind::Endpoint)];
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
            original_visible: true,
            status: Status::UnderConstrained {
                degrees_of_freedom: 0,
            },
            redundant: false,
            tool: Tool::Select,
            tool_panel_open: false,
            pending_trims: Default::default(),
            line_type: LineType::Normal,
            line_mode: LineMode::Normal,
            rectangle_mode: RectangleMode::Normal,
            circle_mode: CircleMode::Normal,
            tangent_lines: Vec::new(),
            tangent_origin: None,
            tangent_held: false,
            chain_points: Vec::new(),
            line_reference: None,
            perpendicular: None,
            anchors: Vec::new(),
            anchor_kinds: Vec::new(),
            selected_vertices: Vec::new(),
            selected: Vec::new(),
            restriction_menu: false,
            armed: None,
            constraint: None,
            placement: Placement::default(),
            settings: nator_document::Settings::default(),
            drawing_steps: None,
            previous_camera: Camera::default(),
            fit_requested: true,
            tracker: Tracker::default(),
            projection: projection::Cache::default(),
            projector: project_tool::Cache::default(),
            dimension: 10.,
            dimension_expression: "10 mm".into(),
            reference: false,
            active_action: None,
            operation_line_type: LineType::Normal,
            mirror_axis: None,
            mirror_axis_active: false,
            offset_mode: nator_sketch::modifications::OffsetMode::Normal,
            offset_second_expression: "10 mm".into(),
            operation: operation_preview::Interaction::default(),
            dragging: direct_drag::Interaction::default(),
            drawing: dynamic_input::Input::default(),
            references: references::Cache::default(),
            dimension_draft: dimension_tool::Draft::default(),
            dimension_existing: dimension_tool::Existing::default(),
        }
    }
}
/// Captura que prende a ponta a uma geometria existente. Alinhamentos, passos da precisão e
/// centros de área só posicionam e deixam a ponta solta.
fn ends_on_element(kind: Kind) -> bool {
    !matches!(kind, Kind::Alignment | Kind::Step | Kind::RegionCenter)
}

fn sketch_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
fn attach(
    sketch: &mut Sketch,
    point: u64,
    kind: Option<Kind>,
    position: Point2,
) -> Result<(), String> {
    nator_sketch::junctions::attach(sketch, point, kind, position).map_err(sketch_error)
}
impl Editor {
    pub(super) fn begin_sketch(&mut self, part: PartId, id: SketchId) {
        self.begin_sketch_in(Some(part), id);
    }
    /// Edição de um rascunho de referência da montagem, em coordenadas da montagem.
    pub(super) fn begin_reference_sketch(&mut self, id: SketchId) {
        self.begin_sketch_in(None, id);
    }
    pub(super) fn begin_history_sketch(&mut self, owner: Option<PartId>, id: SketchId) {
        self.begin_sketch_in(owner, id);
    }
    pub(super) fn select_history_elements(&mut self, entities: &[u64], constraints: &[u64]) {
        let Some(sketch) = self.working_sketch() else {
            return;
        };
        if let Some(work) = self.current_mut().sketch_edit.as_mut() {
            work.select_tool(Tool::Select);
            work.selected = entities
                .iter()
                .copied()
                .filter(|id| sketch.entity(*id).is_ok())
                .collect();
            work.constraint = constraints
                .iter()
                .copied()
                .find(|id| sketch.constraints.iter().any(|c| c.id == *id));
        }
    }
    /// Abrir não cria comando de visibilidade: a edição desenha o rascunho mesmo
    /// oculto, e um comando apagaria Refazer e sujaria o histórico sem haver mudança.
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
        let original_visible = self
            .current()
            .sketch_edit
            .as_ref()
            .filter(|work| work.id == id)
            .map(|work| work.original_visible)
            .unwrap_or(record.visible);
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
        work.original_visible = original_visible;
        work.placement = target;
        work.previous_camera = previous;
        work.fit_requested = false;
        // Rascunhos de referência pertencem à montagem: nenhuma peça entra em construção.
        self.current_mut().active_part = owner;
        self.current_mut().plane_pick = None;
        self.current_mut().sketch_edit = Some(work);
        self.align_sketch_camera(!record.sketch.points.is_empty(), true);
    }
    /// Ferramenta escolhida na faixa, na busca ou pelos atalhos do rascunho.
    pub(super) fn select_sketch_tool(&mut self, tool: Tool) {
        self.image_edit = None;
        self.sanitize_sketch_selection();
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
        self.solve_sketch_with(context, sketch, SolveMode::Normal);
    }
    pub(super) fn sanitize_sketch_selection(&mut self) {
        let Some(sketch) = self.working_sketch() else {
            return;
        };
        if let Some(work) = self.current_mut().sketch_edit.as_mut() {
            work.rename(&sketch.name);
            work.selected.retain(|id| sketch.entity(*id).is_ok());
            work.constraint = work
                .constraint
                .filter(|id| sketch.constraints.iter().any(|c| c.id == *id));
            work.mirror_axis = work.mirror_axis.filter(|id| sketch.line(*id).is_ok());
            work.dimension_existing.retain(&sketch);
        }
    }
    pub(super) fn delete_sketch_selection(&mut self, context: &egui::Context) {
        self.sanitize_sketch_selection();
        let Some(mut sketch) = self.working_sketch() else {
            return;
        };
        let Some(work) = self.current_mut().sketch_edit.as_mut() else {
            return;
        };
        if let Some(id) = work.constraint.filter(|_| work.selected.is_empty()) {
            nator_sketch::restrict::remove(&mut sketch, id);
        } else if work.selected.is_empty() {
            // Del em vértices entre duas retas deixa uma só aresta.
            let mut changed = false;
            for vertex in &work.selected_vertices {
                changed |= nator_sketch::junctions::dissolve(&mut sketch, *vertex)
                    .is_ok_and(|removed| removed);
            }
            if !changed {
                return;
            }
        } else {
            if let Err(error) = nator_sketch::edit::apply(
                &mut sketch,
                EditAction::Delete,
                &work.selected,
                nator_sketch::edit::Options {
                    value_mm: 0.,
                    reference: false,
                },
            ) {
                self.status.warning(error.to_string());
                return;
            }
        }
        work.selected.clear();
        work.constraint = None;
        work.selected_vertices.clear();
        work.dimension_existing = dimension_tool::Existing::default();
        self.solve_sketch(context, sketch);
    }
    pub(super) fn toggle_sketch_stroke(&mut self, context: &egui::Context) {
        self.sanitize_sketch_selection();
        let Some(mut sketch) = self.working_sketch() else {
            return;
        };
        let Some(work) = self.current_mut().sketch_edit.as_mut() else {
            return;
        };
        if work.is_selecting() && !work.selected.is_empty() {
            let reference = !work
                .selected
                .iter()
                .all(|id| sketch.entity(*id).is_ok_and(|e| e.construction));
            for entity in &mut sketch.entities {
                if work.selected.contains(&entity.id) {
                    entity.construction = reference;
                    if !reference {
                        entity.infinite = false;
                    }
                }
            }
            self.solve_sketch(context, sketch);
        } else if matches!(
            work.active_action,
            Some(EditAction::Mirror | EditAction::Offset)
        ) {
            work.operation_line_type = if work.operation_line_type == LineType::Normal {
                LineType::Construction
            } else {
                LineType::Normal
            };
        } else {
            work.line_type = if work.line_type == LineType::Normal {
                LineType::Construction
            } else {
                LineType::Normal
            };
        }
    }
    /// Cotas novas podem virar referência; a intenção é resolvida no trabalhador.
    fn solve_sketch_with(&mut self, context: &egui::Context, sketch: Sketch, dimension: SolveMode) {
        if self.busy() {
            return;
        }
        let Some(work) = &self.current().sketch_edit else {
            return;
        };
        let id = work.id;
        let check_area =
            work.tool == Tool::Line && work.line_type == LineType::Normal && work.is_drawing();
        let precision = work.settings.mouse_step_mm;
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
            // Pontas novas a menos da precisão de desenho fundem no vértice mais antigo.
            if let Some(previous) = document.sketch(id) {
                nator_sketch::fusion::fuse_new(&previous.sketch, &mut sketch, precision)
                    .map_err(sketch_error)?;
            }
            if let SolveMode::Dimension(id, true) = dimension {
                dimension_tool::settle_driven(&mut sketch, id, &parameters)?;
            }
            nator_document::parameters::resolve_sketch(&mut sketch, &parameters)
                .map_err(sketch_error)?;
            let report = match dimension {
                SolveMode::Dimension(id, _) => nator_sketch::dimensions::solve(&sketch, id),
                SolveMode::Normal => solver::solve(&sketch),
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
                report: Box::new(report),
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
        if !work.tool_panel_open
            || work.active_action.is_some()
            || matches!(work.tool, Tool::Dimension | Tool::Trim)
        {
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
                    Status::Conflict => (text(T::SketchConflict), theme::danger()),
                    Status::FullyConstrained => (text(T::FullyConstrained), theme::green()),
                    Status::UnderConstrained { .. } => (text(T::UnderConstrained), theme::accent()),
                };
                ui.colored_label(color, label);
                if let Status::UnderConstrained { degrees_of_freedom } = work.status {
                    ui.label(format!(
                        "{}: {degrees_of_freedom}",
                        text(T::DegreesOfFreedom)
                    ));
                }
                if work.redundant {
                    ui.colored_label(theme::warning(), text(T::Redundant));
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
                // Ligações fracas de projetos antigos não têm efeito nem aparecem.
                for constraint in sketch
                    .constraints
                    .iter()
                    .filter(|c| !c.reference || c.kind.is_dimension())
                {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                work.constraint == Some(constraint.id),
                                constraint_label(&constraint.kind, work.settings),
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
                                        dimension_expression(&constraint.kind, value, work.settings)
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
                    nator_sketch::restrict::remove(&mut sketch, id);
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
        let settings = self.current().session.document().settings();
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
        let image_blocked =
            self.reference_images_overlay(ui, response, &frame, &sketch, !busy && !animating)
                || (!busy && !animating && self.sketch_text_overlay(ui, response, &frame, &sketch));
        let mut text_preview = self.text_preview_sketch();
        let Some(work) = self.current_mut().sketch_edit.as_mut() else {
            return;
        };
        work.settings = settings;
        if hover.is_some()
            && (work.primitive().is_some()
                || work
                    .active_action
                    .is_some_and(|a| matches!(a, EditAction::Offset | EditAction::Mirror)))
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::X))
        {
            let stroke = if work.active_action.is_some() {
                &mut work.operation_line_type
            } else {
                &mut work.line_type
            };
            *stroke = if *stroke == LineType::Normal {
                LineType::Construction
            } else {
                LineType::Normal
            };
            ui.input_mut(|i| {
                i.events.retain(
                    |event| !matches!(event,egui::Event::Text(s) if s.eq_ignore_ascii_case("x")),
                )
            });
        }
        // Grade no próprio plano: o mesmo conjunto de linhas acompanha a câmera,
        // em vez de substituir a área CAD por uma superfície plana independente.
        let center = frame.cursor(response.rect.center()).unwrap_or([0., 0.]);
        // A grade mostra múltiplos da precisão do mouse, sem linhas densas demais.
        let mut step = settings.mouse_step_mm;
        while step * pixels_per_mm < 20. && step < 1e9 {
            step *= 10.;
        }
        let radius = f64::from(response.rect.width().max(response.rect.height())) / pixels_per_mm;
        let corners = [
            response.rect.left_top(),
            response.rect.right_top(),
            response.rect.left_bottom(),
            response.rect.right_bottom(),
        ]
        .into_iter()
        .filter_map(|p| frame.cursor(p))
        .collect::<Vec<_>>();
        for axis in 0..2 {
            let low = corners.iter().map(|p| p[axis]).fold(center[axis], f64::min);
            let high = corners.iter().map(|p| p[axis]).fold(center[axis], f64::max);
            let from = (low / step).floor().max(center[axis] / step - 128.) as i64 - 1;
            let to = (high / step).ceil().min(center[axis] / step + 128.) as i64 + 1;
            for i in from..=to {
                let coordinate = i as f64 * step;
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
        // Áreas projetadas servem apenas de referência: os contornos e vértices já
        // são desenhados como entidades lilás. Nenhum preenchimento cobre o rascunho.
        let display_sketch = text_preview.as_ref().unwrap_or(&sketch);
        for entity in &display_sketch.entities {
            if let Ok(segment) = profiles::segment(display_sketch, entity) {
                let color = if work.selected.contains(&entity.id) {
                    if work.active_action.is_some_and(operation_preview::supported) {
                        egui::Color32::from_rgba_unmultiplied(80, 100, 160, 100)
                    } else {
                        egui::Color32::from_rgb(183, 106, 17)
                    }
                } else if entity.face {
                    theme::face_line()
                } else if display_sketch.text_of(entity.id).is_some() {
                    theme::ink()
                } else if entity.projected {
                    theme::projected()
                } else if entity.construction {
                    theme::construction()
                } else {
                    restrictions::entity_color(work, entity.id)
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
            }
        }
        restrictions::paint_vertices(&frame, &painter, display_sketch, work);
        // Cotas como cotas: extensões, linha com setas e valor (R/Ø, graus, parênteses).
        if work.primitive().is_some() {
            let points = work.references.points.centers.iter().copied().chain(
                sketch
                    .entities
                    .iter()
                    .filter_map(|e| sketch.radial(e.id).ok().map(|(c, _)| c))
                    .chain(
                        sketch
                            .entities
                            .iter()
                            .filter(|e| !e.infinite)
                            .filter_map(|e| sketch.line(e.id).ok())
                            .map(|(a, b)| nator_sketch::scale(nator_sketch::add(a, b), 0.5)),
                    ),
            );
            indicators::centers(&frame, &painter, points);
        }
        let operation_update = operation_preview::canvas(
            ui,
            response,
            &frame,
            &painter,
            &sketch,
            work,
            &parameters,
            projection_key.scene,
            hover,
            !busy && !animating,
        );
        let edit_existing = !operation_update.blocked
            && !image_blocked
            && !work.dragging.active()
            && !busy
            && !animating
            && (work.is_selecting()
                || (work.tool == Tool::Dimension && work.dimension_draft.is_empty()));
        let existing_update = dimension_tool::interact_existing(
            ui,
            response,
            &frame,
            &painter,
            text_preview.as_mut().unwrap_or(&mut sketch),
            &mut work.dimension_existing,
            edit_existing,
            &parameters,
        );
        if existing_update.blocked
            && (response.clicked_by(egui::PointerButton::Primary)
                || response.drag_started_by(egui::PointerButton::Primary))
        {
            work.selected.clear();
            work.selected_vertices.clear();
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
                    .unwrap_or_else(|| dimension_expression(&constraint.kind, value, settings));
                work.reference = constraint.reference;
            }
        }
        dimension_tool::paint_all(
            &frame,
            &painter,
            &sketch,
            work.dimension_existing.selected(),
        );
        let restrictions_enabled = !busy && !animating && !image_blocked;
        let supports = restrictions::supports(&sketch, work);
        work.selected_vertices.retain(|vertex| {
            supports
                .iter()
                .any(|support| nator_sketch::distance(*support, *vertex) < 1e-6)
        });
        let alert_update = if work.is_selecting() {
            restrictions::alerts(
                ui,
                &frame,
                &painter,
                &mut sketch,
                work,
                restrictions_enabled,
            )
        } else {
            restrictions::AlertUpdate::default()
        };
        submit |= alert_update.changed;
        submit |= restrictions::fix_key(ui, &mut sketch, work, restrictions_enabled);
        submit |= restrictions::context_menu(
            response,
            &frame,
            &mut sketch,
            work,
            &supports,
            8. / pixels_per_mm,
            restrictions_enabled,
        );
        let drag_update = direct_drag::canvas(
            ui,
            response,
            &frame,
            &painter,
            &sketch,
            work,
            projection_key.scene,
            !busy
                && !animating
                && !existing_update.blocked
                && !alert_update.blocked
                && !image_blocked,
        );

        let suspended = ui.input(|i| i.modifiers.alt);
        let tangent_selection = work.tool == Tool::Circle
            && work.circle_mode == CircleMode::Tangent
            && work.tangent_lines.len() < 2;
        let stepping = work.primitive().is_some() && !tangent_selection;
        let mut snap = hover.and_then(|point| {
            if suspended {
                None
            } else {
                let exact = snapping::find_with_edges(
                    &sketch,
                    point,
                    8. / pixels_per_mm,
                    &work.tracker,
                    &work.projection.points,
                    snapping::ProjectedEdges {
                        curves: &work.projection.curves,
                        chains: &work.projection.chains,
                    },
                    &work.references.points,
                )
                .ok()
                .flatten();
                // Vértice no raio da precisão prevalece: cliques quase coincidentes
                // ligam nele, em vez de deixar uma falsa conexão muito próxima. O início
                // do próprio traço fica de fora, porque daria um elemento de tamanho zero.
                let exact = if stepping
                    && !exact
                        .as_ref()
                        .is_some_and(|s| matches!(s.kind, Kind::Endpoint(_)))
                {
                    snapping::vertex_within(&sketch, point, settings.mouse_step_mm)
                        .ok()
                        .flatten()
                        .filter(|near| {
                            !work
                                .anchors
                                .iter()
                                .any(|a| nator_sketch::distance(*a, near.point) < 1e-9)
                        })
                        .or(exact)
                } else {
                    exact
                };
                // O primeiro ponto cai na precisão do mouse. Depois dele, cada medida
                // anda em passos a partir da âncora, sem passar por esta grade absoluta.
                if stepping && (work.anchors.is_empty() || exact.is_some()) {
                    snapping::with_step(exact, point, settings.mouse_step_mm)
                        .ok()
                        .flatten()
                } else {
                    exact
                }
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
        // Tangente a partir do início do traço. Pontos exatos continuam tendo prioridade;
        // a reta só encosta na curva, sem criar restrição com a geometria projetada.
        if work.primitive() == Some(Primitive::Line)
            && work.anchors.len() == 1
            && !suspended
            && !snap.as_ref().is_some_and(|s| {
                matches!(
                    s.kind,
                    Kind::Endpoint(_) | Kind::Center(_) | Kind::Midpoint(_) | Kind::Intersection
                )
            })
            && let Some(point) = hover
            && let Ok(Some(tangent)) = snapping::tangent(
                &sketch,
                work.anchors[0],
                snapping::ProjectedEdges {
                    curves: &work.projection.curves,
                    chains: &work.projection.chains,
                },
                point,
                8. / pixels_per_mm,
            )
        {
            snap = Some(tangent);
        }
        if hover.is_some() {
            // Apoios exatos e Alt têm precedência sobre a precisão do mouse.
            work.drawing_steps =
                (stepping && !suspended && snap.is_none()).then(|| settings.drafting_steps());
        }
        if work.primitive().is_some()
            && hover.is_some()
            && !busy
            && !animating
            && matches!(work.tool, Tool::Line | Tool::Rectangle)
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::I))
        {
            work.line_type = if work.line_type == LineType::Infinite {
                LineType::Normal
            } else {
                LineType::Infinite
            };
            work.drawing.infinite = work.line_type == LineType::Infinite;
            ui.input_mut(|i| {
                i.events.retain(
                    |event| !matches!(event, egui::Event::Text(s) if s.eq_ignore_ascii_case("i")),
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
                painter.circle_stroke(position, 5., egui::Stroke::new(1.5, theme::warning()));
            }
            for (a, b) in &snap.guides {
                frame.path(
                    &painter,
                    [*a, *b],
                    egui::Stroke::new(1., theme::green()),
                    true,
                );
            }
        }
        // Selecionar/aparar usa o clique real: a atração ao centro ou ao vértice
        // vizinho não pode deslocar o clique para outro trecho do cruzamento.
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
                egui::Stroke::new(3., theme::danger()),
                false,
            );
        }
        if work.tool == Tool::Circle && work.circle_mode == CircleMode::Tangent {
            for id in &work.tangent_lines {
                if let Ok((a, b)) = sketch.line(*id) {
                    frame.infinite_line(&painter, a, b, egui::Stroke::new(2., theme::green()));
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
                    theme::ink(),
                );
            }
        }
        // Linha que começou num círculo ou arco: com Shift, a origem desliza nele e a
        // linha fica tangente, passando pelo cursor.
        work.tangent_held = false;
        if work.tool == Tool::Line
            && work.line_mode == LineMode::Normal
            && work.anchors.len() == 1
            && let Some(curve) = work.tangent_origin
            && let Some(point) = raw_cursor
            && ui.input(|i| i.modifiers.shift)
            && let Ok(Some(origin)) =
                snapping::tangent_origin(&sketch, curve, point, work.anchors[0])
        {
            work.anchors[0] = origin;
            work.chain_points = vec![origin];
            work.tangent_held = true;
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
                work.drawing_steps,
                !busy && !animating,
            );
            confirm = output.confirm;
            escape = output.cancel;
            valid = output.preview.is_some();
            if let Some(preview) = output.preview {
                let center = match primitive {
                    Primitive::Circle | Primitive::Arc | Primitive::SymmetricLine => {
                        work.anchors.first().copied()
                    }
                    Primitive::Line | Primitive::DiameterCircle => work
                        .anchors
                        .first()
                        .map(|a| nator_sketch::scale(nator_sketch::add(*a, preview.point), 0.5)),
                    Primitive::TangentCircle => Some(preview.point),
                    Primitive::Rectangle
                    | Primitive::CenterRectangle
                    | Primitive::InclinedRectangle
                        if preview.path.len() == 5 =>
                    {
                        let points = preview.path[..4]
                            .iter()
                            .map(|p| [p[0], p[1], 0.])
                            .collect::<Vec<_>>();
                        nator_geometry::polygon_center(&points).map(|p| [p[0], p[1]])
                    }
                    _ => None,
                };
                indicators::centers(&frame, &painter, center);
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
                        theme::construction()
                    } else {
                        theme::ink()
                    },
                );
                if primitive == Primitive::DiameterCircle {
                    frame.path(
                        &painter,
                        [work.anchors[0], preview.point],
                        egui::Stroke::new(1., theme::ink()),
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
            Some(work.projector.update(
                ui,
                &frame,
                &painter,
                response.hover_pos(),
                !busy && response.clicked_by(egui::PointerButton::Primary),
                &sketch,
            ))
        } else {
            None
        };
        if projection_result.as_ref().is_some_and(|r| r.finished) {
            work.activate_selection(ui.ctx());
        }
        // Cada corte recalcula o rascunho em segundo plano. Sem esta fila, os cliques
        // feitos nesse intervalo se perdiam e o trecho pedia outro clique.
        if work.tool != Tool::Trim {
            work.pending_trims.clear();
        } else if busy {
            if response.clicked_by(egui::PointerButton::Primary)
                && let Some(point) = hover
            {
                work.pending_trims.push_back(point);
            }
        } else if !animating
            && let Some(point) = work.pending_trims.pop_front()
            && let Ok(Some(id)) =
                nator_sketch::tools::pick_entity(&sketch, point, 8. / pixels_per_mm)
            && nator_sketch::tools::trim(&mut sketch, id, point).is_ok()
        {
            submit = true;
        }
        if !busy
            && !animating
            && !escape
            && !alert_update.blocked
            && !existing_update.blocked
            && !operation_update.blocked
            && !drag_update.blocked
            && !image_blocked
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
                let corner_action = matches!(
                    work.active_action,
                    Some(EditAction::Fillet | EditAction::Chamfer)
                );
                let mut vertex_handled = false;
                // Com uma restrição à espera, cada clique acumula os elementos dela.
                let additive =
                    work.armed.is_some() || ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                if work.is_selecting() || corner_action {
                    let hit = junction_tool::vertex(&sketch, &supports, cursor, 8. / pixels_per_mm);
                    if work.is_selecting() {
                        match hit {
                            // Duplo clique fixa ou libera; cruzamentos não têm o que fixar.
                            Some(position)
                                if work.armed.is_none()
                                    && response.double_clicked_by(egui::PointerButton::Primary) =>
                            {
                                match nator_sketch::junctions::fixed(&sketch, position).and_then(
                                    |fixed| {
                                        nator_sketch::junctions::set_fixed(
                                            &mut sketch,
                                            position,
                                            !fixed,
                                        )
                                    },
                                ) {
                                    Ok(changed) => submit |= changed,
                                    Err(error) => {
                                        painter.text(
                                            response.rect.left_top(),
                                            egui::Align2::LEFT_TOP,
                                            error.to_string(),
                                            egui::FontId::proportional(13.),
                                            theme::danger(),
                                        );
                                    }
                                }
                                vertex_handled = true;
                            }
                            // O vértice é escolhido sozinho, sem as arestas que chegam nele.
                            Some(position) => {
                                restrictions::toggle_vertex(work, position, additive);
                                work.constraint = None;
                                vertex_handled = true;
                            }
                            None if !additive => work.selected_vertices.clear(),
                            None => {}
                        }
                    }
                    // Arredondar/Chanfrar aceita o vértice: só os cantos clicados mudam,
                    // mesmo que as linhas deles formem outros cantos entre si.
                    if corner_action && let Some(position) = hit {
                        let lines = junction_tool::corner_lines(&sketch, position);
                        if lines.len() >= 2 {
                            work.toggle_corner(&sketch, position);
                            work.constraint = None;
                            vertex_handled = true;
                        }
                    }
                }
                let entity = if vertex_handled {
                    None
                } else {
                    nator_sketch::tools::pick_entity(&sketch, cursor, 8. / pixels_per_mm)
                        .ok()
                        .flatten()
                };
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
                                    theme::danger(),
                                );
                            }
                        }
                    } else {
                        if work.is_selecting()
                            && work.armed.is_none()
                            && response.double_clicked_by(egui::PointerButton::Primary)
                        {
                            if let Ok(connected) = nator_sketch::connected::entities(&sketch, &[id])
                            {
                                work.selected = connected;
                            }
                        } else if work.active_action == Some(EditAction::Mirror)
                            && work.mirror_axis_active
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
                            // Clicar numa linha volta à seleção de cantos pelas linhas.
                            work.operation.corners.clear();
                            if let Some(index) = work.selected.iter().position(|e| *e == id) {
                                work.selected.remove(index);
                            } else {
                                work.selected.push(id);
                            }
                        } else if !additive {
                            work.selected.clear();
                            work.selected_vertices.clear();
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
                            work.dimension_expression = settings.unit.expression(work.dimension);
                        }
                    }
                } else {
                    if work.active_action.is_none() && !additive {
                        work.selected.clear();
                    }
                }
                submit |= restrictions::armed_step(&mut sketch, work);
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
                        theme::danger(),
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
                        work.tangent_origin = snap.as_ref().and_then(|s| match s.kind {
                            Kind::OnCurve(id)
                                if sketch.entity(id).is_ok_and(|e| {
                                    matches!(e.curve, Curve::Circle { .. } | Curve::Arc { .. })
                                }) =>
                            {
                                Some(id)
                            }
                            _ => None,
                        });
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
                                theme::danger(),
                            );
                            work.anchors.pop();
                            work.anchor_kinds.pop();
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
            // O tipo de captura acompanha o cursor; nada fica marcado no desenho.
            if let Some(found) = &snap
                && work.primitive().is_some()
                && let Some(icon) = work.projection.snap_icon(found.kind, found.point)
            {
                icons::paint(
                    &painter,
                    egui::Rect::from_min_size(
                        position + egui::vec2(16., 4.),
                        egui::Vec2::splat(20.),
                    ),
                    icon,
                    true,
                );
            }
        }
        if let Some((candidate, mode)) = drag_update.commit.or(operation_update.commit) {
            self.solve_sketch_with(ui.ctx(), candidate, mode);
        } else if let Some(id) = existing_update.changed {
            self.solve_sketch_with(ui.ctx(), sketch, SolveMode::Dimension(id, false));
        } else if let Some(candidate) = projection_result.as_ref().and_then(|r| r.commit.clone()) {
            self.solve_sketch(ui.ctx(), candidate);
        } else if submit {
            self.solve_sketch(ui.ctx(), sketch);
        }
        if let Some(error) = existing_update.warning {
            self.status.warning(error);
        }
        if let Some(error) = operation_update.warning {
            self.status.warning(error);
        }
        if let Some(error) = drag_update.warning {
            self.status.warning(error);
        }
        if let Some(error) = projection_result.and_then(|r| r.warning) {
            self.status.warning(error);
        }
        if let Some(error) = alert_update.warning {
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
                self.solve_sketch_with(ui.ctx(), candidate, SolveMode::Dimension(id, true));
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
                attach(sketch, start, work.anchor_kinds[0], a)?;
                attach(sketch, end, work.anchor_kinds[1], b)?;
                if work.tangent_held
                    && let Some(curve) = work.tangent_origin
                    && let Curve::Circle { center, .. } | Curve::Arc { center, .. } =
                        sketch.entity(curve).map_err(sketch_error)?.curve
                {
                    // A origem fica presa à curva e a linha, tangente a ela.
                    for kind in [
                        ConstraintKind::PointOnCurve {
                            point: start,
                            curve,
                        },
                        ConstraintKind::Tangent {
                            line: id,
                            arc: curve,
                            point: center,
                        },
                    ] {
                        sketch.add_constraint(kind, false).map_err(sketch_error)?;
                    }
                }
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
                        work.length_field(0),
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
                attach(sketch, point_ids[0], work.anchor_kinds[0], a)?;
            }
            let last = if inclined { 2 } else { 1 };
            attach(sketch, point_ids[2], work.anchor_kinds[last], corners[2])?;
            if inclined {
                attach(sketch, point_ids[1], work.anchor_kinds[1], b)?;
                if let Some((fields, locked)) = &work.drawing.side_fields {
                    if locked[0] && work.line_type != LineType::Infinite {
                        drive(
                            sketch,
                            ConstraintKind::Distance {
                                a: point_ids[0],
                                b: point_ids[1],
                                value: nator_sketch::distance(a, b),
                            },
                            work.settings.unit.normalize_expression(&fields[0]),
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
                        work.length_field(0),
                    )?;
                }
            } else {
                for (index, end) in [(0, 1), (1, 3)] {
                    if work.drawing.locked[index] && work.line_type != LineType::Infinite {
                        let value = corners[2][index] - corners[0][index];
                        let expression = if value < 0. {
                            format!("-({})", work.length_field(index))
                        } else {
                            work.length_field(index)
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
                    attach(sketch, point, work.anchor_kinds[0], a)?;
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
                    format!("({}) / 2", work.length_field(0))
                } else {
                    work.length_field(0)
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
            let clockwise = work.drawing.arc_sweep.value() < 0.;
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
            if nator_sketch::distance(
                sketch.point(start).map_err(sketch_error)?,
                sketch.point(end).map_err(sketch_error)?,
            ) < 1e-9
            {
                return Err(text(T::InvalidArc).into());
            }
            let id = sketch
                .add_curve(
                    Curve::Arc {
                        center,
                        start,
                        end,
                        clockwise,
                    },
                    work.line_type != LineType::Normal,
                )
                .map_err(sketch_error)?;
            attach(sketch, center, work.anchor_kinds[0], a)?;
            // A ponta inicial só mantém a captura se ainda estiver no raio original.
            attach(
                sketch,
                start,
                work.anchor_kinds[1].filter(|_| !work.drawing.locked[0]),
                b,
            )?;
            if work.drawing.locked[0] {
                drive(
                    sketch,
                    ConstraintKind::Radius {
                        circle: id,
                        value: radius,
                    },
                    work.length_field(0),
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
                        "({start_expression}) {} ({})",
                        if clockwise { "-" } else { "+" },
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
fn dimension_expression(
    kind: &ConstraintKind,
    value: f64,
    settings: nator_document::Settings,
) -> String {
    if kind.is_angular() {
        format!("{} deg", value.to_degrees())
    } else {
        settings.unit.expression(value)
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
fn constraint_label(kind: &ConstraintKind, settings: nator_document::Settings) -> String {
    let key = match kind {
        ConstraintKind::Fixed { position, .. } if *position == nator_sketch::restrict::ORIGIN => {
            T::OriginCoincidence
        }
        ConstraintKind::OriginOnCurve { .. } => T::OriginCoincidence,
        ConstraintKind::Fixed { .. } => T::Fixed,
        ConstraintKind::Coincident { .. } => T::Coincident,
        ConstraintKind::Horizontal { .. } => T::Horizontal,
        ConstraintKind::Vertical { .. } => T::Vertical,
        ConstraintKind::OriginDistance { axis: Some(0), .. } => T::HorizontalDimension,
        ConstraintKind::OriginDistance { axis: Some(1), .. } => T::VerticalDimension,
        ConstraintKind::OriginDistance { .. } => T::Dimension,
        ConstraintKind::OriginLineDistance { .. } => T::PointLineDistance,
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
        ConstraintKind::Tangent { .. } | ConstraintKind::CircleTangent { .. } => T::Tangent,
        ConstraintKind::PointsHorizontal { .. } => T::Horizontal,
        ConstraintKind::PointsVertical { .. } => T::Vertical,
        ConstraintKind::Collinear { .. } => T::Collinear,
        ConstraintKind::Symmetric { .. } => T::Symmetry,
        ConstraintKind::EqualRadius { .. } => T::EqualRestriction,
    };
    match dimension_value(kind) {
        Some(value) if kind.is_angular() => {
            format!(
                "{}: {}°",
                text(key),
                nator_drawing::format_mm(value.to_degrees())
            )
        }
        Some(value) => format!("{}: {}", text(key), settings.length_label(value)),
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
