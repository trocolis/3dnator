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
    // Teclado como nos CADs: Esc cancela; Enter confirma fora de campos com várias linhas.
    if context.input(|i| i.key_pressed(egui::Key::Escape)) {
        outcome.cancel = true;
    }
    if ok_enabled && context.input(|i| i.key_pressed(egui::Key::Enter)) {
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
