//! Verificar encaixe: lista interferências e folgas pequenas entre os corpos como estão
//! posicionados. Nada é gravado no projeto; o clique leva a câmera até o ponto.
use super::{icons::Icon, *};
use nator_commands::fit_check::{self, Report, Severity};

/// Contatos listados; os demais entram na contagem.
const LISTED: usize = 30;

fn label(severity: Severity) -> T {
    match severity {
        Severity::Interference => T::FitInterference,
        Severity::Touching => T::FitTouching,
        Severity::Tight => T::FitTight,
    }
}

pub(super) struct Draft {
    pub tab: u64,
    /// Peça comparada com o resto; sem ela, os componentes da montagem entre si.
    part: Option<PartId>,
    report: Option<(u64, Result<Report, String>)>,
    /// Comparação pedida e ainda não iniciada.
    pending: bool,
    /// Ponto escolhido, em coordenadas da montagem.
    focus: Option<Point3>,
}

impl Editor {
    pub(super) fn open_fit_check(&mut self) {
        self.fit_check = Some(Draft {
            tab: self.current().id,
            part: self.current().active_part,
            report: None,
            pending: true,
            focus: None,
        });
    }

    pub(super) fn fit_check_focus(&self) -> Option<Point3> {
        self.fit_check
            .as_ref()
            .filter(|d| d.tab == self.current().id)?
            .focus
    }

    pub(super) fn receive_fit_check(
        &mut self,
        tab: u64,
        revision: u64,
        report: Result<Report, String>,
    ) {
        if let Some(draft) = self.fit_check.as_mut().filter(|d| d.tab == tab) {
            draft.report = Some((revision, report));
        }
    }

    pub(super) fn fit_check_dialog(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.fit_check.take() else {
            return;
        };
        let Some(tab) = self.tabs.iter().find(|t| t.id == draft.tab) else {
            return;
        };
        let document = tab.session.document().clone();
        let revision = tab.session.revision();
        if draft.pending && !self.busy() {
            draft.pending = false;
            draft.report = None;
            let (tab_id, part) = (draft.tab, draft.part);
            self.start(context, None, move || {
                Ok(Outcome::FitChecked {
                    tab: tab_id,
                    revision,
                    report: fit_check::check(&document, part).map_err(|e| e.to_string()),
                })
            });
        }
        let settings = command_panel::settings(context);
        let outcome = command_panel::options(
            context,
            command_panel::Header {
                id: "fit_check",
                icon: Icon::FitCheck,
                title: text(T::FitCheck),
            },
            self.canvas_rect,
            command_panel::WIDTH,
            |ui| {
                let previous = draft.part;
                let active = self.tabs.iter().find(|t| t.id == draft.tab);
                if let Some(part) = active.and_then(|t| t.active_part) {
                    command_panel::row(ui, text(T::FitScopeField), |ui| {
                        command_panel::choice(
                            ui,
                            &mut draft.part,
                            &[
                                (Some(part), Icon::NewBody, T::FitScopePart),
                                (None, Icon::Group, T::FitScopeAll),
                            ],
                        );
                    });
                }
                if draft.part != previous {
                    draft.pending = true;
                }
                let Some((checked, report)) = &draft.report else {
                    command_panel::hint(ui, text(T::FitChecking));
                    return;
                };
                let report = match report {
                    Err(error) => {
                        ui.colored_label(theme::warning(), error);
                        return;
                    }
                    Ok(report) => report,
                };
                if *checked != revision {
                    ui.horizontal(|ui| {
                        ui.colored_label(theme::warning(), text(T::FitStale));
                        if ui.button(text(T::FitAgain)).clicked() {
                            draft.pending = true;
                        }
                    });
                }
                command_panel::row(ui, text(T::FitCompared), |ui| {
                    ui.label(report.compared.to_string())
                });
                if report.beyond_limit > 0 {
                    ui.colored_label(
                        theme::warning(),
                        format!("{}: {}", text(T::FitBeyond), report.beyond_limit),
                    );
                }
                if !report.skipped.is_empty() {
                    command_panel::hint(
                        ui,
                        &format!("{}: {}", text(T::FitSkipped), report.skipped.len()),
                    );
                }
                for (names, error) in report.failed.iter().take(3) {
                    ui.colored_label(
                        theme::warning(),
                        format!(
                            "{}: {} × {} ({error})",
                            text(T::FitFailed),
                            names[0],
                            names[1]
                        ),
                    );
                }
                let clear = !report
                    .contacts
                    .iter()
                    .any(|c| c.severity == Severity::Interference);
                if clear {
                    ui.colored_label(theme::green(), text(T::FitClean));
                }
                for contact in report.contacts.iter().take(LISTED) {
                    let measure = match contact.severity {
                        Severity::Interference => {
                            format!("{:.3} mm³", contact.volume_mm3).replace('.', ",")
                        }
                        _ => format!(
                            "{} {}",
                            settings.format_compact_length(contact.distance_mm),
                            settings.unit.suffix()
                        ),
                    };
                    let line = format!(
                        "{}: {measure}\n{} × {}",
                        text(label(contact.severity)),
                        contact.names[0],
                        contact.names[1]
                    );
                    let color = match contact.severity {
                        Severity::Interference => theme::warning(),
                        _ => theme::muted(),
                    };
                    let chosen = draft.focus == Some(contact.point);
                    if ui
                        .selectable_label(chosen, egui::RichText::new(line).small().color(color))
                        .clicked()
                    {
                        draft.focus = Some(contact.point);
                    }
                }
                if report.contacts.len() > LISTED {
                    command_panel::hint(ui, &format!("… +{}", report.contacts.len() - LISTED));
                }
            },
        );
        if !outcome.cancel {
            self.fit_check = Some(draft);
        }
    }
}
