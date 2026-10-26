//! Pré-visualização do comando em edição. O resultado é calculado em segundo plano numa
//! cópia do documento e exibido no lugar da cena, sem entrar no desfazer. Ao confirmar, o
//! mesmo cálculo é publicado se ainda corresponder à revisão e aos parâmetros do painel;
//! caso contrário, o comando é preparado de novo pelo trabalhador principal.
use super::*;
use nator_commands::CommandError;

/// Cor do resultado em pré-visualização.
const PREVIEW_COLOR: [u8; 3] = [86, 156, 224];
/// Pausa após a última alteração antes de calcular, para não reconstruir a cada tecla.
const DEBOUNCE: Duration = Duration::from_millis(220);

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
    result: Result<(PreparedCommand, Scene), CommandError>,
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
    ready: Option<(String, PreparedCommand)>,
    failed: Option<(String, CommandError)>,
    /// Algum painel pediu pré-visualização neste quadro.
    requested: bool,
    /// Geração exibida no viewport; zero quando a cena do documento está à mostra.
    pub shown: u64,
    generation: u64,
}

/// Chave textual do comando: muda com qualquer parâmetro. Os comandos pré-visualizados
/// não carregam malhas, então a representação é curta.
fn key(command: &Command) -> String {
    format!("{command:?}")
}

impl Editor {
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

    /// Fim do quadro: sem painel pedindo, a cena do documento volta à tela.
    pub(super) fn finish_preview_frame(&mut self) {
        if !std::mem::take(&mut self.preview.requested) {
            self.preview.request = None;
        }
        if self.preview.request.is_none() {
            self.preview.ready = None;
            self.preview.failed = None;
            if self.preview.shown != 0 {
                self.preview.shown = 0;
            }
        }
    }

    /// Recebe resultados e inicia o cálculo pendente quando a edição para de mudar.
    pub(super) fn poll_preview(&mut self, context: &egui::Context) {
        if let Some(receiver) = &self.preview.receiver {
            match receiver.try_recv() {
                Ok(computed) => {
                    self.preview.receiver = None;
                    self.deliver_preview(computed);
                }
                Err(mpsc::TryRecvError::Disconnected) => self.preview.receiver = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
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
        let spawned = std::thread::Builder::new()
            .name("preview-worker".into())
            .spawn(move || {
                let result = PreparedCommand::new(document, revision, command).map(|prepared| {
                    let mut display = prepared.document().clone();
                    let highlight = highlight.or(match prepared.effect() {
                        Effect::Imported { body, .. } => Some(body),
                        _ => None,
                    });
                    if let Some((part, output)) = rollback {
                        roll_back_after(&mut display, part, output);
                    }
                    if let Some(id) = highlight
                        && let Ok(body) = display.body_mut(id)
                    {
                        body.color = Some(nator_geometry::RgbColor::new(PREVIEW_COLOR));
                    }
                    let scene = prepare_scene(&display, &view, active).ok().flatten();
                    (prepared, scene)
                });
                let _ = sender.send(Computed {
                    tab,
                    revision,
                    key,
                    result,
                });
                context.request_repaint();
            });
        if spawned.is_ok() {
            self.preview.receiver = Some(receiver);
        }
    }

    fn deliver_preview(&mut self, computed: Computed) {
        let current = self.preview.request.as_ref().is_some_and(|r| {
            r.tab == computed.tab && r.revision == computed.revision && r.key == computed.key
        });
        if !current {
            return;
        }
        match computed.result {
            Ok((prepared, scene)) => {
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
