//! Mensagens são eventos, distintos do estado curto exibido na barra. A fila
//! preserva avisos entregues no mesmo quadro e a conclusão de tarefas assíncronas.
use super::*;
use std::{collections::VecDeque, fmt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Level {
    Information,
    Warning,
    Error,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Notice {
    pub message: String,
    pub level: Level,
}

pub(super) struct Status {
    last: String,
    pending: VecDeque<Notice>,
    presented: bool,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            last: text(T::Ready).into(),
            pending: VecDeque::new(),
            presented: false,
        }
    }
}
impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.last)
    }
}
impl Status {
    pub fn set(&mut self, message: impl Into<String>) {
        self.report(message.into(), Level::Information);
    }
    pub fn error(&mut self, message: impl ToString) {
        self.report(message.to_string(), Level::Error);
    }
    pub fn warning(&mut self, message: impl Into<String>) {
        self.report(message.into(), Level::Warning);
    }
    /// Diagnóstico já apresentado por um diálogo especializado, como o de
    /// recálculo. Continua disponível aos relatórios sem duplicar sua janela.
    pub fn record(&mut self, message: impl ToString) {
        self.last = message.to_string();
    }
    fn report(&mut self, message: String, level: Level) {
        self.last = message.clone();
        // Pronto e processamento são estados, não mensagens que exijam leitura.
        if message.is_empty() || message == text(T::Ready) || message == text(T::Loading) {
            return;
        }
        let notice = Notice { message, level };
        if self.pending.back() != Some(&notice) {
            self.pending.push_back(notice);
        }
    }
    pub fn indicator(&self, busy: bool) -> (T, egui::Color32) {
        if busy {
            return (T::Loading, theme::ACCENT);
        }
        match self.pending.front().map(|n| n.level) {
            Some(Level::Error) => (T::Failed, theme::DANGER),
            Some(Level::Warning) => (T::StatusAttention, theme::WARNING),
            Some(Level::Information) => (T::StatusMessage, theme::ACCENT),
            None => (T::Ready, theme::GREEN),
        }
    }
    pub fn dismiss(&mut self) {
        self.pending.pop_front();
        self.presented = false;
    }
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
}

impl Editor {
    pub(super) fn status_dialog(&mut self, context: &egui::Context) {
        // A falha paramétrica já tem um diálogo com diagnóstico e resolução.
        // Evitar reapresentar o mesmo erro depois que o usuário o corrigir.
        if self.pending_failure.as_ref().is_some_and(|p| !p.fixing) {
            return;
        }
        if self.pending_close.is_some() {
            return;
        }
        let Some(notice) = self.status.pending.front().cloned() else {
            return;
        };
        let was_presented = self.status.presented;
        let result = egui::Modal::new(egui::Id::new("status_message")).show(context, |ui| {
            ui.set_width(390_f32.min((context.content_rect().width() - 60.).max(180.)));
            ui.heading(text(match notice.level {
                Level::Information => T::StatusMessage,
                Level::Warning => T::StatusAttention,
                Level::Error => T::Failed,
            }));
            ui.add_space(6.);
            egui::ScrollArea::vertical()
                .max_height(context.content_rect().height() * 0.5)
                .show(ui, |ui| {
                    ui.label(&notice.message);
                });
            ui.add_space(8.);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.button(text(T::Ok)).clicked()
            })
            .inner
        });
        self.status.presented = true;
        // Consumir as teclas protege comandos atrás do modal, mas o evento que
        // originou uma mensagem nova não pode fechá-la no mesmo quadro.
        let enter = result.is_top_modal
            && context.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let close = result.should_close();
        if result.inner || (was_presented && (close || enter)) {
            self.status.dismiss();
            context.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_states_do_not_erase_pending_messages_or_open_dialogs() {
        let mut status = Status::default();
        status.set(text(T::Loading));
        assert!(status.pending.is_empty());
        status.error("Falha de importação");
        status.error("Falha de importação");
        status.set(text(T::Ready));
        status.warning("Corrija a medida");
        assert_eq!(status.pending.len(), 2);
        assert_eq!(text(status.indicator(false).0), text(T::Failed));
        status.dismiss();
        assert_eq!(text(status.indicator(false).0), text(T::StatusAttention));
        status.dismiss();
        assert_eq!(text(status.indicator(false).0), text(T::Ready));
    }
}
