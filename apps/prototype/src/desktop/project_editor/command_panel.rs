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

/// Posição inicial: canto superior esquerdo da área 3D, abaixo da faixa de câmera. Deixa
/// livres o centro, onde o modelo costuma estar enquadrado, e o cubo de orientação.
pub(super) fn default_position(canvas: egui::Rect) -> egui::Pos2 {
    egui::pos2(canvas.left() + 10., canvas.top() + 44.)
}

/// Identificação do painel: chave de memória, ícone e nome da ferramenta.
pub(super) struct Header<'a> {
    pub id: &'a str,
    pub icon: Icon,
    pub title: &'a str,
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
    let Header { id, icon, title } = header;
    let mut outcome = Outcome::default();
    // Campo em edição no início do quadro: Enter só conclui o valor digitado; o próximo
    // Enter, fora do campo, confirma o painel.
    let typing = context.egui_wants_keyboard_input();
    egui::Window::new(title)
        .id(egui::Id::new(("command_panel", id)))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(default_position(canvas))
        .min_width(WIDTH)
        .max_width(WIDTH)
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
        .show(context, |ui| {
            ui.set_width(WIDTH);
            // Cabeçalho: ícone e nome da ferramenta em maiúsculas; fechar cancela.
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
                        icons::paint(ui.painter(), rect, icon, true);
                        ui.label(
                            egui::RichText::new(title.to_uppercase())
                                .strong()
                                .size(13.)
                                .color(theme::INK),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icons::button(ui, Icon::Close, text(T::Cancel), false, false)
                                .clicked()
                            {
                                outcome.cancel = true;
                            }
                        });
                    });
                });
            ui.add(egui::Separator::default().spacing(0.));
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height((canvas.height() - 260.).max(180.))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 6.;
                            body(ui);
                        });
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
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::Vec2::splat(16.),
                                    egui::Sense::hover(),
                                );
                                icons::paint(ui.painter(), rect, Icon::Check, true);
                                ui.label(
                                    egui::RichText::new(text(T::PreviewReady))
                                        .small()
                                        .color(theme::MUTED),
                                );
                            }
                            Footer::Failed(_) => {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::Vec2::splat(16.),
                                    egui::Sense::hover(),
                                );
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
                            if ui.add_enabled(ok_enabled, ok).clicked() {
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
    if ok_enabled && !typing && context.input(|i| i.key_pressed(egui::Key::Enter)) {
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
    // Vírgula decimal, exceto dentro de funções, onde separa argumentos.
    let input = if input.contains('(') {
        input.to_owned()
    } else {
        input.replace(',', ".")
    };
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

/// Campo numérico digitável, como nos CADs: mostra o valor na unidade da grandeza; ao
/// receber o foco, seleciona o texto para ser substituído. Cada texto válido atualiza o valor; um
/// texto inválido fica destacado e não altera o campo.
pub(super) fn number(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut f64,
    quantity: Quantity,
    decimals: usize,
    width: f32,
) -> egui::Response {
    let id = ui.make_persistent_id(id_salt);
    let shown = || nator_drawing::format_fixed(*value, decimals);
    let mut buffer = if ui.memory(|m| m.has_focus(id)) {
        ui.data(|d| d.get_temp::<String>(id)).unwrap_or_else(shown)
    } else {
        shown()
    };
    let response = ui.add(
        egui::TextEdit::singleline(&mut buffer)
            .id(id)
            .desired_width(width)
            .horizontal_align(egui::Align::RIGHT),
    );
    if response.changed()
        && let Some(parsed) = parse_number(&buffer, quantity)
    {
        *value = parsed;
    }
    if response.gained_focus()
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
    if response.has_focus() {
        if parse_number(&buffer, quantity).is_none() {
            ui.painter().rect_stroke(
                response.rect.expand(1.),
                3,
                egui::Stroke::new(1.5, theme::DANGER),
                egui::StrokeKind::Outside,
            );
        }
        ui.data_mut(|d| d.insert_temp(id, buffer));
    } else {
        ui.data_mut(|d| d.remove::<String>(id));
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_numbers_accept_decimal_comma_units_and_arithmetic() {
        assert_eq!(parse_number("12,5", Quantity::Length), Some(12.5));
        assert_eq!(parse_number("12,50 mm", Quantity::Length), Some(12.5));
        assert_eq!(parse_number("1 in", Quantity::Length), Some(25.4));
        assert_eq!(parse_number("-2 cm", Quantity::Length), Some(-20.));
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
