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
    guide: Option<nator_commands::extrusion::Guide>,
    choice: Option<nator_document::features::Combine>,
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
                        let removal = if let Some((part, _)) = &extrusion {
                            let operation = highlight.and_then(|id| {
                                display
                                    .part(*part)?
                                    .features
                                    .iter()
                                    .find(|f| f.output == id)
                                    .map(|f| &f.operation)
                            });
                            choice = operation.and_then(nator_commands::extrusion::combination);
                            operation
                                .map(|operation| {
                                    nator_commands::extrusion::removed(&original, *part, operation)
                                })
                                .transpose()
                                .map_err(|e| CommandError {
                                    message: e.to_string(),
                                    failure: None,
                                })?
                                .flatten()
                                .map(|mesh| (*part, mesh))
                        } else {
                            None
                        };
                        if let Some((part, output)) = rollback {
                            roll_back_after(&mut display, part, output);
                        }
                        if let Some(id) = highlight.filter(|_| removal.is_none()) {
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
                        let scene = prepare_scene_with_removal(&display, &view, active, removal)
                            .map_err(|message| CommandError {
                                message,
                                failure: None,
                            })?;
                        Ok((prepared, scene))
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
                if let Some(draft) = self
                    .feature_edit
                    .as_mut()
                    .filter(|d| d.tab() == computed.tab)
                {
                    draft.receive_extrusion(computed.guide, computed.choice);
                }
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
