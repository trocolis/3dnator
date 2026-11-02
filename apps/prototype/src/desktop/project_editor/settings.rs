//! Painel de configurações da aplicação. Desenho guarda as escolhas do documento
//! (unidade, precisões e casas); as demais abas tratam do aplicativo.
use super::{
    Command, Editor, T, command_panel, egui,
    icons::{self, Icon},
    text, theme,
};
use nator_document::LengthUnit;

const REPOSITORY: &str = "https://github.com/trocolis/3dnator";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Tab {
    Appearance,
    #[default]
    Drawing,
    Shortcut,
    Help,
    About,
}

#[derive(Default)]
pub(super) struct Panel {
    open: bool,
    tab: Tab,
    /// Atalho esperando a tecla nova na aba Atalhos.
    capture: Option<&'static str>,
    /// Busca da aba Ajuda.
    search: String,
}

impl Panel {
    pub(super) fn open(&mut self) {
        self.open = true;
    }

    pub(super) fn capturing(&self) -> bool {
        self.open && self.capture.is_some()
    }

    pub(super) fn open_at(&mut self, tab: Tab) {
        self.tab = tab;
        self.open();
    }
}

impl Editor {
    pub(super) fn settings_dialog(&mut self, context: &egui::Context) {
        if !self.settings_panel.open {
            return;
        }
        let mut open = self.settings_panel.open;
        let mut recovery = false;
        egui::Window::new(text(T::SettingsMenu))
            .id(egui::Id::new("application_settings"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(540.)
            .show(context, |ui| {
                ui.set_min_width(540.);
                ui.set_min_height(230.);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.;
                    for (tab, label) in [
                        (Tab::Appearance, T::SettingsAppearance),
                        (Tab::Drawing, T::SettingsDrawing),
                        (Tab::Shortcut, T::SettingsShortcuts),
                        (Tab::Help, T::SettingsHelp),
                        (Tab::About, T::SettingsAbout),
                    ] {
                        ui.selectable_value(&mut self.settings_panel.tab, tab, text(label));
                    }
                });
                ui.separator();
                ui.add_space(8.);
                match self.settings_panel.tab {
                    Tab::Appearance => self.settings_appearance(ui),
                    Tab::Shortcut => self.settings_shortcuts(ui),
                    Tab::Drawing => self.settings_drawing(ui),
                    Tab::Help => recovery = self.settings_help(ui),
                    Tab::About => about(ui),
                }
            });
        self.settings_panel.open = open;
        if recovery {
            self.settings_panel.open = false;
            self.show_recovery = true;
        }
    }

    /// Atalhos por contexto. Clicar na tecla espera a nova combinação; Esc desiste.
    /// Uma combinação já usada no mesmo contexto é recusada com aviso.
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
        hint(ui, text(T::ShortcutsHint));
        let all = shortcuts::bindings();
        egui::ScrollArea::vertical()
            .max_height(320.)
            .show(ui, |ui| {
                for scope in [
                    Scope::System,
                    Scope::Workspace,
                    Scope::Model,
                    Scope::Assembly,
                    Scope::Sketch,
                ] {
                    theme::section(ui, text(scope.label()));
                    egui::Grid::new(("shortcuts", scope as u8))
                        .num_columns(2)
                        .spacing([24., 4.])
                        .show(ui, |ui| {
                            for binding in all.iter().filter(|b| b.scope == scope) {
                                ui.label(text(binding.target.label()));
                                let waiting = self.settings_panel.capture == Some(binding.id);
                                let label = if waiting {
                                    text(T::ShortcutWaiting).to_owned()
                                } else {
                                    shortcuts::format(shortcuts::effective(
                                        &self.preferences.shortcuts,
                                        binding,
                                    ))
                                };
                                if ui.selectable_label(waiting, label).clicked() {
                                    self.settings_panel.capture = Some(binding.id);
                                }
                                ui.end_row();
                            }
                        });
                }
            });
        if !self.preferences.shortcuts.is_empty() && ui.button(text(T::ShortcutsReset)).clicked() {
            self.preferences.shortcuts.clear();
            self.save_preferences(ui.ctx());
        }
    }

    /// Tema e idioma: escolhas pessoais, gravadas fora do projeto e sem desfazer.
    fn settings_appearance(&mut self, ui: &mut egui::Ui) {
        let mut next = self.preferences.clone();
        ui.horizontal(|ui| {
            ui.label(text(T::SettingsTheme));
            for (choice, label) in [
                (theme::Choice::System, T::SettingsSystem),
                (theme::Choice::Light, T::SettingsThemeLight),
                (theme::Choice::Dark, T::SettingsThemeDark),
            ] {
                ui.selectable_value(&mut next.theme, choice, text(label));
            }
        });
        ui.add_space(8.);
        ui.horizontal(|ui| {
            use crate::i18n::Language;
            ui.label(text(T::SettingsLanguage));
            // Cada idioma aparece no próprio nome, para ser achado por quem não lê o atual.
            ui.selectable_value(&mut next.language, None, text(T::SettingsSystem));
            for (language, name) in [
                (Language::Portuguese, "Português"),
                (Language::Spanish, "Español"),
                (Language::English, "English"),
            ] {
                ui.selectable_value(&mut next.language, Some(language), name);
            }
        });
        hint(ui, text(T::SettingsAppearanceHint));
        if next != self.preferences {
            self.preferences = next;
            self.save_preferences(ui.ctx());
        }
    }

    /// Escolhas do documento: unidade, precisões do mouse e casas exibidas. Cada
    /// alteração é uma ação do documento, reversível por desfazer.
    fn settings_drawing(&mut self, ui: &mut egui::Ui) {
        let current = self.current().session.document().settings();
        let mut next = current;
        ui.add_enabled_ui(!self.busy(), |ui| {
            ui.horizontal(|ui| {
                ui.label(text(T::SettingsUnit));
                for (unit, label) in [
                    (LengthUnit::Millimeter, T::SettingsUnitMillimeter),
                    (LengthUnit::Inch, T::SettingsUnitInch),
                ] {
                    if ui
                        .selectable_value(&mut next.unit, unit, text(label))
                        .clicked()
                        && unit != current.unit
                    {
                        // Os atalhos de precisão mudam com a unidade: o passo segue o
                        // atalho equivalente (0,4 mm → 1/64 in), ou o primeiro deles.
                        let index = current
                            .unit
                            .mouse_step_presets_mm()
                            .iter()
                            .position(|v| (v - current.mouse_step_mm).abs() < 1e-9);
                        let presets = unit.mouse_step_presets_mm();
                        if let Some(index) = index {
                            next.mouse_step_mm = presets[index.min(presets.len() - 1)];
                        }
                    }
                }
            });
            hint(ui, text(T::SettingsUnitHint));
            ui.add_space(8.);
            // Cada alteração vira uma ação do documento: as caixas aplicam o valor ao
            // confirmar a digitação e não respondem a arraste, que criaria dezenas delas.
            ui.horizontal(|ui| {
                ui.label(text(T::SettingsMouseStep));
                let custom_id = ui.id().with("custom_mouse_step");
                let presets = next.unit.mouse_step_presets_mm();
                let preset = presets
                    .iter()
                    .any(|value| (value - next.mouse_step_mm).abs() < 1e-9);
                let mut custom =
                    !preset || ui.ctx().data(|data| data.get_temp::<bool>(custom_id)) == Some(true);
                for &value in presets {
                    let label = match next.unit {
                        LengthUnit::Millimeter => format!("{value:.1}").replace('.', ","),
                        LengthUnit::Inch => format!("1/{}", (25.4 / value).round()),
                    };
                    let selected = !custom && (value - next.mouse_step_mm).abs() < 1e-9;
                    if ui.selectable_label(selected, label).clicked() {
                        next.mouse_step_mm = value;
                        custom = false;
                    }
                }
                if ui
                    .selectable_label(custom, text(T::SettingsMouseStepCustom))
                    .clicked()
                {
                    custom = true;
                }
                ui.ctx()
                    .data_mut(|data| data.insert_temp(custom_id, custom));
                if custom {
                    let context = ui.ctx().clone();
                    ui.add(
                        command_panel::length_value(
                            &context,
                            &mut next.mouse_step_mm,
                            nator_document::MIN_MOUSE_STEP_MM..=nator_document::MAX_MOUSE_STEP_MM,
                        )
                        .speed(0.)
                        .max_decimals(usize::from(nator_document::MAX_DISPLAY_DECIMALS))
                        .update_while_editing(false),
                    );
                }
            });
            ui.horizontal(|ui| {
                ui.label(text(T::SettingsMouseAngleStep));
                ui.add(
                    command_panel::localized(egui::DragValue::new(&mut next.mouse_angle_step_deg))
                        .speed(0.)
                        .range(
                            nator_document::MIN_MOUSE_ANGLE_STEP_DEG
                                ..=nator_document::MAX_MOUSE_ANGLE_STEP_DEG,
                        )
                        .max_decimals(2)
                        .suffix("°")
                        .update_while_editing(false),
                );
            });
            hint(ui, text(T::SettingsMouseStepHint));
            ui.add_space(8.);
            ui.horizontal(|ui| {
                ui.label(text(T::DisplayDecimals));
                ui.add(
                    egui::DragValue::new(&mut next.display_decimals)
                        .range(0..=nator_document::MAX_DISPLAY_DECIMALS),
                );
            });
            hint(ui, text(T::DisplayDecimalsHint));
        });
        if next != current {
            self.execute(Command::SetSettings { settings: next });
        }
    }

    /// Tutorial por tarefa, com busca, e a lista dos atalhos em vigor.
    fn settings_help(&mut self, ui: &mut egui::Ui) -> bool {
        use super::{help, shortcuts};
        ui.horizontal(|ui| {
            ui.label(text(T::HelpSearch));
            ui.add(
                egui::TextEdit::singleline(&mut self.settings_panel.search)
                    .desired_width(260.)
                    .hint_text(text(T::HelpSearchHint)),
            );
        });
        let query = self.settings_panel.search.clone();
        egui::ScrollArea::vertical()
            .max_height(300.)
            .show(ui, |ui| {
                let mut found = false;
                for topic in help::TOPICS.iter().filter(|t| help::matches(t, &query)) {
                    found = true;
                    egui::CollapsingHeader::new(egui::RichText::new(topic.title).strong())
                        .default_open(!query.trim().is_empty())
                        .show(ui, |ui| {
                            for (number, step) in topic.steps.iter().enumerate() {
                                ui.label(format!("{}. {step}", number + 1));
                            }
                        });
                }
                // Os atalhos acompanham o que o usuário configurou.
                let title = text(T::SettingsShortcuts);
                let all = shortcuts::bindings();
                let rows = all
                    .iter()
                    .map(|b| {
                        (
                            text(b.target.label()),
                            shortcuts::format(shortcuts::effective(&self.preferences.shortcuts, b)),
                        )
                    })
                    .filter(|(name, key)| {
                        query.trim().is_empty()
                            || help::fold(&format!("{title} {name} {key}"))
                                .contains(&help::fold(query.trim()))
                    })
                    .collect::<Vec<_>>();
                if !rows.is_empty() {
                    found = true;
                    egui::CollapsingHeader::new(egui::RichText::new(title).strong()).show(
                        ui,
                        |ui| {
                            egui::Grid::new("help_shortcuts")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    for (name, key) in rows {
                                        ui.label(name);
                                        ui.label(egui::RichText::new(key).monospace());
                                        ui.end_row();
                                    }
                                });
                        },
                    );
                }
                if !found {
                    hint(ui, text(T::HelpNothingFound));
                }
            });
        ui.separator();
        hint(ui, text(T::SettingsRecoveryHint));
        ui.add_enabled(
            !self.busy() && !self.recoveries.is_empty(),
            egui::Button::new(text(T::Recovery)),
        )
        .clicked()
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
            row(
                ui,
                T::SettingsAboutAuthor,
                text(T::SettingsAboutAuthorValue),
            );
            row(
                ui,
                T::SettingsAboutLicense,
                text(T::SettingsAboutLicenseValue),
            );
            row(ui, T::SettingsAboutProjectFormat, &format);
            ui.label(egui::RichText::new(text(T::SettingsAboutRepository)).color(theme::muted()));
            ui.hyperlink_to(text(T::SettingsAboutRepositoryLink), REPOSITORY);
            ui.end_row();
            row(
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

fn row(ui: &mut egui::Ui, label: T, value: &str) {
    ui.label(egui::RichText::new(text(label)).color(theme::muted()));
    ui.label(egui::RichText::new(value).strong());
    ui.end_row();
}

fn hint(ui: &mut egui::Ui, message: &str) {
    ui.label(egui::RichText::new(message).size(12.).color(theme::muted()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_starts_closed_on_drawing() {
        let mut panel = Panel::default();
        assert!(!panel.open);
        assert_eq!(panel.tab, Tab::Drawing);
        panel.open();
        assert!(panel.open);
    }

    #[test]
    fn opens_the_requested_tab() {
        let mut panel = Panel::default();
        panel.open_at(Tab::About);
        assert!(panel.open);
        assert_eq!(panel.tab, Tab::About);
    }
}
