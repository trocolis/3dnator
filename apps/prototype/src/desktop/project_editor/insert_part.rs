//! Inserção de peças de outro projeto aberto: a definição escolhida (com suas subpeças)
//! é copiada para o projeto ativo como peças novas e independentes, sem vínculo com o
//! arquivo de origem. Na construção, a cópia entra como subpeça da peça ativa.
use super::{
    command_panel::{self as panel, Footer},
    icons::Icon,
    *,
};

pub(super) struct InsertDraft {
    tab: u64,
    source: Option<u64>,
    part: Option<PartId>,
}

impl InsertDraft {
    /// Origem e peça escolhidas, para capturas de ensaio.
    pub(super) fn choose_for_capture(&mut self, source: u64, part: PartId) {
        self.source = Some(source);
        self.part = Some(part);
    }
}

impl Editor {
    pub(super) fn open_insert_part(&mut self) {
        let tab = self.current().id;
        let source = self.tabs.iter().map(|t| t.id).find(|id| *id != tab);
        self.feature_edit = None;
        self.assembly_panel = None;
        self.insert_part = Some(InsertDraft {
            tab,
            source,
            part: None,
        });
    }

    pub(super) fn insert_part_panel(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.insert_part.take() else {
            return;
        };
        if draft.tab != self.current().id {
            return;
        }
        let others = self
            .tabs
            .iter()
            .filter(|t| t.id != draft.tab)
            .map(|t| {
                let name = t
                    .path
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| t.session.document().name().into());
                (t.id, name)
            })
            .collect::<Vec<_>>();
        let source = draft
            .source
            .and_then(|id| self.tabs.iter().find(|t| t.id == id))
            .map(|t| t.session.document().clone());
        let destination = self
            .current()
            .active_part
            .and_then(|p| self.current().session.document().part(p))
            .map(|p| p.name.clone());
        let outcome = panel::show(
            context,
            panel::Header {
                id: "insert_part",
                icon: Icon::Import,
                title: text(T::InsertFromProject),
            },
            self.canvas_rect,
            Footer::None,
            draft.part.is_some() && source.is_some() && !self.busy(),
            |ui| {
                if others.is_empty() {
                    ui.colored_label(theme::WARNING, text(T::NoOtherProjects));
                    return;
                }
                panel::row(ui, text(T::SourceProject), |ui| {
                    let selected = others
                        .iter()
                        .find(|(id, _)| Some(*id) == draft.source)
                        .map(|(_, name)| name.as_str())
                        .unwrap_or("—");
                    egui::ComboBox::from_id_salt("insert_source")
                        .width(170.)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (id, name) in &others {
                                if ui
                                    .selectable_label(draft.source == Some(*id), name)
                                    .clicked()
                                {
                                    draft.source = Some(*id);
                                    draft.part = None;
                                }
                            }
                        });
                });
                if let Some(document) = &source {
                    ui.label(
                        egui::RichText::new(text(T::PartsField))
                            .small()
                            .color(theme::MUTED),
                    );
                    egui::ScrollArea::vertical()
                        .max_height(220.)
                        .show(ui, |ui| {
                            // Definições de topo primeiro: são as que aparecem na montagem.
                            let mut parts = document.parts().iter().collect::<Vec<_>>();
                            parts.sort_by_key(|p| {
                                !document
                                    .instances()
                                    .iter()
                                    .any(|i| i.part == p.id && i.parent.is_none())
                            });
                            for part in parts {
                                let children = document
                                    .instances()
                                    .iter()
                                    .filter(|i| i.parent == Some(part.id))
                                    .count();
                                let label = if children > 0 {
                                    format!("{} ({children} {})", part.name, text(T::SubpartsCount))
                                } else {
                                    part.name.clone()
                                };
                                if icons::entry(ui, Icon::Cube, &label, draft.part == Some(part.id))
                                    .clicked()
                                {
                                    draft.part = Some(part.id);
                                }
                            }
                        });
                }
                ui.label(
                    egui::RichText::new(match &destination {
                        Some(name) => format!("{} {name}.", text(T::InsertIntoPart)),
                        None => text(T::InsertAtRoot).to_owned(),
                    })
                    .small()
                    .color(theme::MUTED),
                );
            },
        );
        if outcome.cancel {
            return;
        }
        if outcome.ok
            && let (Some(source), Some(part)) = (source, draft.part)
        {
            let command = Command::InsertPart {
                source: Arc::new(source),
                part,
                parent: self.current().active_part,
                placement: Placement::default(),
            };
            self.prepare_command(context, draft.tab, command);
            return;
        }
        self.insert_part = Some(draft);
    }
}
