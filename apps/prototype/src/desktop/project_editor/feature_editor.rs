//! Painéis de comando das operações de construção; operações nativas são preparadas e
//! pré-visualizadas fora da interface.
//!
//! Cada ferramenta abre um painel ancorado à área 3D. Campos de seleção ativos recebem os
//! cliques no modelo: perfis do rascunho, arestas, faces e corpos. Ao aplicar, o comando
//! converte essas escolhas em referências estáveis (entidades do rascunho e nomes de faces),
//! que sobrevivem a mudanças de cota. Ao editar uma operação existente, a referência gravada
//! é mantida enquanto o usuário não escolher outra.
use super::{
    command_panel::{self as panel, Footer},
    icons::Icon,
    live_preview::PreviewStatus,
    *,
};
use nator_document::features::{
    BooleanMode, Combine, EdgeRef, ExtrusionLimit, FaceRef, Feature, HoleHeadSpec, HoleSpec,
    LinearAxis, Operation, PatternLayout, RegionRef, ThreadSpec, UntilFace, signature,
};
use nator_sketch::{Curve, Sketch, profiles};
mod extrusion;

/// Ferramentas do painel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Tool {
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
impl Tool {
    pub(super) fn label(self) -> T {
        match self {
            Self::Extrude => T::Extrude,
            Self::Revolve => T::Revolve,
            Self::Join | Self::Cut | Self::Intersect => T::CombineTool,
            Self::Fillet => T::Fillet,
            Self::Chamfer => T::Chamfer,
            Self::Shell => T::Shell,
            Self::Hole => T::HoleTool,
            Self::LinearPattern => T::RectangularPattern,
            Self::CircularPattern => T::CircularPattern,
            Self::Mirror => T::MirrorBody,
            Self::Split => T::SplitBody,
        }
    }
    pub(super) fn icon(self) -> Icon {
        match self {
            Self::Extrude => Icon::Extrude,
            Self::Revolve => Icon::Revolve,
            Self::Hole => Icon::Hole,
            Self::Fillet => Icon::Fillet,
            Self::Chamfer => Icon::Chamfer,
            Self::Shell => Icon::Shell,
            Self::Mirror => Icon::Mirror,
            Self::LinearPattern | Self::CircularPattern => Icon::Pattern,
            Self::Split => Icon::Split,
            Self::Join | Self::Cut | Self::Intersect => Icon::Boolean,
        }
    }
    /// Ferramentas que partem de um rascunho de perfil.
    fn uses_profile(self) -> bool {
        matches!(self, Self::Extrude | Self::Revolve)
    }
    /// Campo que recebe os primeiros cliques ao abrir o painel.
    fn first_field(self) -> Field {
        match self {
            Self::Extrude | Self::Revolve => Field::Profile,
            Self::Fillet | Self::Chamfer => Field::Edges,
            Self::Shell => Field::Faces,
            _ => Field::Base,
        }
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

/// Referência escolhida na área 3D ou gravada na operação em edição.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Kept {
    PlanarFace(FaceRef, nator_sketch::Plane),
    Edge(EdgeRef),
    Face(FaceRef),
    Body(BodyId),
}

/// O que o próximo clique na área 3D deve selecionar para o painel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Awaiting {
    Edge,
    Face,
    Body,
}

/// Campo de seleção do painel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Field {
    Profile,
    Edges,
    Faces,
    Base,
    ToolBody,
    Target,
    UntilFace,
}

/// Eixo da revolução: um eixo do plano do rascunho ou uma linha do rascunho.
#[derive(Debug, Clone, Copy, PartialEq)]
enum RevolveAxis {
    U,
    V,
    Line(u64),
}

/// Direção da extrusão, como nos CADs de mercado. Os limites gravados continuam sendo
/// expressões de início e fim a partir do plano do rascunho.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    OneSide,
    TwoSides,
    Symmetric,
}

/// Extrusão como digitada no painel.
struct ExtrudeDraft {
    direction: Direction,
    distance: String,
    distance2: String,
    /// Deslocamento do início em relação ao plano do rascunho (um lado).
    offset: String,
    flip: bool,
    to_face: bool,
    /// Limites gravados que não seguem os padrões do painel: preservados como estão.
    custom: Option<(String, String)>,
}
impl Default for ExtrudeDraft {
    fn default() -> Self {
        Self {
            direction: Direction::OneSide,
            distance: "0 mm".into(),
            distance2: "0 mm".into(),
            offset: "0 mm".into(),
            flip: false,
            to_face: false,
            custom: None,
        }
    }
}

/// Expressão sem parâmetros que vale zero.
fn is_zero(expression: &str) -> bool {
    nator_document::parameters::Parameters::default()
        .evaluate(expression, nator_document::parameters::Dimension::Length)
        .is_ok_and(|v| v.abs() < 1e-12)
}
fn negated(expression: &str) -> Option<&str> {
    expression.strip_prefix("-(")?.strip_suffix(')')
}
fn negate(expression: &str) -> String {
    negated(expression)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("-({expression})"))
}

impl ExtrudeDraft {
    /// Reconhece os limites gravados pelos padrões do painel; outros ficam personalizados.
    fn from_limits(from: String, to: String) -> Self {
        let mut d = Self::default();
        if let Some(inner) = negated(&from) {
            if inner == to {
                d.direction = Direction::Symmetric;
                d.distance = to;
            } else {
                d.direction = Direction::TwoSides;
                d.distance = to;
                d.distance2 = inner.to_owned();
            }
        } else if is_zero(&from) {
            match negated(&to) {
                Some(inner) => {
                    d.distance = inner.to_owned();
                    d.flip = true;
                }
                None => d.distance = to,
            }
        } else if let Some(rest) = to
            .strip_prefix(&format!("({from}) + ("))
            .and_then(|r| r.strip_suffix(')'))
        {
            d.offset = from.clone();
            match negated(rest) {
                Some(inner) => {
                    d.distance = inner.to_owned();
                    d.flip = true;
                }
                None => d.distance = rest.to_owned(),
            }
        } else {
            d.custom = Some((from, to));
        }
        d
    }
    /// Expressões de início e fim gravadas na operação.
    fn limits(&self) -> (String, String) {
        if let Some((from, to)) = &self.custom {
            return (from.clone(), to.clone());
        }
        match self.direction {
            Direction::OneSide => {
                let distance = if self.flip {
                    negate(&self.distance)
                } else {
                    self.distance.clone()
                };
                if is_zero(&self.offset) {
                    ("0 mm".into(), distance)
                } else {
                    (
                        self.offset.clone(),
                        format!("({}) + ({distance})", self.offset),
                    )
                }
            }
            Direction::Symmetric => (negate(&self.distance), self.distance.clone()),
            Direction::TwoSides => {
                let (a, b) = if self.flip {
                    (&self.distance2, &self.distance)
                } else {
                    (&self.distance, &self.distance2)
                };
                (negate(b), a.clone())
            }
        }
    }
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
    region: Option<usize>,
    kept_region: Option<RegionRef>,
    /// Face plana como alternativa a um perfil de rascunho.
    face_profile: Option<(BodyId, FaceRef)>,
    automatic: bool,
    auto_preferred: Option<BodyId>,
    interaction: extrusion::Interaction,
    axis: RevolveAxis,
    /// Eixo fixo gravado em uma revolução antiga, mantido até outra escolha.
    kept_axis: Option<([f64; 3], [f64; 3])>,
    /// Índice de aresta/face no corpo base, para seleções sem nome estável.
    index: usize,
    kept_topology: Option<Kept>,
    /// Arestas/faces adicionais da mesma operação (arredondamento, chanfro, casca).
    extra_topology: Vec<Kept>,
    /// Campo de seleção que recebe os próximos cliques no modelo.
    field: Option<Field>,
    /// Chanfro por distância e ângulo, com o ângulo em expressão.
    chamfer_angle: Option<String>,
    /// Casca com paredes para fora do sólido original.
    outward: bool,
    /// Resultado da extrusão: 0 novo corpo, 1 união, 2 corte, 3 interseção com `combine_target`.
    combine: usize,
    combine_target: Option<BodyId>,
    /// Face limite da extrusão (corpo e face), escolhida por clique.
    until: Option<ExtrusionLimit>,
    extrude: ExtrudeDraft,
    /// Medida principal das ferramentas de uma só dimensão (raio, distância, espessura).
    size: String,
    angle: String,
    hole: HoleDraft,
    layout: LayoutDraft,
}
impl Draft {
    fn new(tab: u64, part: PartId, tool: Tool) -> Self {
        Self {
            tab,
            part,
            output: None,
            tool,
            sketch: None,
            base: None,
            tool_body: None,
            region: None,
            kept_region: None,
            face_profile: None,
            automatic: tool == Tool::Extrude,
            auto_preferred: None,
            interaction: extrusion::Interaction::default(),
            axis: RevolveAxis::V,
            kept_axis: None,
            index: 0,
            kept_topology: None,
            extra_topology: Vec::new(),
            field: Some(tool.first_field()),
            chamfer_angle: None,
            outward: false,
            combine: 0,
            combine_target: None,
            until: None,
            extrude: ExtrudeDraft::default(),
            size: match tool {
                Tool::Fillet | Tool::Chamfer => "2 mm".into(),
                _ => "2 mm".into(),
            },
            angle: "360 deg".into(),
            hole: HoleDraft::default(),
            layout: LayoutDraft::default(),
        }
    }
    pub fn edit(tab: u64, part: PartId, feature: Feature) -> Self {
        let mut d = Self::new(tab, part, Tool::Extrude);
        d.output = Some(feature.output);
        d.automatic = false;
        d.field = None;
        let nearest = |direction: [f64; 3], table: &[(&str, [f64; 3])]| {
            table
                .iter()
                .position(|(_, v)| nator_geometry::dot(*v, direction).abs() > 0.999)
                .unwrap_or(0)
        };
        match feature.operation {
            Operation::ExtrudeFace {
                base,
                face,
                from,
                to,
                combine,
                until,
            } => {
                d.face_profile = Some((base, face));
                d.extrude = ExtrudeDraft::from_limits(from, to);
                if let Some(combine) = combine {
                    d.combine = match combine.mode {
                        BooleanMode::Join => 1,
                        BooleanMode::Cut => 2,
                        BooleanMode::Intersect => 3,
                    };
                    d.combine_target = Some(combine.target);
                }
                d.extrude.to_face = until.is_some();
                d.until = until;
            }
            Operation::Extrude {
                sketch,
                region,
                from,
                to,
                combine,
                until,
            } => {
                d.sketch = Some(sketch);
                d.kept_region = Some(region);
                d.extrude = ExtrudeDraft::from_limits(from, to);
                if let Some(combine) = combine {
                    d.combine = match combine.mode {
                        BooleanMode::Join => 1,
                        BooleanMode::Cut => 2,
                        BooleanMode::Intersect => 3,
                    };
                    d.combine_target = Some(combine.target);
                }
                d.extrude.to_face = until.is_some();
                d.until = until;
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
                d.size = radius;
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
                d.size = distance;
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
                d.size = thickness;
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
fn describe_topology(kept: &Kept) -> String {
    match kept {
        Kept::Edge(EdgeRef::Between([a, b])) => format!("{a} × {b}"),
        Kept::Face(FaceRef::Named(name)) => name.clone(),
        Kept::Edge(EdgeRef::Index { index, .. }) | Kept::Face(FaceRef::Index { index, .. }) => {
            format!("#{index}")
        }
        Kept::PlanarFace(face, _) => describe_topology(&Kept::Face(face.clone())),
        Kept::Body(id) => format!("#{}", id.0),
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
    /// Tipo de elemento que o próximo clique deve selecionar, se o painel estiver esperando.
    pub(super) fn awaiting(&self) -> Option<Awaiting> {
        if self.tool == Tool::Extrude && matches!(self.field, None | Some(Field::Profile)) {
            return Some(Awaiting::Face);
        }
        match self.field? {
            Field::Edges => Some(Awaiting::Edge),
            Field::Faces | Field::UntilFace => Some(Awaiting::Face),
            Field::Base | Field::ToolBody | Field::Target => Some(Awaiting::Body),
            Field::Profile => None,
        }
    }
    pub(super) fn field(&self) -> Option<Field> {
        self.field
    }
    pub(super) fn part(&self) -> PartId {
        self.part
    }
    pub(super) fn tab(&self) -> u64 {
        self.tab
    }
    /// Operação em edição, cujo histórico é recuado para as seleções.
    pub(super) fn output(&self) -> Option<BodyId> {
        self.output
    }
    pub(super) fn sketch(&self) -> Option<SketchId> {
        self.sketch
    }
    /// Corpo base cujas arestas ou faces estão escolhidas.
    pub(super) fn base(&self) -> Option<BodyId> {
        self.base
    }
    /// Arestas e faces escolhidas no corpo base, para o realce na área 3D.
    pub(super) fn references(&self) -> impl Iterator<Item = &Kept> {
        self.kept_topology.iter().chain(&self.extra_topology)
    }
    /// Região do perfil escolhida por clique no rascunho.
    pub(super) fn choose_region(&mut self, sketch: SketchId, region: RegionRef) {
        self.face_profile = None;
        self.interaction = extrusion::Interaction::default();
        if self.tool == Tool::Extrude && self.sketch == Some(sketch) {
            let mut selected = self
                .kept_region
                .take()
                .map(|r| r.selections().to_vec())
                .unwrap_or_default();
            if let Some(index) = selected.iter().position(|r| *r == region) {
                selected.remove(index);
            } else {
                selected.push(region);
            }
            self.kept_region = match selected.len() {
                0 => None,
                1 => selected.pop(),
                _ => Some(RegionRef::Multiple(selected)),
            };
        } else {
            self.kept_region = Some(region);
        }
        self.sketch = Some(sketch);
        self.region = None;
        // Na revolução, o eixo gravado pertence ao rascunho anterior.
        if self.tool == Tool::Revolve {
            self.kept_axis = None;
        }
        self.field = if self.tool == Tool::Extrude {
            Some(Field::Profile)
        } else {
            None
        };
    }
    pub(super) fn choose_limit(&mut self, limit: ExtrusionLimit) {
        let (from, _) = self.extrude.limits();
        self.until = Some(limit);
        self.extrude.to_face = true;
        self.extrude.custom = None;
        self.extrude.direction = Direction::OneSide;
        self.extrude.offset = from;
        self.field = Some(Field::Profile);
    }
    pub(super) fn region_reference(&self) -> Option<&RegionRef> {
        self.kept_region.as_ref()
    }
    /// Aplica uma seleção gráfica ao campo ativo. Arestas e faces alternam: clicar de novo
    /// numa já escolhida a retira.
    pub(super) fn apply_pick(&mut self, body: BodyId, kept: Kept) {
        if let Kept::PlanarFace(face, plane) = kept {
            let coplanar = self.interaction.guide.is_some_and(|g| {
                nator_geometry::norm(nator_geometry::cross(g.normal, plane.normal())) < 1e-7
                    && nator_geometry::dot(nator_geometry::sub(plane.origin, g.origin), g.normal)
                        .abs()
                        < 1e-6
            });
            if (self.tool == Tool::Extrude && self.complete() && !coplanar)
                || self.field == Some(Field::UntilFace)
            {
                self.choose_limit(UntilFace { body, face }.into());
            } else {
                self.apply_pick(body, Kept::Face(face));
            }
            return;
        }
        match (self.field, kept) {
            (Some(Field::Profile), Kept::Face(face)) if self.tool == Tool::Extrude => {
                self.choose_face_profile(body, face);
            }
            (None, Kept::Face(face)) if self.tool == Tool::Extrude => {
                let (from, _) = self.extrude.limits();
                self.until = Some(UntilFace { body, face }.into());
                self.extrude.to_face = true;
                self.extrude.custom = None;
                self.extrude.direction = Direction::OneSide;
                self.extrude.offset = from;
            }
            (Some(Field::UntilFace), Kept::Face(face)) => {
                self.until = Some(UntilFace { body, face }.into());
                self.extrude.to_face = true;
                self.field = None;
            }
            (Some(Field::Base), Kept::Body(id)) => {
                if self.base != Some(id) {
                    self.kept_topology = None;
                    self.extra_topology.clear();
                }
                self.base = Some(id);
                self.field = match self.tool {
                    Tool::Join | Tool::Cut | Tool::Intersect if self.tool_body.is_none() => {
                        Some(Field::ToolBody)
                    }
                    _ => None,
                };
            }
            (Some(Field::ToolBody), Kept::Body(id)) => {
                self.tool_body = Some(id);
                self.field = None;
            }
            (Some(Field::Target), Kept::Body(id)) => {
                self.automatic = false;
                self.combine_target = Some(id);
                self.field = None;
            }
            (Some(Field::Edges | Field::Faces), kept @ (Kept::Edge(_) | Kept::Face(_))) => {
                if self.base != Some(body) {
                    // Outro corpo: as escolhas anteriores deixam de valer.
                    self.base = Some(body);
                    self.kept_topology = Some(kept);
                    self.extra_topology.clear();
                } else if self.kept_topology.as_ref() == Some(&kept) {
                    self.kept_topology = if self.extra_topology.is_empty() {
                        None
                    } else {
                        Some(self.extra_topology.remove(0))
                    };
                } else if let Some(i) = self.extra_topology.iter().position(|k| *k == kept) {
                    self.extra_topology.remove(i);
                } else if self.kept_topology.is_none() {
                    self.kept_topology = Some(kept);
                } else {
                    self.extra_topology.push(kept);
                }
            }
            _ => {}
        }
    }
    pub(super) fn describe(kept: &Kept) -> String {
        describe_topology(kept)
    }
    pub(super) fn choose_face_profile(&mut self, body: BodyId, face: FaceRef) {
        self.face_profile = Some((body, face));
        self.auto_preferred = Some(body);
        self.sketch = None;
        self.kept_region = None;
        self.region = None;
        self.interaction = extrusion::Interaction::default();
        self.field = None;
    }

    fn extra_edges(&self) -> Vec<EdgeRef> {
        self.extra_topology
            .iter()
            .filter_map(|k| match k {
                Kept::Edge(edge) => Some(edge.clone()),
                _ => None,
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

    /// Seleções obrigatórias ainda vazias impedem o OK.
    fn complete(&self) -> bool {
        match self.tool {
            Tool::Extrude => {
                (self.face_profile.is_some()
                    || (self.sketch.is_some()
                        && (self.kept_region.is_some() || self.region.is_some())))
                    && (!self.extrude.to_face || self.until.is_some())
                    && (self.automatic || self.combine == 0 || self.combine_target.is_some())
            }
            Tool::Revolve => {
                self.sketch.is_some() && (self.kept_region.is_some() || self.region.is_some())
            }
            Tool::Join | Tool::Cut | Tool::Intersect => {
                self.base.is_some() && self.tool_body.is_some() && self.base != self.tool_body
            }
            Tool::Fillet | Tool::Chamfer | Tool::Shell => {
                self.base.is_some() && self.kept_topology.is_some()
            }
            Tool::Hole => self.base.is_some() && self.sketch.is_some(),
            _ => self.base.is_some(),
        }
    }

    fn zero_extrusion(&self, part: &nator_document::Part) -> bool {
        if self.tool != Tool::Extrude || self.extrude.to_face {
            return false;
        }
        let (from, to) = self.extrude.limits();
        let value = |v: &str| {
            part.parameters
                .evaluate(v, nator_document::parameters::Dimension::Length)
                .ok()
        };
        value(&from)
            .zip(value(&to))
            .is_some_and(|(a, b)| (a - b).abs() < 1e-7)
    }
    /// Monta a operação a partir do painel.
    fn operation(&self, part: &nator_document::Part) -> Operation {
        let base = self.base.unwrap_or(BodyId(0));
        let sketch = self.sketch.unwrap_or(SketchId(0));
        let stamp = part
            .bodies
            .iter()
            .find(|b| b.id == base)
            .and_then(|b| b.brep.as_ref())
            .map(|b| signature(b))
            .unwrap_or(0);
        let record = part.sketches.iter().find(|s| s.id == sketch);
        // Nova seleção pela lista: a posição atual vira identidade estável no comando.
        let region = self.kept_region.clone().unwrap_or_else(|| {
            if self.region.is_none() {
                return RegionRef::Index(0);
            }
            let index = self.region.unwrap_or(0);
            match record.map(|r| profiles::regions(&r.sketch)) {
                Some(Ok(list)) if index < list.len() => RegionRef::Boundary(list[index].boundary()),
                _ => RegionRef::Index(index),
            }
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
        match self.tool {
            Tool::Extrude => {
                let (from, to) = self.extrude.limits();
                let combine = self
                    .combine_target
                    .filter(|_| !self.automatic && self.combine > 0)
                    .map(|target| Combine {
                        target,
                        mode: match self.combine {
                            1 => BooleanMode::Join,
                            2 => BooleanMode::Cut,
                            _ => BooleanMode::Intersect,
                        },
                    });
                let until = self.until.clone().filter(|_| self.extrude.to_face);
                if let Some((base, face)) = &self.face_profile {
                    return Operation::ExtrudeFace {
                        base: *base,
                        face: face.clone(),
                        from,
                        to,
                        combine,
                        until,
                    };
                }
                Operation::Extrude {
                    sketch,
                    region,
                    from,
                    to,
                    combine,
                    until,
                }
            }
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
                radius: self.size.clone(),
                additional_edges: self.extra_edges(),
            },
            Tool::Chamfer => Operation::Chamfer {
                base,
                edge,
                distance: self.size.clone(),
                angle: self.chamfer_angle.clone(),
                additional_edges: self.extra_edges(),
            },
            Tool::Shell => Operation::Shell {
                base,
                face,
                thickness: self.size.clone(),
                additional_faces: self
                    .extra_topology
                    .iter()
                    .filter_map(|k| match k {
                        Kept::Face(face) => Some(face.clone()),
                        _ => None,
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

    /// Comando que o OK aplicaria, também usado como chave da pré-visualização.
    pub(super) fn preview_key(&self, part: &nator_document::Part) -> Command {
        self.command(part)
    }
    /// Medida principal (distância da extrusão, raio, espessura), para capturas de ensaio.
    pub(super) fn set_size_for_capture(&mut self, value: &str) {
        self.size = value.into();
        self.extrude.distance = value.into();
    }
    /// Comando que o OK aplicaria.
    fn command(&self, part: &nator_document::Part) -> Command {
        let operation = self.operation(part);
        if self.tool == Tool::Extrude && self.automatic && self.output.is_none() {
            return Command::AddSmartExtrusion {
                part: self.part,
                name: text(T::Extrude).into(),
                operation,
                preferred: self.auto_preferred,
            };
        }
        match self.output {
            Some(output) => Command::EditFeature {
                part: self.part,
                output,
                operation,
            },
            None => Command::AddFeature {
                part: self.part,
                name: text(match self.tool {
                    Tool::Join => T::Join,
                    Tool::Cut => T::Cut,
                    Tool::Intersect => T::Intersect,
                    Tool::LinearPattern => T::LinearPattern,
                    tool => tool.label(),
                })
                .into(),
                operation,
            },
        }
    }
}

/// Eixo ou plano principal escolhido por botões X/Y/Z ou XY/XZ/YZ.
fn axis_choice(ui: &mut egui::Ui, value: &mut usize, options: &[(&str, [f64; 3])]) {
    ui.spacing_mut().item_spacing.x = 2.;
    for (i, (name, _)) in options.iter().enumerate() {
        if ui.selectable_label(*value == i, *name).clicked() {
            *value = i;
        }
    }
}

/// Nome de um corpo da peça, para os campos de seleção.
fn body_name(part: &nator_document::Part, id: Option<BodyId>) -> Option<String> {
    id.and_then(|id| part.bodies.iter().find(|b| b.id == id))
        .map(|b| b.name.clone())
}

impl Editor {
    pub(super) fn open_feature_tool(&mut self, tool: Tool) {
        let Some(part) = self.current().active_part else {
            return;
        };
        let mut d = Draft::new(self.current().id, part, tool);
        let definition = self.current().session.document().part(part);
        // Rascunho em edição ou visível sugere o perfil; o clique continua escolhendo a região.
        d.sketch = self
            .current()
            .sketch_edit
            .as_ref()
            .map(|w| w.id)
            .or_else(|| {
                definition.and_then(|p| {
                    p.sketches
                        .iter()
                        .rev()
                        .find(|s| s.visible)
                        .or_else(|| p.sketches.last())
                        .map(|s| s.id)
                })
            });
        // A seleção explícita é a primeira sugestão de corpo. Caso contrário, o resultado
        // visível da peça; a operação continua pendente de confirmação.
        let suggestion = match self.current().selection {
            Some(Selection::Body { part: owner, id }) if owner == part => Some(id),
            _ => definition
                .and_then(|p| {
                    p.bodies
                        .iter()
                        .rev()
                        .find(|b| b.visible && b.brep.is_some())
                })
                .map(|b| b.id),
        };
        match tool {
            Tool::Extrude | Tool::Revolve => {
                d.combine_target = suggestion;
                d.auto_preferred = suggestion;
            }
            Tool::Fillet | Tool::Chamfer | Tool::Shell => {
                d.base = suggestion;
            }
            Tool::Join | Tool::Cut | Tool::Intersect => {
                let bodies = self
                    .current()
                    .selected
                    .iter()
                    .filter_map(|selection| match selection {
                        Selection::Body { part: owner, id }
                            if *owner == part
                                && self
                                    .current()
                                    .session
                                    .document()
                                    .body(*id)
                                    .is_some_and(|body| body.brep.is_some()) =>
                        {
                            Some(*id)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                d.base = bodies.first().copied().or(suggestion);
                d.tool_body = bodies.get(1).copied();
                d.field = if d.base.is_none() {
                    Some(Field::Base)
                } else if d.tool_body.is_none() {
                    Some(Field::ToolBody)
                } else {
                    None
                };
            }
            Tool::Hole
            | Tool::LinearPattern
            | Tool::CircularPattern
            | Tool::Mirror
            | Tool::Split => {
                d.base = suggestion;
                d.field = suggestion.is_none().then_some(Field::Base);
            }
        }
        if tool == Tool::Shell
            && self.pick_mode == navigation::PickMode::Face
            && let Some(pick) = self.current().picked.as_ref().filter(|pick| {
                pick.part == part
                    && self.current().is_selected(Selection::Body {
                        part,
                        id: pick.body,
                    })
            })
            && let Ok(face) = self.current().session.document().face_at(pick)
        {
            d.apply_pick(pick.body, Kept::Face(face));
        }
        // Concluir o rascunho aberto mantém o perfil visível para a seleção.
        if tool == Tool::Extrude
            && self.current().sketch_edit.is_none()
            && self.pick_mode == navigation::PickMode::Face
            && let Some(pick) = self.current().picked.as_ref().filter(|p| {
                p.part == part
                    && self
                        .current()
                        .is_selected(Selection::Body { part, id: p.body })
            })
            && let Ok(face) = self.current().session.document().face_at(pick)
        {
            d.apply_pick(pick.body, Kept::Face(face));
        }
        if self.current().sketch_edit.is_some() {
            self.finish_sketch();
        }
        self.feature_edit = Some(d);
    }
    /// Converter a aresta percorre a tesselação: a sugestão da seleção atual usa
    /// o mesmo trabalhador dos cliques, sem bloquear a interface na abertura.
    pub(super) fn prefill_selected_topology(&mut self, context: &egui::Context) {
        let Some(draft) = self.feature_edit.as_ref() else {
            return;
        };
        if draft.awaiting() != Some(Awaiting::Edge) || self.pick_mode != navigation::PickMode::Edge
        {
            return;
        }
        let Some(pick) = self
            .selected_pick()
            .filter(|pick| pick.part == draft.part())
            .cloned()
        else {
            return;
        };
        let tab = self.current().id;
        let revision = self.current().session.revision();
        let document = self.current().session.document().clone();
        self.start(context, None, move || {
            let topology = document
                .edge_at(&pick)
                .map(Kept::Edge)
                .map_err(|error| error.to_string());
            Ok(Outcome::Picked {
                tab,
                revision,
                pick: Some(pick),
                mode: navigation::PickMode::Edge,
                additive: false,
                topology: Some(topology),
            })
        });
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
        let record = d
            .sketch
            .and_then(|id| part.sketches.iter().find(|s| s.id == id));
        let regions = record
            .filter(|_| d.tool.uses_profile())
            .and_then(|s| self.profile_areas(s.id, ctx))
            .map(|result| {
                result.and_then(|areas| {
                    areas
                        .map(|a| a.regions.clone())
                        .ok_or_else(|| text(T::FindingClosedAreas).to_owned())
                })
            });
        let command = d.command(&part);
        let complete = d.complete() && nator_kernel::is_available();
        let footer_error;
        let footer = if !complete || d.zero_extrusion(&part) {
            Footer::None
        } else {
            match self.preview_status(&command) {
                PreviewStatus::Waiting => Footer::Waiting,
                PreviewStatus::Ready => Footer::Ready,
                PreviewStatus::Failed(error) => {
                    footer_error = error.to_string();
                    Footer::Failed(&footer_error)
                }
            }
        };
        let title = text(match d.tool {
            Tool::Join | Tool::Cut | Tool::Intersect => T::CombineTool,
            tool => tool.label(),
        });
        let outcome = panel::show(
            ctx,
            panel::Header {
                id: "feature",
                icon: d.tool.icon(),
                title,
            },
            self.canvas_rect,
            footer,
            complete && !d.zero_extrusion(&part) && !self.busy(),
            |ui| {
                if !nator_kernel::is_available() {
                    ui.colored_label(theme::WARNING, text(T::KernelUnavailable));
                }
                if self
                    .pending_failure
                    .as_ref()
                    .is_some_and(|p| p.fixing && Some(p.failure.output) == d.output)
                {
                    ui.colored_label(theme::WARNING, text(T::FixingFailure));
                }
                match d.tool {
                    Tool::Extrude => extrude_panel(ui, &mut d, &part, &regions),
                    Tool::Revolve => revolve_panel(ui, &mut d, &part, &regions, record),
                    Tool::Join | Tool::Cut | Tool::Intersect => combine_panel(ui, &mut d, &part),
                    Tool::Fillet | Tool::Chamfer | Tool::Shell => topology_panel(ui, &mut d, &part),
                    Tool::Hole => hole_panel(ui, &mut d, &part, record),
                    Tool::LinearPattern | Tool::CircularPattern => pattern_panel(ui, &mut d, &part),
                    Tool::Mirror | Tool::Split => plane_panel(ui, &mut d, &part),
                }
            },
        );
        let tab = d.tab;
        // Os campos podem mudar neste mesmo quadro (inclusive ao clicar em OK).
        // Publicar e pedir prévia sempre usa o estado posterior à edição do formulário.
        let command = d.command(&part);
        let complete = d.complete() && nator_kernel::is_available();
        if outcome.cancel {
            self.close_feature_panel(&d);
            return;
        }
        if outcome.ok && complete && !d.zero_extrusion(&part) {
            let command = match d.output {
                Some(output) => self.with_pending_fix(tab, output, command),
                None => command,
            };
            self.apply_previewed(ctx, tab, command);
            self.close_feature_panel(&d);
            return;
        }
        if complete {
            // Ao editar, a prévia mostra o histórico até a operação, como no recuo da linha
            // do tempo durante a edição.
            let rollback = d.output.map(|output| (d.part, output));
            self.preview_command(tab, Some(command), d.output, rollback);
        } else {
            self.preview_command(tab, None, None, None);
        }
        self.feature_edit = Some(d);
    }

    pub(super) fn cancel_feature_tool(&mut self) {
        if let Some(draft) = self.feature_edit.take() {
            self.close_feature_panel(&draft);
        }
    }

    /// Fecha o painel; uma correção pendente volta a oferecer as opções de resolução.
    fn close_feature_panel(&mut self, draft: &Draft) {
        if let Some(pending) = self
            .pending_failure
            .as_mut()
            .filter(|p| p.tab == draft.tab && Some(p.failure.output) == draft.output)
        {
            pending.fixing = false;
        }
    }

    /// Publica a prévia já calculada para o comando, sem recalcular; uma falha atribuída a
    /// uma operação abre a decisão do usuário; sem prévia válida, prepara normalmente.
    pub(super) fn apply_previewed(&mut self, context: &egui::Context, tab: u64, command: Command) {
        if let Some(prepared) = self.take_preview(tab, &command) {
            self.commit_prepared(tab, prepared);
        } else if let Some(error) = self
            .preview_failure(&command)
            .filter(|e| e.failure.is_some())
        {
            self.reject_command(tab, command, error);
        } else {
            self.prepare_command(context, tab, command);
        }
    }
}

/// Perfil escolhido por clique, com alternativa pela lista de regiões.
fn profile_rows(
    ui: &mut egui::Ui,
    d: &mut Draft,
    part: &nator_document::Part,
    regions: &Option<Result<Arc<Vec<profiles::Region>>, String>>,
) {
    let chosen = d
        .kept_region
        .as_ref()
        .map(|r| r.selections().len())
        .unwrap_or_else(|| usize::from(d.face_profile.is_some() || d.region.is_some()));
    let sketch_name = d
        .sketch
        .and_then(|id| part.sketches.iter().find(|s| s.id == id))
        .map(|s| s.sketch.name.clone())
        .or_else(|| {
            d.face_profile
                .as_ref()
                .and_then(|(body, _)| body_name(part, Some(*body)))
        });
    let selection = panel::selection(
        ui,
        text(T::ProfileField),
        chosen,
        sketch_name.as_deref(),
        d.field == Some(Field::Profile),
    );
    if selection.activate {
        d.field = Some(Field::Profile);
    }
    if selection.clear {
        d.face_profile = None;
        d.interaction = extrusion::Interaction::default();
        d.kept_region = None;
        d.region = None;
        d.field = Some(Field::Profile);
    }
    if d.field == Some(Field::Profile) {
        ui.label(
            egui::RichText::new(text(if d.tool == Tool::Extrude {
                T::ExtrusionProfileHint
            } else {
                T::ProfileHint
            }))
            .small()
            .color(theme::MUTED),
        );
    }
    match regions {
        Some(Err(message)) => {
            ui.colored_label(theme::WARNING, message);
        }
        Some(Ok(list)) if list.is_empty() => {
            ui.colored_label(theme::WARNING, text(T::NoRegions));
        }
        _ => {}
    }
}

fn extrude_panel(
    ui: &mut egui::Ui,
    d: &mut Draft,
    part: &nator_document::Part,
    regions: &Option<Result<Arc<Vec<profiles::Region>>, String>>,
) {
    profile_rows(ui, d, part, regions);
    ui.label(
        egui::RichText::new(text(T::ExtrusionMouseHint))
            .small()
            .color(theme::MUTED),
    );
    let e = &mut d.extrude;
    if let Some((from, to)) = &mut e.custom {
        panel::row(ui, text(T::FromField), |ui| panel::expression(ui, from));
        panel::row(ui, text(T::ToField), |ui| panel::expression(ui, to));
        if ui.small_button(text(T::OneSide)).clicked() {
            e.custom = None;
        }
    } else {
        panel::row(ui, text(T::DirectionField), |ui| {
            panel::choice(
                ui,
                &mut e.direction,
                &[
                    (Direction::OneSide, Icon::OneSide, T::OneSide),
                    (Direction::TwoSides, Icon::TwoSides, T::TwoSides),
                    (Direction::Symmetric, Icon::Symmetric, T::Symmetric),
                ],
            );
        });
        panel::row(ui, text(T::ExtentField), |ui| {
            panel::choice(
                ui,
                &mut e.to_face,
                &[
                    (false, Icon::Measure, T::ExtentDistance),
                    (true, Icon::ToFace, T::ExtentToFace),
                ],
            );
        });
        if e.to_face {
            let chosen = usize::from(d.until.is_some());
            let description = d.until.as_ref().map(|limit| match limit {
                ExtrusionLimit::Face(face) => describe_topology(&Kept::Face(face.face.clone())),
                ExtrusionLimit::Sketch { sketch } => part
                    .sketches
                    .iter()
                    .find(|s| s.id == *sketch)
                    .map(|s| s.sketch.name.clone())
                    .unwrap_or_default(),
                ExtrusionLimit::WorkPlane { work_plane } => part
                    .work_planes
                    .iter()
                    .find(|p| p.id == *work_plane)
                    .map(|p| p.name.clone())
                    .unwrap_or_default(),
            });
            let s = panel::selection(
                ui,
                text(T::FacesField),
                chosen,
                description.as_deref(),
                d.field == Some(Field::UntilFace),
            );
            if s.activate {
                d.field = Some(Field::UntilFace);
            }
            if s.clear {
                d.until = None;
            }
        } else {
            let e = &mut d.extrude;
            panel::row(ui, text(T::DistanceField), |ui| {
                panel::expression(ui, &mut e.distance);
                if icons::button(ui, Icon::Flip, text(T::FlipDirection), false, e.flip).clicked() {
                    e.flip = !e.flip;
                }
            });
            if e.direction == Direction::TwoSides {
                panel::row(ui, text(T::Distance2Field), |ui| {
                    panel::expression(ui, &mut e.distance2)
                });
            }
        }
        let e = &mut d.extrude;
        if e.direction == Direction::OneSide {
            panel::row(ui, text(T::StartOffset), |ui| {
                panel::expression(ui, &mut e.offset)
            });
        }
    }
    operation_rows(ui, d, part);
}

/// Resultado: novo corpo, união, corte ou interseção com um corpo alvo.
fn operation_rows(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part) {
    if d.tool == Tool::Extrude && d.output.is_none() {
        ui.checkbox(&mut d.automatic, text(T::AutomaticExtrusion));
    }
    let previous = d.combine;
    panel::row(ui, text(T::OperationField), |ui| {
        panel::choice(
            ui,
            &mut d.combine,
            &[
                (1, Icon::Join, T::Join),
                (2, Icon::CutBody, T::RemoveMaterial),
                (3, Icon::IntersectBody, T::Intersect),
                (0, Icon::NewBody, T::NewBody),
            ],
        );
    });
    if previous != d.combine {
        d.automatic = false;
        if d.combine > 0 && d.combine_target.is_none() {
            d.field = Some(Field::Target);
        }
    }
    if d.combine > 0 {
        let name = body_name(part, d.combine_target);
        let s = panel::selection(
            ui,
            text(T::TargetBody),
            usize::from(d.combine_target.is_some()),
            name.as_deref(),
            d.field == Some(Field::Target),
        );
        if s.activate {
            d.field = Some(Field::Target);
        }
        if s.clear {
            d.combine_target = None;
            d.automatic = false;
            d.field = Some(Field::Target);
        }
    }
}

fn revolve_panel(
    ui: &mut egui::Ui,
    d: &mut Draft,
    part: &nator_document::Part,
    regions: &Option<Result<Arc<Vec<profiles::Region>>, String>>,
    record: Option<&nator_document::SketchRecord>,
) {
    profile_rows(ui, d, part, regions);
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
    let lines = record
        .map(|s| {
            s.sketch
                .entities
                .iter()
                .filter(|e| matches!(e.curve, Curve::Line { .. }))
                .map(|e| e.id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    panel::row(ui, text(T::AxisField), |ui| {
        egui::ComboBox::from_id_salt("revolve_axis")
            .width(170.)
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
    });
    let mut full = d.angle.trim() == "360 deg";
    panel::row(ui, text(T::Angle), |ui| {
        ui.add_enabled_ui(!full, |ui| panel::expression(ui, &mut d.angle));
        if ui.checkbox(&mut full, text(T::FullTurn)).changed() {
            d.angle = if full { "360 deg" } else { "180 deg" }.into();
        }
    });
    operation_rows(ui, d, part);
}

fn body_selection(
    ui: &mut egui::Ui,
    d: &mut Draft,
    part: &nator_document::Part,
    field: Field,
    label: T,
) {
    let value = match field {
        Field::ToolBody => d.tool_body,
        _ => d.base,
    };
    let name = body_name(part, value);
    let s = panel::selection(
        ui,
        text(label),
        usize::from(value.is_some()),
        name.as_deref(),
        d.field == Some(field),
    );
    if s.activate {
        d.field = Some(field);
    }
    if s.clear {
        match field {
            Field::ToolBody => d.tool_body = None,
            _ => {
                d.base = None;
                d.kept_topology = None;
                d.extra_topology.clear();
            }
        }
        d.field = Some(field);
    }
}

fn combine_panel(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part) {
    body_selection(ui, d, part, Field::Base, T::TargetBody);
    body_selection(ui, d, part, Field::ToolBody, T::ToolBodyField);
    panel::row(ui, text(T::OperationField), |ui| {
        panel::choice(
            ui,
            &mut d.tool,
            &[
                (Tool::Join, Icon::Join, T::Join),
                (Tool::Cut, Icon::CutBody, T::Cut),
                (Tool::Intersect, Icon::IntersectBody, T::Intersect),
            ],
        );
    });
}

fn topology_panel(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part) {
    let field = if d.tool == Tool::Shell {
        Field::Faces
    } else {
        Field::Edges
    };
    let chosen = usize::from(d.kept_topology.is_some()) + d.extra_topology.len();
    let description = d.kept_topology.as_ref().map(describe_topology);
    let s = panel::selection(
        ui,
        text(if field == Field::Faces {
            T::FacesField
        } else {
            T::EdgesField
        }),
        chosen,
        description.as_deref(),
        d.field == Some(field),
    );
    if s.activate {
        d.field = Some(field);
    }
    if s.clear {
        d.kept_topology = None;
        d.extra_topology.clear();
        d.field = Some(field);
    }
    if let Some(name) = body_name(part, d.base).filter(|_| chosen > 0) {
        panel::row(ui, text(T::BodyField), |ui| {
            ui.label(egui::RichText::new(name).color(theme::MUTED))
        });
    }
    match d.tool {
        Tool::Fillet => {
            panel::row(ui, text(T::RadiusField), |ui| {
                panel::expression(ui, &mut d.size)
            });
        }
        Tool::Chamfer => {
            let mut by_angle = d.chamfer_angle.is_some();
            panel::row(ui, text(T::ChamferType), |ui| {
                if panel::choice(
                    ui,
                    &mut by_angle,
                    &[
                        (false, Icon::Chamfer, T::EqualDistance),
                        (true, Icon::Measure, T::DistanceAngle),
                    ],
                ) {
                    d.chamfer_angle = by_angle.then(|| "45 deg".into());
                }
            });
            panel::row(ui, text(T::DistanceField), |ui| {
                panel::expression(ui, &mut d.size)
            });
            if let Some(angle) = &mut d.chamfer_angle {
                panel::row(ui, text(T::Angle), |ui| panel::expression(ui, angle));
            }
        }
        _ => {
            panel::row(ui, text(T::ThicknessField), |ui| {
                panel::expression(ui, &mut d.size)
            });
            panel::row(ui, text(T::DirectionField), |ui| {
                panel::choice(
                    ui,
                    &mut d.outward,
                    &[
                        (false, Icon::Shell, T::InwardWalls),
                        (true, Icon::Cube, T::OutwardWalls),
                    ],
                );
            });
        }
    }
    egui::CollapsingHeader::new(text(T::AdvancedGroup))
        .id_salt("topology_advanced")
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text(T::TopologyIndex)).small());
            if ui
                .add(egui::DragValue::new(&mut d.index).range(0..=100000))
                .changed()
            {
                d.kept_topology = None;
                d.extra_topology.clear();
            }
        });
}

fn hole_panel(
    ui: &mut egui::Ui,
    d: &mut Draft,
    part: &nator_document::Part,
    record: Option<&nator_document::SketchRecord>,
) {
    body_selection(ui, d, part, Field::Base, T::BodyField);
    let points = record.map(|r| hole_points(&r.sketch)).unwrap_or_default();
    panel::row(ui, text(T::Profile), |ui| {
        let previous = d.sketch;
        egui::ComboBox::from_id_salt("hole_sketch")
            .width(170.)
            .selected_text(record.map(|s| s.sketch.name.as_str()).unwrap_or("—"))
            .show_ui(ui, |ui| {
                for s in &part.sketches {
                    ui.selectable_value(&mut d.sketch, Some(s.id), &s.sketch.name);
                }
            });
        if d.sketch != previous {
            d.hole.kept_points = None;
        }
    });
    match &d.hole.kept_points {
        Some(kept) => {
            ui.label(format!("{}: {}", text(T::KeptReference), kept.len()));
        }
        None if points.is_empty() => {
            ui.colored_label(theme::WARNING, text(T::NoHolePoints));
        }
        None => {
            ui.label(format!("{}: {}", text(T::HolePoints), points.len()));
        }
    }
    let h = &mut d.hole;
    panel::row(ui, text(T::HoleType), |ui| {
        panel::choice(
            ui,
            &mut h.head,
            &[
                (0, Icon::Hole, T::HeadSimple),
                (1, Icon::Shell, T::Counterbore),
                (2, Icon::Chamfer, T::Countersink),
            ],
        );
    });
    panel::row(ui, text(T::HoleDiameter), |ui| {
        panel::expression(ui, &mut h.diameter)
    });
    if h.head > 0 {
        panel::row(ui, text(T::HeadDiameter), |ui| {
            panel::expression(ui, &mut h.head_diameter)
        });
        if h.head == 1 {
            panel::row(ui, text(T::HeadDepth), |ui| {
                panel::expression(ui, &mut h.head_depth)
            });
        } else {
            panel::row(ui, text(T::HeadAngle), |ui| {
                panel::expression(ui, &mut h.head_angle)
            });
        }
    }
    panel::row(ui, text(T::HoleExtent), |ui| {
        panel::choice(
            ui,
            &mut h.through,
            &[
                (false, Icon::Measure, T::ExtentDistance),
                (true, Icon::ToFace, T::Through),
            ],
        );
    });
    if !h.through {
        panel::row(ui, text(T::HoleDepth), |ui| {
            panel::expression(ui, &mut h.depth)
        });
        ui.checkbox(&mut h.tip, text(T::DrillPoint));
        if h.tip {
            panel::row(ui, text(T::Angle), |ui| {
                panel::expression(ui, &mut h.tip_angle)
            });
        }
    }
    ui.checkbox(&mut h.reverse, text(T::ReverseDirection));
    ui.checkbox(&mut h.thread, text(T::ThreadToggle));
    if h.thread {
        panel::row(ui, text(T::ThreadDesignation), |ui| {
            ui.text_edit_singleline(&mut h.thread_designation)
        });
        panel::row(ui, text(T::ThreadNominal), |ui| {
            panel::expression(ui, &mut h.thread_nominal)
        });
        panel::row(ui, text(T::ThreadPitch), |ui| {
            panel::expression(ui, &mut h.thread_pitch)
        });
    }
}

fn pattern_panel(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part) {
    body_selection(ui, d, part, Field::Base, T::BodyField);
    let circular = d.tool == Tool::CircularPattern;
    let l = &mut d.layout;
    panel::row(
        ui,
        text(if circular {
            T::AxisField
        } else {
            T::DirectionField
        }),
        |ui| axis_choice(ui, &mut l.axis, &AXES),
    );
    panel::row(ui, text(T::CountField), |ui| {
        ui.add(egui::DragValue::new(&mut l.count).range(2..=1000))
    });
    if circular {
        panel::row(ui, text(T::OriginField), |ui| {
            for v in &mut l.origin {
                ui.add(egui::DragValue::new(v).speed(0.5).max_decimals(3));
            }
        });
        panel::row(ui, text(T::TotalAngle), |ui| {
            panel::expression(ui, &mut l.angle)
        });
    } else {
        panel::row(ui, text(T::SpacingField), |ui| {
            panel::expression(ui, &mut l.spacing)
        });
        ui.checkbox(&mut l.second, text(T::SecondDirection));
        if l.second {
            panel::row(ui, text(T::DirectionField), |ui| {
                axis_choice(ui, &mut l.second_axis, &AXES)
            });
            panel::row(ui, text(T::CountField), |ui| {
                ui.add(egui::DragValue::new(&mut l.second_count).range(1..=1000))
            });
            panel::row(ui, text(T::SpacingField), |ui| {
                panel::expression(ui, &mut l.second_spacing)
            });
        }
    }
}

fn plane_panel(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part) {
    body_selection(ui, d, part, Field::Base, T::BodyField);
    let mirror = d.tool == Tool::Mirror;
    let l = &mut d.layout;
    panel::row(ui, text(T::PlaneField), |ui| {
        axis_choice(ui, &mut l.plane, &PLANES)
    });
    panel::row(ui, text(T::OffsetField), |ui| {
        ui.add(
            egui::DragValue::new(&mut l.offset)
                .speed(0.5)
                .suffix(" mm")
                .max_decimals(4),
        )
    });
    if mirror {
        ui.checkbox(&mut l.keep_original, text(T::KeepOriginal));
    } else {
        ui.checkbox(&mut l.flip, text(T::FlipSide));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrusion_limits_round_trip_through_the_panel_directions() {
        for (from, to, direction) in [
            ("0 mm", "espessura", Direction::OneSide),
            ("0 mm", "-(espessura)", Direction::OneSide),
            ("-(largura)", "largura", Direction::Symmetric),
            ("-(5 mm)", "12 mm", Direction::TwoSides),
            ("3 mm", "(3 mm) + (10 mm)", Direction::OneSide),
        ] {
            let draft = ExtrudeDraft::from_limits(from.into(), to.into());
            assert!(draft.custom.is_none(), "{from} → {to}");
            assert_eq!(draft.direction, direction);
            assert_eq!(draft.limits(), (from.to_owned(), to.to_owned()));
        }
        // Limites fora dos padrões do painel são preservados como estão.
        let custom = ExtrudeDraft::from_limits("2 mm".into(), "espessura".into());
        assert_eq!(custom.limits(), ("2 mm".to_owned(), "espessura".to_owned()));
        // Inverter troca o sentido sem perder a expressão digitada.
        let mut flipped = ExtrudeDraft::from_limits("0 mm".into(), "espessura".into());
        flipped.flip = true;
        assert_eq!(flipped.limits().1, "-(espessura)");
    }
}
