//! Operações de organização são comandos explícitos: desagrupar preserva os elementos.
use super::*;

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
        let menu = self.menu_context();
        let busy = self.busy();
        for child in document
            .instances()
            .iter()
            .filter(|i| i.parent == Some(parent))
        {
            let response = icons::entry(
                ui,
                icons::Icon::Cube,
                &child.name,
                self.current().is_selected(Selection::Instance(child.id)),
            );
            if response.clicked() {
                *selection = Some(Selection::Instance(child.id));
            }
            let definition = document
                .part(child.part)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            response.context_menu(|ui| {
                ui.add_enabled_ui(!busy, |ui| {
                    if let Some(action) = browser::instance_menu(ui, &menu, child, &definition) {
                        self.pending_menu = Some(action);
                    }
                });
            });
            ui.indent(("child", depth, child.id.0), |ui| {
                if let Some(part) = document.part(child.part) {
                    for b in &part.bodies {
                        let target = Selection::Body {
                            part: part.id,
                            id: b.id,
                        };
                        let response = icons::entry(
                            ui,
                            icons::Icon::Cube,
                            &b.name,
                            self.current().is_selected(target),
                        );
                        if response.clicked() {
                            *selection = Some(target);
                        }
                        response.context_menu(|ui| {
                            ui.add_enabled_ui(!busy, |ui| {
                                if let Some(action) = browser::body_menu(ui, &menu, part.id, b) {
                                    self.pending_menu = Some(action);
                                }
                            });
                        });
                    }
                    for s in &part.sketches {
                        self.sketch_tree_entry(ui, part.id, s);
                    }
                    self.child_tree(ui, document, part.id, depth + 1, selection);
                }
            });
        }
    }
}
