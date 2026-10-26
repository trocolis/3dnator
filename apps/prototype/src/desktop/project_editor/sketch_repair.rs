//! Ferramenta Reparar rascunho: lista as correções sugeridas pelo diagnóstico, mostra cada
//! uma sobre o rascunho e aplica somente as escolhidas, pelo mesmo caminho de solução das
//! demais edições. Cruzamentos são apenas indicados: exigem decidir onde aparar.
use super::{
    command_panel::{self as panel, Footer},
    icons::Icon,
    *,
};
use nator_sketch::repair::{self, Fix, Report};

pub(super) struct RepairDraft {
    tab: u64,
    sketch: SketchId,
    /// Folga máxima, em mm, para sugerir a união de pontas abertas.
    tolerance: f64,
    report: Option<Report>,
    /// Chave do rascunho analisado; muda quando o rascunho é editado.
    analyzed: Option<(String, f64)>,
    chosen: Vec<bool>,
    error: Option<String>,
}

fn describe(fix: &Fix) -> String {
    match fix {
        Fix::RemoveDuplicate { entity, keeps } => {
            format!("{} #{entity} (= #{keeps})", text(T::RepairDuplicate))
        }
        Fix::RemoveContained { entity, within } => {
            format!("{} #{entity} (⊂ #{within})", text(T::RepairContained))
        }
        Fix::CloseGap { a, b, gap } => format!("{} #{a} · #{b} ({gap:.3} mm)", text(T::RepairGap)),
    }
}

impl Editor {
    pub(super) fn open_sketch_repair(&mut self) {
        let Some(work) = &self.current().sketch_edit else {
            return;
        };
        self.sketch_repair = Some(RepairDraft {
            tab: self.current().id,
            sketch: work.id,
            tolerance: 0.5,
            report: None,
            analyzed: None,
            chosen: Vec::new(),
            error: None,
        });
    }

    /// Recalcula o diagnóstico quando o rascunho ou a tolerância mudam.
    fn refresh_repair(&mut self) {
        let Some(sketch) = self.working_sketch() else {
            return;
        };
        let key = format!(
            "{:?}",
            (&sketch.points, &sketch.entities, sketch.constraints.len())
        );
        let Some(draft) = &mut self.sketch_repair else {
            return;
        };
        let current = Some((key, draft.tolerance));
        if draft.analyzed == current {
            return;
        }
        draft.analyzed = current;
        match repair::suggest(&sketch, draft.tolerance) {
            Ok(report) => {
                draft.chosen = vec![true; report.fixes.len()];
                draft.report = Some(report);
                draft.error = None;
            }
            Err(error) => {
                draft.report = None;
                draft.error = Some(error.to_string());
            }
        }
    }

    pub(super) fn sketch_repair_panel(&mut self, context: &egui::Context) {
        let Some(draft) = &self.sketch_repair else {
            return;
        };
        let editing = self
            .current()
            .sketch_edit
            .as_ref()
            .is_some_and(|w| w.id == draft.sketch && self.current().id == draft.tab);
        if !editing {
            self.sketch_repair = None;
            return;
        }
        self.refresh_repair();
        let Some(mut draft) = self.sketch_repair.take() else {
            return;
        };
        let any = draft.chosen.iter().any(|c| *c);
        let outcome = panel::show(
            context,
            panel::Header {
                id: "sketch_repair",
                icon: Icon::Check,
                title: text(T::RepairSketch),
            },
            self.canvas_rect,
            Footer::None,
            any && !self.busy(),
            |ui| {
                panel::row(ui, text(T::RepairTolerance), |ui| {
                    ui.add(
                        egui::DragValue::new(&mut draft.tolerance)
                            .range(0.001..=50.)
                            .speed(0.05)
                            .suffix(" mm")
                            .max_decimals(3),
                    )
                });
                if let Some(error) = &draft.error {
                    ui.colored_label(theme::DANGER, error);
                }
                if let Some(report) = &draft.report {
                    if report.fixes.is_empty() {
                        ui.label(egui::RichText::new(text(T::RepairNothing)).color(theme::GREEN));
                    }
                    for (fix, chosen) in report.fixes.iter().zip(&mut draft.chosen) {
                        ui.checkbox(chosen, describe(fix));
                    }
                    for point in &report.crossings {
                        ui.colored_label(
                            theme::WARNING,
                            format!(
                                "{} ({:.2}; {:.2}) — {}",
                                text(T::RepairCrossing),
                                point[0],
                                point[1],
                                text(T::RepairCrossingHint)
                            ),
                        );
                    }
                }
                ui.label(
                    egui::RichText::new(text(T::RepairHint))
                        .small()
                        .color(theme::MUTED),
                );
            },
        );
        if outcome.cancel {
            return;
        }
        if outcome.ok
            && let Some(report) = &draft.report
            && let Some(sketch) = self.working_sketch()
        {
            let fixes = report
                .fixes
                .iter()
                .zip(&draft.chosen)
                .filter(|(_, chosen)| **chosen)
                .map(|(fix, _)| fix.clone())
                .collect::<Vec<_>>();
            match repair::apply(&sketch, &fixes) {
                Ok(repaired) => {
                    self.solve_sketch(context, repaired);
                    return;
                }
                Err(error) => draft.error = Some(error.to_string()),
            }
        }
        self.sketch_repair = Some(draft);
    }

    /// Prévia sobre o rascunho: entidades e pontas envolvidas em laranja, cruzamentos em
    /// vermelho. Só desenha; o rascunho não muda até a confirmação.
    pub(super) fn sketch_repair_overlay(&self, ui: &egui::Ui, response: &egui::Response) {
        let (Some(draft), Some(work), Some(sketch)) = (
            &self.sketch_repair,
            &self.current().sketch_edit,
            self.working_sketch(),
        ) else {
            return;
        };
        let Some(report) = &draft.report else {
            return;
        };
        let frame = sketch_scene::Frame {
            camera: self.viewport.camera.clone(),
            bounds: self.display_bounds(),
            rect: response.rect,
            placement: work.placement,
            plane: sketch.plane,
        };
        let painter = ui
            .painter()
            .with_clip_rect(response.rect.intersect(ui.clip_rect()));
        let orange = egui::Color32::from_rgb(232, 128, 22);
        for (fix, chosen) in report.fixes.iter().zip(&draft.chosen) {
            let alpha = if *chosen { 255 } else { 90 };
            let color = egui::Color32::from_rgba_unmultiplied(232, 128, 22, alpha);
            for id in fix.entities() {
                if let Ok(entity) = sketch.entity(id)
                    && let Ok(curve) = nator_sketch::profiles::segment(&sketch, entity)
                {
                    frame.path(
                        &painter,
                        curve.sample(64),
                        egui::Stroke::new(4., color),
                        false,
                    );
                }
            }
            for id in fix.points() {
                if let Ok(point) = sketch.point(id)
                    && let Some(p) = frame.screen(point)
                {
                    painter.circle_stroke(p, 9., egui::Stroke::new(2.5, orange));
                }
            }
        }
        for point in &report.crossings {
            if let Some(p) = frame.screen(*point) {
                let s = 7.;
                let stroke = egui::Stroke::new(2.5, theme::DANGER);
                painter.line_segment([p + egui::vec2(-s, -s), p + egui::vec2(s, s)], stroke);
                painter.line_segment([p + egui::vec2(-s, s), p + egui::vec2(s, -s)], stroke);
            }
        }
    }
}
