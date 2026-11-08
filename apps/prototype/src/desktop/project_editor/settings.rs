//! Painel de configurações da aplicação, numa coluna única dividida em seções. Desenho
//! guarda as escolhas do documento (unidade, precisões e casas); Aparência e os atalhos
//! tratam do aplicativo. Atalhos e Sobre abrem janelas próprias.
use super::{
    Command, Editor, T, command_panel, egui,
    icons::{self, Icon},
    text, theme,
};
use nator_document::LengthUnit;

const REPOSITORY: &str = "https://github.com/trocolis/3dnator";
/// Coluna dos rótulos: as entradas de todas as seções começam no mesmo ponto.
const LABEL_WIDTH: f32 = 190.;
/// Largura comum das entradas, seletores, campos numéricos e botões.
const FIELD_WIDTH: f32 = 170.;
/// Largura da tecla na janela de atalhos.
const KEY_WIDTH: f32 = 130.;
const ICON_SIZE: f32 = 18.;

#[derive(Default)]
pub(super) struct Panel {
    open: bool,
    shortcuts: bool,
    about: bool,
    /// Atalho esperando a tecla nova na janela de atalhos.
    capture: Option<&'static str>,
}

impl Panel {
    pub(super) fn open(&mut self) {
        self.open = true;
    }

    pub(super) fn open_shortcuts(&mut self) {
        self.shortcuts = true;
    }

    pub(super) fn open_about(&mut self) {
        self.about = true;
    }

    pub(super) fn capturing(&self) -> bool {
        self.shortcuts && self.capture.is_some()
    }
}

impl Editor {
    pub(super) fn settings_dialog(&mut self, context: &egui::Context) {
        if self.settings_panel.open {
            let mut open = true;
            egui::Window::new(text(T::SettingsMenu))
                .id(egui::Id::new("application_settings"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(context, |ui| {
                    ui.spacing_mut().item_spacing.y = 6.;
                    heading(ui, T::SettingsAppearance);
                    self.settings_appearance(ui);
                    ui.add_space(8.);
                    heading(ui, T::SettingsDrawing);
                    self.settings_drawing(ui);
                    ui.add_space(4.);
                    ui.separator();
                    if ui.button(text(T::SettingsAboutButton)).clicked() {
                        self.settings_panel.about = true;
                    }
                });
            self.settings_panel.open = open;
        }
        if self.settings_panel.shortcuts {
            let mut open = true;
            egui::Window::new(text(T::SettingsShortcuts))
                .id(egui::Id::new("application_shortcuts"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(context.content_rect().center())
                .show(context, |ui| self.settings_shortcuts(ui));
            self.settings_panel.shortcuts = open;
            if !open {
                self.settings_panel.capture = None;
            }
        }
        if self.settings_panel.about {
            let mut open = true;
            egui::Window::new(text(T::SettingsAbout))
                .id(egui::Id::new("application_about"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(context.content_rect().center())
                .show(context, |ui| {
                    ui.set_width(420.);
                    about(ui);
                });
            self.settings_panel.about = open;
        }
    }

    /// Tema, idioma e tamanho da faixa: escolhas pessoais, gravadas fora do projeto e sem desfazer.
    fn settings_appearance(&mut self, ui: &mut egui::Ui) {
        use crate::i18n::Language;
        let mut next = self.preferences.clone();
        row(ui, T::SettingsTheme, T::SettingsThemeHint, |ui| {
            choice(
                ui,
                "settings_theme",
                &mut next.theme,
                &[
                    (theme::Choice::System, text(T::SettingsSystem)),
                    (theme::Choice::Light, text(T::SettingsThemeLight)),
                    (theme::Choice::Dark, text(T::SettingsThemeDark)),
                ],
            )
        });
        // Cada idioma aparece no próprio nome, para ser achado por quem não lê o atual.
        row(ui, T::SettingsLanguage, T::SettingsLanguageHint, |ui| {
            choice(
                ui,
                "settings_language",
                &mut next.language,
                &[
                    (None, text(T::SettingsSystem)),
                    (Some(Language::Portuguese), "Português"),
                    (Some(Language::Spanish), "Español"),
                    (Some(Language::English), "English"),
                ],
            )
        });
        row(ui, T::RibbonSizeMenu, T::RibbonSizeHint, |ui| {
            use super::preferences::RibbonSize;
            choice(
                ui,
                "settings_ribbon",
                &mut next.ribbon,
                &[
                    (RibbonSize::Large, text(T::RibbonLarge)),
                    (RibbonSize::Normal, text(T::RibbonNormal)),
                    (RibbonSize::Compact, text(T::RibbonCompact)),
                ],
            )
        });
        if next != self.preferences {
            self.preferences = next;
            self.save_preferences(ui.ctx());
        }
    }

    /// Escolhas do documento: unidade, precisões do mouse e casas exibidas. Cada
    /// alteração é uma ação do documento, reversível por desfazer. Os atalhos, do
    /// aplicativo, ficam aqui por serem a ferramenta de quem desenha.
    fn settings_drawing(&mut self, ui: &mut egui::Ui) {
        let current = self.current().session.document().settings();
        let mut next = current;
        let height = ui.spacing().interact_size.y;
        ui.add_enabled_ui(!self.busy(), |ui| {
            row(ui, T::SettingsUnit, T::SettingsUnitHint, |ui| {
                choice(
                    ui,
                    "settings_unit",
                    &mut next.unit,
                    &[
                        (LengthUnit::Millimeter, text(T::SettingsUnitMillimeter)),
                        (LengthUnit::Inch, text(T::SettingsUnitInch)),
                    ],
                )
            });
            // As caixas aplicam o valor ao confirmar a digitação e não respondem a
            // arraste, que criaria dezenas de ações.
            row(ui, T::SettingsMouseStep, T::SettingsMouseStepHint, |ui| {
                let context = ui.ctx().clone();
                ui.add_sized(
                    [FIELD_WIDTH, height],
                    command_panel::length_value(
                        &context,
                        &mut next.mouse_step_mm,
                        nator_document::MIN_MOUSE_STEP_MM..=nator_document::MAX_MOUSE_STEP_MM,
                    )
                    .speed(0.)
                    .max_decimals(usize::from(nator_document::MAX_DISPLAY_DECIMALS))
                    .update_while_editing(false),
                )
            });
            row(
                ui,
                T::SettingsMouseAngleStep,
                T::SettingsMouseStepHint,
                |ui| {
                    ui.add_sized(
                        [FIELD_WIDTH, height],
                        command_panel::localized(egui::DragValue::new(
                            &mut next.mouse_angle_step_deg,
                        ))
                        .speed(0.)
                        .range(
                            nator_document::MIN_MOUSE_ANGLE_STEP_DEG
                                ..=nator_document::MAX_MOUSE_ANGLE_STEP_DEG,
                        )
                        .max_decimals(2)
                        .suffix("°")
                        .update_while_editing(false),
                    )
                },
            );
            row(ui, T::DisplayDecimals, T::DisplayDecimalsHint, |ui| {
                ui.add_sized(
                    [FIELD_WIDTH, height],
                    egui::DragValue::new(&mut next.display_decimals)
                        .range(0..=nator_document::MAX_DISPLAY_DECIMALS)
                        .speed(0.)
                        .update_while_editing(false),
                )
            });
        });
        row(ui, T::SettingsShortcuts, T::SettingsShortcutsHint, |ui| {
            let button = ui.add_sized(
                [FIELD_WIDTH, height],
                egui::Button::new(text(T::SettingsConfigure)),
            );
            if button.clicked() {
                self.settings_panel.shortcuts = true;
            }
            button
        });
        if next != current {
            self.execute(Command::SetSettings { settings: next });
        }
    }

    /// Atalhos por contexto, em três colunas: ícone, ação e tecla. Clicar na tecla
    /// espera a nova combinação; Esc desiste. Uma combinação já usada no mesmo
    /// contexto é recusada com aviso.
    fn settings_shortcuts(&mut self, ui: &mut egui::Ui) {
        use super::shortcuts::{self, Scope};
        if let Some(id) = self.settings_panel.capture {
            let pressed = ui.input(|i| {
                i.events.iter().find_map(|event| match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some((*modifiers, *key)),
                    _ => None,
                })
            });
            if let Some((modifiers, key)) = pressed {
                self.settings_panel.capture = None;
                if key != egui::Key::Escape {
                    let combo = (modifiers, key);
                    match shortcuts::conflict(&self.preferences.shortcuts, id, combo) {
                        Some(other) => self.status.warning(format!(
                            "{}: {}",
                            text(T::ShortcutConflict),
                            text(other.target.label())
                        )),
                        None => {
                            self.preferences
                                .shortcuts
                                .insert(id.to_owned(), shortcuts::format(combo));
                            self.save_preferences(ui.ctx());
                        }
                    }
                }
            }
        }
        let all = shortcuts::bindings();
        let height = ui.spacing().interact_size.y;
        egui::ScrollArea::vertical()
            .max_height(460.)
            .show(ui, |ui| {
                // Uma grade só para todos os contextos mantém as colunas alinhadas.
                egui::Grid::new("application_shortcuts_grid")
                    .num_columns(3)
                    .spacing([12., 4.])
                    .min_col_width(ICON_SIZE)
                    .show(ui, |ui| {
                        for scope in [
                            Scope::System,
                            Scope::Workspace,
                            Scope::Model,
                            Scope::Assembly,
                            Scope::Sketch,
                        ] {
                            ui.label("");
                            heading(ui, scope.label());
                            ui.end_row();
                            for binding in all.iter().filter(|b| b.scope == scope) {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::Vec2::splat(ICON_SIZE),
                                    egui::Sense::hover(),
                                );
                                icons::paint(ui.painter(), rect, binding.target.icon(), true);
                                fixed_label(ui, text(binding.target.label()));
                                let waiting = self.settings_panel.capture == Some(binding.id);
                                let label = if waiting {
                                    text(T::ShortcutWaiting).to_owned()
                                } else {
                                    shortcuts::format(shortcuts::effective(
                                        &self.preferences.shortcuts,
                                        binding,
                                    ))
                                };
                                let key = ui
                                    .add(
                                        egui::Button::new(label)
                                            .selected(waiting)
                                            .min_size(egui::vec2(KEY_WIDTH, height)),
                                    )
                                    .on_hover_text(text(T::ShortcutsHint));
                                if key.clicked() {
                                    self.settings_panel.capture = Some(binding.id);
                                }
                                ui.end_row();
                            }
                        }
                    });
            });
        ui.separator();
        if ui
            .add_enabled(
                !self.preferences.shortcuts.is_empty(),
                egui::Button::new(text(T::ShortcutsReset)),
            )
            .clicked()
        {
            self.preferences.shortcuts.clear();
            self.save_preferences(ui.ctx());
        }
    }
}

fn about(ui: &mut egui::Ui) {
    ui.add_space(4.);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 76.), egui::Sense::hover());
    let icon = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(72.));
    ui.painter().rect_filled(icon, 18, theme::blue_fill());
    ui.painter().rect_stroke(
        icon,
        18,
        egui::Stroke::new(1., theme::border()),
        egui::StrokeKind::Inside,
    );
    icons::paint(ui.painter(), icon.shrink(12.), Icon::Cube, true);
    ui.add_space(6.);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(text(T::AppName))
                .size(24.)
                .strong()
                .color(theme::ink()),
        );
        ui.label(
            egui::RichText::new(format!(
                "{} {}",
                text(T::SettingsAboutVersion),
                env!("CARGO_PKG_VERSION")
            ))
            .size(13.)
            .color(theme::muted()),
        );
    });
    ui.add_space(10.);
    ui.label(egui::RichText::new(text(T::SettingsAboutDescription)).size(13.));
    ui.add_space(8.);
    ui.separator();
    ui.add_space(6.);
    let format = nator_project_io::FORMAT_VERSION.to_string();
    egui::Grid::new("application_about")
        .num_columns(2)
        .spacing([24., 7.])
        .show(ui, |ui| {
            about_row(
                ui,
                T::SettingsAboutAuthor,
                text(T::SettingsAboutAuthorValue),
            );
            about_row(
                ui,
                T::SettingsAboutLicense,
                text(T::SettingsAboutLicenseValue),
            );
            about_row(ui, T::SettingsAboutProjectFormat, &format);
            ui.label(egui::RichText::new(text(T::SettingsAboutRepository)).color(theme::muted()));
            ui.hyperlink_to(text(T::SettingsAboutRepositoryLink), REPOSITORY);
            ui.end_row();
            about_row(
                ui,
                T::SettingsAboutTechnologies,
                text(T::SettingsAboutTechnologiesValue),
            );
        });
    ui.add_space(12.);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(text(T::SettingsAboutCopyright))
                .size(11.)
                .color(theme::muted()),
        );
    });
}

fn about_row(ui: &mut egui::Ui, label: T, value: &str) {
    ui.label(egui::RichText::new(text(label)).color(theme::muted()));
    ui.label(egui::RichText::new(value).strong());
    ui.end_row();
}

fn heading(ui: &mut egui::Ui, label: T) {
    ui.label(
        egui::RichText::new(text(label))
            .size(12.)
            .strong()
            .color(theme::muted()),
    );
}

/// Linha de configuração: rótulo na coluna fixa e a entrada em seguida. A descrição
/// aparece em balão sobre o rótulo e sobre a entrada.
fn row(ui: &mut egui::Ui, label: T, hint: T, field: impl FnOnce(&mut egui::Ui) -> egui::Response) {
    ui.horizontal(|ui| {
        fixed_label(ui, text(label)).on_hover_text(text(hint));
        field(ui).on_hover_text(text(hint));
    });
}

/// Rótulo alinhado à esquerda na largura da coluna de rótulos; o excesso é cortado.
fn fixed_label(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let height = ui.spacing().interact_size.y;
    ui.allocate_ui_with_layout(
        egui::vec2(LABEL_WIDTH, height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(LABEL_WIDTH);
            ui.add(egui::Label::new(label).truncate())
        },
    )
    .inner
}

/// Seletor de uma opção entre poucas, com a largura comum das entradas.
fn choice<V: Clone + PartialEq>(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut V,
    options: &[(V, &str)],
) -> egui::Response {
    let current = options
        .iter()
        .find(|(option, _)| option == value)
        .map_or("", |(_, label)| *label);
    egui::ComboBox::from_id_salt(id)
        .width(FIELD_WIDTH)
        .selected_text(current)
        .show_ui(ui, |ui| {
            for (option, label) in options {
                ui.selectable_value(value, option.clone(), *label);
            }
        })
        .response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_starts_closed() {
        let mut panel = Panel::default();
        assert!(!panel.open && !panel.shortcuts && !panel.about);
        panel.open();
        assert!(panel.open);
    }

    #[test]
    fn captures_keys_only_with_the_shortcuts_window() {
        let mut panel = Panel {
            capture: Some("save"),
            ..Panel::default()
        };
        assert!(!panel.capturing());
        panel.open_shortcuts();
        assert!(panel.capturing());
    }
}
