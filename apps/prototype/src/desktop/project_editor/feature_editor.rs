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
use nator_commands::solid_check;
use nator_document::PaintBoundary;
use nator_document::features::{
    BooleanMode, Combine, ContactRepair, EdgeRef, ExtrusionLimit, ExtrusionProfile, FaceRef,
    Feature, HoleHeadSpec, HoleSpec, LabelSpec, LinearAxis, Operation, PatternLayout, ProfileRef,
    RegionRef, SweepRoute, ThreadSpec, UntilFace, signature,
};
use nator_document::print_profile::{Fastener, HolePreset};
use nator_sketch::{Curve, Sketch, profiles};
mod extrusion;
mod extrusion_profiles;
pub(super) mod fastening;
pub(super) mod label;
mod measure;
mod revolve;

/// Ferramentas do painel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Tool {
    Extrude,
    Revolve,
    Loft,
    Sweep,
    Pipe,
    Label,
    Join,
    Cut,
    Intersect,
    Fillet,
    Chamfer,
    Shell,
    MergeFaces,
    RemoveFaces,
    Hole,
    LinearPattern,
    CircularPattern,
    Mirror,
    Split,
    CheckSolid,
    BedFace,
    ComponentCut,
    Lip,
    ScrewBoss,
    SnapFit,
}
impl Tool {
    pub(super) fn label(self) -> T {
        match self {
            Self::Extrude => T::Extrude,
            Self::Revolve => T::Revolve,
            Self::Loft => T::Loft,
            Self::Sweep => T::Sweep,
            Self::Pipe => T::Pipe,
            Self::Label => T::Label,
            Self::Join | Self::Cut | Self::Intersect => T::CombineTool,
            Self::Fillet => T::Fillet,
            Self::Chamfer => T::Chamfer,
            Self::Shell => T::Shell,
            Self::MergeFaces => T::MergeAreas,
            Self::RemoveFaces => T::RemoveFaces,
            Self::Hole => T::HoleTool,
            Self::LinearPattern => T::RectangularPattern,
            Self::CircularPattern => T::CircularPattern,
            Self::Mirror => T::MirrorBody,
            Self::Split => T::SplitBody,
            Self::CheckSolid => T::CheckSolid,
            Self::BedFace => T::BedFace,
            Self::ComponentCut => T::ComponentCut,
            Self::Lip => T::Lip,
            Self::ScrewBoss => T::ScrewBoss,
            Self::SnapFit => T::SnapFit,
        }
    }
    pub(super) fn icon(self) -> Icon {
        match self {
            Self::Extrude => Icon::Extrude,
            Self::Revolve => Icon::Revolve,
            Self::Loft => Icon::Loft,
            Self::Sweep => Icon::Sweep,
            Self::Pipe => Icon::Pipe,
            Self::Label => Icon::Label,
            Self::Hole => Icon::Hole,
            Self::Fillet => Icon::Fillet,
            Self::Chamfer => Icon::Chamfer,
            Self::Shell => Icon::Shell,
            Self::MergeFaces => Icon::Join,
            Self::RemoveFaces => Icon::Delete,
            Self::Mirror => Icon::Mirror,
            Self::LinearPattern | Self::CircularPattern => Icon::Pattern,
            Self::Split => Icon::Split,
            Self::Join | Self::Cut | Self::Intersect => Icon::Boolean,
            Self::CheckSolid => Icon::CheckSolid,
            Self::BedFace => Icon::BedFace,
            Self::ComponentCut => Icon::ComponentCut,
            Self::Lip => Icon::Lip,
            Self::ScrewBoss => Icon::ScrewBoss,
            Self::SnapFit => Icon::SnapFit,
        }
    }
    /// Ferramentas que partem de um rascunho de perfil.
    fn uses_profile(self) -> bool {
        matches!(
            self,
            Self::Extrude | Self::Revolve | Self::Loft | Self::Sweep
        )
    }
    /// Campo que recebe os primeiros cliques ao abrir o painel.
    fn first_field(self) -> Field {
        match self {
            Self::Extrude | Self::Revolve | Self::Loft | Self::Sweep => Field::Profile,
            Self::Pipe => Field::SweepPath,
            Self::Fillet | Self::Chamfer => Field::Edges,
            Self::Shell
            | Self::MergeFaces
            | Self::RemoveFaces
            | Self::Label
            | Self::BedFace
            | Self::Lip => Field::Faces,
            Self::ComponentCut => Field::Component,
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
    /// Face plana clicada e, na escolha do perfil, as condições da célula sob o cursor.
    PlanarFace(FaceRef, nator_sketch::Plane, Vec<PaintBoundary>),
    Edge(EdgeRef),
    /// Arestas que chegam ao vértice clicado em Arredondar ou Chanfrar.
    Edges(Vec<EdgeRef>),
    Face(FaceRef),
    Body(BodyId),
    /// Corpo de qualquer peça, com a posição da ocorrência clicada na montagem.
    Component(BodyId, nator_geometry::Placement),
}

/// O que o próximo clique na área 3D deve selecionar para o painel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Awaiting {
    Edge,
    Face,
    Body,
    Component,
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
    ExitFace,
    CuttingPlane,
    RevolveAxis,
    EndProfile,
    SweepAxis,
    SweepPath,
    Component,
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
    nut_flats: String,
    nut_depth: String,
    /// Direção de cima gravada na gota; `None` mantém a seção circular.
    teardrop: Option<[f64; 3]>,
    /// Posição do fixador na tabela do perfil.
    fastener: usize,
    reverse: bool,
    thread: bool,
    thread_designation: String,
    thread_nominal: String,
    thread_pitch: String,
    /// Pontos gravados na operação em edição; mantidos enquanto o rascunho não mudar.
    kept_points: Option<Vec<u64>>,
    direction_source: Option<(BodyId, SketchId)>,
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
            nut_flats: "5.5 mm".into(),
            nut_depth: "2.6 mm".into(),
            teardrop: None,
            fastener: 2,
            reverse: false,
            thread: false,
            thread_designation: "M6".into(),
            thread_nominal: "6 mm".into(),
            thread_pitch: "1 mm".into(),
            kept_points: None,
            direction_source: None,
        }
    }
}

impl HoleDraft {
    /// Preenche o painel com uma especificação gravada ou com um preset de fixação.
    fn load(&mut self, spec: HoleSpec) {
        self.diameter = spec.diameter;
        self.through = spec.depth.is_none();
        if let Some(depth) = spec.depth {
            self.depth = depth;
        }
        match spec.head {
            HoleHeadSpec::Simple => self.head = 0,
            HoleHeadSpec::Counterbore { diameter, depth } => {
                self.head = 1;
                self.head_diameter = diameter;
                self.head_depth = depth;
            }
            HoleHeadSpec::Countersink { diameter, angle } => {
                self.head = 2;
                self.head_diameter = diameter;
                self.head_angle = angle;
            }
            HoleHeadSpec::Nut {
                across_flats,
                depth,
            } => {
                self.head = 3;
                self.nut_flats = across_flats;
                self.nut_depth = depth;
            }
        }
        self.tip = spec.tip_angle.is_some();
        if let Some(angle) = spec.tip_angle {
            self.tip_angle = angle;
        }
        self.teardrop = spec.teardrop;
        self.reverse = spec.reverse;
        self.thread = spec.thread.is_some();
        if let Some(thread) = spec.thread {
            self.thread_designation = thread.designation;
            self.thread_nominal = thread.nominal;
            self.thread_pitch = thread.pitch;
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
    face_profile: Option<FaceArea>,
    extra_extrusion_profiles: Vec<ExtrusionProfile>,
    extrusion_profile_keys: std::collections::HashMap<String, u64>,
    extrusion_plane: Option<nator_sketch::Plane>,
    automatic: bool,
    auto_preferred: Option<BodyId>,
    interaction: extrusion::Interaction,
    revolve_axis: Option<(SketchId, u64)>,
    additional_profiles: Vec<ProfileRef>,
    end_profile: Option<ProfileRef>,
    ruled: bool,
    developable: bool,
    sweep_route: SweepRoute,
    pipe_thickness: Option<String>,
    hole_pairs: Option<Vec<nator_document::features::HolePair>>,
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
    through_shell: bool,
    exit_face: Option<FaceRef>,
    /// Resultado da extrusão: 0 novo corpo, 1 união, 2 corte, 3 interseção com `combine_target`.
    combine: usize,
    /// Na união, junta as faces novas às vizinhas no mesmo plano.
    merge_coplanar: bool,
    combine_target: Option<BodyId>,
    /// Face limite da extrusão (corpo e face), escolhida por clique.
    until: Option<ExtrusionLimit>,
    extrude: ExtrudeDraft,
    /// Medida principal das ferramentas de uma só dimensão (raio, distância, espessura).
    size: String,
    angle: String,
    hole: HoleDraft,
    layout: LayoutDraft,
    cutting_plane: Option<ExtrusionLimit>,
    fixed_plane: Option<nator_sketch::Plane>,
    label: LabelSpec,
    label_drag: Option<label::Drag>,
    measure: measure::Interaction,
    /// Contatos marcados para reparo, pelos nomes das faces em volta de cada um.
    contacts: Vec<Vec<String>>,
    repair_mode: ContactRepair,
    /// Análise do corpo base na revisão indicada; refeita quando o documento muda.
    report: Option<(u64, Result<solid_check::Report, String>)>,
    /// Tamanho na mesa para o corpo e a face indicados, calculado uma vez por escolha.
    bed_size: Option<(BodyId, FaceRef, Result<[f64; 3], String>)>,
    /// Remoção da orientação pedida no painel.
    clear_bed: bool,
    fastening: fastening::FasteningDraft,
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
            extra_extrusion_profiles: Vec::new(),
            extrusion_profile_keys: std::collections::HashMap::new(),
            extrusion_plane: None,
            automatic: tool == Tool::Extrude,
            auto_preferred: None,
            interaction: extrusion::Interaction::default(),
            revolve_axis: None,
            additional_profiles: vec![],
            end_profile: None,
            ruled: true,
            developable: true,
            sweep_route: if tool == Tool::Pipe {
                SweepRoute::Sketch {
                    sketch: SketchId(0),
                    entities: vec![],
                }
            } else {
                SweepRoute::Automatic
            },
            pipe_thickness: Some("1 mm".into()),
            hole_pairs: None,
            kept_axis: None,
            index: 0,
            kept_topology: None,
            extra_topology: Vec::new(),
            field: Some(tool.first_field()),
            chamfer_angle: None,
            outward: false,
            through_shell: false,
            exit_face: None,
            combine: 0,
            merge_coplanar: true,
            combine_target: None,
            until: None,
            extrude: ExtrudeDraft::default(),
            size: match tool {
                Tool::CheckSolid => "0.05 mm".into(),
                Tool::Pipe => "10 mm".into(),
                _ => "2 mm".into(),
            },
            angle: "360 deg".into(),
            hole: HoleDraft::default(),
            layout: LayoutDraft::default(),
            cutting_plane: None,
            fixed_plane: None,
            label: Default::default(),
            label_drag: None,
            measure: Default::default(),
            contacts: Vec::new(),
            repair_mode: ContactRepair::Weld,
            report: None,
            bed_size: None,
            clear_bed: false,
            fastening: Default::default(),
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
        let removing = matches!(feature.operation, Operation::RemoveFaces { .. });
        match feature.operation {
            Operation::Label { base, face, spec } => {
                d.tool = Tool::Label;
                d.base = Some(base);
                d.kept_topology = Some(Kept::Face(face));
                d.label = spec;
            }
            Operation::ExtrudeMany {
                profiles,
                profile_keys,
                from,
                to,
                combine,
                until,
            } => {
                d.extrusion_profile_keys = profiles
                    .iter()
                    .zip(profile_keys)
                    .map(|(p, key)| (format!("{p:?}"), key))
                    .collect();
                d.set_extrusion_profiles(profiles);
                d.extrude = ExtrudeDraft::from_limits(from, to);
                d.automatic = false;
                if let Some(combine) = combine {
                    d.combine = match combine.mode {
                        BooleanMode::Join => 1,
                        BooleanMode::Cut => 2,
                        BooleanMode::Intersect => 3,
                    };
                    d.combine_target = Some(combine.target);
                    d.merge_coplanar = combine.merge_coplanar;
                }
                d.extrude.to_face = until.is_some();
                d.until = until;
            }
            Operation::ExtrudeFace {
                base,
                face,
                from,
                to,
                combine,
                until,
                boundaries,
            } => {
                d.face_profile = Some(FaceArea {
                    body: base,
                    face,
                    boundaries,
                });
                d.extrude = ExtrudeDraft::from_limits(from, to);
                if let Some(combine) = combine {
                    d.combine = match combine.mode {
                        BooleanMode::Join => 1,
                        BooleanMode::Cut => 2,
                        BooleanMode::Intersect => 3,
                    };
                    d.combine_target = Some(combine.target);
                    d.merge_coplanar = combine.merge_coplanar;
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
                    d.merge_coplanar = combine.merge_coplanar;
                }
                d.extrude.to_face = until.is_some();
                d.until = until;
            }
            Operation::Pipe {
                sketch,
                entities,
                diameter,
                thickness,
            } => {
                d.tool = Tool::Pipe;
                d.sweep_route = SweepRoute::Sketch { sketch, entities };
                d.size = diameter;
                d.pipe_thickness = thickness;
                d.field = None;
            }
            Operation::Sweep {
                start,
                end,
                route,
                holes,
            } => {
                d.tool = Tool::Sweep;
                d.sketch = Some(start.sketch);
                d.kept_region = Some(start.region);
                d.end_profile = Some(end);
                d.sweep_route = route;
                d.hole_pairs = holes;
            }
            Operation::Loft {
                start,
                end,
                ruled,
                developable,
                holes,
            } => {
                d.tool = Tool::Loft;
                d.sketch = Some(start.sketch);
                d.kept_region = Some(start.region);
                d.end_profile = Some(end);
                d.ruled = ruled;
                d.developable = developable;
                d.hole_pairs = holes;
            }
            Operation::Revolve {
                sketch,
                region,
                angle,
                axis_line,
                axis_origin,
                axis_direction,
                axis_sketch,
                additional_profiles,
            } => {
                d.tool = Tool::Revolve;
                d.sketch = Some(sketch);
                d.kept_region = Some(region);
                d.angle = angle;
                d.additional_profiles = additional_profiles;
                match axis_line {
                    Some(line) => d.revolve_axis = Some((axis_sketch.unwrap_or(sketch), line)),
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
            Operation::RepairContacts {
                base,
                contacts,
                mode,
                radius,
            } => {
                d.tool = Tool::CheckSolid;
                d.base = Some(base);
                d.contacts = contacts;
                d.repair_mode = mode;
                d.size = radius;
            }
            Operation::Shell {
                base,
                face,
                thickness,
                additional_faces,
                outward,
                through,
                exit_face,
            } => {
                d.tool = Tool::Shell;
                d.base = Some(base);
                d.kept_topology = Some(Kept::Face(face));
                d.extra_topology = additional_faces.into_iter().map(Kept::Face).collect();
                d.size = thickness;
                d.outward = outward;
                d.through_shell = through;
                d.exit_face = exit_face;
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
                d.hole.kept_points = Some(points);
                d.hole.load(spec);
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
                cutting_plane,
                reverse,
            } => {
                d.tool = Tool::Split;
                d.base = Some(base);
                d.cutting_plane = cutting_plane;
                if d.cutting_plane.is_none() {
                    d.fixed_plane = nator_sketch::Plane::from_normal(origin, normal).ok();
                } else {
                    d.layout.flip = reverse;
                }
            }
            Operation::SplitRemainder { base, division } => {
                // As entradas da interface resolvem essa referência antes de
                // abrir o painel, para editar o plano da divisão primária.
                d.tool = Tool::Split;
                d.base = Some(base);
                d.output = Some(division);
            }
            Operation::ComponentCut {
                base,
                outline,
                clearance,
                floor,
                depth,
            } => {
                d.tool = Tool::ComponentCut;
                d.base = Some(base);
                let f = &mut d.fastening;
                f.keep_outline(outline);
                f.clearance = clearance;
                f.floor = floor;
                f.through = depth.is_none();
                f.depth = depth.unwrap_or_else(|| f.depth.clone());
            }
            Operation::MergeFaces { base, faces } | Operation::RemoveFaces { base, faces } => {
                d.tool = if removing {
                    Tool::RemoveFaces
                } else {
                    Tool::MergeFaces
                };
                d.base = Some(base);
                let mut faces = faces.into_iter().map(Kept::Face);
                d.kept_topology = faces.next();
                d.extra_topology = faces.collect();
            }
            Operation::Lip { base, face, spec } => {
                d.tool = Tool::Lip;
                d.base = Some(base);
                d.kept_topology = Some(Kept::Face(face));
                d.fastening.lip = spec;
            }
            Operation::ScrewBoss {
                base,
                sketch,
                points,
                spec,
            } => {
                d.tool = Tool::ScrewBoss;
                d.base = Some(base);
                d.sketch = Some(sketch);
                d.fastening.points = Some(points);
                d.fastening.boss = spec;
            }
            Operation::SnapFit {
                base,
                sketch,
                points,
                spec,
            } => {
                d.tool = Tool::SnapFit;
                d.base = Some(base);
                d.sketch = Some(sketch);
                d.fastening.points = Some(points);
                d.fastening.snap = spec;
            }
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
        Kept::PlanarFace(face, ..) => describe_topology(&Kept::Face(face.clone())),
        Kept::Edges(edges) => format!("{} × {}", text(T::EdgesField), edges.len()),
        Kept::Body(id) | Kept::Component(id, _) => format!("#{}", id.0),
    }
}

/// Pontos de um rascunho que posicionam furos: centros de círculos e pontos isolados.
fn hole_points(sketch: &Sketch) -> Vec<u64> {
    nator_commands::features::hole_points(sketch)
}

/// Face plana usada como perfil. Sem condições, a extrusão vale para a face inteira.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct FaceArea {
    pub(super) body: BodyId,
    pub(super) face: FaceRef,
    pub(super) boundaries: Vec<PaintBoundary>,
}

/// Referência de uma face plana clicada. Falha em faces curvas, que não são áreas;
/// `cell` acrescenta as condições dos rascunhos visíveis sobre a face.
pub(super) fn planar_area(
    document: &Document,
    pick: &nator_document::assembly::Pick,
    cell: bool,
) -> Result<Kept, String> {
    let face = document.face_at(pick).map_err(|e| e.to_string())?;
    let (plane, _) = sketch_scene::face_plane(document, pick, pick.placement)?;
    let boundaries = if cell {
        nator_commands::painting::boundaries(document, pick).map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };
    Ok(Kept::PlanarFace(face, plane, boundaries))
}

impl Draft {
    /// Tipo de elemento que o próximo clique deve selecionar, se o painel estiver esperando.
    pub(super) fn awaiting(&self) -> Option<Awaiting> {
        if self.tool == Tool::Extrude && matches!(self.field, None | Some(Field::Profile)) {
            return Some(Awaiting::Face);
        }
        match self.field? {
            Field::Edges => Some(Awaiting::Edge),
            Field::Faces | Field::UntilFace | Field::ExitFace | Field::CuttingPlane => {
                Some(Awaiting::Face)
            }
            Field::Base | Field::ToolBody | Field::Target => Some(Awaiting::Body),
            Field::Component => Some(Awaiting::Component),
            Field::Profile
            | Field::RevolveAxis
            | Field::EndProfile
            | Field::SweepAxis
            | Field::SweepPath => None,
        }
    }
    pub(super) fn choosing_cutting_plane(&self) -> bool {
        self.tool == Tool::Split && self.field == Some(Field::CuttingPlane)
    }
    pub(super) fn planar_pick(&self) -> bool {
        self.accepts_face_profile() || self.choosing_cutting_plane()
    }
    pub(super) fn cursor_icon(&self) -> Icon {
        self.tool.icon()
    }

    /// Extrusão esperando uma área: só regiões, células e faces planas respondem.
    pub(super) fn choosing_area(&self) -> bool {
        self.tool == Tool::Extrude && matches!(self.field, None | Some(Field::Profile))
    }
    /// Face ou célula escolhida como perfil.
    pub(super) fn face_area(&self) -> Option<&FaceArea> {
        self.face_profile.as_ref()
    }
    pub(super) fn choose_cutting_plane(
        &mut self,
        plane: Option<ExtrusionLimit>,
        fixed: Option<nator_sketch::Plane>,
    ) {
        self.cutting_plane = plane;
        self.fixed_plane = fixed;
        self.field = None;
    }
    pub(super) fn arrow_input_active(&self) -> bool {
        self.extrusion_input_active() || self.measure.input.active()
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
    /// Faces escolhidas, na ordem da seleção.
    fn chosen_faces(&self) -> Vec<FaceRef> {
        self.references()
            .filter_map(|kept| match kept {
                Kept::Face(face) => Some(face.clone()),
                _ => None,
            })
            .collect()
    }
    /// Região do perfil escolhida por clique no rascunho.
    pub(super) fn choose_region(&mut self, sketch: SketchId, region: RegionRef) {
        if matches!(self.tool, Tool::Loft | Tool::Sweep) {
            self.hole_pairs = None;
            if self.field == Some(Field::EndProfile) {
                self.end_profile = Some(ProfileRef { sketch, region });
                self.field = None;
            } else {
                self.sketch = Some(sketch);
                self.kept_region = Some(region);
                self.region = None;
                self.field = Some(Field::EndProfile);
            }
            return;
        }
        if self.tool == Tool::Revolve {
            if self.kept_region.is_none() {
                self.sketch = Some(sketch);
                self.kept_region = Some(region);
            } else if self.sketch == Some(sketch) {
                let mut selected = self
                    .kept_region
                    .take()
                    .map(|r| r.selections().to_vec())
                    .unwrap_or_default();
                if let Some(i) = selected.iter().position(|r| *r == region) {
                    selected.remove(i);
                } else {
                    selected.push(region);
                }
                self.kept_region = match selected.len() {
                    0 => None,
                    1 => selected.pop(),
                    _ => Some(RegionRef::Multiple(selected)),
                };
                if self.kept_region.is_none() && !self.additional_profiles.is_empty() {
                    let next = self.additional_profiles.remove(0);
                    self.sketch = Some(next.sketch);
                    self.kept_region = Some(next.region);
                }
            } else {
                let profile = ProfileRef { sketch, region };
                if let Some(i) = self.additional_profiles.iter().position(|p| *p == profile) {
                    self.additional_profiles.remove(i);
                } else {
                    self.additional_profiles.push(profile);
                }
            }
            self.region = None;
            self.field = Some(Field::Profile);
            return;
        }
        if self.tool == Tool::Extrude {
            self.toggle_extrusion_profile(ExtrusionProfile::Sketch(ProfileRef { sketch, region }));
        } else {
            self.face_profile = None;
            self.interaction = extrusion::Interaction::default();
            self.kept_region = Some(region);
            self.sketch = Some(sketch);
            self.region = None;
            self.field = None;
        }
    }

    pub(super) fn selected_profiles(&self) -> Vec<ProfileRef> {
        if self.tool == Tool::Extrude {
            return self
                .extrusion_profiles()
                .into_iter()
                .filter_map(|p| match p {
                    ExtrusionProfile::Sketch(p) => Some(p),
                    _ => None,
                })
                .collect();
        }
        self.sketch
            .zip(self.kept_region.clone())
            .map(|(sketch, region)| ProfileRef { sketch, region })
            .into_iter()
            .chain(self.additional_profiles.clone())
            .chain(self.end_profile.clone())
            .collect()
    }
    pub(super) fn requires_distinct_plane(&self) -> bool {
        matches!(self.tool, Tool::Loft | Tool::Sweep) && self.field == Some(Field::EndProfile)
    }
    pub(super) fn accepts_coplanar_profiles(&self) -> bool {
        self.tool == Tool::Revolve
    }
    pub(super) fn choose_sweep_path(&mut self, sketch: SketchId, entity: u64) {
        self.sweep_route = SweepRoute::Sketch {
            sketch,
            entities: vec![entity],
        };
        self.field = None;
    }
    pub(super) fn choose_revolve_axis(&mut self, sketch: SketchId, line: u64) {
        if self.tool == Tool::Sweep {
            self.sweep_route = SweepRoute::Axis {
                sketch,
                line,
                reverse: false,
            };
            self.field = None;
            return;
        }
        self.revolve_axis = Some((sketch, line));
        self.kept_axis = None;
        self.field = if self.kept_region.is_none() {
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
        if let Kept::Edges(edges) = kept {
            // Outro corpo troca a seleção; um vértice do mesmo corpo só acrescenta.
            for edge in edges {
                let edge = Kept::Edge(edge);
                if self.base != Some(body) || !self.references().any(|kept| *kept == edge) {
                    self.apply_pick(body, edge);
                }
            }
            return;
        }
        if let Kept::PlanarFace(face, plane, boundaries) = kept {
            if self.choosing_cutting_plane() {
                self.choose_cutting_plane(Some(UntilFace { body, face }.into()), None);
                return;
            }
            let support = self.extrusion_plane.or_else(|| {
                self.interaction
                    .guide
                    .and_then(|g| nator_sketch::Plane::from_normal(g.origin, g.normal).ok())
            });
            let coplanar = support.is_none_or(|support| {
                nator_geometry::norm(nator_geometry::cross(support.normal(), plane.normal())) < 1e-7
                    && nator_geometry::dot(
                        nator_geometry::sub(plane.origin, support.origin),
                        support.normal(),
                    )
                    .abs()
                        < 1e-6
            });
            if self.extrusion_profiles().is_empty() {
                self.extrusion_plane = Some(plane);
            }
            if (self.tool == Tool::Extrude && self.complete() && !coplanar)
                || self.field == Some(Field::UntilFace)
            {
                self.choose_limit(UntilFace { body, face }.into());
            } else if self.choosing_area() {
                self.choose_face_area(FaceArea {
                    body,
                    face,
                    boundaries,
                });
            } else {
                self.apply_pick(body, Kept::Face(face));
            }
            return;
        }
        if self.tool == Tool::Label && self.field == Some(Field::Faces) {
            self.base = Some(body);
            self.kept_topology = Some(kept);
            self.extra_topology.clear();
            self.field = None;
            return;
        }
        match (self.field, kept) {
            (Some(Field::ExitFace), Kept::Face(face)) => {
                if self.base == Some(body) {
                    self.exit_face = Some(face);
                    self.field = None;
                }
            }
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
                    self.contacts.clear();
                    self.report = None;
                }
                self.base = Some(id);
                self.field = match self.tool {
                    Tool::Join | Tool::Cut | Tool::Intersect if self.tool_body.is_none() => {
                        Some(Field::ToolBody)
                    }
                    Tool::Split => Some(Field::CuttingPlane),
                    Tool::ComponentCut if !self.fastening.has_outline() => Some(Field::Component),
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
            (Some(Field::Component), Kept::Component(body, pose)) => {
                self.fastening.component = Some((body, pose));
                self.fastening.outline = None;
                self.field = None;
            }
            // Só uma face pode ficar apoiada: o novo clique substitui a escolha.
            (Some(Field::Faces), kept @ Kept::Face(_))
                if matches!(self.tool, Tool::BedFace | Tool::Lip) =>
            {
                self.base = Some(body);
                self.kept_topology = Some(kept);
            }
            (Some(Field::Edges | Field::Faces), kept @ (Kept::Edge(_) | Kept::Face(_))) => {
                if self.base != Some(body) {
                    // Outro corpo: as escolhas anteriores deixam de valer.
                    self.base = Some(body);
                    self.exit_face = None;
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
    /// Corpo que ainda precisa ser analisado para o painel de verificação.
    fn pending_check(&self, revision: u64) -> Option<BodyId> {
        let current = self.report.as_ref().is_some_and(|r| r.0 == revision);
        self.base
            .filter(|_| self.tool == Tool::CheckSolid && !current)
    }
    /// Uma operação nova marca tudo o que a análise achou; ao editar, valem as marcas gravadas
    /// que ainda existem, para que um contato perdido não bloqueie a correção do reparo.
    pub(super) fn receive_check(
        &mut self,
        tab: u64,
        revision: u64,
        body: BodyId,
        report: Result<solid_check::Report, String>,
    ) {
        if self.tab != tab || self.base != Some(body) {
            return;
        }
        if let Ok(report) = &report {
            let found = report.contacts.iter().map(|c| c.faces.clone());
            if self.output.is_none() {
                self.contacts = found.collect();
            } else {
                let found = found.collect::<Vec<_>>();
                self.contacts.retain(|faces| found.contains(faces));
            }
        }
        self.report = Some((revision, report));
    }
    /// Calcula o tamanho só quando o corpo ou a face mudam, não a cada quadro.
    fn refresh_bed_size(&mut self, document: &Document) {
        let (Tool::BedFace, Some(body), Some(Kept::Face(face))) =
            (self.tool, self.base, &self.kept_topology)
        else {
            return;
        };
        if self
            .bed_size
            .as_ref()
            .is_some_and(|(b, f, _)| *b == body && f == face)
        {
            return;
        }
        let size = document
            .body(body)
            .ok_or_else(|| text(T::BodyField).to_owned())
            .and_then(|record| record.print_direction(face).map_err(|e| e.to_string()))
            .and_then(|down| {
                nator_commands::export::size_on_bed(document, body, Some(down))
                    .ok_or_else(|| text(T::Failed).to_owned())
            });
        self.bed_size = Some((body, face.clone(), size));
    }
    /// Polilinhas dos contatos analisados, em coordenadas da peça, para o realce 3D.
    pub(super) fn contact_lines(&self) -> impl Iterator<Item = &Vec<nator_geometry::Point3>> {
        self.report
            .iter()
            .filter_map(|(_, report)| report.as_ref().ok())
            .flat_map(|report| &report.contacts)
            .flat_map(|contact| &contact.segments)
    }
    /// Lascas analisadas, em coordenadas da peça, marcadas na cena.
    pub(super) fn sliver_points(&self) -> impl Iterator<Item = nator_geometry::Point3> {
        self.report
            .iter()
            .filter_map(|(_, report)| report.as_ref().ok())
            .flat_map(|report| &report.slivers)
            .map(|sliver| sliver.center)
    }
    pub(super) fn describe(kept: &Kept) -> String {
        describe_topology(kept)
    }
    pub(super) fn choose_face_profile(&mut self, body: BodyId, face: FaceRef) {
        self.set_face_area(FaceArea {
            body,
            face,
            boundaries: Vec::new(),
        });
    }
    /// Clicar de novo na área escolhida a retira, como nas regiões de rascunho.
    fn choose_face_area(&mut self, area: FaceArea) {
        self.toggle_extrusion_profile(ExtrusionProfile::Face {
            base: area.body,
            face: area.face,
            boundaries: area.boundaries,
        });
    }

    fn set_face_area(&mut self, area: FaceArea) {
        self.extra_extrusion_profiles.clear();
        self.extrusion_plane = None;
        self.auto_preferred = Some(area.body);
        self.face_profile = Some(area);
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

    /// Sugestão de apresentação; a direção concreta continua gravada na operação.
    fn suggest_hole_direction(&mut self, part: &nator_document::Part) {
        if self.tool != Tool::Hole || self.output.is_some() {
            return;
        }
        let Some(pair) = self.base.zip(self.sketch) else {
            return;
        };
        if self.hole.direction_source == Some(pair) {
            return;
        }
        let Some(body) = part.bodies.iter().find(|b| b.id == pair.0) else {
            return;
        };
        let Some(record) = part.sketches.iter().find(|s| s.id == pair.1) else {
            return;
        };
        let center = body.placement.apply(body.geometry.mesh().bounds().center());
        self.hole.reverse = nator_geometry::dot(
            nator_geometry::sub(center, record.sketch.plane.origin),
            record.sketch.plane.normal(),
        ) > 0.;
        self.hole.direction_source = Some(pair);
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
                3 => HoleHeadSpec::Nut {
                    across_flats: h.nut_flats.clone(),
                    depth: h.nut_depth.clone(),
                },
                _ => HoleHeadSpec::Simple,
            },
            tip_angle: (h.tip && !h.through).then(|| h.tip_angle.clone()),
            teardrop: h.teardrop,
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
        let normal = self
            .fixed_plane
            .map(|p| p.normal())
            .unwrap_or(PLANES[self.layout.plane].1);
        let origin = self
            .fixed_plane
            .map(|p| p.origin)
            .unwrap_or_else(|| normal.map(|v| v * self.layout.offset));
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
        if !self.extrusion_input_valid() || !self.measure.input.valid() {
            return false;
        }
        match self.tool {
            Tool::Extrude => {
                (self.face_profile.is_some()
                    || (self.sketch.is_some()
                        && (self.kept_region.is_some() || self.region.is_some())))
                    && (!self.extrude.to_face || self.until.is_some())
                    && (self.automatic || self.combine == 0 || self.combine_target.is_some())
            }
            Tool::Pipe => {
                matches!(&self.sweep_route, SweepRoute::Sketch {sketch, entities} if sketch.0 != 0 && !entities.is_empty())
            }
            Tool::Sweep => {
                self.sketch.is_some()
                    && self.kept_region.is_some()
                    && self.end_profile.is_some()
                    && match &self.sweep_route {
                        SweepRoute::Automatic => true,
                        SweepRoute::Axis { sketch, line, .. } => sketch.0 != 0 && *line != 0,
                        SweepRoute::Sketch { sketch, entities } => {
                            sketch.0 != 0 && !entities.is_empty()
                        }
                    }
            }
            Tool::Loft => {
                self.sketch.is_some() && self.kept_region.is_some() && self.end_profile.is_some()
            }
            Tool::Revolve => {
                self.sketch.is_some()
                    && (self.kept_region.is_some() || self.region.is_some())
                    && (self.revolve_axis.is_some() || self.kept_axis.is_some())
            }
            Tool::Join | Tool::Cut | Tool::Intersect => {
                self.base.is_some() && self.tool_body.is_some() && self.base != self.tool_body
            }
            Tool::Fillet
            | Tool::Chamfer
            | Tool::Shell
            | Tool::RemoveFaces
            | Tool::Label
            | Tool::BedFace
            | Tool::Lip => self.base.is_some() && self.kept_topology.is_some(),
            Tool::MergeFaces => {
                self.base.is_some()
                    && self.kept_topology.is_some()
                    && !self.extra_topology.is_empty()
            }
            Tool::Hole | Tool::ScrewBoss | Tool::SnapFit => {
                self.base.is_some() && self.sketch.is_some()
            }
            Tool::ComponentCut => self.base.is_some() && self.fastening.has_outline(),
            Tool::Split => {
                self.base.is_some() && (self.cutting_plane.is_some() || self.fixed_plane.is_some())
            }
            Tool::CheckSolid => self.base.is_some() && !self.contacts.is_empty(),
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
                        merge_coplanar: self.combine == 1 && self.merge_coplanar,
                    });
                let until = self.until.clone().filter(|_| self.extrude.to_face);
                let profiles = self.extrusion_profiles();
                let same_sketch = profiles
                    .first()
                    .and_then(|p| match p {
                        ExtrusionProfile::Sketch(p) => Some(p.sketch),
                        _ => None,
                    })
                    .filter(|id| {
                        profiles
                            .iter()
                            .all(|p| matches!(p,ExtrusionProfile::Sketch(p) if p.sketch==*id))
                    });
                if profiles.len() > 1 && same_sketch.is_none() {
                    let profile_keys = profiles
                        .iter()
                        .map(|p| {
                            self.extrusion_profile_keys
                                .get(&format!("{p:?}"))
                                .copied()
                                .unwrap_or_else(|| signature(format!("{p:?}").as_bytes()))
                        })
                        .collect();
                    return Operation::ExtrudeMany {
                        profiles,
                        profile_keys,
                        from,
                        to,
                        combine,
                        until,
                    };
                }
                if let Some(sketch) = same_sketch
                    && profiles.len() > 1
                {
                    let regions = profiles
                        .into_iter()
                        .filter_map(|p| match p {
                            ExtrusionProfile::Sketch(p) => Some(p.region),
                            _ => None,
                        })
                        .collect();
                    return Operation::Extrude {
                        sketch,
                        region: RegionRef::Multiple(regions),
                        from,
                        to,
                        combine,
                        until,
                    };
                }
                if let Some(area) = &self.face_profile {
                    return Operation::ExtrudeFace {
                        base: area.body,
                        face: area.face.clone(),
                        from,
                        to,
                        combine,
                        until,
                        boundaries: area.boundaries.clone(),
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
            Tool::Label => Operation::Label {
                base: self.base.unwrap_or(BodyId(0)),
                face,
                spec: self.label.clone(),
            },
            Tool::Pipe => {
                let (sketch, entities) = match &self.sweep_route {
                    SweepRoute::Sketch { sketch, entities } => (*sketch, entities.clone()),
                    _ => (SketchId(0), vec![]),
                };
                Operation::Pipe {
                    sketch,
                    entities,
                    diameter: self.size.clone(),
                    thickness: self.pipe_thickness.clone(),
                }
            }
            Tool::Sweep => Operation::Sweep {
                start: ProfileRef { sketch, region },
                end: self.end_profile.clone().unwrap_or(ProfileRef {
                    sketch: SketchId(0),
                    region: RegionRef::Index(0),
                }),
                route: self.sweep_route.clone(),
                holes: self.hole_pairs.clone(),
            },
            Tool::Loft => Operation::Loft {
                start: ProfileRef { sketch, region },
                end: self.end_profile.clone().unwrap_or(ProfileRef {
                    sketch: SketchId(0),
                    region: RegionRef::Index(0),
                }),
                ruled: self.ruled,
                developable: self.developable,
                holes: self.hole_pairs.clone(),
            },
            Tool::Revolve => {
                let plane = record.map(|r| r.sketch.plane).unwrap_or_default();
                let (axis_origin, axis_direction) =
                    self.kept_axis.unwrap_or((plane.origin, plane.v));
                Operation::Revolve {
                    sketch,
                    region,
                    axis_origin,
                    axis_direction,
                    angle: self.angle.clone(),
                    axis_line: self.revolve_axis.map(|(_, line)| line),
                    axis_sketch: self.revolve_axis.map(|(sketch, _)| sketch),
                    additional_profiles: self.additional_profiles.clone(),
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
            // `BedFace` não entra no histórico: `command` responde antes de chegar aqui.
            Tool::CheckSolid | Tool::BedFace => Operation::RepairContacts {
                base,
                contacts: self.contacts.clone(),
                mode: self.repair_mode,
                radius: self.size.clone(),
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
                through: self.through_shell,
                exit_face: self.exit_face.clone(),
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
            Tool::ComponentCut => Operation::ComponentCut {
                base,
                outline: self.fastening.captured(),
                clearance: self.fastening.clearance.clone(),
                floor: self.fastening.floor.clone(),
                depth: (!self.fastening.through).then(|| self.fastening.depth.clone()),
            },
            Tool::MergeFaces => Operation::MergeFaces {
                base,
                faces: self.chosen_faces(),
            },
            Tool::RemoveFaces => Operation::RemoveFaces {
                base,
                faces: self.chosen_faces(),
            },
            Tool::Lip => Operation::Lip {
                base,
                face,
                spec: self.fastening.lip.clone(),
            },
            Tool::ScrewBoss | Tool::SnapFit => {
                let points =
                    self.fastening.points.clone().unwrap_or_else(|| {
                        record.map(|r| hole_points(&r.sketch)).unwrap_or_default()
                    });
                if self.tool == Tool::ScrewBoss {
                    Operation::ScrewBoss {
                        base,
                        sketch,
                        points,
                        spec: self.fastening.boss.clone(),
                    }
                } else {
                    Operation::SnapFit {
                        base,
                        sketch,
                        points,
                        spec: self.fastening.snap.clone(),
                    }
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
                    cutting_plane: self.cutting_plane.clone(),
                    reverse: self.layout.flip,
                }
            }
        }
    }

    /// Comando que o OK aplicaria, também usado como chave da pré-visualização.
    pub(super) fn preview_key(&self, part: &nator_document::Part) -> Command {
        self.command(part)
    }
    /// Medida principal (distância da extrusão, raio, espessura), para capturas de ensaio.
    pub(super) fn preset_hole_for_capture(&mut self) {
        let m3 = nator_document::print_profile::PrintProfile::default().fasteners[2].clone();
        self.hole.load(HolePreset::CaptiveNut.spec(&m3));
    }
    /// Rascunho dos pontos e fixador comum, para capturas de torres e presilhas.
    pub(super) fn prepare_fastening_for_capture(&mut self, sketch: SketchId) {
        self.sketch = Some(sketch);
        let m3 = nator_document::print_profile::PrintProfile::default().fasteners[2].clone();
        self.fastening.load_boss(HolePreset::HeatInsert, &m3);
    }
    pub(super) fn set_size_for_capture(&mut self, value: &str) {
        self.size = value.into();
        self.extrude.distance = value.into();
    }
    /// Comando que o OK aplicaria.
    fn command(&self, part: &nator_document::Part) -> Command {
        if self.tool == Tool::BedFace {
            return Command::SetPrintFace {
                body: self.base.unwrap_or(BodyId(0)),
                face: match &self.kept_topology {
                    Some(Kept::Face(face)) => Some(face.clone()),
                    _ => None,
                },
            };
        }
        let operation = self.operation(part);
        if self.tool == Tool::Split && self.output.is_none() {
            return Command::SplitBody {
                part: self.part,
                name: text(T::SplitBody).into(),
                operation,
            };
        }
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
                    Tool::CheckSolid => T::RepairContact,
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
    pub(super) fn edit_cad_feature(&mut self, part: PartId, feature: Feature) {
        let feature = if let Operation::SplitRemainder { division, .. } = feature.operation {
            self.current()
                .session
                .document()
                .part(part)
                .and_then(|p| p.features.iter().find(|f| f.output == division))
                .cloned()
        } else {
            Some(feature)
        };
        if let Some(feature) = feature {
            self.feature_edit = Some(Draft::edit(self.current().id, part, feature));
        }
    }
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
            Tool::Pipe => {}
            Tool::Extrude | Tool::Revolve | Tool::Loft | Tool::Sweep => {
                d.combine_target = suggestion;
                d.auto_preferred = suggestion;
            }
            Tool::Fillet
            | Tool::Chamfer
            | Tool::Shell
            | Tool::MergeFaces
            | Tool::RemoveFaces
            | Tool::Label
            | Tool::BedFace
            | Tool::Lip => {
                d.base = suggestion;
            }
            Tool::ComponentCut => {
                d.base = suggestion;
                d.field = Some(if suggestion.is_none() {
                    Field::Base
                } else {
                    Field::Component
                });
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
            | Tool::ScrewBoss
            | Tool::SnapFit
            | Tool::LinearPattern
            | Tool::CircularPattern
            | Tool::Mirror
            | Tool::Split
            | Tool::CheckSolid => {
                d.base = suggestion;
                d.field = if suggestion.is_none() {
                    Some(Field::Base)
                } else if tool == Tool::Split {
                    Some(Field::CuttingPlane)
                } else {
                    None
                };
            }
        }
        if matches!(tool, Tool::Shell | Tool::Label | Tool::BedFace | Tool::Lip)
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
            d.capture_label_pick(pick);
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
        // A face já selecionada vale pela célula do clique, que exige o núcleo.
        if draft.choosing_area() {
            let part = draft.part();
            let Some(pick) = self.current().picked.clone().filter(|p| {
                p.part == part
                    && self.pick_mode == navigation::PickMode::Face
                    && self.current().sketch_edit.is_none()
                    && self
                        .current()
                        .is_selected(Selection::Body { part, id: p.body })
            }) else {
                return;
            };
            let tab = self.current().id;
            let revision = self.current().session.revision();
            let document = self.current().session.document().clone();
            self.start(context, None, move || {
                let kept = planar_area(&document, &pick, true).ok();
                Ok(Outcome::Picked {
                    tab,
                    revision,
                    pick: Some(pick),
                    mode: navigation::PickMode::Face,
                    additive: false,
                    topology: kept.map(Ok),
                })
            });
            return;
        }
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
            let topology = nator_commands::topology::edge_at(&document, &pick)
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
        d.suggest_hole_direction(&part);
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
        let revision = self
            .tabs
            .iter()
            .find(|t| t.id == d.tab)
            .map_or(0, |t| t.session.revision());
        if let Some(body) = d.pending_check(revision)
            && !self.busy()
        {
            self.check_solid(ctx, d.tab, d.part, body);
        }
        let mut bed = None;
        let mut fasteners = Vec::new();
        let mut max_strain_pct = 0.;
        if let Some(document) = self
            .tabs
            .iter()
            .find(|t| t.id == d.tab)
            .map(|t| t.session.document())
        {
            d.refresh_bed_size(document);
            bed = document.print_profile().bed_mm;
            if matches!(d.tool, Tool::Hole | Tool::ScrewBoss) {
                fasteners = document.print_profile().fasteners.clone();
            }
            max_strain_pct = document.print_profile().max_strain_pct;
            if let Some(tab) = self.tabs.iter().find(|t| t.id == d.tab)
                && let Ok(target) = sketch_scene::part_placement(tab, d.part)
            {
                d.fastening.refresh_outline(document, target);
            }
        }
        let command = d.command(&part);
        if d.tool == Tool::Label {
            if let Some(spec) = self.normalized_label_preview(&command) {
                d.label.outline = spec.outline;
                d.label.outline_key = spec.outline_key;
                if d.label_drag.is_none() {
                    d.label.position = spec.position;
                    d.label.uv = spec.uv;
                    d.label.normal = spec.normal;
                }
            }
            self.label_fonts.poll(ctx);
        }
        let complete = d.complete() && nator_kernel::is_available();
        let footer_error;
        let footer = if !complete {
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
            complete
                && !self.busy()
                && (!d.zero_extrusion(&part)
                    || matches!(self.preview_status(&command), PreviewStatus::Ready)),
            |ui| {
                if !nator_kernel::is_available() {
                    ui.colored_label(theme::warning(), text(T::KernelUnavailable));
                }
                if self
                    .pending_failure
                    .as_ref()
                    .is_some_and(|p| p.fixing && Some(p.failure.output) == d.output)
                {
                    ui.colored_label(theme::warning(), text(T::FixingFailure));
                }
                match d.tool {
                    Tool::Label => label::panel(ui, &mut d, &self.label_fonts.names),
                    Tool::Extrude => {
                        extrude_panel(ui, &mut d, &part, &regions);
                        if d.zero_extrusion(&part) {
                            ui.label(
                                egui::RichText::new(text(T::ZeroExtrusionHint))
                                    .small()
                                    .color(theme::muted()),
                            );
                        }
                    }
                    Tool::Revolve => revolve_panel(ui, &mut d, &part, &regions, record),
                    Tool::Pipe => pipe_panel(ui, &mut d, &part),
                    Tool::Loft | Tool::Sweep => loft_panel(ui, &mut d, &part, &regions),
                    Tool::Join | Tool::Cut | Tool::Intersect => combine_panel(ui, &mut d, &part),
                    Tool::Fillet
                    | Tool::Chamfer
                    | Tool::Shell
                    | Tool::MergeFaces
                    | Tool::RemoveFaces => topology_panel(ui, &mut d, &part),
                    Tool::Hole => hole_panel(ui, &mut d, &part, record, &fasteners),
                    Tool::LinearPattern | Tool::CircularPattern => pattern_panel(ui, &mut d, &part),
                    Tool::Mirror | Tool::Split => plane_panel(ui, &mut d, &part),
                    Tool::CheckSolid => check_panel(ui, &mut d, &part),
                    Tool::BedFace => bed_panel(ui, &mut d, &part, bed),
                    Tool::ComponentCut => fastening::component_panel(ui, &mut d, &part),
                    Tool::Lip => fastening::lip_panel(ui, &mut d),
                    Tool::ScrewBoss => fastening::boss_panel(ui, &mut d, &part, record, &fasteners),
                    Tool::SnapFit => {
                        fastening::snap_panel(ui, &mut d, &part, record, max_strain_pct)
                    }
                }
            },
        );
        let tab = d.tab;
        d.suggest_hole_direction(&part);
        // Os campos podem mudar neste mesmo quadro (inclusive ao clicar em OK).
        // Publicar e pedir prévia sempre usa o estado posterior à edição do formulário.
        let command = d.command(&part);
        let complete = d.complete() && nator_kernel::is_available();
        if outcome.cancel {
            self.close_feature_panel(&d);
            return;
        }
        if d.clear_bed
            && let Some(body) = d.base
        {
            self.prepare_command(ctx, tab, Command::SetPrintFace { body, face: None });
            self.close_feature_panel(&d);
            return;
        }
        if outcome.ok && complete {
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

    /// A análise lê os B-reps do histórico: fica no trabalhador, como as medições.
    fn check_solid(&mut self, context: &egui::Context, tab: u64, part: PartId, body: BodyId) {
        let Some(target) = self.tabs.iter().find(|t| t.id == tab) else {
            return;
        };
        let document = target.session.document().clone();
        let revision = target.session.revision();
        self.start(context, None, move || {
            Ok(Outcome::SolidChecked {
                tab,
                revision,
                body,
                report: solid_check::inspect(&document, part, body).map_err(|e| e.to_string()),
            })
        });
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
    let chosen = if d.tool == Tool::Extrude {
        d.extrusion_profiles().len()
    } else {
        d.kept_region
            .as_ref()
            .map(|r| r.selections().len())
            .unwrap_or_else(|| usize::from(d.face_profile.is_some() || d.region.is_some()))
            + d.additional_profiles
                .iter()
                .map(|p| p.region.selections().len())
                .sum::<usize>()
    };
    let sketch_name = d
        .sketch
        .and_then(|id| part.sketches.iter().find(|s| s.id == id))
        .map(|s| s.sketch.name.clone())
        .or_else(|| {
            d.face_profile
                .as_ref()
                .and_then(|area| body_name(part, Some(area.body)))
        });
    let selection = panel::selection(
        ui,
        text(if matches!(d.tool, Tool::Loft | Tool::Sweep) {
            T::StartProfile
        } else {
            T::ProfileField
        }),
        chosen,
        sketch_name.as_deref(),
        d.field == Some(Field::Profile),
    );
    if selection.activate {
        d.field = Some(Field::Profile);
    }
    if selection.clear {
        d.face_profile = None;
        d.extra_extrusion_profiles.clear();
        d.extrusion_plane = None;
        d.interaction = extrusion::Interaction::default();
        d.kept_region = None;
        d.additional_profiles.clear();
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
            .color(theme::muted()),
        );
    }
    match regions {
        Some(Err(message)) => {
            ui.colored_label(theme::warning(), message);
        }
        Some(Ok(list)) if list.is_empty() => {
            ui.colored_label(theme::warning(), text(T::NoRegions));
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
            .color(theme::muted()),
    );
    let e = &mut d.extrude;
    if let Some((from, to)) = &mut e.custom {
        panel::row(ui, text(T::FromField), |ui| {
            panel::length_expression(ui, from)
        });
        panel::row(ui, text(T::ToField), |ui| panel::length_expression(ui, to));
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
                panel::length_expression(ui, &mut e.distance);
                if icons::button(ui, Icon::Flip, text(T::FlipDirection), false, e.flip).clicked() {
                    e.flip = !e.flip;
                }
            });
            if e.direction == Direction::TwoSides {
                panel::row(ui, text(T::Distance2Field), |ui| {
                    panel::length_expression(ui, &mut e.distance2)
                });
            }
        }
        let e = &mut d.extrude;
        if e.direction == Direction::OneSide {
            panel::row(ui, text(T::StartOffset), |ui| {
                panel::length_expression(ui, &mut e.offset)
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
    if d.combine == 1 && !d.automatic {
        ui.checkbox(&mut d.merge_coplanar, text(T::MergeCoplanarAreas));
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

/// Seletores independentes de início/fim, sem exigir planos paralelos.
fn loft_panel(
    ui: &mut egui::Ui,
    d: &mut Draft,
    part: &nator_document::Part,
    regions: &Option<Result<Arc<Vec<profiles::Region>>, String>>,
) {
    profile_rows(ui, d, part, regions);
    let name = d
        .end_profile
        .as_ref()
        .and_then(|p| part.sketches.iter().find(|s| s.id == p.sketch))
        .map(|s| s.sketch.name.as_str());
    let selection = panel::selection(
        ui,
        text(T::EndProfile),
        usize::from(d.end_profile.is_some()),
        name,
        d.field == Some(Field::EndProfile),
    );
    if selection.activate {
        d.field = Some(Field::EndProfile);
    }
    if selection.clear {
        d.end_profile = None;
        d.field = Some(Field::EndProfile);
    }
    if d.tool == Tool::Loft {
        ui.checkbox(&mut d.ruled, text(T::RuledLoft));
        ui.checkbox(&mut d.developable, text(T::DevelopableLoft))
            .on_hover_text(text(T::DevelopableLoftHint));
    } else {
        sweep_controls(ui, d, part);
    }
    if d.hole_pairs.is_some() && ui.button(text(T::UpdateHolePairs)).clicked() {
        d.hole_pairs = None;
    }
    ui.label(
        egui::RichText::new(text(if d.tool == Tool::Loft {
            T::LoftHint
        } else {
            T::SweepHint
        }))
        .small()
        .color(theme::muted()),
    );
}

fn pipe_panel(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part) {
    let (sketch, count) = match &d.sweep_route {
        SweepRoute::Sketch { sketch, entities } => (*sketch, entities.len()),
        _ => (SketchId(0), 0),
    };
    let name = part
        .sketches
        .iter()
        .find(|s| s.id == sketch)
        .map(|s| s.sketch.name.as_str());
    let selection = panel::selection(
        ui,
        text(T::RouteField),
        count,
        name,
        d.field == Some(Field::SweepPath),
    );
    if selection.activate {
        d.field = Some(Field::SweepPath);
    }
    if selection.clear {
        d.sweep_route = SweepRoute::Sketch {
            sketch: SketchId(0),
            entities: vec![],
        };
        d.field = Some(Field::SweepPath);
    }
    panel::row(ui, text(T::PipeDiameter), |ui| {
        panel::length_expression(ui, &mut d.size)
    });
    let mut hollow = d.pipe_thickness.is_some();
    if ui.checkbox(&mut hollow, text(T::PipeHollow)).changed() {
        d.pipe_thickness = hollow.then(|| "1 mm".into());
    }
    if let Some(thickness) = &mut d.pipe_thickness {
        panel::row(ui, text(T::ThicknessField), |ui| {
            panel::length_expression(ui, thickness)
        });
    }
    ui.label(
        egui::RichText::new(text(T::PipeHint))
            .small()
            .color(theme::muted()),
    );
}

fn sweep_controls(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part) {
    let mut mode = match d.sweep_route {
        SweepRoute::Automatic => 0,
        SweepRoute::Axis { .. } => 1,
        SweepRoute::Sketch { .. } => 2,
    };
    panel::row(ui, text(T::RouteField), |ui| {
        if panel::choice(
            ui,
            &mut mode,
            &[
                (0, Icon::Sweep, T::AutomaticRoute),
                (1, Icon::Line, T::AxisField),
                (2, Icon::Sketch, T::Sketch),
            ],
        ) {
            d.sweep_route = match mode {
                1 => SweepRoute::Axis {
                    sketch: SketchId(0),
                    line: 0,
                    reverse: false,
                },
                2 => SweepRoute::Sketch {
                    sketch: SketchId(0),
                    entities: vec![],
                },
                _ => SweepRoute::Automatic,
            };
            d.field = match mode {
                1 => Some(Field::SweepAxis),
                2 => Some(Field::SweepPath),
                _ => None,
            };
        }
    });
    let (field, chosen, sketch) = match &d.sweep_route {
        SweepRoute::Automatic => return,
        SweepRoute::Axis { sketch, line, .. } => {
            (Field::SweepAxis, usize::from(*line != 0), *sketch)
        }
        SweepRoute::Sketch { sketch, entities } => (Field::SweepPath, entities.len(), *sketch),
    };
    let name = part
        .sketches
        .iter()
        .find(|s| s.id == sketch)
        .map(|s| s.sketch.name.as_str());
    let selection = panel::selection(
        ui,
        text(if mode == 1 {
            T::AxisField
        } else {
            T::RouteField
        }),
        chosen,
        name,
        d.field == Some(field),
    );
    if selection.activate {
        d.field = Some(field);
    }
    if selection.clear {
        d.sweep_route = if mode == 1 {
            SweepRoute::Axis {
                sketch: SketchId(0),
                line: 0,
                reverse: false,
            }
        } else {
            SweepRoute::Sketch {
                sketch: SketchId(0),
                entities: vec![],
            }
        };
        d.field = Some(field);
    }
    if let SweepRoute::Axis { reverse, .. } = &mut d.sweep_route {
        ui.checkbox(reverse, text(T::LongArc));
    }
}

fn revolve_panel(
    ui: &mut egui::Ui,
    d: &mut Draft,
    part: &nator_document::Part,
    regions: &Option<Result<Arc<Vec<profiles::Region>>, String>>,
    _record: Option<&nator_document::SketchRecord>,
) {
    profile_rows(ui, d, part, regions);
    let description = d
        .revolve_axis
        .map(|(sketch, line)| {
            let name = part
                .sketches
                .iter()
                .find(|s| s.id == sketch)
                .map(|s| s.sketch.name.as_str())
                .unwrap_or("—");
            format!("{name} · {} #{line}", text(T::Entity))
        })
        .or_else(|| d.kept_axis.map(|_| text(T::KeptReference).into()));
    let selected = d.revolve_axis.is_some() || d.kept_axis.is_some();
    let selection = panel::selection(
        ui,
        text(T::AxisField),
        usize::from(selected),
        description.as_deref(),
        d.field == Some(Field::RevolveAxis),
    );
    if selection.activate {
        d.field = Some(Field::RevolveAxis);
    }
    if selection.clear {
        d.revolve_axis = None;
        d.kept_axis = None;
        d.field = Some(Field::RevolveAxis);
    }
    if d.field == Some(Field::RevolveAxis) {
        ui.label(
            egui::RichText::new(text(T::RevolveAxisHint))
                .small()
                .color(theme::muted()),
        );
    }
    let mut full = d.angle.trim() == "360 deg";
    panel::row(ui, text(T::Angle), |ui| {
        ui.add_enabled_ui(!full, |ui| panel::expression(ui, &mut d.angle));
        if ui.checkbox(&mut full, text(T::FullTurn)).changed() {
            d.angle = if full { "360 deg" } else { "180 deg" }.into();
        }
    });
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
    let field = if matches!(d.tool, Tool::Shell | Tool::MergeFaces | Tool::RemoveFaces) {
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
            ui.label(egui::RichText::new(name).color(theme::muted()))
        });
    }
    match d.tool {
        Tool::Fillet => {
            panel::row(ui, text(T::RadiusField), |ui| {
                panel::length_expression(ui, &mut d.size)
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
                panel::length_expression(ui, &mut d.size)
            });
            if let Some(angle) = &mut d.chamfer_angle {
                panel::row(ui, text(T::Angle), |ui| panel::expression(ui, angle));
            }
        }
        // Sem medidas: as faces escolhidas bastam.
        Tool::MergeFaces => {}
        Tool::RemoveFaces => {
            ui.label(
                egui::RichText::new(text(T::RemoveFacesHint))
                    .small()
                    .color(theme::muted()),
            );
        }
        _ => {
            panel::row(ui, text(T::ThicknessField), |ui| {
                panel::length_expression(ui, &mut d.size)
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
    if d.tool == Tool::Shell {
        ui.checkbox(&mut d.through_shell, text(T::ThroughShell));
        if d.through_shell {
            let description = d
                .exit_face
                .as_ref()
                .map(|f| describe_topology(&Kept::Face(f.clone())));
            let selection = panel::selection(
                ui,
                text(T::ExitFace),
                usize::from(d.exit_face.is_some()),
                description.as_deref(),
                d.field == Some(Field::ExitFace),
            );
            if selection.activate {
                d.field = Some(Field::ExitFace);
            }
            if selection.clear {
                d.exit_face = None;
                d.field = Some(Field::ExitFace);
            }
            ui.label(
                egui::RichText::new(text(T::AutomaticExitHint))
                    .small()
                    .color(theme::muted()),
            );
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

/// Fixação pronta para peças impressas: preenche os campos, que continuam editáveis.
fn hole_presets(ui: &mut egui::Ui, d: &mut Draft, fasteners: &[Fastener]) {
    let label = |preset: HolePreset| {
        text(match preset {
            HolePreset::Through => T::PresetThrough,
            HolePreset::RecessedHead => T::PresetRecessed,
            HolePreset::SelfTapping => T::PresetSelfTap,
            HolePreset::HeatInsert => T::PresetInsert,
            HolePreset::CaptiveNut => T::PresetNut,
        })
    };
    let mut chosen = None;
    panel::row(ui, text(T::HolePresetField), |ui| {
        egui::ComboBox::from_id_salt("hole_fastener")
            .width(60.)
            .selected_text(
                fasteners
                    .get(d.hole.fastener)
                    .map_or("—", |f| f.name.as_str()),
            )
            .show_ui(ui, |ui| {
                for (index, fastener) in fasteners.iter().enumerate() {
                    ui.selectable_value(&mut d.hole.fastener, index, &fastener.name);
                }
            });
        egui::ComboBox::from_id_salt("hole_preset")
            .width(110.)
            .selected_text(text(T::HolePresetChoose))
            .show_ui(ui, |ui| {
                for preset in HolePreset::ALL {
                    if ui.button(label(preset)).clicked() {
                        chosen = Some(preset);
                    }
                }
            });
    });
    if let (Some(preset), Some(fastener)) = (chosen, fasteners.get(d.hole.fastener)) {
        d.hole.load(preset.spec(fastener));
    }
}

fn hole_panel(
    ui: &mut egui::Ui,
    d: &mut Draft,
    part: &nator_document::Part,
    record: Option<&nator_document::SketchRecord>,
    fasteners: &[Fastener],
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
            ui.colored_label(theme::warning(), text(T::NoHolePoints));
        }
        None => {
            ui.label(format!("{}: {}", text(T::HolePoints), points.len()));
        }
    }
    hole_presets(ui, d, fasteners);
    // Cima na impressão: o oposto da face na mesa, ou Z enquanto ela não foi escolhida.
    let up = d
        .base
        .and_then(|id| part.bodies.iter().find(|b| b.id == id))
        .and_then(|b| b.print_down.map(|down| b.placement.apply_vector(down)))
        .map_or([0., 0., 1.], |down| down.map(|v| -v));
    let h = &mut d.hole;
    panel::row(ui, text(T::HoleType), |ui| {
        panel::choice(
            ui,
            &mut h.head,
            &[
                (0, Icon::Hole, T::HeadSimple),
                (1, Icon::Shell, T::Counterbore),
                (2, Icon::Chamfer, T::Countersink),
                (3, Icon::Pattern, T::CaptiveNutHead),
            ],
        );
    });
    panel::row(ui, text(T::HoleDiameter), |ui| {
        panel::length_expression(ui, &mut h.diameter)
    });
    if h.head == 3 {
        panel::row(ui, text(T::NutFlats), |ui| {
            panel::length_expression(ui, &mut h.nut_flats)
        });
        panel::row(ui, text(T::NutDepth), |ui| {
            panel::length_expression(ui, &mut h.nut_depth)
        });
    } else if h.head > 0 {
        panel::row(ui, text(T::HeadDiameter), |ui| {
            panel::length_expression(ui, &mut h.head_diameter)
        });
        if h.head == 1 {
            panel::row(ui, text(T::HeadDepth), |ui| {
                panel::length_expression(ui, &mut h.head_depth)
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
            panel::length_expression(ui, &mut h.depth)
        });
        ui.add_enabled_ui(h.teardrop.is_none(), |ui| {
            ui.checkbox(&mut h.tip, text(T::DrillPoint));
        });
        if h.tip {
            panel::row(ui, text(T::Angle), |ui| {
                panel::expression(ui, &mut h.tip_angle)
            });
        }
    }
    let mut drop = h.teardrop.is_some();
    ui.add_enabled_ui(matches!(h.head, 0 | 3), |ui| {
        if ui
            .checkbox(&mut drop, text(T::TeardropHole))
            .on_hover_text(text(T::TeardropHint))
            .changed()
        {
            h.teardrop = drop.then_some(up);
            h.tip &= !drop;
        }
    });
    // Rebaixo e escareado não têm seção em gota.
    if !matches!(h.head, 0 | 3) {
        h.teardrop = None;
    }
    ui.checkbox(&mut h.reverse, text(T::ReverseDirection));
    ui.checkbox(&mut h.thread, text(T::ThreadToggle));
    if h.thread {
        panel::row(ui, text(T::ThreadDesignation), |ui| {
            ui.text_edit_singleline(&mut h.thread_designation)
        });
        panel::row(ui, text(T::ThreadNominal), |ui| {
            panel::length_expression(ui, &mut h.thread_nominal)
        });
        panel::row(ui, text(T::ThreadPitch), |ui| {
            panel::length_expression(ui, &mut h.thread_pitch)
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
            panel::length_expression(ui, &mut l.spacing)
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
                panel::length_expression(ui, &mut l.second_spacing)
            });
        }
    }
}

/// Face apoiada e o tamanho resultante. A orientação vale só para a impressão.
fn bed_panel(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part, bed: Option<[f64; 3]>) {
    let description = d.kept_topology.as_ref().map(describe_topology);
    let s = panel::selection(
        ui,
        text(T::BedFaceField),
        usize::from(d.kept_topology.is_some()),
        description.as_deref(),
        d.field == Some(Field::Faces),
    );
    if s.activate {
        d.field = Some(Field::Faces);
    }
    if s.clear {
        d.kept_topology = None;
        d.bed_size = None;
        d.field = Some(Field::Faces);
    }
    let body = d
        .base
        .and_then(|id| part.bodies.iter().find(|b| b.id == id));
    if let Some(body) = body {
        panel::row(ui, text(T::BodyField), |ui| {
            ui.label(egui::RichText::new(&body.name).color(theme::muted()))
        });
    }
    panel::hint(ui, text(T::BedFaceHint));
    if let Some((.., Ok(size))) = &d.bed_size {
        let settings = panel::settings(ui.ctx());
        let [x, y, z] = size.map(|v| settings.format_compact_length(v));
        panel::row(ui, text(T::BedSize), |ui| {
            ui.label(format!("{x} × {y} × {z} {}", settings.unit.suffix()))
        });
        if bed.is_some_and(|b| (0..3).any(|i| size[i] > b[i])) {
            ui.colored_label(theme::warning(), text(T::BedTooSmall));
        }
    }
    if body.is_some_and(|b| b.print_down.is_some()) && ui.button(text(T::BedRemove)).clicked() {
        d.clear_bed = true;
    }
}

/// Resultado da análise e, havendo contatos, as escolhas do reparo.
fn check_panel(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part) {
    body_selection(ui, d, part, Field::Base, T::BodyField);
    if d.base.is_none() {
        d.contacts.clear();
        d.report = None;
        panel::hint(ui, text(T::CheckSolidHint));
        return;
    }
    let report = match d.report.as_ref().map(|(_, report)| report) {
        None => {
            panel::hint(ui, text(T::CheckingSolid));
            return;
        }
        Some(Err(error)) => {
            ui.colored_label(theme::warning(), error);
            return;
        }
        Some(Ok(report)) => report,
    };
    for sliver in &report.slivers {
        let settings = panel::settings(ui.ctx());
        let [x, y, z] = sliver.center.map(|v| settings.format_compact_length(v));
        ui.colored_label(theme::warning(), text(T::SolidSliver));
        panel::hint(ui, &format!("X {x}   Y {y}   Z {z}"));
        panel::hint(
            ui,
            &format!(
                "{} {:.7} mm · {}",
                text(T::SolidSliverWidth),
                sliver.width,
                sliver.face
            )
            .replace('.', ","),
        );
        if let Some(origin) = &sliver.origin {
            let sketch = origin
                .sketch
                .as_ref()
                .map(|name| format!(" · {name}"))
                .unwrap_or_default();
            panel::hint(
                ui,
                &format!(
                    "{}: {}. {}{sketch}",
                    text(T::ContactOrigin),
                    origin.step,
                    origin.name
                ),
            );
        }
        panel::hint(ui, text(T::SolidSliverHint));
    }
    if report.contacts.is_empty() {
        panel::hint(
            ui,
            text(if report.unexplained_mesh_defects() {
                T::SolidMeshDefects
            } else {
                T::SolidHasNoDefects
            }),
        );
        return;
    }
    let settings = panel::settings(ui.ctx());
    let length = |value| panel::Quantity::Length.compact(value, settings);
    let mut toggled = None;
    for (index, contact) in report.contacts.iter().enumerate() {
        let mut marked = d.contacts.contains(&contact.faces);
        let label = format!(
            "{} {} · {} {}",
            text(T::Contact),
            index + 1,
            length(contact.length),
            panel::Quantity::Length.unit(settings)
        );
        if ui.checkbox(&mut marked, label).changed() {
            toggled = Some((contact.faces.clone(), marked));
        }
        if let Some([x, y, z]) = contact.segments.first().and_then(|s| s.first()).copied() {
            panel::hint(
                ui,
                &format!("X {}   Y {}   Z {}", length(x), length(y), length(z)),
            );
        }
        if let Some(origin) = &contact.origin {
            let sketch = origin
                .sketch
                .as_ref()
                .map(|name| format!(" · {name}"))
                .unwrap_or_default();
            panel::hint(
                ui,
                &format!(
                    "{}: {}. {}{sketch}",
                    text(T::ContactOrigin),
                    origin.step,
                    origin.name
                ),
            );
        }
    }
    match toggled {
        Some((faces, true)) => d.contacts.push(faces),
        Some((faces, false)) => d.contacts.retain(|c| *c != faces),
        None => {}
    }
    panel::row(ui, text(T::RepairModeField), |ui| {
        panel::choice_hints(
            ui,
            &mut d.repair_mode,
            &[
                (
                    ContactRepair::Weld,
                    Icon::Boolean,
                    T::WeldContact,
                    T::WeldContactHint,
                ),
                (
                    ContactRepair::Separate,
                    Icon::Split,
                    T::SeparateContact,
                    T::SeparateContactHint,
                ),
            ],
        )
    });
    panel::row(ui, text(T::RadiusField), |ui| {
        panel::length_expression(ui, &mut d.size)
    });
}

fn plane_panel(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part) {
    body_selection(ui, d, part, Field::Base, T::BodyField);
    let mirror = d.tool == Tool::Mirror;
    if !mirror {
        let chosen = d.cutting_plane.is_some() || d.fixed_plane.is_some();
        let description = match &d.cutting_plane {
            Some(ExtrusionLimit::Face(UntilFace { face, .. })) => {
                Some(describe_topology(&Kept::Face(face.clone())))
            }
            Some(ExtrusionLimit::WorkPlane { work_plane }) => part
                .work_planes
                .iter()
                .find(|w| w.id == *work_plane)
                .map(|w| w.name.clone()),
            Some(ExtrusionLimit::Sketch { sketch }) => part
                .sketches
                .iter()
                .find(|s| s.id == *sketch)
                .map(|s| s.sketch.name.clone()),
            None => d.fixed_plane.map(|_| text(T::Origin).into()),
        };
        let selection = panel::selection(
            ui,
            text(T::CuttingPlane),
            usize::from(chosen),
            description.as_deref(),
            d.field == Some(Field::CuttingPlane),
        );
        if selection.activate {
            d.field = Some(Field::CuttingPlane);
        }
        if selection.clear {
            d.cutting_plane = None;
            d.fixed_plane = None;
            d.field = Some(Field::CuttingPlane);
        }
        ui.label(
            egui::RichText::new(text(T::CuttingPlaneHint))
                .small()
                .color(theme::muted()),
        );
        ui.checkbox(&mut d.layout.flip, text(T::FlipSide));
        return;
    }
    let l = &mut d.layout;
    panel::row(ui, text(T::PlaneField), |ui| {
        axis_choice(ui, &mut l.plane, &PLANES)
    });
    panel::row(ui, text(T::OffsetField), |ui| {
        let context = ui.ctx().clone();
        ui.add(panel::length_value(&context, &mut l.offset, -1e7..=1e7))
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

    fn check_report(body: BodyId, faces: &[&[&str]]) -> solid_check::Report {
        solid_check::Report {
            slivers: Vec::new(),
            body,
            contacts: faces
                .iter()
                .map(|names| solid_check::Contact {
                    faces: names.iter().map(|n| (*n).to_owned()).collect(),
                    segments: vec![vec![[0.; 3], [0., 0., 1.]]],
                    length: 1.,
                    origin: None,
                })
                .collect(),
            mesh: nator_geometry::Topology {
                boundary_edges: 0,
                non_manifold_edges: faces.len(),
                inconsistent_edges: 0,
            },
        }
    }

    #[test]
    fn fastening_operations_come_back_unchanged_from_the_panel()
    -> Result<(), Box<dyn std::error::Error>> {
        use nator_document::features::{ComponentOutline, LipKind, LipReference, LipSpec};
        let mut document = Document::new("Painel")?;
        let mesh = std::sync::Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?));
        let (id, _) = document.add_mesh(None, "Malha".into(), mesh)?;
        let part = document.part(id).ok_or("Peça")?.clone();
        let body = BodyId(7);
        let (u, v) = ([0., 1., 0.], [0., 0., 1.]);
        let outline = ComponentOutline {
            source: "Conector".into(),
            plane: nator_sketch::Plane {
                origin: [5., 0., 0.],
                u,
                v,
            },
            contours: vec![vec![[0., 0.], [4., 0.], [4., 3.]]],
        };
        let defaults = fastening::FasteningDraft::default();
        let spec = LipSpec {
            kind: LipKind::Groove,
            reference: LipReference::Outer,
            ..defaults.lip.clone()
        };
        for operation in [
            Operation::ComponentCut {
                base: body,
                outline,
                clearance: "folga_livre".into(),
                floor: "1 mm".into(),
                depth: Some("7 mm".into()),
            },
            Operation::Lip {
                base: body,
                face: FaceRef::Named("b7:rim".into()),
                spec,
            },
            Operation::ScrewBoss {
                base: body,
                sketch: SketchId(3),
                points: vec![11, 12],
                spec: defaults.boss.clone(),
            },
            Operation::SnapFit {
                base: body,
                sketch: SketchId(3),
                points: vec![13],
                spec: defaults.snap.clone(),
            },
        ] {
            let feature = Feature {
                name: "Fixação".into(),
                output: BodyId(8),
                operation: operation.clone(),
                suppressed: false,
            };
            let draft = Draft::edit(1, PartId(2), feature);
            assert!(draft.complete(), "{operation:?}");
            assert_eq!(
                format!("{:?}", draft.operation(&part)),
                format!("{operation:?}")
            );
        }
        Ok(())
    }

    #[test]
    fn the_component_tool_waits_for_the_silhouette_before_allowing_ok() {
        let mut draft = Draft::new(1, PartId(2), Tool::ComponentCut);
        draft.base = Some(BodyId(7));
        assert_eq!(draft.field, Some(Field::Component));
        assert_eq!(draft.awaiting(), Some(Awaiting::Component));
        assert!(!draft.complete());
        draft.apply_pick(BodyId(9), Kept::Component(BodyId(9), Default::default()));
        assert!(draft.field.is_none());
        // O clique sozinho ainda não capturou a silhueta.
        assert!(!draft.complete());
    }

    #[test]
    fn hole_presets_fill_the_panel_and_come_back_unchanged() {
        use nator_document::print_profile::PrintProfile;
        let profile = PrintProfile::default();
        for fastener in &profile.fasteners {
            for preset in HolePreset::ALL {
                let spec = preset.spec(fastener);
                let mut draft = Draft::new(1, PartId(2), Tool::Hole);
                draft.hole.load(spec.clone());
                assert_eq!(draft.hole_spec(), spec, "{preset:?} {}", fastener.name);
            }
        }
        // Uma gota gravada volta com a direção original.
        let mut draft = Draft::new(1, PartId(2), Tool::Hole);
        draft.hole.load(HoleSpec {
            teardrop: Some([0., 1., 0.]),
            ..HolePreset::Through.spec(&profile.fasteners[0])
        });
        assert_eq!(draft.hole_spec().teardrop, Some([0., 1., 0.]));
    }

    #[test]
    fn new_checks_mark_every_contact_and_edits_keep_what_still_exists() {
        const FIRST: &[&str] = &["a", "b", "c", "d"];
        const SECOND: &[&str] = &["e", "f", "g", "h"];
        let body = BodyId(7);
        let mut draft = Draft::new(1, PartId(2), Tool::CheckSolid);
        draft.base = Some(body);
        assert!(!draft.complete());
        assert_eq!(draft.pending_check(5), Some(body));
        draft.receive_check(1, 5, body, Ok(check_report(body, &[FIRST, SECOND])));
        assert_eq!(draft.contacts.len(), 2);
        assert!(draft.complete());
        assert_eq!(draft.pending_check(5), None);
        // Revisão nova refaz a análise; resposta de outra aba ou corpo é ignorada.
        assert_eq!(draft.pending_check(6), Some(body));
        draft.receive_check(2, 6, body, Ok(check_report(body, &[])));
        draft.receive_check(1, 6, BodyId(8), Ok(check_report(body, &[])));
        assert_eq!(draft.contacts.len(), 2);

        let feature = Feature {
            name: "Reparo".into(),
            output: BodyId(9),
            operation: Operation::RepairContacts {
                base: body,
                contacts: vec![
                    FIRST.iter().map(|n| (*n).to_owned()).collect(),
                    SECOND.iter().map(|n| (*n).to_owned()).collect(),
                ],
                mode: ContactRepair::Separate,
                radius: "0.1 mm".into(),
            },
            suppressed: false,
        };
        let mut edit = Draft::edit(1, PartId(2), feature);
        assert!(edit.tool == Tool::CheckSolid && edit.repair_mode == ContactRepair::Separate);
        assert_eq!(edit.size, "0.1 mm");
        edit.receive_check(1, 5, body, Ok(check_report(body, &[SECOND])));
        assert_eq!(edit.contacts.len(), 1);
        edit.receive_check(1, 6, body, Ok(check_report(body, &[])));
        assert!(!edit.complete());
    }

    #[test]
    fn shell_exit_is_independent_from_openings_and_stays_in_the_same_body() {
        let mut draft = Draft::new(1, PartId(1), Tool::Shell);
        let entry = FaceRef::Named("base:end".into());
        let exit = FaceRef::Named("base:start".into());
        draft.apply_pick(BodyId(2), Kept::Face(entry.clone()));
        draft.through_shell = true;
        draft.field = Some(Field::ExitFace);
        draft.apply_pick(BodyId(3), Kept::Face(exit.clone()));
        assert!(draft.exit_face.is_none());
        draft.apply_pick(BodyId(2), Kept::Face(exit.clone()));
        assert_eq!(draft.exit_face, Some(exit));
        assert_eq!(draft.kept_topology, Some(Kept::Face(entry)));
        assert!(draft.extra_topology.is_empty());
        assert!(draft.complete());
    }

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
