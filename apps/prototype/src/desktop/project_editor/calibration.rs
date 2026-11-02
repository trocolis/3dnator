//! Calibração da impressora: exporta a peça de teste e converte as medidas em perfil. Vale
//! para qualquer impressora ou fatiador, porque só usa o que saiu impresso.
use super::{export_editor, icons::Icon, *};
use nator_commands::{calibration, export};
use nator_document::print_profile::{CALIBRATION_FITS, CALIBRATION_HOLES, Calibration};

pub(super) struct Draft {
    tab: u64,
    measured: Calibration,
}

impl Editor {
    pub(super) fn open_calibration(&mut self) {
        self.calibration = Some(Draft {
            tab: self.current().id,
            measured: Calibration {
                holes: CALIBRATION_HOLES,
                fits: [1, 3, 5],
            },
        });
    }

    fn export_calibration(&mut self, context: &egui::Context, bed: Option<[f64; 3]>) {
        self.start(context, None, move || {
            let files = calibration::parts().map_err(|e| e.to_string())?;
            let layout = export::prepare_for_bed(&files, bed).map_err(|e| e.to_string())?;
            let Some(paths) = export_editor::write_three_mf(&layout.files, "Calibracao")? else {
                return Ok(Outcome::Cancelled);
            };
            export_editor::open_slicer_if_asked(&paths)?;
            Ok(Outcome::Exported)
        });
    }

    pub(super) fn calibration_dialog(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.calibration.take() else {
            return;
        };
        let Some(profile) = self
            .tabs
            .iter()
            .find(|t| t.id == draft.tab)
            .map(|t| t.session.document().print_profile().clone())
        else {
            return;
        };
        let mut export = false;
        let mut apply = None;
        let result = profile.calibrated(&draft.measured);
        let fit_label = |index: usize| format!("{} mm", CALIBRATION_FITS[index]).replace('.', ",");
        let outcome = command_panel::options(
            context,
            command_panel::Header {
                id: "calibration",
                icon: Icon::Calibrate,
                title: text(T::Calibrate),
            },
            self.canvas_rect,
            command_panel::WIDTH,
            |ui| {
                command_panel::hint(ui, text(T::CalibrateStepPrint));
                ui.add_enabled_ui(!self.busy() && nator_kernel::is_available(), |ui| {
                    export = ui.button(text(T::CalibrateExport)).clicked();
                });
                command_panel::hint(ui, text(T::CalibrateStepHoles));
                for (nominal, measured) in CALIBRATION_HOLES.iter().zip(&mut draft.measured.holes) {
                    let label = format!("{} {nominal} mm", text(T::CalibrateHole));
                    command_panel::row(ui, &label, |ui| {
                        ui.add(
                            command_panel::localized(egui::DragValue::new(measured))
                                .speed(0.01)
                                .range(0.1..=20.)
                                .max_decimals(2)
                                .suffix(" mm"),
                        )
                    });
                }
                command_panel::hint(ui, text(T::CalibrateStepFits));
                for (slot, label) in [T::PrintFitTight, T::PrintFitSliding, T::PrintFitFree]
                    .into_iter()
                    .enumerate()
                {
                    command_panel::row(ui, text(label), |ui| {
                        egui::ComboBox::from_id_salt(("calibration_fit", slot))
                            .selected_text(fit_label(draft.measured.fits[slot]))
                            .show_ui(ui, |ui| {
                                for index in 0..CALIBRATION_FITS.len() {
                                    ui.selectable_value(
                                        &mut draft.measured.fits[slot],
                                        index,
                                        fit_label(index),
                                    );
                                }
                            });
                    });
                }
                match &result {
                    Ok(next) => {
                        let line = format!(
                            "{}: {:.2} mm",
                            text(T::PrintHoleCompensation),
                            next.hole_compensation_mm
                        );
                        ui.label(line.replace('.', ","));
                        ui.add_enabled_ui(!self.busy(), |ui| {
                            if ui.button(text(T::CalibrateApply)).clicked() {
                                apply = Some(next.clone());
                            }
                        });
                    }
                    Err(error) => {
                        ui.colored_label(theme::warning(), error.to_string());
                    }
                }
            },
        );
        if export {
            self.export_calibration(context, profile.bed_mm);
        }
        if let Some(profile) = apply {
            self.prepare_command(context, draft.tab, Command::SetPrintProfile { profile });
            return;
        }
        if !outcome.cancel {
            self.calibration = Some(draft);
        }
    }
}
