mod assembly_tools;
mod browser;
pub(crate) mod capture;
mod command_panel;
mod drawing_editor;
mod export_editor;
mod feature_editor;
mod highlight;
mod icons;
mod insert_part;
mod live_preview;
mod mesh_import;
mod navigation;
mod notifications;
mod parameter_editor;
mod point_snap;
mod profile_pick;
mod project_tree;
mod section;
mod selection_tools;
mod shell;
mod sketch_editor;
mod sketch_repair;
mod sketch_scene;
mod theme;
mod timeline;
mod view_cube;
mod views;
mod window_frame;
use crate::i18n::{Text as T, text};
use eframe::{egui, egui_wgpu};
use nator_commands::{Command, Effect, PreparedCommand, Session};
use nator_document::{BodyId, Document, InstanceId, MeshAsset, PartId, SketchId};
use nator_geometry::{Mesh, Placement, Point3, reference_bracket};
use nator_project_io::{read_project, recovery_directory, recovery_files, write_project};
use nator_renderer::{Camera, PreparedMesh, Viewport, wgpu};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub fn run() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([1280., 820.])
            .with_min_inner_size([900., 600.]),
        ..Default::default()
    };
    eframe::run_native(
        text(T::Title),
        options,
        Box::new(|context| Ok(Box::new(Editor::new(context)?))),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Selection {
    Instance(InstanceId),
    Body { part: PartId, id: BodyId },
}

/// Isolamento, transparência e bloqueio de seleção por instância. Cada mudança incrementa
/// `version`, que faz parte da chave da cena exibida.
#[derive(Debug, Clone, Default)]
pub(super) struct ViewState {
    isolated: Option<HashSet<InstanceId>>,
    transparent: HashSet<InstanceId>,
    locked: HashSet<InstanceId>,
    /// Seleção destacada na cena: ocorrências que passam por uma instância selecionada e
    /// corpos selecionados na peça em construção.
    selected_instances: HashSet<InstanceId>,
    selected_bodies: HashSet<BodyId>,
    version: u64,
}
impl ViewState {
    fn touched(&mut self) {
        self.version = self.version.wrapping_add(1);
    }
    fn in_isolation(&self, occurrence: &nator_document::assembly::Occurrence) -> bool {
        self.isolated
            .as_ref()
            .is_none_or(|set| occurrence.path.iter().any(|i| set.contains(i)))
    }
    /// Estilo de uma ocorrência. Na construção de uma peça, as demais ficam esmaecidas,
    /// salvo as subpeças dela (alguma instância do caminho é da peça ativa).
    fn style(
        &self,
        document: &Document,
        occurrence: &nator_document::assembly::Occurrence,
        part: PartId,
        active: Option<PartId>,
    ) -> Option<nator_document::Style> {
        use nator_document::Style;
        if !self.in_isolation(occurrence) {
            return None;
        }
        if occurrence.path.iter().any(|i| self.transparent.contains(i)) {
            return Some(Style::Transparent);
        }
        let inside = |active: PartId| {
            part == active
                || occurrence
                    .path
                    .iter()
                    .any(|i| document.instance(*i).is_some_and(|i| i.part == active))
        };
        Some(match active {
            Some(active) if !inside(active) => Style::Dimmed,
            _ => Style::Normal,
        })
    }
    /// Destaque de seleção sobre o estilo base; elementos translúcidos continuam translúcidos.
    fn highlight(
        &self,
        occurrence: &nator_document::assembly::Occurrence,
        body: BodyId,
        style: nator_document::Style,
    ) -> nator_document::Style {
        let chosen = self.selected_bodies.contains(&body)
            || occurrence
                .path
                .iter()
                .any(|i| self.selected_instances.contains(i));
        if chosen && !style.is_translucent() {
            nator_document::Style::Selected
        } else {
            style
        }
    }
    /// Atualiza o destaque a partir da seleção da aba; só invalida a cena se mudar.
    fn set_selection(&mut self, selected: &[Selection]) {
        let mut instances = HashSet::new();
        let mut bodies = HashSet::new();
        for item in selected {
            match *item {
                Selection::Instance(id) => {
                    instances.insert(id);
                }
                Selection::Body { id, .. } => {
                    bodies.insert(id);
                }
            }
        }
        if instances != self.selected_instances || bodies != self.selected_bodies {
            self.selected_instances = instances;
            self.selected_bodies = bodies;
            self.touched();
        }
    }
    /// Ocorrências bloqueadas ou fora do isolamento não podem ser selecionadas na área 3D.
    fn pickable(&self, occurrence: &nator_document::assembly::Occurrence) -> bool {
        self.in_isolation(occurrence) && !occurrence.path.iter().any(|i| self.locked.contains(i))
    }
}

/// Identifica a cena exibida: muda com a revisão, o estado visual e a peça em construção.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SceneKey {
    tab: u64,
    revision: u64,
    view: u64,
    active: Option<PartId>,
    /// Geração da pré-visualização exibida; zero para a cena do documento.
    preview: u64,
}

struct Tab {
    id: u64,
    session: Session,
    path: Option<PathBuf>,
    camera: Camera,
    scene_bounds: Option<nator_geometry::Bounds>,
    /// Elemento principal da seleção, usado pelas ferramentas e pelos menus da árvore.
    selection: Option<Selection>,
    /// Todos os elementos selecionados (Ctrl+clique, seleção por janela), incluindo o principal.
    selected: Vec<Selection>,
    active_part: Option<PartId>,
    sketch_edit: Option<sketch_editor::Workspace>,
    plane_pick: Option<sketch_scene::PlanePicker>,
    camera_transition: Option<sketch_scene::CameraTransition>,
    changed_at: Instant,
    recovery_path: PathBuf,
    recovery_source: Option<PathBuf>,
    recovery_state: Option<u64>,
    picked: Option<nator_document::assembly::Pick>,
    measurements: Vec<Point3>,
    /// Clique de referência para ângulo entre faces e distância entre corpos.
    reference: Option<nator_document::assembly::Pick>,
    /// Controles visuais da aba: não entram no documento nem no desfazer.
    view: ViewState,
    /// Corte de seção da área 3D, também apenas visual.
    section: Option<section::Section>,
    drag_start: Option<egui::Pos2>,
}
impl Tab {
    fn modified(&self) -> bool {
        self.session.is_dirty()
            || self
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.pending.is_some())
    }
    fn new(
        id: u64,
        document: Document,
        path: Option<PathBuf>,
        recovered: Option<PathBuf>,
    ) -> Result<Self, String> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let recovery_path =
            recovery_directory().join(format!("{}-{nonce}-{id}.3dnator", document.id()));
        let mut session = Session::new(document).map_err(|e| e.to_string())?;
        if recovered.is_some() {
            session.mark_unsaved();
        }
        Ok(Self {
            id,
            session,
            path,
            camera: Camera::default(),
            scene_bounds: None,
            selection: None,
            selected: Vec::new(),
            active_part: None,
            sketch_edit: None,
            plane_pick: None,
            camera_transition: None,
            changed_at: Instant::now(),
            recovery_path,
            recovery_source: recovered,
            recovery_state: None,
            picked: None,
            measurements: Vec::new(),
            reference: None,
            view: ViewState::default(),
            section: None,
            drag_start: None,
        })
    }
    fn select(&mut self, selection: Option<Selection>) {
        self.selected = selection.into_iter().collect();
        self.selection = selection;
        self.refresh_selection();
    }
    /// Ctrl+clique: acrescenta ou retira um elemento, que passa a ser o principal.
    fn toggle(&mut self, item: Selection) {
        if let Some(index) = self.selected.iter().position(|s| *s == item) {
            self.selected.remove(index);
            self.selection = self.selected.last().copied();
        } else {
            self.selected.push(item);
            self.selection = Some(item);
        }
        self.refresh_selection();
    }
    /// Seleção por janela: substitui ou, com Ctrl, acrescenta.
    fn select_many(&mut self, items: Vec<Selection>, additive: bool) {
        if !additive {
            self.selected.clear();
        }
        for item in items {
            if !self.selected.contains(&item) {
                self.selected.push(item);
            }
        }
        self.selection = self.selected.last().copied();
        self.refresh_selection();
    }
    fn is_selected(&self, item: Selection) -> bool {
        self.selected.contains(&item)
    }
    /// Descarta elementos que deixaram de existir e atualiza campos e destaque.
    fn refresh_selection(&mut self) {
        let document = self.session.document();
        self.selected.retain(|item| match *item {
            Selection::Instance(id) => document.instance(id).is_some(),
            Selection::Body { id, .. } => document.body(id).is_some(),
        });
        if self.selection.is_some_and(|s| !self.selected.contains(&s)) {
            self.selection = self.selected.last().copied();
        }
        self.view.set_selection(&self.selected);
    }
    fn changed(&mut self) {
        self.picked = None;
        self.changed_at = Instant::now();
        self.recovery_state = None;
        if !self.session.is_dirty() {
            let _ = nator_project_io::remove_recovery(&self.recovery_path);
        }
        if self
            .active_part
            .is_some_and(|p| self.session.document().part(p).is_none())
        {
            self.active_part = None;
        }
        self.refresh_selection();
        if self
            .sketch_edit
            .as_ref()
            .is_some_and(|w| self.session.document().sketch(w.id).is_none())
        {
            self.sketch_edit = None;
        }
    }
}

enum Outcome {
    Picked {
        tab: u64,
        revision: u64,
        pick: Option<nator_document::assembly::Pick>,
        mode: navigation::PickMode,
        /// Ctrl pressionado no clique: alterna o elemento na seleção.
        additive: bool,
        /// Referência escolhida para o diálogo de operações, quando ele esperava um clique.
        topology: Option<Result<feature_editor::Kept, String>>,
    },
    Drawing {
        tab: u64,
        revision: u64,
        sheet: nator_drawing::Sheet,
    },
    DrawingReference {
        tab: u64,
        revision: u64,
        first: bool,
        reference: nator_document::drawing::DrawingTopologyRef,
    },
    Prepared {
        tab: u64,
        command: PreparedCommand,
    },
    /// Edição recusada por uma falha de recálculo atribuída a uma operação.
    Rejected {
        tab: u64,
        command: Command,
        error: nator_commands::CommandError,
    },
    SketchCanceled {
        tab: u64,
        id: SketchId,
        command: PreparedCommand,
    },
    SketchSolved {
        tab: u64,
        revision: u64,
        id: SketchId,
        report: nator_sketch::solver::Report,
        prepared: Result<PreparedCommand, nator_commands::CommandError>,
        chain_closed: bool,
    },
    Imported {
        tab: u64,
        revision: u64,
        destination: Option<PartId>,
        name: String,
        geometry: Arc<MeshAsset>,
    },
    /// STL lido sem escala, aguardando a confirmação da unidade.
    MeshRead {
        tab: u64,
        revision: u64,
        destination: Option<PartId>,
        name: String,
        mesh: Mesh,
    },
    Opened {
        document: Document,
        path: Option<PathBuf>,
        recovery: Option<PathBuf>,
    },
    Saved {
        tab: u64,
        state: u64,
        path: PathBuf,
        close_after: bool,
    },
    Scene {
        key: SceneKey,
        bounds: nator_geometry::Bounds,
        mesh: Option<Mesh>,
        prepared: Option<PreparedMesh>,
    },
    Recovered {
        tab: u64,
        state: u64,
    },
    Exported,
    Measured {
        tab: u64,
        revision: u64,
        lines: Vec<String>,
    },
    ExportSummary {
        tab: u64,
        revision: u64,
        text: String,
    },
    Cancelled,
}
struct Worker {
    receiver: Receiver<Result<Outcome, String>>,
    recovery: Option<(u64, u64)>,
}

pub struct Editor {
    gpu: egui_wgpu::RenderState,
    viewport: Viewport,
    texture: egui::TextureId,
    tabs: Vec<Tab>,
    active: usize,
    next_tab: u64,
    worker: Option<Worker>,
    status: notifications::Status,
    scene: Option<Mesh>,
    rendered: Option<SceneKey>,
    /// Chave da preparação de cena; liberada ao receber a cena calculada.
    requested: Option<SceneKey>,
    import_scale: f64,
    pending_close: Option<u64>,
    want_exit: bool,
    allow_exit: bool,
    recoveries: Vec<PathBuf>,
    show_recovery: bool,
    parameters_edit: Option<parameter_editor::Draft>,
    feature_edit: Option<feature_editor::Draft>,
    drawing_edit: Option<drawing_editor::Draft>,
    export_edit: Option<export_editor::Draft>,
    pick_mode: navigation::PickMode,
    alignment: Option<(u64, u64, nator_document::assembly::Pick)>,
    move_freely: bool,
    rotate_angle: f64,
    /// Linhas do último relatório de medição, já formatadas para exibição.
    measure_report: Vec<String>,
    /// Disposição da interface; não modifica o documento nem o histórico.
    shell: shell::ShellState,
    timeline: timeline::TimelineState,
    /// Edição rejeitada por falha de recálculo, aguardando correção, supressão ou cancelamento.
    pending_failure: Option<timeline::PendingFailure>,
    /// Aba cuja falha pendente está sendo resolvida pelo comando em preparação.
    resolving: Option<u64>,
    preview: live_preview::LivePreview,
    /// Área 3D do último quadro, para ancorar os painéis de comando.
    canvas_rect: egui::Rect,
    /// Pré-seleção sob o cursor e dados de realce por geometria.
    hover: Option<highlight::Hover>,
    highlight_cache: highlight::Cache,
    profile_cache: profile_pick::Cache,
    point_cache: point_snap::Cache,
    /// Janela de seleção, pressão longa e lista de sobrepostos.
    gestures: selection_tools::SelectionGestures,
    /// Painel de montagem ou posicionamento aberto.
    assembly_panel: Option<assembly_tools::Panel>,
    /// Renomeação pela árvore e ação escolhida num menu de subnível.
    rename: Option<browser::RenameDraft>,
    /// Painel do corte de seção aberto.
    section_edit: Option<section::SectionDraft>,
    /// Reparo do rascunho em edição.
    sketch_repair: Option<sketch_repair::RepairDraft>,
    /// Miniaturas de projetos em disco (recuperação), por caminho.
    thumbnails: std::collections::HashMap<PathBuf, Option<egui::TextureHandle>>,
    /// Inserção de peça de outra aba.
    insert_part: Option<insert_part::InsertDraft>,
    /// Confirmação de unidade e escala de um STL lido.
    mesh_import: Option<mesh_import::MeshImportDraft>,
}

/// Origem de uma importação pelo diálogo de arquivos (ou o suporte de referência embutido).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ImportSource {
    Reference,
    Any,
    Step,
    Stl,
}

/// Cena estilizada e dados de GPU. As arestas vivas já foram analisadas uma vez por geometria.
fn prepare_scene(
    document: &Document,
    view: &ViewState,
    active: Option<PartId>,
) -> Result<Option<(Mesh, PreparedMesh)>, String> {
    prepare_scene_with_removal(document, view, active, None)
}

fn prepare_scene_with_removal(
    document: &Document,
    view: &ViewState,
    active: Option<PartId>,
    removal: Option<(PartId, Mesh)>,
) -> Result<Option<(Mesh, PreparedMesh)>, String> {
    use nator_document::Style;
    let Some(mut scene) = document
        .scene_styled_bodies(|occurrence, part, body| {
            view.style(document, occurrence, part, active)
                .map(|style| view.highlight(occurrence, body, style))
        })
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let mut styles = scene
        .styles
        .iter()
        .map(|style| match style {
            Style::Normal => nator_renderer::STYLE_NORMAL,
            Style::Dimmed => nator_renderer::STYLE_DIMMED,
            Style::Transparent => nator_renderer::STYLE_TRANSPARENT,
            Style::Selected => nator_renderer::STYLE_SELECTED,
        })
        .collect::<Vec<_>>();
    if let Some((part_id, removed)) = removal {
        let mut vertices = scene.mesh.vertices().to_vec();
        let mut triangles = scene.mesh.triangles().to_vec();
        let (_, creases) = removed.edge_analysis(Some(std::f64::consts::FRAC_PI_6));
        for occurrence in document
            .occurrences()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|o| {
                document
                    .instance(o.instance)
                    .is_some_and(|i| i.part == part_id)
            })
        {
            let Some(part) = document.part(part_id) else {
                continue;
            };
            if view.style(document, &occurrence, part.id, active).is_none() {
                continue;
            }
            let offset =
                u32::try_from(vertices.len()).map_err(|_| "Prévia excede o limite de vértices.")?;
            vertices.extend(
                removed
                    .vertices()
                    .iter()
                    .map(|p| occurrence.placement.apply(*p)),
            );
            triangles.extend(removed.triangles().iter().map(|t| t.map(|i| i + offset)));
            scene.creases.extend_from_slice(&creases);
            styles.extend(std::iter::repeat_n(
                nator_renderer::STYLE_REMOVAL,
                removed.triangles().len(),
            ));
            scene.colors.extend(std::iter::repeat_n(
                nator_geometry::RgbColor::new([224, 50, 58]),
                removed.triangles().len(),
            ));
        }
        scene.mesh = Mesh::new(vertices, triangles).map_err(|e| e.to_string())?;
    }
    let prepared = PreparedMesh::with_appearance(
        &scene.mesh,
        &scene.creases,
        &styles,
        scene.opaque_triangles,
        &scene.colors,
    )?;
    Ok(Some((scene.mesh, prepared)))
}

impl Editor {
    fn new(
        context: &eframe::CreationContext<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let gpu = context
            .wgpu_render_state
            .clone()
            .ok_or("Renderizador wgpu indisponível.")?;
        Self::from_gpu(gpu, &context.egui_ctx, Document::new(text(T::Untitled))?)
    }

    fn from_gpu(
        gpu: egui_wgpu::RenderState,
        context: &egui::Context,
        document: Document,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut viewport = Viewport::new(&gpu.device, &reference_bracket()?)?;
        viewport.clear(&gpu.device);
        let texture = gpu.renderer.write().register_native_texture(
            &gpu.device,
            viewport.view(),
            wgpu::FilterMode::Linear,
        );
        let tab = Tab::new(1, document, None, None)?;
        theme::apply(context);
        viewport.set_background(nator_geometry::RgbColor::new([233, 238, 245]));
        let recoveries = recovery_files(&recovery_directory()).unwrap_or_default();
        Ok(Self {
            gpu,
            viewport,
            texture,
            tabs: vec![tab],
            active: 0,
            next_tab: 2,
            worker: None,
            status: notifications::Status::default(),
            scene: None,
            rendered: None,
            requested: None,
            import_scale: 1.,
            pending_close: None,
            want_exit: false,
            allow_exit: false,
            show_recovery: !recoveries.is_empty(),
            recoveries,
            parameters_edit: None,
            feature_edit: None,
            drawing_edit: None,
            export_edit: None,
            pick_mode: navigation::PickMode::Face,
            alignment: None,
            move_freely: false,
            rotate_angle: 15.,
            measure_report: Vec::new(),
            shell: shell::ShellState::new(),
            timeline: timeline::TimelineState::default(),
            pending_failure: None,
            resolving: None,
            preview: live_preview::LivePreview::default(),
            canvas_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 820.)),
            hover: None,
            highlight_cache: highlight::Cache::default(),
            point_cache: point_snap::Cache::default(),
            profile_cache: profile_pick::Cache::default(),
            gestures: selection_tools::SelectionGestures::default(),
            assembly_panel: None,
            rename: None,
            section_edit: None,
            sketch_repair: None,
            thumbnails: std::collections::HashMap::new(),
            insert_part: None,
            mesh_import: None,
        })
    }
    fn busy(&self) -> bool {
        self.worker.is_some()
    }
    fn current(&self) -> &Tab {
        &self.tabs[self.active]
    }
    fn current_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }
    fn find_tab(&mut self, id: u64) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }
    fn invalidate_scene(&mut self) {
        // Manter a última cena válida enquanto a revisão nova é preparada evita
        // apagar o modelo a cada segmento editado no rascunho.
        self.rendered = None;
        self.requested = None;
    }
    fn switch(&mut self, index: usize) {
        if index >= self.tabs.len() || index == self.active {
            return;
        }
        self.current_mut().camera = self.viewport.camera.clone();
        self.active = index;
        self.shell.measure_open = false;
        self.viewport.clear(&self.gpu.device);
        self.scene = None;
        self.viewport.camera = self.current().camera.clone();
        self.invalidate_scene();
    }
    fn add_tab(
        &mut self,
        document: Document,
        path: Option<PathBuf>,
        recovered: Option<PathBuf>,
    ) -> Result<(), String> {
        if let Some(path) = &path
            && let Some(index) = self.tabs.iter().position(|t| t.path.as_ref() == Some(path))
        {
            self.switch(index);
            return Ok(());
        }
        let id = self.next_tab;
        self.next_tab = id.checked_add(1).ok_or("Abas esgotadas.")?;
        let tab = Tab::new(id, document, path, recovered)?;
        self.tabs.push(tab);
        let index = self.tabs.len() - 1;
        self.switch(index);
        Ok(())
    }
    fn execute(&mut self, command: Command) {
        if self.busy() {
            return;
        }
        match self.current_mut().session.execute(command) {
            Ok(effect) => {
                let tab = self.current_mut();
                if let Effect::Imported { part, body } = effect {
                    tab.selection = Some(Selection::Body { part, id: body });
                }
                if let Effect::SketchCreated { part, .. } = effect {
                    tab.active_part = Some(part);
                }
                let consumed = matches!(effect, Effect::Imported { .. })
                    && tab.sketch_edit.as_ref().is_some_and(|w| {
                        tab.session
                            .document()
                            .sketch(w.id)
                            .is_some_and(|s| !s.visible)
                    });
                tab.changed();
                self.invalidate_scene();
                if consumed {
                    self.finish_sketch();
                }
                if let Effect::SketchCreated { part, sketch } = effect {
                    self.begin_sketch(part, sketch);
                }
                self.follow_assembly_effect(effect);
            }
            Err(error) => self.status.error(error.to_string()),
        }
    }
    /// Efeitos dos comandos de montagem na aba ativa: o rascunho de referência novo abre
    /// para edição; a peça nova entra em construção, pronta para o primeiro rascunho.
    fn follow_assembly_effect(&mut self, effect: Effect) {
        match effect {
            Effect::ReferenceSketchCreated { sketch } => self.begin_reference_sketch(sketch),
            Effect::PartCreated { part } => {
                let instance = self
                    .current()
                    .session
                    .document()
                    .instances()
                    .iter()
                    .find(|i| i.part == part)
                    .map(|i| i.id);
                if let Some(instance) = instance {
                    self.current_mut()
                        .select(Some(Selection::Instance(instance)));
                }
                self.current_mut().active_part = Some(part);
                self.shell.tab = shell::RibbonTab::Model;
            }
            _ => {}
        }
    }
    fn start(
        &mut self,
        context: &egui::Context,
        recovery: Option<(u64, u64)>,
        task: impl FnOnce() -> Result<Outcome, String> + Send + 'static,
    ) {
        if self.busy() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let context = context.clone();
        match std::thread::Builder::new()
            .name("project-worker".into())
            .spawn(move || {
                let result = task();
                let _ = sender.send(result);
                context.request_repaint();
            }) {
            Ok(_) => {
                self.worker = Some(Worker { receiver, recovery });
                self.status.set(text(T::Loading));
            }
            Err(error) => self.status.error(error.to_string()),
        }
    }
    fn collect(&mut self, context: &egui::Context) {
        let Some(result) = self
            .worker
            .as_ref()
            .and_then(|w| match w.receiver.try_recv() {
                Ok(value) => Some(value),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(text(T::Failed).into())),
                Err(mpsc::TryRecvError::Empty) => None,
            })
        else {
            return;
        };
        let recovery = self.worker.take().and_then(|w| w.recovery);
        self.status.set(text(T::Ready));
        match result {
            Ok(Outcome::Picked {
                tab,
                revision,
                pick,
                mode,
                additive,
                topology,
            }) => {
                let current = self
                    .find_tab(tab)
                    .is_some_and(|t| t.session.revision() == revision);
                // Com um painel esperando, o clique pertence ao campo de seleção ativo.
                let consumed = topology.is_some();
                if current
                    && let (Some(result), Some(p), Some(draft)) =
                        (topology, pick.as_ref(), self.feature_edit.as_mut())
                {
                    match result {
                        Ok(_) if p.part != draft.part() => {
                            self.status.warning(text(T::PickOtherPart));
                        }
                        Ok(kept) => {
                            self.status.set(feature_editor::Draft::describe(&kept));
                            draft.apply_pick(p.body, kept);
                        }
                        Err(error) => self.status.error(error),
                    }
                }
                if let Some(target) = self
                    .find_tab(tab)
                    .filter(|t| t.session.revision() == revision && !consumed)
                {
                    if let Some(mut p) = pick {
                        navigation::snap_pick(&mut p, mode);
                        let item = if target.active_part == Some(p.part) {
                            Selection::Body {
                                part: p.part,
                                id: p.body,
                            }
                        } else {
                            Selection::Instance(p.instance)
                        };
                        if additive {
                            target.toggle(item);
                        } else {
                            target.select(Some(item));
                        }
                        target.picked = Some(p);
                    } else if !additive {
                        target.picked = None;
                        target.select(None);
                    }
                }
            }
            Ok(Outcome::Prepared { tab, command }) => self.commit_prepared(tab, command),
            Ok(Outcome::Rejected {
                tab,
                command,
                error,
            }) => self.reject_command(tab, command, error),
            Ok(Outcome::SketchCanceled { tab, id, command }) => {
                let active = self.current().id == tab;
                let mut restored = false;
                if let Some(target) = self.find_tab(tab)
                    && target
                        .sketch_edit
                        .as_ref()
                        .is_some_and(|work| work.id == id)
                {
                    match target.session.commit(command) {
                        Ok(_) => {
                            if let Some(work) = target.sketch_edit.as_mut() {
                                work.pending = None;
                                work.reset_drawing(context);
                            }
                            if !active && let Some(work) = target.sketch_edit.take() {
                                target.camera.copy_orientation(&work.previous_camera);
                                target.camera_transition = None;
                            }
                            target.changed();
                            self.requested = None;
                            restored = true;
                        }
                        Err(error) => self.status.error(error.to_string()),
                    }
                }
                if restored {
                    self.pending_failure
                        .take_if(|failure| failure.tab == tab && failure.sketch == Some(id));
                    if active {
                        self.finish_sketch();
                    }
                }
            }
            Ok(Outcome::SketchSolved {
                tab,
                revision,
                id,
                report,
                prepared,
                chain_closed,
            }) => {
                let mut rejected = None;
                if let Some(target) = self
                    .find_tab(tab)
                    .filter(|t| t.session.revision() == revision)
                    && let Some(work) = target.sketch_edit.as_mut().filter(|w| w.id == id)
                {
                    work.status = report.status.clone();
                    work.redundant = report.redundant;
                    if report.status == nator_sketch::solver::Status::Conflict {
                        work.pending = Some(report.sketch);
                        self.status.warning(text(T::SketchConflict));
                    } else {
                        work.pending = None;
                        match prepared.and_then(|command| Ok(target.session.commit(command)?)) {
                            Ok(_) => {
                                if chain_closed && work.tool() == sketch_editor::Tool::Line {
                                    work.reset_drawing(context);
                                }
                                target.changed();
                                self.requested = None;
                            }
                            Err(error) => {
                                work.pending = Some(report.sketch.clone());
                                rejected = Some((error, report.sketch));
                            }
                        }
                    }
                }
                // A alteração do rascunho quebrou uma operação: ela fica pendente e o
                // usuário decide entre corrigir, suprimir ou cancelar.
                if let Some((error, sketch)) = rejected {
                    if error.failure.is_some() {
                        self.status.record(&error);
                    } else {
                        self.status.error(&error);
                    }
                    if let Some(failure) = error.failure {
                        self.pending_failure = Some(timeline::PendingFailure {
                            tab,
                            command: Command::UpdateSketch { id, sketch },
                            failure,
                            sketch: Some(id),
                            fixing: false,
                        });
                    }
                }
            }
            Err(error) => {
                if let Some((id, state)) = recovery
                    && let Some(tab) = self.find_tab(id)
                {
                    tab.recovery_state = Some(state);
                }
                self.status.error(format!(
                    "{}: {error}",
                    if recovery.is_some() {
                        text(T::RecoveryFailed)
                    } else {
                        text(T::Failed)
                    }
                ));
            }
            Ok(Outcome::Drawing {
                tab,
                revision,
                sheet,
            }) => {
                if self
                    .tabs
                    .iter()
                    .any(|t| t.id == tab && t.session.revision() == revision)
                    && let Some(d) = self.drawing_edit.as_mut().filter(|d| d.tab == tab)
                {
                    d.preview = Some(sheet);
                    d.preview_revision = Some(revision);
                }
            }
            Ok(Outcome::DrawingReference {
                tab,
                revision,
                first,
                reference,
            }) => {
                if self
                    .tabs
                    .iter()
                    .any(|t| t.id == tab && t.session.revision() == revision)
                    && let Some(draft) = self.drawing_edit.as_mut().filter(|d| d.tab == tab)
                {
                    draft.keep_reference(first, reference);
                }
            }
            Ok(Outcome::Cancelled) => {}
            Ok(Outcome::Imported {
                tab,
                revision,
                destination,
                name,
                geometry,
            }) => {
                match deliver_import(&mut self.tabs, tab, revision, destination, name, geometry) {
                    Ok(true) => {
                        self.status.set(text(T::Imported));
                        self.requested = None;
                    }
                    Ok(false) => self.status.warning(text(T::StaleResult)),
                    Err(error) => self.status.error(error),
                }
            }
            Ok(Outcome::MeshRead {
                tab,
                revision,
                destination,
                name,
                mesh,
            }) => {
                self.mesh_import = Some(mesh_import::MeshImportDraft::new(
                    tab,
                    revision,
                    destination,
                    name,
                    mesh,
                    self.import_scale,
                ));
            }
            Ok(Outcome::Opened {
                document,
                path,
                recovery,
            }) => {
                if let Err(error) = self.add_tab(document, path, recovery) {
                    self.status.error(error);
                }
            }
            Ok(Outcome::Saved {
                tab,
                state,
                path,
                close_after,
            }) => {
                if let Some(target) = self.find_tab(tab) {
                    target.path = Some(path);
                    target.session.mark_saved(state);
                    target.recovery_state = None;
                }
                self.status.set(text(T::Saved));
                if close_after {
                    self.close_tab(tab);
                }
            }
            Ok(Outcome::Scene {
                key,
                bounds,
                mesh,
                prepared,
            }) => {
                // `requested` representa trabalho em voo, não uma cena em cache.
                // Depois de exibir uma prévia, cancelar volta à mesma chave da cena
                // original; uma chave concluída aqui bloquearia essa restauração.
                if self.requested == Some(key) {
                    self.requested = None;
                }
                if self.scene_key() == key {
                    self.show_scene(key, bounds, mesh, prepared);
                }
            }
            Ok(Outcome::Recovered { tab, state }) => {
                if let Some(target) = self.find_tab(tab) {
                    target.recovery_state = Some(state);
                }
                self.status.set(text(T::RecoverySaved));
            }
            Ok(Outcome::Exported) => self.status.set(text(T::Exported)),
            Ok(Outcome::ExportSummary {
                tab,
                revision,
                text,
            }) => {
                if let Some(draft) = self.export_edit.as_mut() {
                    draft.set_summary(tab, revision, text);
                }
            }
            Ok(Outcome::Measured {
                tab,
                revision,
                lines,
            }) => {
                if self
                    .find_tab(tab)
                    .is_some_and(|t| t.session.revision() == revision)
                {
                    self.measure_report = lines;
                }
            }
        }
    }
    /// Publica um comando preparado na aba de origem, com os efeitos sobre seleção,
    /// rascunho, folhas e falhas pendentes.
    fn commit_prepared(&mut self, tab: u64, command: PreparedCommand) {
        let mut saved = None;
        let mut assembly_effect = None;
        let mut created = None;
        let mut consumed = false;
        let active_tab = self.current().id;
        if let Some(target) = self.find_tab(tab) {
            match target.session.commit(command) {
                Ok(effect) => {
                    if let Effect::SketchCreated { part, sketch } = effect {
                        target.active_part = Some(part);
                        let mut work = sketch_editor::Workspace::new(sketch);
                        work.original_sketch = target
                            .session
                            .document()
                            .sketch(sketch)
                            .map(|record| record.sketch.clone());
                        work.previous_camera = target.camera.clone();
                        work.placement = target
                            .plane_pick
                            .as_ref()
                            .map(|p| p.placement)
                            .unwrap_or_default();
                        if active_tab != tab {
                            target.sketch_edit = Some(work);
                            target.plane_pick = None;
                        }
                        created = Some((part, sketch));
                    }
                    if let Effect::Imported { part, body } = effect {
                        target.select(Some(Selection::Body { part, id: body }));
                        consumed = target.sketch_edit.as_ref().is_some_and(|w| {
                            target
                                .session
                                .document()
                                .sketch(w.id)
                                .is_some_and(|s| !s.visible)
                        });
                        if consumed
                            && active_tab != tab
                            && let Some(work) = target.sketch_edit.take()
                        {
                            target.camera.copy_orientation(&work.previous_camera);
                            target.camera_transition = None;
                        }
                    }
                    if let Effect::SheetSaved { id } = effect {
                        saved = Some(id);
                    }
                    if matches!(
                        effect,
                        Effect::ReferenceSketchCreated { .. } | Effect::PartCreated { .. }
                    ) {
                        assembly_effect = Some(effect);
                    }
                    target.changed();
                    self.requested = None;
                    self.resolve_pending_failure(tab);
                }
                Err(error) => self.status.error(error.to_string()),
            }
        }
        if tab == active_tab {
            if let Some((part, sketch)) = created {
                self.begin_sketch(part, sketch);
            }
            if consumed {
                self.finish_sketch();
            }
            if let Some(effect) = assembly_effect {
                self.follow_assembly_effect(effect);
            }
        }
        if let Some(id) = saved
            && let Some(draft) = self.drawing_edit.as_mut().filter(|d| d.tab == tab)
        {
            draft.keep_sheet_id(id);
        }
    }
    /// Edição recusada: uma falha atribuída a uma operação aguarda a decisão do usuário.
    fn reject_command(&mut self, tab: u64, command: Command, error: nator_commands::CommandError) {
        self.resolving = None;
        if error.failure.is_some() {
            self.status.record(&error);
        } else {
            self.status.error(&error);
        }
        if let Some(failure) = error.failure
            && self.tabs.iter().any(|t| t.id == tab)
        {
            // Uma resolução que falha em outra operação mantém o rascunho pendente.
            let sketch = self
                .pending_failure
                .take_if(|p| p.tab == tab)
                .and_then(|p| p.sketch);
            self.pending_failure = Some(timeline::PendingFailure {
                tab,
                command,
                failure,
                sketch,
                fixing: false,
            });
        }
    }
    /// Publica uma cena preparada no viewport, preservando o enquadramento em coordenadas
    /// do mundo quando os limites mudam.
    fn show_scene(
        &mut self,
        key: SceneKey,
        bounds: nator_geometry::Bounds,
        mesh: Option<Mesh>,
        prepared: Option<PreparedMesh>,
    ) {
        let mut camera = self.viewport.camera.clone();
        let previous = self.current().scene_bounds.or_else(|| {
            self.current()
                .sketch_edit
                .as_ref()
                .map(|_| sketch_scene::EMPTY_BOUNDS)
        });
        if let Some(previous) = previous {
            camera.reframe(previous, bounds);
            if let Some(transition) = &mut self.current_mut().camera_transition {
                transition.from.reframe(previous, bounds);
                transition.to.reframe(previous, bounds);
            }
        } else {
            let [w, h] = self.viewport.size();
            camera.fit(f64::from(w) / f64::from(h));
        }
        let result = if let Some(prepared) = prepared {
            self.viewport.set_prepared_mesh(&self.gpu.device, &prepared)
        } else {
            self.viewport.clear(&self.gpu.device);
            Ok(())
        };
        match result {
            Ok(()) => {
                self.current_mut().scene_bounds = Some(bounds);
                self.scene = mesh;
                self.rendered = Some(key);
                self.viewport.camera = camera;
            }
            Err(error) => self.status.error(error),
        }
    }
    fn scene_key(&self) -> SceneKey {
        SceneKey {
            tab: self.current().id,
            revision: self.current().session.revision(),
            view: self.current().view.version,
            active: self.current().active_part,
            preview: self.preview.shown,
        }
    }
    fn ensure_scene(&mut self, context: &egui::Context) {
        let key = self.scene_key();
        if self.rendered == Some(key) || self.requested == Some(key) || self.busy() {
            return;
        }
        self.requested = Some(key);
        let document = self.current().session.document().clone();
        let view = self.current().view.clone();
        self.start(context, None, move || {
            let scene = prepare_scene(&document, &view, key.active)?;
            let (mesh, prepared) = match scene {
                Some((mesh, prepared)) => (Some(mesh), Some(prepared)),
                None => (None, None),
            };
            let bounds = mesh
                .as_ref()
                .map(Mesh::bounds)
                .unwrap_or_else(|| sketch_scene::document_bounds(&document, &view));
            Ok(Outcome::Scene {
                key,
                bounds,
                mesh,
                prepared,
            })
        });
    }
    /// Importa um modelo para `destination` (peça em construção) ou, sem destino, como
    /// peça nova. O filtro do diálogo segue a origem escolhida.
    fn import(
        &mut self,
        context: &egui::Context,
        source: ImportSource,
        destination: Option<PartId>,
    ) {
        let tab = self.current().id;
        let revision = self.current().session.revision();
        let document = self.current().session.document().clone();
        self.start(context, None, move || {
            let (mesh, name) = if source == ImportSource::Reference {
                (
                    reference_bracket().map_err(|e| e.to_string())?,
                    text(T::Reference).into(),
                )
            } else {
                // Os filtros dos diálogos no Linux distinguem maiúsculas: "PECA.STEP" também
                // precisa aparecer.
                let (label, extensions): (_, &[&str]) = match source {
                    ImportSource::Step => (text(T::StepFilter), &["step", "stp", "STEP", "STP"]),
                    ImportSource::Stl => (text(T::StlFilter), &["stl", "STL"]),
                    _ => (
                        text(T::ModelFilter),
                        &["stl", "step", "stp", "STL", "STEP", "STP"],
                    ),
                };
                let Some(path) = rfd::FileDialog::new()
                    .add_filter(label, extensions)
                    .pick_file()
                else {
                    return Ok(Outcome::Cancelled);
                };
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| text(T::MeshBody).into());
                if path.extension().is_some_and(|e| {
                    e.eq_ignore_ascii_case("step") || e.eq_ignore_ascii_case("stp")
                }) {
                    #[cfg(feature = "occt")]
                    {
                        let command = PreparedCommand::new(
                            document,
                            revision,
                            Command::ImportStep {
                                path,
                                destination,
                                name,
                            },
                        )
                        .map_err(|e| e.to_string())?;
                        return Ok(Outcome::Prepared { tab, command });
                    }
                    #[cfg(not(feature = "occt"))]
                    {
                        let _ = document;
                        nator_kernel::require_available().map_err(|e| e.to_string())?;
                        return Err(text(T::KernelUnavailable).into());
                    }
                }
                // Sem escala: o painel de confirmação escolhe a unidade do arquivo.
                return Ok(Outcome::MeshRead {
                    tab,
                    revision,
                    destination,
                    name,
                    mesh: nator_project_io::read_stl(&path, 1.).map_err(|e| e.to_string())?,
                });
            };
            Ok(Outcome::Imported {
                tab,
                revision,
                destination,
                name,
                geometry: Arc::new(MeshAsset::new(mesh)),
            })
        });
    }
    fn open(&mut self, context: &egui::Context, recovery: Option<PathBuf>) {
        self.start(context, None, move || {
            let is_recovery = recovery.is_some();
            let path = if let Some(path) = recovery {
                path
            } else {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter(text(T::FileFilter), &["3dnator"])
                    .pick_file()
                else {
                    return Ok(Outcome::Cancelled);
                };
                path
            };
            let path = path.canonicalize().map_err(|e| e.to_string())?;
            let document = read_project(&path).map_err(|e| e.to_string())?;
            Ok(Outcome::Opened {
                document,
                path: (!is_recovery).then(|| path.clone()),
                recovery: is_recovery.then_some(path),
            })
        });
    }
    fn save(&mut self, context: &egui::Context, tab_id: u64, save_as: bool, close_after: bool) {
        if self
            .tabs
            .iter()
            .find(|t| t.id == tab_id)
            .is_some_and(|t| t.sketch_edit.as_ref().is_some_and(|w| w.pending.is_some()))
        {
            self.status.warning(text(T::PendingEdit));
            return;
        }
        let other_paths = self
            .tabs
            .iter()
            .filter(|t| t.id != tab_id)
            .filter_map(|t| t.path.clone())
            .collect::<Vec<_>>();
        let Some(tab) = self.find_tab(tab_id) else {
            return;
        };
        let document = tab.session.document().clone();
        let state = tab.session.state();
        let existing = if save_as { None } else { tab.path.clone() };
        let recovery = tab.recovery_path.clone();
        let recovery_source = tab.recovery_source.clone();
        self.start(context, None, move || {
            let mut path = if let Some(path) = existing {
                path
            } else {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter(text(T::FileFilter), &["3dnator"])
                    .set_file_name("projeto.3dnator")
                    .save_file()
                else {
                    return Ok(Outcome::Cancelled);
                };
                path
            };
            if !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("3dnator"))
            {
                path.set_extension("3dnator");
            }
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            if other_paths.contains(&canonical) {
                return Err(text(T::FileAlreadyOpen).into());
            }
            write_project(&path, &document).map_err(|e| e.to_string())?;
            // A recuperação só é removida após a confirmação da gravação principal.
            let _ = nator_project_io::remove_recovery(&recovery);
            if let Some(source) = recovery_source {
                let _ = nator_project_io::remove_recovery(&source);
            }
            Ok(Outcome::Saved {
                tab: tab_id,
                state,
                path: path.canonicalize().unwrap_or(path),
                close_after,
            })
        });
    }
    /// Exporta o escopo da aba capturada com cores e estrutura de montagem.
    fn export_step(
        &mut self,
        context: &egui::Context,
        tab: u64,
        scope: nator_commands::export::StepScope,
    ) {
        let Some(tab) = self.tabs.iter().find(|t| t.id == tab) else {
            return;
        };
        let document = tab.session.document().clone();
        let name = document
            .name()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();
        self.start(context, None, move || {
            let Some(mut path) = rfd::FileDialog::new()
                .add_filter(text(T::StepFilter), &["step", "stp"])
                .set_file_name(format!("{name}.step"))
                .save_file()
            else {
                return Ok(Outcome::Cancelled);
            };
            if !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("step") || e.eq_ignore_ascii_case("stp"))
            {
                path.set_extension("step");
            }
            if path.exists() {
                return Err(text(T::OutputExists).into());
            }
            #[cfg(feature = "occt")]
            {
                let scene = nator_commands::export::step_scene(&document, scope)
                    .map_err(|e| e.to_string())?;
                // Gerar em temporário e publicar sem substituir um arquivo existente.
                nator_project_io::export_step(&path, |temp| {
                    scene.write(temp).map_err(|e| e.to_string())
                })
                .map_err(|e| e.to_string())?;
                Ok(Outcome::Exported)
            }
            #[cfg(not(feature = "occt"))]
            {
                let _ = (document, scope);
                nator_kernel::require_available().map_err(|e| e.to_string())?;
                Err(text(T::KernelUnavailable).into())
            }
        });
    }
    fn maybe_recover(&mut self, context: &egui::Context) {
        if self.busy() {
            return;
        }
        let pending = self
            .tabs
            .iter()
            .find(|t| t.session.is_dirty() && t.recovery_state != Some(t.session.state()));
        let Some(tab) = pending else {
            return;
        };
        let elapsed = tab.changed_at.elapsed();
        if elapsed < Duration::from_secs(2) {
            context.request_repaint_after(Duration::from_secs(2) - elapsed);
            return;
        }
        let id = tab.id;
        let state = tab.session.state();
        let path = tab.recovery_path.clone();
        let document = tab.session.document().clone();
        self.start(context, Some((id, state)), move || {
            nator_project_io::write_recovery(&path, &document).map_err(|e| e.to_string())?;
            Ok(Outcome::Recovered { tab: id, state })
        });
    }
    fn close_tab(&mut self, id: u64) {
        let previous_active = self.current().id;
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else {
            return;
        };
        if self.tabs.len() == 1 {
            let result = Document::new(text(T::Untitled))
                .map_err(|e| e.to_string())
                .and_then(|doc| self.add_tab(doc, None, None));
            if let Err(error) = result {
                self.status.error(error);
                return;
            }
        }
        let tab = self.tabs.remove(index);
        let _ = nator_project_io::remove_recovery(&tab.recovery_path);
        if let Some(source) = tab.recovery_source {
            let _ = nator_project_io::remove_recovery(&source);
        }
        self.active = self
            .tabs
            .iter()
            .position(|t| t.id == previous_active)
            .unwrap_or_else(|| index.min(self.tabs.len() - 1));
        self.viewport.clear(&self.gpu.device);
        self.scene = None;
        self.viewport.camera = self.current().camera.clone();
        self.invalidate_scene();
        self.pending_close = None;
    }
    fn request_close(&mut self, id: u64) {
        if self.busy() {
            return;
        }
        if self.tabs.iter().any(|t| t.id == id && t.modified()) {
            self.pending_close = Some(id);
        } else {
            self.close_tab(id);
        }
    }
}

impl eframe::App for Editor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
    fn on_exit(&mut self) {
        self.gpu.renderer.write().free_texture(&self.texture);
    }
}
impl Editor {
    fn draw(&mut self, ui: &mut egui::Ui) {
        self.collect(ui.ctx());
        if ui.ctx().input(|i| i.viewport().close_requested()) && !self.allow_exit {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.want_exit = true;
        }
        if self.want_exit && !self.busy() && self.pending_close.is_none() {
            if let Some(tab) = self.tabs.iter().find(|t| t.modified()) {
                self.pending_close = Some(tab.id);
            } else {
                self.allow_exit = true;
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        self.poll_preview(ui.ctx());
        self.ensure_scene(ui.ctx());
        // Um diálogo já aberto recebe Enter/Esc antes dos painéis de comando.
        // Mensagens geradas neste quadro ainda podem abrir ao final do desenho.
        let status_open = self.status.has_pending();
        if status_open {
            self.status_dialog(ui.ctx());
        }
        self.shortcuts(ui.ctx());
        self.toolbar(ui);
        if self.shell.browser_visible {
            self.tree(ui);
        }
        self.canvas(ui);
        self.sketch_tools_window(ui.ctx());
        self.sketch_action_panel(ui.ctx());
        self.measure_window(ui.ctx());
        self.dialogs(ui.ctx());
        self.feature_dialog(ui.ctx());
        self.assembly_panel(ui.ctx());
        self.section_panel(ui.ctx());
        self.sketch_repair_panel(ui.ctx());
        self.insert_part_panel(ui.ctx());
        self.mesh_import_panel(ui.ctx());
        self.drawing_dialog(ui.ctx());
        self.export_dialog(ui.ctx());
        self.parameter_dialog(ui.ctx());
        self.command_palette(ui.ctx());
        self.timeline_dialogs(ui.ctx());
        self.select_other_popup(ui.ctx());
        self.browser_rename_dialog(ui.ctx());
        self.preview.finish_frame(ui.ctx());
        window_frame::resize_handles(ui);
        self.maybe_recover(ui.ctx());
        if !status_open {
            self.status_dialog(ui.ctx());
        }
    }
}

/// Entrega identificada por aba e revisão, sem depender da aba que está visível.
fn deliver_import(
    tabs: &mut [Tab],
    id: u64,
    revision: u64,
    destination: Option<PartId>,
    name: String,
    geometry: Arc<MeshAsset>,
) -> Result<bool, String> {
    let Some(tab) = tabs
        .iter_mut()
        .find(|t| t.id == id && t.session.revision() == revision)
    else {
        return Ok(false);
    };
    if let Effect::Imported { part, body } = tab
        .session
        .execute(Command::ImportMesh {
            destination,
            name,
            geometry,
        })
        .map_err(|e| e.to_string())?
    {
        tab.selection = Some(Selection::Body { part, id: body });
    }
    tab.changed();
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn background_import_targets_original_tab_and_rejects_stale_results()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut tabs = vec![
            Tab::new(1, Document::new("A")?, None, None)?,
            Tab::new(2, Document::new("B")?, None, None)?,
        ];
        let asset = Arc::new(MeshAsset::new(reference_bracket()?));
        assert!(deliver_import(
            &mut tabs,
            1,
            0,
            None,
            "Base".into(),
            Arc::clone(&asset)
        )?);
        assert_eq!(tabs[0].session.document().parts().len(), 1);
        assert!(tabs[1].session.document().parts().is_empty());
        assert!(!deliver_import(
            &mut tabs,
            1,
            0,
            None,
            "Resultado antigo".into(),
            asset
        )?);
        assert_eq!(tabs[0].session.document().parts().len(), 1);
        Ok(())
    }
}
