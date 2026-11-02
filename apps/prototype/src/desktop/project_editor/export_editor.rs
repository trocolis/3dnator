//! Exportação para impressão 3D: formato, escopo, qualidade e resumo antes de gravar.
//! O planejamento e a gravação ocorrem na thread de trabalho; nada é sobrescrito.
use super::*;
use nator_commands::export::{self, Quality, Scope};
use nator_project_io::{PrintExportPreferences, ThreeMfObject};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Format {
    ThreeMf,
    Stl,
}

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
    format: Format,
    /// Aplica a face na mesa e distribui os objetos; desligado, mantém as coordenadas.
    prepare: bool,
    /// Caixa do trecho, em coordenadas da peça; só para corpo ou peça.
    excerpt: Option<export::Excerpt>,
    preferences: PrintExportPreferences,
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
fn describe(files: &[export::File], settings: nator_document::Settings) -> String {
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
        lines.push(format!(
            "{} × {} × {} {}",
            settings.format_compact_length(s[0]),
            settings.format_compact_length(s[1]),
            settings.format_compact_length(s[2]),
            settings.unit.suffix()
        ));
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

/// Linhas de tamanho no resumo; projetos importados chegam a centenas de corpos.
const SIZE_LINES: usize = 8;

/// Tamanhos na mesa e o que não coube. Excessos não impedem a exportação.
fn describe_bed(plan: &export::PrintPlan, settings: nator_document::Settings) -> String {
    let layout = &plan.layout;
    let size = |name: &str, size: [f64; 3]| {
        let [x, y, z] = size.map(|v| settings.format_compact_length(v));
        format!("{name}: {x} × {y} × {z} {}", settings.unit.suffix())
    };
    let mut lines = match &plan.group {
        Some((name, group)) => vec![size(name, *group)],
        None => layout
            .files
            .iter()
            .take(SIZE_LINES)
            .map(|file| size(&file.name, file.size))
            .collect(),
    };
    if plan.group.is_none() && layout.files.len() > SIZE_LINES {
        lines.push(format!("… +{}", layout.files.len() - SIZE_LINES));
    }
    if !plan.skipped.is_empty() {
        lines.push(skipped_note(&plan.skipped));
    }
    if !layout.oversized.is_empty() {
        lines.push(format!(
            "{}: {}",
            text(T::ExportOversized),
            layout.oversized.join(", ")
        ));
    }
    if layout.overflow {
        lines.push(text(T::ExportOverflow).to_owned());
    }
    lines.join("\n")
}

fn skipped_note(skipped: &[String]) -> String {
    format!("{}: {}", text(T::ExportSkipped), skipped.join(", "))
}

fn preferences_path() -> Option<PathBuf> {
    nator_project_io::recovery_directory()
        .parent()
        .map(|p| p.join("print-export.json"))
}

/// Nome do arquivo único: o do objeto, ou o geral quando há vários.
fn single_name(layout: &[export::Placed], general: &str) -> String {
    match layout {
        [file] => file.name.clone(),
        _ => general.to_owned(),
    }
}

/// Abre os arquivos no fatiador, se o usuário pediu nas preferências de exportação.
pub(super) fn open_slicer_if_asked(paths: &[PathBuf]) -> Result<(), String> {
    let preferences = preferences_path()
        .map(|path| nator_project_io::read_print_export(&path))
        .unwrap_or_default();
    if preferences.open_slicer {
        slicer::open(&preferences.slicer_command, paths)
            .map_err(|e| format!("{}: {e}", text(T::ExportNotOpened)))?;
    }
    Ok(())
}

fn save_dialog(name: &str, filter: T, extension: &str) -> Option<PathBuf> {
    let mut path = rfd::FileDialog::new()
        .add_filter(text(filter), &[extension])
        .set_file_name(format!("{name}.{extension}"))
        .save_file()?;
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(extension))
    {
        path.set_extension(extension);
    }
    Some(path)
}

/// Grava o 3MF com um objeto por arquivo planejado. `None` significa cancelado.
pub(super) fn write_three_mf(
    layout: &[export::Placed],
    general: &str,
) -> Result<Option<Vec<PathBuf>>, String> {
    let Some(path) = save_dialog(&single_name(layout, general), T::ThreeMfFilter, "3mf") else {
        return Ok(None);
    };
    let objects = layout
        .iter()
        .map(|file| ThreeMfObject {
            name: &file.name,
            mesh: &file.mesh,
            items: &file.items,
        })
        .collect::<Vec<_>>();
    nator_project_io::write_3mf(&path, &objects).map_err(|e| e.to_string())?;
    Ok(Some(vec![path]))
}

/// Grava STL: um arquivo escolhido, ou uma pasta quando há vários. O conjunto da montagem
/// vai para um único arquivo, com um sólido por corpo. `None` significa cancelado.
fn write_stl(
    meshes: &[(String, nator_geometry::Mesh)],
    group: Option<&str>,
) -> Result<Option<Vec<PathBuf>>, String> {
    if let Some(name) = group {
        let Some(path) = save_dialog(name, T::StlFilter, "stl") else {
            return Ok(None);
        };
        let list = meshes.iter().map(|(_, mesh)| mesh).collect::<Vec<_>>();
        nator_project_io::write_stl_group(&path, &list).map_err(|e| e.to_string())?;
        return Ok(Some(vec![path]));
    }
    if let [(name, mesh)] = meshes {
        let Some(path) = save_dialog(name, T::StlFilter, "stl") else {
            return Ok(None);
        };
        nator_project_io::write_stl(&path, mesh).map_err(|e| e.to_string())?;
        return Ok(Some(vec![path]));
    }
    let Some(folder) = rfd::FileDialog::new().pick_folder() else {
        return Ok(None);
    };
    let list = meshes
        .iter()
        .map(|(name, mesh)| (name.clone(), mesh))
        .collect::<Vec<_>>();
    nator_project_io::write_stl_files(&folder, &list)
        .map(Some)
        .map_err(|e| e.to_string())
}

impl Editor {
    /// O menu da árvore exporta a definição clicada, em coordenadas da peça,
    /// independentemente da seleção anterior ou da posição de sua ocorrência.
    pub(super) fn open_export_target(&mut self, selection: Selection) {
        self.open_export();
        if let Some(draft) = &mut self.export_edit {
            match selection {
                Selection::Body { id, part } => {
                    draft.body = Some(id);
                    draft.part = Some(part);
                    draft.scope = 0;
                }
                Selection::Instance(id) => {
                    draft.body = None;
                    draft.part = self
                        .tabs
                        .iter()
                        .find(|t| t.id == draft.tab)
                        .and_then(|t| t.session.document().instance(id))
                        .map(|i| i.part);
                    draft.scope = 1;
                }
            }
        }
    }
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
            // Sem corpo escolhido, a peça em edição é o alvo natural de uma impressão.
            scope: if body.is_some() {
                0
            } else if part.is_some() {
                1
            } else {
                3
            },
            body,
            part,
            quality: 1,
            linear_mm: 0.02,
            angular_deg: 15.,
            summary: None,
            format: Format::ThreeMf,
            prepare: true,
            excerpt: None,
            preferences: preferences_path()
                .map(|path| nator_project_io::read_print_export(&path))
                .unwrap_or_default(),
        });
    }

    /// Arestas do trecho na montagem, para ver a caixa enquanto ela é ajustada.
    pub(super) fn excerpt_edges(&self) -> Vec<[Point3; 2]> {
        let Some(draft) = self
            .export_edit
            .as_ref()
            .filter(|d| d.tab == self.current().id)
        else {
            return Vec::new();
        };
        let (Some(excerpt), Some(part)) = (draft.excerpt, draft.part) else {
            return Vec::new();
        };
        let Ok(placement) = sketch_scene::part_placement(self.current(), part) else {
            return Vec::new();
        };
        let corner = |bits: u8| {
            placement.apply(std::array::from_fn(|axis| {
                if bits >> axis & 1 == 0 {
                    excerpt.min[axis]
                } else {
                    excerpt.max[axis]
                }
            }))
        };
        (0..8_u8)
            .flat_map(|a| [1_u8, 2, 4].map(|bit| (a, a | bit)))
            .filter(|(a, b)| a != b)
            .map(|(a, b)| [corner(a), corner(b)])
            .collect()
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
        let mut action = None;
        let title = text(T::ExportForPrint);
        let outcome = command_panel::options(
            ctx,
            command_panel::Header {
                id: "export",
                icon: icons::Icon::Export,
                title,
            },
            self.canvas_rect,
            command_panel::WIDTH,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(text(T::ExportFormat));
                    ui.radio_value(&mut draft.format, Format::ThreeMf, "3MF");
                    ui.radio_value(&mut draft.format, Format::Stl, "STL");
                });
                if ui
                    .checkbox(&mut draft.prepare, text(T::ExportPrepareBed))
                    .changed()
                {
                    draft.summary = None;
                }
                ui.checkbox(
                    &mut draft.preferences.open_slicer,
                    text(T::ExportOpenSlicer),
                );
                if draft.preferences.open_slicer {
                    ui.horizontal(|ui| {
                        ui.label(text(T::ExportSlicerCommand));
                        ui.text_edit_singleline(&mut draft.preferences.slicer_command);
                    });
                    ui.small(text(T::ExportSlicerHint));
                }
                egui::ComboBox::from_label(text(T::ExportScope))
                    .selected_text(text(SCOPES[draft.scope]))
                    .show_ui(ui, |ui| {
                        for (i, label) in SCOPES.iter().enumerate() {
                            let available = match i {
                                0 => draft.body.is_some(),
                                1 => draft.part.is_some(),
                                _ => true,
                            };
                            ui.add_enabled_ui(available, |ui| {
                                if ui
                                    .selectable_value(&mut draft.scope, i, text(*label))
                                    .changed()
                                {
                                    draft.summary = None;
                                }
                            });
                        }
                    });
                let boxable = draft.scope < 2;
                if !boxable {
                    draft.excerpt = None;
                }
                ui.add_enabled_ui(boxable, |ui| {
                    let mut chosen = draft.excerpt.is_some();
                    if ui.checkbox(&mut chosen, text(T::ExcerptToggle)).changed() {
                        draft.excerpt = chosen
                            .then(|| {
                                draft
                                    .scope()
                                    .and_then(|s| export::scope_bounds(&document, s))
                            })
                            .flatten();
                        draft.summary = None;
                    }
                });
                if let Some(excerpt) = &mut draft.excerpt {
                    let before = *excerpt;
                    for (axis, label) in ["X", "Y", "Z"].into_iter().enumerate() {
                        command_panel::row(ui, label, |ui| {
                            ui.add(command_panel::localized(
                                egui::DragValue::new(&mut excerpt.min[axis])
                                    .speed(0.5)
                                    .max_decimals(2),
                            ));
                            ui.add(command_panel::localized(
                                egui::DragValue::new(&mut excerpt.max[axis])
                                    .speed(0.5)
                                    .max_decimals(2),
                            ));
                        });
                    }
                    if *excerpt != before {
                        draft.summary = None;
                    }
                    command_panel::hint(ui, text(T::ExcerptHint));
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
                    #[cfg(feature = "occt")]
                    if !solid_body {
                        ui.small(text(T::MeshIsNotSolid));
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(text(T::ComputeSummary)).clicked() {
                            action = Some(false);
                        }
                        if ui.button(text(T::ExportFiles)).clicked() {
                            action = Some(true);
                        }
                        if solid_body
                            && ui.button(text(T::ExportStep)).clicked()
                            && let Some(body) = draft.body
                        {
                            self.export_step(ctx, draft.tab, export::StepScope::Body(body));
                        }
                    });
                    ui.horizontal(|ui| {
                        if let Some(part) = draft.part
                            && ui.button(text(T::ExportPartStep)).clicked()
                        {
                            self.export_step(ctx, draft.tab, export::StepScope::Part(part));
                        }
                        if ui.button(text(T::ExportAssemblyStep)).clicked() {
                            self.export_step(ctx, draft.tab, export::StepScope::Assembly);
                        }
                    });
                });
            },
        );
        let open = !outcome.cancel;
        if let (Some(write), Some(scope)) = (action, draft.scope()) {
            let quality = draft.quality();
            let tab = draft.tab;
            let (format, prepare, excerpt) = (draft.format, draft.prepare, draft.excerpt);
            let preferences = draft.preferences.clone();
            draft.summary = None;
            self.start(ctx, None, move || {
                let bed = document.print_profile().bed_mm;
                let plan = export::print_plan_of(&document, scope, quality, prepare, bed, excerpt)
                    .map_err(|e| e.to_string())?;
                if !write {
                    let mut files = describe(&plan.files, document.settings());
                    if prepare || !plan.skipped.is_empty() {
                        files = format!("{files}\n{}", describe_bed(&plan, document.settings()));
                    }
                    return Ok(Outcome::ExportSummary {
                        tab,
                        revision,
                        text: files,
                    });
                }
                // Antes de abrir o seletor e nas coordenadas da modelagem: o erro cita o corpo
                // e a posição que o usuário encontra na área 3D.
                for file in &plan.files {
                    nator_project_io::require_printable(&file.name, &file.mesh)
                        .map_err(|e| e.to_string())?;
                }
                let written = match format {
                    Format::ThreeMf => write_three_mf(&plan.layout.files, document.name())?,
                    Format::Stl => {
                        let meshes = plan
                            .layout
                            .files
                            .iter()
                            .map(|file| Ok((file.name.clone(), file.positioned()?)))
                            .collect::<Result<Vec<_>, nator_document::DocumentError>>()
                            .map_err(|e| e.to_string())?;
                        write_stl(&meshes, plan.group.as_ref().map(|(name, _)| name.as_str()))?
                    }
                };
                let Some(paths) = written else {
                    return Ok(Outcome::Cancelled);
                };
                // A escolha vale a partir de uma exportação concluída; falhar ao guardá-la
                // não invalida o arquivo já gravado.
                if let Some(path) = preferences_path() {
                    let _ = nator_project_io::write_print_export(&path, &preferences);
                }
                if preferences.open_slicer {
                    slicer::open(&preferences.slicer_command, &paths)
                        .map_err(|e| format!("{}: {e}", text(T::ExportNotOpened)))?;
                }
                Ok(if plan.skipped.is_empty() {
                    Outcome::Exported
                } else {
                    Outcome::ExportedWith(format!(
                        "{}. {}",
                        text(T::Exported),
                        skipped_note(&plan.skipped)
                    ))
                })
            });
        }
        if open {
            self.export_edit = Some(draft);
        }
    }
}
