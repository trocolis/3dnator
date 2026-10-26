//! Operações de organização são comandos explícitos: desagrupar preserva os elementos.
use super::*;
use nator_document::GroupId;
pub(super) struct Draft {
    tab: u64,
    name: String,
    members: Vec<InstanceId>,
    delta: Point3,
    group: Option<GroupId>,
    parent: Option<PartId>,
}
impl Draft {
    pub fn new(tab: u64) -> Self {
        Self {
            tab,
            name: text(T::Groups).into(),
            members: Vec::new(),
            delta: [0.; 3],
            group: None,
            parent: None,
        }
    }
}
impl Editor {
    pub(super) fn child_tree(
        &mut self,
        ui: &mut egui::Ui,
        document: &Document,
        parent: PartId,
        depth: usize,
        selection: &mut Option<Selection>,
    ) {
        if depth > 64 {
            return;
        }
        for child in document
            .instances()
            .iter()
            .filter(|i| i.parent == Some(parent))
        {
            if ui
                .selectable_label(
                    self.current().selection == Some(Selection::Instance(child.id)),
                    format!("{} · {}", child.name, text(T::Assembly)),
                )
                .clicked()
            {
                *selection = Some(Selection::Instance(child.id));
            }
            ui.indent(("child", depth, child.id.0), |ui| {
                if let Some(part) = document.part(child.part) {
                    for b in &part.bodies {
                        let target = Selection::Body {
                            part: part.id,
                            id: b.id,
                        };
                        if ui
                            .selectable_label(self.current().selection == Some(target), &b.name)
                            .clicked()
                        {
                            *selection = Some(target);
                        }
                    }
                    for s in &part.sketches {
                        if ui.button(&s.sketch.name).clicked() {
                            self.begin_sketch(part.id, s.id);
                        }
                    }
                    self.child_tree(ui, document, part.id, depth + 1, selection);
                }
            });
        }
    }
    pub(super) fn assembly_dialog(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.assembly_edit.take() else {
            return;
        };
        let Some(tab) = self.tabs.iter().find(|t| t.id == draft.tab) else {
            return;
        };
        let document = tab.session.document().clone();
        let selection = tab.selection;
        let mut open = true;
        let mut command = None;
        egui::Window::new(text(T::AssemblyTools))
            .open(&mut open)
            .show(context, |ui| {
                ui.add_enabled_ui(!self.busy(), |ui| {
                    if let Some(Selection::Instance(id)) = selection
                        && let Some(instance) = document.instance(id)
                    {
                        ui.label(&instance.name);
                        if ui.button(text(T::DuplicateInstance)).clicked() {
                            command = Some(Command::AddInstance {
                                part: instance.part,
                                parent: instance.parent,
                                placement: instance.placement,
                            });
                        }
                        if ui.button(text(T::IndependentClone)).clicked() {
                            command = Some(Command::CloneInstance {
                                id,
                                name: format!("{} (cópia)", instance.name),
                            });
                        }
                        egui::ComboBox::from_label(text(T::ParentPart))
                            .selected_text(
                                draft
                                    .parent
                                    .and_then(|p| document.part(p))
                                    .map(|p| p.name.as_str())
                                    .unwrap_or(text(T::RootAssembly)),
                            )
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut draft.parent, None, text(T::RootAssembly));
                                for p in document.parts().iter().filter(|p| p.id != instance.part) {
                                    ui.selectable_value(&mut draft.parent, Some(p.id), &p.name);
                                }
                            });
                        if ui.button(text(T::Apply)).clicked() {
                            command = Some(Command::ReparentInstance {
                                id,
                                parent: draft.parent,
                            });
                        }
                        if ui.button(text(T::DeleteInstance)).clicked() {
                            command = Some(Command::RemoveInstance { id });
                        }
                    } else {
                        ui.label(text(T::SelectElement));
                    }
                    ui.separator();
                    ui.heading(text(T::Groups));
                    ui.label(text(T::GroupMembers));
                    egui::ScrollArea::vertical()
                        .max_height(140.)
                        .show(ui, |ui| {
                            for i in document.instances() {
                                let mut member = draft.members.contains(&i.id);
                                if ui.checkbox(&mut member, &i.name).changed() {
                                    if member {
                                        draft.members.push(i.id);
                                    } else {
                                        draft.members.retain(|id| *id != i.id);
                                    }
                                }
                            }
                        });
                    ui.label(text(T::GroupName));
                    ui.text_edit_singleline(&mut draft.name);
                    if ui.button(text(T::CreateGroup)).clicked() {
                        command = Some(Command::CreateGroup {
                            name: draft.name.clone(),
                            members: draft.members.clone(),
                        });
                    }
                    for g in document.groups() {
                        ui.selectable_value(&mut draft.group, Some(g.id), &g.name);
                    }
                    if let Some(id) = draft.group {
                        ui.label(text(T::GroupDelta));
                        ui.horizontal(|ui| {
                            for v in &mut draft.delta {
                                ui.add(egui::DragValue::new(v).speed(0.1).range(-1e9..=1e9));
                            }
                        });
                        if ui.button(text(T::Apply)).clicked() {
                            match Placement::new(draft.delta, [0.; 3]) {
                                Ok(delta) => command = Some(Command::MoveGroup { id, delta }),
                                Err(e) => self.status = e.to_string(),
                            }
                        }
                        if ui.button(text(T::Ungroup)).clicked() {
                            command = Some(Command::RemoveGroup { id });
                            draft.group = None;
                        }
                    }
                });
            });
        if let Some(command) = command {
            self.prepare_command(context, draft.tab, command);
        }
        if open {
            self.assembly_edit = Some(draft);
        }
    }
}
