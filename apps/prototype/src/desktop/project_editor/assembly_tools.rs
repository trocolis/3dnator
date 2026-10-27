//! Ferramentas de montagem e posicionamento em painéis de comando, no padrão dos CADs de
//! mercado: Mover/Copiar com manipulador 3D, Alinhar por face, aresta ou vértice, padrões e
//! espelhamento de componentes. Na montagem a unidade é a instância; na construção, o corpo
//! da peça ativa. Toda alteração é um comando (ou um lote, em uma só ação de desfazer) e
//! passa pela prévia ao vivo; nenhum vínculo permanente é criado entre peças.
use super::{
    command_panel::{self as panel, Footer},
    icons::{self, Icon},
    live_preview::PreviewStatus,
    *,
};
use nator_document::{
    assembly::Pick,
    features::{LinearAxis, PatternLayout},
};
use nator_geometry::{cross, dot, norm, sub};

/// Eixos principais do mundo.
const AXES: [Point3; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
const AXIS_NAMES: [&str; 3] = ["X", "Y", "Z"];
const PLANES: [(&str, Point3); 3] = [
    ("XY", [0., 0., 1.]),
    ("XZ", [0., 1., 0.]),
    ("YZ", [1., 0., 0.]),
];
/// Comprimento das setas do manipulador, em pontos de tela.
const ARROW: f32 = 82.;

/// Painel de montagem aberto.
pub(super) enum Panel {
    /// Texto dos seis campos e estado do manipulador ficam em caixa para manter o enum leve.
    Move(Box<MoveDraft>),
    /// Guarda duas seleções completas; em caixa, para não inflar as demais variantes.
    Align(Box<AlignDraft>),
    Pattern(PatternDraft),
    Mirror(MirrorDraft),
}

impl Panel {
    fn numbers_valid(&self) -> bool {
        match self {
            Self::Move(d) => d.numbers_valid(),
            Self::Pattern(d) if d.circular => d
                .origin_inputs
                .iter()
                .all(|i| i.is_valid(panel::Quantity::Length)),
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Pivot {
    /// Centro de massa dos elementos (densidade uniforme; caixa para malhas abertas).
    Center,
    /// Origem do primeiro elemento.
    Origin,
    /// Ponto escolhido na área 3D (vértice, aresta ou face, conforme o modo de seleção),
    /// capturado ao escolher a opção.
    Point(Point3),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Handle {
    Axis(usize),
    Ring(usize),
    Free,
}

struct GizmoDrag {
    handle: Handle,
    start: egui::Pos2,
    translation: Point3,
    rotation: Point3,
    angle: f32,
}

/// As ferramentas da aba Peça mantêm a intenção escolhida. O menu de contexto
/// conserva a alternância Mover/Copiar que já existia.
enum MoveMode {
    Move,
    Copy,
    MoveOrCopy(bool),
}
impl MoveMode {
    fn copies(&self) -> bool {
        matches!(self, Self::Copy | Self::MoveOrCopy(true))
    }
}

pub(super) struct MoveDraft {
    tab: u64,
    targets: Vec<Selection>,
    /// O campo Objetos recebe os cliques no modelo.
    picking: bool,
    translation: Point3,
    /// Graus em torno dos eixos do mundo, aplicados em X, Y e Z, pelo pivô.
    rotation: Point3,
    /// Texto em edição separado dos valores usados na prévia, inclusive fora do foco.
    translation_inputs: [panel::NumberInput; 3],
    rotation_inputs: [panel::NumberInput; 3],
    pivot: Pivot,
    mode: MoveMode,
    drag: Option<GizmoDrag>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AlignField {
    From,
    To,
}

pub(super) struct AlignDraft {
    tab: u64,
    kind: navigation::PickMode,
    from: Option<Pick>,
    to: Option<Anchor>,
    field: Option<AlignField>,
    flip: bool,
}

/// Destino de um alinhamento: geometria do modelo ou de um rascunho de referência da
/// montagem (ponto, linha ou plano). Só a posição é usada; nenhum vínculo é guardado.
#[derive(Debug, Clone)]
pub(super) struct Anchor {
    point: Point3,
    normal: Point3,
    edge: [Point3; 2],
    label: String,
    /// Veio de um rascunho de referência (desenhado na cor das referências).
    reference: bool,
}
impl Anchor {
    fn from_pick(pick: &Pick, label: String) -> Self {
        Self {
            point: pick.point,
            normal: pick.normal,
            edge: [pick.vertices[0], pick.vertices[1]],
            label,
            reference: false,
        }
    }
}

/// Distância de `p` ao segmento `a`–`b` na tela e o parâmetro do ponto mais próximo.
fn segment_distance(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> (f32, f32) {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_sq().max(1e-6)).clamp(0., 1.);
    (p.distance(a + d * t), t)
}

/// Geometria de rascunhos de referência sob o cursor, conforme o tipo de alinhamento:
/// ponto mais próximo (12 pt), linha mais próxima (8 pt) ou o plano dentro da extensão do
/// rascunho.
fn reference_anchor(
    document: &Document,
    camera: &Camera,
    bounds: nator_geometry::Bounds,
    rect: egui::Rect,
    position: egui::Pos2,
    kind: navigation::PickMode,
) -> Option<Anchor> {
    let mut best: Option<(f32, Anchor)> = None;
    let mut offer = |distance: f32, anchor: Anchor| {
        if best.as_ref().is_none_or(|(d, _)| distance < *d) {
            best = Some((distance, anchor));
        }
    };
    for record in document
        .reference_sketches()
        .iter()
        .filter(|s| document.shows_reference_sketch(s))
    {
        let sketch = &record.sketch;
        let frame = sketch_scene::Frame {
            camera: camera.clone(),
            bounds,
            rect,
            placement: Placement::default(),
            plane: sketch.plane,
        };
        let normal = sketch.plane.normal();
        match kind {
            navigation::PickMode::Vertex => {
                let Ok(points) = nator_commands::point_snap::reference_points(sketch) else {
                    continue;
                };
                for point in points {
                    if let Some(screen) = frame.screen(point) {
                        let distance = screen.distance(position);
                        if distance < 12. {
                            let world = frame.world(point);
                            offer(
                                distance,
                                Anchor {
                                    point: world,
                                    normal,
                                    edge: [world; 2],
                                    label: sketch.name.clone(),
                                    reference: true,
                                },
                            );
                        }
                    }
                }
            }
            navigation::PickMode::Edge => {
                for entity in &sketch.entities {
                    let Ok((a, b)) = sketch.line(entity.id) else {
                        continue;
                    };
                    let (Some(sa), Some(sb)) = (frame.screen(a), frame.screen(b)) else {
                        continue;
                    };
                    let (distance, t) = segment_distance(position, sa, sb);
                    if distance < 8. {
                        let (wa, wb) = (frame.world(a), frame.world(b));
                        let t = f64::from(t);
                        offer(
                            distance,
                            Anchor {
                                point: std::array::from_fn(|i| wa[i] + (wb[i] - wa[i]) * t),
                                normal,
                                edge: [wa, wb],
                                label: sketch.name.clone(),
                                reference: true,
                            },
                        );
                    }
                }
            }
            navigation::PickMode::Face => {
                let (Some(hit), Some((min, max))) =
                    (frame.cursor(position), sketch_scene::sketch_limits(sketch))
                else {
                    continue;
                };
                // A região útil do plano é a extensão do rascunho, com folga de 20 %.
                let margin = (0..2)
                    .map(|k| (max[k] - min[k]) * 0.2)
                    .fold(10_f64, f64::max);
                if (0..2).all(|k| hit[k] >= min[k] - margin && hit[k] <= max[k] + margin) {
                    let world = frame.world(hit);
                    offer(
                        f32::MAX / 2.,
                        Anchor {
                            point: world,
                            normal,
                            edge: [world; 2],
                            label: sketch.name.clone(),
                            reference: true,
                        },
                    );
                }
            }
        }
    }
    best.map(|(_, anchor)| anchor)
}

pub(super) struct PatternDraft {
    tab: u64,
    target: Option<InstanceId>,
    picking: bool,
    circular: bool,
    axis: usize,
    count: u32,
    spacing: String,
    angle: String,
    origin: Point3,
    origin_inputs: [panel::NumberInput; 3],
}

pub(super) struct MirrorDraft {
    tab: u64,
    target: Option<InstanceId>,
    picking: bool,
    plane: usize,
    offset: f64,
}

/// Posição atual de um elemento: local (instância relativa ao pai, corpo relativo à peça) e
/// no mundo (ocorrência).
struct Frame {
    item: Selection,
    local: Placement,
    world: Placement,
}

fn frames(tab: &Tab, items: &[Selection]) -> Vec<Frame> {
    let document = tab.session.document();
    let Ok(occurrences) = document.occurrences() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| match *item {
            Selection::Instance(id) => {
                let local = document.instance(id)?.placement;
                let world = occurrences.iter().find(|o| o.instance == id)?.placement;
                Some(Frame {
                    item: *item,
                    local,
                    world,
                })
            }
            Selection::Body { part, id } => {
                let local = document.body(id)?.placement;
                let parent = sketch_scene::part_placement(tab, part).ok()?;
                Some(Frame {
                    item: *item,
                    local,
                    world: parent.compose(local).ok()?,
                })
            }
        })
        .collect()
}

/// Centro de massa local de um conjunto de corpos (malhas abertas usam a caixa).
fn bodies_center<'a>(bodies: impl Iterator<Item = &'a nator_document::Body>) -> Option<Point3> {
    let mut sum = [0.; 3];
    let mut weight = 0.;
    for body in bodies {
        let mesh = body.geometry.mesh();
        let (center, volume) = if body.geometry.topology().has_closed_oriented_edges() {
            mesh.mass_center().unwrap_or((mesh.bounds().center(), 1.))
        } else {
            (mesh.bounds().center(), 1.)
        };
        let p = body.placement.apply(center);
        let w = volume.abs().max(1e-9);
        for k in 0..3 {
            sum[k] += p[k] * w;
        }
        weight += w;
    }
    (weight > 0.).then(|| sum.map(|v| v / weight))
}

/// Ponto do pivô no mundo.
fn pivot_point(tab: &Tab, frames: &[Frame], pivot: Pivot) -> Option<Point3> {
    let document = tab.session.document();
    match pivot {
        Pivot::Origin => return frames.first().map(|f| f.world.apply([0.; 3])),
        Pivot::Point(point) => return Some(point),
        Pivot::Center => {}
    }
    let mut sum = [0.; 3];
    let mut count = 0.;
    for frame in frames {
        let center = match frame.item {
            Selection::Instance(id) => {
                let part = document.instance(id).and_then(|i| document.part(i.part))?;
                // Centro em coordenadas da peça, levado ao mundo pela ocorrência.
                bodies_center(part.shown_bodies()).map(|c| frame.world.apply(c))
            }
            Selection::Body { id, .. } => {
                let body = document.body(id)?;
                let parent = frame.world.compose(frame.local.inverse().ok()?).ok()?;
                bodies_center(std::iter::once(body)).map(|c| parent.apply(c))
            }
        };
        if let Some(c) = center {
            for k in 0..3 {
                sum[k] += c[k];
            }
            count += 1.;
        }
    }
    (count > 0.).then(|| sum.map(|v| v / count))
}

/// Transformação rígida no mundo: rotações X, Y e Z pelo pivô, depois a translação.
fn world_delta(pivot: Point3, translation: Point3, rotation: Point3) -> Option<Placement> {
    let mut delta = Placement::default();
    for (axis, angle) in AXES.iter().zip(rotation) {
        if angle.abs() > 1e-12 {
            delta = delta.rotated_about(pivot, *axis, angle.to_radians()).ok()?;
        }
    }
    delta.translated(translation).ok()
}

/// Nova posição local para aplicar `delta` (no mundo) a um elemento: P' = P ∘ O⁻¹ ∘ D ∘ O.
fn moved_local(frame: &Frame, delta: Placement) -> Option<Placement> {
    let inverse = frame.world.inverse().ok()?;
    frame
        .local
        .compose(inverse)
        .ok()?
        .compose(delta)
        .ok()?
        .compose(frame.world)
        .ok()
}

/// Comandos de mover (ou copiar) os elementos pela transformação no mundo.
fn move_commands(tab: &Tab, frames: &[Frame], delta: Placement, copy: bool) -> Vec<Command> {
    let document = tab.session.document();
    frames
        .iter()
        .filter_map(|frame| {
            let placement = moved_local(frame, delta)?;
            Some(match (frame.item, copy) {
                (Selection::Instance(id), false) => Command::MoveInstance { id, placement },
                (Selection::Instance(id), true) => {
                    let instance = document.instance(id)?;
                    Command::AddInstance {
                        part: instance.part,
                        parent: instance.parent,
                        placement,
                    }
                }
                (Selection::Body { id, .. }, false) => Command::MoveBody { id, placement },
                (Selection::Body { id, .. }, true) => Command::CopyBody { id, placement },
            })
        })
        .collect()
}

fn batch(label: &'static str, mut commands: Vec<Command>) -> Option<Command> {
    match commands.len() {
        0 => None,
        1 => commands.pop(),
        _ => Some(Command::Batch { label, commands }),
    }
}

/// Rotação mínima que leva `from` a `to`, pelo ponto dado.
fn align_rotation(from: Point3, to: Point3, pivot: Point3) -> Option<Placement> {
    let angle = nator_geometry::query::angle_between(from, to).ok()?;
    if angle.abs() < 1e-9 {
        return Some(Placement::default());
    }
    let mut axis = cross(from, to);
    if norm(axis) < 1e-9 {
        // Opostos: qualquer eixo perpendicular serve.
        axis = cross(
            from,
            if from[0].abs() < from[1].abs() {
                [1., 0., 0.]
            } else {
                [0., 1., 0.]
            },
        );
    }
    Placement::default().rotated_about(pivot, axis, angle).ok()
}

impl AlignDraft {
    /// Transformação que leva a geometria de origem à de destino.
    fn delta(&self) -> Option<Placement> {
        let (from, to) = (self.from.as_ref()?, self.to.as_ref()?);
        let rotation = match self.kind {
            navigation::PickMode::Face => {
                let target = if self.flip {
                    to.normal
                } else {
                    to.normal.map(|v| -v)
                };
                align_rotation(from.normal, target, from.point)?
            }
            navigation::PickMode::Edge => {
                let a = sub(from.vertices[1], from.vertices[0]);
                let mut b = sub(to.edge[1], to.edge[0]);
                if self.flip {
                    b = b.map(|v| -v);
                }
                align_rotation(a, b, from.point)?
            }
            navigation::PickMode::Vertex => Placement::default(),
        };
        rotation.translated(sub(to.point, from.point)).ok()
    }
}

impl MoveDraft {
    pub(super) fn new(tab: u64, targets: Vec<Selection>) -> Self {
        Self {
            tab,
            picking: targets.is_empty(),
            targets,
            translation: [0.; 3],
            rotation: [0.; 3],
            translation_inputs: Default::default(),
            rotation_inputs: Default::default(),
            pivot: Pivot::Center,
            mode: MoveMode::MoveOrCopy(false),
            drag: None,
        }
    }

    fn numbers_valid(&self) -> bool {
        self.translation_inputs
            .iter()
            .all(|i| i.is_valid(panel::Quantity::Length))
            && self
                .rotation_inputs
                .iter()
                .all(|i| i.is_valid(panel::Quantity::Angle))
    }
}

impl Editor {
    /// O painel determina o tipo real de captura, inclusive ao trocar o tipo no
    /// próprio painel; não depende do modo geral de inspeção do viewport.
    pub(super) fn alignment_pick_mode(&self) -> Option<navigation::PickMode> {
        match &self.assembly_panel {
            Some(Panel::Align(d)) if d.tab == self.current().id && d.field.is_some() => {
                Some(d.kind)
            }
            _ => None,
        }
    }

    pub(super) fn alignment_source_part(&self) -> Option<PartId> {
        match &self.assembly_panel {
            Some(Panel::Align(d)) if d.field == Some(AlignField::From) => {
                self.current().active_part
            }
            _ => None,
        }
    }

    pub(super) fn alignment_reference_point(
        &self,
        bounds: nator_geometry::Bounds,
        rect: egui::Rect,
        position: egui::Pos2,
    ) -> Option<(Point3, String)> {
        let Some(Panel::Align(d)) = &self.assembly_panel else {
            return None;
        };
        if d.field != Some(AlignField::To) {
            return None;
        }
        reference_anchor(
            self.current().session.document(),
            &self.viewport.camera,
            bounds,
            rect,
            position,
            d.kind,
        )
        .map(|anchor| (anchor.point, anchor.label))
    }

    /// Elementos posicionáveis da seleção atual: instâncias na montagem, corpos da peça ativa
    /// na construção.
    fn movable_selection(&self) -> Vec<Selection> {
        let active = self.current().active_part;
        self.current()
            .selected
            .iter()
            .copied()
            .filter(|s| match *s {
                Selection::Instance(_) => active.is_none(),
                Selection::Body { part, .. } => active == Some(part),
            })
            .collect()
    }
    fn selected_instance(&self) -> Option<InstanceId> {
        self.current().selected.iter().rev().find_map(|s| match s {
            Selection::Instance(id) => Some(*id),
            _ => None,
        })
    }
    pub(super) fn open_move(&mut self) {
        let targets = self.movable_selection();
        self.feature_edit = None;
        self.assembly_panel = Some(Panel::Move(Box::new(MoveDraft::new(
            self.current().id,
            targets,
        ))));
    }
    /// Entradas separadas da aba Peça: copiar nunca move o original, e mover não
    /// cria uma cópia. A seleção continua disponível no painel e no viewport.
    pub(super) fn open_body_move(&mut self, copy: bool) {
        self.open_move();
        if let Some(Panel::Move(draft)) = &mut self.assembly_panel {
            draft.mode = if copy { MoveMode::Copy } else { MoveMode::Move };
        }
    }
    /// Translação e rotação do Mover/Copiar aberto, para capturas de ensaio.
    pub(super) fn set_move_for_capture(&mut self, translation: Point3, rotation: Point3) {
        if let Some(Panel::Move(draft)) = &mut self.assembly_panel {
            draft.translation = translation;
            draft.rotation = rotation;
        }
    }
    /// Origem e destino de um alinhamento para capturas de ensaio. O destino define
    /// normal +Z e aresta +X, também válidos para os modos face e aresta.
    pub(super) fn set_align_for_capture(&mut self, from: Pick, to: Point3) {
        if let Some(Panel::Align(d)) = &mut self.assembly_panel {
            d.from = Some(from);
            d.to = Some(Anchor {
                point: to,
                normal: [0., 0., 1.],
                edge: [to, [to[0] + 1., to[1], to[2]]],
                label: text(T::ReferenceSketch).into(),
                reference: true,
            });
            d.field = None;
        }
    }
    /// Mover por alinhamento (face a face, aresta a aresta ou ponto a ponto). O modo de
    /// seleção acompanha o tipo, para a pré-seleção realçar o elemento certo.
    pub(super) fn open_align(&mut self, kind: navigation::PickMode) {
        self.feature_edit = None;
        self.pick_mode = kind;
        self.assembly_panel = Some(Panel::Align(Box::new(AlignDraft {
            tab: self.current().id,
            kind,
            from: None,
            to: None,
            field: Some(AlignField::From),
            flip: false,
        })));
    }
    pub(super) fn open_instance_pattern(&mut self, circular: bool) {
        let target = self.selected_instance();
        self.feature_edit = None;
        self.assembly_panel = Some(Panel::Pattern(PatternDraft {
            tab: self.current().id,
            target,
            picking: target.is_none(),
            circular,
            axis: 0,
            count: 3,
            spacing: "100 mm".into(),
            angle: "360 deg".into(),
            origin: [0.; 3],
            origin_inputs: Default::default(),
        }));
    }
    pub(super) fn open_instance_mirror(&mut self) {
        let target = self.selected_instance();
        self.feature_edit = None;
        self.assembly_panel = Some(Panel::Mirror(MirrorDraft {
            tab: self.current().id,
            target,
            picking: target.is_none(),
            plane: 2,
            offset: 0.,
        }));
    }

    /// Instância nova da mesma definição, ao lado da original.
    pub(super) fn duplicate_selection(&mut self, context: &egui::Context) {
        let commands = self
            .current()
            .selected
            .iter()
            .filter_map(|s| match s {
                Selection::Instance(id) => Some(Command::DuplicateInstance { id: *id }),
                _ => None,
            })
            .collect();
        if let Some(command) = batch("Nova instância compartilhada", commands) {
            self.prepare_command(context, self.current().id, command);
        }
    }
    /// Peça nova e independente para cada instância selecionada.
    pub(super) fn clone_selection(&mut self, context: &egui::Context) {
        let document = self.current().session.document();
        let commands = self
            .current()
            .selected
            .iter()
            .filter_map(|s| match s {
                Selection::Instance(id) => document.instance(*id),
                _ => None,
            })
            .map(|i| Command::CloneInstance {
                id: i.id,
                name: format!("{} ({})", i.name, text(T::CopySuffix)),
            })
            .collect();
        if let Some(command) = batch("Clonar peça independente", commands) {
            self.prepare_command(context, self.current().id, command);
        }
    }
    /// Grupo com as instâncias selecionadas do mesmo nível.
    pub(super) fn group_selection(&mut self, context: &egui::Context) {
        let members = self
            .current()
            .selected
            .iter()
            .filter_map(|s| match s {
                Selection::Instance(id) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>();
        if members.is_empty() {
            self.status.warning(text(T::GroupMembers));
            return;
        }
        let name = format!(
            "{} {}",
            text(T::GroupLabel),
            self.current().session.document().groups().len() + 1
        );
        self.prepare_command(
            context,
            self.current().id,
            Command::CreateGroup { name, members },
        );
    }
    /// Desfaz os grupos que contêm instâncias selecionadas, preservando as posições.
    pub(super) fn ungroup_selection(&mut self, context: &egui::Context) {
        let document = self.current().session.document();
        let commands = document
            .groups()
            .iter()
            .filter(|g| {
                g.members
                    .iter()
                    .any(|m| self.current().is_selected(Selection::Instance(*m)))
            })
            .map(|g| Command::RemoveGroup { id: g.id })
            .collect();
        if let Some(command) = batch("Desagrupar elementos", commands) {
            self.prepare_command(context, self.current().id, command);
        }
    }

    /// Clique na área 3D com um painel de montagem esperando seleção; devolve verdadeiro se
    /// o consumiu. A consulta é síncrona: só o raio contra as malhas visíveis.
    pub(super) fn assembly_pick(&mut self, response: &egui::Response) -> bool {
        let waiting = match &self.assembly_panel {
            // Fora do campo Objetos, o clique escolhe um ponto (para o pivô).
            Some(Panel::Move(d)) => d.drag.is_none(),
            Some(Panel::Align(d)) => d.field.is_some(),
            Some(Panel::Pattern(d)) => d.picking,
            Some(Panel::Mirror(d)) => d.picking,
            None => false,
        };
        if !waiting {
            return self.assembly_panel.is_some() && response.clicked();
        }
        let Some(position) = response
            .interact_pointer_pos()
            .filter(|_| response.clicked())
        else {
            return false;
        };
        let Some(bounds) = self.current().scene_bounds else {
            return true;
        };
        let rect = response.rect;
        let ray = self.viewport.camera.world_ray(
            [
                f64::from((position.x - rect.center().x) / rect.width() * 2.),
                f64::from((rect.center().y - position.y) / rect.height() * 2.),
            ],
            bounds,
            f64::from(rect.aspect_ratio()),
        );
        let view = self.current().view.clone();
        let model = if self.alignment_pick_mode() == Some(navigation::PickMode::Vertex) {
            let document = self.current().session.document().clone();
            self.point_target(&document, rect, position, &response.ctx)
                .map(|target| target.pick)
        } else {
            self.current()
                .session
                .document()
                .pick_all_with(ray, |o| view.pickable(o))
                .ok()
                .and_then(|picks| section::first_visible(picks, self.current().section))
        };
        // Destino do alinhamento: pontos e linhas de rascunhos de referência têm prioridade
        // (são encaixes precisos); para faces, o plano do rascunho vale quando o clique não
        // atinge o modelo.
        if let Some(Panel::Align(d)) = &self.assembly_panel
            && d.field == Some(AlignField::To)
        {
            let kind = d.kind;
            let reference = reference_anchor(
                self.current().session.document(),
                &self.viewport.camera,
                bounds,
                rect,
                position,
                kind,
            );
            let anchor = match (kind, reference, model) {
                (navigation::PickMode::Face, _, Some(pick)) | (_, None, Some(pick)) => {
                    let mut pick = pick;
                    navigation::snap_pick(&mut pick, kind);
                    let label = self
                        .current()
                        .session
                        .document()
                        .instance(pick.instance)
                        .map(|i| i.name.clone())
                        .unwrap_or_default();
                    Some(Anchor::from_pick(&pick, label))
                }
                (_, Some(reference), _) => Some(reference),
                (_, None, None) => None,
            };
            if let (Some(anchor), Some(Panel::Align(d))) = (anchor, &mut self.assembly_panel) {
                d.to = Some(anchor);
                d.field = None;
            }
            return true;
        }
        let Some(mut pick) = model else {
            return true;
        };
        let active = self.current().active_part;
        let item = if active == Some(pick.part) {
            Selection::Body {
                part: pick.part,
                id: pick.body,
            }
        } else {
            Selection::Instance(pick.instance)
        };
        let mode = self.pick_mode;
        match &mut self.assembly_panel {
            Some(Panel::Move(d)) if !d.picking => {
                navigation::snap_pick(&mut pick, mode);
                self.current_mut().picked = Some(pick);
            }
            Some(Panel::Move(d)) => {
                // Alterna o elemento; na construção, só corpos da peça ativa se movem.
                if active.is_some_and(|p| p != pick.part) {
                    self.status.warning(text(T::PickOtherPart));
                } else if let Some(i) = d.targets.iter().position(|t| *t == item) {
                    d.targets.remove(i);
                } else {
                    d.targets.push(item);
                }
            }
            Some(Panel::Align(d)) => {
                navigation::snap_pick(&mut pick, d.kind);
                // A origem é sempre geometria do elemento que se move.
                if d.field == Some(AlignField::From) {
                    if active.is_some_and(|p| p != pick.part) {
                        self.status.warning(text(T::PickOtherPart));
                    } else {
                        d.from = Some(pick);
                        d.field = d.to.is_none().then_some(AlignField::To);
                    }
                }
            }
            Some(Panel::Pattern(d)) => {
                d.target = Some(pick.instance);
                d.picking = false;
            }
            Some(Panel::Mirror(d)) => {
                d.target = Some(pick.instance);
                d.picking = false;
            }
            None => {}
        }
        true
    }

    /// Painel de montagem aberto, com prévia e aplicação.
    pub(super) fn assembly_panel(&mut self, context: &egui::Context) {
        let Some(mut panel) = self.assembly_panel.take() else {
            return;
        };
        let tab = match &panel {
            Panel::Move(d) => d.tab,
            Panel::Align(d) => d.tab,
            Panel::Pattern(d) => d.tab,
            Panel::Mirror(d) => d.tab,
        };
        if tab != self.current().id {
            // Trocar de aba encerra o painel: ele pertence ao documento em que foi aberto.
            self.preview_command(tab, None, None, None);
            return;
        }
        let command = self.panel_command(&panel);
        let footer_text;
        let footer = match &command {
            None => Footer::None,
            Some(c) => match self.preview_status(c) {
                PreviewStatus::Waiting => Footer::Waiting,
                PreviewStatus::Ready => Footer::Ready,
                PreviewStatus::Failed(e) => {
                    footer_text = e.to_string();
                    Footer::Failed(&footer_text)
                }
            },
        };
        let (icon, title) = match &panel {
            Panel::Move(d) if matches!(d.mode, MoveMode::Copy) => (Icon::Copy, T::CopyMenu),
            Panel::Move(_) => (Icon::Move, T::MoveMenu),
            Panel::Align(d) => match d.kind {
                navigation::PickMode::Face => (Icon::FaceToFace, T::MoveFaceToFace),
                navigation::PickMode::Edge => (Icon::EdgeToEdge, T::MoveEdgeToEdge),
                navigation::PickMode::Vertex => (Icon::PointToPoint, T::MovePointToPoint),
            },
            Panel::Pattern(d) if d.circular => (Icon::Pattern, T::CircularInstancePattern),
            Panel::Pattern(_) => (Icon::Pattern, T::RectangularInstancePattern),
            Panel::Mirror(_) => (Icon::Mirror, T::MirrorComponent),
        };
        let document = self.current().session.document().clone();
        let picked = self.current().picked.as_ref().map(|p| p.point);
        let outcome = panel::show_validated(
            context,
            panel::Header {
                id: "assembly",
                icon,
                title: text(title),
            },
            self.canvas_rect,
            footer,
            command.is_some() && !self.busy(),
            |ui| {
                match &mut panel {
                    Panel::Move(d) => move_panel(ui, d, &document, picked),
                    Panel::Align(d) => align_panel(ui, d, &document),
                    Panel::Pattern(d) => pattern_panel(ui, d, &document),
                    Panel::Mirror(d) => mirror_panel(ui, d, &document),
                }
                panel.numbers_valid()
            },
        );
        if outcome.cancel {
            self.preview_command(tab, None, None, None);
            return;
        }
        // Valores digitados neste quadro já estão no rascunho: o comando é refeito com eles.
        let command = self.panel_command(&panel);
        if outcome.ok
            && let Some(command) = command
        {
            self.apply_previewed(context, tab, command);
            return;
        }
        // Durante o arraste do manipulador, a prévia espera o fim do gesto; o contorno
        // dos elementos acompanha o cursor imediatamente.
        let dragging = matches!(&panel, Panel::Move(d) if d.drag.is_some());
        if !dragging {
            self.preview_command(tab, command, None, None);
        } else {
            self.keep_preview();
        }
        self.assembly_panel = Some(panel);
    }

    fn panel_command(&self, panel: &Panel) -> Option<Command> {
        if !panel.numbers_valid() {
            return None;
        }
        let tab = self.current();
        match panel {
            Panel::Move(d) => {
                let frames = frames(tab, &d.targets);
                let pivot = pivot_point(tab, &frames, d.pivot)?;
                let delta = world_delta(pivot, d.translation, d.rotation)?;
                let unchanged = d
                    .translation
                    .iter()
                    .chain(&d.rotation)
                    .all(|v| v.abs() < 1e-12);
                if unchanged && !d.mode.copies() {
                    return None;
                }
                batch(
                    if d.mode.copies() {
                        "Copiar elementos"
                    } else {
                        "Mover elementos"
                    },
                    move_commands(tab, &frames, delta, d.mode.copies()),
                )
            }
            Panel::Align(d) => {
                let from = d.from.as_ref()?;
                if tab.active_part.is_some_and(|p| p != from.part) {
                    return None;
                }
                d.to.as_ref()?;
                let delta = d.delta()?;
                let item = if tab.active_part == Some(from.part) {
                    Selection::Body {
                        part: from.part,
                        id: from.body,
                    }
                } else {
                    Selection::Instance(from.instance)
                };
                let frames = frames(tab, &[item]);
                batch(
                    "Alinhar elemento",
                    move_commands(tab, &frames, delta, false),
                )
            }
            Panel::Pattern(d) => Some(Command::PatternInstance {
                id: d.target?,
                layout: if d.circular {
                    PatternLayout::Circular {
                        axis_origin: d.origin,
                        axis_direction: AXES[d.axis],
                        count: d.count,
                        angle: d.angle.clone(),
                    }
                } else {
                    PatternLayout::Linear {
                        first: LinearAxis {
                            direction: AXES[d.axis],
                            count: d.count,
                            spacing: d.spacing.clone(),
                        },
                        second: None,
                    }
                },
            }),
            Panel::Mirror(d) => {
                let id = d.target?;
                let instance = tab.session.document().instance(id)?;
                let normal = PLANES[d.plane].1;
                Some(Command::MirrorInstance {
                    id,
                    origin: normal.map(|v| v * d.offset),
                    normal,
                    name: format!("{} ({})", instance.name, text(T::MirroredSuffix)),
                })
            }
        }
    }

    /// Manipulador do Mover/Copiar: setas por eixo, anéis de rotação e centro livre.
    /// Devolve verdadeiro quando o ponteiro está sobre ele ou o arrasta.
    pub(super) fn move_gizmo(&mut self, ui: &egui::Ui, response: &egui::Response) -> bool {
        let Some(Panel::Move(draft)) = &self.assembly_panel else {
            return false;
        };
        if draft.tab != self.current().id || draft.targets.is_empty() {
            return false;
        }
        let Some(bounds) = self.current().scene_bounds else {
            return false;
        };
        let frames = frames(self.current(), &draft.targets);
        let Some(pivot) = pivot_point(self.current(), &frames, draft.pivot) else {
            return false;
        };
        let rect = response.rect;
        let camera = self.viewport.camera.clone();
        let center_world: Point3 = std::array::from_fn(|i| pivot[i] + draft.translation[i]);
        let project = |p: Point3| sketch_scene::project(&camera, bounds, rect, p);
        let Some(center) = project(center_world) else {
            return false;
        };
        // Escala: milímetros por ponto numa direção paralela à tela.
        let view = camera.direction();
        let side = {
            let c = cross(view, [0., 0., 1.]);
            if norm(c) < 1e-6 {
                cross(view, [0., 1., 0.])
            } else {
                c
            }
        };
        let side = side.map(|v| v / norm(side));
        let Some(probe) = project(std::array::from_fn(|i| center_world[i] + side[i])) else {
            return false;
        };
        let pixels_per_mm = center.distance(probe).max(1e-6);
        let length = f64::from(ARROW / pixels_per_mm);
        let colors = [
            egui::Color32::from_rgb(214, 64, 64),
            egui::Color32::from_rgb(52, 160, 82),
            egui::Color32::from_rgb(48, 112, 214),
        ];
        let tips = AXES.map(|a| project(std::array::from_fn(|i| center_world[i] + a[i] * length)));
        let rings = std::array::from_fn::<_, 3, _>(|k| {
            let u = AXES[(k + 1) % 3];
            let v = AXES[(k + 2) % 3];
            let radius = length * 0.62;
            (0..=48)
                .filter_map(|s| {
                    let t = std::f64::consts::TAU * f64::from(s) / 48.;
                    project(std::array::from_fn(|i| {
                        center_world[i] + radius * (t.cos() * u[i] + t.sin() * v[i])
                    }))
                })
                .collect::<Vec<_>>()
        });
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let distance_to = |points: &[egui::Pos2], p: egui::Pos2| {
            points
                .windows(2)
                .map(|w| {
                    let (a, b) = (w[0], w[1]);
                    let d = b - a;
                    let t = ((p - a).dot(d) / d.length_sq().max(1e-6)).clamp(0., 1.);
                    p.distance(a + d * t)
                })
                .fold(f32::INFINITY, f32::min)
        };
        let hovered = pointer.and_then(|p| {
            if p.distance(center) < 9. {
                return Some(Handle::Free);
            }
            for (k, tip) in tips.iter().enumerate() {
                if let Some(tip) = tip
                    && distance_to(&[center, *tip], p) < 7.
                {
                    return Some(Handle::Axis(k));
                }
            }
            (0..3)
                .find(|k| distance_to(&rings[*k], p) < 6.)
                .map(Handle::Ring)
        });
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let active = draft.drag.as_ref().map(|d| d.handle).or(hovered);
        // Anéis primeiro, setas por cima e o centro no topo.
        for (k, ring) in rings.iter().enumerate() {
            let width = if active == Some(Handle::Ring(k)) {
                3.
            } else {
                1.6
            };
            painter.add(egui::Shape::line(
                ring.clone(),
                egui::Stroke::new(width, colors[k].gamma_multiply(0.85)),
            ));
        }
        for (k, tip) in tips.iter().enumerate() {
            let Some(tip) = tip else {
                continue;
            };
            let width = if active == Some(Handle::Axis(k)) {
                4.
            } else {
                2.5
            };
            painter.line_segment([center, *tip], egui::Stroke::new(width, colors[k]));
            let d = (*tip - center).normalized();
            let n = egui::vec2(-d.y, d.x);
            painter.add(egui::Shape::convex_polygon(
                vec![*tip + d * 10., *tip + n * 5., *tip - n * 5.],
                colors[k],
                egui::Stroke::NONE,
            ));
            painter.text(
                *tip + d * 18.,
                egui::Align2::CENTER_CENTER,
                AXIS_NAMES[k],
                egui::FontId::proportional(11.),
                colors[k],
            );
        }
        painter.circle(
            center,
            if active == Some(Handle::Free) { 8. } else { 6. },
            egui::Color32::WHITE,
            egui::Stroke::new(2., theme::ACCENT),
        );
        let down = ui.input(|i| i.pointer.primary_down());
        let pressed = ui.input(|i| i.pointer.primary_pressed());
        let Some(Panel::Move(draft)) = &mut self.assembly_panel else {
            return false;
        };
        if pressed && let (Some(handle), Some(start)) = (hovered, pointer) {
            let angle = (start - center).angle();
            draft.drag = Some(GizmoDrag {
                handle,
                start,
                translation: draft.translation,
                rotation: draft.rotation,
                angle,
            });
        }
        if let (Some(drag), Some(position)) = (&draft.drag, pointer) {
            match drag.handle {
                Handle::Axis(k) => {
                    if let Some(tip) = tips[k] {
                        let per_mm = (tip - center) / length as f32;
                        if per_mm.length_sq() > 1e-6 {
                            let delta =
                                f64::from((position - drag.start).dot(per_mm) / per_mm.length_sq());
                            let mut t = drag.translation;
                            t[k] = ((drag.translation[k] + delta) * 100.).round() / 100.;
                            draft.translation = t;
                        }
                    }
                }
                Handle::Ring(k) => {
                    // Ângulo de tela com Y para cima; o sinal depende de o eixo apontar para
                    // quem observa.
                    let now = (position - center).angle();
                    let mut delta = f64::from(drag.angle - now).to_degrees();
                    delta = (delta + 540.).rem_euclid(360.) - 180.;
                    let sign = if dot(AXES[k], view) >= 0. { 1. } else { -1. };
                    let mut r = drag.rotation;
                    r[k] = snap_rotation(drag.rotation[k] + sign * delta);
                    draft.rotation = r;
                    // Ângulo junto ao cursor, destacado quando fixado num múltiplo de 45°.
                    let snapped = r[k].rem_euclid(ROTATION_SNAP.0) == 0.;
                    let label = format!("{}°", nator_drawing::format_fixed(r[k], 0));
                    let at = position + egui::vec2(16., -18.);
                    let galley = painter.layout_no_wrap(
                        label,
                        egui::FontId::proportional(13.),
                        if snapped {
                            egui::Color32::WHITE
                        } else {
                            theme::INK
                        },
                    );
                    let frame = egui::Rect::from_min_size(at, galley.size()).expand(4.);
                    painter.rect_filled(
                        frame,
                        4,
                        if snapped {
                            colors[k]
                        } else {
                            egui::Color32::from_white_alpha(230)
                        },
                    );
                    painter.galley(at, galley, theme::INK);
                }
                Handle::Free => {
                    let aspect = f64::from(rect.aspect_ratio());
                    let cursor = |p: egui::Pos2| {
                        [
                            f64::from((p.x - rect.center().x) / rect.width() * 2.),
                            f64::from((rect.center().y - p.y) / rect.height() * 2.),
                        ]
                    };
                    let a = camera.world_on_target_plane(cursor(drag.start), bounds, aspect);
                    let b = camera.world_on_target_plane(cursor(position), bounds, aspect);
                    draft.translation = std::array::from_fn(|i| {
                        ((drag.translation[i] + b[i] - a[i]) * 100.).round() / 100.
                    });
                }
            }
            ui.ctx().request_repaint();
        }
        let dragging = draft.drag.is_some();
        if !down {
            draft.drag = None;
        }
        // Contorno dos elementos na posição atual, sem esperar a prévia.
        if dragging && let Some(delta) = world_delta(pivot, draft.translation, draft.rotation) {
            let document = self.current().session.document().clone();
            let mut overlay = highlight::Overlay::default();
            for frame in &frames {
                let Some(moved) = delta.compose(frame.world).ok() else {
                    continue;
                };
                match frame.item {
                    Selection::Instance(id) => {
                        if let Some(part) =
                            document.instance(id).and_then(|i| document.part(i.part))
                        {
                            for body in part.shown_bodies() {
                                let pose = moved.compose(body.placement).unwrap_or(moved);
                                overlay.extend(highlight::body_outline(
                                    &mut self.highlight_cache,
                                    body,
                                    pose,
                                ));
                            }
                        }
                    }
                    Selection::Body { id, .. } => {
                        if let Some(body) = document.body(id) {
                            overlay.extend(highlight::body_outline(
                                &mut self.highlight_cache,
                                body,
                                moved,
                            ));
                        }
                    }
                }
            }
            highlight::draw(
                &painter,
                &camera,
                bounds,
                rect,
                &overlay,
                highlight::Look::Selected,
            );
        }
        hovered.is_some() || dragging
    }
}

impl Editor {
    /// Origem e destino do alinhamento escolhidos, desenhados sobre o modelo: origem em
    /// azul, destino na cor das referências quando vem de um rascunho de referência.
    pub(super) fn align_overlay(&self, ui: &egui::Ui, rect: egui::Rect) {
        let Some(Panel::Align(d)) = &self.assembly_panel else {
            return;
        };
        let Some(bounds) = self.current().scene_bounds else {
            return;
        };
        let camera = &self.viewport.camera;
        let project = |p: Point3| sketch_scene::project(camera, bounds, rect, p);
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let mark = |point: Point3, edge: Option<[Point3; 2]>, color: egui::Color32| {
            if let Some([a, b]) = edge
                && let (Some(a), Some(b)) = (project(a), project(b))
            {
                painter.line_segment([a, b], egui::Stroke::new(3.5, color));
            }
            if let Some(p) = project(point) {
                painter.circle(p, 6., egui::Color32::WHITE, egui::Stroke::new(2.5, color));
            }
        };
        let edge = d.kind == navigation::PickMode::Edge;
        if let Some(from) = &d.from {
            mark(
                from.point,
                edge.then_some([from.vertices[0], from.vertices[1]]),
                theme::ACCENT,
            );
        }
        if let Some(to) = &d.to {
            mark(
                to.point,
                edge.then_some(to.edge),
                if to.reference {
                    theme::REFERENCE
                } else {
                    egui::Color32::from_rgb(232, 128, 22)
                },
            );
        }
    }
}

/// Rótulo de um elemento para os campos de seleção.
fn item_name(document: &Document, item: Selection) -> String {
    match item {
        Selection::Instance(id) => document
            .instance(id)
            .map(|i| i.name.clone())
            .unwrap_or_default(),
        Selection::Body { id, .. } => document
            .body(id)
            .map(|b| b.name.clone())
            .unwrap_or_default(),
    }
}

/// Fixação da rotação pelo manipulador: múltiplos de 45° capturam o anel a menos de 4°;
/// fora disso, o ângulo anda em passos de 1°.
const ROTATION_SNAP: (f64, f64) = (45., 4.);

fn snap_rotation(degrees: f64) -> f64 {
    let nearest = (degrees / ROTATION_SNAP.0).round() * ROTATION_SNAP.0;
    if (degrees - nearest).abs() <= ROTATION_SNAP.1 {
        nearest
    } else {
        degrees.round()
    }
}

/// Três campos digitáveis (X, Y, Z) com a precisão de exibição do documento.
fn number_row(
    ui: &mut egui::Ui,
    label: &str,
    values: &mut Point3,
    inputs: &mut [panel::NumberInput; 3],
    quantity: panel::Quantity,
    document: &Document,
) {
    let decimals = usize::from(document.settings().display_decimals);
    panel::row(ui, &format!("{label} ({})", quantity.unit()), |ui| {
        ui.spacing_mut().item_spacing.x = 3.;
        let width = ((ui.available_width() - 6.) / 3. - 8.).clamp(44., 90.);
        for (axis, (value, input)) in values.iter_mut().zip(inputs).enumerate() {
            input
                .show(ui, (label, axis), value, quantity, decimals, width)
                .on_hover_text(AXIS_NAMES[axis]);
        }
    });
}

fn move_panel(ui: &mut egui::Ui, d: &mut MoveDraft, document: &Document, picked: Option<Point3>) {
    let description = d.targets.first().map(|t| item_name(document, *t));
    let s = panel::selection(
        ui,
        text(T::ObjectsField),
        d.targets.len(),
        description.as_deref(),
        d.picking,
    );
    if s.activate {
        d.picking = !d.picking;
    }
    if s.clear {
        d.targets.clear();
        d.picking = true;
    }
    number_row(
        ui,
        text(T::TranslationField),
        &mut d.translation,
        &mut d.translation_inputs,
        panel::Quantity::Length,
        document,
    );
    number_row(
        ui,
        text(T::RotationField),
        &mut d.rotation,
        &mut d.rotation_inputs,
        panel::Quantity::Angle,
        document,
    );
    panel::row(ui, text(T::PivotField), |ui| {
        panel::choice(
            ui,
            &mut d.pivot,
            &[
                (Pivot::Center, Icon::Measure, T::PivotCenter),
                (Pivot::Origin, Icon::Fit, T::PivotOrigin),
            ],
        );
        let chosen = matches!(d.pivot, Pivot::Point(_));
        if ui
            .add_enabled_ui(picked.is_some() || chosen, |ui| {
                icons::button(ui, Icon::Select, text(T::PivotPicked), false, chosen)
            })
            .inner
            .clicked()
            && let Some(point) = picked
        {
            d.pivot = Pivot::Point(point);
        }
    });
    // A alternância permanece só no painel genérico do menu de contexto. Mover e
    // Copiar na aba Peça têm intenção fixa desde a abertura, inclusive sem seleção.
    if let MoveMode::MoveOrCopy(copy) = &mut d.mode {
        if d.targets
            .iter()
            .any(|t| matches!(t, Selection::Body { .. }))
        {
            ui.checkbox(copy, text(T::CreateCopy));
        } else {
            *copy = false;
        }
    }
    ui.horizontal(|ui| {
        if ui.small_button(text(T::ResetTransform)).clicked() {
            d.translation = [0.; 3];
            d.rotation = [0.; 3];
            d.translation_inputs = Default::default();
            d.rotation_inputs = Default::default();
        }
    });
    if !d.numbers_valid() {
        ui.label(
            egui::RichText::new(text(T::InvalidTransformNumber))
                .small()
                .color(theme::DANGER),
        );
    }
    ui.label(
        egui::RichText::new(text(T::MoveHint))
            .small()
            .color(theme::MUTED),
    );
}

fn align_panel(ui: &mut egui::Ui, d: &mut AlignDraft, document: &Document) {
    panel::row(ui, text(T::AlignType), |ui| {
        if panel::choice(
            ui,
            &mut d.kind,
            &[
                (navigation::PickMode::Face, Icon::FaceToFace, T::PickFace),
                (navigation::PickMode::Edge, Icon::EdgeToEdge, T::PickEdge),
                (
                    navigation::PickMode::Vertex,
                    Icon::PointToPoint,
                    T::PickVertex,
                ),
            ],
        ) {
            d.from = None;
            d.to = None;
            d.field = Some(AlignField::From);
        }
    });
    for (field, label) in [
        (AlignField::From, T::AlignFrom),
        (AlignField::To, T::AlignTo),
    ] {
        let description = match field {
            AlignField::From => d.from.as_ref().map(|p| {
                document
                    .instance(p.instance)
                    .map(|i| i.name.clone())
                    .unwrap_or_default()
            }),
            AlignField::To => d.to.as_ref().map(|a| a.label.clone()),
        };
        let chosen = match field {
            AlignField::From => d.from.is_some(),
            AlignField::To => d.to.is_some(),
        };
        let s = panel::selection(
            ui,
            text(label),
            usize::from(chosen),
            description.as_deref(),
            d.field == Some(field),
        );
        if s.activate {
            d.field = Some(field);
        }
        if s.clear {
            match field {
                AlignField::From => d.from = None,
                AlignField::To => d.to = None,
            }
            d.field = Some(field);
        }
    }
    if d.kind != navigation::PickMode::Vertex {
        panel::row(ui, text(T::FlipDirection), |ui| {
            if icons::button(ui, Icon::Flip, text(T::FlipDirection), false, d.flip).clicked() {
                d.flip = !d.flip;
            }
        });
    }
    ui.label(
        egui::RichText::new(text(match d.kind {
            navigation::PickMode::Face => T::MoveFaceHint,
            navigation::PickMode::Edge => T::MoveEdgeHint,
            navigation::PickMode::Vertex => T::MovePointHint,
        }))
        .small()
        .color(theme::MUTED),
    );
    ui.label(
        egui::RichText::new(text(T::AlignHint))
            .small()
            .color(theme::MUTED),
    );
}

fn instance_selection(
    ui: &mut egui::Ui,
    target: &mut Option<InstanceId>,
    picking: &mut bool,
    document: &Document,
) {
    let description = target
        .and_then(|id| document.instance(id))
        .map(|i| i.name.clone());
    let s = panel::selection(
        ui,
        text(T::ComponentField),
        usize::from(target.is_some()),
        description.as_deref(),
        *picking,
    );
    if s.activate {
        *picking = true;
    }
    if s.clear {
        *target = None;
        *picking = true;
    }
}

fn pattern_panel(ui: &mut egui::Ui, d: &mut PatternDraft, document: &Document) {
    instance_selection(ui, &mut d.target, &mut d.picking, document);
    panel::row(ui, text(T::ChamferType), |ui| {
        panel::choice(
            ui,
            &mut d.circular,
            &[
                (false, Icon::Pattern, T::RectangularPattern),
                (true, Icon::Revolve, T::CircularPattern),
            ],
        );
    });
    panel::row(
        ui,
        text(if d.circular {
            T::AxisField
        } else {
            T::DirectionField
        }),
        |ui| {
            ui.spacing_mut().item_spacing.x = 2.;
            for (i, name) in AXIS_NAMES.iter().enumerate() {
                if ui.selectable_label(d.axis == i, *name).clicked() {
                    d.axis = i;
                }
            }
        },
    );
    panel::row(ui, text(T::CountField), |ui| {
        ui.add(egui::DragValue::new(&mut d.count).range(2..=1000))
    });
    if d.circular {
        number_row(
            ui,
            text(T::OriginField),
            &mut d.origin,
            &mut d.origin_inputs,
            panel::Quantity::Length,
            document,
        );
        if !d
            .origin_inputs
            .iter()
            .all(|i| i.is_valid(panel::Quantity::Length))
        {
            ui.label(
                egui::RichText::new(text(T::InvalidTransformNumber))
                    .small()
                    .color(theme::DANGER),
            );
        }
        panel::row(ui, text(T::TotalAngle), |ui| {
            panel::expression(ui, &mut d.angle)
        });
    } else {
        panel::row(ui, text(T::SpacingField), |ui| {
            panel::expression(ui, &mut d.spacing)
        });
    }
}

fn mirror_panel(ui: &mut egui::Ui, d: &mut MirrorDraft, document: &Document) {
    instance_selection(ui, &mut d.target, &mut d.picking, document);
    panel::row(ui, text(T::PlaneField), |ui| {
        ui.spacing_mut().item_spacing.x = 2.;
        for (i, (name, _)) in PLANES.iter().enumerate() {
            if ui.selectable_label(d.plane == i, *name).clicked() {
                d.plane = i;
            }
        }
    });
    panel::row(ui, text(T::OffsetField), |ui| {
        ui.add(
            egui::DragValue::new(&mut d.offset)
                .speed(0.5)
                .suffix(" mm")
                .max_decimals(4),
        )
    });
    ui.label(
        egui::RichText::new(text(T::MirrorHint))
            .small()
            .color(theme::MUTED),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_sketches_offer_line_midpoints_and_circle_quadrants()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut document = Document::new("Apoios de referência")?;
        let mut sketch = nator_sketch::Sketch::new("Referência", nator_sketch::Plane::xy());
        sketch.add_line([0., 0.], [20., 0.], false)?;
        sketch.add_circle([50., 0.], 10., false)?;
        let center = sketch.add_point([25., -10.])?;
        let start = sketch.add_point([30., -10.])?;
        let end = sketch.add_point([25., -15.])?;
        sketch.add_curve(
            nator_sketch::Curve::Arc {
                center,
                start,
                end,
                clockwise: true,
            },
            false,
        )?;
        document.add_reference_sketch(sketch)?;
        let mut camera = Camera::default();
        camera.set_direction([0., 0., 1.]);
        let bounds = nator_geometry::Bounds {
            min: [-10., -20., -5.],
            max: [70., 20., 5.],
        };
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600., 400.));
        for point in [
            [10., 0., 0.],
            [60., 0., 0.],
            [50., 10., 0.],
            [40., 0., 0.],
            [50., -10., 0.],
            [25., -10., 0.],
            [25. + 5. / 2_f64.sqrt(), -10. - 5. / 2_f64.sqrt(), 0.],
        ] {
            let position =
                sketch_scene::project(&camera, bounds, rect, point).ok_or("Projeção ausente")?;
            let anchor = reference_anchor(
                &document,
                &camera,
                bounds,
                rect,
                position,
                navigation::PickMode::Vertex,
            )
            .ok_or("Apoio de referência ausente")?;
            assert!(norm(sub(anchor.point, point)) < 1e-9);
            assert!(anchor.reference);
        }
        Ok(())
    }

    #[test]
    fn ring_rotation_snaps_near_multiples_of_45_degrees() {
        for (raw, snapped) in [
            (2.6, 0.),
            (-3.9, 0.),
            (43.2, 45.),
            (88.5, 90.),
            (138.9, 135.),
            (-178., -180.),
            (30.4, 30.),
            (61.6, 62.),
        ] {
            assert_eq!(snap_rotation(raw), snapped, "{raw}");
        }
    }

    #[test]
    fn world_moves_become_local_placements_under_rotated_parents()
    -> Result<(), Box<dyn std::error::Error>> {
        // Pai girado 90° em Z e deslocado; o filho fica a 10 mm em X do pai.
        let parent = Placement::new([100., 0., 0.], [0., 0., std::f64::consts::FRAC_PI_2])?;
        let local = Placement::new([10., 0., 0.], [0.; 3])?;
        let frame = Frame {
            item: Selection::Instance(InstanceId(1)),
            local,
            world: parent.compose(local)?,
        };
        // Mover 5 mm em X no mundo equivale a −5 mm em Y local do pai.
        let delta = world_delta([0.; 3], [5., 0., 0.], [0.; 3]).ok_or("Delta inválido")?;
        let moved = moved_local(&frame, delta).ok_or("Posição inválida")?;
        let world = parent.compose(moved)?.apply([0.; 3]);
        assert!(norm(sub(world, [105., 10., 0.])) < 1e-9);
        assert!(norm(sub(moved.translation_mm(), [10., -5., 0.])) < 1e-9);
        // Girar 90° em Z pelo próprio ponto mantém a origem e acrescenta a rotação.
        let pivot = frame.world.apply([0.; 3]);
        let delta = world_delta(pivot, [0.; 3], [0., 0., 90.]).ok_or("Delta inválido")?;
        let turned = parent.compose(moved_local(&frame, delta).ok_or("Posição inválida")?)?;
        assert!(norm(sub(turned.apply([0.; 3]), pivot)) < 1e-9);
        assert!(norm(sub(turned.apply_vector([1., 0., 0.]), [-1., 0., 0.])) < 1e-9);
        Ok(())
    }

    #[test]
    fn faces_align_opposed_and_points_coincide() -> Result<(), Box<dyn std::error::Error>> {
        let pick = |point: Point3, normal: Point3| Pick {
            instance: InstanceId(1),
            part: PartId(1),
            body: BodyId(1),
            triangle: 0,
            point,
            normal,
            vertices: [point; 3],
            placement: Placement::default(),
        };
        let draft = AlignDraft {
            tab: 1,
            kind: navigation::PickMode::Face,
            from: Some(pick([0., 0., 5.], [0., 0., 1.])),
            to: Some(Anchor::from_pick(
                &pick([20., 0., 0.], [1., 0., 0.]),
                String::new(),
            )),
            field: None,
            flip: false,
        };
        let delta = draft.delta().ok_or("Alinhamento inválido")?;
        // O ponto de origem vai ao destino e a normal fica oposta à do destino.
        assert!(norm(sub(delta.apply([0., 0., 5.]), [20., 0., 0.])) < 1e-9);
        assert!(norm(sub(delta.apply_vector([0., 0., 1.]), [-1., 0., 0.])) < 1e-9);
        Ok(())
    }
}
