//! Painel de comando ancorado à área 3D, no padrão dos CADs de mercado: título com ícone,
//! linhas rotuladas, campos de seleção que recebem cliques no modelo, escolhas por ícone e
//! rodapé com o estado da pré-visualização, OK e Cancelar. Só desenha e devolve intenções;
//! os comandos continuam sendo montados pelos editores de cada ferramenta.
use super::{
    icons::{self, Icon},
    *,
};

/// Largura do painel, em pontos.
pub(super) const WIDTH: f32 = 330.;
/// Coluna dos rótulos.
const LABEL_WIDTH: f32 = 112.;

/// Situação exibida no rodapé.
pub(super) enum Footer<'a> {
    /// Sem pré-visualização (o comando é aplicado só ao confirmar).
    None,
    Waiting,
    Ready,
    Failed(&'a str),
}

/// Decisões tomadas no painel neste quadro.
#[derive(Default)]
pub(super) struct Outcome {
    pub ok: bool,
    pub cancel: bool,
}

/// A posição usa o controle inteiro, incluindo Home/Enquadrar e setas. O painel
/// fica alinhado à direita do canvas; memória de janelas antigas não muda a âncora.
fn position(canvas: egui::Rect, width: f32) -> egui::Pos2 {
    let cube = view_cube::rect(canvas);
    egui::pos2(
        (canvas.right() - width - 10.).max(canvas.left() + 10.),
        if cube.is_positive() {
            cube.bottom() + 12.
        } else {
            canvas.top() + 10.
        },
    )
}
pub(super) fn id(key: &str) -> egui::Id {
    egui::Id::new(("command_panel", key))
}
fn panel_width(canvas: egui::Rect, requested: f32) -> f32 {
    requested.min((canvas.width() - 20.).max(1.))
}

/// Identificação estável compartilhada pelos painéis de opções e de confirmação.
#[derive(Clone, Copy)]
pub(super) struct Header<'a> {
    pub id: &'a str,
    pub icon: Icon,
    pub title: &'a str,
}

fn window(
    context: &egui::Context,
    header: Header<'_>,
    canvas: egui::Rect,
    width: f32,
) -> egui::Window<'static> {
    egui::Window::new(header.title)
        .id(id(header.id))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .constrain(false)
        .fixed_pos(position(canvas, width))
        .min_width(width)
        .max_width(width)
        .frame(
            egui::Frame::window(&context.global_style())
                .inner_margin(0)
                .corner_radius(8)
                .shadow(egui::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(40),
                }),
        )
}

/// Mesmo cabeçalho do Mover: ícone, nome em maiúsculas e fechar à direita.
fn heading(ui: &mut egui::Ui, header: Header<'_>) -> bool {
    let mut close = false;
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(244, 247, 251))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .corner_radius(egui::CornerRadius {
            nw: 8,
            ne: 8,
            sw: 0,
            se: 0,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(22.), egui::Sense::hover());
                icons::paint(ui.painter(), rect, header.icon, true);
                ui.label(
                    egui::RichText::new(header.title.to_uppercase())
                        .strong()
                        .size(13.)
                        .color(theme::INK),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = icons::button(ui, Icon::Close, text(T::Cancel), false, false).clicked();
                });
            });
        });
    ui.add(egui::Separator::default().spacing(0.));
    close
}

fn content(
    ui: &mut egui::Ui,
    canvas: egui::Rect,
    width: f32,
    has_footer: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    // Reservar a altura real abaixo do navegador: em telas curtas o conteúdo
    // rola, sem empurrar o cabeçalho sobre o cubo nem esconder OK/Cancelar.
    let available = canvas.bottom()
        - position(canvas, width).y
        - 48.
        - 20.
        - 20.
        - if has_footer { 52. } else { 0. };
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            // Linhas de seleção e tabelas extensas também podem precisar rolar
            // horizontalmente; nunca alargam o painel para fora do canvas.
            egui::ScrollArea::both()
                .min_scrolled_width(0.)
                .min_scrolled_height(0.)
                .max_width((width - 24.).max(1.))
                .max_height(available.max(1.))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 6.;
                    body(ui);
                });
        });
}

/// Opções aplicadas durante o uso da ferramenta, sem um OK adicional. Os
/// editores conservam seus próprios comandos e a semântica de Enter/Esc.
pub(super) fn options(
    context: &egui::Context,
    header: Header<'_>,
    canvas: egui::Rect,
    width: f32,
    body: impl FnOnce(&mut egui::Ui),
) -> Outcome {
    let mut outcome = Outcome::default();
    let width = panel_width(canvas, width);
    window(context, header, canvas, width).show(context, |ui| {
        ui.set_width(width);
        outcome.cancel = heading(ui, header);
        content(ui, canvas, width, false, body);
    });
    outcome
}

/// Painel flutuante com cabeçalho, conteúdo rolável e rodapé.
pub(super) fn show(
    context: &egui::Context,
    header: Header<'_>,
    canvas: egui::Rect,
    footer: Footer<'_>,
    ok_enabled: bool,
    body: impl FnOnce(&mut egui::Ui),
) -> Outcome {
    show_validated(context, header, canvas, footer, ok_enabled, |ui| {
        body(ui);
        true
    })
}

/// A validade é colhida depois de desenhar os campos, antes do rodapé: digitar um texto
/// inválido e clicar em OK no mesmo quadro nunca publica o último valor válido.
pub(super) fn show_validated(
    context: &egui::Context,
    header: Header<'_>,
    canvas: egui::Rect,
    footer: Footer<'_>,
    ok_enabled: bool,
    body: impl FnOnce(&mut egui::Ui) -> bool,
) -> Outcome {
    let mut outcome = Outcome::default();
    let mut fields_valid = true;
    // Enter termina a edição do campo antes de confirmar o comando inteiro.
    let typing = context.egui_wants_keyboard_input();
    let width = panel_width(canvas, WIDTH);
    window(context, header, canvas, width).show(context, |ui| {
        ui.set_width(width);
        outcome.cancel = heading(ui, header);
        content(ui, canvas, width, true, |ui| {
            fields_valid = body(ui);
        });
        ui.add(egui::Separator::default().spacing(0.));
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    match footer {
                        Footer::None => {}
                        Footer::Waiting => {
                            ui.spinner();
                            ui.label(
                                egui::RichText::new(text(T::PreviewComputing))
                                    .small()
                                    .color(theme::MUTED),
                            );
                        }
                        Footer::Ready => {
                            let (rect, _) = ui
                                .allocate_exact_size(egui::Vec2::splat(16.), egui::Sense::hover());
                            icons::paint(ui.painter(), rect, Icon::Check, true);
                            ui.label(
                                egui::RichText::new(text(T::PreviewReady))
                                    .small()
                                    .color(theme::MUTED),
                            );
                        }
                        Footer::Failed(_) => {
                            let (rect, _) = ui
                                .allocate_exact_size(egui::Vec2::splat(16.), egui::Sense::hover());
                            icons::paint(ui.painter(), rect, Icon::Warning, true);
                            ui.label(
                                egui::RichText::new(text(T::PreviewFailed))
                                    .small()
                                    .color(theme::DANGER),
                            );
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(text(T::Cancel)).clicked() {
                            outcome.cancel = true;
                        }
                        let ok = egui::Button::new(
                            egui::RichText::new(text(T::Ok))
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(theme::ACCENT)
                        .min_size(egui::vec2(64., 26.));
                        if ui.add_enabled(ok_enabled && fields_valid, ok).clicked() {
                            outcome.ok = true;
                        }
                    });
                });
                if let Footer::Failed(message) = footer {
                    ui.label(egui::RichText::new(message).small().color(theme::DANGER));
                }
            });
    });
    // Teclado como nos CADs: Esc cancela e Enter confirma, fora de campos em edição (neles,
    // Esc e Enter apenas encerram a digitação).
    if !typing && context.input(|i| i.key_pressed(egui::Key::Escape)) {
        outcome.cancel = true;
    }
    if ok_enabled && fields_valid && !typing && context.input(|i| i.key_pressed(egui::Key::Enter)) {
        outcome.ok = true;
    }
    outcome
}

/// Linha rotulada: rótulo à esquerda, controles à direita.
pub(super) fn row<R>(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(LABEL_WIDTH, ui.spacing().interact_size.y),
            egui::Sense::hover(),
        );
        let galley = ui.painter().layout(
            label.to_owned(),
            egui::FontId::proportional(12.5),
            theme::MUTED,
            LABEL_WIDTH - 4.,
        );
        ui.painter().galley(
            egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.),
            galley,
            theme::MUTED,
        );
        add(ui)
    })
    .inner
}

/// Campo de seleção: ativo, recebe os próximos cliques no modelo. Mostra a quantidade
/// escolhida e permite limpar.
#[derive(Default)]
pub(super) struct Selection {
    pub activate: bool,
    pub clear: bool,
}
pub(super) fn selection(
    ui: &mut egui::Ui,
    label: &str,
    chosen: usize,
    description: Option<&str>,
    active: bool,
) -> Selection {
    let mut result = Selection::default();
    row(ui, label, |ui| {
        let caption = match (chosen, description) {
            (0, _) => text(T::SelectPrompt).to_owned(),
            (1, Some(description)) => description.to_owned(),
            (n, _) => format!("{n} {}", text(T::SelectedCount)),
        };
        let width = (ui.available_width() - 30.).max(80.);
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 26.), egui::Sense::click());
        let (fill, stroke) = if active {
            (theme::BLUE_FILL, egui::Stroke::new(1.5, theme::ACCENT))
        } else if chosen > 0 {
            (egui::Color32::WHITE, egui::Stroke::new(1., theme::ACCENT))
        } else {
            (
                egui::Color32::WHITE,
                egui::Stroke::new(1., egui::Color32::from_rgb(196, 207, 220)),
            )
        };
        ui.painter()
            .rect(rect, 4, fill, stroke, egui::StrokeKind::Inside);
        let icon = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 13., rect.center().y),
            egui::Vec2::splat(15.),
        );
        icons::paint(
            ui.painter(),
            icon,
            if chosen > 0 {
                Icon::Check
            } else {
                Icon::Select
            },
            true,
        );
        let mut job = egui::text::LayoutJob::simple(
            caption,
            egui::FontId::proportional(12.5),
            if chosen > 0 || active {
                theme::ACCENT
            } else {
                theme::MUTED
            },
            (rect.width() - 30.).max(1.),
        );
        job.wrap.max_rows = 1;
        let galley = ui.painter().layout_job(job);
        ui.painter().galley(
            egui::pos2(rect.left() + 25., rect.center().y - galley.size().y / 2.),
            galley,
            theme::ACCENT,
        );
        if response.clicked() {
            result.activate = true;
        }
        if chosen > 0
            && icons::button(ui, Icon::Close, text(T::ClearSelection), false, false).clicked()
        {
            result.clear = true;
        }
    });
    result
}

/// Escolha exclusiva entre opções com ícone; devolve verdadeiro se mudou.
pub(super) fn choice<V: PartialEq + Copy>(
    ui: &mut egui::Ui,
    value: &mut V,
    options: &[(V, Icon, T)],
) -> bool {
    let mut changed = false;
    ui.spacing_mut().item_spacing.x = 2.;
    for (option, icon, label) in options {
        let selected = *value == *option;
        let response = icons::button(ui, *icon, text(*label), false, selected);
        if response.clicked() && !selected {
            *value = *option;
            changed = true;
        }
    }
    changed
}

/// Campo de expressão dimensional (aceita parâmetros e unidades).
pub(super) fn expression(ui: &mut egui::Ui, value: &mut String) -> egui::Response {
    ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(ui.available_width().min(170.))
            .font(egui::TextStyle::Monospace),
    )
    .on_hover_text(text(T::ExpressionHint))
}

/// Grandeza de um campo numérico digitável.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Quantity {
    /// Milímetros.
    Length,
    /// Graus.
    Angle,
}
impl Quantity {
    /// Unidade mostrada no rótulo da linha; o campo exibe só o número.
    pub(super) fn unit(self) -> &'static str {
        match self {
            Self::Length => "mm",
            Self::Angle => "°",
        }
    }
}

/// Lê o texto de um campo numérico: vírgula ou ponto decimal, unidades e contas, sem
/// parâmetros ("12,5", "1 in", "90 + 45"). Ângulos sem unidade são graus.
pub(super) fn parse_number(input: &str, quantity: Quantity) -> Option<f64> {
    // O avaliador já aceita vírgula decimal; argumentos de funções usam ponto e vírgula.
    let parameters = nator_document::parameters::Parameters::default();
    let value = match quantity {
        Quantity::Length => parameters
            .evaluate(input.trim(), nator_document::parameters::Dimension::Length)
            .ok()?,
        Quantity::Angle => parameters
            .evaluate(input.trim(), nator_document::parameters::Dimension::Angle)
            .ok()?
            .to_degrees(),
    };
    value.is_finite().then_some(value)
}

/// Limite simétrico dos deslocamentos (mm) e rotações (graus) do painel Mover.
/// Valores fora da faixa são recusados, sem truncamento silencioso.
const NUMBER_LIMIT: f64 = 1e7;

/// Texto transitório pertencente ao painel, preservado inclusive fora do foco. Fechar o
/// painel descarta esse estado; uma nova abertura nunca herda erros da anterior.
#[derive(Default)]
pub(super) struct NumberInput {
    buffer: Option<String>,
    last_value: Option<f64>,
}

impl NumberInput {
    pub fn is_valid(&self, quantity: Quantity) -> bool {
        self.buffer
            .as_ref()
            .is_none_or(|s| Self::parse(s, quantity).is_some())
    }

    fn parse(buffer: &str, quantity: Quantity) -> Option<f64> {
        parse_number(buffer, quantity).filter(|v| v.abs() <= NUMBER_LIMIT)
    }

    /// Campo digitável: cada expressão válida atualiza a prévia. Texto inválido continua
    /// visível ao perder foco; Esc volta ao último valor válido. Arrastar o manipulador
    /// sincroniza o campo com o novo valor sem reutilizar a expressão anterior.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        value: &mut f64,
        quantity: Quantity,
        decimals: usize,
        width: f32,
    ) -> egui::Response {
        if self.last_value != Some(*value) {
            self.buffer = None;
        }
        let id = ui.make_persistent_id(id_salt);
        let mut buffer = self
            .buffer
            .take()
            .unwrap_or_else(|| nator_drawing::format_fixed(*value, decimals));
        let response = ui.add(
            egui::TextEdit::singleline(&mut buffer)
                .id(id)
                .desired_width(width)
                .horizontal_align(egui::Align::RIGHT),
        );
        let parsed = Self::parse(&buffer, quantity);
        if response.changed()
            && let Some(parsed) = parsed
        {
            *value = parsed;
        }
        if response.gained_focus()
            && !response.changed()
            && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id)
        {
            let end = egui::text::CCursor::new(buffer.chars().count());
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    end,
                )));
            state.store(ui.ctx(), id);
        }
        let cancel = (response.has_focus() || response.lost_focus())
            && ui.input(|i| i.key_pressed(egui::Key::Escape));
        if cancel {
            // O egui pode retirar o foco já no início do quadro de Esc. Consumir o
            // evento impede que o rodapé também o interprete como cancelar o painel.
            ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        }
        if !cancel && parsed.is_none() {
            ui.painter().rect_stroke(
                response.rect.expand(1.),
                3,
                egui::Stroke::new(1.5, theme::DANGER),
                egui::StrokeKind::Outside,
            );
        }
        if !cancel && (response.has_focus() || parsed.is_none()) {
            self.buffer = Some(buffer);
        }
        self.last_value = Some(*value);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

    /// Memória de posição de janelas anteriores e mudanças de tamanho não podem
    /// recolocar uma ferramenta sobre o navegador. O conteúdo longo deve rolar.
    #[test]
    fn tool_panels_stay_below_the_view_control_after_reopening_and_resizing() -> TestResult<()> {
        let context = egui::Context::default();
        for (size, requested_width) in [
            (egui::vec2(1440., 900.), WIDTH),
            (egui::vec2(900., 600.), WIDTH),
            (egui::vec2(900., 600.), 720.),
        ] {
            let canvas =
                egui::Rect::from_min_max(egui::pos2(268., 198.), egui::pos2(size.x, size.y - 42.));
            let input = || egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            };
            let mut old = context.run_ui(input(), |_| {
                egui::Window::new("Posição antiga")
                    .id(id("options_position_test"))
                    .fixed_pos(canvas.left_top())
                    .show(&context, |ui| {
                        ui.label("Anterior");
                    });
            });
            old.textures_delta.clear();
            for _ in 0..2 {
                let mut output = context.run_ui(input(), |_| {
                    options(
                        &context,
                        Header {
                            id: "options_position_test",
                            icon: Icon::Line,
                            title: "Linha",
                        },
                        canvas,
                        requested_width,
                        |ui| {
                            ui.allocate_exact_size(
                                egui::vec2(requested_width * 2., 20.),
                                egui::Sense::hover(),
                            );
                            for index in 0..80 {
                                ui.label(format!("Campo {index}"));
                            }
                        },
                    );
                    show(
                        &context,
                        Header {
                            id: "command_position_test",
                            icon: Icon::Move,
                            title: "Mover",
                        },
                        canvas,
                        Footer::None,
                        true,
                        |ui| {
                            ui.allocate_exact_size(
                                egui::vec2(requested_width * 2., 20.),
                                egui::Sense::hover(),
                            );
                            for index in 0..80 {
                                ui.label(format!("Campo {index}"));
                            }
                        },
                    );
                });
                output.textures_delta.clear();
            }
            for key in ["options_position_test", "command_position_test"] {
                let rect = egui::AreaState::load(&context, id(key))
                    .ok_or("Painel ausente")?
                    .rect();
                assert!(
                    (rect.top() - (view_cube::rect(canvas).bottom() + 12.)).abs() < 1.,
                    "{key}: {rect:?}"
                );
                assert!(
                    rect.left() >= canvas.left() && rect.right() <= canvas.right(),
                    "{key}: {rect:?}"
                );
                assert!(
                    rect.bottom() <= canvas.bottom(),
                    "{key}: {rect:?}; canvas: {canvas:?}"
                );
            }
        }
        Ok(())
    }

    /// Eventos reais do egui exercitam foco, edição e confirmação, sem adaptador gráfico.
    fn number_frame(
        context: &egui::Context,
        input: &mut NumberInput,
        value: &mut f64,
        events: Vec<egui::Event>,
        focus: bool,
    ) -> TestResult<(egui::Response, Outcome)> {
        let mut response = None;
        let mut outcome = Outcome::default();
        let canvas = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.));
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(canvas),
                events,
                ..Default::default()
            },
            |_| {
                outcome = show_validated(
                    context,
                    Header {
                        id: "number_test",
                        icon: Icon::Move,
                        title: "Mover",
                    },
                    canvas,
                    Footer::None,
                    true,
                    |ui| {
                        let field = input.show(ui, "number", value, Quantity::Length, 2, 100.);
                        if focus {
                            field.request_focus();
                        }
                        response = Some(field);
                        input.is_valid(Quantity::Length)
                    },
                );
            },
        );
        output.textures_delta.clear();
        Ok((response.ok_or("Campo não composto")?, outcome))
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn replace(text: &str) -> Vec<egui::Event> {
        vec![
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            },
            egui::Event::Text(text.into()),
        ]
    }

    #[test]
    fn invalid_numbers_survive_blur_and_block_panel_confirmation() -> TestResult<()> {
        let context = egui::Context::default();
        let mut input = NumberInput::default();
        let mut value = 5.;
        number_frame(&context, &mut input, &mut value, vec![], true)?;
        let (field, _) = number_frame(&context, &mut input, &mut value, vec![], false)?;
        number_frame(&context, &mut input, &mut value, replace("12 +"), false)?;
        assert_eq!(value, 5.);
        context.memory_mut(|m| m.surrender_focus(field.id));
        let (_, outcome) = number_frame(
            &context,
            &mut input,
            &mut value,
            vec![key(egui::Key::Enter)],
            false,
        )?;
        assert_eq!(input.buffer.as_deref(), Some("12 +"));
        assert!(!input.is_valid(Quantity::Length));
        assert!(!outcome.ok);
        // Uma mudança externa (por exemplo, arrastar o manipulador) substitui o texto.
        value = 30.;
        number_frame(&context, &mut input, &mut value, vec![], false)?;
        assert!(input.is_valid(Quantity::Length));
        assert!(input.buffer.is_none());
        Ok(())
    }

    #[test]
    fn enter_finishes_typing_before_confirming_and_escape_restores_invalid_input() -> TestResult<()>
    {
        let context = egui::Context::default();
        let mut input = NumberInput::default();
        let mut value = 5.;
        number_frame(&context, &mut input, &mut value, vec![], true)?;
        number_frame(&context, &mut input, &mut value, vec![], false)?;
        let (_, outcome) = number_frame(
            &context,
            &mut input,
            &mut value,
            {
                let mut events = replace("1 in");
                events.push(key(egui::Key::Enter));
                events
            },
            false,
        )?;
        assert_eq!(value, 25.4);
        assert!(!outcome.ok);
        number_frame(&context, &mut input, &mut value, vec![], false)?;
        let (_, outcome) = number_frame(
            &context,
            &mut input,
            &mut value,
            vec![key(egui::Key::Enter)],
            false,
        )?;
        assert!(outcome.ok);

        number_frame(&context, &mut input, &mut value, vec![], true)?;
        number_frame(&context, &mut input, &mut value, vec![], false)?;
        number_frame(&context, &mut input, &mut value, replace("1 / 0"), false)?;
        assert!(!input.is_valid(Quantity::Length));
        let (_, outcome) = number_frame(
            &context,
            &mut input,
            &mut value,
            vec![key(egui::Key::Escape)],
            false,
        )?;
        assert!(!outcome.cancel);
        assert!(input.is_valid(Quantity::Length));
        assert_eq!(value, 25.4);
        Ok(())
    }

    #[test]
    fn out_of_range_numbers_are_rejected_without_clamping() -> TestResult<()> {
        let context = egui::Context::default();
        let mut input = NumberInput::default();
        let mut value = 5.;
        number_frame(&context, &mut input, &mut value, vec![], true)?;
        number_frame(&context, &mut input, &mut value, vec![], false)?;
        number_frame(&context, &mut input, &mut value, replace("10000001"), false)?;
        assert_eq!(value, 5.);
        assert!(!input.is_valid(Quantity::Length));
        assert_eq!(NumberInput::parse("-10000001", Quantity::Length), None);
        assert_eq!(NumberInput::parse("10000000", Quantity::Length), Some(1e7));
        Ok(())
    }

    #[test]
    fn clicking_a_number_selects_its_text_and_allows_correcting_an_error() -> TestResult<()> {
        let context = egui::Context::default();
        let mut input = NumberInput::default();
        let mut value = 5.;
        // Dois quadros estabilizam a posição inicial da janela do egui.
        number_frame(&context, &mut input, &mut value, vec![], false)?;
        let (field, _) = number_frame(&context, &mut input, &mut value, vec![], false)?;
        let position = field.rect.center();
        for pressed in [true, false] {
            number_frame(
                &context,
                &mut input,
                &mut value,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                false,
            )?;
        }
        number_frame(
            &context,
            &mut input,
            &mut value,
            vec![egui::Event::Text("12 +".into())],
            false,
        )?;
        assert_eq!(input.buffer.as_deref(), Some("12 +"));
        assert_eq!(value, 5.);
        // Completar a conta recupera a validade sem restaurar ou redigitar o prefixo.
        number_frame(
            &context,
            &mut input,
            &mut value,
            vec![egui::Event::Text("3".into())],
            false,
        )?;
        assert!(input.is_valid(Quantity::Length));
        assert_eq!(value, 15.);
        Ok(())
    }

    #[test]
    fn typed_numbers_accept_decimal_comma_units_and_arithmetic() {
        assert_eq!(parse_number("12,5", Quantity::Length), Some(12.5));
        assert_eq!(parse_number("12,50 mm", Quantity::Length), Some(12.5));
        assert_eq!(parse_number("1 in", Quantity::Length), Some(25.4));
        assert_eq!(parse_number("-2 cm", Quantity::Length), Some(-20.));
        assert_eq!(
            parse_number("max(12,5 mm; 2 mm)", Quantity::Length),
            Some(12.5)
        );
        assert_eq!(
            parse_number("(12,5 + 2,5) * 2", Quantity::Length),
            Some(30.)
        );
        assert_eq!(parse_number("90 + 45", Quantity::Angle), Some(135.));
        assert!(parse_number("45°", Quantity::Angle).is_some_and(|v| (v - 45.).abs() < 1e-9));
        assert!(
            parse_number("0,5 rad", Quantity::Angle)
                .is_some_and(|v| (v - 0.5_f64.to_degrees()).abs() < 1e-9)
        );
        // Unidade incompatível ou texto incompleto não alteram o campo.
        assert_eq!(parse_number("10 deg", Quantity::Length), None);
        assert_eq!(parse_number("12 +", Quantity::Length), None);
        assert_eq!(parse_number("", Quantity::Length), None);
    }
}
