//! Menus de contexto e renomeação da árvore do projeto, no padrão do navegador dos CADs de
//! mercado. Os menus só devolvem a intenção; a aplicação passa pelos comandos (ou pelos
//! controles visuais da aba, que não entram no documento).
use super::{
    icons::{self, Icon},
    *,
};
use nator_document::{GroupId, Instance};

/// Elemento renomeado pela árvore.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum RenameTarget {
    Instance(InstanceId),
    Part(PartId),
    Body(BodyId),
    Group(GroupId),
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
    Deactivate,
    Select(Vec<Selection>),
    Move(Vec<Selection>),
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
}

/// Estado da aba usado pelos menus.
pub(super) struct MenuContext {
    pub active: Option<PartId>,
    pub isolated: bool,
    pub transparent: HashSet<InstanceId>,
    pub locked: HashSet<InstanceId>,
    pub parts: Vec<(PartId, String)>,
    pub selected_instances: usize,
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

/// Botão de ativação da peça (construção), à esquerda do nome.
pub(super) fn activation(ui: &mut egui::Ui, active: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(18.), egui::Sense::click());
    let center = rect.center();
    ui.painter().circle_stroke(
        center,
        5.5,
        egui::Stroke::new(1.3, if active { theme::ACCENT } else { theme::MUTED }),
    );
    if active {
        ui.painter().circle_filled(center, 3., theme::ACCENT);
    } else if response.hovered() {
        ui.painter().circle_filled(center, 2.5, theme::BLUE_FILL);
    }
    response.on_hover_text(text(if active { T::LeavePart } else { T::EditPart }))
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
    ui.separator();
    if item(ui, Icon::Move, T::MoveCopyTool) {
        action = Some(MenuAction::Move(vec![target]));
    }
    if item(ui, Icon::NewBody, T::NewInstanceTool) {
        action = Some(MenuAction::Command(Box::new(Command::DuplicateInstance {
            id: instance.id,
        })));
    }
    if item(ui, Icon::Boolean, T::CloneTool) {
        action = Some(MenuAction::Command(Box::new(Command::CloneInstance {
            id: instance.id,
            name: format!("{part_name} ({})", text(T::CopySuffix)),
        })));
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
    if !editable && item(ui, Icon::Cube, T::EditPart) {
        action = Some(MenuAction::Activate(part));
    }
    ui.add_enabled_ui(editable, |ui| {
        if item(ui, Icon::Move, T::MoveCopyTool) {
            action = Some(MenuAction::Move(vec![target]));
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
    if item(ui, Icon::Export, T::Export) {
        action = Some(MenuAction::Export(target));
    }
    ui.separator();
    ui.add_enabled_ui(editable, |ui| {
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
    if action.is_some() {
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
        }
    }

    pub(super) fn apply_menu_action(&mut self, context: &egui::Context, action: MenuAction) {
        let tab = self.current().id;
        match action {
            MenuAction::Activate(part) => {
                self.current_mut().active_part = Some(part);
                self.finish_sketch();
            }
            MenuAction::Deactivate => {
                self.current_mut().active_part = None;
                self.finish_sketch();
            }
            MenuAction::Select(items) => self.current_mut().select_many(items, false),
            MenuAction::Move(items) => {
                self.current_mut().select_many(items, false);
                self.open_move();
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
                self.open_export();
            }
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
            let command = match draft.target {
                RenameTarget::Instance(id) => Command::RenameInstance { id, name },
                RenameTarget::Part(id) => Command::RenamePart { id, name },
                RenameTarget::Body(id) => Command::RenameBody { id, name },
                RenameTarget::Group(id) => Command::RenameGroup { id, name },
            };
            self.prepare_command(context, draft.tab, command);
        } else if cancel {
            self.rename = None;
        }
    }
}
