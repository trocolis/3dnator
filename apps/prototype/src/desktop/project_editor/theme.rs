//! Paleta e medidas da interface. Os painéis claros dão contraste às ferramentas;
//! a barra escura separa ações de arquivo da área de modelagem.
use super::egui;

pub(super) const INK: egui::Color32 = egui::Color32::from_rgb(48, 62, 80);
pub(super) const MUTED: egui::Color32 = egui::Color32::from_rgb(103, 118, 137);
pub(super) const ACCENT: egui::Color32 = egui::Color32::from_rgb(35, 112, 180);
pub(super) const BLUE_FILL: egui::Color32 = egui::Color32::from_rgb(225, 239, 252);
pub(super) const BORDER: egui::Color32 = egui::Color32::from_rgb(218, 225, 234);
pub(super) const PANEL: egui::Color32 = egui::Color32::from_rgb(250, 252, 255);
pub(super) const CANVAS: egui::Color32 = egui::Color32::from_rgb(233, 238, 245);
pub(super) const HEADER: egui::Color32 = egui::Color32::from_rgb(40, 49, 64);
pub(super) const GREEN: egui::Color32 = egui::Color32::from_rgb(30, 126, 88);
pub(super) const WARNING: egui::Color32 = egui::Color32::from_rgb(157, 93, 15);
pub(super) const DANGER: egui::Color32 = egui::Color32::from_rgb(189, 63, 68);

pub(super) fn apply(context: &egui::Context) {
    let mut style = egui::Style {
        visuals: egui::Visuals::light(),
        ..Default::default()
    };
    style.spacing.item_spacing = egui::vec2(8., 7.);
    style.spacing.button_padding = egui::vec2(10., 6.);
    style.spacing.interact_size = egui::vec2(28., 28.);
    style.spacing.indent = 18.;
    style.spacing.combo_width = 152.;
    style.visuals.override_text_color = Some(INK);
    style.visuals.panel_fill = PANEL;
    style.visuals.window_fill = egui::Color32::WHITE;
    style.visuals.extreme_bg_color = egui::Color32::WHITE;
    style.visuals.faint_bg_color = egui::Color32::from_rgb(241, 245, 250);
    style.visuals.window_stroke = egui::Stroke::new(1., BORDER);
    style.visuals.window_corner_radius = egui::CornerRadius::same(10);
    style.visuals.selection.bg_fill = BLUE_FILL;
    style.visuals.selection.stroke = egui::Stroke::new(1.5, ACCENT);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1., BORDER);
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1., MUTED);
    style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::WHITE;
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(239, 244, 250);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1., BORDER);
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1., INK);
    style.visuals.widgets.hovered.weak_bg_fill = BLUE_FILL;
    style.visuals.widgets.hovered.bg_fill = BLUE_FILL;
    style.visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1., egui::Color32::from_rgb(167, 204, 238));
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, ACCENT);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(203, 226, 248);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, ACCENT);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
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
    context.set_theme(egui::ThemePreference::Light);
    context.set_style_of(egui::Theme::Light, style);
}

pub(super) fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .inner_margin(12)
        .stroke(egui::Stroke::new(1., BORDER))
}

pub(super) fn section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(3.);
    ui.label(egui::RichText::new(label).size(11.).strong().color(MUTED));
    ui.add_space(2.);
}
