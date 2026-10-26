//! Realce desenhado sobre a imagem da área 3D: pré-seleção sob o cursor, face ou aresta
//! escolhida e referências dos painéis de comando. Corpos e componentes selecionados são
//! tingidos na própria GPU (estilo de cena); aqui ficam os elementos menores que um corpo e
//! a pré-seleção, que muda a cada movimento do cursor e não pode reconstruir a cena.
//!
//! A geometria vem da malha de exibição, com a face de origem de cada triângulo: é só
//! apresentação. As referências gravadas continuam sendo nomes de faces do B-rep.
use super::*;
use nator_document::{
    Body,
    assembly::Pick,
    features::{EdgeRef, FaceRef},
};
use nator_geometry::{cross, dot, norm, sub};
use std::collections::HashMap;

/// Intervalo mínimo entre consultas de pré-seleção, para não disputar a interface.
const HOVER_INTERVAL: Duration = Duration::from_millis(30);
/// Acima disto, a pré-seleção é desligada: o raio percorre todos os triângulos.
const HOVER_TRIANGLES: usize = 400_000;

/// Geometria de realce em coordenadas do mundo.
#[derive(Debug, Default, Clone)]
pub(super) struct Overlay {
    /// Segmentos com a normal do triângulo de origem; segmentos de costas são omitidos.
    pub segments: Vec<([Point3; 2], Option<Point3>)>,
    pub triangles: Vec<[Point3; 3]>,
    pub points: Vec<Point3>,
}
impl Overlay {
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() && self.triangles.is_empty() && self.points.is_empty()
    }
    fn extend(&mut self, other: Overlay) {
        self.segments.extend(other.segments);
        self.triangles.extend(other.triangles);
        self.points.extend(other.points);
    }
}

/// Aparência do realce.
#[derive(Clone, Copy)]
pub(super) enum Look {
    Hover,
    Selected,
}
impl Look {
    fn color(self, alpha: u8) -> egui::Color32 {
        match self {
            Self::Hover => egui::Color32::from_rgba_unmultiplied(64, 156, 236, alpha),
            Self::Selected => egui::Color32::from_rgba_unmultiplied(28, 104, 196, alpha),
        }
    }
}

/// Arestas internas de uma malha: vértices e os dois triângulos vizinhos.
type Adjacency = Vec<(u32, u32, usize, usize)>;
/// Arestas vivas com a normal do triângulo que as marca.
type Creases = Vec<([Point3; 2], Point3)>;

/// Dados derivados por geometria, reaproveitados enquanto o `MeshAsset` for o mesmo.
#[derive(Default)]
pub(super) struct Cache {
    creases: HashMap<usize, Arc<Creases>>,
    adjacency: HashMap<usize, Arc<Adjacency>>,
}
impl Cache {
    fn key(body: &Body) -> usize {
        Arc::as_ptr(&body.geometry) as usize
    }
    fn trim(&mut self) {
        // Geometrias antigas saem quando o documento muda bastante.
        if self.creases.len() > 256 {
            self.creases.clear();
        }
        if self.adjacency.len() > 64 {
            self.adjacency.clear();
        }
    }
    /// Arestas vivas do corpo, com a normal do triângulo que as marca.
    fn creases(&mut self, body: &Body) -> Arc<Creases> {
        self.trim();
        Arc::clone(self.creases.entry(Self::key(body)).or_insert_with(|| {
            let mesh = body.geometry.mesh();
            let mut out = Vec::new();
            for (triangle, mask) in mesh.triangles().iter().zip(body.geometry.creases()) {
                let points = mesh.triangle_points(*triangle);
                let normal = cross(sub(points[1], points[0]), sub(points[2], points[0]));
                for k in 0..3 {
                    // Bit k: aresta oposta ao canto k (ver `Mesh::edge_analysis`).
                    if mask & (1 << k) != 0 {
                        out.push(([points[(k + 1) % 3], points[(k + 2) % 3]], normal));
                    }
                }
            }
            Arc::new(out)
        }))
    }
    fn adjacency(&mut self, body: &Body) -> Arc<Adjacency> {
        self.trim();
        Arc::clone(self.adjacency.entry(Self::key(body)).or_insert_with(|| {
            let mut edges: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
            for (index, t) in body.geometry.mesh().triangles().iter().enumerate() {
                for k in 0..3 {
                    let (a, b) = (t[k], t[(k + 1) % 3]);
                    edges.entry((a.min(b), a.max(b))).or_default().push(index);
                }
            }
            Arc::new(
                edges
                    .into_iter()
                    .filter_map(|((a, b), list)| match list.as_slice() {
                        [first, second] => Some((a, b, *first, *second)),
                        _ => None,
                    })
                    .collect(),
            )
        }))
    }
}

fn transform(pose: Placement, normal: Point3) -> Point3 {
    sub(pose.apply(normal), pose.apply([0.; 3]))
}

/// Contorno (arestas vivas) de um corpo na pose dada.
pub(super) fn body_outline(cache: &mut Cache, body: &Body, pose: Placement) -> Overlay {
    Overlay {
        segments: cache
            .creases(body)
            .iter()
            .map(|(s, n)| (s.map(|p| pose.apply(p)), Some(transform(pose, *n))))
            .collect(),
        ..Default::default()
    }
}

/// Índice da face de origem pelo nome estável.
fn face_index(body: &Body, name: &str) -> Option<u32> {
    body.face_names
        .iter()
        .position(|n| n == name)
        .and_then(|i| u32::try_from(i).ok())
}

/// Triângulos de uma face de origem.
pub(super) fn face(body: &Body, pose: Placement, face: u32) -> Overlay {
    let Some(faces) = body.geometry.triangle_faces() else {
        return Overlay::default();
    };
    let mesh = body.geometry.mesh();
    Overlay {
        triangles: mesh
            .triangles()
            .iter()
            .zip(faces)
            .filter(|(_, f)| **f == face)
            .map(|(t, _)| mesh.triangle_points(*t).map(|p| pose.apply(p)))
            .collect(),
        ..Default::default()
    }
}

/// Segmentos da fronteira entre duas faces de origem.
pub(super) fn edge(cache: &mut Cache, body: &Body, pose: Placement, pair: [u32; 2]) -> Overlay {
    let Some(faces) = body.geometry.triangle_faces() else {
        return Overlay::default();
    };
    let mesh = body.geometry.mesh();
    let wanted = (pair[0].min(pair[1]), pair[0].max(pair[1]));
    Overlay {
        segments: cache
            .adjacency(body)
            .iter()
            .filter(|(_, _, t, u)| {
                let (f, g) = (faces[*t], faces[*u]);
                (f.min(g), f.max(g)) == wanted
            })
            .map(|(a, b, _, _)| {
                (
                    [
                        pose.apply(mesh.vertices()[*a as usize]),
                        pose.apply(mesh.vertices()[*b as usize]),
                    ],
                    None,
                )
            })
            .collect(),
        ..Default::default()
    }
}

/// Realce de uma referência gravada (aresta por par de faces ou face por nome).
pub(super) fn reference(
    cache: &mut Cache,
    body: &Body,
    pose: Placement,
    kept: &feature_editor::Kept,
) -> Overlay {
    match kept {
        feature_editor::Kept::Edge(EdgeRef::Between([a, b])) => {
            match (face_index(body, a), face_index(body, b)) {
                (Some(a), Some(b)) => edge(cache, body, pose, [a, b]),
                _ => Overlay::default(),
            }
        }
        feature_editor::Kept::Face(FaceRef::Named(name)) => face_index(body, name)
            .map(|f| face(body, pose, f))
            .unwrap_or_default(),
        feature_editor::Kept::Body(_) => body_outline(cache, body, pose),
        _ => Overlay::default(),
    }
}

/// Desenha o realce projetado no retângulo da área 3D.
pub(super) fn draw(
    painter: &egui::Painter,
    camera: &Camera,
    bounds: nator_geometry::Bounds,
    rect: egui::Rect,
    overlay: &Overlay,
    look: Look,
) {
    let toward = camera.direction();
    let project = |p: Point3| sketch_scene::project(camera, bounds, rect, p);
    let mut mesh = egui::Mesh::default();
    let fill = look.color(match look {
        Look::Hover => 70,
        Look::Selected => 95,
    });
    for triangle in &overlay.triangles {
        let normal = cross(sub(triangle[1], triangle[0]), sub(triangle[2], triangle[0]));
        if dot(normal, toward) <= 0. {
            continue;
        }
        let Some(points) = triangle
            .iter()
            .map(|p| project(*p))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let base = mesh.vertices.len() as u32;
        for p in points {
            mesh.colored_vertex(p, fill);
        }
        mesh.add_triangle(base, base + 1, base + 2);
    }
    if !mesh.is_empty() {
        painter.add(egui::Shape::mesh(mesh));
    }
    let stroke = egui::Stroke::new(
        match look {
            Look::Hover => 2.5,
            Look::Selected => 3.,
        },
        look.color(255),
    );
    for (segment, normal) in &overlay.segments {
        if normal.is_some_and(|n| norm(n) > 0. && dot(n, toward) <= 0.) {
            continue;
        }
        if let (Some(a), Some(b)) = (project(segment[0]), project(segment[1])) {
            painter.line_segment([a, b], stroke);
        }
    }
    for point in &overlay.points {
        if let Some(p) = project(*point) {
            painter.circle(p, 5., egui::Color32::WHITE, stroke);
        }
    }
}

/// Pré-seleção calculada para a posição do cursor.
pub(super) struct Hover {
    pub tab: u64,
    pub revision: u64,
    pub position: egui::Pos2,
    /// Contexto que decidiu o tipo de realce (painel, modo de seleção, construção).
    pub context: String,
    pub overlay: Overlay,
    pub label: String,
    computed: Instant,
}

impl Hover {
    /// Realce definido por outro controle (por exemplo, a lista de sobrepostos).
    pub(super) fn fixed(
        tab: u64,
        revision: u64,
        position: egui::Pos2,
        overlay: Overlay,
        label: String,
    ) -> Self {
        Self {
            tab,
            revision,
            position,
            context: String::new(),
            overlay,
            label,
            computed: Instant::now(),
        }
    }
}

impl Editor {
    /// Contexto de pré-seleção: muda quando o painel, o campo ou o modo mudam.
    fn hover_context(&self) -> String {
        format!(
            "{:?}|{:?}|{:?}|{:?}",
            self.feature_edit
                .as_ref()
                .and_then(|d| d.field().map(|f| (d.part(), d.output(), f))),
            self.pick_mode,
            self.current().active_part,
            self.current().view.version
        )
    }

    /// Documento usado nas seleções: ao editar uma operação, o histórico até antes dela.
    pub(super) fn picking_document(&self) -> Document {
        let mut document = self.current().session.document().clone();
        if let Some(draft) = &self.feature_edit
            && let Some(output) = draft.output()
        {
            live_preview::roll_back(&mut document, draft.part(), output, true);
        }
        document
    }

    /// Recalcula a pré-seleção quando o cursor se move sobre a área 3D.
    pub(super) fn update_hover(&mut self, ui: &egui::Ui, response: &egui::Response) {
        // A lista de sobrepostos controla o realce enquanto está aberta.
        if self.gestures.other.is_some() {
            return;
        }
        let position = response.hover_pos();
        let blocked = self.current().sketch_edit.is_some()
            || self.current().plane_pick.is_some()
            || ui.input(|i| i.pointer.any_down())
            || self
                .feature_edit
                .as_ref()
                .is_some_and(|d| d.field() == Some(feature_editor::Field::Profile))
            || self
                .scene
                .as_ref()
                .is_none_or(|s| s.triangles().len() > HOVER_TRIANGLES);
        let Some(position) = position.filter(|_| !blocked) else {
            self.hover = None;
            return;
        };
        let tab = self.current().id;
        let revision = self.current().session.revision();
        let context = self.hover_context();
        if let Some(hover) = &self.hover
            && hover.tab == tab
            && hover.revision == revision
            && hover.context == context
            && (hover.position.distance(position) < 1.5
                || hover.computed.elapsed() < HOVER_INTERVAL)
        {
            if hover.position.distance(position) >= 1.5 {
                ui.ctx().request_repaint_after(HOVER_INTERVAL);
            }
            return;
        }
        let Some(bounds) = self.current().scene_bounds else {
            return;
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
        let document = self.picking_document();
        let view = self.current().view.clone();
        let pick = document
            .pick_with(ray, |occurrence| view.pickable(occurrence))
            .ok()
            .flatten();
        let (overlay, label) = match pick {
            Some(pick) => self.pick_overlay(&document, &pick),
            None => (Overlay::default(), String::new()),
        };
        self.hover = Some(Hover {
            tab,
            revision,
            position,
            context,
            overlay,
            label,
            computed: Instant::now(),
        });
    }

    /// O que realçar para um ponto escolhido, conforme o painel ativo ou o modo de seleção.
    pub(super) fn pick_overlay(&mut self, document: &Document, pick: &Pick) -> (Overlay, String) {
        let Some(body) = document.body(pick.body) else {
            return Default::default();
        };
        let pose = pick
            .placement
            .compose(body.placement)
            .unwrap_or(pick.placement);
        let face_of = |pick: &Pick| {
            body.geometry
                .triangle_faces()
                .and_then(|faces| faces.get(pick.triangle).copied())
        };
        let awaiting = self.feature_edit.as_ref().and_then(|d| d.awaiting());
        let other_part = self
            .feature_edit
            .as_ref()
            .is_some_and(|d| d.part() != pick.part);
        let construction = self.current().active_part == Some(pick.part);
        let mode = self.pick_mode;
        let cache = &mut self.highlight_cache;
        if let Some(awaiting) = awaiting {
            if other_part {
                return Default::default();
            }
            return match awaiting {
                feature_editor::Awaiting::Edge => match document.edge_at(pick) {
                    Ok(EdgeRef::Between([a, b])) => {
                        match (face_index(body, &a), face_index(body, &b)) {
                            (Some(f), Some(g)) => {
                                (edge(cache, body, pose, [f, g]), format!("{a} × {b}"))
                            }
                            _ => Default::default(),
                        }
                    }
                    _ => Default::default(),
                },
                feature_editor::Awaiting::Face => match face_of(pick) {
                    Some(f) => (
                        face(body, pose, f),
                        body.face_names.get(f as usize).cloned().unwrap_or_default(),
                    ),
                    None => Default::default(),
                },
                feature_editor::Awaiting::Body => {
                    (body_outline(cache, body, pose), body.name.clone())
                }
            };
        }
        // Fora da peça em construção, a unidade é o componente inteiro.
        if !construction {
            let mut overlay = Overlay::default();
            if let Some(part) = document.part(pick.part) {
                for b in part.bodies.iter().filter(|b| b.visible) {
                    let pose = pick
                        .placement
                        .compose(b.placement)
                        .unwrap_or(pick.placement);
                    overlay.extend(body_outline(cache, b, pose));
                }
            }
            let name = document
                .instance(pick.instance)
                .map(|i| i.name.clone())
                .unwrap_or_default();
            return (overlay, name);
        }
        match mode {
            navigation::PickMode::Face => match face_of(pick) {
                Some(f) => (face(body, pose, f), body.name.clone()),
                None => (
                    Overlay {
                        triangles: vec![pick.vertices],
                        ..Default::default()
                    },
                    body.name.clone(),
                ),
            },
            navigation::PickMode::Edge => {
                let mut snapped = pick.clone();
                navigation::snap_pick(&mut snapped, navigation::PickMode::Edge);
                let overlay = match document.edge_at(pick) {
                    Ok(EdgeRef::Between([a, b])) => {
                        match (face_index(body, &a), face_index(body, &b)) {
                            (Some(f), Some(g)) => edge(cache, body, pose, [f, g]),
                            _ => Overlay::default(),
                        }
                    }
                    // Malhas: a aresta do triângulo mais próxima do cursor.
                    _ => Overlay {
                        segments: vec![([snapped.vertices[0], snapped.vertices[1]], None)],
                        ..Default::default()
                    },
                };
                (overlay, body.name.clone())
            }
            navigation::PickMode::Vertex => {
                let mut snapped = pick.clone();
                navigation::snap_pick(&mut snapped, navigation::PickMode::Vertex);
                (
                    Overlay {
                        points: vec![snapped.point],
                        ..Default::default()
                    },
                    body.name.clone(),
                )
            }
        }
    }

    /// Realces persistentes (referências do painel e face/aresta escolhida) e a pré-seleção.
    pub(super) fn draw_highlights(&mut self, ui: &egui::Ui, rect: egui::Rect) {
        let Some(bounds) = self.current().scene_bounds else {
            return;
        };
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let camera = self.viewport.camera.clone();
        let mut selected = Overlay::default();
        // Referências escolhidas no painel de comando, sobre o corpo base.
        if let Some(draft) = self
            .feature_edit
            .as_ref()
            .filter(|d| d.tab() == self.current().id)
        {
            let document = self.picking_document();
            if let (Some(base), Ok(placement)) = (
                draft.base().and_then(|id| document.body(id).cloned()),
                sketch_scene::part_placement(self.current(), draft.part()),
            ) {
                let pose = placement.compose(base.placement).unwrap_or(placement);
                for kept in draft.references() {
                    selected.extend(reference(&mut self.highlight_cache, &base, pose, kept));
                }
            }
        }
        // Face ou aresta escolhida com o modo de seleção, na peça em construção.
        if self.feature_edit.is_none()
            && let Some(pick) = self.current().picked.clone()
            && self.current().active_part == Some(pick.part)
            && self.pick_mode != navigation::PickMode::Vertex
        {
            let document = self.current().session.document().clone();
            if let Some(body) = document.body(pick.body) {
                let pose = pick
                    .placement
                    .compose(body.placement)
                    .unwrap_or(pick.placement);
                let face_of = body
                    .geometry
                    .triangle_faces()
                    .and_then(|faces| faces.get(pick.triangle).copied());
                selected.extend(match (self.pick_mode, face_of) {
                    (navigation::PickMode::Face, Some(f)) => face(body, pose, f),
                    (navigation::PickMode::Edge, Some(_)) => match document.edge_at(&pick) {
                        Ok(EdgeRef::Between([a, b])) => {
                            match (face_index(body, &a), face_index(body, &b)) {
                                (Some(f), Some(g)) => {
                                    edge(&mut self.highlight_cache, body, pose, [f, g])
                                }
                                _ => Overlay::default(),
                            }
                        }
                        _ => Overlay::default(),
                    },
                    _ => Overlay::default(),
                });
            }
        }
        if !selected.is_empty() {
            draw(&painter, &camera, bounds, rect, &selected, Look::Selected);
        }
        if let Some(hover) = self
            .hover
            .as_ref()
            .filter(|h| h.tab == self.current().id && !h.overlay.is_empty())
        {
            draw(&painter, &camera, bounds, rect, &hover.overlay, Look::Hover);
            if !hover.label.is_empty() {
                let galley = painter.layout_no_wrap(
                    hover.label.clone(),
                    egui::FontId::proportional(11.5),
                    egui::Color32::WHITE,
                );
                let at = hover.position + egui::vec2(14., 14.);
                let frame = egui::Rect::from_min_size(at, galley.size() + egui::vec2(10., 6.));
                painter.rect_filled(
                    frame,
                    4,
                    egui::Color32::from_rgba_unmultiplied(40, 49, 64, 220),
                );
                painter.galley(at + egui::vec2(5., 3.), galley, egui::Color32::WHITE);
            }
        }
    }
}
