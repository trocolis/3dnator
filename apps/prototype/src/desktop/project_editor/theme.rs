//! Paleta e medidas da interface. Os painéis claros dão contraste às ferramentas;
//! a barra escura separa ações de arquivo da área de modelagem.
use super::egui;

use std::sync::atomic::{AtomicBool, Ordering};

/// Tema ativo neste quadro. As cores abaixo são lidas a cada desenho, então trocar o tema
/// (ou o do sistema) vale de imediato, sem reconstruir os painéis.
static DARK: AtomicBool = AtomicBool::new(false);

/// Escolha do usuário: claro, escuro ou o do sistema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum Choice {
    #[default]
    System,
    Light,
    Dark,
}

pub(super) fn is_dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

/// Atualiza o tema do quadro; devolve `true` se ele mudou.
pub(super) fn sync(context: &egui::Context) -> bool {
    let dark = context.theme() == egui::Theme::Dark;
    DARK.swap(dark, Ordering::Relaxed) != dark
}

const fn rgb(r: u8, g: u8, b: u8) -> egui::Color32 {
    egui::Color32::from_rgb(r, g, b)
}

macro_rules! palette {
    ($($(#[$doc:meta])* $name:ident = $light:expr, $dark:expr;)*) => {
        $(
            $(#[$doc])*
            pub(super) fn $name() -> egui::Color32 {
                if is_dark() { $dark } else { $light }
            }
        )*
    };
}

palette! {
    ink = rgb(48, 62, 80), rgb(226, 232, 240);
    muted = rgb(103, 118, 137), rgb(150, 163, 182);
    accent = rgb(35, 112, 180), rgb(96, 165, 235);
    blue_fill = rgb(225, 239, 252), rgb(32, 55, 84);
    border = rgb(218, 225, 234), rgb(58, 68, 84);
    panel = rgb(250, 252, 255), rgb(31, 36, 45);
    canvas = rgb(233, 238, 245), rgb(23, 27, 34);
    /// Fundo translúcido dos textos das cotas sobre o desenho, na cor da área de edição.
    label_fill = egui::Color32::from_rgba_unmultiplied_const(233, 238, 245, 225),
        egui::Color32::from_rgba_unmultiplied_const(23, 27, 34, 225);
    /// Véu sobre as etapas do histórico posteriores ao marcador.
    veil = egui::Color32::from_rgba_unmultiplied_const(233, 238, 245, 150),
        egui::Color32::from_rgba_unmultiplied_const(23, 27, 34, 150);
    header = rgb(40, 49, 64), rgb(15, 19, 26);
    /// Fundo de cartões, campos e janelas: branco no claro.
    surface = egui::Color32::WHITE, rgb(39, 45, 56);
    /// Cabeçalhos de painéis e passagem do mouse sobre listas.
    subtle = rgb(244, 247, 251), rgb(45, 52, 64);
    /// Etapa do histórico com falha e etapa afetada por ela.
    failed_fill = rgb(253, 225, 225), rgb(84, 38, 42);
    affected_fill = rgb(255, 238, 214), rgb(82, 60, 30);
    green = rgb(30, 126, 88), rgb(76, 192, 142);
    warning = rgb(157, 93, 15), rgb(232, 164, 74);
    danger = rgb(189, 63, 68), rgb(238, 104, 110);
    /// Rascunhos de referência da montagem, distintos dos rascunhos de peça.
    reference = rgb(124, 82, 196), rgb(172, 142, 232);
    /// Referências capturadas por Projetar e suas áreas fechadas.
    projected = rgb(158, 95, 205), rgb(192, 142, 240);
    /// Linhas da face onde o rascunho foi criado: referência fixa, distinta da projeção
    /// lilás, do laranja das linhas de referência e do âmbar dos avisos.
    face_line = rgb(214, 172, 0), rgb(232, 192, 40);
    /// Geometria auxiliar, excluída das áreas: segmentos de construção e retas infinitas.
    construction = rgb(224, 123, 31), rgb(242, 152, 72);
    /// Planos de trabalho da peça e a prévia do plano deslocado.
    work_plane = rgb(196, 146, 24), rgb(222, 176, 62);
}

/// Estilo de um tema, com a paleta correspondente.
fn style(dark: bool) -> egui::Style {
    DARK.store(dark, Ordering::Relaxed);
    let (ink, muted, accent, blue_fill, border, panel, surface) = (
        ink(),
        muted(),
        accent(),
        blue_fill(),
        border(),
        panel(),
        surface(),
    );
    let mut style = egui::Style {
        visuals: if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        },
        ..Default::default()
    };
    style.spacing.item_spacing = egui::vec2(8., 7.);
    style.spacing.button_padding = egui::vec2(10., 6.);
    style.spacing.interact_size = egui::vec2(28., 28.);
    style.spacing.indent = 18.;
    style.spacing.combo_width = 152.;
    let v = &mut style.visuals;
    v.override_text_color = Some(ink);
    v.panel_fill = panel;
    v.window_fill = surface;
    v.extreme_bg_color = surface;
    v.faint_bg_color = if dark {
        rgb(35, 41, 51)
    } else {
        rgb(241, 245, 250)
    };
    v.window_stroke = egui::Stroke::new(1., border);
    v.window_corner_radius = egui::CornerRadius::same(10);
    v.selection.bg_fill = blue_fill;
    v.selection.stroke = egui::Stroke::new(1.5, accent);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1., border);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1., muted);
    v.widgets.inactive.weak_bg_fill = surface;
    v.widgets.inactive.bg_fill = if dark {
        rgb(46, 54, 67)
    } else {
        rgb(239, 244, 250)
    };
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1., border);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1., ink);
    v.widgets.hovered.weak_bg_fill = blue_fill;
    v.widgets.hovered.bg_fill = blue_fill;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(
        1.,
        if dark {
            rgb(64, 110, 160)
        } else {
            rgb(167, 204, 238)
        },
    );
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, accent);
    v.widgets.active.bg_fill = if dark {
        rgb(40, 76, 118)
    } else {
        rgb(203, 226, 248)
    };
    v.widgets.active.fg_stroke = egui::Stroke::new(1.5, accent);
    for widget in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
    ] {
        widget.corner_radius = egui::CornerRadius::same(5);
    }
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(13.));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(13.));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(16.));
    style
}

/// Instala os dois estilos e escolhe o tema; "Sistema" segue o do sistema operacional.
pub(super) fn apply(context: &egui::Context, choice: Choice) {
    context.set_style_of(egui::Theme::Light, style(false));
    context.set_style_of(egui::Theme::Dark, style(true));
    context.set_theme(match choice {
        Choice::System => egui::ThemePreference::System,
        Choice::Light => egui::ThemePreference::Light,
        Choice::Dark => egui::ThemePreference::Dark,
    });
    sync(context);
}

pub(super) fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(panel())
        .inner_margin(12)
        .stroke(egui::Stroke::new(1., border()))
}

/// Cartão flutuante sobre a área de edição, com a borda e a sombra dos painéis de comando.
pub(super) fn floating_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(surface())
        .stroke(egui::Stroke::new(1., border()))
        .corner_radius(8)
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: egui::Color32::from_black_alpha(40),
        })
}

pub(super) fn section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(3.);
    ui.label(egui::RichText::new(label).size(11.).strong().color(muted()));
    ui.add_space(2.);
}
