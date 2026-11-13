//! Estrutura visual inspirada na organização por tarefas de um CAD desktop.
//! Este módulo só escolhe ferramentas e apresenta o estado: alterações continuam
//! passando pelos comandos e pelos trabalhadores já existentes no editor.
use super::{
    icons::{self, Icon},
    *,
};
use feature_editor::Tool as SolidTool;
use nator_sketch::edit::Action as SketchEdit;
use view_modes::{CameraMode, PrimaryRoute};

/// Todas as abas compartilham a mesma faixa, sem rodapé com o nome do grupo.
const GROUP_HORIZONTAL_GAP: f32 = 3.;
/// Oito botões de 20 pontos por fileira, com todas as restrições expostas.
const RESTRICT_COLUMN_WIDTH: f32 = 176.;
const RESTRICT_CAPTION_HEIGHT: f32 = 12.;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) enum RibbonTab {
    /// Aba principal: o projeto começa pela montagem.
    #[default]
    Assembly,
    Model,
    Sketch,
    Print,
}
impl RibbonTab {
    fn label(self) -> T {
        match self {
            Self::Model => T::ModelTab,
            Self::Assembly => T::Assembly,
            Self::Sketch => T::Sketch,
            Self::Print => T::PrintTab,
        }
    }
}

#[derive(Default)]
pub(super) struct ShellState {
    pub tab: RibbonTab,
    pub browser_visible: bool,
    pub measure_open: bool,
    /// Modo em que o botão esquerdo move a câmera; `None` deixa-o com a ferramenta atual.
    pub camera_mode: Option<CameraMode>,
    /// Botão esquerdo pressionado no canvas e repassado à câmera.
    pub camera_press: PrimaryRoute,
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
    Select,
    New,
    Open,
    Save,
    SaveAs,
    Undo,
    Redo,
    Import,
    Reference,
    NewSketch,
    SketchImage,
    SketchText,
    Paint,
    SketchXY,
    SketchXZ,
    SketchYZ,
    SketchFace,
    OffsetPlane,
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
    ImportReference,
    /// STEP, STL ou 3MF como referência filha da peça em edição (pasta Referências dela),
    /// que depois pode virar subpeça.
    ImportIntoPart,
    MoveFace,
    MoveEdge,
    MovePoint,
    Parameters,
    PrintProfile,
    Calibrate,
    PrintCheck,
    FitCheck,
    Measure,
    Drawing,
    ExportPrint,
    ExportStep,
    Home,
    Fit,
    PanCamera,
    OrbitCamera,
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
    CancelSketch,
}
impl Action {
    fn label(self) -> T {
        match self {
            Self::Select => T::Select,
            Self::New => T::NewProject,
            Self::Open => T::OpenProject,
            Self::Save => T::Save,
            Self::SaveAs => T::SaveAs,
            Self::Undo => T::Undo,
            Self::Redo => T::Redo,
            Self::Import => T::Import,
            Self::Reference => T::Reference,
            Self::NewSketch => T::NewSketch,
            Self::SketchImage => T::SketchImage,
            Self::SketchText => T::SketchTextTool,
            Self::Paint => T::Paint,
            Self::SketchXY => T::SketchXY,
            Self::SketchXZ => T::SketchXZ,
            Self::SketchYZ => T::SketchYZ,
            Self::SketchFace => T::SketchOnFace,
            Self::OffsetPlane => T::OffsetPlane,
            Self::Solid(tool) => tool.label(),
            Self::MoveBody => T::MoveMenu,
            Self::CopyBody => T::CopyMenu,
            Self::MoveCopy => T::MoveMenu,
            Self::Align => T::AlignTool,
            Self::NewInstance => T::CopyMenu,
            Self::CloneInstance => T::CloneTool,
            Self::InstancePattern(false) => T::RectangularInstancePattern,
            Self::InstancePattern(true) => T::CircularInstancePattern,
            Self::InstanceMirror => T::MirrorComponent,
            Self::Group => T::GroupTool,
            Self::Ungroup => T::UngroupTool,
            Self::Section => T::SectionAnalysis,
            Self::InsertPart => T::InsertFromProject,
            Self::ReferenceSketch => T::ReferenceSketch,
            Self::NewEmptyPart => T::NewPartMenu,
            Self::ImportReference => T::ImportReference,
            Self::ImportIntoPart => T::ImportIntoPart,
            Self::MoveFace => T::MoveFaceToFace,
            Self::MoveEdge => T::MoveEdgeToEdge,
            Self::MovePoint => T::MovePointToPoint,
            Self::PrintProfile => T::PrintProfile,
            Self::Calibrate => T::Calibrate,
            Self::PrintCheck => T::PrintCheck,
            Self::FitCheck => T::FitCheck,
            Self::Parameters => T::Parameters,
            Self::Measure => T::MeasureTool,
            Self::Drawing => T::Drawing,
            Self::ExportPrint => T::ExportForPrint,
            Self::ExportStep => T::ExportAssemblyStep,
            Self::Home => T::ViewHome,
            Self::Fit => T::Fit,
            Self::PanCamera => T::PanCamera,
            Self::OrbitCamera => T::OrbitCamera,
            Self::Iso => T::Iso,
            Self::Front => T::Front,
            Self::Top => T::Top,
            Self::Side => T::Side,
            Self::Perspective => T::Perspective,
            Self::Shadows => T::Shadows,
            Self::Triangles => T::Triangulation,
            Self::Browser => T::Project,
            Self::FinishSketch => T::FinishSketch,
            Self::CancelSketch => T::CancelSketch,
            Self::SketchTool(tool) => tool.label(),
            Self::SketchEdit(action) => sketch_editor::edit_label(action),
        }
    }
    fn caption(self) -> T {
        match self {
            Self::FinishSketch => T::FinishSketchShort,
            Self::CancelSketch => T::Cancel,
            Self::Solid(SolidTool::Fillet) => T::FilletShort,
            Self::Solid(SolidTool::Mirror) => T::MirrorShort,
            Self::Solid(SolidTool::Split) => T::SplitShort,
            Self::Solid(SolidTool::CheckSolid) => T::CheckSolidShort,
            Self::Solid(SolidTool::BedFace) => T::BedFaceShort,
            Self::Solid(SolidTool::ComponentCut) => T::ComponentCutShort,
            Self::Solid(SolidTool::ScrewBoss) => T::ScrewBossShort,
            Self::ExportPrint => T::ExportForPrintShort,
            Self::PrintCheck => T::PrintCheckShort,
            Self::FitCheck => T::FitCheckShort,
            Self::SketchTool(sketch_editor::Tool::Arc) => T::ArcShort,
            Self::InstancePattern(false) => T::RectangularPattern,
            Self::InstancePattern(true) => T::CircularPattern,
            Self::InstanceMirror => T::MirrorShort,
            Self::ExportStep => T::StepShort,
            Self::SketchFace => T::SketchFaceShort,
            Self::SketchTool(sketch_editor::Tool::Select) => T::SketchDiagnosticsShort,
            Self::ImportReference | Self::ImportIntoPart => T::ImportMenu,
            _ => self.label(),
        }
    }
    fn icon(self) -> Icon {
        match self {
            Self::Select => Icon::Select,
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
            Self::OffsetPlane => Icon::OffsetPlane,
            Self::SketchImage => Icon::Image,
            Self::SketchText => Icon::Label,
            Self::Paint => Icon::Paint,
            Self::Solid(tool) => match tool {
                SolidTool::Extrude => Icon::Extrude,
                SolidTool::Revolve => Icon::Revolve,
                SolidTool::Loft => Icon::Loft,
                SolidTool::Sweep => Icon::Sweep,
                SolidTool::Pipe => Icon::Pipe,
                SolidTool::Label => Icon::Label,
                SolidTool::Hole => Icon::Hole,
                SolidTool::Fillet => Icon::Fillet,
                SolidTool::Chamfer => Icon::Chamfer,
                SolidTool::Shell => Icon::Shell,
                SolidTool::MergeFaces => Icon::Join,
                SolidTool::RemoveFaces => Icon::Delete,
                SolidTool::Mirror => Icon::Mirror,
                SolidTool::LinearPattern | SolidTool::CircularPattern => Icon::Pattern,
                SolidTool::Split => Icon::Split,
                SolidTool::Join | SolidTool::Cut | SolidTool::Intersect => Icon::Boolean,
                SolidTool::CheckSolid => Icon::CheckSolid,
                SolidTool::BedFace => Icon::BedFace,
                SolidTool::ComponentCut => Icon::ComponentCut,
                SolidTool::Lip => Icon::Lip,
                SolidTool::ScrewBoss => Icon::ScrewBoss,
                SolidTool::SnapFit => Icon::SnapFit,
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
            Self::ImportReference | Self::ImportIntoPart => Icon::Import,
            Self::MoveFace => Icon::FaceToFace,
            Self::MoveEdge => Icon::EdgeToEdge,
            Self::MovePoint => Icon::PointToPoint,
            Self::PrintProfile => Icon::PrintProfile,
            Self::Calibrate => Icon::Calibrate,
            Self::PrintCheck => Icon::PrintCheck,
            Self::FitCheck => Icon::FitCheck,
            Self::Parameters => Icon::Parameters,
            Self::Measure => Icon::Measure,
            Self::Drawing => Icon::Drawing,
            Self::ExportPrint | Self::ExportStep => Icon::Export,
            Self::Home => Icon::Home,
            Self::Fit => Icon::Fit,
            Self::PanCamera => Icon::PanCamera,
            Self::OrbitCamera => Icon::OrbitCamera,
            Self::Iso => Icon::Cube,
            Self::Front => Icon::Front,
            Self::Top => Icon::Top,
            Self::Side => Icon::Side,
            Self::Perspective => Icon::Perspective,
            Self::Shadows => Icon::Eye,
            Self::Triangles => Icon::Pattern,
            Self::Browser => Icon::Folder,
            Self::FinishSketch => Icon::Check,
            Self::CancelSketch => Icon::Close,
            Self::SketchTool(tool) => tool.icon(),
            Self::SketchEdit(action) => sketch_editor::edit_icon(action),
        }
    }
    fn shortcut(self) -> Option<&'static str> {
        match self {
            Self::Select => Some("Esc"),
            Self::Save => Some("Ctrl+S"),
            Self::Undo => Some("Ctrl+Z"),
            Self::Redo => Some("Ctrl+Shift+Z"),
            Self::SketchTool(tool) => Some(match tool {
                sketch_editor::Tool::Select => "Esc",
                sketch_editor::Tool::Line => "L",
                sketch_editor::Tool::Rectangle => "R",
                sketch_editor::Tool::Circle => "C",
                sketch_editor::Tool::Arc => "A",
                sketch_editor::Tool::Project => "P",
                sketch_editor::Tool::Trim => "G",
                sketch_editor::Tool::Dimension => "D",
            }),
            Self::SketchEdit(action) => match action {
                SketchEdit::Offset => Some("S"),
                SketchEdit::Fillet => Some("O"),
                SketchEdit::Chamfer => Some("H"),
                SketchEdit::Mirror => Some("E"),
                SketchEdit::Delete => Some("Del"),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Também alimenta a busca: nenhuma entrada anuncia uma ferramenta sem implementação.
const SEARCH_ACTIONS: &[Action] = &[
    Action::Select,
    Action::New,
    Action::Open,
    Action::Save,
    Action::SaveAs,
    Action::Undo,
    Action::Redo,
    Action::Import,
    Action::Reference,
    Action::NewSketch,
    Action::SketchImage,
    Action::SketchText,
    Action::SketchXY,
    Action::SketchXZ,
    Action::SketchYZ,
    Action::SketchFace,
    Action::OffsetPlane,
    Action::Paint,
    Action::Solid(SolidTool::Extrude),
    Action::Solid(SolidTool::Revolve),
    Action::Solid(SolidTool::Loft),
    Action::Solid(SolidTool::Pipe),
    Action::Solid(SolidTool::Fillet),
    Action::Solid(SolidTool::Chamfer),
    Action::Solid(SolidTool::Shell),
    Action::Solid(SolidTool::MergeFaces),
    Action::Solid(SolidTool::RemoveFaces),
    Action::Solid(SolidTool::Hole),
    Action::Solid(SolidTool::Join),
    Action::Solid(SolidTool::Cut),
    Action::Solid(SolidTool::Intersect),
    Action::Solid(SolidTool::LinearPattern),
    Action::Solid(SolidTool::CircularPattern),
    Action::Solid(SolidTool::Mirror),
    Action::Solid(SolidTool::Split),
    Action::Solid(SolidTool::CheckSolid),
    Action::Solid(SolidTool::BedFace),
    Action::Solid(SolidTool::ComponentCut),
    Action::Solid(SolidTool::Lip),
    Action::Solid(SolidTool::ScrewBoss),
    Action::Solid(SolidTool::SnapFit),
    Action::MoveCopy,
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
    Action::ImportReference,
    Action::ImportIntoPart,
    Action::MoveFace,
    Action::MoveEdge,
    Action::MovePoint,
    Action::Parameters,
    Action::PrintProfile,
    Action::Calibrate,
    Action::PrintCheck,
    Action::FitCheck,
    Action::Drawing,
    Action::Measure,
    Action::ExportPrint,
    Action::ExportStep,
    Action::Home,
    Action::Fit,
    Action::PanCamera,
    Action::OrbitCamera,
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
    Action::SketchTool(sketch_editor::Tool::Project),
    Action::SketchEdit(SketchEdit::Offset),
    Action::SketchEdit(SketchEdit::Fillet),
    Action::SketchEdit(SketchEdit::Chamfer),
    Action::SketchEdit(SketchEdit::Extend),
    Action::SketchEdit(SketchEdit::Mirror),
    Action::SketchTool(sketch_editor::Tool::Dimension),
    Action::SketchEdit(SketchEdit::Delete),
    Action::FinishSketch,
    Action::CancelSketch,
];

/// Barra de navegação, na ordem de exibição: modos do clique, enquadramentos e câmera.
const NAVIGATION_GROUPS: [&[Action]; 3] = [
    &[Action::Select, Action::Measure],
    &[Action::Home, Action::Fit],
    &[Action::PanCamera, Action::OrbitCamera],
];

fn group_width(count: usize) -> f32 {
    count as f32 * icons::RIBBON_BUTTON_SIZE.x
        + count.saturating_sub(1) as f32 * GROUP_HORIZONTAL_GAP
}

/// Lado do botão só com ícone da faixa compacta.
const COMPACT_BUTTON: f32 = 30.;
/// Faixa do nome da seção sob os botões, no tamanho Maior.
const SECTION_CAPTION_HEIGHT: f32 = 15.;

fn compact_group_width(count: usize) -> f32 {
    count as f32 * COMPACT_BUTTON + count.saturating_sub(1) as f32 * GROUP_HORIZONTAL_GAP
}

impl Editor {
    /// Botão Importar da aba Construção habilitado, para os ensaios.
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn import_into_part_enabled(&self) -> bool {
        self.action_enabled(Action::ImportIntoPart)
    }
    fn action_enabled(&self, action: Action) -> bool {
        let editable = !self.busy() && self.pending_close.is_none();
        match action {
            Action::SketchImage => editable && self.current().sketch_edit.is_some(),
            Action::SketchText => {
                editable && self.current().sketch_edit.is_some() && nator_kernel::is_available()
            }
            Action::Home | Action::Fit | Action::PanCamera | Action::OrbitCamera => true,
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
            Action::Solid(SolidTool::Label) => false,
            Action::Paint | Action::Solid(_) => {
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
            // O plano pertence à peça em construção; sem ela não há onde guardá-lo.
            Action::OffsetPlane => {
                editable
                    && self.current().sketch_edit.is_none()
                    && self.current().active_part.is_some()
            }
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
                        || matches!(
                            action,
                            Action::NewInstance
                                | Action::CloneInstance
                                | Action::InstancePattern(_)
                                | Action::InstanceMirror
                        ))
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
            Action::ImportIntoPart => {
                editable
                    && self.current().sketch_edit.is_none()
                    && self
                        .current()
                        .active_part
                        .and_then(|id| self.current().session.document().part(id))
                        .is_some_and(|p| !p.reference)
            }
            Action::ReferenceSketch
            | Action::NewEmptyPart
            | Action::ImportReference
            | Action::MoveFace
            | Action::MoveEdge
            | Action::MovePoint => editable && self.current().sketch_edit.is_none(),
            Action::NewSketch | Action::SketchXY | Action::SketchXZ | Action::SketchYZ => {
                editable && self.current().sketch_edit.is_none()
            }
            Action::SketchFace => {
                editable && self.current().picked.is_some() && self.current().sketch_edit.is_none()
            }
            // Medir usa cliques na área 3D, que o rascunho em edição reserva às ferramentas.
            Action::Measure => editable && self.current().sketch_edit.is_none(),
            Action::PrintCheck => editable && self.current().active_part.is_some(),
            Action::SketchTool(_) | Action::SketchEdit(_) => {
                editable && self.current().sketch_edit.is_some()
            }
            Action::CancelSketch => editable && self.current().sketch_edit.is_some(),
            Action::FinishSketch => {
                editable
                    && self.sketch_text.is_none()
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
            Action::SketchImage => self.image_edit.is_some(),
            Action::SketchText => self.sketch_text.is_some(),
            Action::Paint => self.paint_edit.is_some(),
            Action::Select => self.shell.camera_mode.is_none() && !self.shell.measure_open,
            Action::Measure => self.shell.measure_open && self.shell.camera_mode.is_none(),
            Action::PanCamera => self.shell.camera_mode == Some(CameraMode::Pan),
            Action::OrbitCamera => self.shell.camera_mode == Some(CameraMode::Orbit),
            Action::Perspective => {
                self.viewport.camera.projection == nator_renderer::Projection::Perspective
            }
            Action::Shadows => self.viewport.show_shadows,
            Action::Triangles => self.viewport.show_triangles,
            Action::Browser => self.shell.browser_visible,
            Action::Section => self.current().section.is_some_and(|s| s.visible),
            // Selecionar no rascunho é o estado neutro: o botão acende só com o diagnóstico aberto.
            Action::SketchTool(tool) => self.current().sketch_edit.as_ref().is_some_and(|w| {
                w.tool() == tool
                    && (tool != sketch_editor::Tool::Select
                        || (w.is_selecting() && w.diagnostics_open()))
            }),
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
        let hint = if let Some(shortcut) = action.shortcut() {
            format!("{} · {shortcut}", text(action.label()))
        } else {
            text(action.label()).to_owned()
        };
        let response = ui
            .add_enabled_ui(enabled, |ui| {
                if large {
                    icons::ribbon_button(ui, action.icon(), text(action.caption()), selected, &hint)
                } else {
                    icons::button_with_hint(
                        ui,
                        action.icon(),
                        text(action.label()),
                        false,
                        selected,
                        &hint,
                    )
                }
            })
            .inner;
        #[cfg(all(test, feature = "occt"))]
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                egui::Id::new(("ribbon_action_rect", text(action.label()))),
                response.rect,
            );
        });
        if !enabled && action == Action::OffsetPlane && self.current().active_part.is_none() {
            response
                .clone()
                .on_disabled_hover_text(text(T::ChooseActivePart));
        } else if !enabled && matches!(action, Action::Solid(_) | Action::Parameters) {
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
    /// Ação de um atalho de teclado (`shortcuts`).
    pub(super) fn run_shortcut(&mut self, context: &egui::Context, target: shortcuts::Target) {
        use shortcuts::Target;
        match target {
            Target::Save => self.save(context, self.current().id, false, false),
            Target::Undo => self.step_history(context, false),
            Target::Redo => self.step_history(context, true),
            Target::Search => {
                self.shell.search_open = true;
                self.shell.search_focus = true;
            }
            Target::Tool(tool) => self.select_sketch_tool(tool),
            Target::Edit(action) => self.open_sketch_action(context, action),
            Target::Delete => self.delete_sketch_selection(context),
            Target::Stroke => self.toggle_sketch_stroke(context),
            Target::Extrude => self.activate_action(context, Action::Solid(SolidTool::Extrude)),
            Target::Revolve => self.activate_action(context, Action::Solid(SolidTool::Revolve)),
            Target::Fillet => self.activate_action(context, Action::Solid(SolidTool::Fillet)),
            Target::Chamfer => self.activate_action(context, Action::Solid(SolidTool::Chamfer)),
            Target::Move => self.activate_action(context, Action::MoveCopy),
        }
    }
    fn activate_action(&mut self, context: &egui::Context, action: Action) {
        if !self.action_enabled(action) {
            return;
        }
        // Uma nova ferramenta assume a interação; cancelar a anterior preserva
        // os elementos selecionados para preencher os campos compatíveis.
        if matches!(
            action,
            Action::NewSketch
                | Action::SketchFace
                | Action::OffsetPlane
                | Action::ReferenceSketch
                | Action::Paint
                | Action::Solid(_)
                | Action::MoveBody
                | Action::CopyBody
                | Action::MoveCopy
                | Action::Align
                | Action::MoveFace
                | Action::MoveEdge
                | Action::MovePoint
                | Action::InstancePattern(_)
                | Action::InstanceMirror
                | Action::Section
                | Action::InsertPart
                | Action::Parameters
                | Action::PrintProfile
                | Action::Calibrate
                | Action::PrintCheck
                | Action::FitCheck
                | Action::Measure
                | Action::Drawing
                | Action::ExportPrint
                | Action::SketchTool(_)
                | Action::SketchEdit(_)
        ) {
            self.activate_selection(context);
        }
        if matches!(
            action,
            Action::Home
                | Action::Fit
                | Action::Iso
                | Action::Front
                | Action::Top
                | Action::Side
                | Action::Perspective
        ) {
            self.current_mut().camera_transition = None;
        }
        match action {
            Action::Select => {
                self.shell.camera_mode = None;
                self.shell.measure_open = false;
                self.measure_tool.clear();
                context.request_repaint();
            }
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
            Action::Undo | Action::Redo => self.step_history(context, action == Action::Redo),
            Action::Import => self.import(
                context,
                ImportSource::Any,
                self.current().active_part,
                false,
            ),
            Action::Reference => self.import(
                context,
                ImportSource::Reference,
                self.current().active_part,
                false,
            ),
            Action::ReferenceSketch => {
                let pick = (self.pick_mode == navigation::PickMode::Face)
                    .then(|| self.selected_pick().cloned())
                    .flatten();
                self.start_reference_sketch_plane();
                if let Some(pick) = pick {
                    self.choose_picked_plane(context, pick);
                }
            }
            Action::NewEmptyPart => {
                let count = self
                    .current()
                    .session
                    .document()
                    .next_name_number(nator_document::ElementKind::Part);
                self.execute(Command::CreatePart {
                    name: format!("{} {count}", text(T::NewPartName)),
                });
            }
            // Abre direto o seletor com STEP, STL e 3MF; o modelo entra só como referência.
            Action::ImportReference => self.import(context, ImportSource::Any, None, true),
            Action::ImportIntoPart => {
                self.import(context, ImportSource::Any, self.current().active_part, true)
            }
            Action::MoveFace => self.open_align(navigation::PickMode::Face),
            Action::MoveEdge => self.open_align(navigation::PickMode::Edge),
            Action::MovePoint => self.open_align(navigation::PickMode::Vertex),
            Action::SketchImage => self.load_reference_image(context),
            Action::SketchText => self.open_sketch_text(context),
            Action::NewSketch => {
                let pick = (self.pick_mode == navigation::PickMode::Face)
                    .then(|| self.selected_pick().cloned())
                    .flatten();
                self.start_sketch_plane();
                if let Some(pick) = pick {
                    self.choose_picked_plane(context, pick);
                }
            }
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
            Action::OffsetPlane => {
                let pick = (self.pick_mode == navigation::PickMode::Face)
                    .then(|| self.selected_pick().cloned())
                    .flatten();
                self.open_offset_plane();
                if let Some(pick) = pick {
                    self.choose_picked_plane(context, pick);
                }
            }
            Action::Paint => self.open_paint(),
            Action::Solid(tool) => {
                self.open_feature_tool(tool);
                self.prefill_selected_topology(context);
            }
            Action::MoveCopy => {
                if self.shell.tab == RibbonTab::Assembly {
                    self.enter_assembly_context(context);
                }
                self.open_move();
            }
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
            Action::PrintProfile => self.open_print_profile(),
            Action::Calibrate => self.open_calibration(),
            Action::PrintCheck => self.open_print_check(),
            Action::FitCheck => self.open_fit_check(),
            Action::Measure => {
                self.shell.measure_open = true;
                self.measure_tool.clear();
            }
            Action::Drawing => self.open_drawing(None),
            Action::ExportPrint => self.open_export(),
            Action::ExportStep => self.export_step(
                context,
                self.current().id,
                nator_commands::export::StepScope::Assembly,
            ),
            Action::Home => {
                // Mesmo efeito do Home do cubo: a projeção escolhida não muda.
                self.viewport
                    .camera
                    .set_direction(Camera::default().direction());
                let [w, h] = self.viewport.size();
                self.viewport.camera.fit(f64::from(w) / f64::from(h));
            }
            Action::Fit => {
                if let Some(work) = &mut self.current_mut().sketch_edit {
                    work.fit_requested = true;
                    return;
                }
                let [w, h] = self.viewport.size();
                self.viewport.camera.fit(f64::from(w) / f64::from(h));
            }
            Action::PanCamera => self.toggle_camera_mode(CameraMode::Pan),
            Action::OrbitCamera => self.toggle_camera_mode(CameraMode::Orbit),
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
            Action::SketchTool(tool) => self.select_sketch_tool(tool),
            Action::SketchEdit(action) => self.open_sketch_action(context, action),
            Action::FinishSketch => self.finish_sketch(),
            Action::CancelSketch => self.cancel_sketch(context),
        }
    }
    /// Arrastar e Rotacionar suspendem a ferramenta em uso sem cancelá-la: um novo clique
    /// devolve o botão esquerdo a ela.
    fn toggle_camera_mode(&mut self, mode: CameraMode) {
        let active = self.shell.camera_mode == Some(mode);
        self.shell.camera_mode = (!active).then_some(mode);
    }
    /// Painel dos modos de navegação, centrado na base da área de edição.
    pub(super) fn navigation_bar(&mut self, context: &egui::Context) {
        egui::Area::new(egui::Id::new("navigation_bar"))
            .order(egui::Order::Middle)
            .fade_in(false)
            .constrain(false)
            .pivot(egui::Align2::CENTER_BOTTOM)
            .fixed_pos(self.canvas_rect.center_bottom() - egui::vec2(0., 10.))
            .show(context, |ui| {
                theme::floating_frame()
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.;
                            for (index, group) in NAVIGATION_GROUPS.iter().enumerate() {
                                if index > 0 {
                                    ui.separator();
                                }
                                for action in *group {
                                    self.action_button(ui, *action, false);
                                }
                            }
                        });
                    });
            });
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
                self.shell.before_sketch = Some(self.shell.tab);
                self.shell.tab = RibbonTab::Sketch;
            } else {
                // Confirmar ou cancelar volta ao contexto de edição da peça, que é
                // a aba Construção, mesmo que o rascunho tenha sido aberto da Montagem.
                // Rascunhos de referência não têm peça: pertencem à Montagem.
                self.shell.before_sketch = None;
                self.shell.tab = if self.current().active_part.is_some() {
                    RibbonTab::Model
                } else {
                    RibbonTab::Assembly
                };
            }
            self.shell.sketch_context = sketch;
        }
        egui::Panel::top("application_bar")
            .frame(
                egui::Frame::new()
                    .fill(theme::header())
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
                    ui.label(egui::RichText::new(text(T::AppName)).strong().size(18.));
                    ui.separator();
                    self.action_button(ui, Action::New, false);
                    let open = ui
                        .add_enabled_ui(self.action_enabled(Action::Open), |ui| {
                            icons::button(ui, Icon::Open, text(T::OpenProject), false, false)
                        })
                        .inner;
                    let mut open_dialog = false;
                    let mut open_recent = None;
                    let recent = self.recent.clone();
                    egui::Popup::menu(&open).show(|ui| {
                        if ui
                            .add_enabled(
                                self.action_enabled(Action::Open),
                                egui::Button::new(text(T::OpenProject)),
                            )
                            .clicked()
                        {
                            open_dialog = true;
                            ui.close();
                        }
                        if !recent.is_empty() {
                            ui.separator();
                            ui.label(
                                egui::RichText::new(text(T::RecentProjects))
                                    .small()
                                    .color(theme::muted()),
                            );
                            for path in recent {
                                let name = path
                                    .file_name()
                                    .map(|name| name.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| path.display().to_string());
                                if icons::menu_item(ui, None, &name, 24.)
                                    .on_hover_text(path.display().to_string())
                                    .clicked()
                                {
                                    open_recent = Some(path);
                                    ui.close();
                                }
                            }
                        }
                    });
                    if open_dialog {
                        self.activate_action(ui.ctx(), Action::Open);
                    }
                    if let Some(path) = open_recent {
                        self.open_recent(ui.ctx(), path);
                    }
                    let save = ui
                        .add_enabled_ui(self.action_enabled(Action::Save), |ui| {
                            icons::button(ui, Icon::Save, text(T::Save), false, false)
                        })
                        .inner;
                    let mut save_now = false;
                    let mut save_as = false;
                    egui::Popup::menu(&save).show(|ui| {
                        if ui
                            .add_enabled(
                                self.action_enabled(Action::Save),
                                egui::Button::new(text(T::Save)),
                            )
                            .clicked()
                        {
                            save_now = true;
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                self.action_enabled(Action::SaveAs),
                                egui::Button::new(text(T::SaveAs)),
                            )
                            .clicked()
                        {
                            save_as = true;
                            ui.close();
                        }
                    });
                    if save_now {
                        self.activate_action(ui.ctx(), Action::Save);
                    }
                    if save_as {
                        self.activate_action(ui.ctx(), Action::SaveAs);
                    }
                    let settings =
                        icons::button(ui, Icon::Settings, text(T::SettingsMenu), false, false);
                    if settings.clicked() {
                        self.settings_panel.open();
                    }
                    // As ações secundárias continuam acessíveis pelo menu de Salvar e pela busca.
                    ui.separator();
                    let width =
                        (ui.available_width() - 20. - window_frame::CONTROLS_WIDTH).max(110.);
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
                    .fill(theme::surface())
                    .inner_margin(egui::Margin::symmetric(12, 3))
                    .stroke(egui::Stroke::new(1., theme::border())),
            )
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.;
                    let mut previous_selected = None;
                    for tab in [
                        RibbonTab::Assembly,
                        RibbonTab::Model,
                        RibbonTab::Sketch,
                        RibbonTab::Print,
                    ] {
                        let selected = self.shell.tab == tab;
                        // A aba ativa já tem seu próprio contorno. O divisor aparece
                        // apenas entre duas abas inativas, sem duplicar essa borda.
                        if previous_selected == Some(false) && !selected {
                            ui.label(egui::RichText::new("|").color(theme::muted()));
                        }
                        previous_selected = Some(selected);
                        let response = ui.add(
                            egui::Button::new(
                                egui::RichText::new(text(tab.label())).strong().color(
                                    if selected {
                                        theme::accent()
                                    } else {
                                        theme::muted()
                                    },
                                ),
                            )
                            .min_size(egui::vec2(104., 30.))
                            .selected(selected)
                            .frame(selected),
                        );
                        if response.clicked() {
                            if tab == RibbonTab::Assembly
                                && (!selected || self.current().active_part.is_some())
                            {
                                self.enter_assembly_context(ui.ctx());
                            } else {
                                self.shell.tab = tab;
                            }
                        }
                    }
                    // Mostrar/ocultar a árvore continua na busca (S), como as demais vistas.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icons::button(ui, Icon::Search, text(T::SearchTools), false, false)
                            .clicked()
                        {
                            self.shell.search_open = true;
                            self.shell.search_focus = true;
                        }
                    });
                });
                ui.add(egui::Separator::default().spacing(4.));
                self.ribbon_scroller(ui);
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
                            ui.label(egui::RichText::new("|").color(theme::muted()));
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
                                theme::header()
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
        // Grupos passam à linha seguinte quando necessário; nenhuma ferramenta
        // depende de abrir um menu ou deslocar a faixa para fora da tela.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(GROUP_HORIZONTAL_GAP, 6.);
            self.ribbon_groups(ui);
        });
    }

    fn sketch_section(
        &mut self,
        ui: &mut egui::Ui,
        label: Option<T>,
        promoted: &[Action],
        secondary: &[Action],
    ) {
        let mut actions = promoted.to_vec();
        for action in secondary {
            if !actions.contains(action) {
                actions.push(*action);
            }
        }
        self.ribbon_group(ui, label, &actions);
    }

    /// Seção Restringir: Dimencionar em destaque, as seis restrições mais usadas e, sob elas,
    /// o rótulo que abre a lista completa.
    fn restrict_group(&mut self, ui: &mut egui::Ui) {
        let enabled =
            !self.busy() && self.pending_close.is_none() && self.current().sketch_edit.is_some();
        if self.preferences.ribbon == preferences::RibbonSize::Compact {
            // Compacto: as três ferramentas e um ícone que abre a lista de restrições.
            self.ribbon_group_layout(
                ui,
                compact_group_width(4),
                None,
                |editor: &mut Self, ui: &mut egui::Ui| {
                    editor.action_button(
                        ui,
                        Action::SketchTool(sketch_editor::Tool::Dimension),
                        false,
                    );
                    editor.action_button(
                        ui,
                        Action::SketchTool(sketch_editor::Tool::Select),
                        false,
                    );
                    editor.action_button(ui, Action::SketchEdit(SketchEdit::Construction), false);
                    let response = ui
                        .add_enabled_ui(enabled, |ui| {
                            icons::button(
                                ui,
                                Icon::ConstraintPerpendicular,
                                text(T::Restrict),
                                false,
                                false,
                            )
                        })
                        .inner;
                    editor.shell.menu_rects.push(response.rect);
                    let mut chosen = None;
                    egui::Popup::menu(&response).show(|ui| {
                        if let Some(choice) = editor.restriction_menu(ui) {
                            chosen = Some(choice);
                        }
                    });
                    if let Some(choice) = chosen {
                        editor.apply_restriction(ui.ctx(), choice);
                    }
                },
            );
            return;
        }
        let width = group_width(3) + GROUP_HORIZONTAL_GAP + RESTRICT_COLUMN_WIDTH;
        // A seção já tem a própria legenda ("Restringir ▾"), também no tamanho Maior.
        self.ribbon_group_layout(ui, width, None, |editor: &mut Self, ui: &mut egui::Ui| {
            editor.action_button(ui, Action::SketchTool(sketch_editor::Tool::Dimension), true);
            // Diagnóstico ao lado de Dimensionar, do tamanho das demais ferramentas.
            editor.action_button(ui, Action::SketchTool(sketch_editor::Tool::Select), true);
            editor.action_button(ui, Action::SketchEdit(SketchEdit::Construction), true);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(RESTRICT_COLUMN_WIDTH, icons::RIBBON_BUTTON_SIZE.y),
                egui::Sense::hover(),
            );
            let mut column = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            column.spacing_mut().item_spacing = egui::vec2(2., 1.);
            column.spacing_mut().button_padding = egui::vec2(1., 1.);
            column.spacing_mut().interact_size.y = 20.;
            let mut chosen = None;
            for row in sketch_editor::GALLERY.chunks(8) {
                column.horizontal(|ui| {
                    for restriction in row {
                        let (icon, label) = sketch_editor::presentation(*restriction);
                        let armed = editor.restriction_is_armed(*restriction);
                        let button = ui
                            .add_enabled_ui(enabled, |ui| {
                                icons::tree_button(ui, icon, text(label), armed)
                            })
                            .inner;
                        #[cfg(all(test, feature = "occt"))]
                        ui.ctx().data_mut(|data| {
                            data.insert_temp(
                                egui::Id::new(("gallery_rect", text(label))),
                                button.rect,
                            );
                        });
                        if button.clicked() {
                            chosen = Some(sketch_editor::Choice::Apply(*restriction));
                        }
                    }
                });
            }
            let response = column
                .add_enabled_ui(enabled, |ui| {
                    ui.allocate_exact_size(
                        egui::vec2(RESTRICT_COLUMN_WIDTH, RESTRICT_CAPTION_HEIGHT),
                        egui::Sense::click(),
                    )
                })
                .inner
                .1;
            editor.shell.menu_rects.push(response.rect);
            let label = text(T::Restrict);
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label)
            });
            if response.hovered() && enabled {
                column
                    .painter()
                    .rect_filled(response.rect, 3, theme::blue_fill());
            }
            let ink = if enabled {
                theme::ink()
            } else {
                theme::muted()
            };
            column.painter().text(
                response.rect.left_center() + egui::vec2(2., 0.),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(10.5),
                ink,
            );
            let arrow = response.rect.right_center() - egui::vec2(7., 0.);
            column.painter().add(egui::Shape::convex_polygon(
                vec![
                    arrow + egui::vec2(-3.5, -1.5),
                    arrow + egui::vec2(3.5, -1.5),
                    arrow + egui::vec2(0., 2.5),
                ],
                ink,
                egui::Stroke::NONE,
            ));
            egui::Popup::menu(&response).show(|ui| {
                if let Some(choice) = editor.restriction_menu(ui) {
                    chosen = Some(choice);
                }
            });
            if let Some(choice) = chosen {
                editor.apply_restriction(ui.ctx(), choice);
            }
        });
    }
    fn ribbon_group(&mut self, ui: &mut egui::Ui, label: Option<T>, actions: &[Action]) {
        let compact = self.preferences.ribbon == preferences::RibbonSize::Compact;
        let width = if compact {
            compact_group_width(actions.len())
        } else {
            group_width(actions.len())
        };
        self.ribbon_group_layout(ui, width, label, |editor, ui| {
            for action in actions {
                editor.action_button(ui, *action, !compact);
            }
        });
    }
    /// Corpo do grupo conforme o tamanho da faixa: no Maior, o nome da seção fica centrado
    /// sob os botões; no Compacto, a altura cai para a dos ícones pequenos.
    fn ribbon_group_layout(
        &mut self,
        ui: &mut egui::Ui,
        width: f32,
        label: Option<T>,
        body: impl FnOnce(&mut Self, &mut egui::Ui),
    ) {
        let size = self.preferences.ribbon;
        let buttons = match size {
            preferences::RibbonSize::Compact => COMPACT_BUTTON,
            _ => icons::RIBBON_BUTTON_SIZE.y,
        };
        let caption = match (size, label) {
            (preferences::RibbonSize::Large, Some(label)) => Some(label),
            _ => None,
        };
        let height = buttons
            + if size == preferences::RibbonSize::Large {
                SECTION_CAPTION_HEIGHT
            } else {
                0.
            };
        ui.allocate_ui(egui::vec2(width, height), |ui| {
            ui.set_width(width);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.;
                ui.allocate_ui_with_layout(
                    egui::vec2(width, buttons),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        ui.set_min_height(buttons);
                        body(self, ui);
                    },
                );
                if size == preferences::RibbonSize::Large {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(width, SECTION_CAPTION_HEIGHT),
                        egui::Sense::hover(),
                    );
                    if let Some(label) = caption {
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            text(label),
                            egui::FontId::proportional(10.5),
                            theme::muted(),
                        );
                    }
                }
            });
        });
        ui.separator();
    }
    fn ribbon_groups(&mut self, ui: &mut egui::Ui) {
        self.shell.menu_rects.clear();
        match self.shell.tab {
            RibbonTab::Model => {
                self.ribbon_group(ui, Some(T::SectionSketch), &[Action::NewSketch]);
                self.ribbon_group(ui, Some(T::SectionImport), &[Action::ImportIntoPart]);
                self.ribbon_group(
                    ui,
                    Some(T::SectionCreate),
                    &[
                        Action::Solid(SolidTool::Extrude),
                        Action::Solid(SolidTool::Revolve),
                        Action::Solid(SolidTool::Loft),
                        Action::Solid(SolidTool::Sweep),
                        Action::Solid(SolidTool::Pipe),
                        Action::Solid(SolidTool::Hole),
                    ],
                );
                self.ribbon_group(
                    ui,
                    Some(T::SectionModify),
                    &[
                        Action::Solid(SolidTool::Shell),
                        Action::Solid(SolidTool::Fillet),
                        Action::Solid(SolidTool::Chamfer),
                        Action::Solid(SolidTool::RemoveFaces),
                        Action::Paint,
                    ],
                );
                self.ribbon_group(
                    ui,
                    Some(T::SectionArrange),
                    &[
                        Action::MoveBody,
                        Action::CopyBody,
                        Action::Solid(SolidTool::Join),
                        Action::Solid(SolidTool::Split),
                        Action::Solid(SolidTool::MergeFaces),
                        Action::Solid(SolidTool::Mirror),
                        Action::Solid(SolidTool::LinearPattern),
                        Action::Solid(SolidTool::CircularPattern),
                    ],
                );
                self.sketch_section(
                    ui,
                    Some(T::SectionInspect),
                    &[Action::Solid(SolidTool::CheckSolid), Action::OffsetPlane],
                    &[Action::Parameters, Action::Section],
                );
            }
            // Fluxo próprio da montagem, na ordem de trabalho: referências, peças,
            // posicionamento, cópias e organização.
            RibbonTab::Assembly => {
                self.ribbon_group(ui, Some(T::SectionReference), &[Action::ReferenceSketch]);
                self.ribbon_group(ui, Some(T::SectionPart), &[Action::NewEmptyPart]);
                self.ribbon_group(ui, Some(T::SectionImport), &[Action::ImportReference]);
                self.ribbon_group(ui, Some(T::SectionPlace), &[Action::MoveCopy]);
                self.ribbon_group(
                    ui,
                    Some(T::SectionCopy),
                    &[Action::NewInstance, Action::InstanceMirror],
                );
                self.ribbon_group(
                    ui,
                    Some(T::SectionGroups),
                    &[Action::Group, Action::Ungroup],
                );
                self.ribbon_group(
                    ui,
                    Some(T::SectionDocument),
                    &[Action::Drawing, Action::Section, Action::ExportStep],
                );
            }
            RibbonTab::Print => {
                self.ribbon_group(
                    ui,
                    Some(T::SectionPrinter),
                    &[Action::PrintProfile, Action::Calibrate],
                );
                self.ribbon_group(
                    ui,
                    Some(T::SectionPrepare),
                    &[Action::Solid(SolidTool::BedFace), Action::PrintCheck],
                );
                self.ribbon_group(
                    ui,
                    Some(T::SectionFit),
                    &[Action::Solid(SolidTool::ComponentCut), Action::FitCheck],
                );
                self.ribbon_group(
                    ui,
                    Some(T::SectionFastening),
                    &[
                        Action::Solid(SolidTool::Lip),
                        Action::Solid(SolidTool::ScrewBoss),
                        Action::Solid(SolidTool::SnapFit),
                    ],
                );
                self.ribbon_group(ui, Some(T::SectionExport), &[Action::ExportPrint]);
            }
            RibbonTab::Sketch => {
                // A aba Rascunho mantém o ferramental completo visível fora da edição;
                // os botões ficam desativados até existir um rascunho em edição.
                self.ribbon_group(ui, Some(T::SectionSketch), &[Action::NewSketch]);
                // Todas as ferramentas ficam expostas; os grupos quebram linha.
                self.sketch_section(
                    ui,
                    Some(T::SectionCreate),
                    &[
                        Action::SketchTool(sketch_editor::Tool::Line),
                        Action::SketchTool(sketch_editor::Tool::Rectangle),
                        Action::SketchTool(sketch_editor::Tool::Circle),
                        Action::SketchTool(sketch_editor::Tool::Arc),
                        Action::SketchTool(sketch_editor::Tool::Project),
                        Action::SketchText,
                    ],
                    &[
                        Action::SketchTool(sketch_editor::Tool::Line),
                        Action::SketchTool(sketch_editor::Tool::Rectangle),
                        Action::SketchTool(sketch_editor::Tool::Circle),
                        Action::SketchTool(sketch_editor::Tool::Arc),
                        Action::SketchTool(sketch_editor::Tool::Project),
                        Action::SketchText,
                        Action::SketchImage,
                    ],
                );
                self.sketch_section(
                    ui,
                    Some(T::SectionModify),
                    &[
                        Action::SketchTool(sketch_editor::Tool::Trim),
                        Action::SketchEdit(SketchEdit::Offset),
                        Action::SketchEdit(SketchEdit::Fillet),
                        Action::SketchEdit(SketchEdit::Chamfer),
                    ],
                    // Ações antes secundárias também têm botões visíveis.
                    &[
                        Action::SketchTool(sketch_editor::Tool::Trim),
                        Action::SketchEdit(SketchEdit::Offset),
                        Action::SketchEdit(SketchEdit::Fillet),
                        Action::SketchEdit(SketchEdit::Chamfer),
                        Action::SketchEdit(SketchEdit::Extend),
                        Action::SketchEdit(SketchEdit::Mirror),
                        Action::SketchEdit(SketchEdit::Delete),
                    ],
                );
                self.restrict_group(ui);
                let exit = [Action::FinishSketch, Action::CancelSketch];
                self.sketch_section(ui, Some(T::SectionFinish), &exit, &[]);
            }
        }
    }
    fn timeline(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("timeline")
            .frame(
                egui::Frame::new()
                    .fill(theme::surface())
                    .inner_margin(egui::Margin::symmetric(10, 3))
                    .stroke(egui::Stroke::new(1., theme::border())),
            )
            .show(ui, |ui| {
                let (row, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 34.),
                    egui::Sense::hover(),
                );
                let busy = self.busy();
                let (label, color) = if !busy && self.pending_failure.is_some() {
                    (text(T::Failed), theme::danger())
                } else {
                    self.status.indicator(busy)
                };
                let label = label.to_owned();
                let text_width = ui
                    .painter()
                    .layout_no_wrap(
                        label.clone(),
                        egui::TextStyle::Small.resolve(ui.style()),
                        theme::muted(),
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
                // O histórico paramétrico pertence às peças: a montagem e a impressão não o
                // mostram; desfazer, refazer e o estado continuam no rodapé.
                let history = matches!(self.shell.tab, RibbonTab::Model | RibbonTab::Sketch);
                if history {
                    controls.label(
                        egui::RichText::new(text(T::ParametricHistory))
                            .strong()
                            .size(12.),
                    );
                }
                self.action_button(&mut controls, Action::Undo, false);
                self.action_button(&mut controls, Action::Redo, false);
                ui.painter().line_segment(
                    [
                        egui::pos2(left.right(), row.top() + 4.),
                        egui::pos2(left.right(), row.bottom() - 4.),
                    ],
                    egui::Stroke::new(1., theme::border()),
                );
                if history && middle.is_positive() {
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
                        egui::Label::new(egui::RichText::new(&label).small().color(theme::muted()))
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
                                if icons::menu_item(
                                    ui,
                                    Some(action.icon()),
                                    text(action.label()),
                                    28.,
                                )
                                .clicked()
                                {
                                    chosen = Some(action);
                                }
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
