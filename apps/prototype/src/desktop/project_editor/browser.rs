//! Menus de contexto e renomeação da árvore do projeto, no padrão do navegador dos CADs de
//! mercado. Os menus só devolvem a intenção; a aplicação passa pelos comandos (ou pelos
//! controles visuais da aba, que não entram no documento).
use super::{
    icons::{self, Icon},
    *,
};
use nator_document::{DocumentError, GroupId, Instance};

/// Elemento renomeado pela árvore.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum RenameTarget {
    Instance(InstanceId),
    Part(PartId),
    Body(BodyId),
    Group(GroupId),
    Sketch(SketchId),
}

pub(super) struct RenameDraft {
    tab: u64,
    target: RenameTarget,
    name: String,
    focus: bool,
}

/// Intenção escolhida num menu de contexto da árvore.
pub(super) enum MenuAction {
    Activate(PartId),
    MakeReference(InstanceId),
    Deactivate,
    Select(Vec<Selection>),
    Move(Vec<Selection>),
    Copy(Vec<Selection>, bool),
    Command(Box<Command>),
    Pattern(InstanceId, bool),
    Mirror(InstanceId),
    Isolate(InstanceId),
    ExitIsolation,
    Transparent(InstanceId, bool),
    Lock(InstanceId, bool),
    Rename(RenameTarget, String),
    Group,
    Export(Selection),
    ExportStep(nator_commands::export::StepScope),
}

/// Estado da aba usado pelos menus.
pub(super) struct MenuContext {
    pub active: Option<PartId>,
    pub isolated: bool,
    pub transparent: HashSet<InstanceId>,
    pub locked: HashSet<InstanceId>,
    pub parts: Vec<(PartId, String)>,
    pub selected_instances: usize,
    /// Definições usadas por mais de uma instância (cópias compartilhadas).
    pub shared_parts: HashSet<PartId>,
}

/// Item "Tornar independente" para uma cópia compartilhada; nada para peças únicas.
pub(super) fn independent_item(
    ui: &mut egui::Ui,
    context: &MenuContext,
    instance: &Instance,
) -> Option<MenuAction> {
    if !context.shared_parts.contains(&instance.part) {
        return None;
    }
    let clicked = ui
        .horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(16.), egui::Sense::hover());
            icons::paint(ui.painter(), rect, Icon::Copy, ui.is_enabled());
            ui.add(egui::Button::new(text(T::MakeIndependent)).frame(false))
                .on_hover_text(text(T::MakeIndependentHint))
                .clicked()
        })
        .inner;
    clicked.then(|| {
        MenuAction::Command(Box::new(Command::MakeInstanceIndependent {
            id: instance.id,
            name: instance.name.clone(),
        }))
    })
}

fn item(ui: &mut egui::Ui, icon: Icon, label: T) -> bool {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(16.), egui::Sense::hover());
        icons::paint(ui.painter(), rect, icon, ui.is_enabled());
        ui.add(egui::Button::new(text(label)).frame(false))
            .clicked()
    })
    .inner
}

/// Menu de uma instância (componente).
pub(super) fn instance_menu(
    ui: &mut egui::Ui,
    context: &MenuContext,
    instance: &Instance,
    part_name: &str,
) -> Option<MenuAction> {
    let mut action = None;
    let target = Selection::Instance(instance.id);
    if context.active == Some(instance.part) {
        if item(ui, Icon::Cube, T::LeavePart) {
            action = Some(MenuAction::Deactivate);
        }
    } else if item(ui, Icon::Cube, T::EditPart) {
        action = Some(MenuAction::Activate(instance.part));
    }
    // Sempre clicável: com uma ferramenta aberta ou um cálculo em andamento, o clique
    // explica o que falta em vez de não fazer nada.
    let convert = ui.scope(|ui| item(ui, Icon::Import, T::ConvertToReference));
    convert
        .response
        .on_hover_text(text(T::ConvertToReferenceHint));
    if convert.inner {
        action = Some(MenuAction::MakeReference(instance.id));
    }
    ui.separator();
    if item(ui, Icon::Export, T::ExportStep) {
        action = Some(MenuAction::ExportStep(
            nator_commands::export::StepScope::Part(instance.part),
        ));
    }
    if item(ui, Icon::Export, T::Export) {
        action = Some(MenuAction::Export(target));
    }
    ui.separator();
    if item(ui, Icon::Move, T::MoveCopyTool) {
        action = Some(MenuAction::Move(vec![target]));
    }
    if item(ui, Icon::Copy, T::CopyMenu) {
        action = Some(MenuAction::Copy(vec![target], false));
    }
    if let Some(independent) = independent_item(ui, context, instance) {
        action = Some(independent);
    }
    if item(ui, Icon::Pattern, T::RectangularInstancePattern) {
        action = Some(MenuAction::Pattern(instance.id, false));
    }
    if item(ui, Icon::Pattern, T::CircularInstancePattern) {
        action = Some(MenuAction::Pattern(instance.id, true));
    }
    if item(ui, Icon::Mirror, T::MirrorComponent) {
        action = Some(MenuAction::Mirror(instance.id));
    }
    ui.menu_button(text(T::ParentPart), |ui| {
        if ui
            .add_enabled(
                instance.parent.is_some(),
                egui::Button::new(text(T::RootAssembly)),
            )
            .clicked()
        {
            action = Some(MenuAction::Command(Box::new(Command::ReparentInstance {
                id: instance.id,
                parent: None,
            })));
        }
        for (id, name) in context.parts.iter().filter(|(id, _)| *id != instance.part) {
            if ui
                .add_enabled(instance.parent != Some(*id), egui::Button::new(name))
                .clicked()
            {
                action = Some(MenuAction::Command(Box::new(Command::ReparentInstance {
                    id: instance.id,
                    parent: Some(*id),
                })));
            }
        }
    });
    if context.selected_instances > 1 && item(ui, Icon::Group, T::GroupTool) {
        action = Some(MenuAction::Group);
    }
    ui.separator();
    if context.isolated {
        if item(ui, Icon::Eye, T::ExitIsolation) {
            action = Some(MenuAction::ExitIsolation);
        }
    } else if item(ui, Icon::Eye, T::Isolate) {
        action = Some(MenuAction::Isolate(instance.id));
    }
    let mut transparent = context.transparent.contains(&instance.id);
    if ui
        .checkbox(&mut transparent, text(T::TransparentToggle))
        .changed()
    {
        action = Some(MenuAction::Transparent(instance.id, transparent));
    }
    let mut selectable = !context.locked.contains(&instance.id);
    if ui.checkbox(&mut selectable, text(T::Selectable)).changed() {
        action = Some(MenuAction::Lock(instance.id, !selectable));
    }
    if item(
        ui,
        if instance.visible {
            Icon::EyeOff
        } else {
            Icon::Eye
        },
        if instance.visible { T::Hide } else { T::Show },
    ) {
        action = Some(MenuAction::Command(Box::new(
            Command::SetInstanceVisibility {
                id: instance.id,
                visible: !instance.visible,
            },
        )));
    }
    ui.separator();
    if item(ui, Icon::Sketch, T::RenameFeature) {
        action = Some(MenuAction::Rename(
            RenameTarget::Instance(instance.id),
            instance.name.clone(),
        ));
    }
    if item(ui, Icon::Sketch, T::RenameDefinition) {
        action = Some(MenuAction::Rename(
            RenameTarget::Part(instance.part),
            part_name.to_owned(),
        ));
    }
    if item(ui, Icon::Delete, T::DeleteFeature) {
        action = Some(MenuAction::Command(Box::new(Command::RemoveInstance {
            id: instance.id,
        })));
    }
    if action.is_some() {
        ui.close();
    }
    action
}

/// Menu de um corpo de peça.
pub(super) fn body_menu(
    ui: &mut egui::Ui,
    context: &MenuContext,
    part: PartId,
    body: &nator_document::Body,
) -> Option<MenuAction> {
    let mut action = None;
    let target = Selection::Body { part, id: body.id };
    let editable = context.active == Some(part);
    let mut editing_color = false;
    if !editable && item(ui, Icon::Cube, T::EditPart) {
        action = Some(MenuAction::Activate(part));
    }
    ui.add_enabled_ui(editable, |ui| {
        if item(ui, Icon::Move, T::MoveCopyTool) {
            action = Some(MenuAction::Move(vec![target]));
        }
        if item(ui, Icon::Copy, T::CopyMenu) {
            action = Some(MenuAction::Copy(vec![target], true));
        }
    });
    if item(
        ui,
        if body.visible {
            Icon::EyeOff
        } else {
            Icon::Eye
        },
        if body.visible { T::Hide } else { T::Show },
    ) {
        action = Some(MenuAction::Command(Box::new(Command::SetBodyVisibility {
            id: body.id,
            visible: !body.visible,
        })));
    }
    ui.add_enabled_ui(body.brep.is_some(), |ui| {
        if item(ui, Icon::Export, T::ExportStep) {
            action = Some(MenuAction::ExportStep(
                nator_commands::export::StepScope::Body(body.id),
            ));
        }
    });
    if item(ui, Icon::Export, T::Export) {
        action = Some(MenuAction::Export(target));
    }
    ui.separator();
    ui.add_enabled_ui(editable, |ui| {
        ui.menu_button(text(T::BodyColor), |ui| {
            // A cor continua sendo um comando reversível. Manter o menu aberto
            // durante a escolha permite ajustar os canais sem um painel lateral.
            let mut channels = body
                .color
                .unwrap_or(nator_geometry::RgbColor::NEUTRAL)
                .channels();
            if ui.color_edit_button_srgb(&mut channels).changed() {
                editing_color = true;
                action = Some(MenuAction::Command(Box::new(Command::SetBodyColor {
                    id: body.id,
                    color: Some(nator_geometry::RgbColor::new(channels)),
                })));
            }
            if body.color.is_some() && ui.button(text(T::NeutralColor)).clicked() {
                action = Some(MenuAction::Command(Box::new(Command::SetBodyColor {
                    id: body.id,
                    color: None,
                })));
            }
        });
        if item(ui, Icon::Sketch, T::RenameFeature) {
            action = Some(MenuAction::Rename(
                RenameTarget::Body(body.id),
                body.name.clone(),
            ));
        }
        if item(ui, Icon::Delete, T::DeleteFeature) {
            action = Some(MenuAction::Command(Box::new(Command::RemoveBody {
                id: body.id,
            })));
        }
    });
    if action.is_some() && !editing_color {
        ui.close();
    }
    action
}

/// Menu de um grupo organizacional.
pub(super) fn group_menu(
    ui: &mut egui::Ui,
    group: &nator_document::assembly::Group,
) -> Option<MenuAction> {
    let mut action = None;
    let members = group
        .members
        .iter()
        .map(|id| Selection::Instance(*id))
        .collect::<Vec<_>>();
    if item(ui, Icon::Select, T::SelectMembers) {
        action = Some(MenuAction::Select(members.clone()));
    }
    if item(ui, Icon::Move, T::MoveCopyTool) {
        action = Some(MenuAction::Move(members));
    }
    if item(ui, Icon::Sketch, T::RenameFeature) {
        action = Some(MenuAction::Rename(
            RenameTarget::Group(group.id),
            group.name.clone(),
        ));
    }
    let (icon, label) = if group.rigid {
        (Icon::LockOpen, T::MakeGroupOrganizational)
    } else {
        (Icon::Group, T::MakeGroupRigid)
    };
    if item(ui, icon, label) {
        action = Some(MenuAction::Command(Box::new(Command::SetGroupRigid {
            id: group.id,
            rigid: !group.rigid,
        })));
    }
    if item(ui, Icon::Group, T::UngroupTool) {
        action = Some(MenuAction::Command(Box::new(Command::RemoveGroup {
            id: group.id,
        })));
    }
    if action.is_some() {
        ui.close();
    }
    action
}

impl Editor {
    /// Motivo que impede converter em referência agora, para avisar em vez de ignorar.
    pub(super) fn convert_reference_blocker(&self) -> Option<T> {
        if self.busy() {
            Some(T::ConvertWaitWork)
        } else if self.current().sketch_edit.is_some() {
            Some(T::ConvertFinishSketch)
        } else if self.pending_failure.is_some() {
            Some(T::ConvertResolveFailure)
        } else if !self.selection_active() {
            Some(T::ConvertToReferenceHint)
        } else {
            None
        }
    }
    pub(super) fn menu_context(&self) -> MenuContext {
        let tab = self.current();
        MenuContext {
            active: tab.active_part,
            isolated: tab.view.isolated.is_some(),
            transparent: tab.view.transparent.clone(),
            locked: tab.view.locked.clone(),
            parts: tab
                .session
                .document()
                .parts()
                .iter()
                .map(|p| (p.id, p.name.clone()))
                .collect(),
            selected_instances: tab
                .selected
                .iter()
                .filter(|s| matches!(s, Selection::Instance(_)))
                .count(),
            shared_parts: {
                let mut seen = HashSet::new();
                tab.session
                    .document()
                    .instances()
                    .iter()
                    .filter(|i| !seen.insert(i.part))
                    .map(|i| i.part)
                    .collect()
            },
        }
    }

    pub(super) fn apply_menu_action(&mut self, context: &egui::Context, action: MenuAction) {
        let tab = self.current().id;
        match action {
            // Entrar na edição de uma peça leva da faixa Montagem à Peça; sair, de volta.
            MenuAction::Activate(part)
                if self
                    .current()
                    .session
                    .document()
                    .part(part)
                    .is_some_and(|p| p.reference) =>
            {
                self.status.warning(text(T::ReferenceNotEditable));
            }
            MenuAction::MakeReference(instance) => {
                if let Some(reason) = self.convert_reference_blocker() {
                    self.status.warning(text(reason));
                    return;
                }
                let Some(part) = self
                    .current()
                    .session
                    .document()
                    .instance(instance)
                    .map(|i| i.part)
                else {
                    return;
                };
                self.current_mut()
                    .select(Some(Selection::Instance(instance)));
                self.prepare_command(
                    context,
                    tab,
                    Command::SetPartReference {
                        part,
                        reference: true,
                    },
                );
            }
            MenuAction::Activate(part) => {
                self.current_mut().active_part = Some(part);
                self.finish_sketch();
                if self.shell.tab == shell::RibbonTab::Assembly {
                    self.shell.tab = shell::RibbonTab::Model;
                }
            }
            MenuAction::Deactivate => {
                self.current_mut().active_part = None;
                self.finish_sketch();
                if self.shell.tab == shell::RibbonTab::Model {
                    self.shell.tab = shell::RibbonTab::Assembly;
                }
            }
            MenuAction::Select(items) => self.current_mut().select_many(items, false),
            MenuAction::Move(items) => {
                self.activate_selection(context);
                self.finish_sketch();
                if items.iter().all(|s| matches!(s, Selection::Instance(_))) {
                    self.current_mut().active_part = None;
                    self.shell.tab = shell::RibbonTab::Assembly;
                } else if let Some(Selection::Body { part, .. }) = items.first() {
                    self.current_mut().active_part = Some(*part);
                    self.shell.tab = shell::RibbonTab::Model;
                }
                self.current_mut().select_many(items, false);
                self.open_move();
            }
            MenuAction::Copy(items, independent) => {
                self.apply_menu_action(context, MenuAction::Move(items));
                self.open_copy(independent);
            }
            MenuAction::Command(command) => self.prepare_command(context, tab, *command),
            MenuAction::Pattern(id, circular) => {
                self.current_mut().select(Some(Selection::Instance(id)));
                self.open_instance_pattern(circular);
            }
            MenuAction::Mirror(id) => {
                self.current_mut().select(Some(Selection::Instance(id)));
                self.open_instance_mirror();
            }
            MenuAction::Isolate(id) => {
                let view = &mut self.current_mut().view;
                view.isolated = Some(HashSet::from([id]));
                view.touched();
            }
            MenuAction::ExitIsolation => {
                let view = &mut self.current_mut().view;
                view.isolated = None;
                view.touched();
            }
            MenuAction::Transparent(id, on) => {
                let view = &mut self.current_mut().view;
                if on {
                    view.transparent.insert(id);
                } else {
                    view.transparent.remove(&id);
                }
                view.touched();
            }
            MenuAction::Lock(id, on) => {
                let view = &mut self.current_mut().view;
                if on {
                    view.locked.insert(id);
                } else {
                    view.locked.remove(&id);
                }
                view.touched();
            }
            MenuAction::Rename(target, name) => {
                self.rename = Some(RenameDraft {
                    tab,
                    target,
                    name,
                    focus: true,
                });
            }
            MenuAction::Group => self.group_selection(context),
            MenuAction::Export(item) => {
                self.current_mut().select(Some(item));
                self.open_export_target(item);
            }
            MenuAction::ExportStep(scope) => self.export_step(context, tab, scope),
        }
    }

    /// Renomeação aberta pela árvore.
    pub(super) fn browser_rename_dialog(&mut self, context: &egui::Context) {
        let Some(draft) = &mut self.rename else {
            return;
        };
        let mut apply = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("browser_rename")).show(context, |ui| {
            ui.set_min_width(300.);
            ui.heading(text(T::RenameFeature));
            let field =
                ui.add(egui::TextEdit::singleline(&mut draft.name).desired_width(f32::INFINITY));
            if draft.focus {
                field.request_focus();
                draft.focus = false;
            }
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                apply = true;
            }
            ui.horizontal(|ui| {
                if ui.button(text(T::Ok)).clicked() {
                    apply = true;
                }
                if ui.button(text(T::Cancel)).clicked() {
                    cancel = true;
                }
            });
        });
        if context.input(|i| i.key_pressed(egui::Key::Escape)) {
            cancel = true;
        }
        if apply && let Some(draft) = self.rename.take() {
            let name = draft.name.trim().to_owned();
            let document = self.current().session.document();
            let command = match draft.target {
                RenameTarget::Instance(id) => rename_instance(document, id, name),
                RenameTarget::Part(id) => Command::RenamePart { id, name },
                RenameTarget::Body(id) => Command::RenameBody { id, name },
                RenameTarget::Group(id) => Command::RenameGroup { id, name },
                RenameTarget::Sketch(id) => Command::RenameSketch { id, name },
            };
            self.prepare_command(context, draft.tab, command);
        } else if cancel {
            self.rename = None;
        }
    }
}

/// Renomear uma peça na árvore muda o nome da instância e também o da definição — que
/// aparece na edição e nas cópias — quando só essa instância usa a definição ou quando ela
/// ainda tem o nome automático ("Peça 34"). Definições compartilhadas e já nomeadas mantêm
/// o nome; para elas há "Renomear definição da peça".
pub(super) fn rename_instance(document: &Document, id: InstanceId, name: String) -> Command {
    let rename = Command::RenameInstance {
        id,
        name: name.clone(),
    };
    let Some(part) = document.instance(id).and_then(|i| document.part(i.part)) else {
        return rename;
    };
    let shared = document
        .instances()
        .iter()
        .filter(|i| i.part == part.id)
        .count()
        > 1;
    let automatic = part.name.rsplit_once(' ').is_some_and(|(prefix, number)| {
        ["Peça", "Part", "Pieza"].contains(&prefix)
            && !number.is_empty()
            && number.chars().all(|c| c.is_ascii_digit())
    });
    if (shared && !automatic) || part.name == name {
        return rename;
    }
    Command::Batch {
        label: "Renomear peça",
        commands: vec![rename, Command::RenamePart { id: part.id, name }],
    }
}

/// Posição no mundo da ocorrência dada pelo caminho (instâncias da raiz até ela),
/// conferindo que cada passo é filho do anterior.
fn path_world(document: &Document, path: &[InstanceId]) -> Result<Placement, DocumentError> {
    let mut world = Placement::default();
    let mut parent = None;
    for &step in path {
        let instance = document
            .instance(step)
            .ok_or_else(|| DocumentError("Instância inexistente.".into()))?;
        if instance.parent != parent {
            return Err(DocumentError("Caminho da peça inconsistente.".into()));
        }
        world = world
            .compose(instance.placement)
            .map_err(|e| DocumentError(e.to_string()))?;
        parent = Some(instance.part);
    }
    Ok(world)
}

/// Comando para levar uma peça ou subpeça às Referências, arrastando-a na árvore.
/// `source` é o caminho da ocorrência arrastada; `destination`, o caminho da peça que a
/// recebe (`None`: Referências do projeto). A instância muda de pai mantendo a posição no
/// mundo, medida nas ocorrências arrastada e de destino, e a definição passa a ser
/// referência. Devolve `None` quando nada muda.
pub(super) fn move_to_references(
    document: &Document,
    source: &[InstanceId],
    destination: Option<&[InstanceId]>,
) -> Result<Option<Command>, DocumentError> {
    let &id = source
        .last()
        .ok_or_else(|| DocumentError("Caminho da peça vazio.".into()))?;
    let world = path_world(document, source)?;
    let instance = document
        .instance(id)
        .ok_or_else(|| DocumentError("Instância inexistente.".into()))?;
    let part = instance.part;
    let (parent, parent_world) = match destination {
        None => (None, Placement::default()),
        Some(path) => {
            let &last = path
                .last()
                .ok_or_else(|| DocumentError("Peça de destino ausente.".into()))?;
            if path.contains(&id) {
                return Err(DocumentError(
                    "Uma peça não pode ir para dentro de si mesma.".into(),
                ));
            }
            let target = document
                .instance(last)
                .ok_or_else(|| DocumentError("Peça de destino inexistente.".into()))?
                .part;
            if target == part {
                return Err(DocumentError(
                    "Uma peça não pode ir para dentro de si mesma.".into(),
                ));
            }
            if document.part(target).is_some_and(|p| p.reference) {
                return Err(DocumentError(
                    "Referências não recebem outros elementos.".into(),
                ));
            }
            (Some(target), path_world(document, path)?)
        }
    };
    let mut commands = Vec::new();
    // Mesmo pai: a instância já pertence à definição dele, em todas as ocorrências.
    if instance.parent != parent {
        let placement = parent_world
            .inverse()
            .and_then(|inverse| inverse.compose(world))
            .map_err(|e| DocumentError(e.to_string()))?;
        commands.push(Command::ReparentInstance { id, parent });
        commands.push(Command::MoveInstance { id, placement });
    }
    if !document.part(part).is_some_and(|p| p.reference) {
        commands.push(Command::SetPartReference {
            part,
            reference: true,
        });
    }
    Ok(match commands.len() {
        0 => None,
        1 => commands.pop(),
        _ => Some(Command::Batch {
            label: "Mover para Referências",
            commands,
        }),
    })
}

#[cfg(test)]
mod rename_tests {
    use super::*;
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn part_name(session: &Session, part: PartId) -> Result<String> {
        Ok(session
            .document()
            .part(part)
            .ok_or("Peça ausente")?
            .name
            .clone())
    }

    #[test]
    fn renaming_a_part_in_the_tree_also_renames_its_definition_when_appropriate() -> Result<()> {
        let mut document = Document::new("Nomes")?;
        let (part, _) = document.add_mesh(
            None,
            "Corpo".into(),
            Arc::new(MeshAsset::new(reference_bracket()?)),
        )?;
        let first = document
            .instances()
            .iter()
            .find(|i| i.part == part)
            .ok_or("Instância ausente")?
            .id;
        document.part_mut(part)?.name = "Peça 34".into();
        let mut session = Session::new(document)?;
        // Instância única: a definição acompanha, numa só ação de desfazer.
        session.execute(rename_instance(session.document(), first, "Barra".into()))?;
        assert_eq!(part_name(&session, part)?, "Barra");
        assert!(session.undo());
        assert_eq!(part_name(&session, part)?, "Peça 34");
        // Compartilhada com nome automático: também acompanha.
        session.execute(Command::DuplicateInstance { id: first })?;
        session.execute(rename_instance(session.document(), first, "Barra 1".into()))?;
        assert_eq!(part_name(&session, part)?, "Barra 1");
        // Compartilhada e já nomeada: só a instância muda.
        session.execute(rename_instance(session.document(), first, "Barra X".into()))?;
        assert_eq!(part_name(&session, part)?, "Barra 1");
        let names = session
            .document()
            .instances()
            .iter()
            .filter(|i| i.part == part)
            .map(|i| i.name.clone())
            .collect::<Vec<_>>();
        assert!(names.contains(&"Barra X".to_owned()));
        // A cópia compartilhada herda o nome da instância copiada.
        session.execute(Command::DuplicateInstance { id: first })?;
        let copy = session
            .document()
            .instances()
            .last()
            .ok_or("Cópia ausente")?;
        assert_eq!(copy.name, "Barra X");
        Ok(())
    }
}

#[cfg(test)]
mod reference_tests {
    use super::*;
    use nator_geometry::{norm, sub};
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn references_move_between_parts_keeping_their_world_position() -> Result<()> {
        let mut document = Document::new("Montagem")?;
        let mut add = |name: &str| -> Result<(PartId, InstanceId)> {
            let (part, _) = document.add_mesh(
                None,
                name.into(),
                Arc::new(MeshAsset::new(reference_bracket()?)),
            )?;
            let instance = document
                .instances()
                .iter()
                .find(|i| i.part == part)
                .ok_or("Instância ausente")?
                .id;
            Ok((part, instance))
        };
        let (a, a_instance) = add("A")?;
        let (b, b_instance) = add("B")?;
        let (reference, r_instance) = add("Motor")?;
        document.instance_mut(a_instance)?.placement =
            Placement::new([50., 0., 0.], [0., 0., 0.7])?;
        document.instance_mut(b_instance)?.placement =
            Placement::new([0., -40., 10.], [0.3, 0., 0.])?;
        document.instance_mut(r_instance)?.placement = Placement::new([5., 6., 7.], [0., 0.2, 0.])?;
        let world = document
            .instance(r_instance)
            .ok_or("Referência ausente")?
            .placement;
        let mut session = Session::new(document)?;
        // Da raiz para dentro de A: vira referência e mantém a posição no mundo.
        let command = move_to_references(session.document(), &[r_instance], Some(&[a_instance]))?
            .ok_or("Comando ausente")?;
        session.execute(command)?;
        let at = |document: &Document, path: &[InstanceId]| path_world(document, path);
        let document = session.document();
        assert_eq!(
            document
                .instance(r_instance)
                .ok_or("Referência ausente")?
                .parent,
            Some(a)
        );
        assert!(document.part(reference).ok_or("Peça ausente")?.reference);
        let moved = at(document, &[a_instance, r_instance])?;
        for point in [[0., 0., 0.], [10., -3., 7.]] {
            assert!(
                nator_geometry::norm(nator_geometry::sub(moved.apply(point), world.apply(point)))
                    < 1e-9
            );
        }
        // De A para B, pela ocorrência arrastada.
        let command = move_to_references(
            session.document(),
            &[a_instance, r_instance],
            Some(&[b_instance]),
        )?
        .ok_or("Comando ausente")?;
        session.execute(command)?;
        let document = session.document();
        assert_eq!(
            document
                .instance(r_instance)
                .ok_or("Referência ausente")?
                .parent,
            Some(b)
        );
        let moved = at(document, &[b_instance, r_instance])?;
        assert!(
            nator_geometry::norm(nator_geometry::sub(
                moved.apply([1., 2., 3.]),
                world.apply([1., 2., 3.])
            )) < 1e-9
        );
        // Soltar no mesmo pai não faz nada; referência não recebe; peça não entra em si.
        assert!(
            move_to_references(document, &[b_instance, r_instance], Some(&[b_instance]))?.is_none()
        );
        assert!(
            move_to_references(document, &[a_instance], Some(&[b_instance, r_instance])).is_err()
        );
        assert!(move_to_references(document, &[b_instance], Some(&[b_instance])).is_err());
        // De volta à raiz do projeto.
        let command = move_to_references(document, &[b_instance, r_instance], None)?
            .ok_or("Comando ausente")?;
        session.execute(command)?;
        let back = session
            .document()
            .instance(r_instance)
            .ok_or("Referência ausente")?;
        assert!(back.parent.is_none());
        assert!(
            nator_geometry::norm(nator_geometry::sub(
                back.placement.apply([1., 2., 3.]),
                world.apply([1., 2., 3.])
            )) < 1e-9
        );
        Ok(())
    }

    #[test]
    fn subpart_moves_to_references_keeping_its_world_position() -> Result<()> {
        let mut document = Document::new("Montagem")?;
        let (parent_part, _) = document.add_mesh(
            None,
            "Conjunto".into(),
            Arc::new(MeshAsset::new(reference_bracket()?)),
        )?;
        let (child_part, _) = document.add_mesh(
            None,
            "Suporte".into(),
            Arc::new(MeshAsset::new(reference_bracket()?)),
        )?;
        let instance_of = |document: &Document, part| {
            document
                .instances()
                .iter()
                .find(|i| i.part == part)
                .map(|i| i.id)
                .ok_or("Instância ausente")
        };
        let root = instance_of(&document, parent_part)?;
        let child = instance_of(&document, child_part)?;
        document.instance_mut(root)?.placement = Placement::new([100., 0., 0.], [0., 0., 0.5])?;
        document.reparent(child, Some(parent_part))?;
        document.instance_mut(child)?.placement = Placement::new([0., 20., 5.], [0.2, 0., 0.])?;
        let before = document
            .occurrences()?
            .into_iter()
            .find(|o| o.instance == child)
            .ok_or("Ocorrência ausente")?
            .placement;
        let command =
            move_to_references(&document, &[root, child], None)?.ok_or("Comando ausente")?;
        let mut session = Session::new(document)?;
        session.execute(command)?;
        let document = session.document();
        let moved = document.instance(child).ok_or("Subpeça ausente")?;
        assert!(moved.parent.is_none());
        assert!(document.part(child_part).ok_or("Peça ausente")?.reference);
        assert!(!document.part(parent_part).ok_or("Pai ausente")?.reference);
        for point in [[0., 0., 0.], [10., -3., 7.]] {
            assert!(norm(sub(moved.placement.apply(point), before.apply(point))) < 1e-9);
        }
        // Já é referência: nada a fazer. Caminho que não corresponde à árvore falha.
        assert!(move_to_references(document, &[child], None)?.is_none());
        assert!(move_to_references(document, &[root, child], None).is_err());
        assert!(session.undo());
        assert_eq!(
            session
                .document()
                .instance(child)
                .ok_or("Subpeça ausente")?
                .parent,
            Some(parent_part)
        );
        Ok(())
    }
}

#[cfg(all(test, feature = "occt"))]
mod reference_step_tests {
    use super::*;
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// Ensaio optativo com um STEP de montagem real: uma subpeça vai às Referências na
    /// mesma posição e continua exportável em STEP com seus corpos.
    #[test]
    #[ignore = "Requer um STEP externo indicado por NATOR_STEP_PATH."]
    fn external_step_subpart_moves_to_references_and_exports_step() -> Result<()> {
        let path = std::path::PathBuf::from(
            std::env::var_os("NATOR_STEP_PATH").ok_or("Informe NATOR_STEP_PATH.")?,
        );
        let mut session = Session::new(Document::new("STEP")?)?;
        session.execute(Command::ImportStep {
            path,
            destination: None,
            name: "Montagem".into(),
        })?;
        let occurrence = session
            .document()
            .occurrences()?
            .into_iter()
            .find(|o| o.path.len() >= 2)
            .ok_or("O STEP não tem subpeças")?;
        let part = session
            .document()
            .instance(occurrence.instance)
            .ok_or("Instância ausente")?
            .part;
        let command = move_to_references(session.document(), &occurrence.path, None)?
            .ok_or("Comando ausente")?;
        session.execute(command)?;
        let document = session.document();
        let moved = document
            .instance(occurrence.instance)
            .ok_or("Subpeça ausente")?;
        assert!(moved.parent.is_none());
        assert!(document.part(part).ok_or("Peça ausente")?.reference);
        let origin = moved.placement.apply([0.; 3]);
        let before = occurrence.placement.apply([0.; 3]);
        assert!(nator_geometry::norm(nator_geometry::sub(origin, before)) < 1e-9);
        let scene = nator_commands::export::step_scene(
            document,
            nator_commands::export::StepScope::Part(part),
        )?;
        assert!(scene.parts.iter().any(|p| !p.bodies.is_empty()));
        eprintln!(
            "Subpeça {} movida às Referências; STEP com {} peças.",
            moved.name,
            scene.parts.len()
        );
        Ok(())
    }
}
