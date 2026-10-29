//! Navegação após desfazer/refazer. O núcleo informa IDs/contexto; somente a
//! interface decide quais editores abrir, sem salvar egui no documento.
use super::*;
use nator_document::activity::EditContext;
impl Editor {
    pub(super) fn step_history(&mut self, ctx: &egui::Context, redo: bool) {
        let scope = if redo {
            self.current().session.redo_context()
        } else {
            self.current().session.undo_context()
        };
        let changed = if redo {
            self.current_mut().session.redo()
        } else {
            self.current_mut().session.undo()
        };
        if !changed {
            return;
        }
        // Dentro do mesmo rascunho preservamos ferramenta e propriedades
        // abertas (por exemplo Cota/Imagem). Só a prévia CAD fica obsoleta.
        self.cancel_feature_tool();
        self.preview_command(self.current().id, None, None, None);
        if let Some(work) = self.current_mut().sketch_edit.as_mut() {
            work.pending = None;
        }
        match scope.unwrap_or(EditContext::Keep) {
            EditContext::Keep => {}
            EditContext::Assembly => {
                self.current_mut().sketch_edit = None;
                self.current_mut().active_part = None;
                self.shell.tab = shell::RibbonTab::Assembly;
            }
            EditContext::Part(part) => self.history_part(part),
            EditContext::Sketch { part, sketch } => {
                if self.current().session.document().sketch(sketch).is_some() {
                    if self
                        .current()
                        .sketch_edit
                        .as_ref()
                        .is_none_or(|w| w.id != sketch)
                    {
                        self.current_mut().sketch_edit = None;
                        self.begin_history_sketch(part, sketch);
                    }
                    self.shell.tab = shell::RibbonTab::Sketch;
                } else if let Some(part) = part {
                    self.history_part(part)
                } else {
                    self.current_mut().sketch_edit = None;
                    self.current_mut().active_part = None;
                    self.shell.tab = shell::RibbonTab::Assembly;
                }
            }
        }
        self.sanitize_sketch_selection();
        self.refresh_reference_image();
        self.current_mut().changed();
        self.invalidate_scene();
        ctx.request_repaint();
    }
    fn history_part(&mut self, part: PartId) {
        self.current_mut().sketch_edit = None;
        self.current_mut().active_part = self.current().session.document().part(part).map(|p| p.id);
        self.shell.tab = if self.current().active_part.is_some() {
            shell::RibbonTab::Model
        } else {
            shell::RibbonTab::Assembly
        };
    }
}
