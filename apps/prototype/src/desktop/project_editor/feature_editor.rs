//! Parâmetros da construção; operações nativas são preparadas fora da interface.
//!
//! A interface escolhe regiões, arestas e faces pela posição atual ou por clique na área
//! 3D. Ao aplicar, o comando converte essas escolhas em referências estáveis (entidades do
//! rascunho e nomes de faces), que sobrevivem a mudanças de cota. Ao editar uma operação
//! existente, a referência gravada é mantida enquanto o usuário não escolher outra.
use super::*;
use nator_document::features::{
    BooleanMode, EdgeRef, FaceRef, Feature, HoleHeadSpec, HoleSpec, LinearAxis, Operation,
    PatternLayout, RegionRef, ThreadSpec, signature,
};
use nator_sketch::{Curve, Sketch, profiles};

/// Ferramentas do diálogo, na ordem exibida.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Tool {
    Extrude,
    Revolve,
    Join,
    Cut,
    Intersect,
    Fillet,
    Chamfer,
    Shell,
    Hole,
    LinearPattern,
    CircularPattern,
    Mirror,
    Split,
}
const TOOLS: [Tool; 13] = [
    Tool::Extrude,
    Tool::Revolve,
    Tool::Join,
    Tool::Cut,
    Tool::Intersect,
    Tool::Fillet,
    Tool::Chamfer,
    Tool::Shell,
    Tool::Hole,
    Tool::LinearPattern,
    Tool::CircularPattern,
    Tool::Mirror,
    Tool::Split,
];
impl Tool {
    fn label(self) -> T {
        match self {
            Self::Extrude => T::Extrude,
            Self::Revolve => T::Revolve,
            Self::Join => T::Join,
            Self::Cut => T::Cut,
            Self::Intersect => T::Intersect,
            Self::Fillet => T::Fillet,
            Self::Chamfer => T::Chamfer,
            Self::Shell => T::Shell,
            Self::Hole => T::HoleTool,
            Self::LinearPattern => T::LinearPattern,
            Self::CircularPattern => T::CircularPattern,
            Self::Mirror => T::MirrorBody,
            Self::Split => T::SplitBody,
        }
    }
    /// Ferramentas que partem de um rascunho de perfil.
    fn uses_profile(self) -> bool {
        matches!(self, Self::Extrude | Self::Revolve)
    }
    fn uses_topology(self) -> bool {
        matches!(self, Self::Fillet | Self::Chamfer | Self::Shell)
    }
}

/// Eixos e planos principais da peça, usados por padrões, espelhamento e divisão.
const AXES: [(&str, [f64; 3]); 3] = [
    ("X", [1., 0., 0.]),
    ("Y", [0., 1., 0.]),
    ("Z", [0., 0., 1.]),
];
/// Planos pela normal: XY (normal Z), XZ (normal Y), YZ (normal X).
const PLANES: [(&str, [f64; 3]); 3] = [
    ("XY", [0., 0., 1.]),
    ("XZ", [0., 1., 0.]),
    ("YZ", [1., 0., 0.]),
];

/// Referência topológica gravada na operação em edição ou escolhida na área 3D.
pub(super) enum Kept {
    Edge(EdgeRef),
    Face(FaceRef),
}

/// O que o próximo clique na área 3D deve selecionar para o diálogo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Awaiting {
    Edge,
    Face,
}

/// Eixo da revolução: um eixo do plano do rascunho ou uma linha do rascunho.
#[derive(Debug, Clone, Copy, PartialEq)]
enum RevolveAxis {
    U,
    V,
    Line(u64),
}

/// Medidas do furo como digitadas, com expressões.
struct HoleDraft {
    diameter: String,
    depth: String,
    through: bool,
    head: usize,
    head_diameter: String,
    head_depth: String,
    head_angle: String,
    tip: bool,
    tip_angle: String,
    reverse: bool,
    thread: bool,
    thread_designation: String,
    thread_nominal: String,
    thread_pitch: String,
    /// Pontos gravados na operação em edição; mantidos enquanto o rascunho não mudar.
    kept_points: Option<Vec<u64>>,
}
impl Default for HoleDraft {
    fn default() -> Self {
        Self {
            diameter: "6 mm".into(),
            depth: "10 mm".into(),
            through: true,
            head: 0,
            head_diameter: "11 mm".into(),
            head_depth: "6 mm".into(),
            head_angle: "90 deg".into(),
            tip: true,
            tip_angle: "118 deg".into(),
            reverse: false,
            thread: false,
            thread_designation: "M6".into(),
            thread_nominal: "6 mm".into(),
            thread_pitch: "1 mm".into(),
            kept_points: None,
        }
    }
}

/// Padrões, espelhamento e divisão.
struct LayoutDraft {
    axis: usize,
    count: u32,
    spacing: String,
    second: bool,
    second_axis: usize,
    second_count: u32,
    second_spacing: String,
    origin: [f64; 3],
    angle: String,
    plane: usize,
    offset: f64,
    keep_original: bool,
    flip: bool,
}
impl Default for LayoutDraft {
    fn default() -> Self {
        Self {
            axis: 0,
            count: 3,
            spacing: "20 mm".into(),
            second: false,
            second_axis: 1,
            second_count: 2,
            second_spacing: "20 mm".into(),
            origin: [0.; 3],
            angle: "360 deg".into(),
            plane: 2,
            offset: 0.,
            keep_original: true,
            flip: false,
        }
    }
}

pub(super) struct Draft {
    tab: u64,
    part: PartId,
    output: Option<BodyId>,
    tool: Tool,
    sketch: Option<SketchId>,
    base: Option<BodyId>,
    tool_body: Option<BodyId>,
    /// Posição na lista atual de regiões do rascunho selecionado.
    region: usize,
    kept_region: Option<RegionRef>,
    axis: RevolveAxis,
    /// Eixo fixo gravado em uma revolução antiga, mantido até outra escolha.
    kept_axis: Option<([f64; 3], [f64; 3])>,
    /// Índice de aresta/face no corpo base, quando o usuário escolhe uma nova seleção.
    index: usize,
    kept_topology: Option<Kept>,
    /// Arestas/faces adicionais da mesma operação (arredondamento, chanfro, casca).
    extra_topology: Vec<Kept>,
    /// O próximo clique na área 3D escolhe a aresta/face da operação.
    selecting: bool,
    /// O próximo clique acrescenta uma aresta/face em vez de substituir a principal.
    appending: bool,
    /// Chanfro por distância e ângulo, com o ângulo em expressão.
    chamfer_angle: Option<String>,
    /// Casca com paredes para fora do sólido original.
    outward: bool,
    from: String,
    to: String,
    angle: String,
    hole: HoleDraft,
    layout: LayoutDraft,
}
impl Draft {
    fn new(tab: u64, part: PartId) -> Self {
        Self {
            tab,
            part,
            output: None,
            tool: Tool::Extrude,
            sketch: None,
            base: None,
            tool_body: None,
            region: 0,
            kept_region: None,
            axis: RevolveAxis::V,
            kept_axis: None,
            index: 0,
            kept_topology: None,
            extra_topology: Vec::new(),
            selecting: false,
            appending: false,
            chamfer_angle: None,
            outward: false,
            from: "0 mm".into(),
            to: "10 mm".into(),
            angle: "360 deg".into(),
            hole: HoleDraft::default(),
            layout: LayoutDraft::default(),
        }
    }
    pub fn edit(tab: u64, part: PartId, feature: Feature) -> Self {
        let mut d = Self::new(tab, part);
        d.output = Some(feature.output);
        let nearest = |direction: [f64; 3], table: &[(&str, [f64; 3])]| {
            table
                .iter()
                .position(|(_, v)| nator_geometry::dot(*v, direction).abs() > 0.999)
                .unwrap_or(0)
        };
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
                axis_line,
                axis_origin,
                axis_direction,
            } => {
                d.tool = Tool::Revolve;
                d.sketch = Some(sketch);
                d.kept_region = Some(region);
                d.angle = angle;
                match axis_line {
                    Some(line) => d.axis = RevolveAxis::Line(line),
                    None => d.kept_axis = Some((axis_origin, axis_direction)),
                }
            }
            Operation::Boolean { base, tool, mode } => {
                d.tool = match mode {
                    BooleanMode::Join => Tool::Join,
                    BooleanMode::Cut => Tool::Cut,
                    BooleanMode::Intersect => Tool::Intersect,
                };
                d.base = Some(base);
                d.tool_body = Some(tool);
            }
            Operation::Fillet {
                base,
                edge,
                radius,
                additional_edges,
            } => {
                d.tool = Tool::Fillet;
                d.base = Some(base);
                d.kept_topology = Some(Kept::Edge(edge));
                d.extra_topology = additional_edges.into_iter().map(Kept::Edge).collect();
                d.to = radius;
            }
            Operation::Chamfer {
                base,
                edge,
                distance,
                angle,
                additional_edges,
            } => {
                d.tool = Tool::Chamfer;
                d.base = Some(base);
                d.kept_topology = Some(Kept::Edge(edge));
                d.extra_topology = additional_edges.into_iter().map(Kept::Edge).collect();
                d.to = distance;
                d.chamfer_angle = angle;
            }
            Operation::Shell {
                base,
                face,
                thickness,
                additional_faces,
                outward,
            } => {
                d.tool = Tool::Shell;
                d.base = Some(base);
                d.kept_topology = Some(Kept::Face(face));
                d.extra_topology = additional_faces.into_iter().map(Kept::Face).collect();
                d.to = thickness;
                d.outward = outward;
            }
            Operation::Hole {
                base,
                sketch,
                points,
                spec,
            } => {
                d.tool = Tool::Hole;
                d.base = Some(base);
                d.sketch = Some(sketch);
                let h = &mut d.hole;
                h.kept_points = Some(points);
                h.diameter = spec.diameter;
                h.through = spec.depth.is_none();
                if let Some(depth) = spec.depth {
                    h.depth = depth;
                }
                match spec.head {
                    HoleHeadSpec::Simple => h.head = 0,
                    HoleHeadSpec::Counterbore { diameter, depth } => {
                        h.head = 1;
                        h.head_diameter = diameter;
                        h.head_depth = depth;
                    }
                    HoleHeadSpec::Countersink { diameter, angle } => {
                        h.head = 2;
                        h.head_diameter = diameter;
                        h.head_angle = angle;
                    }
                }
                h.tip = spec.tip_angle.is_some();
                if let Some(angle) = spec.tip_angle {
                    h.tip_angle = angle;
                }
                h.reverse = spec.reverse;
                if let Some(thread) = spec.thread {
                    h.thread = true;
                    h.thread_designation = thread.designation;
                    h.thread_nominal = thread.nominal;
                    h.thread_pitch = thread.pitch;
                }
            }
            Operation::Pattern { base, layout } => {
                d.base = Some(base);
                let l = &mut d.layout;
                match layout {
                    PatternLayout::Linear { first, second } => {
                        d.tool = Tool::LinearPattern;
                        l.axis = nearest(first.direction, &AXES);
                        l.count = first.count;
                        l.spacing = first.spacing;
                        if let Some(second) = second {
                            l.second = true;
                            l.second_axis = nearest(second.direction, &AXES);
                            l.second_count = second.count;
                            l.second_spacing = second.spacing;
                        }
                    }
                    PatternLayout::Circular {
                        axis_origin,
                        axis_direction,
                        count,
                        angle,
                    } => {
                        d.tool = Tool::CircularPattern;
                        l.axis = nearest(axis_direction, &AXES);
                        l.origin = axis_origin;
                        l.count = count;
                        l.angle = angle;
                    }
                }
            }
            Operation::Mirror {
                base,
                origin,
                normal,
                keep_original,
            } => {
                d.tool = Tool::Mirror;
                d.base = Some(base);
                d.layout.plane = nearest(normal, &PLANES);
                d.layout.offset = nator_geometry::dot(origin, PLANES[d.layout.plane].1);
                d.layout.keep_original = keep_original;
            }
            Operation::Split {
                base,
                origin,
                normal,
            } => {
                d.tool = Tool::Split;
                d.base = Some(base);
                d.layout.plane = nearest(normal, &PLANES);
                let axis = PLANES[d.layout.plane].1;
                d.layout.offset = nator_geometry::dot(origin, axis);
                d.layout.flip = nator_geometry::dot(normal, axis) < 0.;
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

/// Pontos de um rascunho que posicionam furos: centros de círculos e pontos isolados.
fn hole_points(sketch: &Sketch) -> Vec<u64> {
    let used = sketch
        .entities
        .iter()
        .flat_map(|e| e.point_ids())
        .collect::<std::collections::HashSet<_>>();
    let centers = sketch.entities.iter().filter_map(|e| match e.curve {
        Curve::Circle { center, .. } if !e.construction => Some(center),
        _ => None,
    });
    let isolated = sketch
        .points
        .iter()
        .map(|p| p.id)
        .filter(|id| !used.contains(id));
    let mut points = centers.chain(isolated).collect::<Vec<_>>();
    points.sort_unstable();
    points.dedup();
    points
}

impl Draft {
    /// Tipo de elemento que o próximo clique deve selecionar, se o diálogo estiver esperando.
    pub(super) fn awaiting(&self) -> Option<Awaiting> {
        let waiting = self.selecting || self.appending;
        match self.tool {
            Tool::Fillet | Tool::Chamfer if waiting => Some(Awaiting::Edge),
            Tool::Shell if waiting => Some(Awaiting::Face),
            _ => None,
        }
    }
    pub(super) fn part(&self) -> PartId {
        self.part
    }
    /// Aplica uma seleção gráfica: o corpo clicado passa a ser a base da operação.
    pub(super) fn apply_pick(&mut self, body: BodyId, kept: Kept) {
        if self.appending && self.base == Some(body) {
            self.extra_topology.push(kept);
        } else {
            // Outro corpo, ou seleção principal: as escolhas anteriores deixam de valer.
            self.base = Some(body);
            self.kept_topology = Some(kept);
            self.extra_topology.clear();
        }
        self.selecting = false;
        self.appending = false;
    }
    pub(super) fn describe(kept: &Kept) -> String {
        describe_topology(kept)
    }

    fn extra_edges(&self) -> Vec<EdgeRef> {
        self.extra_topology
            .iter()
            .filter_map(|k| match k {
                Kept::Edge(edge) => Some(edge.clone()),
                Kept::Face(_) => None,
            })
            .collect()
    }

    fn hole_spec(&self) -> HoleSpec {
        let h = &self.hole;
        HoleSpec {
            diameter: h.diameter.clone(),
            depth: (!h.through).then(|| h.depth.clone()),
            head: match h.head {
                1 => HoleHeadSpec::Counterbore {
                    diameter: h.head_diameter.clone(),
                    depth: h.head_depth.clone(),
                },
                2 => HoleHeadSpec::Countersink {
                    diameter: h.head_diameter.clone(),
                    angle: h.head_angle.clone(),
                },
                _ => HoleHeadSpec::Simple,
            },
            tip_angle: (h.tip && !h.through).then(|| h.tip_angle.clone()),
            reverse: h.reverse,
            thread: h.thread.then(|| ThreadSpec {
                designation: h.thread_designation.clone(),
                nominal: h.thread_nominal.clone(),
                pitch: h.thread_pitch.clone(),
                depth: None,
            }),
        }
    }

    fn plane(&self) -> ([f64; 3], [f64; 3]) {
        let normal = PLANES[self.layout.plane].1;
        let origin = normal.map(|v| v * self.layout.offset);
        (
            origin,
            if self.layout.flip {
                normal.map(|v| -v)
            } else {
                normal
            },
        )
    }

    /// Monta a operação a partir do que está no diálogo.
    fn operation(
        &self,
        part: &nator_document::Part,
        regions: &Option<Result<Vec<profiles::Region>, String>>,
    ) -> Operation {
        let base = self.base.unwrap_or(BodyId(0));
        let sketch = self.sketch.unwrap_or(SketchId(0));
        let stamp = part
            .bodies
            .iter()
            .find(|b| b.id == base)
            .and_then(|b| b.brep.as_ref())
            .map(|b| signature(b))
            .unwrap_or(0);
        // Nova seleção: a posição atual vira identidade estável no comando.
        let region = self.kept_region.clone().unwrap_or_else(|| match regions {
            Some(Ok(list)) if self.region < list.len() => {
                RegionRef::Boundary(list[self.region].boundary())
            }
            _ => RegionRef::Index(self.region),
        });
        let edge = match &self.kept_topology {
            Some(Kept::Edge(edge)) => edge.clone(),
            _ => EdgeRef::Index {
                index: self.index,
                signature: stamp,
            },
        };
        let face = match &self.kept_topology {
            Some(Kept::Face(face)) => face.clone(),
            _ => FaceRef::Index {
                index: self.index,
                signature: stamp,
            },
        };
        let record = part.sketches.iter().find(|s| s.id == sketch);
        match self.tool {
            Tool::Extrude => Operation::Extrude {
                sketch,
                region,
                from: self.from.clone(),
                to: self.to.clone(),
            },
            Tool::Revolve => {
                let plane = record.map(|r| r.sketch.plane).unwrap_or_default();
                let (axis_origin, axis_direction) = self.kept_axis.unwrap_or(match self.axis {
                    RevolveAxis::U => (plane.origin, plane.u),
                    _ => (plane.origin, plane.v),
                });
                Operation::Revolve {
                    sketch,
                    region,
                    axis_origin,
                    axis_direction,
                    angle: self.angle.clone(),
                    axis_line: match self.axis {
                        RevolveAxis::Line(line) if self.kept_axis.is_none() => Some(line),
                        _ => None,
                    },
                }
            }
            Tool::Join | Tool::Cut | Tool::Intersect => Operation::Boolean {
                base,
                tool: self.tool_body.unwrap_or(BodyId(0)),
                mode: match self.tool {
                    Tool::Join => BooleanMode::Join,
                    Tool::Cut => BooleanMode::Cut,
                    _ => BooleanMode::Intersect,
                },
            },
            Tool::Fillet => Operation::Fillet {
                base,
                edge,
                radius: self.to.clone(),
                additional_edges: self.extra_edges(),
            },
            Tool::Chamfer => Operation::Chamfer {
                base,
                edge,
                distance: self.to.clone(),
                angle: self.chamfer_angle.clone(),
                additional_edges: self.extra_edges(),
            },
            Tool::Shell => Operation::Shell {
                base,
                face,
                thickness: self.to.clone(),
                additional_faces: self
                    .extra_topology
                    .iter()
                    .filter_map(|k| match k {
                        Kept::Face(face) => Some(face.clone()),
                        Kept::Edge(_) => None,
                    })
                    .collect(),
                outward: self.outward,
            },
            Tool::Hole => {
                Operation::Hole {
                    base,
                    sketch,
                    points: self.hole.kept_points.clone().unwrap_or_else(|| {
                        record.map(|r| hole_points(&r.sketch)).unwrap_or_default()
                    }),
                    spec: self.hole_spec(),
                }
            }
            Tool::LinearPattern => {
                let l = &self.layout;
                Operation::Pattern {
                    base,
                    layout: PatternLayout::Linear {
                        first: LinearAxis {
                            direction: AXES[l.axis].1,
                            count: l.count,
                            spacing: l.spacing.clone(),
                        },
                        second: l.second.then(|| LinearAxis {
                            direction: AXES[l.second_axis].1,
                            count: l.second_count,
                            spacing: l.second_spacing.clone(),
                        }),
                    },
                }
            }
            Tool::CircularPattern => {
                let l = &self.layout;
                Operation::Pattern {
                    base,
                    layout: PatternLayout::Circular {
                        axis_origin: l.origin,
                        axis_direction: AXES[l.axis].1,
                        count: l.count,
                        angle: l.angle.clone(),
                    },
                }
            }
            Tool::Mirror => {
                let (origin, normal) = self.plane();
                Operation::Mirror {
                    base,
                    origin,
                    normal,
                    keep_original: self.layout.keep_original,
                }
            }
            Tool::Split => {
                let (origin, normal) = self.plane();
                Operation::Split {
                    base,
                    origin,
                    normal,
                }
            }
        }
    }
}

/// Lista de seleção de um eixo/plano principal.
fn choose(
    ui: &mut egui::Ui,
    label: T,
    value: &mut usize,
    options: &[(&str, [f64; 3])],
    salt: &str,
) {
    egui::ComboBox::new(salt, text(label))
        .selected_text(options[*value].0)
        .show_ui(ui, |ui| {
            for (i, (name, _)) in options.iter().enumerate() {
                ui.selectable_value(value, i, *name);
            }
        });
}

fn labeled_text(ui: &mut egui::Ui, label: T, value: &mut String) {
    ui.horizontal(|ui| {
        ui.label(text(label));
        ui.text_edit_singleline(value);
    });
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
        let record = d
            .sketch
            .and_then(|id| part.sketches.iter().find(|s| s.id == id));
        let regions = record
            .filter(|_| d.tool.uses_profile())
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
                    .selected_text(text(d.tool.label()))
                    .show_ui(ui, |ui| {
                        for tool in TOOLS {
                            ui.selectable_value(&mut d.tool, tool, text(tool.label()));
                        }
                    });
                if d.tool.uses_profile() || d.tool == Tool::Hole {
                    let previous = d.sketch;
                    egui::ComboBox::from_label(text(T::Profile))
                        .selected_text(record.map(|s| s.sketch.name.as_str()).unwrap_or("—"))
                        .show_ui(ui, |ui| {
                            for s in &part.sketches {
                                ui.selectable_value(&mut d.sketch, Some(s.id), &s.sketch.name);
                            }
                        });
                    if d.sketch != previous {
                        d.kept_region = None;
                        d.region = 0;
                        d.hole.kept_points = None;
                        d.kept_axis = None;
                        d.axis = RevolveAxis::V;
                    }
                }
                if d.tool.uses_profile() {
                    region_controls(ui, &mut d, &regions);
                }
                match d.tool {
                    Tool::Extrude => {
                        labeled_text(ui, T::StartLimit, &mut d.from);
                        labeled_text(ui, T::EndLimit, &mut d.to);
                    }
                    Tool::Revolve => {
                        revolve_controls(ui, &mut d, record.map(|r| &r.sketch));
                        labeled_text(ui, T::Angle, &mut d.angle);
                    }
                    _ => {}
                }
                if !d.tool.uses_profile() {
                    body_controls(ui, &mut d, &part);
                }
                if d.tool.uses_topology() {
                    if let Some(kept) = &d.kept_topology {
                        ui.label(format!(
                            "{}: {}",
                            text(T::KeptReference),
                            describe_topology(kept)
                        ));
                    }
                    let mut remove = None;
                    for (i, kept) in d.extra_topology.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("+ {}", describe_topology(kept)));
                            if ui.small_button("×").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove {
                        d.extra_topology.remove(i);
                    }
                    if d.selecting || d.appending {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, text(T::PickingTopology));
                    } else {
                        ui.horizontal(|ui| {
                            if ui.button(text(T::PickInViewport)).clicked() {
                                d.selecting = true;
                            }
                            if d.kept_topology.is_some()
                                && ui.button(text(T::PickAnother)).clicked()
                            {
                                d.appending = true;
                            }
                        });
                    }
                    if d.tool == Tool::Chamfer {
                        let mut by_angle = d.chamfer_angle.is_some();
                        if ui
                            .checkbox(&mut by_angle, text(T::ChamferByAngle))
                            .changed()
                        {
                            d.chamfer_angle = by_angle.then(|| "45 deg".into());
                        }
                        if let Some(angle) = &mut d.chamfer_angle {
                            labeled_text(ui, T::Angle, angle);
                        }
                    }
                    if d.tool == Tool::Shell {
                        ui.checkbox(&mut d.outward, text(T::ShellOutward));
                    }
                    ui.label(text(T::TopologyIndex));
                    if ui
                        .add(egui::DragValue::new(&mut d.index).range(0..=100000))
                        .changed()
                    {
                        d.kept_topology = None;
                    }
                    labeled_text(ui, T::EndLimit, &mut d.to);
                }
                match d.tool {
                    Tool::Hole => hole_controls(ui, &mut d, record.map(|r| &r.sketch)),
                    Tool::LinearPattern => {
                        let l = &mut d.layout;
                        choose(ui, T::PatternDirection, &mut l.axis, &AXES, "pattern_axis");
                        ui.add(
                            egui::DragValue::new(&mut l.count)
                                .range(2..=1000)
                                .prefix(format!("{} ", text(T::PatternCount))),
                        );
                        labeled_text(ui, T::PatternSpacing, &mut l.spacing);
                        ui.checkbox(&mut l.second, text(T::SecondDirection));
                        if l.second {
                            choose(
                                ui,
                                T::PatternDirection,
                                &mut l.second_axis,
                                &AXES,
                                "pattern_axis_2",
                            );
                            ui.add(
                                egui::DragValue::new(&mut l.second_count)
                                    .range(1..=1000)
                                    .prefix(format!("{} ", text(T::PatternCount))),
                            );
                            labeled_text(ui, T::PatternSpacing, &mut l.second_spacing);
                        }
                    }
                    Tool::CircularPattern => {
                        let l = &mut d.layout;
                        choose(ui, T::RevolveAxis, &mut l.axis, &AXES, "circular_axis");
                        ui.horizontal(|ui| {
                            ui.label(text(T::AxisOrigin));
                            for v in &mut l.origin {
                                ui.add(egui::DragValue::new(v).speed(0.5));
                            }
                        });
                        ui.add(
                            egui::DragValue::new(&mut l.count)
                                .range(2..=1000)
                                .prefix(format!("{} ", text(T::PatternCount))),
                        );
                        labeled_text(ui, T::TotalAngle, &mut l.angle);
                    }
                    Tool::Mirror | Tool::Split => {
                        let l = &mut d.layout;
                        choose(ui, T::PlaneChoice, &mut l.plane, &PLANES, "mirror_plane");
                        ui.horizontal(|ui| {
                            ui.label(text(T::PlaneOffset));
                            ui.add(egui::DragValue::new(&mut l.offset).speed(0.5));
                        });
                        if d.tool == Tool::Mirror {
                            ui.checkbox(&mut l.keep_original, text(T::KeepOriginal));
                        } else {
                            ui.checkbox(&mut l.flip, text(T::FlipSide));
                        }
                    }
                    _ => {}
                }
                ui.add_enabled_ui(!self.busy() && nator_kernel::is_available(), |ui| {
                    if ui.button(text(T::Apply)).clicked() {
                        let operation = d.operation(&part, &regions);
                        command = Some(if let Some(output) = d.output {
                            Command::EditFeature {
                                part: d.part,
                                output,
                                operation,
                            }
                        } else {
                            Command::AddFeature {
                                part: d.part,
                                name: text(d.tool.label()).into(),
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

fn region_controls(
    ui: &mut egui::Ui,
    d: &mut Draft,
    regions: &Option<Result<Vec<profiles::Region>, String>>,
) {
    match regions {
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
                            .selectable_label(d.kept_region.is_none() && d.region == i, label(i))
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
}

/// Eixo da revolução: eixos U/V do plano ou uma linha (de construção ou não) do rascunho.
fn revolve_controls(ui: &mut egui::Ui, d: &mut Draft, sketch: Option<&Sketch>) {
    let label = |axis: RevolveAxis| match axis {
        RevolveAxis::U => text(T::AxisU).to_owned(),
        RevolveAxis::V => text(T::AxisV).to_owned(),
        RevolveAxis::Line(id) => format!("{} #{id}", text(T::Entity)),
    };
    let selected = if d.kept_axis.is_some() {
        text(T::KeptReference).to_owned()
    } else {
        label(d.axis)
    };
    let lines = sketch
        .map(|s| {
            s.entities
                .iter()
                .filter(|e| matches!(e.curve, Curve::Line { .. }))
                .map(|e| e.id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    egui::ComboBox::from_label(text(T::RevolveAxis))
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for axis in [RevolveAxis::U, RevolveAxis::V]
                .into_iter()
                .chain(lines.into_iter().map(RevolveAxis::Line))
            {
                if ui
                    .selectable_label(d.kept_axis.is_none() && d.axis == axis, label(axis))
                    .clicked()
                {
                    d.axis = axis;
                    d.kept_axis = None;
                }
            }
        });
}

/// Corpo base (e corpo ferramenta nas booleanas), entre os sólidos da peça.
fn body_controls(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part) {
    let two = matches!(d.tool, Tool::Join | Tool::Cut | Tool::Intersect);
    let output = d.output;
    for (label, slot) in [
        (T::SourceBody, &mut d.base),
        (T::ToolBody, &mut d.tool_body),
    ]
    .into_iter()
    .take(if two { 2 } else { 1 })
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
                    .filter(|b| b.brep.is_some() && Some(b.id) != output)
                {
                    ui.selectable_value(slot, Some(b.id), &b.name);
                }
            });
    }
}

fn hole_controls(ui: &mut egui::Ui, d: &mut Draft, sketch: Option<&Sketch>) {
    let points = sketch.map(hole_points).unwrap_or_default();
    match &d.hole.kept_points {
        Some(kept) => {
            ui.label(format!("{}: {}", text(T::KeptReference), kept.len()));
        }
        None if points.is_empty() => {
            ui.colored_label(egui::Color32::YELLOW, text(T::NoHolePoints));
        }
        None => {
            ui.label(format!("{}: {}", text(T::HolePoints), points.len()));
        }
    }
    let h = &mut d.hole;
    labeled_text(ui, T::HoleDiameter, &mut h.diameter);
    ui.checkbox(&mut h.through, text(T::Through));
    if !h.through {
        labeled_text(ui, T::HoleDepth, &mut h.depth);
        ui.checkbox(&mut h.tip, text(T::DrillPoint));
        if h.tip {
            labeled_text(ui, T::Angle, &mut h.tip_angle);
        }
    }
    let heads = [T::HeadSimple, T::Counterbore, T::Countersink];
    egui::ComboBox::from_label(text(T::HoleHead))
        .selected_text(text(heads[h.head]))
        .show_ui(ui, |ui| {
            for (i, head) in heads.into_iter().enumerate() {
                ui.selectable_value(&mut h.head, i, text(head));
            }
        });
    if h.head > 0 {
        labeled_text(ui, T::HeadDiameter, &mut h.head_diameter);
        if h.head == 1 {
            labeled_text(ui, T::HeadDepth, &mut h.head_depth);
        } else {
            labeled_text(ui, T::HeadAngle, &mut h.head_angle);
        }
    }
    ui.checkbox(&mut h.reverse, text(T::ReverseDirection));
    ui.checkbox(&mut h.thread, text(T::ThreadToggle));
    if h.thread {
        labeled_text(ui, T::ThreadDesignation, &mut h.thread_designation);
        labeled_text(ui, T::ThreadNominal, &mut h.thread_nominal);
        labeled_text(ui, T::ThreadPitch, &mut h.thread_pitch);
    }
}
