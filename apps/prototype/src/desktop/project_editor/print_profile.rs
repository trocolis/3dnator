//! Perfil de impressão do projeto. Só grandezas físicas: serve a qualquer impressora ou
//! fatiador. A biblioteca é pessoal; o projeto guarda a própria cópia do perfil escolhido.
use super::{
    command_panel::{self as panel, Footer},
    icons::Icon,
    *,
};
use nator_document::print_profile::{self as profile, Fastener, PrintProfile};

/// Valores de impressão são sempre em mm: é a unidade de bicos, camadas e fatiadores.
fn value<'a>(
    target: &'a mut f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
) -> egui::DragValue<'a> {
    panel::localized(egui::DragValue::new(target))
        .speed(0.01)
        .range(range)
        .max_decimals(3)
        .suffix(suffix.to_owned())
}

pub(super) struct Draft {
    pub tab: u64,
    profile: PrintProfile,
    library: Vec<PrintProfile>,
    error: Option<String>,
    /// Fixador em edição na tabela.
    fastener: usize,
}
impl Draft {
    fn new(tab: u64, profile: PrintProfile, library: Result<Vec<PrintProfile>, String>) -> Self {
        let (library, error) = match library {
            Ok(library) => (library, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        Self {
            tab,
            profile,
            library,
            error,
            fastener: 0,
        }
    }
    /// Grava sobre o perfil de mesmo nome; a biblioteca fica em ordem alfabética.
    fn remember(&mut self) -> Vec<PrintProfile> {
        let mut library = self.library.clone();
        library.retain(|p| p.name != self.profile.name);
        library.push(self.profile.clone());
        library.sort_by(|a, b| a.name.cmp(&b.name));
        library
    }
}

/// Medidas de um fixador por vez: a tabela inteira não cabe na largura do painel.
fn fastener_table(ui: &mut egui::Ui, profile: &mut PrintProfile, selected: &mut usize) {
    const SIZES: [T; 8] = [
        T::FastenerDiameter,
        T::FastenerPilot,
        T::FastenerInsertDiameter,
        T::FastenerInsertLength,
        T::FastenerNutFlats,
        T::FastenerNutHeight,
        T::FastenerHeadDiameter,
        T::FastenerHeadHeight,
    ];
    *selected = (*selected).min(profile.fasteners.len().saturating_sub(1));
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("print_fastener")
            .width(80.)
            .selected_text(
                profile
                    .fasteners
                    .get(*selected)
                    .map_or("—", |f| f.name.as_str()),
            )
            .show_ui(ui, |ui| {
                for (index, fastener) in profile.fasteners.iter().enumerate() {
                    ui.selectable_value(selected, index, &fastener.name);
                }
            });
        if ui.button(text(T::FastenerAdd)).clicked() {
            let template = profile.fasteners.get(*selected).cloned();
            profile.fasteners.push(Fastener {
                name: text(T::FastenerNew).into(),
                ..template.unwrap_or_else(|| PrintProfile::default().fasteners[0].clone())
            });
            *selected = profile.fasteners.len() - 1;
        }
        if ui
            .add_enabled(
                profile.fasteners.len() > 1,
                egui::Button::new(text(T::DeleteFeature)),
            )
            .clicked()
        {
            profile.fasteners.remove(*selected);
        }
    });
    let Some(fastener) = profile.fasteners.get_mut(*selected) else {
        return;
    };
    panel::row(ui, text(T::PrintProfileName), |ui| {
        ui.text_edit_singleline(&mut fastener.name)
    });
    for (label, size) in SIZES.into_iter().zip(fastener.sizes_mut()) {
        panel::row(ui, text(label), |ui| ui.add(value(size, 0.1..=100., " mm")));
    }
}

fn library_path() -> Option<std::path::PathBuf> {
    nator_project_io::recovery_directory()
        .parent()
        .map(|p| p.join("print-profiles.json"))
}

impl Editor {
    pub(super) fn open_print_profile(&mut self) {
        // Um arquivo ilegível é informado, nunca sobrescrito por uma lista vazia.
        let library = library_path()
            .map(|path| nator_project_io::read_print_profiles(&path))
            .transpose()
            .map(Option::unwrap_or_default)
            .map_err(|e| e.to_string());
        let profile = self.current().session.document().print_profile().clone();
        self.print_profile_edit = Some(Draft::new(self.current().id, profile, library));
    }

    pub(super) fn print_profile_dialog(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.print_profile_edit.take() else {
            return;
        };
        let Some(current) = self
            .tabs
            .iter()
            .find(|t| t.id == draft.tab)
            .map(|t| t.session.document().print_profile().clone())
        else {
            return;
        };
        let readable = draft.error.is_none();
        let valid = draft.profile.validate();
        let mut save = false;
        let outcome = panel::show(
            context,
            panel::Header {
                id: "print_profile",
                icon: Icon::PrintProfile,
                title: text(T::PrintProfile),
            },
            self.canvas_rect,
            Footer::None,
            valid.is_ok() && !self.busy(),
            |ui| {
                let d = &mut draft;
                panel::row(ui, text(T::PrintProfileName), |ui| {
                    ui.text_edit_singleline(&mut d.profile.name)
                });
                panel::row(ui, text(T::PrintProfileLibrary), |ui| {
                    // A largura reserva o botão Salvar na mesma linha do painel estreito.
                    egui::ComboBox::from_id_salt("print_profile_library")
                        .width(92.)
                        .selected_text(text(T::PrintProfileChoose))
                        .show_ui(ui, |ui| {
                            if ui.button(PrintProfile::default().name).clicked() {
                                d.profile = PrintProfile::default();
                            }
                            for saved in &d.library {
                                if ui.button(&saved.name).clicked() {
                                    d.profile = saved.clone();
                                }
                            }
                        });
                    save = ui
                        .add_enabled(
                            readable && d.profile.validate().is_ok(),
                            egui::Button::new(text(T::PrintProfileSave)),
                        )
                        .clicked();
                });
                let p = &mut d.profile;
                panel::row(ui, text(T::PrintNozzle), |ui| {
                    ui.add(value(&mut p.nozzle_mm, profile::NOZZLE_RANGE_MM, " mm"))
                });
                panel::row(ui, text(T::PrintLayer), |ui| {
                    ui.add(value(&mut p.layer_mm, profile::LAYER_RANGE_MM, " mm"))
                });
                for (label, target) in [
                    (T::PrintFitTight, &mut p.fit_tight_mm),
                    (T::PrintFitSliding, &mut p.fit_sliding_mm),
                    (T::PrintFitFree, &mut p.fit_free_mm),
                    (T::PrintHoleCompensation, &mut p.hole_compensation_mm),
                ] {
                    panel::row(ui, text(label), |ui| {
                        ui.add(value(target, profile::FIT_RANGE_MM, " mm"))
                    });
                }
                panel::row(ui, text(T::PrintOverhang), |ui| {
                    ui.add(value(
                        &mut p.max_overhang_deg,
                        profile::OVERHANG_RANGE_DEG,
                        "°",
                    ))
                });
                panel::row(ui, text(T::PrintDensity), |ui| {
                    ui.add(value(
                        &mut p.density_g_cm3,
                        profile::DENSITY_RANGE_G_CM3,
                        " g/cm³",
                    ))
                });
                panel::row(ui, text(T::PrintPrice), |ui| {
                    ui.add(
                        panel::localized(egui::DragValue::new(&mut p.price_per_kg))
                            .speed(1.)
                            .range(profile::PRICE_RANGE)
                            .max_decimals(2),
                    )
                });
                panel::row(ui, text(T::PrintInfill), |ui| {
                    ui.add(
                        panel::localized(egui::DragValue::new(&mut p.infill_pct))
                            .speed(1.)
                            .range(profile::INFILL_RANGE_PCT)
                            .suffix(" %"),
                    )
                });
                panel::row(ui, text(T::PrintWallLines), |ui| {
                    ui.add(
                        egui::DragValue::new(&mut p.wall_lines)
                            .speed(0.1)
                            .range(profile::WALL_LINES_RANGE)
                            .max_decimals(0),
                    )
                });
                panel::row(ui, text(T::PrintMaxStrain), |ui| {
                    ui.add(
                        panel::localized(egui::DragValue::new(&mut p.max_strain_pct))
                            .speed(0.1)
                            .range(profile::STRAIN_RANGE_PCT)
                            .max_decimals(1)
                            .suffix(" %"),
                    )
                });
                let mut limited = p.bed_mm.is_some();
                if ui.checkbox(&mut limited, text(T::PrintBedLimit)).changed() {
                    p.bed_mm = limited.then_some([220., 220., 250.]);
                }
                if let Some(bed) = &mut p.bed_mm {
                    for (label, side) in ["X", "Y", "Z"].into_iter().zip(bed) {
                        panel::row(ui, label, |ui| {
                            ui.add(value(side, profile::BED_RANGE_MM, " mm"))
                        });
                    }
                }
                panel::hint(ui, text(T::PrintProfileHint));
                panel::hint(
                    ui,
                    &format!(
                        "{}: {}",
                        text(T::PrintProfileNames),
                        p.parameters().into_keys().collect::<Vec<_>>().join(", ")
                    ),
                );
                if let Err(error) = &valid {
                    ui.colored_label(theme::warning(), error.to_string());
                }
                if let Some(error) = &d.error {
                    ui.colored_label(theme::warning(), error);
                }
                egui::CollapsingHeader::new(text(T::Fasteners)).show(ui, |ui| {
                    fastener_table(ui, &mut d.profile, &mut d.fastener);
                });
            },
        );
        if outcome.cancel {
            return;
        }
        if save {
            let library = draft.remember();
            let written = library_path()
                .ok_or_else(|| text(T::PrintProfileNoLibrary).to_owned())
                .and_then(|path| {
                    nator_project_io::write_print_profiles(&path, &library)
                        .map_err(|e| e.to_string())
                });
            match written {
                Ok(()) => draft.library = library,
                Err(error) => draft.error = Some(error),
            }
        }
        if outcome.ok {
            if draft.profile != current {
                self.prepare_command(
                    context,
                    draft.tab,
                    Command::SetPrintProfile {
                        profile: draft.profile,
                    },
                );
            }
            return;
        }
        self.print_profile_edit = Some(draft);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saving_replaces_the_same_name_and_keeps_the_library_sorted() {
        let saved = |name: &str, nozzle_mm| PrintProfile {
            name: name.into(),
            nozzle_mm,
            ..Default::default()
        };
        let mut draft = Draft::new(
            1,
            saved("Bico 0,6", 0.6),
            Ok(vec![saved("Bico 0,6", 0.4), saved("Abs", 0.4)]),
        );
        let library = draft.remember();
        assert_eq!(library.len(), 2);
        assert_eq!(library[0].name, "Abs");
        assert!((library[1].nozzle_mm - 0.6).abs() < 1e-12);
    }

    #[test]
    fn unreadable_libraries_are_reported_and_start_empty() {
        let draft = Draft::new(1, PrintProfile::default(), Err("Falhou".into()));
        assert!(draft.library.is_empty());
        assert_eq!(draft.error.as_deref(), Some("Falhou"));
    }
}
