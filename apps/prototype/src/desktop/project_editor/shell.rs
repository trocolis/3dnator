//! Estrutura visual inspirada na organização por tarefas de um CAD desktop.
//! Este módulo só escolhe ferramentas e apresenta o estado: alterações continuam
//! passando pelos comandos e pelos trabalhadores já existentes no editor.
use super::{
    icons::{self, Icon},
    *,
};
use feature_editor::Tool as SolidTool;
use nator_sketch::edit::Action as SketchEdit;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) enum RibbonTab {
    /// Aba principal: o projeto começa pela montagem.
    #[default]
    Assembly,
    Model,
    Inspect,
    Sketch,
}
impl RibbonTab {
    fn label(self) -> T {
        match self {
            Self::Model => T::ModelTab,
            Self::Assembly => T::Assembly,
            Self::Inspect => T::InspectTab,
            Self::Sketch => T::Sketch,
        }
    }
}

#[derive(Default)]
pub(super) struct ShellState {
    pub tab: RibbonTab,
    pub browser_visible: bool,
    pub measure_open: bool,
    pub search_open: bool,
    query: String,
    pub search_focus: bool,
    /// A aba contextual muda ao entrar/sair do rascunho, sem impedir que o usuário
    /// consulte as demais ferramentas enquanto a edição 2D estiver aberta.
    sketch_context: Option<(u64, SketchId)>,
    /// Aba ativa antes do rascunho, restaurada ao concluí-lo.
    before_sketch: Option<RibbonTab>,
    /// Botões de menu da faixa no último quadro (capturas e ensaios de eventos).
    pub menu_rects: Vec<egui::Rect>,
}
impl ShellState {
    pub fn new() -> Self {
        Self {
            browser_visible: true,
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    New,
    Open,
    Save,
    SaveAs,
    Undo,
    Redo,
    Import,
    Reference,
    NewSketch,
    SketchXY,
    SketchXZ,
    SketchYZ,
    SketchFace,
    Solid(SolidTool),
    MoveBody,
    CopyBody,
    MoveCopy,
    Align,
    NewInstance,
    CloneInstance,
    InstancePattern(bool),
    InstanceMirror,
    Group,
    Ungroup,
    Section,
    InsertPart,
    ReferenceSketch,
    NewEmptyPart,
    NewPartStep,
    NewPartStl,
    MoveFace,
    MoveEdge,
    MovePoint,
    Parameters,
    Measure,
    Drawing,
    ExportStl,
    ExportStep,
    Fit,
    Iso,
    Front,
    Top,
    Side,
    Perspective,
    Shadows,
    Triangles,
    Browser,
    SketchTool(sketch_editor::Tool),
    SketchEdit(SketchEdit),
    FinishSketch,
}
impl Action {
    fn label(self) -> T {
        match self {
            Self::New => T::NewProject,
            Self::Open => T::OpenProject,
            Self::Save => T::Save,
            Self::SaveAs => T::SaveAs,
            Self::Undo => T::Undo,
            Self::Redo => T::Redo,
            Self::Import => T::Import,
            Self::Reference => T::Reference,
            Self::NewSketch => T::NewSketch,
            Self::SketchXY => T::SketchXY,
            Self::SketchXZ => T::SketchXZ,
            Self::SketchYZ => T::SketchYZ,
            Self::SketchFace => T::SketchOnFace,
            Self::Solid(tool) => tool.label(),
            Self::MoveBody => T::MoveMenu,
            Self::CopyBody => T::CopyMenu,
            Self::MoveCopy => T::MoveCopyTool,
            Self::Align => T::AlignTool,
            Self::NewInstance => T::NewInstanceTool,
            Self::CloneInstance => T::CloneTool,
            Self::InstancePattern(false) => T::RectangularInstancePattern,
            Self::InstancePattern(true) => T::CircularInstancePattern,
            Self::InstanceMirror => T::MirrorComponent,
            Self::Group => T::GroupTool,
            Self::Ungroup => T::UngroupTool,
            Self::Section => T::SectionAnalysis,
            Self::InsertPart => T::InsertFromProject,
            Self::ReferenceSketch => T::ReferenceSketch,
            Self::NewEmptyPart => T::NewEmptyPart,
            Self::NewPartStep => T::NewPartFromStep,
            Self::NewPartStl => T::NewPartFromStl,
            Self::MoveFace => T::MoveFaceToFace,
            Self::MoveEdge => T::MoveEdgeToEdge,
            Self::MovePoint => T::MovePointToPoint,
            Self::Parameters => T::Parameters,
            Self::Measure => T::MeasureSelection,
            Self::Drawing => T::Drawing,
            Self::ExportStl => T::ExportStl,
            Self::ExportStep => T::ExportAssemblyStep,
            Self::Fit => T::Fit,
            Self::Iso => T::Iso,
            Self::Front => T::Front,
            Self::Top => T::Top,
            Self::Side => T::Side,
            Self::Perspective => T::Perspective,
            Self::Shadows => T::Shadows,
            Self::Triangles => T::Triangulation,
            Self::Browser => T::Project,
            Self::FinishSketch => T::FinishSketch,
            Self::SketchTool(tool) => tool.label(),
            Self::SketchEdit(action) => sketch_editor::edit_label(action),
        }
    }
    fn caption(self) -> T {
        match self {
            Self::Solid(SolidTool::Fillet) => T::FilletShort,
            Self::Solid(SolidTool::Mirror) => T::MirrorShort,
            Self::Solid(SolidTool::Split) => T::SplitShort,
            Self::SketchTool(sketch_editor::Tool::Arc) => T::ArcShort,
            Self::SketchEdit(SketchEdit::CircularPattern) => T::SketchCircularShort,
            Self::SketchEdit(SketchEdit::RectangularPattern) => T::SketchRectangularShort,
            Self::InstancePattern(false) => T::RectangularPattern,
            Self::InstancePattern(true) => T::CircularPattern,
            Self::InstanceMirror => T::MirrorShort,
            Self::ExportStep => T::StepShort,
            Self::SketchFace => T::SketchFaceShort,
            _ => self.label(),
        }
    }
    fn icon(self) -> Icon {
        match self {
            Self::New => Icon::New,
            Self::Open => Icon::Open,
            Self::Save => Icon::Save,
            Self::SaveAs => Icon::SaveAs,
            Self::Undo => Icon::Undo,
            Self::Redo => Icon::Redo,
            Self::Import => Icon::Import,
            Self::Reference => Icon::Cube,
            Self::NewSketch
            | Self::SketchXY
            | Self::SketchXZ
            | Self::SketchYZ
            | Self::SketchFace => Icon::Sketch,
            Self::Solid(tool) => match tool {
                SolidTool::Extrude => Icon::Extrude,
                SolidTool::Revolve => Icon::Revolve,
                SolidTool::Hole => Icon::Hole,
                SolidTool::Fillet => Icon::Fillet,
                SolidTool::Chamfer => Icon::Chamfer,
                SolidTool::Shell => Icon::Shell,
                SolidTool::Mirror => Icon::Mirror,
                SolidTool::LinearPattern | SolidTool::CircularPattern => Icon::Pattern,
                SolidTool::Split => Icon::Split,
                SolidTool::Join | SolidTool::Cut | SolidTool::Intersect => Icon::Boolean,
            },
            Self::MoveCopy | Self::MoveBody => Icon::Move,
            Self::CopyBody => Icon::Copy,
            Self::Align => Icon::Align,
            Self::NewInstance => Icon::NewBody,
            Self::CloneInstance => Icon::Boolean,
            Self::InstancePattern(_) => Icon::Pattern,
            Self::InstanceMirror => Icon::Mirror,
            Self::Group | Self::Ungroup => Icon::Group,
            Self::Section => Icon::Section,
            Self::InsertPart => Icon::Import,
            Self::ReferenceSketch => Icon::ReferenceSketch,
            Self::NewEmptyPart => Icon::NewBody,
            Self::NewPartStep => Icon::StepFile,
            Self::NewPartStl => Icon::StlFile,
            Self::MoveFace => Icon::FaceToFace,
            Self::MoveEdge => Icon::EdgeToEdge,
            Self::MovePoint => Icon::PointToPoint,
            Self::Parameters => Icon::Parameters,
            Self::Measure => Icon::Measure,
            Self::Drawing => Icon::Drawing,
            Self::ExportStl | Self::ExportStep => Icon::Export,
            Self::Fit => Icon::Fit,
            Self::Iso => Icon::Cube,
            Self::Front => Icon::Front,
            Self::Top => Icon::Top,
            Self::Side => Icon::Side,
            Self::Perspective => Icon::Perspective,
            Self::Shadows => Icon::Eye,
            Self::Triangles => Icon::Pattern,
            Self::Browser => Icon::Folder,
            Self::FinishSketch => Icon::Check,
            Self::SketchTool(tool) => tool.icon(),
            Self::SketchEdit(action) => sketch_editor::edit_icon(action),
        }
    }
    fn shortcut(self) -> Option<&'static str> {
        match self {
            Self::Save => Some("Ctrl+S"),
            Self::Undo => Some("Ctrl+Z"),
            Self::Redo => Some("Ctrl+Shift+Z"),
            _ => None,
        }
    }
}

/// Botão de menu da faixa: ícone e nome principais e as subopções (ação, nome no menu e
/// explicação).
struct Menu<'a> {
    icon: Icon,
    label: T,
    items: &'a [(Action, T, T)],
}

/// As abas compartilham as opções de alinhamento; Livre muda apenas o alvo:
/// corpos na Peça e componentes na Montagem.
fn move_menu_items(free: Action) -> [(Action, T, T); 4] {
    [
        (free, T::MoveFree, T::MoveFreeHint),
        (Action::MoveFace, T::MoveFaceToFace, T::MoveFaceHint),
        (Action::MoveEdge, T::MoveEdgeToEdge, T::MoveEdgeHint),
        (Action::MovePoint, T::MovePointToPoint, T::MovePointHint),
    ]
}

/// Também alimenta a busca: nenhuma entrada anuncia uma ferramenta sem implementação.
const SEARCH_ACTIONS: &[Action] = &[
    Action::New,
    Action::Open,
    Action::Save,
    Action::SaveAs,
    Action::Undo,
    Action::Redo,
    Action::Import,
    Action::Reference,
    Action::NewSketch,
    Action::SketchXY,
    Action::SketchXZ,
    Action::SketchYZ,
    Action::SketchFace,
    Action::Solid(SolidTool::Extrude),
    Action::Solid(SolidTool::Revolve),
    Action::Solid(SolidTool::Hole),
    Action::Solid(SolidTool::Fillet),
    Action::Solid(SolidTool::Chamfer),
    Action::Solid(SolidTool::Shell),
    Action::Solid(SolidTool::Join),
    Action::Solid(SolidTool::Cut),
    Action::Solid(SolidTool::Intersect),
    Action::Solid(SolidTool::LinearPattern),
    Action::Solid(SolidTool::CircularPattern),
    Action::Solid(SolidTool::Mirror),
    Action::Solid(SolidTool::Split),
    Action::MoveCopy,
    Action::MoveBody,
    Action::CopyBody,
    Action::Align,
    Action::NewInstance,
    Action::CloneInstance,
    Action::InstancePattern(false),
    Action::InstancePattern(true),
    Action::InstanceMirror,
    Action::Group,
    Action::Ungroup,
    Action::Section,
    Action::InsertPart,
    Action::ReferenceSketch,
    Action::NewEmptyPart,
    Action::NewPartStep,
    Action::NewPartStl,
    Action::MoveFace,
    Action::MoveEdge,
    Action::MovePoint,
    Action::Parameters,
    Action::Drawing,
    Action::Measure,
    Action::ExportStl,
    Action::ExportStep,
    Action::Fit,
    Action::Iso,
    Action::Front,
    Action::Top,
    Action::Side,
    Action::Perspective,
    Action::Shadows,
    Action::Triangles,
    Action::Browser,
    Action::SketchTool(sketch_editor::Tool::Select),
    Action::SketchTool(sketch_editor::Tool::Line),
    Action::SketchTool(sketch_editor::Tool::Rectangle),
    Action::SketchTool(sketch_editor::Tool::Circle),
    Action::SketchTool(sketch_editor::Tool::Arc),
    Action::SketchTool(sketch_editor::Tool::Trim),
    Action::SketchEdit(SketchEdit::Offset),
    Action::SketchEdit(SketchEdit::Fillet),
    Action::SketchEdit(SketchEdit::Chamfer),
    Action::SketchEdit(SketchEdit::Extend),
    Action::SketchEdit(SketchEdit::Mirror),
    Action::SketchEdit(SketchEdit::CircularPattern),
    Action::SketchEdit(SketchEdit::RectangularPattern),
    Action::SketchEdit(SketchEdit::Dimension),
    Action::SketchEdit(SketchEdit::HorizontalVertical),
    Action::SketchEdit(SketchEdit::Coincident),
    Action::SketchEdit(SketchEdit::Tangent),
    Action::SketchEdit(SketchEdit::Equal),
    Action::SketchEdit(SketchEdit::Parallel),
    Action::SketchEdit(SketchEdit::Perpendicular),
    Action::SketchEdit(SketchEdit::Construction),
    Action::SketchEdit(SketchEdit::FixPoint),
    Action::SketchEdit(SketchEdit::Delete),
    Action::FinishSketch,
];

impl Editor {
    fn action_enabled(&self, action: Action) -> bool {
        let editable = !self.busy() && self.pending_close.is_none();
        match action {
            Action::Fit => true,
            Action::Iso
            | Action::Front
            | Action::Top
            | Action::Side
            | Action::Perspective
            | Action::Shadows
            | Action::Triangles => true,
            Action::Browser => true,
            Action::Undo => editable && self.current().session.undo_label().is_some(),
            Action::Redo => editable && self.current().session.redo_label().is_some(),
            Action::Solid(_) => {
                editable
                    && self.current().active_part.is_some()
                    && nator_kernel::is_available()
                    && !self
                        .current()
                        .sketch_edit
                        .as_ref()
                        .is_some_and(|w| w.pending.is_some())
            }
            Action::Parameters => editable && self.current().active_part.is_some(),
            Action::NewInstance
            | Action::CloneInstance
            | Action::Group
            | Action::InstancePattern(_)
            | Action::InstanceMirror => {
                editable
                    && self.current().sketch_edit.is_none()
                    && (self
                        .current()
                        .selected
                        .iter()
                        .any(|s| matches!(s, Selection::Instance(_)))
                        || matches!(action, Action::InstancePattern(_) | Action::InstanceMirror))
            }
            Action::Ungroup => {
                editable
                    && self.current().session.document().groups().iter().any(|g| {
                        g.members
                            .iter()
                            .any(|m| self.current().is_selected(Selection::Instance(*m)))
                    })
            }
            Action::MoveCopy | Action::Align => editable && self.current().sketch_edit.is_none(),
            Action::MoveBody | Action::CopyBody => {
                editable
                    && self.current().sketch_edit.is_none()
                    && self
                        .current()
                        .active_part
                        .and_then(|id| self.current().session.document().part(id))
                        .is_some_and(|p| p.listed_bodies().next().is_some())
            }
            Action::InsertPart => {
                editable && self.current().sketch_edit.is_none() && self.tabs.len() > 1
            }
            Action::ReferenceSketch
            | Action::NewEmptyPart
            | Action::NewPartStl
            | Action::MoveFace
            | Action::MoveEdge
            | Action::MovePoint => editable && self.current().sketch_edit.is_none(),
            Action::NewPartStep => {
                editable && self.current().sketch_edit.is_none() && nator_kernel::is_available()
            }
            Action::NewSketch | Action::SketchXY | Action::SketchXZ | Action::SketchYZ => {
                editable && self.current().sketch_edit.is_none()
            }
            Action::SketchFace => {
                editable && self.current().picked.is_some() && self.current().sketch_edit.is_none()
            }
            Action::Measure => editable && self.current().picked.is_some(),
            Action::SketchTool(_) | Action::SketchEdit(_) => {
                editable && self.current().sketch_edit.is_some()
            }
            Action::FinishSketch => {
                editable
                    && self
                        .current()
                        .sketch_edit
                        .as_ref()
                        .is_some_and(|w| w.pending.is_none())
            }
            Action::ExportStep => editable && nator_kernel::is_available(),
            _ => editable,
        }
    }
    fn action_selected(&self, action: Action) -> bool {
        match action {
            Action::Perspective => {
                self.viewport.camera.projection == nator_renderer::Projection::Perspective
            }
            Action::Shadows => self.viewport.show_shadows,
            Action::Triangles => self.viewport.show_triangles,
            Action::Browser => self.shell.browser_visible,
            Action::Section => self.current().section.is_some_and(|s| s.visible),
            Action::SketchTool(tool) => self
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.tool() == tool),
            Action::SketchEdit(action) => self
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.active_action() == Some(action)),
            _ => false,
        }
    }
    fn action_button(&mut self, ui: &mut egui::Ui, action: Action, large: bool) {
        let enabled = self.action_enabled(action);
        let selected = self.action_selected(action);
        let response = ui
            .add_enabled_ui(enabled, |ui| {
                icons::button(
                    ui,
                    action.icon(),
                    text(if large {
                        action.caption()
                    } else {
                        action.label()
                    }),
                    large,
                    selected,
                )
            })
            .inner;
        let hint = if let Some(shortcut) = action.shortcut() {
            format!("{} · {shortcut}", text(action.label()))
        } else {
            text(action.label()).to_owned()
        };
        let response = response.on_hover_text(hint);
        if !enabled && matches!(action, Action::Solid(_) | Action::Parameters) {
            response
                .clone()
                .on_disabled_hover_text(text(if nator_kernel::is_available() {
                    T::ChooseActivePart
                } else {
                    T::KernelUnavailable
                }));
        }
        if response.clicked() {
            self.activate_action(ui.ctx(), action);
        }
    }
    pub(super) fn fit_view(&mut self, context: &egui::Context) {
        self.activate_action(context, Action::Fit);
    }
    fn activate_action(&mut self, context: &egui::Context, action: Action) {
        if !self.action_enabled(action) {
            return;
        }
        if matches!(
            action,
            Action::Fit
                | Action::Iso
                | Action::Front
                | Action::Top
                | Action::Side
                | Action::Perspective
        ) {
            self.current_mut().camera_transition = None;
        }
        match action {
            Action::New => {
                if let Err(error) = Document::new(text(T::Untitled))
                    .map_err(|e| e.to_string())
                    .and_then(|d| self.add_tab(d, None, None))
                {
                    self.status.error(error);
                }
            }
            Action::Open => self.open(context, None),
            Action::Save | Action::SaveAs => {
                self.save(context, self.current().id, action == Action::SaveAs, false)
            }
            Action::Undo | Action::Redo => {
                let changed = if action == Action::Undo {
                    self.current_mut().session.undo()
                } else {
                    self.current_mut().session.redo()
                };
                if changed {
                    self.current_mut().changed();
                    self.invalidate_scene();
                }
            }
            Action::Import => self.import(context, ImportSource::Any, self.current().active_part),
            Action::Reference => {
                self.import(context, ImportSource::Reference, self.current().active_part)
            }
            Action::ReferenceSketch => self.start_reference_sketch_plane(),
            Action::NewEmptyPart => {
                let count = self.current().session.document().parts().len() + 1;
                self.execute(Command::CreatePart {
                    name: format!("{} {count}", text(T::NewPartName)),
                });
            }
            Action::NewPartStep => self.import(context, ImportSource::Step, None),
            Action::NewPartStl => self.import(context, ImportSource::Stl, None),
            Action::MoveFace => self.open_align(navigation::PickMode::Face),
            Action::MoveEdge => self.open_align(navigation::PickMode::Edge),
            Action::MovePoint => self.open_align(navigation::PickMode::Vertex),
            Action::NewSketch => self.start_sketch_plane(),
            Action::SketchXY | Action::SketchXZ | Action::SketchYZ => {
                let plane = match action {
                    Action::SketchXZ => nator_sketch::Plane::xz(),
                    Action::SketchYZ => nator_sketch::Plane::yz(),
                    _ => nator_sketch::Plane::xy(),
                };
                self.execute(Command::AddSketch {
                    destination: self.current().active_part,
                    sketch: nator_sketch::Sketch::new(text(T::Sketch), plane),
                });
            }
            Action::SketchFace => self.sketch_on_selected_face(context),
            Action::Solid(tool) => self.open_feature_tool(tool),
            Action::MoveCopy => self.open_move(),
            Action::MoveBody => self.open_body_move(false),
            Action::CopyBody => self.open_body_move(true),
            Action::Align => self.open_align(navigation::PickMode::Face),
            Action::NewInstance => self.duplicate_selection(context),
            Action::CloneInstance => self.clone_selection(context),
            Action::InstancePattern(circular) => self.open_instance_pattern(circular),
            Action::InstanceMirror => self.open_instance_mirror(),
            Action::Group => self.group_selection(context),
            Action::Section => self.open_section(),
            Action::InsertPart => self.open_insert_part(),
            Action::Ungroup => self.ungroup_selection(context),
            Action::Parameters => self.open_parameters(),
            Action::Measure => {
                self.shell.measure_open = true;
                self.measure_report.clear();
                self.measure_current_selection(context);
            }
            Action::Drawing => self.open_drawing(None),
            Action::ExportStl => self.open_export(),
            Action::ExportStep => self.export_step(
                context,
                self.current().id,
                nator_commands::export::StepScope::Assembly,
            ),
            Action::Fit => {
                if let Some(work) = &mut self.current_mut().sketch_edit {
                    work.fit_requested = true;
                    return;
                }
                let [w, h] = self.viewport.size();
                self.viewport.camera.fit(f64::from(w) / f64::from(h));
            }
            Action::Iso => self
                .viewport
                .camera
                .set_direction(Camera::default().direction()),
            Action::Front => {
                self.viewport.camera.set_direction([0., -1., 0.]);
            }
            Action::Top => {
                self.viewport.camera.set_direction([0., 0., 1.]);
            }
            Action::Side => {
                self.viewport.camera.set_direction([1., 0., 0.]);
            }
            Action::Perspective => {
                self.viewport.camera.projection = match self.viewport.camera.projection {
                    nator_renderer::Projection::Orthographic => {
                        nator_renderer::Projection::Perspective
                    }
                    nator_renderer::Projection::Perspective => {
                        nator_renderer::Projection::Orthographic
                    }
                };
            }
            Action::Shadows => self.viewport.show_shadows = !self.viewport.show_shadows,
            Action::Triangles => self.viewport.show_triangles = !self.viewport.show_triangles,
            Action::Browser => self.shell.browser_visible = !self.shell.browser_visible,
            Action::SketchTool(tool) => {
                if let Some(work) = &mut self.current_mut().sketch_edit {
                    work.select_tool(tool);
                }
            }
            Action::SketchEdit(action) => self.open_sketch_action(context, action),
            Action::FinishSketch => self.finish_sketch(),
        }
    }
    fn sketch_on_selected_face(&mut self, context: &egui::Context) {
        let Some(pick) = self.current().picked.clone() else {
            return;
        };
        if self.current().active_part.is_none() {
            self.current_mut().active_part = Some(pick.part);
        }
        self.start_sketch_plane();
        self.choose_picked_plane(context, pick);
    }
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        let sketch = self
            .current()
            .sketch_edit
            .as_ref()
            .map(|w| (self.current().id, w.id));
        if sketch != self.shell.sketch_context {
            if sketch.is_some() {
                if self.shell.tab != RibbonTab::Sketch {
                    self.shell.before_sketch = Some(self.shell.tab);
                }
                self.shell.tab = RibbonTab::Sketch;
            } else {
                // Volta à aba de origem (Montagem para rascunhos de referência).
                self.shell.tab = self.shell.before_sketch.take().unwrap_or_default();
            }
            self.shell.sketch_context = sketch;
        }
        egui::Panel::top("application_bar")
            .frame(
                egui::Frame::new()
                    .fill(theme::HEADER)
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| {
                ui.set_min_height(32.);
                let style = ui.style_mut();
                style.visuals = egui::Visuals::dark();
                style.visuals.override_text_color = Some(egui::Color32::from_rgb(231, 237, 246));
                style.spacing.item_spacing.x = 5.;
                let drag_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(ui.available_width(), 32.),
                );
                window_frame::drag_region(ui, drag_rect);
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(25., 27.), egui::Sense::hover());
                    icons::paint(ui.painter(), rect, Icon::Cube, true);
                    ui.label(egui::RichText::new("3dnator").strong().size(18.));
                    ui.separator();
                    ui.menu_button(text(T::FileMenu), |ui| {
                        for action in [
                            Action::New,
                            Action::Open,
                            Action::Save,
                            Action::SaveAs,
                            Action::Import,
                            Action::ExportStl,
                            Action::ExportStep,
                        ] {
                            if ui
                                .add_enabled(
                                    self.action_enabled(action),
                                    egui::Button::new(text(action.label())),
                                )
                                .clicked()
                            {
                                self.activate_action(ui.ctx(), action);
                                ui.close();
                            }
                        }
                        ui.separator();
                        if ui
                            .add_enabled(
                                !self.recoveries.is_empty(),
                                egui::Button::new(text(T::Recovery)),
                            )
                            .clicked()
                        {
                            self.show_recovery = true;
                            ui.close();
                        }
                        ui.menu_button(text(T::DisplayDecimals), |ui| {
                            let mut decimals = self
                                .current()
                                .session
                                .document()
                                .settings()
                                .display_decimals;
                            if ui
                                .add_enabled(
                                    !self.busy(),
                                    egui::DragValue::new(&mut decimals)
                                        .range(0..=nator_document::MAX_DISPLAY_DECIMALS),
                                )
                                .changed()
                            {
                                self.execute(Command::SetSettings {
                                    settings: nator_document::Settings {
                                        display_decimals: decimals,
                                    },
                                });
                            }
                            ui.label(text(T::DisplayDecimalsHint));
                        });
                    });
                    self.action_button(ui, Action::Save, false);
                    ui.separator();
                    let width =
                        (ui.available_width() - 90. - window_frame::CONTROLS_WIDTH).max(110.);
                    let (tabs_rect, _) =
                        ui.allocate_exact_size(egui::vec2(width, 32.), egui::Sense::hover());
                    // O espaço reservado precisa permanecer ocupado mesmo quando há
                    // poucas abas. Um escopo que avançasse o cursor pelo conteúdo
                    // reduziria a faixa e afastaria os controles da borda direita.
                    let mut tabs_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(tabs_rect)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    self.document_tabs(&mut tabs_ui);
                    self.action_button(ui, Action::New, false);
                    if icons::button(ui, Icon::Search, text(T::SearchTools), false, false).clicked()
                    {
                        self.shell.search_open = true;
                        self.shell.search_focus = true;
                    }
                    ui.separator();
                    if window_frame::controls(ui).close.clicked() {
                        // Percorrer o mesmo fluxo de fechamento usado pela decoração
                        // nativa: aguardar a tarefa e oferecer Salvar/Descartar/Cancelar.
                        self.want_exit = true;
                        ui.ctx().request_repaint();
                    }
                });
            });
        egui::Panel::top("ribbon")
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::symmetric(12, 5))
                    .stroke(egui::Stroke::new(1., theme::BORDER)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.;
                    let mut previous_selected = None;
                    for tab in [
                        RibbonTab::Assembly,
                        RibbonTab::Model,
                        RibbonTab::Inspect,
                        RibbonTab::Sketch,
                    ] {
                        if tab == RibbonTab::Sketch && sketch.is_none() {
                            continue;
                        }
                        let selected = self.shell.tab == tab;
                        // A aba ativa já tem seu próprio contorno. O divisor aparece
                        // apenas entre duas abas inativas, sem duplicar essa borda.
                        if previous_selected == Some(false) && !selected {
                            ui.label(egui::RichText::new("|").color(theme::MUTED));
                        }
                        previous_selected = Some(selected);
                        let response = ui.add(
                            egui::Button::new(
                                egui::RichText::new(text(tab.label())).strong().color(
                                    if selected {
                                        theme::ACCENT
                                    } else {
                                        theme::MUTED
                                    },
                                ),
                            )
                            .min_size(egui::vec2(104., 30.))
                            .selected(selected)
                            .frame(selected),
                        );
                        if response.clicked() {
                            self.shell.tab = tab;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.action_button(ui, Action::Browser, false);
                    });
                });
                ui.separator();
                if self.shell.tab == RibbonTab::Sketch {
                    // Concluir permanece à direita mesmo quando as ferramentas precisam
                    // rolar em uma janela menor: sair do rascunho nunca fica escondido.
                    ui.horizontal(|ui| {
                        let width = (ui.available_width() - 88.).max(76.);
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(width, 88.), egui::Sense::hover());
                        let mut groups = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(rect)
                                .layout(egui::Layout::left_to_right(egui::Align::Min)),
                        );
                        self.ribbon_scroller(&mut groups);
                        self.sketch_section(ui, T::SketchExitGroup, &[Action::FinishSketch], &[]);
                    });
                } else {
                    self.ribbon_scroller(ui);
                }
            });
        self.timeline(ui);
    }
    fn document_tabs(&mut self, ui: &mut egui::Ui) {
        let mut switch = None;
        let mut close = None;
        egui::ScrollArea::horizontal()
            .id_salt("document_tabs")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.;
                    for (index, tab) in self.tabs.iter().enumerate() {
                        let active = index == self.active;
                        if index > 0 && !active && index - 1 != self.active {
                            ui.label(egui::RichText::new("|").color(theme::MUTED));
                        }
                        let name = tab
                            .path
                            .as_ref()
                            .and_then(|p| p.file_stem())
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|| tab.session.document().name().into());
                        let label = format!("{}{}", if tab.modified() { "• " } else { "" }, name);
                        egui::Frame::new()
                            .fill(if active {
                                egui::Color32::from_rgb(69, 82, 101)
                            } else {
                                theme::HEADER
                            })
                            .corner_radius(5)
                            .inner_margin(egui::Margin::symmetric(12, 1))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(egui::RichText::new(label).size(12.))
                                                .frame(false),
                                        )
                                        .clicked()
                                    {
                                        switch = Some(index);
                                    }
                                    if ui
                                        .add_enabled_ui(!self.busy(), |ui| {
                                            icons::button(
                                                ui,
                                                Icon::Close,
                                                text(T::Close),
                                                false,
                                                false,
                                            )
                                        })
                                        .inner
                                        .clicked()
                                    {
                                        close = Some(tab.id);
                                    }
                                });
                            });
                    }
                });
            });
        if let Some(index) = switch {
            self.switch(index);
        }
        if let Some(id) = close {
            self.request_close(id);
        }
    }
    fn ribbon_scroller(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::horizontal()
            .id_salt("ribbon_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.;
                    self.ribbon_groups(ui);
                });
            });
    }

    /// O menu do rótulo reúne as ferramentas secundárias do grupo, como na referência
    /// visual; só aparecem intenções que o rascunho sabe executar.
    fn sketch_section_menu(&mut self, ui: &mut egui::Ui, label: T, actions: &[Action]) {
        let mut chosen = None;
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            let label_width = ui
                .painter()
                .layout_no_wrap(
                    text(label).to_owned(),
                    egui::FontId::proportional(10.),
                    theme::MUTED,
                )
                .size()
                .x;
            let button = egui::Button::new(
                egui::RichText::new(text(label))
                    .size(10.)
                    .color(theme::MUTED),
            )
            .frame(false)
            .min_size(egui::vec2(label_width + 26., 20.));
            let (response, _) =
                egui::containers::menu::MenuButton::from_button(button).ui(ui, |ui| {
                    for action in actions {
                        let enabled = self.action_enabled(*action);
                        ui.horizontal(|ui| {
                            let (rect, _) = ui
                                .allocate_exact_size(egui::Vec2::splat(20.), egui::Sense::hover());
                            icons::paint(ui.painter(), rect, action.icon(), enabled);
                            if ui
                                .add_enabled(
                                    enabled,
                                    egui::Button::new(text(action.label())).frame(false),
                                )
                                .clicked()
                            {
                                chosen = Some(*action);
                                ui.close();
                            }
                        });
                    }
                });
            let at = egui::pos2(response.rect.right() - 8., response.rect.center().y);
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    at + egui::vec2(-3., -2.),
                    at + egui::vec2(3., -2.),
                    at + egui::vec2(0., 2.),
                ],
                theme::MUTED,
                egui::Stroke::NONE,
            ));
        });
        if let Some(action) = chosen {
            self.activate_action(ui.ctx(), action);
        }
    }

    fn sketch_section(
        &mut self,
        ui: &mut egui::Ui,
        label: T,
        promoted: &[Action],
        menu: &[Action],
    ) {
        ui.vertical(|ui| {
            let width = promoted.len() as f32 * 76. + promoted.len().saturating_sub(1) as f32 * 3.;
            ui.set_width(width);
            ui.horizontal(|ui| {
                for action in promoted {
                    self.action_button(ui, *action, true);
                }
            });
            if menu.is_empty() {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_sized(
                        [76., 20.],
                        egui::Label::new(
                            egui::RichText::new(text(label))
                                .size(10.)
                                .color(theme::MUTED),
                        ),
                    );
                });
            } else {
                self.sketch_section_menu(ui, label, menu);
            }
        });
        ui.separator();
    }

    fn sketch_constraints(&mut self, ui: &mut egui::Ui) {
        const GALLERY: [SketchEdit; 6] = [
            SketchEdit::HorizontalVertical,
            SketchEdit::Coincident,
            SketchEdit::Tangent,
            SketchEdit::Equal,
            SketchEdit::Parallel,
            SketchEdit::Perpendicular,
        ];
        ui.vertical(|ui| {
            ui.set_width(155.);
            ui.horizontal(|ui| {
                self.action_button(ui, Action::SketchEdit(SketchEdit::Dimension), true);
                // A grade precisa de layout vertical próprio: herdar o alinhamento
                // central da linha deslocaria a primeira fileira e cortaria o rodapé.
                let (rect, _) = ui.allocate_exact_size(egui::vec2(80., 64.), egui::Sense::hover());
                let mut gallery = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect.shrink2(egui::vec2(0., 6.)))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                gallery.spacing_mut().interact_size.y = 24.;
                egui::Grid::new("constraint_gallery")
                    .num_columns(3)
                    .min_col_width(24.)
                    .min_row_height(24.)
                    .spacing([2., 4.])
                    .show(&mut gallery, |ui| {
                        ui.spacing_mut().button_padding = egui::vec2(2., 2.);
                        for (index, kind) in GALLERY.into_iter().enumerate() {
                            let action = Action::SketchEdit(kind);
                            let enabled = self.action_enabled(action);
                            let selected = self.action_selected(action);
                            let response = ui
                                .add_enabled_ui(enabled, |ui| {
                                    icons::tree_button(
                                        ui,
                                        action.icon(),
                                        text(action.label()),
                                        selected,
                                    )
                                })
                                .inner;
                            if response.clicked() {
                                self.activate_action(ui.ctx(), action);
                            }
                            if index % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
            });
            self.sketch_section_menu(
                ui,
                T::ConstrainGroup,
                &[
                    Action::SketchEdit(SketchEdit::Dimension),
                    Action::SketchEdit(SketchEdit::HorizontalVertical),
                    Action::SketchEdit(SketchEdit::Coincident),
                    Action::SketchEdit(SketchEdit::Tangent),
                    Action::SketchEdit(SketchEdit::Equal),
                    Action::SketchEdit(SketchEdit::Parallel),
                    Action::SketchEdit(SketchEdit::Perpendicular),
                    Action::SketchEdit(SketchEdit::Construction),
                    Action::SketchEdit(SketchEdit::FixPoint),
                ],
            );
        });
        ui.separator();
    }
    fn ribbon_group(&mut self, ui: &mut egui::Ui, label: T, actions: &[Action]) {
        self.ribbon_group_with(ui, label, actions.len(), |editor, ui| {
            for action in actions {
                editor.action_button(ui, *action, true);
            }
        });
    }
    /// Um grupo pode combinar botões diretos e menus, mantendo o mesmo rótulo e separador.
    fn ribbon_group_with(
        &mut self,
        ui: &mut egui::Ui,
        label: T,
        count: usize,
        body: impl FnOnce(&mut Self, &mut egui::Ui),
    ) {
        ui.vertical(|ui| {
            let width = count as f32 * 76. + count.saturating_sub(1) as f32 * 3.;
            ui.set_min_width(width);
            ui.set_max_width(width);
            ui.horizontal(|ui| body(self, ui));
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(text(label))
                        .size(10.)
                        .color(theme::MUTED),
                );
            });
        });
        ui.separator();
    }
    /// Grupo com um botão de menu: o clique abre as subopções, cada uma com ícone, nome e
    /// uma linha de explicação, habilitadas conforme o contexto.
    fn ribbon_menu_group(&mut self, ui: &mut egui::Ui, label: T, menu: Menu<'_>) {
        self.ribbon_group_with(ui, label, 1, |editor, ui| {
            editor.ribbon_menu_button(ui, menu);
        });
    }
    fn ribbon_menu_button(&mut self, ui: &mut egui::Ui, menu: Menu<'_>) {
        let enabled = ui.is_enabled() && menu.items.iter().any(|(a, _, _)| self.action_enabled(*a));
        let response = ui
            .add_enabled_ui(enabled, |ui| {
                icons::button(ui, menu.icon, text(menu.label), true, false)
            })
            .inner;
        self.shell.menu_rects.push(response.rect);
        // Seta de menu centrada sob o rótulo, sem cobrir rótulos longos.
        let corner = response.rect.center_bottom() + egui::vec2(0., -6.);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                corner + egui::vec2(-4., -2.),
                corner + egui::vec2(4., -2.),
                corner + egui::vec2(0., 3.),
            ],
            if enabled { theme::INK } else { theme::MUTED },
            egui::Stroke::NONE,
        ));
        let mut chosen = None;
        egui::Popup::menu(&response).show(|ui| {
            // Largura fixa: a descrição quebra em até duas linhas em vez de alargar o menu.
            ui.set_width(300.);
            for (action, name, hint) in menu.items {
                let enabled = self.action_enabled(*action);
                // Item indisponível por falta do núcleo: o motivo substitui a descrição.
                let (hint, hint_color) = if !enabled
                    && *action == Action::NewPartStep
                    && !nator_kernel::is_available()
                {
                    (T::StepNeedsKernel, theme::WARNING)
                } else {
                    (*hint, theme::MUTED)
                };
                let row = ui
                    .add_enabled_ui(enabled, |ui| {
                        let (rect, row) =
                            ui.allocate_exact_size(egui::vec2(300., 54.), egui::Sense::click());
                        if row.hovered() && enabled {
                            ui.painter().rect_filled(rect, 4, theme::BLUE_FILL);
                        }
                        let icon = egui::Rect::from_center_size(
                            egui::pos2(rect.left() + 20., rect.center().y),
                            egui::Vec2::splat(24.),
                        );
                        icons::paint(ui.painter(), icon, action.icon(), enabled);
                        let ink = if enabled { theme::INK } else { theme::MUTED };
                        ui.painter().text(
                            egui::pos2(rect.left() + 42., rect.top() + 7.),
                            egui::Align2::LEFT_TOP,
                            text(*name),
                            egui::FontId::proportional(13.),
                            ink,
                        );
                        let mut job = egui::text::LayoutJob::simple(
                            text(hint).to_owned(),
                            egui::FontId::proportional(10.5),
                            hint_color,
                            rect.width() - 48.,
                        );
                        job.wrap.max_rows = 2;
                        let galley = ui.painter().layout_job(job);
                        ui.painter().galley(
                            egui::pos2(rect.left() + 42., rect.top() + 24.),
                            galley,
                            hint_color,
                        );
                        row.on_hover_text(text(hint))
                            .on_disabled_hover_text(text(hint))
                    })
                    .inner;
                if row.clicked() && enabled {
                    chosen = Some(*action);
                    ui.close();
                }
            }
        });
        if let Some(action) = chosen {
            self.activate_action(ui.ctx(), action);
        }
    }
    fn ribbon_groups(&mut self, ui: &mut egui::Ui) {
        self.shell.menu_rects.clear();
        match self.shell.tab {
            RibbonTab::Model => {
                self.ribbon_group(ui, T::Sketch, &[Action::NewSketch]);
                self.ribbon_group(
                    ui,
                    T::CreateGroupLabel,
                    &[
                        Action::Solid(SolidTool::Extrude),
                        Action::Solid(SolidTool::Revolve),
                        Action::Solid(SolidTool::Hole),
                    ],
                );
                self.ribbon_group(
                    ui,
                    T::FinishingGroup,
                    &[
                        Action::Solid(SolidTool::Fillet),
                        Action::Solid(SolidTool::Chamfer),
                        Action::Solid(SolidTool::Shell),
                    ],
                );
                self.ribbon_group_with(ui, T::ModifyGroup, 4, |editor, ui| {
                    let enabled = editor.action_enabled(Action::MoveBody);
                    ui.add_enabled_ui(enabled, |ui| {
                        editor.ribbon_menu_button(
                            ui,
                            Menu {
                                icon: Icon::Move,
                                label: T::MoveMenu,
                                items: &move_menu_items(Action::MoveBody),
                            },
                        );
                    });
                    for action in [
                        Action::CopyBody,
                        Action::Solid(SolidTool::Join),
                        Action::Solid(SolidTool::Split),
                    ] {
                        editor.action_button(ui, action, true);
                    }
                });
            }
            // Fluxo próprio da montagem, na ordem de trabalho: referências, peças,
            // posicionamento, cópias e organização.
            RibbonTab::Assembly => {
                self.ribbon_group(ui, T::ReferenceGroup, &[Action::ReferenceSketch]);
                self.ribbon_menu_group(
                    ui,
                    T::PartsGroup,
                    Menu {
                        icon: Icon::NewBody,
                        label: T::NewPartMenu,
                        items: &[
                            (Action::NewEmptyPart, T::NewEmptyPart, T::EmptyPartHint),
                            (Action::NewPartStep, T::NewPartFromStep, T::StepPartHint),
                            (Action::NewPartStl, T::NewPartFromStl, T::StlPartHint),
                        ],
                    },
                );
                self.ribbon_menu_group(
                    ui,
                    T::PositionGroup,
                    Menu {
                        icon: Icon::Move,
                        label: T::MoveMenu,
                        items: &move_menu_items(Action::MoveCopy),
                    },
                );
                self.ribbon_menu_group(
                    ui,
                    T::CopyMenu,
                    Menu {
                        icon: Icon::Copy,
                        label: T::CopyMenu,
                        items: &[
                            (Action::NewInstance, T::CopyNewInstance, T::CopyInstanceHint),
                            (
                                Action::CloneInstance,
                                T::CopyIndependent,
                                T::CopyIndependentHint,
                            ),
                            (
                                Action::InstancePattern(false),
                                T::CopyPattern,
                                T::CopyPatternHint,
                            ),
                        ],
                    },
                );
                self.ribbon_group(ui, T::ArrangeGroup, &[Action::Group, Action::Ungroup]);
            }
            RibbonTab::Inspect => {
                self.ribbon_group(ui, T::AnalysesGroup, &[Action::Measure, Action::Section]);
                self.ribbon_group(ui, T::DocumentGroup, &[Action::Drawing, Action::Parameters]);
                self.ribbon_group(ui, T::ExportGroup, &[Action::ExportStl, Action::ExportStep]);
            }
            RibbonTab::Sketch => {
                self.sketch_section(
                    ui,
                    T::CreateGroupLabel,
                    &[
                        Action::SketchTool(sketch_editor::Tool::Line),
                        Action::SketchTool(sketch_editor::Tool::Rectangle),
                        Action::SketchTool(sketch_editor::Tool::Circle),
                        Action::SketchTool(sketch_editor::Tool::Arc),
                    ],
                    &[
                        Action::SketchTool(sketch_editor::Tool::Select),
                        Action::SketchTool(sketch_editor::Tool::Line),
                        Action::SketchTool(sketch_editor::Tool::Rectangle),
                        Action::SketchTool(sketch_editor::Tool::Circle),
                        Action::SketchTool(sketch_editor::Tool::Arc),
                    ],
                );
                self.sketch_section(
                    ui,
                    T::ModifyGroup,
                    &[
                        Action::SketchTool(sketch_editor::Tool::Trim),
                        Action::SketchEdit(SketchEdit::Offset),
                        Action::SketchEdit(SketchEdit::Fillet),
                    ],
                    &[
                        Action::SketchTool(sketch_editor::Tool::Trim),
                        Action::SketchEdit(SketchEdit::Offset),
                        Action::SketchEdit(SketchEdit::Fillet),
                        Action::SketchEdit(SketchEdit::Chamfer),
                        Action::SketchEdit(SketchEdit::Extend),
                        Action::SketchEdit(SketchEdit::Delete),
                    ],
                );
                let patterns = [
                    Action::SketchEdit(SketchEdit::Mirror),
                    Action::SketchEdit(SketchEdit::CircularPattern),
                    Action::SketchEdit(SketchEdit::RectangularPattern),
                ];
                self.sketch_section(ui, T::SketchPatternGroup, &patterns, &patterns);
                self.sketch_constraints(ui);
            }
        }
    }
    fn timeline(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("timeline")
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::symmetric(10, 3))
                    .stroke(egui::Stroke::new(1., theme::BORDER)),
            )
            .show(ui, |ui| {
                let (row, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 34.),
                    egui::Sense::hover(),
                );
                let busy = self.busy();
                let (label, color) = if !busy && self.pending_failure.is_some() {
                    (text(T::Failed), theme::DANGER)
                } else {
                    self.status.indicator(busy)
                };
                let label = label.to_owned();
                let text_width = ui
                    .painter()
                    .layout_no_wrap(
                        label.clone(),
                        egui::TextStyle::Small.resolve(ui.style()),
                        theme::MUTED,
                    )
                    .size()
                    .x;
                // Mensagens recebem mais espaço, mas mantêm a barra em uma linha
                // e deixam ao menos metade da largura para histórico e operações.
                let status_width = (text_width + 20.).clamp(130., (row.width() * 0.45).max(130.));
                // Reservas independentes mantêm o status na direita e impedem
                // que uma linha do tempo longa crie uma segunda linha no rodapé.
                let left =
                    egui::Rect::from_min_max(row.min, egui::pos2(row.left() + 150., row.bottom()));
                let right = egui::Rect::from_min_max(
                    egui::pos2(row.right() - status_width, row.top()),
                    row.max,
                );
                let middle = egui::Rect::from_min_max(
                    egui::pos2(left.right() + 8., row.top()),
                    egui::pos2(right.left() - 8., row.bottom()),
                );
                let mut controls = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(left)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                controls.spacing_mut().item_spacing.x = 4.;
                controls.label(
                    egui::RichText::new(text(T::ParametricHistory))
                        .strong()
                        .size(12.),
                );
                self.action_button(&mut controls, Action::Undo, false);
                self.action_button(&mut controls, Action::Redo, false);
                ui.painter().line_segment(
                    [
                        egui::pos2(left.right(), row.top() + 4.),
                        egui::pos2(left.right(), row.bottom() - 4.),
                    ],
                    egui::Stroke::new(1., theme::BORDER),
                );
                if middle.is_positive() {
                    let mut strip = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(middle)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    strip.set_clip_rect(middle.intersect(ui.clip_rect()));
                    self.timeline_strip(&mut strip);
                }
                let label_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        right.right() - text_width.min(status_width - 20.),
                        right.top(),
                    ),
                    right.max,
                );
                let mut state = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(label_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                state
                    .add(
                        egui::Label::new(egui::RichText::new(&label).small().color(theme::MUTED))
                            .truncate(),
                    )
                    .on_hover_text(&label);
                ui.painter().circle_filled(
                    egui::pos2(label_rect.left() - 12., row.center().y),
                    3.,
                    color,
                );
            });
    }
    pub(super) fn command_palette(&mut self, context: &egui::Context) {
        if !self.shell.search_open {
            return;
        }
        let mut open = true;
        let mut chosen = None;
        egui::Window::new(text(T::SearchTools))
            .id(egui::Id::new("command_palette"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.)
            .anchor(egui::Align2::CENTER_TOP, [0., 85.])
            .show(context, |ui| {
                let field = ui.add(
                    egui::TextEdit::singleline(&mut self.shell.query)
                        .hint_text(text(T::SearchHint))
                        .desired_width(f32::INFINITY),
                );
                if self.shell.search_focus {
                    field.request_focus();
                    self.shell.search_focus = false;
                }
                let query = normalized_search(&self.shell.query);
                let matches: Vec<_> = SEARCH_ACTIONS
                    .iter()
                    .copied()
                    .filter(|action| normalized_search(text(action.label())).contains(&query))
                    .collect();
                let first = matches
                    .iter()
                    .copied()
                    .find(|action| self.action_enabled(*action));
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    chosen = first;
                }
                if matches.is_empty() {
                    ui.label(text(T::NoMatchingTools));
                }
                egui::ScrollArea::vertical()
                    .max_height(330.)
                    .show(ui, |ui| {
                        for action in matches {
                            ui.add_enabled_ui(self.action_enabled(action), |ui| {
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::Vec2::splat(22.),
                                        egui::Sense::hover(),
                                    );
                                    icons::paint(
                                        ui.painter(),
                                        rect,
                                        action.icon(),
                                        ui.is_enabled(),
                                    );
                                    if ui
                                        .add_sized(
                                            [ui.available_width(), 28.],
                                            egui::Button::new(text(action.label())).frame(false),
                                        )
                                        .clicked()
                                    {
                                        chosen = Some(action);
                                    }
                                });
                            });
                        }
                    });
            });
        if context.input(|i| i.key_pressed(egui::Key::Escape)) {
            open = false;
        }
        self.shell.search_open = open;
        if let Some(action) = chosen {
            self.shell.search_open = false;
            self.activate_action(context, action);
        }
    }
}

/// A busca trata os acentos mais comuns do português sem alterar rótulos nem nomes.
fn normalized_search(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_search_finds_portuguese_names_without_accents() {
        assert!(normalized_search(text(T::Extrude)).contains("extrusao"));
        assert!(normalized_search(text(T::NewSketch)).contains("rascunho"));
        assert!(
            SEARCH_ACTIONS
                .iter()
                .any(|a| normalized_search(text(a.label())).contains("parametros"))
        );
    }
}
