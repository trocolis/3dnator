//! Parâmetros da construção; operações nativas são preparadas fora da interface.
//!
//! A interface escolhe regiões, arestas e faces pela posição atual. Ao aplicar, o comando
//! converte essas posições em referências estáveis (entidades do rascunho e nomes de faces),
//! que sobrevivem a mudanças de cota. Ao editar uma operação existente, a referência gravada
//! é mantida enquanto o usuário não escolher outra região, aresta ou face.
use super::*;
use nator_document::features::{
    BooleanMode, EdgeRef, FaceRef, Feature, Operation, RegionRef, signature,
};
use nator_sketch::profiles;
const TOOLS: [T; 8] = [
    T::Extrude,
    T::Revolve,
    T::Join,
    T::Cut,
    T::Intersect,
    T::Fillet,
    T::Chamfer,
    T::Shell,
];

/// Referência topológica gravada na operação em edição.
enum Kept {
    Edge(EdgeRef),
    Face(FaceRef),
}

pub(super) struct Draft {
    tab: u64,
    part: PartId,
    output: Option<BodyId>,
    kind: usize,
    sketch: Option<SketchId>,
    base: Option<BodyId>,
    tool: Option<BodyId>,
    /// Posição na lista atual de regiões do rascunho selecionado.
    region: usize,
    kept_region: Option<RegionRef>,
    /// Índice de aresta/face no corpo base, quando o usuário escolhe uma nova seleção.
    index: usize,
    kept_topology: Option<Kept>,
    from: String,
    to: String,
    angle: String,
}
impl Draft {
    fn new(tab: u64, part: PartId) -> Self {
        Self {
            tab,
            part,
            output: None,
            kind: 0,
            sketch: None,
            base: None,
            tool: None,
            region: 0,
            kept_region: None,
            index: 0,
            kept_topology: None,
            from: "0 mm".into(),
            to: "10 mm".into(),
            angle: "360 deg".into(),
        }
    }
    pub fn edit(tab: u64, part: PartId, feature: Feature) -> Self {
        let mut d = Self::new(tab, part);
        d.output = Some(feature.output);
        match feature.operation {
            Operation::Extrude {
                sketch,
                region,
                from,
                to,
            } => {
                d.sketch = Some(sketch);
                d.kept_region = Some(region);
                d.from = from;
                d.to = to;
            }
            Operation::Revolve {
                sketch,
                region,
                angle,
                ..
            } => {
                d.kind = 1;
                d.sketch = Some(sketch);
                d.kept_region = Some(region);
                d.angle = angle;
            }
            Operation::Boolean { base, tool, mode } => {
                d.kind = match mode {
                    BooleanMode::Join => 2,
                    BooleanMode::Cut => 3,
                    BooleanMode::Intersect => 4,
                };
                d.base = Some(base);
                d.tool = Some(tool);
            }
            Operation::Fillet { base, edge, radius } => {
                d.kind = 5;
                d.base = Some(base);
                d.kept_topology = Some(Kept::Edge(edge));
                d.to = radius;
            }
            Operation::Chamfer {
                base,
                edge,
                distance,
            } => {
                d.kind = 6;
                d.base = Some(base);
                d.kept_topology = Some(Kept::Edge(edge));
                d.to = distance;
            }
            Operation::Shell {
                base,
                face,
                thickness,
            } => {
                d.kind = 7;
                d.base = Some(base);
                d.kept_topology = Some(Kept::Face(face));
                d.to = thickness;
            }
            Operation::Transform { .. } => {}
        }
        d
    }
}

/// Descrição curta de uma referência para o usuário reconhecer o que está gravado.
fn describe_region(region: &RegionRef) -> String {
    match region {
        RegionRef::Boundary(ids) => format!("{} ({} entidades)", text(T::KeptReference), ids.len()),
        RegionRef::Index(i) => format!("{} (#{})", text(T::KeptReference), i + 1),
    }
}
fn describe_topology(kept: &Kept) -> String {
    match kept {
        Kept::Edge(EdgeRef::Between([a, b])) => format!("{a} × {b}"),
        Kept::Face(FaceRef::Named(name)) => name.clone(),
        Kept::Edge(EdgeRef::Index { index, .. }) | Kept::Face(FaceRef::Index { index, .. }) => {
            format!("#{index}")
        }
    }
}

impl Editor {
    pub(super) fn open_features(&mut self) {
        if let Some(part) = self.current().active_part {
            let mut d = Draft::new(self.current().id, part);
            d.sketch = self.current().sketch_edit.as_ref().map(|w| w.id);
            self.feature_edit = Some(d);
        }
    }
    pub(super) fn feature_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.feature_edit.take() else {
            return;
        };
        let Some(part) = self
            .tabs
            .iter()
            .find(|t| t.id == d.tab)
            .and_then(|t| t.session.document().part(d.part))
            .cloned()
        else {
            return;
        };
        // Regiões atuais do rascunho escolhido; um perfil inválido mostra o diagnóstico.
        let regions = d
            .sketch
            .and_then(|id| part.sketches.iter().find(|s| s.id == id))
            .map(|s| profiles::regions(&s.sketch).map_err(|e| e.to_string()));
        let mut open = true;
        let mut command = None;
        egui::Window::new(text(T::SolidTools))
            .open(&mut open)
            .show(ctx, |ui| {
                if !nator_kernel::is_available() {
                    ui.colored_label(egui::Color32::YELLOW, text(T::KernelUnavailable));
                }
                egui::ComboBox::from_label(text(T::Operation))
                    .selected_text(text(TOOLS[d.kind]))
                    .show_ui(ui, |ui| {
                        for (i, t) in TOOLS.iter().enumerate() {
                            ui.selectable_value(&mut d.kind, i, text(*t));
                        }
                    });
                if d.kind < 2 {
                    let previous = d.sketch;
                    egui::ComboBox::from_label(text(T::Profile))
                        .selected_text(
                            d.sketch
                                .and_then(|id| part.sketches.iter().find(|s| s.id == id))
                                .map(|s| s.sketch.name.as_str())
                                .unwrap_or("—"),
                        )
                        .show_ui(ui, |ui| {
                            for s in &part.sketches {
                                ui.selectable_value(&mut d.sketch, Some(s.id), &s.sketch.name);
                            }
                        });
                    if d.sketch != previous {
                        d.kept_region = None;
                        d.region = 0;
                    }
                    match &regions {
                        Some(Ok(list)) if !list.is_empty() => {
                            let label = |i: usize| {
                                let r = &list[i];
                                format!(
                                    "{} {} · {:.1} mm² · {} {}",
                                    text(T::Region),
                                    i + 1,
                                    r.area(),
                                    r.holes.len(),
                                    text(T::RegionHoles)
                                )
                            };
                            let selected = match &d.kept_region {
                                Some(kept) => describe_region(kept),
                                None => label(d.region.min(list.len() - 1)),
                            };
                            egui::ComboBox::from_label(text(T::Region))
                                .selected_text(selected)
                                .show_ui(ui, |ui| {
                                    for i in 0..list.len() {
                                        if ui
                                            .selectable_label(
                                                d.kept_region.is_none() && d.region == i,
                                                label(i),
                                            )
                                            .clicked()
                                        {
                                            d.region = i;
                                            d.kept_region = None;
                                        }
                                    }
                                });
                        }
                        Some(Err(message)) => {
                            ui.colored_label(egui::Color32::YELLOW, message);
                        }
                        _ => {
                            ui.colored_label(egui::Color32::YELLOW, text(T::NoRegions));
                        }
                    }
                    if d.kind == 0 {
                        ui.label(text(T::StartLimit));
                        ui.text_edit_singleline(&mut d.from);
                    } else {
                        ui.label(text(T::Angle));
                        ui.text_edit_singleline(&mut d.angle);
                    }
                } else {
                    for (label, slot) in [(T::SourceBody, &mut d.base), (T::ToolBody, &mut d.tool)]
                        .into_iter()
                        .take(if d.kind < 5 { 2 } else { 1 })
                    {
                        egui::ComboBox::from_label(text(label))
                            .selected_text(
                                slot.and_then(|id| part.bodies.iter().find(|b| b.id == id))
                                    .map(|b| b.name.as_str())
                                    .unwrap_or("—"),
                            )
                            .show_ui(ui, |ui| {
                                for b in part
                                    .bodies
                                    .iter()
                                    .filter(|b| b.brep.is_some() && Some(b.id) != d.output)
                                {
                                    ui.selectable_value(slot, Some(b.id), &b.name);
                                }
                            });
                    }
                    if d.kind >= 5 {
                        if let Some(kept) = &d.kept_topology {
                            ui.label(format!(
                                "{}: {}",
                                text(T::KeptReference),
                                describe_topology(kept)
                            ));
                        }
                        ui.label(text(T::TopologyIndex));
                        if ui
                            .add(egui::DragValue::new(&mut d.index).range(0..=100000))
                            .changed()
                        {
                            d.kept_topology = None;
                        }
                    }
                }
                if d.kind == 0 || d.kind >= 5 {
                    ui.label(text(T::EndLimit));
                    ui.text_edit_singleline(&mut d.to);
                }
                ui.add_enabled_ui(!self.busy() && nator_kernel::is_available(), |ui| {
                    if ui.button(text(T::Apply)).clicked() {
                        let base = d.base.unwrap_or(BodyId(0));
                        let stamp = part
                            .bodies
                            .iter()
                            .find(|b| b.id == base)
                            .and_then(|b| b.brep.as_ref())
                            .map(|b| signature(b))
                            .unwrap_or(0);
                        // Nova seleção: a posição atual vira identidade estável no comando.
                        let region = d.kept_region.clone().unwrap_or_else(|| match &regions {
                            Some(Ok(list)) if d.region < list.len() => {
                                RegionRef::Boundary(list[d.region].boundary())
                            }
                            _ => RegionRef::Index(d.region),
                        });
                        let edge = match &d.kept_topology {
                            Some(Kept::Edge(edge)) => edge.clone(),
                            _ => EdgeRef::Index {
                                index: d.index,
                                signature: stamp,
                            },
                        };
                        let face = match &d.kept_topology {
                            Some(Kept::Face(face)) => face.clone(),
                            _ => FaceRef::Index {
                                index: d.index,
                                signature: stamp,
                            },
                        };
                        let operation = match d.kind {
                            0 => Operation::Extrude {
                                sketch: d.sketch.unwrap_or(SketchId(0)),
                                region,
                                from: d.from.clone(),
                                to: d.to.clone(),
                            },
                            1 => Operation::Revolve {
                                sketch: d.sketch.unwrap_or(SketchId(0)),
                                region,
                                axis_origin: [0.; 3],
                                axis_direction: [0., 1., 0.],
                                angle: d.angle.clone(),
                            },
                            2..=4 => Operation::Boolean {
                                base,
                                tool: d.tool.unwrap_or(BodyId(0)),
                                mode: match d.kind {
                                    2 => BooleanMode::Join,
                                    3 => BooleanMode::Cut,
                                    _ => BooleanMode::Intersect,
                                },
                            },
                            5 => Operation::Fillet {
                                base,
                                edge,
                                radius: d.to.clone(),
                            },
                            6 => Operation::Chamfer {
                                base,
                                edge,
                                distance: d.to.clone(),
                            },
                            _ => Operation::Shell {
                                base,
                                face,
                                thickness: d.to.clone(),
                            },
                        };
                        command = Some(if let Some(output) = d.output {
                            Command::EditFeature {
                                part: d.part,
                                output,
                                operation,
                            }
                        } else {
                            Command::AddFeature {
                                part: d.part,
                                name: text(TOOLS[d.kind]).into(),
                                operation,
                            }
                        });
                    }
                    if let Some(output) = d.output
                        && ui.button(text(T::RemoveFeature)).clicked()
                    {
                        command = Some(Command::RemoveFeature {
                            part: d.part,
                            output,
                        });
                    }
                });
            });
        if let Some(command) = command {
            self.prepare_command(ctx, d.tab, command);
        }
        if open {
            self.feature_edit = Some(d);
        }
    }
}
