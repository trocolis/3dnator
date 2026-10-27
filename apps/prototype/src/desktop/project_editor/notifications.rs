//! Informações e avisos ficam na barra de status. Somente erros entram na fila
//! de diálogos, preservando falhas recebidas durante tarefas assíncronas.
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
    current: Option<Notice>,
    pending: VecDeque<Notice>,
    presented: bool,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            last: text(T::Ready).into(),
            current: None,
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
        self.current = Some(Notice {
            message: self.last.clone(),
            level: Level::Error,
        });
    }
    fn report(&mut self, message: String, level: Level) {
        self.last = message.clone();
        // A preparação de cena também emite Pronto/Processando. Essas transições
        // não devem apagar uma informação ou aviso antes que se possa lê-lo.
        if message.is_empty() || message == text(T::Ready) || message == text(T::Loading) {
            if message == text(T::Ready)
                && self.pending.is_empty()
                && self
                    .current
                    .as_ref()
                    .is_some_and(|n| n.level == Level::Error)
            {
                self.current = None;
            }
            return;
        }
        let notice = Notice { message, level };
        self.current = Some(notice.clone());
        if level == Level::Error && self.pending.back() != Some(&notice) {
            self.pending.push_back(notice);
        }
    }
    pub fn indicator(&self, busy: bool) -> (&str, egui::Color32) {
        if let Some(error) = self.pending.front() {
            return (&error.message, theme::DANGER);
        }
        if busy {
            return (text(T::Loading), theme::ACCENT);
        }
        match self.current.as_ref() {
            Some(notice) => (
                &notice.message,
                match notice.level {
                    Level::Error => theme::DANGER,
                    Level::Warning => theme::WARNING,
                    Level::Information => theme::GREEN,
                },
            ),
            None => (text(T::Ready), theme::GREEN),
        }
    }
    pub fn dismiss(&mut self) {
        self.pending.pop_front();
        if self.pending.is_empty()
            && self
                .current
                .as_ref()
                .is_some_and(|n| n.level == Level::Error)
        {
            self.current = None;
        }
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
            ui.heading(text(T::Failed));
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
    fn information_and_warnings_stay_in_status_while_only_errors_open_dialogs() {
        let mut status = Status::default();
        status.set("Projeto salvo");
        assert!(!status.has_pending());
        assert_eq!(status.indicator(false), ("Projeto salvo", theme::GREEN));
        status.warning("Corrija a medida");
        status.set(text(T::Loading));
        assert_eq!(status.indicator(true).0, text(T::Loading));
        status.set(text(T::Ready));
        assert!(status.pending.is_empty());
        assert_eq!(
            status.indicator(false),
            ("Corrija a medida", theme::WARNING)
        );
        status.error("Falha de importação");
        status.error("Falha de importação");
        status.error("Arquivo inválido");
        status.set(text(T::Ready));
        status.set("Informação recebida durante o erro");
        assert_eq!(status.pending.len(), 2);
        assert_eq!(status.indicator(false).0, "Falha de importação");
        status.dismiss();
        assert_eq!(status.indicator(false).0, "Arquivo inválido");
        status.dismiss();
        assert!(!status.has_pending());
        assert_eq!(
            status.indicator(false).0,
            "Informação recebida durante o erro"
        );
    }
}
