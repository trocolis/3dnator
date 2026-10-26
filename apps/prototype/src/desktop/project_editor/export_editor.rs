//! Exportação STL para impressão 3D: escopo, qualidade e resumo antes de gravar.
//! O planejamento e a gravação ocorrem na thread de trabalho; nada é sobrescrito.
use super::*;
use nator_commands::export::{self, Quality, Scope};

/// Escopos na ordem exibida.
const SCOPES: [T; 5] = [
    T::ScopeBody,
    T::ScopePart,
    T::ScopeDefinitions,
    T::ScopeJoined,
    T::ScopeOccurrences,
];
const QUALITIES: [T; 4] = [
    T::QualityDraft,
    T::QualityStandard,
    T::QualityHigh,
    T::QualityCustom,
];

pub(super) struct Draft {
    tab: u64,
    scope: usize,
    body: Option<BodyId>,
    part: Option<PartId>,
    quality: usize,
    linear_mm: f64,
    angular_deg: f64,
    /// Resumo calculado para a revisão indicada.
    summary: Option<(u64, String)>,
}
impl Draft {
    fn scope(&self) -> Option<Scope> {
        match self.scope {
            0 => self.body.map(Scope::Body),
            1 => self.part.map(Scope::Part),
            2 => Some(Scope::Definitions),
            3 => Some(Scope::Joined),
            _ => Some(Scope::AllOccurrences),
        }
    }
    fn quality(&self) -> Quality {
        match self.quality {
            0 => Quality::Draft,
            1 => Quality::Standard,
            2 => Quality::High,
            _ => Quality::Custom {
                linear_mm: self.linear_mm,
                angular_rad: self.angular_deg.to_radians(),
            },
        }
    }
    pub(super) fn set_summary(&mut self, tab: u64, revision: u64, text: String) {
        if tab == self.tab {
            self.summary = Some((revision, text));
        }
    }
}

/// Texto do resumo: arquivos, triângulos, tamanho, dimensões e ocorrências.
fn describe(files: &[export::File]) -> String {
    use nator_drawing::format_mm as mm;
    let summary = export::summarize(files);
    let size = if summary.bytes >= 1 << 20 {
        format!("{} MiB", mm(summary.bytes as f64 / f64::from(1 << 20)))
    } else {
        format!("{} KiB", mm(summary.bytes as f64 / 1024.))
    };
    let mut lines = vec![format!(
        "{} {} · {} {} · {size}",
        summary.files,
        text(T::FilesLabel),
        summary.triangles,
        text(T::Triangles)
    )];
    if let Some(b) = summary.bounds {
        let s = b.size();
        lines.push(format!("{} × {} × {} mm", mm(s[0]), mm(s[1]), mm(s[2])));
    }
    for file in files.iter().filter(|f| f.occurrences > 1) {
        lines.push(format!(
            "{}: {} {}",
            file.name,
            file.occurrences,
            text(T::Occurrences)
        ));
    }
    lines.join("\n")
}

impl Editor {
    pub(super) fn open_export(&mut self) {
        let (body, part) = match self.current().selection {
            Some(Selection::Body { id, part }) => (Some(id), Some(part)),
            Some(Selection::Instance(id)) => (
                None,
                self.current()
                    .session
                    .document()
                    .instance(id)
                    .map(|i| i.part),
            ),
            _ => (None, self.current().active_part),
        };
        self.export_edit = Some(Draft {
            tab: self.current().id,
            scope: if body.is_some() { 0 } else { 3 },
            body,
            part,
            quality: 1,
            linear_mm: 0.02,
            angular_deg: 15.,
            summary: None,
        });
    }

    pub(super) fn export_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.export_edit.take() else {
            return;
        };
        let Some(tab) = self.tabs.iter().find(|t| t.id == draft.tab) else {
            return;
        };
        let document = tab.session.document().clone();
        let revision = tab.session.revision();
        let solid_body = draft
            .body
            .and_then(|b| document.body(b))
            .is_some_and(|b| b.brep.is_some());
        let mut open = true;
        let mut action = None;
        egui::Window::new(text(T::ExportStl))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(text(T::ExportScope));
                for (i, label) in SCOPES.iter().enumerate() {
                    let available = match i {
                        0 => draft.body.is_some(),
                        1 => draft.part.is_some(),
                        _ => true,
                    };
                    ui.add_enabled_ui(available, |ui| {
                        if ui.radio_value(&mut draft.scope, i, text(*label)).changed() {
                            draft.summary = None;
                        }
                    });
                }
                egui::ComboBox::from_label(text(T::ExportQuality))
                    .selected_text(text(QUALITIES[draft.quality]))
                    .show_ui(ui, |ui| {
                        for (i, label) in QUALITIES.iter().enumerate() {
                            if ui
                                .selectable_value(&mut draft.quality, i, text(*label))
                                .changed()
                            {
                                draft.summary = None;
                            }
                        }
                    });
                if draft.quality == 3 {
                    ui.horizontal(|ui| {
                        ui.label(text(T::LinearTolerance));
                        ui.add(
                            egui::DragValue::new(&mut draft.linear_mm)
                                .range(0.001..=5.)
                                .speed(0.001),
                        );
                        ui.label(text(T::AngularTolerance));
                        ui.add(
                            egui::DragValue::new(&mut draft.angular_deg)
                                .range(1.0..=60.)
                                .speed(0.5),
                        );
                    });
                }
                match &draft.summary {
                    Some((r, summary)) if *r == revision => {
                        ui.monospace(summary);
                    }
                    _ => {
                        ui.small(text(T::ComputeSummaryHint));
                    }
                }
                ui.add_enabled_ui(!self.busy(), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(text(T::ComputeSummary)).clicked() {
                            action = Some(false);
                        }
                        if ui.button(text(T::ExportFiles)).clicked() {
                            action = Some(true);
                        }
                        if solid_body && ui.button(text(T::ExportStep)).clicked() {
                            self.export_step(ctx);
                        }
                    });
                });
            });
        if let (Some(write), Some(scope)) = (action, draft.scope()) {
            let quality = draft.quality();
            let tab = draft.tab;
            draft.summary = None;
            self.start(ctx, None, move || {
                let files = export::plan(&document, scope, quality).map_err(|e| e.to_string())?;
                if !write {
                    return Ok(Outcome::ExportSummary {
                        tab,
                        revision,
                        text: describe(&files),
                    });
                }
                if let [file] = files.as_slice() {
                    let Some(mut path) = rfd::FileDialog::new()
                        .add_filter(text(T::StlFilter), &["stl"])
                        .set_file_name(format!("{}.stl", file.name))
                        .save_file()
                    else {
                        return Ok(Outcome::Cancelled);
                    };
                    if !path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("stl"))
                    {
                        path.set_extension("stl");
                    }
                    nator_project_io::write_stl(&path, &file.mesh).map_err(|e| e.to_string())?;
                } else {
                    let Some(folder) = rfd::FileDialog::new().pick_folder() else {
                        return Ok(Outcome::Cancelled);
                    };
                    let list = files
                        .iter()
                        .map(|f| (f.name.clone(), &f.mesh))
                        .collect::<Vec<_>>();
                    nator_project_io::write_stl_files(&folder, &list).map_err(|e| e.to_string())?;
                }
                Ok(Outcome::Exported)
            });
        }
        if open {
            self.export_edit = Some(draft);
        }
    }
}
