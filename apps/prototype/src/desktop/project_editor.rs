mod assembly_editor;
pub(crate) mod capture;
mod drawing_editor;
mod export_editor;
mod feature_editor;
mod navigation;
mod parameter_editor;
mod sketch_editor;
mod views;
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

#[derive(Clone, Copy, PartialEq, Eq)]
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
}

struct Tab {
    id: u64,
    session: Session,
    path: Option<PathBuf>,
    camera: Camera,
    scene_bounds: Option<nator_geometry::Bounds>,
    selection: Option<Selection>,
    active_part: Option<PartId>,
    sketch_edit: Option<sketch_editor::Workspace>,
    name_input: String,
    position_input: Point3,
    rotation_input: Point3,
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
            active_part: None,
            sketch_edit: None,
            name_input: String::new(),
            position_input: [0.; 3],
            rotation_input: [0.; 3],
            changed_at: Instant::now(),
            recovery_path,
            recovery_source: recovered,
            recovery_state: None,
            picked: None,
            measurements: Vec::new(),
            reference: None,
            view: ViewState::default(),
            drag_start: None,
        })
    }
    fn select(&mut self, selection: Option<Selection>) {
        self.selection = selection;
        let values = match selection {
            Some(Selection::Body { id, .. }) => self
                .session
                .document()
                .body(id)
                .map(|b| (b.name.clone(), b.placement)),
            Some(Selection::Instance(id)) => self.session.document().instance(id).and_then(|i| {
                self.session
                    .document()
                    .part(i.part)
                    .map(|p| (p.name.clone(), i.placement))
            }),
            None => None,
        };
        if let Some((name, placement)) = values {
            self.name_input = name;
            self.position_input = placement.translation_mm();
            self.rotation_input = placement.rotation_rad().map(f64::to_degrees);
        } else {
            self.selection = None;
        }
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
        self.select(self.selection);
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
    SketchSolved {
        tab: u64,
        revision: u64,
        id: SketchId,
        report: nator_sketch::solver::Report,
        prepared: Result<PreparedCommand, String>,
    },
    Imported {
        tab: u64,
        revision: u64,
        destination: Option<PartId>,
        name: String,
        geometry: Arc<MeshAsset>,
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
    status: String,
    scene: Option<Mesh>,
    rendered: Option<SceneKey>,
    requested: Option<SceneKey>,
    import_scale: f64,
    pending_close: Option<u64>,
    want_exit: bool,
    allow_exit: bool,
    recoveries: Vec<PathBuf>,
    show_recovery: bool,
    parameters_edit: Option<parameter_editor::Draft>,
    assembly_edit: Option<assembly_editor::Draft>,
    feature_edit: Option<feature_editor::Draft>,
    drawing_edit: Option<drawing_editor::Draft>,
    export_edit: Option<export_editor::Draft>,
    pick_mode: navigation::PickMode,
    alignment: Option<(u64, u64, nator_document::assembly::Pick)>,
    move_freely: bool,
    rotate_angle: f64,
    /// Linhas do último relatório de medição, já formatadas para exibição.
    measure_report: Vec<String>,
}

/// Cena estilizada e dados de GPU. As arestas vivas já foram analisadas uma vez por geometria.
fn prepare_scene(
    document: &Document,
    view: &ViewState,
    active: Option<PartId>,
) -> Result<Option<(Mesh, PreparedMesh)>, String> {
    use nator_document::Style;
    let Some(scene) = document
        .scene_styled(|occurrence, part| view.style(document, occurrence, part, active))
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let styles = scene
        .styles
        .iter()
        .map(|style| match style {
            Style::Normal => nator_renderer::STYLE_NORMAL,
            Style::Dimmed => nator_renderer::STYLE_DIMMED,
            Style::Transparent => nator_renderer::STYLE_TRANSPARENT,
        })
        .collect::<Vec<_>>();
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
        context.set_visuals(egui::Visuals::dark());
        let recoveries = recovery_files(&recovery_directory()).unwrap_or_default();
        Ok(Self {
            gpu,
            viewport,
            texture,
            tabs: vec![tab],
            active: 0,
            next_tab: 2,
            worker: None,
            status: text(T::Ready).into(),
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
            assembly_edit: None,
            feature_edit: None,
            drawing_edit: None,
            export_edit: None,
            pick_mode: navigation::PickMode::Face,
            alignment: None,
            move_freely: false,
            rotate_angle: 15.,
            measure_report: Vec::new(),
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
        self.viewport.clear(&self.gpu.device);
        self.scene = None;
        self.rendered = None;
        self.requested = None;
    }
    fn switch(&mut self, index: usize) {
        if index >= self.tabs.len() || index == self.active {
            return;
        }
        self.current_mut().camera = self.viewport.camera.clone();
        self.active = index;
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
                if let Effect::SketchCreated { part, sketch } = effect {
                    tab.active_part = Some(part);
                    tab.sketch_edit = Some(sketch_editor::Workspace::new(sketch));
                }
                tab.changed();
                self.invalidate_scene();
                if let Effect::SketchCreated { part, sketch } = effect {
                    self.begin_sketch(part, sketch);
                }
            }
            Err(error) => self.status = error.to_string(),
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
                self.status = text(T::Loading).into();
            }
            Err(error) => self.status = error.to_string(),
        }
    }
    fn collect(&mut self) {
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
        self.status = text(T::Ready).into();
        match result {
            Ok(Outcome::Picked {
                tab,
                revision,
                pick,
                mode,
                topology,
            }) => {
                let current = self
                    .find_tab(tab)
                    .is_some_and(|t| t.session.revision() == revision);
                if current
                    && let (Some(result), Some(p), Some(draft)) =
                        (topology, pick.as_ref(), self.feature_edit.as_mut())
                {
                    match result {
                        Ok(_) if p.part != draft.part() => {
                            self.status = text(T::PickOtherPart).into();
                        }
                        Ok(kept) => {
                            self.status = feature_editor::Draft::describe(&kept);
                            draft.apply_pick(p.body, kept);
                        }
                        Err(error) => self.status = error,
                    }
                }
                if let Some(target) = self
                    .find_tab(tab)
                    .filter(|t| t.session.revision() == revision)
                {
                    if let Some(mut p) = pick {
                        navigation::snap_pick(&mut p, mode);
                        target.select(Some(if target.active_part == Some(p.part) {
                            Selection::Body {
                                part: p.part,
                                id: p.body,
                            }
                        } else {
                            Selection::Instance(p.instance)
                        }));
                        target.picked = Some(p);
                    } else {
                        target.picked = None;
                        target.select(None);
                    }
                }
            }
            Ok(Outcome::Prepared { tab, command }) => {
                let mut saved = None;
                if let Some(target) = self.find_tab(tab) {
                    match target.session.commit(command) {
                        Ok(effect) => {
                            if let Effect::SheetSaved { id } = effect {
                                saved = Some(id);
                            }
                            target.changed();
                            self.requested = None;
                        }
                        Err(error) => self.status = error.to_string(),
                    }
                }
                if let Some(id) = saved
                    && let Some(draft) = self.drawing_edit.as_mut().filter(|d| d.tab == tab)
                {
                    draft.keep_sheet_id(id);
                }
            }
            Ok(Outcome::SketchSolved {
                tab,
                revision,
                id,
                report,
                prepared,
            }) => {
                if let Some(target) = self
                    .find_tab(tab)
                    .filter(|t| t.session.revision() == revision)
                    && let Some(work) = target.sketch_edit.as_mut().filter(|w| w.id == id)
                {
                    work.status = report.status.clone();
                    work.redundant = report.redundant;
                    if report.status == nator_sketch::solver::Status::Conflict {
                        work.pending = Some(report.sketch);
                        self.status = text(T::SketchConflict).into();
                    } else {
                        work.pending = None;
                        match prepared.and_then(|command| {
                            target.session.commit(command).map_err(|e| e.to_string())
                        }) {
                            Ok(_) => {
                                target.changed();
                                self.requested = None;
                            }
                            Err(error) => {
                                work.pending = Some(report.sketch);
                                self.status = error.to_string();
                            }
                        }
                    }
                }
            }
            Err(error) => {
                if let Some((id, state)) = recovery
                    && let Some(tab) = self.find_tab(id)
                {
                    tab.recovery_state = Some(state);
                }
                self.status = format!(
                    "{}: {error}",
                    if recovery.is_some() {
                        text(T::RecoveryFailed)
                    } else {
                        text(T::Failed)
                    }
                );
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
                        self.status = text(T::Imported).into();
                        self.requested = None;
                    }
                    Ok(false) => self.status = text(T::StaleResult).into(),
                    Err(error) => self.status = error,
                }
            }
            Ok(Outcome::Opened {
                document,
                path,
                recovery,
            }) => {
                if let Err(error) = self.add_tab(document, path, recovery) {
                    self.status = error;
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
                self.status = text(T::Saved).into();
                if close_after {
                    self.close_tab(tab);
                }
            }
            Ok(Outcome::Scene {
                key,
                mesh,
                prepared,
            }) => {
                if self.scene_key() == key {
                    let mut camera = self.current().camera.clone();
                    let new_bounds = mesh.as_ref().map(Mesh::bounds);
                    if let Some(bounds) = new_bounds {
                        if let Some(previous) = self.current().scene_bounds {
                            camera.reframe(previous, bounds);
                        } else {
                            let [w, h] = self.viewport.size();
                            camera.fit(f64::from(w) / f64::from(h));
                        }
                    }
                    let result = if let Some(prepared) = prepared {
                        self.viewport.set_prepared_mesh(&self.gpu.device, &prepared)
                    } else {
                        self.viewport.clear(&self.gpu.device);
                        Ok(())
                    };
                    match result {
                        Ok(()) => {
                            if let Some(bounds) = new_bounds {
                                self.current_mut().scene_bounds = Some(bounds);
                            }
                            self.scene = mesh;
                            self.rendered = Some(key);
                            self.viewport.camera = camera;
                        }
                        Err(error) => self.status = error,
                    }
                }
            }
            Ok(Outcome::Recovered { tab, state }) => {
                if let Some(target) = self.find_tab(tab) {
                    target.recovery_state = Some(state);
                }
                self.status = text(T::RecoverySaved).into();
            }
            Ok(Outcome::Exported) => self.status = text(T::Exported).into(),
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
    fn scene_key(&self) -> SceneKey {
        SceneKey {
            tab: self.current().id,
            revision: self.current().session.revision(),
            view: self.current().view.version,
            active: self.current().active_part,
        }
    }
    fn ensure_scene(&mut self, context: &egui::Context) {
        let key = self.scene_key();
        if self.rendered == Some(key) || self.requested == Some(key) || self.busy() {
            return;
        }
        self.viewport.clear(&self.gpu.device);
        self.scene = None;
        self.requested = Some(key);
        let document = self.current().session.document().clone();
        let view = self.current().view.clone();
        self.start(context, None, move || {
            let scene = prepare_scene(&document, &view, key.active)?;
            let (mesh, prepared) = match scene {
                Some((mesh, prepared)) => (Some(mesh), Some(prepared)),
                None => (None, None),
            };
            Ok(Outcome::Scene {
                key,
                mesh,
                prepared,
            })
        });
    }
    fn import(&mut self, context: &egui::Context, reference: bool) {
        let tab = self.current().id;
        let revision = self.current().session.revision();
        let destination = self.current().active_part;
        let scale = self.import_scale;
        let document = self.current().session.document().clone();
        self.start(context, None, move || {
            let (mesh, name) = if reference {
                (
                    reference_bracket().map_err(|e| e.to_string())?,
                    text(T::Reference).into(),
                )
            } else {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter(text(T::ModelFilter), &["stl", "step", "stp"])
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
                (
                    nator_project_io::read_stl(&path, scale).map_err(|e| e.to_string())?,
                    name,
                )
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
            self.status = text(T::PendingEdit).into();
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
                self.status = error;
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
        self.collect();
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
        self.ensure_scene(ui.ctx());
        self.shortcuts(ui.ctx());
        self.toolbar(ui);
        self.tree(ui);
        self.properties(ui);
        self.canvas(ui);
        self.dialogs(ui.ctx());
        self.assembly_dialog(ui.ctx());
        self.feature_dialog(ui.ctx());
        self.drawing_dialog(ui.ctx());
        self.export_dialog(ui.ctx());
        self.parameter_dialog(ui.ctx());
        self.maybe_recover(ui.ctx());
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
