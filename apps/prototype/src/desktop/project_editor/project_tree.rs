//! Árvore do projeto: Projeto → Peças, Rascunhos (de referência) e Origem; cada peça →
//! Corpos, Subpeças, Rascunhos, Planos e Origem (oculta por padrão). Descendentes têm visibilidade
//! própria, gravada no documento e com desfazer; a efetiva combina a dos nós acima, e um
//! nó oculto por um ancestral aparece esmaecido. O lápis de uma peça entra na edição dela.
//! Pastas vazias dentro das peças não aparecem.
//!
//! Desenhar a árvore só colhe intenções; o editor as aplica depois (`Editor::tree`).
use super::{
    browser,
    icons::{self, Icon},
    *,
};
use nator_document::{Folder, Instance, SheetRecord, WorkPlaneId, WorkPlaneRecord};

/// Intenção escolhida na árvore neste quadro.
pub(super) enum Intent {
    Select(Selection),
    Command(Box<Command>),
    Menu(browser::MenuAction),
    /// Entra na edição da peça e abre o nó dela.
    EditPart(PartId, egui::Id),
    LeavePart,
    EditSketch(PartId, SketchId),
    /// Plano de trabalho clicado: vale como escolha quando um seletor de suporte está aberto.
    PickWorkPlane(WorkPlaneId),
    /// Reabre o plano de trabalho no painel do plano deslocado.
    EditWorkPlane(WorkPlaneId),
    EditReference(SketchId),
    ToggleSection,
    OpenSection,
    RemoveSection,
    OpenDrawing(Box<SheetRecord>),
}

pub(super) struct Tree<'a> {
    document: &'a Document,
    tab: &'a Tab,
    menu: &'a browser::MenuContext,
    busy: bool,
    /// Painel de corte em edição (realça a análise na árvore).
    section_open: bool,
    pub intents: Vec<Intent>,
}

impl<'a> Tree<'a> {
    pub fn new(
        document: &'a Document,
        tab: &'a Tab,
        menu: &'a browser::MenuContext,
        busy: bool,
        section_open: bool,
    ) -> Self {
        Self {
            document,
            tab,
            menu,
            busy,
            section_open,
            intents: Vec::new(),
        }
    }

    fn id(&self, key: impl std::hash::Hash + std::fmt::Debug) -> egui::Id {
        egui::Id::new(("project_tree", self.tab.id, key))
    }

    /// Olho da visibilidade própria do nó.
    fn eye(&self, ui: &mut egui::Ui, visible: bool, enabled: bool) -> bool {
        ui.add_enabled_ui(enabled && !self.busy, |ui| {
            icons::tree_button(
                ui,
                if visible { Icon::Eye } else { Icon::EyeOff },
                text(T::Visibility),
                false,
            )
        })
        .inner
        .clicked()
    }

    /// Nó recolhível: a seta e o clique no rótulo (quando `header` devolve verdadeiro)
    /// abrem e fecham o corpo.
    fn node(
        &mut self,
        ui: &mut egui::Ui,
        id: egui::Id,
        default_open: bool,
        header: impl FnOnce(&mut Self, &mut egui::Ui) -> bool,
        body: impl FnOnce(&mut Self, &mut egui::Ui),
    ) {
        let state = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            id,
            default_open,
        );
        let mut toggle = false;
        let mut response = state.show_header(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.;
            toggle = header(self, ui);
        });
        if toggle {
            response.toggle();
        }
        response.body(|ui| body(self, ui));
    }

    /// Linha sem filhos, alinhada às que têm seta.
    fn leaf<R>(&mut self, ui: &mut egui::Ui, add: impl FnOnce(&mut Self, &mut egui::Ui) -> R) -> R {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.;
            ui.add_space(ui.spacing().indent);
            add(self, ui)
        })
        .inner
    }

    /// Pasta com visibilidade própria; o corpo recebe a visibilidade efetiva. Vazia, vira
    /// uma linha sem seta.
    #[allow(clippy::too_many_arguments)]
    fn folder(
        &mut self,
        ui: &mut egui::Ui,
        id: egui::Id,
        folder: Folder,
        icon: Icon,
        label: String,
        parent_shown: bool,
        empty: bool,
        body: impl FnOnce(&mut Self, &mut egui::Ui, bool),
    ) {
        let visible = self.document.folder_visible(folder).unwrap_or(true);
        let shown = parent_shown && visible;
        if empty {
            self.leaf(ui, |tree, ui| {
                if tree.eye(ui, visible, true) {
                    tree.intents
                        .push(Intent::Command(Box::new(Command::SetFolderVisibility {
                            folder,
                            visible: !visible,
                        })));
                }
                icons::entry_styled(ui, icon, &label, false, !shown);
            });
            return;
        }
        self.node(
            ui,
            id,
            true,
            |tree, ui| {
                if tree.eye(ui, visible, true) {
                    tree.intents
                        .push(Intent::Command(Box::new(Command::SetFolderVisibility {
                            folder,
                            visible: !visible,
                        })));
                }
                icons::entry_styled(ui, icon, &label, false, !shown).clicked()
            },
            |tree, ui| body(tree, ui, shown),
        );
    }

    fn origin(&mut self, ui: &mut egui::Ui, folder: Folder, parent_shown: bool) {
        let visible = self.document.folder_visible(folder).unwrap_or(false);
        self.leaf(ui, |tree, ui| {
            if tree.eye(ui, visible, true) {
                tree.intents
                    .push(Intent::Command(Box::new(Command::SetFolderVisibility {
                        folder,
                        visible: !visible,
                    })));
            }
            icons::entry_styled(
                ui,
                Icon::Origin,
                text(T::Origin),
                false,
                !(parent_shown && visible),
            )
        });
    }

    /// Raiz Projeto, seguida de grupos, análises e folhas técnicas.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Recuo curto: a hierarquia tem até quatro níveis antes dos elementos.
        ui.spacing_mut().indent = 14.;
        ui.spacing_mut().item_spacing.y = 0.;
        ui.spacing_mut().interact_size.y = 20.;
        let id = self.id("project");
        self.node(
            ui,
            id,
            true,
            |tree, ui| {
                // O cubo é a marca do aplicativo; a raiz não tem olho nem ação
                // de ocultação, permanecendo acessível em qualquer projeto.
                icons::entry_styled(ui, Icon::Cube, text(T::ProjectNode), false, false)
                    .on_hover_text(tree.document.name())
                    .clicked()
            },
            |tree, ui| tree.project_body(ui, true),
        );
        self.groups(ui);
        self.analyses(ui);
        self.sheets(ui);
    }

    fn project_body(&mut self, ui: &mut egui::Ui, shown: bool) {
        let document = self.document;
        let roots = document
            .instances()
            .iter()
            .filter(|i| i.parent.is_none())
            .collect::<Vec<_>>();
        self.folder(
            ui,
            self.id("parts"),
            Folder::Parts,
            Icon::Folder,
            format!("{} ({})", text(T::PartsFolder), roots.len()),
            shown,
            false,
            |tree, ui, shown| {
                if roots.is_empty() {
                    tree.leaf(ui, |_, ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(text(T::BrowserEmpty)).strong());
                            ui.label(
                                egui::RichText::new(text(T::StartSketch))
                                    .small()
                                    .color(theme::MUTED),
                            );
                        });
                    });
                }
                for instance in roots {
                    tree.part(ui, instance, vec![instance.id.0], shown, 0);
                }
            },
        );
        let references = document.reference_sketches();
        self.folder(
            ui,
            self.id("references"),
            Folder::ReferenceSketches,
            Icon::Folder,
            format!("{} ({})", text(T::SketchesGroup), references.len()),
            shown,
            references.is_empty(),
            |tree, ui, shown| {
                for record in references {
                    tree.reference_sketch(ui, record, shown);
                }
            },
        );
        self.origin(ui, Folder::Origin, shown);
    }

    fn reference_sketch(
        &mut self,
        ui: &mut egui::Ui,
        record: &nator_document::SketchRecord,
        parent_shown: bool,
    ) {
        let editing = self
            .tab
            .sketch_edit
            .as_ref()
            .is_some_and(|w| w.id == record.id);
        self.leaf(ui, |tree, ui| {
            if tree.eye(ui, record.visible, !editing) {
                tree.intents
                    .push(Intent::Command(Box::new(Command::SetSketchVisibility {
                        id: record.id,
                        visible: !record.visible,
                    })));
            }
            let response = icons::entry_styled(
                ui,
                Icon::ReferenceSketch,
                &record.sketch.name,
                editing,
                !(parent_shown && record.visible),
            )
            .on_hover_text(text(T::ReferenceSketchHint));
            if response.clicked() && !tree.busy {
                tree.intents.push(Intent::EditReference(record.id));
            }
            response.context_menu(|ui| {
                ui.add_enabled_ui(!tree.busy, |ui| {
                    if ui.button(text(T::EditFeatureShort)).clicked() {
                        tree.intents.push(Intent::EditReference(record.id));
                        ui.close();
                    }
                    if ui.button(text(T::DeleteFeature)).clicked() {
                        tree.intents
                            .push(Intent::Command(Box::new(Command::RemoveSketch {
                                id: record.id,
                            })));
                        ui.close();
                    }
                });
            });
        });
    }

    /// Nó de uma peça (instância), com as pastas da definição. `path` distingue as
    /// ocorrências de uma subpeça quando o pai aparece mais de uma vez.
    fn part(
        &mut self,
        ui: &mut egui::Ui,
        instance: &'a Instance,
        path: Vec<u64>,
        parent_shown: bool,
        depth: usize,
    ) {
        let document = self.document;
        let Some(part) = document.part(instance.part) else {
            return;
        };
        if depth > 64 {
            return;
        }
        let shown = parent_shown && instance.visible;
        let active = self.tab.active_part == Some(part.id);
        let id = self.id(("part", &path));
        self.node(
            ui,
            id,
            active,
            |tree, ui| {
                if tree.eye(ui, instance.visible, true) {
                    tree.intents
                        .push(Intent::Command(Box::new(Command::SetInstanceVisibility {
                            id: instance.id,
                            visible: !instance.visible,
                        })));
                }
                let edit = ui
                    .add_enabled_ui(!tree.busy, |ui| {
                        icons::tree_button(
                            ui,
                            Icon::Edit,
                            text(if active { T::StopEditPart } else { T::EditPart }),
                            active,
                        )
                    })
                    .inner;
                if edit.clicked() {
                    tree.intents.push(if active {
                        Intent::LeavePart
                    } else {
                        Intent::EditPart(part.id, id)
                    });
                }
                let response = icons::entry_styled(
                    ui,
                    Icon::Part,
                    &instance.name,
                    tree.tab.is_selected(Selection::Instance(instance.id)),
                    !shown,
                )
                .on_hover_text(&part.name);
                if response.clicked() {
                    tree.intents
                        .push(Intent::Select(Selection::Instance(instance.id)));
                }
                if response.double_clicked() && !tree.busy {
                    tree.intents.push(Intent::EditPart(part.id, id));
                }
                response.context_menu(|ui| {
                    ui.add_enabled_ui(!tree.busy, |ui| {
                        if let Some(action) =
                            browser::instance_menu(ui, tree.menu, instance, &part.name)
                        {
                            tree.intents.push(Intent::Menu(action));
                        }
                    });
                });
                false
            },
            |tree, ui| {
                // Resultados intermediários pertencem à linha do tempo. Listá-los aqui
                // fazia cada modificação parecer uma cópia nova do corpo anterior.
                let bodies = part.listed_bodies().collect::<Vec<_>>();
                if !bodies.is_empty() {
                    tree.folder(
                        ui,
                        tree.id(("bodies", &path)),
                        Folder::Bodies(part.id),
                        Icon::Folder,
                        format!("{} ({})", text(T::SolidBodies), bodies.len()),
                        shown,
                        false,
                        |tree, ui, shown| {
                            for body in bodies {
                                tree.body(ui, part.id, body, shown);
                            }
                        },
                    );
                }
                let children = document
                    .instances()
                    .iter()
                    .filter(|i| i.parent == Some(part.id))
                    .collect::<Vec<_>>();
                if !children.is_empty() {
                    tree.folder(
                        ui,
                        tree.id(("subparts", &path)),
                        Folder::Subparts(part.id),
                        Icon::Folder,
                        format!("{} ({})", text(T::SubpartsFolder), children.len()),
                        shown,
                        false,
                        |tree, ui, shown| {
                            for child in children {
                                let mut nested = path.clone();
                                nested.push(child.id.0);
                                tree.part(ui, child, nested, shown, depth + 1);
                            }
                        },
                    );
                }
                if !part.sketches.is_empty() {
                    tree.folder(
                        ui,
                        tree.id(("sketches", &path)),
                        Folder::Sketches(part.id),
                        Icon::Folder,
                        format!("{} ({})", text(T::SketchesGroup), part.sketches.len()),
                        shown,
                        false,
                        |tree, ui, shown| {
                            for record in &part.sketches {
                                tree.sketch(ui, part.id, record, shown);
                            }
                        },
                    );
                }
                if !part.work_planes.is_empty() {
                    tree.folder(
                        ui,
                        tree.id(("planes", &path)),
                        Folder::WorkPlanes(part.id),
                        Icon::Folder,
                        format!("{} ({})", text(T::WorkPlanesFolder), part.work_planes.len()),
                        shown,
                        false,
                        |tree, ui, shown| {
                            for record in &part.work_planes {
                                tree.work_plane(ui, record, shown);
                            }
                        },
                    );
                }
                tree.origin(ui, Folder::PartOrigin(part.id), shown);
            },
        );
    }

    /// Plano de trabalho: o olho alterna a visibilidade; o duplo clique e o menu editam a
    /// distância, e o menu também cria um rascunho nele ou o exclui. Usado num rascunho, o
    /// plano fica oculto, mas continua listado aqui.
    fn work_plane(&mut self, ui: &mut egui::Ui, record: &WorkPlaneRecord, parent_shown: bool) {
        self.leaf(ui, |tree, ui| {
            if tree.eye(ui, record.visible, true) {
                tree.intents
                    .push(Intent::Command(Box::new(Command::SetWorkPlaneVisibility {
                        id: record.id,
                        visible: !record.visible,
                    })));
            }
            let response = icons::entry_styled(
                ui,
                Icon::WorkPlane,
                &record.name,
                false,
                !(parent_shown && record.visible),
            )
            .on_hover_text(format!(
                "{}: {} mm",
                text(T::OffsetField),
                nator_drawing::format_mm(record.plane.offset)
            ));
            let editable = !tree.busy && tree.tab.sketch_edit.is_none();
            if response.clicked() && !tree.busy {
                tree.intents.push(Intent::PickWorkPlane(record.id));
            }
            if response.double_clicked() && editable {
                tree.intents.push(Intent::EditWorkPlane(record.id));
            }
            response.context_menu(|ui| {
                ui.add_enabled_ui(!tree.busy, |ui| {
                    if ui
                        .add_enabled(editable, egui::Button::new(text(T::EditFeatureShort)))
                        .clicked()
                    {
                        tree.intents.push(Intent::EditWorkPlane(record.id));
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            tree.tab.sketch_edit.is_none(),
                            egui::Button::new(text(T::NewSketchOnPlane)),
                        )
                        .clicked()
                    {
                        tree.intents.push(Intent::Command(Box::new(
                            Command::AddSketchOnWorkPlane {
                                id: record.id,
                                name: text(T::Sketch).into(),
                            },
                        )));
                        ui.close();
                    }
                    if ui.button(text(T::DeleteFeature)).clicked() {
                        tree.intents
                            .push(Intent::Command(Box::new(Command::RemoveWorkPlane {
                                id: record.id,
                            })));
                        ui.close();
                    }
                });
            });
        });
    }

    fn body(
        &mut self,
        ui: &mut egui::Ui,
        part: PartId,
        body: &nator_document::Body,
        parent_shown: bool,
    ) {
        self.leaf(ui, |tree, ui| {
            if tree.eye(ui, body.visible, true) {
                tree.intents
                    .push(Intent::Command(Box::new(Command::SetBodyVisibility {
                        id: body.id,
                        visible: !body.visible,
                    })));
            }
            let target = Selection::Body { part, id: body.id };
            let response = icons::entry_styled(
                ui,
                Icon::Cube,
                &body.name,
                tree.tab.is_selected(target),
                !(parent_shown && body.visible),
            );
            if response.clicked() {
                tree.intents.push(Intent::Select(target));
            }
            response.context_menu(|ui| {
                ui.add_enabled_ui(!tree.busy, |ui| {
                    if let Some(action) = browser::body_menu(ui, tree.menu, part, body) {
                        tree.intents.push(Intent::Menu(action));
                    }
                });
            });
        });
    }

    fn sketch(
        &mut self,
        ui: &mut egui::Ui,
        part: PartId,
        record: &nator_document::SketchRecord,
        parent_shown: bool,
    ) {
        let editing = self
            .tab
            .sketch_edit
            .as_ref()
            .is_some_and(|w| w.id == record.id);
        self.leaf(ui, |tree, ui| {
            if tree.eye(ui, record.visible, !editing) {
                tree.intents
                    .push(Intent::Command(Box::new(Command::SetSketchVisibility {
                        id: record.id,
                        visible: !record.visible,
                    })));
            }
            if icons::entry_styled(
                ui,
                Icon::Sketch,
                &record.sketch.name,
                editing,
                !(parent_shown && record.visible),
            )
            .clicked()
                && !tree.busy
            {
                tree.intents.push(Intent::EditSketch(part, record.id));
            }
        });
    }

    /// Grupos organizacionais: o clique seleciona os membros.
    fn groups(&mut self, ui: &mut egui::Ui) {
        let groups = self.document.groups();
        if groups.is_empty() {
            return;
        }
        ui.separator();
        theme::section(ui, &format!("{} ({})", text(T::Groups), groups.len()));
        for group in groups {
            let members = group
                .members
                .iter()
                .map(|id| Selection::Instance(*id))
                .collect::<Vec<_>>();
            let chosen = !members.is_empty() && members.iter().all(|m| self.tab.is_selected(*m));
            let response = icons::entry(ui, Icon::Group, &group.name, chosen).on_hover_text(
                format!("{} {}", group.members.len(), text(T::SelectedCount)),
            );
            if response.clicked() {
                self.intents
                    .push(Intent::Menu(browser::MenuAction::Select(members)));
            }
            response.context_menu(|ui| {
                ui.add_enabled_ui(!self.busy, |ui| {
                    if let Some(action) = browser::group_menu(ui, group) {
                        self.intents.push(Intent::Menu(action));
                    }
                });
            });
        }
    }

    /// Análises da aba: corte de seção (controle visual, fora do documento).
    fn analyses(&mut self, ui: &mut egui::Ui) {
        let Some(cut) = self.tab.section else {
            return;
        };
        ui.separator();
        theme::section(ui, text(T::AnalysesGroup));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.;
            if icons::tree_button(
                ui,
                if cut.visible { Icon::Eye } else { Icon::EyeOff },
                text(T::Visibility),
                false,
            )
            .clicked()
            {
                self.intents.push(Intent::ToggleSection);
            }
            let response = icons::entry(
                ui,
                Icon::Section,
                text(T::SectionAnalysis),
                self.section_open,
            );
            if response.double_clicked() {
                self.intents.push(Intent::OpenSection);
            }
            response.context_menu(|ui| {
                if ui.button(text(T::EditFeatureShort)).clicked() {
                    self.intents.push(Intent::OpenSection);
                    ui.close();
                }
                if ui.button(text(T::DeleteFeature)).clicked() {
                    self.intents.push(Intent::RemoveSection);
                    ui.close();
                }
            });
        });
    }

    fn sheets(&mut self, ui: &mut egui::Ui) {
        let sheets = self.document.sheets();
        if sheets.is_empty() {
            return;
        }
        ui.separator();
        theme::section(ui, text(T::DocumentGroup));
        for sheet in sheets {
            if icons::entry(ui, Icon::Drawing, &sheet.specification.title, false).clicked() {
                self.intents
                    .push(Intent::OpenDrawing(Box::new(sheet.clone())));
            }
        }
    }
}
