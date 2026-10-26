//! Confirmação da importação STL: o arquivo não informa a unidade, então o painel mostra as
//! dimensões lidas na unidade assumida e permite corrigir a escala antes de inserir o corpo
//! de malha. A malha continua sendo malha: não há conversão para sólido.
use super::{
    command_panel::{self as panel, Footer},
    icons::Icon,
    *,
};

/// Unidades comuns de arquivos STL e o fator para milímetros.
const UNITS: [(&str, f64); 4] = [("mm", 1.), ("cm", 10.), ("m", 1000.), ("in", 25.4)];

pub(super) struct MeshImportDraft {
    tab: u64,
    revision: u64,
    destination: Option<PartId>,
    name: String,
    /// Malha como lida, sem escala.
    mesh: Arc<Mesh>,
    /// Índice em `UNITS`, ou `None` para o fator personalizado.
    unit: Option<usize>,
    custom: f64,
}

impl MeshImportDraft {
    pub(super) fn new(
        tab: u64,
        revision: u64,
        destination: Option<PartId>,
        name: String,
        mesh: Mesh,
        custom: f64,
    ) -> Self {
        Self {
            tab,
            revision,
            destination,
            name,
            mesh: Arc::new(mesh),
            unit: Some(0),
            custom,
        }
    }
    fn scale(&self) -> f64 {
        self.unit.map(|i| UNITS[i].1).unwrap_or(self.custom)
    }
}

impl Editor {
    pub(super) fn mesh_import_panel(&mut self, context: &egui::Context) {
        let Some(mut draft) = self.mesh_import.take() else {
            return;
        };
        if draft.tab != self.current().id {
            // A importação pertence à aba de origem; trocar de aba a cancela.
            return;
        }
        let size = draft.mesh.bounds().size();
        let triangles = draft.mesh.triangles().len();
        let outcome = panel::show(
            context,
            panel::Header {
                id: "mesh_import",
                icon: Icon::Import,
                title: text(T::InsertMesh),
            },
            self.canvas_rect,
            Footer::None,
            !self.busy(),
            |ui| {
                ui.label(egui::RichText::new(&draft.name).strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{triangles} {}",
                        text(T::Triangles).to_lowercase()
                    ))
                    .small()
                    .color(theme::MUTED),
                );
                panel::row(ui, text(T::FileUnit), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.;
                    for (i, (name, _)) in UNITS.iter().enumerate() {
                        if ui.selectable_label(draft.unit == Some(i), *name).clicked() {
                            draft.unit = Some(i);
                        }
                    }
                    if ui
                        .selectable_label(draft.unit.is_none(), text(T::CustomScale))
                        .clicked()
                    {
                        draft.unit = None;
                    }
                });
                if draft.unit.is_none() {
                    panel::row(ui, text(T::Scale), |ui| {
                        ui.add(
                            egui::DragValue::new(&mut draft.custom)
                                .range(0.000_001..=1_000_000.)
                                .speed(0.01),
                        )
                    });
                }
                let unit = draft.unit.map(|i| UNITS[i].0).unwrap_or("×");
                panel::row(ui, text(T::FileSize), |ui| {
                    ui.monospace(format!(
                        "{} × {} × {} {unit}",
                        self.mm(size[0]),
                        self.mm(size[1]),
                        self.mm(size[2])
                    ))
                });
                let scale = draft.scale();
                panel::row(ui, text(T::ResultSize), |ui| {
                    ui.monospace(format!(
                        "{} × {} × {} mm",
                        self.mm(size[0] * scale),
                        self.mm(size[1] * scale),
                        self.mm(size[2] * scale)
                    ))
                });
                ui.label(
                    egui::RichText::new(text(T::MeshImportHint))
                        .small()
                        .color(theme::MUTED),
                );
            },
        );
        if outcome.cancel {
            return;
        }
        if outcome.ok {
            let scale = draft.scale();
            self.import_scale = draft.custom;
            let MeshImportDraft {
                tab,
                revision,
                destination,
                name,
                mesh,
                ..
            } = draft;
            // Escalar e analisar as arestas fica no trabalhador: malhas podem ser grandes.
            self.start(context, None, move || {
                let mesh = if (scale - 1.).abs() < 1e-15 {
                    Arc::unwrap_or_clone(mesh)
                } else {
                    Mesh::new(
                        mesh.vertices()
                            .iter()
                            .map(|p| p.map(|v| v * scale))
                            .collect(),
                        mesh.triangles().to_vec(),
                    )
                    .map_err(|e| e.to_string())?
                };
                Ok(Outcome::Imported {
                    tab,
                    revision,
                    destination,
                    name,
                    geometry: Arc::new(MeshAsset::new(mesh)),
                })
            });
            return;
        }
        self.mesh_import = Some(draft);
    }
}
