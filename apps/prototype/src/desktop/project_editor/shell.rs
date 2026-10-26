//! Estrutura visual inspirada na organização por tarefas de um CAD desktop.
//! Este módulo só escolhe ferramentas e apresenta o estado: alterações continuam
//! passando pelos comandos e pelos trabalhadores já existentes no editor.
use super::{
    icons::{self, Icon},
    *,
};
use feature_editor::Tool as SolidTool;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) enum RibbonTab {
    #[default]
    Model,
    Assembly,
    Insert,
    Inspect,
    View,
    Sketch,
}
impl RibbonTab {
    fn label(self) -> T {
        match self {
            Self::Model => T::ModelTab,
            Self::Assembly => T::Assembly,
            Self::Insert => T::InsertTab,
            Self::Inspect => T::InspectTab,
            Self::View => T::ViewTab,
            Self::Sketch => T::Sketch,
        }
    }
}

#[derive(Default)]
pub(super) struct ShellState {
    pub tab: RibbonTab,
    pub browser_visible: bool,
    pub properties_visible: bool,
    pub measure_requested: bool,
    pub search_open: bool,
    query: String,
    pub search_focus: bool,
    /// A aba contextual muda ao entrar/sair do rascunho, sem impedir que o usuário
    /// consulte as demais ferramentas enquanto a edição 2D estiver aberta.
    sketch_context: Option<(u64, SketchId)>,
}
impl ShellState {
    pub fn new() -> Self {
        Self {
            browser_visible: true,
            properties_visible: true,
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
    Assembly,
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
    Properties,
    SketchTool(sketch_editor::Tool),
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
            Self::Assembly => T::AssemblyTools,
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
            Self::Properties => T::Properties,
            Self::FinishSketch => T::FinishSketch,
            Self::SketchTool(tool) => tool.label(),
        }
    }
    fn caption(self) -> T {
        match self {
            Self::Solid(SolidTool::Fillet) => T::FilletShort,
            Self::Solid(SolidTool::Mirror) => T::MirrorShort,
            Self::SketchTool(sketch_editor::Tool::Arc) => T::ArcShort,
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
            Self::Assembly => Icon::Move,
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
            Self::Properties => Icon::Panel,
            Self::FinishSketch => Icon::Check,
            Self::SketchTool(tool) => tool.icon(),
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
    Action::Assembly,
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
    Action::Properties,
    Action::SketchTool(sketch_editor::Tool::Select),
    Action::SketchTool(sketch_editor::Tool::Line),
    Action::SketchTool(sketch_editor::Tool::Rectangle),
    Action::SketchTool(sketch_editor::Tool::Circle),
    Action::SketchTool(sketch_editor::Tool::Arc),
    Action::SketchTool(sketch_editor::Tool::Trim),
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
            Action::Browser | Action::Properties => true,
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
            Action::NewSketch | Action::SketchXY | Action::SketchXZ | Action::SketchYZ => {
                editable && self.current().sketch_edit.is_none()
            }
            Action::SketchFace => {
                editable && self.current().picked.is_some() && self.current().sketch_edit.is_none()
            }
            Action::Measure => editable && self.current().picked.is_some(),
            Action::SketchTool(_) => editable && self.current().sketch_edit.is_some(),
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
            Action::Properties => self.shell.properties_visible,
            Action::SketchTool(tool) => self
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.tool() == tool),
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
                    self.status = error;
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
            Action::Import | Action::Reference => self.import(context, action == Action::Reference),
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
            Action::Assembly => {
                self.assembly_edit = Some(assembly_editor::Draft::new(self.current().id))
            }
            Action::Parameters => self.open_parameters(),
            Action::Measure => {
                self.shell.properties_visible = true;
                self.shell.measure_requested = true;
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
            Action::Properties => self.shell.properties_visible = !self.shell.properties_visible,
            Action::SketchTool(tool) => {
                if let Some(work) = &mut self.current_mut().sketch_edit {
                    work.select_tool(tool);
                }
            }
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
            self.shell.tab = if sketch.is_some() {
                RibbonTab::Sketch
            } else {
                RibbonTab::Model
            };
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
                    });
                    for action in [Action::Save, Action::Undo, Action::Redo] {
                        self.action_button(ui, action, false);
                    }
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
                    for tab in [
                        RibbonTab::Model,
                        RibbonTab::Assembly,
                        RibbonTab::Insert,
                        RibbonTab::Inspect,
                        RibbonTab::View,
                        RibbonTab::Sketch,
                    ] {
                        if tab == RibbonTab::Sketch && sketch.is_none() {
                            continue;
                        }
                        let selected = self.shell.tab == tab;
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
                            .selected(selected)
                            .frame(selected),
                        );
                        if response.clicked() {
                            self.shell.tab = tab;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.action_button(ui, Action::Properties, false);
                        self.action_button(ui, Action::Browser, false);
                    });
                });
                ui.separator();
                egui::ScrollArea::horizontal()
                    .id_salt("ribbon_scroll")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.;
                            ui.vertical(|ui| {
                                ui.set_min_width(90.);
                                ui.set_max_width(90.);
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(90., 34.),
                                    egui::Sense::hover(),
                                );
                                icons::paint(
                                    ui.painter(),
                                    egui::Rect::from_center_size(
                                        rect.center(),
                                        egui::Vec2::splat(28.),
                                    ),
                                    if sketch.is_some() {
                                        Icon::Sketch
                                    } else {
                                        Icon::Cube
                                    },
                                    true,
                                );
                                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                    ui.label(
                                        egui::RichText::new(text(T::DesignWorkspace))
                                            .strong()
                                            .color(theme::ACCENT),
                                    );
                                    ui.label(
                                        egui::RichText::new(text(if sketch.is_some() {
                                            T::Sketch
                                        } else {
                                            T::ModelingWorkspace
                                        }))
                                        .small()
                                        .color(theme::MUTED),
                                    );
                                });
                            });
                            ui.separator();
                            self.ribbon_groups(ui);
                        });
                    });
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
                    for (index, tab) in self.tabs.iter().enumerate() {
                        let active = index == self.active;
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
                            .inner_margin(egui::Margin::symmetric(7, 1))
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
    fn ribbon_group(&mut self, ui: &mut egui::Ui, label: T, actions: &[Action]) {
        ui.vertical(|ui| {
            let width = actions.len() as f32 * 76. + actions.len().saturating_sub(1) as f32 * 3.;
            ui.set_min_width(width);
            ui.set_max_width(width);
            ui.horizontal(|ui| {
                for action in actions {
                    self.action_button(ui, *action, true);
                }
            });
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
    fn ribbon_groups(&mut self, ui: &mut egui::Ui) {
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
                    T::ModifyGroup,
                    &[
                        Action::Solid(SolidTool::Fillet),
                        Action::Solid(SolidTool::Chamfer),
                        Action::Solid(SolidTool::Shell),
                    ],
                );
                self.ribbon_group(
                    ui,
                    T::ArrangeGroup,
                    &[
                        Action::Solid(SolidTool::Mirror),
                        Action::Solid(SolidTool::LinearPattern),
                    ],
                );
                self.ribbon_group(ui, T::ManageGroup, &[Action::Parameters]);
            }
            RibbonTab::Assembly => {
                self.ribbon_group(ui, T::ArrangeGroup, &[Action::Assembly]);
                self.ribbon_group(
                    ui,
                    T::ModelTab,
                    &[
                        Action::Solid(SolidTool::Mirror),
                        Action::Solid(SolidTool::LinearPattern),
                        Action::Solid(SolidTool::CircularPattern),
                    ],
                );
                self.ribbon_group(ui, T::DocumentGroup, &[Action::Drawing, Action::ExportStep]);
            }
            RibbonTab::Insert => {
                self.ribbon_group(ui, T::ImportGroup, &[Action::Import, Action::Reference]);
                self.ribbon_group(
                    ui,
                    T::SketchPlanes,
                    &[
                        Action::SketchXY,
                        Action::SketchXZ,
                        Action::SketchYZ,
                        Action::SketchFace,
                    ],
                );
                self.ribbon_group(ui, T::ExportGroup, &[Action::ExportStl, Action::ExportStep]);
            }
            RibbonTab::Inspect => {
                self.ribbon_group(
                    ui,
                    T::DocumentGroup,
                    &[Action::Measure, Action::Drawing, Action::Parameters],
                );
                self.ribbon_group(ui, T::ExportGroup, &[Action::ExportStl, Action::ExportStep]);
            }
            RibbonTab::View => {
                self.ribbon_group(
                    ui,
                    T::CameraGroup,
                    &[
                        Action::Fit,
                        Action::Iso,
                        Action::Front,
                        Action::Top,
                        Action::Side,
                    ],
                );
                self.ribbon_group(
                    ui,
                    T::VisualControls,
                    &[Action::Perspective, Action::Shadows, Action::Triangles],
                );
                self.ribbon_group(ui, T::PanelsGroup, &[Action::Browser, Action::Properties]);
            }
            RibbonTab::Sketch => {
                self.ribbon_group(
                    ui,
                    T::CreateGroupLabel,
                    &[
                        Action::SketchTool(sketch_editor::Tool::Select),
                        Action::SketchTool(sketch_editor::Tool::Line),
                        Action::SketchTool(sketch_editor::Tool::Rectangle),
                        Action::SketchTool(sketch_editor::Tool::Circle),
                        Action::SketchTool(sketch_editor::Tool::Arc),
                    ],
                );
                self.ribbon_group(
                    ui,
                    T::ModifyGroup,
                    &[
                        Action::SketchTool(sketch_editor::Tool::Trim),
                        Action::Parameters,
                    ],
                );
                self.ribbon_group(ui, T::FinishGroup, &[Action::FinishSketch]);
            }
        }
    }
    fn timeline(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("timeline")
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .stroke(egui::Stroke::new(1., theme::BORDER)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(text(T::ParametricHistory))
                            .strong()
                            .size(12.),
                    );
                    self.action_button(ui, Action::Undo, false);
                    self.action_button(ui, Action::Redo, false);
                    ui.separator();
                    let part = self
                        .current()
                        .active_part
                        .and_then(|id| self.current().session.document().part(id))
                        .cloned();
                    if let Some(part) = part.filter(|p| !p.features.is_empty()) {
                        egui::ScrollArea::horizontal()
                            .id_salt("feature_timeline")
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    for feature in &part.features {
                                        let icon = operation_icon(&feature.operation);
                                        if ui
                                            .add_enabled_ui(!self.busy(), |ui| {
                                                icons::button(ui, icon, &feature.name, false, false)
                                            })
                                            .inner
                                            .clicked()
                                        {
                                            self.feature_edit = Some(feature_editor::Draft::edit(
                                                self.current().id,
                                                part.id,
                                                feature.clone(),
                                            ));
                                        }
                                    }
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(3., 24.),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 1, theme::ACCENT);
                                });
                            });
                    } else {
                        ui.label(
                            egui::RichText::new(text(T::TimelineEmpty))
                                .small()
                                .color(theme::MUTED),
                        );
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if self.busy() {
                        ui.spinner();
                    } else {
                        let (r, _) =
                            ui.allocate_exact_size(egui::vec2(8., 8.), egui::Sense::hover());
                        ui.painter().circle_filled(r.center(), 3., theme::GREEN);
                    }
                    ui.label(
                        egui::RichText::new(&self.status)
                            .small()
                            .color(theme::MUTED),
                    );
                    if !self.recoveries.is_empty()
                        && ui
                            .small_button(format!(
                                "{} ({})",
                                text(T::Recovery),
                                self.recoveries.len()
                            ))
                            .clicked()
                    {
                        self.show_recovery = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(text(T::UnitsMm))
                                .small()
                                .color(theme::MUTED),
                        );
                    });
                });
            });
    }
    pub(super) fn camera_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.;
            for action in [
                Action::Fit,
                Action::Iso,
                Action::Front,
                Action::Top,
                Action::Side,
            ] {
                self.action_button(ui, action, false);
            }
            ui.separator();
            egui::ComboBox::from_id_salt("camera_projection")
                .width(120.)
                .selected_text(text(match self.viewport.camera.projection {
                    nator_renderer::Projection::Orthographic => T::Orthographic,
                    nator_renderer::Projection::Perspective => T::Perspective,
                }))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.viewport.camera.projection,
                        nator_renderer::Projection::Orthographic,
                        text(T::Orthographic),
                    );
                    ui.selectable_value(
                        &mut self.viewport.camera.projection,
                        nator_renderer::Projection::Perspective,
                        text(T::Perspective),
                    );
                })
                .response
                .on_hover_text(text(T::Projection));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(text(if self.current().active_part.is_some() {
                        T::Construction
                    } else {
                        T::Assembly
                    }))
                    .small()
                    .color(theme::MUTED),
                );
            });
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

fn operation_icon(operation: &nator_document::features::Operation) -> Icon {
    use nator_document::features::Operation as O;
    match operation {
        O::Extrude { .. } => Icon::Extrude,
        O::Revolve { .. } => Icon::Revolve,
        O::Hole { .. } => Icon::Hole,
        O::Fillet { .. } => Icon::Fillet,
        O::Chamfer { .. } => Icon::Chamfer,
        O::Shell { .. } => Icon::Shell,
        O::Boolean { .. } => Icon::Boolean,
        O::Pattern { .. } => Icon::Pattern,
        O::Mirror { .. } => Icon::Mirror,
        O::Split { .. } => Icon::Split,
        O::Transform { .. } => Icon::Move,
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
