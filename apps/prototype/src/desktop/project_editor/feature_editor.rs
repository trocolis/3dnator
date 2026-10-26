//! Parâmetros da construção; operações nativas são preparadas fora da interface.
use super::*;
use nator_document::features::{BooleanMode, Feature, Operation, signature};
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
pub(super) struct Draft {
    tab: u64,
    part: PartId,
    output: Option<BodyId>,
    kind: usize,
    sketch: Option<SketchId>,
    base: Option<BodyId>,
    tool: Option<BodyId>,
    region: usize,
    index: usize,
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
            index: 0,
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
                d.region = region;
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
                d.region = region;
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
            Operation::Fillet {
                base, edge, radius, ..
            } => {
                d.kind = 5;
                d.base = Some(base);
                d.index = edge;
                d.to = radius;
            }
            Operation::Chamfer {
                base,
                edge,
                distance,
                ..
            } => {
                d.kind = 6;
                d.base = Some(base);
                d.index = edge;
                d.to = distance;
            }
            Operation::Shell {
                base,
                face,
                thickness,
                ..
            } => {
                d.kind = 7;
                d.base = Some(base);
                d.index = face;
                d.to = thickness;
            }
            Operation::Transform { .. } => {}
        }
        d
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
                    ui.label(text(T::Region));
                    ui.add(egui::DragValue::new(&mut d.region).range(0..=1024));
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
                        ui.label(text(T::TopologyIndex));
                        ui.add(egui::DragValue::new(&mut d.index).range(0..=100000));
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
                        let operation = match d.kind {
                            0 => Operation::Extrude {
                                sketch: d.sketch.unwrap_or(SketchId(0)),
                                region: d.region,
                                from: d.from.clone(),
                                to: d.to.clone(),
                            },
                            1 => Operation::Revolve {
                                sketch: d.sketch.unwrap_or(SketchId(0)),
                                region: d.region,
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
                                edge: d.index,
                                signature: stamp,
                                radius: d.to.clone(),
                            },
                            6 => Operation::Chamfer {
                                base,
                                edge: d.index,
                                signature: stamp,
                                distance: d.to.clone(),
                            },
                            _ => Operation::Shell {
                                base,
                                face: d.index,
                                signature: stamp,
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
