//! Pré-visualização do comando em edição. O resultado é calculado em segundo plano numa
//! cópia do documento e exibido no lugar da cena, sem entrar no desfazer. Ao confirmar, o
//! mesmo cálculo é publicado se ainda corresponder à revisão e aos parâmetros do painel;
//! caso contrário, o comando é preparado de novo pelo trabalhador principal.
use super::*;
use nator_commands::CommandError;

/// Cor do resultado em pré-visualização.
pub(super) const PREVIEW_COLOR: [u8; 3] = [86, 156, 224];
/// Pausa após a última alteração antes de calcular, para não reconstruir a cada tecla.
const DEBOUNCE: Duration = Duration::from_millis(220);
/// Cálculos de prévia ao mesmo tempo, contando os abandonados que ainda não terminaram. O
/// OCCT não interrompe uma operação longa (uma remoção de faces num STEP grande leva
/// minutos); a prévia nova não espera por ela, mas também não empilha cálculos sem fim.
const MAX_RUNNING: usize = 2;

#[cfg(test)]
thread_local! {
    /// Atraso artificial das prévias pedidas por esta thread, para ensaiar cálculos longos.
    pub(super) static PREVIEW_DELAY: std::cell::Cell<Duration> =
        const { std::cell::Cell::new(Duration::ZERO) };
}

/// Conta as threads de prévia vivas; o guarda desconta ao terminar, inclusive em pânico.
struct Running(Arc<std::sync::atomic::AtomicUsize>);
impl Drop for Running {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// OK dado enquanto a prévia do mesmo comando ainda calculava: o resultado dela é publicado
/// quando chegar, sem refazer o cálculo no trabalhador nem bloquear a interface.
struct Adopted {
    tab: u64,
    command: Command,
    receiver: Receiver<Computed>,
}

struct Request {
    tab: u64,
    revision: u64,
    key: String,
    command: Command,
    /// Corpo destacado quando já é conhecido (edição); novas operações usam o efeito.
    highlight: Option<BodyId>,
    /// Operação em edição: a exibição recua o histórico até ela, como a linha do tempo.
    rollback: Option<(PartId, BodyId)>,
    changed: Instant,
}

type Scene = Option<(Mesh, PreparedMesh)>;

struct Computed {
    tab: u64,
    revision: u64,
    key: String,
    guide: Option<nator_commands::extrusion::Guide>,
    choice: Option<nator_document::features::Combine>,
    result: Result<Option<(PreparedCommand, Scene)>, CommandError>,
}

/// Situação da pré-visualização do comando atual, para o rodapé do painel.
pub(super) enum PreviewStatus<'a> {
    Waiting,
    Ready,
    Failed(&'a CommandError),
}

#[derive(Default)]
pub(super) struct LivePreview {
    request: Option<Request>,
    receiver: Option<Receiver<Computed>>,
    /// O que a thread de `receiver` calcula: aba, revisão e chave.
    running: Option<(u64, u64, String)>,
    /// Threads de prévia vivas, inclusive as abandonadas.
    threads: Arc<std::sync::atomic::AtomicUsize>,
    adopted: Option<Adopted>,
    ready: Option<(String, PreparedCommand)>,
    failed: Option<(String, CommandError)>,
    empty: Option<String>,
    /// Algum painel pediu pré-visualização neste quadro.
    requested: bool,
    /// Geração exibida no viewport; zero quando a cena do documento está à mostra.
    pub shown: u64,
    generation: u64,
}

impl LivePreview {
    /// Fim do quadro: sem comando completo, descarta a prévia e agenda a restauração
    /// da cena real. Os painéis são processados depois de `ensure_scene`; sem outro
    /// quadro, a GPU conservaria a cópia temporária até a próxima interação do usuário.
    pub(super) fn finish_frame(&mut self, context: &egui::Context) {
        if !std::mem::take(&mut self.requested) {
            self.request = None;
        }
        if self.request.is_none() {
            self.ready = None;
            self.failed = None;
            self.empty = None;
            if std::mem::take(&mut self.shown) != 0 {
                // Somente a transição pede redesenho. Quadros ociosos continuam sem
                // repintura contínua; resultados tardios serão rejeitados sem request.
                context.request_repaint();
            }
        }
    }
}

/// Chave textual do comando: muda com qualquer parâmetro. Os comandos pré-visualizados
/// não carregam malhas, então a representação é curta.
fn key(command: &Command) -> String {
    format!("{command:?}")
}

impl Editor {
    pub(super) fn normalized_label_preview(
        &self,
        command: &Command,
    ) -> Option<nator_document::features::LabelSpec> {
        let (ready_key, prepared) = self.preview.ready.as_ref()?;
        if *ready_key != key(command) || prepared.revision() != self.current().session.revision() {
            return None;
        }
        let output = match command {
            Command::EditFeature { output, .. } => *output,
            _ => match prepared.effect() {
                Effect::Imported { body, .. } => body,
                _ => return None,
            },
        };
        prepared
            .document()
            .parts()
            .iter()
            .flat_map(|p| &p.features)
            .find_map(|f| {
                if f.output != output {
                    return None;
                }
                if let nator_document::features::Operation::Label { spec, .. } = &f.operation {
                    Some(spec.clone())
                } else {
                    None
                }
            })
    }

    /// Chamado a cada quadro pelo painel aberto com o comando que o OK aplicaria.
    pub(super) fn preview_command(
        &mut self,
        tab: u64,
        command: Option<Command>,
        highlight: Option<BodyId>,
        rollback: Option<(PartId, BodyId)>,
    ) {
        self.preview.requested = true;
        let Some(command) = command else {
            self.preview.request = None;
            return;
        };
        let Some(revision) = self
            .tabs
            .iter()
            .find(|t| t.id == tab)
            .map(|t| t.session.revision())
        else {
            return;
        };
        let key = key(&command);
        let same = self
            .preview
            .request
            .as_ref()
            .is_some_and(|r| r.tab == tab && r.revision == revision && r.key == key);
        if !same {
            self.preview.request = Some(Request {
                tab,
                revision,
                key,
                command,
                highlight,
                rollback,
                changed: Instant::now(),
            });
        }
    }

    /// Mantém a prévia atual neste quadro sem trocar o comando (por exemplo, durante o
    /// arraste de um manipulador).
    pub(super) fn keep_preview(&mut self) {
        self.preview.requested = true;
    }

    /// Nenhuma prévia pedida ou calculando: o painel não tem comando completo ou já terminou.
    pub(super) fn preview_idle(&self) -> bool {
        match &self.preview.request {
            None => true,
            Some(request) => {
                self.preview.failed.as_ref().map(|(k, _)| k) == Some(&request.key)
                    || self.preview.ready.as_ref().map(|(k, _)| k) == Some(&request.key)
                    || self.preview.empty.as_ref() == Some(&request.key)
            }
        }
    }

    /// Situação da pré-visualização do comando dado, se for o atual.
    pub(super) fn preview_status(&self, command: &Command) -> PreviewStatus<'_> {
        let key = key(command);
        if self.preview.ready.as_ref().is_some_and(|(k, _)| *k == key) {
            return PreviewStatus::Ready;
        }
        match &self.preview.failed {
            Some((k, error)) if *k == key => PreviewStatus::Failed(error),
            _ => PreviewStatus::Waiting,
        }
    }

    /// Resultado já calculado para o comando, se ainda valer para a revisão da aba.
    pub(super) fn take_preview(&mut self, tab: u64, command: &Command) -> Option<PreparedCommand> {
        let key = key(command);
        let revision = self.tabs.iter().find(|t| t.id == tab)?.session.revision();
        let (k, prepared) = self.preview.ready.take()?;
        if k == key && prepared.revision() == revision {
            Some(prepared)
        } else {
            None
        }
    }

    /// Falha já calculada para o comando, para decidir sem recalcular.
    pub(super) fn preview_failure(&self, command: &Command) -> Option<CommandError> {
        let key = key(command);
        self.preview
            .failed
            .as_ref()
            .filter(|(k, _)| *k == key)
            .map(|(_, e)| e.clone())
    }

    /// OK com a prévia do mesmo comando ainda calculando: adota esse cálculo. Devolve falso
    /// quando não há cálculo em andamento para este comando.
    pub(super) fn adopt_preview(&mut self, tab: u64, command: &Command) -> bool {
        let Some(revision) = self
            .tabs
            .iter()
            .find(|t| t.id == tab)
            .map(|t| t.session.revision())
        else {
            return false;
        };
        let running = self
            .preview
            .running
            .as_ref()
            .is_some_and(|r| *r == (tab, revision, key(command)));
        if !running || self.preview.adopted.is_some() {
            return false;
        }
        let Some(receiver) = self.preview.receiver.take() else {
            return false;
        };
        self.preview.running = None;
        self.preview.adopted = Some(Adopted {
            tab,
            command: command.clone(),
            receiver,
        });
        self.status.set(text(T::FinishingInBackground));
        true
    }

    /// OK no Remover faces sem prévia (a concordância incompleta a suspende): o cálculo,
    /// que pode levar minutos, roda numa thread própria e é publicado ao terminar, sem
    /// ocupar o trabalhador do projeto nem bloquear a interface.
    pub(super) fn adopt_removal(&mut self, tab: u64, command: &Command) -> bool {
        let removal = matches!(
            command,
            Command::AddFeature {
                operation: nator_document::features::Operation::RemoveFaces { .. },
                ..
            } | Command::EditFeature {
                operation: nator_document::features::Operation::RemoveFaces { .. },
                ..
            }
        );
        if !removal || self.preview.adopted.is_some() {
            return false;
        }
        let Some(target) = self.tabs.iter().find(|t| t.id == tab) else {
            return false;
        };
        let document = target.session.document().clone();
        let revision = target.session.revision();
        let (sender, receiver) = mpsc::channel();
        self.preview
            .threads
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let alive = Running(self.preview.threads.clone());
        let job = command.clone();
        let key = key(command);
        let spawned = std::thread::Builder::new()
            .name("removal-commit".into())
            .spawn(move || {
                let _alive = alive;
                let result = PreparedCommand::new(document, revision, job)
                    .map(|prepared| Some((prepared, None)));
                let _ = sender.send(Computed {
                    tab,
                    revision,
                    key,
                    guide: None,
                    choice: None,
                    result,
                });
            });
        if spawned.is_err() {
            return false;
        }
        self.preview.adopted = Some(Adopted {
            tab,
            command: command.clone(),
            receiver,
        });
        self.status.set(text(T::FinishingInBackground));
        true
    }

    /// Cálculo de prévia em andamento e cálculo adotado por um OK, para ensaios.
    #[cfg(all(test, feature = "occt"))]
    pub(super) fn preview_threads_for_test(&self) -> (bool, bool) {
        (
            self.preview.running.is_some(),
            self.preview.adopted.is_some(),
        )
    }

    /// Publica o cálculo adotado quando ele chega.
    fn poll_adopted(&mut self, context: &egui::Context) {
        let Some(adopted) = &self.preview.adopted else {
            return;
        };
        let computed = match adopted.receiver.try_recv() {
            Ok(computed) => computed,
            Err(mpsc::TryRecvError::Empty) => {
                context.request_repaint_after(Duration::from_millis(200));
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                if let Some(adopted) = self.preview.adopted.take() {
                    self.status.error(text(T::FinishingFailed));
                    drop(adopted);
                }
                return;
            }
        };
        let Some(Adopted { tab, command, .. }) = self.preview.adopted.take() else {
            return;
        };
        match computed.result {
            Ok(Some((prepared, _))) => {
                self.commit_prepared(tab, prepared);
                self.status.set(text(T::Ready));
            }
            Err(error) => self.reject_command(tab, command, error),
            // Sem prévia para este comando: prepara pelo caminho normal.
            Ok(None) => self.prepare_command(context, tab, command),
        }
    }

    /// Recebe resultados e inicia o cálculo pendente quando a edição para de mudar.
    pub(super) fn poll_preview(&mut self, context: &egui::Context) {
        self.poll_adopted(context);
        if let Some(receiver) = &self.preview.receiver {
            match receiver.try_recv() {
                Ok(computed) => {
                    self.preview.receiver = None;
                    self.preview.running = None;
                    self.deliver_preview(computed);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.preview.receiver = None;
                    self.preview.running = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        // Um cálculo que não corresponde mais ao pedido é abandonado: termina sozinho e o
        // resultado é descartado. A prévia nova não espera minutos por ele.
        let stale = self
            .preview
            .running
            .as_ref()
            .is_some_and(|(tab, revision, key)| {
                self.preview
                    .request
                    .as_ref()
                    .is_none_or(|r| r.tab != *tab || r.revision != *revision || r.key != *key)
            });
        if stale {
            self.preview.receiver = None;
            self.preview.running = None;
        }
        if self.preview.receiver.is_some() {
            return;
        }
        let Some(request) = &self.preview.request else {
            return;
        };
        let settled = self
            .preview
            .ready
            .as_ref()
            .map(|(k, _)| k)
            .or(self.preview.failed.as_ref().map(|(k, _)| k))
            .is_some_and(|k| *k == request.key);
        if settled {
            return;
        }
        let elapsed = request.changed.elapsed();
        if elapsed < DEBOUNCE {
            context.request_repaint_after(DEBOUNCE - elapsed);
            return;
        }
        let Some(tab) = self.tabs.iter().find(|t| t.id == request.tab) else {
            return;
        };
        if tab.session.revision() != request.revision {
            self.preview.request = None;
            return;
        }
        if self.preview.empty.as_ref() == Some(&request.key) {
            return;
        }
        if self
            .preview
            .threads
            .load(std::sync::atomic::Ordering::SeqCst)
            >= MAX_RUNNING
        {
            context.request_repaint_after(Duration::from_millis(200));
            return;
        }
        let document = tab.session.document().clone();
        let view = tab.view.clone();
        let active = tab.active_part;
        let (tab, revision, key, command, highlight, rollback) = (
            request.tab,
            request.revision,
            request.key.clone(),
            request.command.clone(),
            request.highlight,
            request.rollback,
        );
        let (sender, receiver) = mpsc::channel();
        let context = context.clone();
        self.preview
            .threads
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let alive = Running(self.preview.threads.clone());
        let running = (tab, revision, key.clone());
        #[cfg(test)]
        let delay = PREVIEW_DELAY.with(std::cell::Cell::get);
        let spawned = std::thread::Builder::new()
            .name("preview-worker".into())
            .spawn(move || {
                let _alive = alive;
                #[cfg(test)]
                std::thread::sleep(delay);
                let extrusion = match &command {
                    Command::AddFeature {
                        part, operation, ..
                    }
                    | Command::EditFeature {
                        part, operation, ..
                    }
                    | Command::AddSmartExtrusion {
                        part, operation, ..
                    } => Some((*part, operation.clone())),
                    _ => None,
                };
                let guide = extrusion.as_ref().and_then(|(part, operation)| {
                    nator_commands::extrusion::guide(&document, *part, operation).ok()
                });
                let original = document.clone();
                let mut choice = None;
                let result =
                    PreparedCommand::new(document, revision, command).and_then(|prepared| {
                        let mut display = prepared.document().clone();
                        let highlight = highlight.or(match prepared.effect() {
                            Effect::Imported { body, .. } => Some(body),
                            _ => None,
                        });
                        let failure = |e: nator_document::DocumentError| CommandError {
                            message: e.to_string(),
                            failure: None,
                        };
                        let mut volume = None;
                        if let Some((part, _)) = &extrusion {
                            let operation = highlight.and_then(|id| {
                                display
                                    .part(*part)?
                                    .features
                                    .iter()
                                    .find(|f| f.output == id)
                                    .map(|f| &f.operation)
                            });
                            choice = operation.and_then(nator_commands::extrusion::combination);
                            if let Some(operation) = operation {
                                volume =
                                    nator_commands::extrusion::removed(&original, *part, operation)
                                        .map_err(failure)?
                                        .map(|mesh| (PreviewVolume::Removed, *part, mesh));
                                if volume.is_none() {
                                    // O destaque é só apresentação: sem ele, a prévia ainda vale.
                                    volume = nator_commands::extrusion::added(
                                        &original, *part, operation,
                                    )
                                    .ok()
                                    .flatten()
                                    .map(|mesh| (PreviewVolume::Added, *part, mesh));
                                }
                            }
                        }
                        if let Some((part, output)) = rollback {
                            roll_back_after(&mut display, part, output);
                        }
                        // O material novo aparece sobre o corpo original, sem disputar
                        // a mesma superfície com o resultado da união.
                        if let (Some((PreviewVolume::Added, part, _)), Some(id)) =
                            (&volume, highlight)
                        {
                            roll_back(&mut display, *part, id, true);
                        }
                        // Pintar o resultado de uma união ou corte realçaria o corpo
                        // inteiro, não o efeito da extrusão.
                        let combined = choice.is_some_and(|c| {
                            c.mode != nator_document::features::BooleanMode::Intersect
                        });
                        if let Some(id) = highlight.filter(|_| volume.is_none() && !combined) {
                            // A cor de ocorrência prevalece sobre a do corpo: retirá-la na cópia
                            // exibida deixa a tinta da prévia visível também em componentes
                            // importados com cor própria.
                            let part = display
                                .parts()
                                .iter()
                                .find(|p| p.bodies.iter().any(|b| b.id == id))
                                .map(|p| p.id);
                            let colored = display
                                .instances()
                                .iter()
                                .filter(|i| Some(i.part) == part && i.color.is_some())
                                .map(|i| i.id)
                                .collect::<Vec<_>>();
                            for instance in colored {
                                if let Ok(i) = display.instance_mut(instance) {
                                    i.color = None;
                                }
                            }
                            if let Ok(body) = display.body_mut(id) {
                                body.color = Some(nator_geometry::RgbColor::new(PREVIEW_COLOR));
                            }
                        }
                        let scene = prepare_scene_with_removal(&display, &view, active, volume)
                            .map_err(|message| CommandError {
                                message,
                                failure: None,
                            })?;
                        Ok(Some((prepared, scene)))
                    });
                let _ = sender.send(Computed {
                    tab,
                    revision,
                    key,
                    guide,
                    choice,
                    result,
                });
                context.request_repaint();
            });
        if spawned.is_ok() {
            self.preview.receiver = Some(receiver);
            self.preview.running = Some(running);
        }
    }

    fn deliver_preview(&mut self, computed: Computed) {
        let current = self.preview.request.as_ref().is_some_and(|r| {
            r.tab == computed.tab && r.revision == computed.revision && r.key == computed.key
        });
        if !current {
            return;
        }
        // A seta continua disponível mesmo sem uma divisão possível (perfil livre
        // ou face inteira em zero), para permitir iniciar o arraste da extrusão.
        if let Some(draft) = self
            .feature_edit
            .as_mut()
            .filter(|d| d.tab() == computed.tab)
        {
            draft.receive_extrusion(computed.guide, computed.choice);
        }
        match computed.result {
            Ok(None) => {
                self.preview.ready = None;
                self.preview.failed = None;
                self.preview.shown = 0;
                self.preview.empty = Some(computed.key);
            }
            Ok(Some((prepared, scene))) => {
                self.preview.empty = None;
                self.preview.failed = None;
                if computed.tab == self.current().id
                    && let Some((mesh, data)) = scene
                {
                    self.preview.generation += 1;
                    self.preview.shown = self.preview.generation;
                    let key = self.scene_key();
                    self.show_scene(key, mesh.bounds(), Some(mesh), Some(data));
                }
                self.preview.ready = Some((computed.key, prepared));
            }
            Err(error) => {
                // Uma prévia anterior não representa mais os parâmetros: volta a cena real.
                self.preview.ready = None;
                self.preview.shown = 0;
                self.preview.failed = Some((computed.key, error));
            }
        }
    }
}

/// Recua o histórico de uma cópia do documento para logo depois de `output` (inclusive) ou,
/// com `before`, para logo antes dela. Recuar não exige recálculo: as operações anteriores ao
/// marcador não dependem das posteriores. Só a visibilidade derivada do histórico muda.
pub(super) fn roll_back(document: &mut Document, part: PartId, output: BodyId, before: bool) {
    let Ok(piece) = document.part_mut(part) else {
        return;
    };
    let Some(index) = piece.features.iter().position(|f| f.output == output) else {
        return;
    };
    let marker = if before { index } else { index + 1 };
    let current = piece.history_marker.unwrap_or(piece.features.len());
    if marker >= current {
        return;
    }
    let hidden = nator_document::features::hidden_by_history(piece);
    piece.history_marker = Some(marker);
    nator_document::features::sync_visibility(piece, &hidden);
}

fn roll_back_after(document: &mut Document, part: PartId, output: BodyId) {
    roll_back(document, part, output, false);
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

    fn frame(context: &egui::Context, preview: &mut LivePreview) -> TestResult<Duration> {
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            preview.finish_frame(ui.ctx());
        });
        output.textures_delta.clear();
        Ok(output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .ok_or("Viewport principal ausente")?
            .repaint_delay)
    }

    #[test]
    fn leaving_a_visible_preview_schedules_restoration_without_more_input() -> TestResult<()> {
        // Cobre tanto Cancelar no Mover (pedido explícito de None) quanto fechar um
        // painel de operação (nenhum pedido no quadro), sem eventos de mouse/teclado.
        for explicit_cancel in [true, false] {
            let context = egui::Context::default();
            let command = Command::DuplicateInstance { id: InstanceId(1) };
            let mut preview = LivePreview {
                request: Some(Request {
                    tab: 1,
                    revision: 0,
                    key: key(&command),
                    command,
                    highlight: None,
                    rollback: None,
                    changed: Instant::now(),
                }),
                shown: 1,
                generation: 1,
                ..Default::default()
            };
            // Drena os redesenhos iniciais do egui. A prévia mantida pelo painel
            // deve conseguir ficar ociosa, para o ensaio não depender de animações.
            let mut delay = Duration::ZERO;
            for _ in 0..4 {
                preview.requested = true;
                delay = frame(&context, &mut preview)?;
            }
            assert_eq!(delay, Duration::MAX);
            assert_eq!(preview.shown, 1);

            if explicit_cancel {
                preview.requested = true;
                preview.request = None;
            }
            assert_eq!(frame(&context, &mut preview)?, Duration::ZERO);
            assert_eq!(preview.shown, 0);
            assert!(preview.request.is_none());
            // Restaurar não pode transformar a janela ociosa num laço de redesenho.
            for _ in 0..4 {
                delay = frame(&context, &mut preview)?;
            }
            assert_eq!(delay, Duration::MAX);
        }
        Ok(())
    }

    #[test]
    fn canceling_before_a_preview_is_visible_keeps_the_viewport_idle() -> TestResult<()> {
        let context = egui::Context::default();
        let mut preview = LivePreview::default();
        let mut delay = Duration::ZERO;
        for _ in 0..4 {
            delay = frame(&context, &mut preview)?;
        }
        assert_eq!(delay, Duration::MAX);
        preview.requested = true;
        assert_eq!(frame(&context, &mut preview)?, Duration::MAX);
        Ok(())
    }
}
