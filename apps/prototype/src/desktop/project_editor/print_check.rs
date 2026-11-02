//! Verificar para impressão: resultado em cores na própria cena e regiões clicáveis. Nada é
//! gravado no projeto: as cores somem ao fechar o painel.
use super::{icons::Icon, *};
use nator_commands::print_check::{self, Report};
use nator_geometry::printability::Issue;

/// Regiões listadas por problema; as demais entram na contagem.
const LISTED: usize = 6;

fn label(issue: Issue) -> T {
    match issue {
        Issue::Overhang => T::IssueOverhang,
        Issue::ThinWall => T::IssueThinWall,
        Issue::Gap => T::IssueGap,
    }
}

pub(super) struct Draft {
    pub tab: u64,
    part: PartId,
    body: Option<BodyId>,
    /// Análise do corpo na revisão indicada.
    report: Option<(u64, BodyId, Result<Report, String>)>,
    hidden: Vec<Issue>,
    /// Região escolhida, em coordenadas da peça.
    focus: Option<Point3>,
}
impl Draft {
    fn current(&self, revision: u64) -> bool {
        self.report
            .as_ref()
            .is_some_and(|(r, body, _)| *r == revision && Some(*body) == self.body)
    }
}

impl Editor {
    pub(super) fn open_print_check(&mut self) {
        let Some(part) = self.current().active_part else {
            return;
        };
        let definition = self.current().session.document().part(part);
        let selected = match self.current().selection {
            Some(Selection::Body { id, part: owner }) if owner == part => Some(id),
            _ => None,
        };
        let body =
            selected.or_else(|| definition.and_then(|p| p.shown_bodies().map(|b| b.id).last()));
        self.print_check = Some(Draft {
            tab: self.current().id,
            part,
            body,
            report: None,
            hidden: Vec::new(),
            focus: None,
        });
    }

    /// Fecha o painel e devolve a cena às cores do modelo.
    pub(super) fn close_print_check(&mut self) {
        if let Some(draft) = self.print_check.take()
            && let Some(tab) = self.find_tab(draft.tab)
            && tab.view.inspection.take().is_some()
        {
            tab.view.touched();
        }
    }

    #[cfg(all(test, feature = "occt"))]
    pub(super) fn hide_print_issue_for_capture(&mut self, issue: Issue) {
        if let Some(draft) = &mut self.print_check {
            draft.hidden.push(issue);
        }
        self.publish_inspection();
    }

    pub(super) fn print_check_focus(&self) -> Option<Point3> {
        let draft = self
            .print_check
            .as_ref()
            .filter(|d| d.tab == self.current().id)?;
        let placement = sketch_scene::part_placement(self.current(), draft.part).ok()?;
        draft.focus.map(|point| placement.apply(point))
    }

    fn publish_inspection(&mut self) {
        let Some(draft) = &self.print_check else {
            return;
        };
        let inspection = match &draft.report {
            Some((_, _, Ok(report))) => Some(Arc::new(
                report.inspection(|issue| !draft.hidden.contains(&issue)),
            )),
            _ => None,
        };
        let tab = draft.tab;
        if let Some(target) = self.find_tab(tab) {
            target.view.inspection = inspection;
            target.view.touched();
        }
    }

    pub(super) fn receive_print_check(
        &mut self,
        tab: u64,
        revision: u64,
        body: BodyId,
        report: Result<Report, String>,
    ) {
        let Some(draft) = self.print_check.as_mut().filter(|d| d.tab == tab) else {
            return;
        };
        draft.report = Some((revision, body, report));
        self.publish_inspection();
    }

    pub(super) fn print_check_dialog(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.print_check.take() else {
            return;
        };
        let Some(tab) = self.tabs.iter().find(|t| t.id == draft.tab) else {
            return;
        };
        let document = tab.session.document().clone();
        let revision = tab.session.revision();
        let Some(part) = document.part(draft.part) else {
            return;
        };
        if !draft.current(revision)
            && !self.busy()
            && let Some(body) = draft.body
        {
            let (tab_id, analyzed) = (draft.tab, document.clone());
            self.start(context, None, move || {
                Ok(Outcome::PrintChecked {
                    tab: tab_id,
                    revision,
                    body,
                    report: print_check::inspect(&analyzed, body).map_err(|e| e.to_string()),
                })
            });
        }
        let settings = command_panel::settings(context);
        let mut go_to = None;
        let mut toggled = false;
        let outcome = command_panel::options(
            context,
            command_panel::Header {
                id: "print_check",
                icon: Icon::PrintCheck,
                title: text(T::PrintCheck),
            },
            self.canvas_rect,
            command_panel::WIDTH,
            |ui| {
                let name = draft
                    .body
                    .and_then(|id| part.bodies.iter().find(|b| b.id == id))
                    .map(|b| b.name.clone())
                    .unwrap_or_default();
                egui::ComboBox::from_label(text(T::BodyField))
                    .selected_text(name)
                    .show_ui(ui, |ui| {
                        for body in part.shown_bodies() {
                            ui.selectable_value(&mut draft.body, Some(body.id), &body.name);
                        }
                    });
                let report = draft
                    .report
                    .as_ref()
                    .filter(|(_, body, _)| Some(*body) == draft.body)
                    .map(|(_, _, report)| report);
                let report = match report {
                    None => {
                        command_panel::hint(ui, text(T::CheckingSolid));
                        return;
                    }
                    Some(Err(error)) => {
                        ui.colored_label(theme::warning(), error);
                        return;
                    }
                    Some(Ok(report)) => report,
                };
                let suffix = settings.unit.suffix();
                let [x, y, z] = report.size.map(|v| settings.format_compact_length(v));
                command_panel::row(ui, text(T::BedSize), |ui| {
                    ui.label(format!("{x} × {y} × {z} {suffix}"))
                });
                command_panel::row(ui, text(T::PrintContact), |ui| {
                    ui.label(format!("{:.1} mm²", report.contact_area).replace('.', ","))
                });
                command_panel::row(ui, text(T::PrintMass), |ui| {
                    ui.label(format!("{:.1} g", report.estimate.mass_g).replace('.', ","))
                })
                .on_hover_text(text(T::PrintEstimateHint));
                if let Some(cost) = report.estimate.cost {
                    command_panel::row(ui, text(T::PrintCost), |ui| {
                        ui.label(format!("{cost:.2}").replace('.', ","))
                    });
                }
                if !report.oriented {
                    command_panel::hint(ui, text(T::PrintNoFace));
                }
                if report.oversized {
                    ui.colored_label(theme::warning(), text(T::BedTooSmall));
                }
                if report.small_base {
                    ui.colored_label(theme::warning(), text(T::PrintSmallBase));
                }
                if let Some((count, [x, y, z])) = report.slivers {
                    let at = [x, y, z].map(|v| settings.format_compact_length(v));
                    command_panel::hint(
                        ui,
                        &format!(
                            "{count} {} X {}   Y {}   Z {}",
                            text(T::PrintSlivers),
                            at[0],
                            at[1],
                            at[2]
                        ),
                    );
                }
                if report.regions.is_empty() {
                    ui.colored_label(theme::green(), text(T::PrintCheckClean));
                }
                for issue in Issue::ALL {
                    let regions = report
                        .regions
                        .iter()
                        .filter(|r| r.issue == issue)
                        .collect::<Vec<_>>();
                    if regions.is_empty() {
                        continue;
                    }
                    let area = regions.iter().map(|r| r.area).sum::<f64>();
                    let mut shown = !draft.hidden.contains(&issue);
                    ui.horizontal(|ui| {
                        let [r, g, b] = print_check::color(issue).channels();
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(12., 12.), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, 2., egui::Color32::from_rgb(r, g, b));
                        let title = format!(
                            "{} · {} {} · {:.1} mm²",
                            text(label(issue)),
                            regions.len(),
                            text(T::PrintRegions),
                            area
                        );
                        if ui.checkbox(&mut shown, title.replace('.', ",")).changed() {
                            toggled = true;
                        }
                    });
                    if !shown {
                        if !draft.hidden.contains(&issue) {
                            draft.hidden.push(issue);
                        }
                        continue;
                    }
                    draft.hidden.retain(|hidden| *hidden != issue);
                    for region in regions.iter().take(LISTED) {
                        let measure = if issue == Issue::Overhang {
                            format!("{:.0}°", region.worst)
                        } else {
                            format!("{} {suffix}", settings.format_compact_length(region.worst))
                        };
                        let row = format!("{:.1} mm² · {measure}", region.area).replace('.', ",");
                        let selected = draft.focus == Some(region.center);
                        if ui.selectable_label(selected, row).clicked() {
                            go_to = Some(region.center);
                        }
                    }
                    if regions.len() > LISTED {
                        command_panel::hint(ui, &format!("… +{}", regions.len() - LISTED));
                    }
                }
                command_panel::hint(ui, text(T::PrintCheckHint));
            },
        );
        if let Some(point) = go_to {
            draft.focus = Some(point);
            if let (Some(bounds), Ok(placement)) = (
                self.current().scene_bounds,
                sketch_scene::part_placement(self.current(), draft.part),
            ) {
                self.viewport
                    .camera
                    .focus_world(placement.apply(point), bounds);
            }
        }
        self.print_check = Some(draft);
        if outcome.cancel {
            self.close_print_check();
        } else if toggled {
            self.publish_inspection();
        }
    }
}
