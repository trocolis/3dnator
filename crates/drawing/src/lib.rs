//! Folhas vetoriais a partir de projeções próprias ou malhas, sem dependência da interface
//! ou do núcleo CAD. A legenda identifica a origem e a precisão das vistas.
use nator_geometry::{Mesh, Point3, ProjectedSegment, cross, dot, norm, sub};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fmt};
mod export;
pub use export::{pdf, png};
#[derive(Debug, Clone)]
pub struct DrawingError(pub String);
impl fmt::Display for DrawingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for DrawingError {}
type Result<T> = std::result::Result<T, DrawingError>;
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Paper {
    A4,
    A3,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum Orientation {
    #[default]
    Landscape,
    Portrait,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Style {
    Conventional,
    Blueprint,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum View {
    Front,
    Top,
    Right,
    Isometric,
}
impl View {
    pub fn basis(self) -> (Point3, Point3, Point3) {
        match self {
            Self::Front => ([1., 0., 0.], [0., 0., 1.], [0., -1., 0.]),
            Self::Top => ([1., 0., 0.], [0., 1., 0.], [0., 0., 1.]),
            Self::Right => ([0., 1., 0.], [0., 0., 1.], [1., 0., 0.]),
            Self::Isometric => {
                let r = [
                    std::f64::consts::FRAC_1_SQRT_2,
                    std::f64::consts::FRAC_1_SQRT_2,
                    0.,
                ];
                let d = [1. / 3_f64.sqrt(), -1. / 3_f64.sqrt(), 1. / 3_f64.sqrt()];
                (r, cross(d, r), d)
            }
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Front => "FRONTAL",
            Self::Top => "SUPERIOR",
            Self::Right => "DIREITA",
            Self::Isometric => "ISOMETRICA",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub origin: Point3,
    pub normal: Point3,
}
/// Identificador estável: 0 é reservado ao corte A–A legado; 1 a 25 representam B–B
/// a Z–Z. Remover um corte não renomeia os demais nem suas referências visuais.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionView {
    pub id: u32,
    pub section: Section,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specification {
    pub title: String,
    pub paper: Paper,
    pub style: Style,
    pub scale: f64,
    pub hidden_lines: bool,
    pub dimensions: bool,
    pub section: Option<Section>,
    #[serde(default)]
    pub additional_sections: Vec<SectionView>,
    #[serde(default)]
    pub orientation: Orientation,
    #[serde(default = "default_dpi")]
    pub png_dpi: u32,
    /// Revisão exibida na legenda (por exemplo `A`).
    #[serde(default = "default_revision")]
    pub revision: String,
    /// Data exibida na legenda, como digitada (por exemplo `26/10/2026`).
    #[serde(default)]
    pub date: String,
}
fn default_revision() -> String {
    "A".into()
}
fn default_dpi() -> u32 {
    150
}
impl Default for Specification {
    fn default() -> Self {
        Self {
            title: "Desenho técnico".into(),
            paper: Paper::A4,
            style: Style::Blueprint,
            scale: 1.,
            hidden_lines: false,
            dimensions: true,
            section: None,
            additional_sections: Vec::new(),
            orientation: Orientation::Landscape,
            png_dpi: default_dpi(),
            revision: default_revision(),
            date: String::new(),
        }
    }
}

/// Medida em mm com vírgula decimal e até três casas, sem zeros à direita (`60`, `12,5`).
pub fn format_mm(value: f64) -> String {
    let text = format!("{:.3}", (value * 1000.).round() / 1000.);
    let text = text.trim_end_matches('0').trim_end_matches('.');
    let text = if text == "-0" { "0" } else { text };
    text.replace('.', ",")
}
impl Specification {
    pub fn size(&self) -> [f64; 2] {
        let size = match self.paper {
            Paper::A4 => [297., 210.],
            Paper::A3 => [420., 297.],
        };
        if self.orientation == Orientation::Portrait {
            [size[1], size[0]]
        } else {
            size
        }
    }
    pub fn has_sections(&self) -> bool {
        self.section.is_some() || !self.additional_sections.is_empty()
    }
    pub fn section_views(&self) -> Vec<(&Section, char)> {
        self.section
            .iter()
            .map(|s| (s, 'A'))
            .chain(self.additional_sections.iter().map(|s| {
                (
                    &s.section,
                    char::from_u32(u32::from('A').saturating_add(s.id)).unwrap_or('?'),
                )
            }))
            .collect()
    }
    pub fn validate(&self) -> Result<()> {
        let invalid_text = |t: &str| t.len() > 512 || t.chars().any(char::is_control);
        if !(72..=300).contains(&self.png_dpi) {
            return Err(DrawingError(
                "A resolução PNG deve ficar entre 72 e 300 DPI.".into(),
            ));
        }
        if self.additional_sections.len() > 7 {
            return Err(DrawingError(
                "A folha aceita até sete cortes adicionais ao corte A–A.".into(),
            ));
        }
        if !self.scale.is_finite()
            || self.scale <= 0.
            || self.scale > 1000.
            || invalid_text(&self.title)
            || invalid_text(&self.revision)
            || invalid_text(&self.date)
        {
            return Err(DrawingError(
                "Título, revisão, data ou escala da folha inválidos.".into(),
            ));
        }
        for (index, section) in self.additional_sections.iter().enumerate() {
            if !(1..=25).contains(&section.id)
                || self.additional_sections[..index]
                    .iter()
                    .any(|s| s.id == section.id)
            {
                return Err(DrawingError(
                    "Identificador de corte inválido ou repetido.".into(),
                ));
            }
        }
        for (s, _) in self.section_views() {
            let length = norm(s.normal);
            if s.origin.iter().chain(&s.normal).any(|v| !v.is_finite())
                || !length.is_finite()
                || length < 1e-9
            {
                return Err(DrawingError("Plano de corte inválido.".into()));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub enum Primitive {
    Line {
        a: [f64; 2],
        b: [f64; 2],
        width: f64,
        dashed: bool,
    },
    Text {
        at: [f64; 2],
        size: f64,
        value: String,
    },
}
#[derive(Debug, Clone)]
pub struct Sheet {
    pub size: [f64; 2],
    pub style: Style,
    pub primitives: Vec<Primitive>,
    /// A visibilidade das vistas veio do B-rep; curvas continuam discretizadas para
    /// exportação vetorial. Cortes gerados a partir de malhas são identificados à parte.
    pub exact_projection: bool,
}

pub struct ProjectedView {
    pub view: View,
    pub segments: Vec<ProjectedSegment>,
}
impl Sheet {
    pub fn line(&mut self, a: [f64; 2], b: [f64; 2], width: f64, dashed: bool) {
        self.primitives.push(Primitive::Line {
            a,
            b,
            width,
            dashed,
        });
    }
    pub fn text(&mut self, at: [f64; 2], size: f64, value: impl Into<String>) {
        self.primitives.push(Primitive::Text {
            at,
            size,
            value: value.into(),
        });
    }
    pub fn dimension(&mut self, a: [f64; 2], b: [f64; 2], offset: [f64; 2], label: String) {
        let p = [a[0] + offset[0], a[1] + offset[1]];
        let q = [b[0] + offset[0], b[1] + offset[1]];
        self.line(a, p, 0.15, false);
        self.line(b, q, 0.15, false);
        self.line(p, q, 0.18, false);
        let d = [q[0] - p[0], q[1] - p[1]];
        let length = d[0].hypot(d[1]);
        if length > 1e-9 {
            let u = [d[0] / length, d[1] / length];
            for (tip, sign) in [(p, 1.), (q, -1.)] {
                for side in [-1., 1.] {
                    self.line(
                        tip,
                        [
                            tip[0] + sign * u[0] * 2. - side * u[1] * 0.6,
                            tip[1] + sign * u[1] * 2. + side * u[0] * 0.6,
                        ],
                        0.18,
                        false,
                    );
                }
            }
        }
        self.text(
            [(p[0] + q[0]) / 2. + 1., (p[1] + q[1]) / 2. - 1.],
            2.5,
            label,
        );
    }
}
struct Edge {
    a: Point3,
    b: Point3,
    normals: Vec<Point3>,
}
fn feature_edges(mesh: &Mesh) -> Vec<Edge> {
    let key = |p: Point3| p.map(|v| if v == 0. { 0 } else { v.to_bits() });
    let mut edges = HashMap::new();
    for t in mesh.triangles() {
        let p = mesh.triangle_points(*t);
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let n = n.map(|v| v / norm(n));
        for (a, b) in [(p[0], p[1]), (p[1], p[2]), (p[2], p[0])] {
            let (ka, kb) = (key(a), key(b));
            let k = if ka < kb { (ka, kb) } else { (kb, ka) };
            edges
                .entry(k)
                .or_insert_with(|| Edge {
                    a,
                    b,
                    normals: Vec::new(),
                })
                .normals
                .push(n);
        }
    }
    let mut out = edges.into_values().collect::<Vec<_>>();
    out.sort_by_key(|e| (key(e.a), key(e.b)));
    out
}
// Z-buffer limita o custo de remoção aproximada de linhas ocultas em modelos de malha.
// A resolução é explícita; não equivale ao HLR exato de superfícies analíticas CAD.
fn depth_buffer(
    mesh: &Mesh,
    basis: (Point3, Point3, Point3),
    min: [f64; 2],
    span: f64,
) -> Result<Vec<f64>> {
    let mut depth = vec![f64::NEG_INFINITY; 512 * 512];
    let mut budget = 0usize;
    let project = |p: Point3| {
        [
            (dot(p, basis.0) - min[0]) * 500. / span + 6.,
            (dot(p, basis.1) - min[1]) * 500. / span + 6.,
            dot(p, basis.2),
        ]
    };
    for t in mesh.triangles() {
        let p = mesh.triangle_points(*t).map(project);
        let lo = std::array::from_fn::<_, 2, _>(|i| {
            p.iter()
                .map(|p| p[i])
                .fold(f64::INFINITY, f64::min)
                .floor()
                .clamp(0., 511.) as usize
        });
        let hi = std::array::from_fn::<_, 2, _>(|i| {
            p.iter()
                .map(|p| p[i])
                .fold(f64::NEG_INFINITY, f64::max)
                .ceil()
                .clamp(0., 511.) as usize
        });
        budget += (hi[0] - lo[0] + 1) * (hi[1] - lo[1] + 1);
        if budget > 80_000_000 {
            return Err(DrawingError(
                "Vista muito complexa para remoção de linhas ocultas; reduza a tesselação.".into(),
            ));
        }
        let area =
            (p[1][1] - p[2][1]) * (p[0][0] - p[2][0]) + (p[2][0] - p[1][0]) * (p[0][1] - p[2][1]);
        if area.abs() < 1e-12 {
            continue;
        }
        for y in lo[1]..=hi[1] {
            for x in lo[0]..=hi[0] {
                let fx = x as f64 + 0.5;
                let fy = y as f64 + 0.5;
                let a = ((p[1][1] - p[2][1]) * (fx - p[2][0])
                    + (p[2][0] - p[1][0]) * (fy - p[2][1]))
                    / area;
                let b = ((p[2][1] - p[0][1]) * (fx - p[2][0])
                    + (p[0][0] - p[2][0]) * (fy - p[2][1]))
                    / area;
                let c = 1. - a - b;
                if a >= -1e-8 && b >= -1e-8 && c >= -1e-8 {
                    let z = a * p[0][2] + b * p[1][2] + c * p[2][2];
                    let i = y * 512 + x;
                    depth[i] = depth[i].max(z);
                }
            }
        }
    }
    Ok(depth)
}
pub fn generate(mesh: &Mesh, spec: &Specification) -> Result<Sheet> {
    generate_with_notes(mesh, spec, &[])
}

/// Folha com notas derivadas do modelo (por exemplo, roscas dos furos) na legenda.
pub fn generate_with_notes(mesh: &Mesh, spec: &Specification, notes: &[String]) -> Result<Sheet> {
    generate_with_projections(mesh, spec, notes, None)
}

/// A geometria precisa chega como dados de apresentação próprios, sem ligar esta camada
/// ao núcleo CAD. Ausência de projeções usa o método aproximado de malhas explicitamente.
pub fn generate_with_projections(
    mesh: &Mesh,
    spec: &Specification,
    notes: &[String],
    projections: Option<&[ProjectedView]>,
) -> Result<Sheet> {
    spec.validate()?;
    if notes.len() > 64
        || notes
            .iter()
            .any(|n| n.len() > 256 || n.chars().any(char::is_control))
    {
        return Err(DrawingError("Notas da folha inválidas.".into()));
    }
    if projections.is_none() && mesh.triangles().len() > 200_000 {
        return Err(DrawingError(
            "Folhas de malha limitadas a duzentos mil triângulos.".into(),
        ));
    }
    let size = spec.size();
    let mut sheet = Sheet {
        size,
        style: spec.style,
        primitives: Vec::new(),
        exact_projection: projections.is_some(),
    };
    for (a, b) in [
        ([8., 8.], [size[0] - 8., 8.]),
        ([size[0] - 8., 8.], [size[0] - 8., size[1] - 8.]),
        ([size[0] - 8., size[1] - 8.], [8., size[1] - 8.]),
        ([8., size[1] - 8.], [8., 8.]),
    ] {
        sheet.line(a, b, 0.35, false);
    }
    sheet.line(
        [8., size[1] - 28.],
        [size[0] - 8., size[1] - 28.],
        0.25,
        false,
    );
    sheet.text([12., size[1] - 20.], 4., spec.title.clone());
    sheet.text(
        [12., size[1] - 12.],
        2.5,
        format!(
            "3dnator | mm | ESCALA {}:1 | REV {} | DATA {} | {}",
            format_mm(spec.scale),
            spec.revision,
            if spec.date.is_empty() {
                "-"
            } else {
                spec.date.as_str()
            },
            if projections.is_some() {
                if spec.has_sections() {
                    "HLR DO B-REP / CORTE DA MALHA"
                } else {
                    "HLR DO B-REP"
                }
            } else {
                "VISTAS DERIVADAS DA MALHA"
            }
        ),
    );
    // Notas na faixa da legenda: até três linhas, com a contagem do restante.
    for (row, note) in notes.iter().take(3).enumerate() {
        let value = if row == 2 && notes.len() > 3 {
            format!("{note} (+{} notas)", notes.len() - 3)
        } else {
            note.clone()
        };
        sheet.text(
            [size[0] * 0.58, size[1] - 22. + row as f64 * 4.5],
            2.5,
            format!("{}. {value}", row + 1),
        );
    }
    if let Some(views) = projections {
        let mut count = 0;
        for (index, view) in views.iter().enumerate() {
            count += view.segments.len();
            if views[..index].iter().any(|v| v.view == view.view)
                || count > 500000
                || view
                    .segments
                    .iter()
                    .any(|s| s.a.iter().chain(&s.b).any(|v| !v.is_finite()))
            {
                return Err(DrawingError(
                    "Projeções da folha inválidas ou excessivas.".into(),
                ));
            }
        }
    }
    let edges = if projections.is_none() {
        feature_edges(mesh)
    } else {
        Vec::new()
    };
    if edges.len() > 600_000 {
        return Err(DrawingError(
            "Malha com arestas demais para a folha.".into(),
        ));
    }
    let sections = spec.section_views();
    let rows = if sections.is_empty() {
        2
    } else {
        (3 + sections.len()).div_ceil(2)
    };
    let cell = [(size[0] - 36.) / 2., (size[1] - 58.) / rows as f64];
    // As vistas ortogonais vêm primeiro; os cortes ocupam células adicionais com
    // identificadores estáveis. Sem cortes, a quarta célula mostra a isométrica.
    let views: &[View] = if spec.has_sections() {
        &[View::Front, View::Right, View::Top]
    } else {
        &[View::Front, View::Right, View::Top, View::Isometric]
    };
    for (index, view) in views.iter().copied().enumerate() {
        let basis = view.basis();
        let projected = if let Some(projections) = projections {
            Some(projections.iter().find(|p| p.view == view).ok_or_else(|| {
                DrawingError("Uma vista não recebeu sua projeção precisa.".into())
            })?)
        } else {
            None
        };
        let points = if let Some(projected) = projected {
            if projected.segments.is_empty() {
                return Err(DrawingError("A vista precisa não contém segmentos.".into()));
            }
            projected
                .segments
                .iter()
                .flat_map(|s| [s.a, s.b])
                .collect::<Vec<_>>()
        } else {
            mesh.vertices()
                .iter()
                .map(|p| [dot(*p, basis.0), dot(*p, basis.1)])
                .collect::<Vec<_>>()
        };
        let min = std::array::from_fn::<_, 2, _>(|i| {
            points.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min)
        });
        let max = std::array::from_fn::<_, 2, _>(|i| {
            points
                .iter()
                .map(|p| p[i])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let extent = [max[0] - min[0], max[1] - min[1]];
        if extent[0] * spec.scale > cell[0] - 16. || extent[1] * spec.scale > cell[1] - 16. {
            return Err(DrawingError(
                "A escala não cabe na folha; reduza a escala ou use A3.".into(),
            ));
        }
        let origin = [
            18. + (index % 2) as f64 * cell[0] + cell[0] / 2.,
            12. + (index / 2) as f64 * cell[1] + cell[1] / 2.,
        ];
        let map = |p: [f64; 2]| {
            [
                origin[0] + (p[0] - (min[0] + max[0]) / 2.) * spec.scale,
                origin[1] - (p[1] - (min[1] + max[1]) / 2.) * spec.scale,
            ]
        };
        sheet.text(
            [origin[0] - 18., origin[1] + cell[1] / 2. - 1.],
            2.5,
            view.label(),
        );
        if let Some(projected) = projected {
            for segment in &projected.segments {
                if !segment.hidden || spec.hidden_lines {
                    sheet.line(
                        map(segment.a),
                        map(segment.b),
                        if segment.hidden { 0.12 } else { 0.25 },
                        segment.hidden,
                    );
                }
            }
        } else {
            let span = extent[0].max(extent[1]).max(1e-6);
            let depth = depth_buffer(mesh, basis, min, span)?;
            for e in &edges {
                let visible_edge = e.normals.len() != 2
                    || dot(e.normals[0], e.normals[1]) < 0.998
                    || dot(e.normals[0], basis.2) * dot(e.normals[1], basis.2) < 0.;
                if !visible_edge {
                    continue;
                }
                let (a, b) = (
                    [dot(e.a, basis.0), dot(e.a, basis.1)],
                    [dot(e.b, basis.0), dot(e.b, basis.1)],
                );
                let count =
                    (((a[0] - b[0]).hypot(a[1] - b[1]) / span * 500.).ceil() as usize).max(1);
                let mut runs: Vec<(f64, f64, bool)> = Vec::new();
                for i in 0..count {
                    let lo = i as f64 / count as f64;
                    let hi = (i + 1) as f64 / count as f64;
                    let t = (lo + hi) / 2.;
                    let p = std::array::from_fn::<_, 3, _>(|k| e.a[k] + (e.b[k] - e.a[k]) * t);
                    let x = ((dot(p, basis.0) - min[0]) * 500. / span + 6.)
                        .floor()
                        .clamp(0., 511.) as usize;
                    let y = ((dot(p, basis.1) - min[1]) * 500. / span + 6.)
                        .floor()
                        .clamp(0., 511.) as usize;
                    let hidden = dot(p, basis.2) < depth[y * 512 + x] - span * 0.006;
                    if let Some(last) = runs.last_mut().filter(|r| r.2 == hidden) {
                        last.1 = hi;
                    } else {
                        runs.push((lo, hi, hidden));
                    }
                }
                for (lo, hi, hidden) in runs {
                    if !hidden || spec.hidden_lines {
                        sheet.line(
                            map([a[0] + (b[0] - a[0]) * lo, a[1] + (b[1] - a[1]) * lo]),
                            map([a[0] + (b[0] - a[0]) * hi, a[1] + (b[1] - a[1]) * hi]),
                            if hidden { 0.12 } else { 0.25 },
                            hidden,
                        );
                    }
                }
            }
        }
        for (section, label) in &sections {
            cutting_plane_trace(&mut sheet, mesh, section, basis, &map, &label.to_string())?;
        }
        if spec.dimensions && view != View::Isometric {
            let a = map(min);
            let b = map(max);
            sheet.dimension(a, [b[0], a[1]], [0., 7.], format_mm(extent[0]));
            sheet.dimension(a, [a[0], b[1]], [-7., 0.], format_mm(extent[1]));
        }
    }
    for (index, (section, label)) in sections.iter().enumerate() {
        let position = index + 3;
        let origin = [
            18. + cell[0] * (0.5 + (position % 2) as f64),
            12. + cell[1] * (0.5 + (position / 2) as f64),
        ];
        section_view(
            &mut sheet,
            mesh,
            section,
            spec.scale,
            origin,
            cell,
            &label.to_string(),
        )?;
    }
    if sheet.primitives.len() > 500_000 {
        return Err(DrawingError("Folha contém elementos demais.".into()));
    }
    Ok(sheet)
}
/// Eixos U/V de um plano de corte, com U × V = normal.
fn section_basis(section: &Section) -> (Point3, Point3, Point3) {
    let n = section.normal.map(|v| v / norm(section.normal));
    let helper = if n[2].abs() < 0.9 {
        [0., 0., 1.]
    } else {
        [0., 1., 0.]
    };
    let u = cross(helper, n);
    let u = u.map(|v| v / norm(u));
    (u, cross(n, u), n)
}

/// Encadeia segmentos de corte em contornos fechados, ligando extremidades coincidentes.
/// Cadeias abertas (malha com furos) são descartadas: hachurar exige contorno fechado.
pub fn section_loops(segments: &[[Point3; 2]]) -> Vec<Vec<Point3>> {
    let key = |p: Point3| p.map(|v| (v * 1e6).round() as i64);
    let mut by_point: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
    for (i, [a, b]) in segments.iter().enumerate() {
        by_point.entry(key(*a)).or_default().push(i);
        by_point.entry(key(*b)).or_default().push(i);
    }
    let mut used = vec![false; segments.len()];
    let mut loops = Vec::new();
    for start in 0..segments.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let first = segments[start][0];
        let mut points = vec![first];
        let mut current = segments[start][1];
        let mut closed = false;
        for _ in 0..segments.len() {
            if key(current) == key(first) {
                closed = true;
                break;
            }
            points.push(current);
            let Some(next) = by_point
                .get(&key(current))
                .and_then(|list| list.iter().copied().find(|i| !used[*i]))
            else {
                break;
            };
            used[next] = true;
            let [a, b] = segments[next];
            current = if key(a) == key(current) { b } else { a };
        }
        if closed && points.len() >= 3 {
            loops.push(points);
        }
    }
    loops
}

/// Hachura a 45° de contornos 2D pela regra par-ímpar, com espaçamento `spacing`.
pub fn hatch(loops: &[Vec<[f64; 2]>], spacing: f64) -> Vec<[[f64; 2]; 2]> {
    if loops.is_empty() || !spacing.is_finite() || spacing <= 0. {
        return Vec::new();
    }
    // Coordenada rotacionada: t = (x + y)/√2 percorre as linhas, s = (x - y)/√2 as cruza.
    let r = std::f64::consts::FRAC_1_SQRT_2;
    let rotate = |p: [f64; 2]| [(p[0] - p[1]) * r, (p[0] + p[1]) * r];
    let back = |q: [f64; 2]| [(q[0] + q[1]) * r, (q[1] - q[0]) * r];
    let edges = loops
        .iter()
        .flat_map(|l| (0..l.len()).map(move |i| (rotate(l[i]), rotate(l[(i + 1) % l.len()]))))
        .collect::<Vec<_>>();
    let (lo, hi) = edges
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), (a, b)| {
            (lo.min(a[1]).min(b[1]), hi.max(a[1]).max(b[1]))
        });
    let mut lines = Vec::new();
    let steps = ((hi - lo) / spacing).ceil() as usize;
    if steps > 20_000 {
        return lines;
    }
    for k in 0..steps {
        let t = lo + (k as f64 + 0.5) * spacing;
        let mut hits = edges
            .iter()
            .filter(|(a, b)| (a[1] > t) != (b[1] > t))
            .map(|(a, b)| a[0] + (t - a[1]) * (b[0] - a[0]) / (b[1] - a[1]))
            .collect::<Vec<_>>();
        hits.sort_by(f64::total_cmp);
        for [enter, leave] in hits.as_chunks::<2>().0 {
            lines.push([back([*enter, t]), back([*leave, t])]);
        }
    }
    lines
}

/// Traço do plano nas vistas em que aparece como linha, com o identificador do corte.
fn cutting_plane_trace(
    sheet: &mut Sheet,
    mesh: &Mesh,
    section: &Section,
    basis: (Point3, Point3, Point3),
    map: &dyn Fn([f64; 2]) -> [f64; 2],
    label: &str,
) -> Result<()> {
    let n = section.normal.map(|v| v / norm(section.normal));
    // O plano só vira linha quando é perpendicular ao plano da vista.
    if dot(n, basis.2).abs() > 1e-6 {
        return Ok(());
    }
    let points = section_segments(mesh, section)?
        .into_iter()
        .flatten()
        .map(|p| map([dot(p, basis.0), dot(p, basis.1)]))
        .collect::<Vec<_>>();
    let direction = [dot(n, basis.1), -dot(n, basis.0)];
    let along = |p: [f64; 2]| p[0] * direction[0] - p[1] * direction[1];
    let (Some(a), Some(b)) = (
        points
            .iter()
            .copied()
            .min_by(|p, q| along(*p).total_cmp(&along(*q))),
        points
            .iter()
            .copied()
            .max_by(|p, q| along(*p).total_cmp(&along(*q))),
    ) else {
        return Ok(());
    };
    let d = [b[0] - a[0], b[1] - a[1]];
    let length = d[0].hypot(d[1]);
    if length < 1e-9 {
        return Ok(());
    }
    let u = [d[0] / length, d[1] / length];
    let (p, q) = (
        [a[0] - u[0] * 6., a[1] - u[1] * 6.],
        [b[0] + u[0] * 6., b[1] + u[1] * 6.],
    );
    sheet.line(p, q, 0.25, true);
    sheet.text([p[0] - 2., p[1] - 1.], 3.5, label);
    sheet.text([q[0] + 1., q[1] - 1.], 3.5, label);
    Ok(())
}

/// Vista em corte identificada: contornos da seção no plano de corte, hachurados.
fn section_view(
    sheet: &mut Sheet,
    mesh: &Mesh,
    section: &Section,
    scale: f64,
    origin: [f64; 2],
    cell: [f64; 2],
    label: &str,
) -> Result<()> {
    let (u, v, _) = section_basis(section);
    let loops = section_loops(&section_segments(mesh, section)?)
        .into_iter()
        .map(|l| {
            l.into_iter()
                .map(|p| {
                    [
                        dot(sub(p, section.origin), u),
                        dot(sub(p, section.origin), v),
                    ]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    sheet.text(
        [origin[0] - 18., origin[1] + cell[1] / 2. - 1.],
        2.5,
        format!("CORTE {label}-{label}"),
    );
    if loops.is_empty() {
        sheet.text(origin, 2.5, "O PLANO NÃO CORTA O MODELO");
        return Ok(());
    }
    let (min, max) = loops.iter().flatten().fold(
        ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]),
        |(lo, hi), p| {
            (
                [lo[0].min(p[0]), lo[1].min(p[1])],
                [hi[0].max(p[0]), hi[1].max(p[1])],
            )
        },
    );
    if (max[0] - min[0]) * scale > cell[0] - 16. || (max[1] - min[1]) * scale > cell[1] - 16. {
        return Err(DrawingError(
            "O corte não cabe na folha; reduza a escala ou use A3.".into(),
        ));
    }
    let map = |p: [f64; 2]| {
        [
            origin[0] + (p[0] - (min[0] + max[0]) / 2.) * scale,
            origin[1] - (p[1] - (min[1] + max[1]) / 2.) * scale,
        ]
    };
    for l in &loops {
        for i in 0..l.len() {
            sheet.line(map(l[i]), map(l[(i + 1) % l.len()]), 0.35, false);
        }
    }
    // Hachura com 2,5 mm de espaçamento no papel.
    for [a, b] in hatch(&loops, 2.5 / scale) {
        sheet.line(map(a), map(b), 0.13, false);
    }
    Ok(())
}

/// Interseção visual com triângulos. Não altera os corpos, nem os converte em sólidos.
pub fn section_segments(mesh: &Mesh, section: &Section) -> Result<Vec<[Point3; 2]>> {
    let spec = Specification {
        section: Some(section.clone()),
        ..Specification::default()
    };
    spec.validate()?;
    let mut segments = Vec::new();
    let n = section.normal.map(|v| v / norm(section.normal));
    for t in mesh.triangles() {
        let p = mesh.triangle_points(*t);
        let mut hits = Vec::new();
        for (a, b) in [(p[0], p[1]), (p[1], p[2]), (p[2], p[0])] {
            let da = dot(sub(a, section.origin), n);
            let db = dot(sub(b, section.origin), n);
            if da.abs() < 1e-9 && !hits.iter().any(|h| norm(sub(*h, a)) < 1e-8) {
                hits.push(a);
            }
            if da * db < 0. {
                let h = std::array::from_fn(|i| a[i] + (b[i] - a[i]) * da / (da - db));
                if !hits.iter().any(|p| norm(sub(*p, h)) < 1e-8) {
                    hits.push(h);
                }
            }
        }
        if hits.len() == 2 && norm(sub(hits[0], hits[1])) > 1e-9 {
            segments.push([hits[0], hits[1]]);
        }
    }
    Ok(segments)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multiple_sections_keep_their_labels_in_portrait_and_landscape() -> Result<()> {
        let mesh = nator_geometry::reference_bracket().map_err(|e| DrawingError(e.to_string()))?;
        let mut spec = Specification {
            section: Some(Section {
                origin: [0., 0., 3.],
                normal: [0., 0., 1.],
            }),
            additional_sections: vec![SectionView {
                id: 2,
                section: Section {
                    origin: [0., 0., 10.],
                    normal: [0., 0., 1.],
                },
            }],
            orientation: Orientation::Portrait,
            png_dpi: 96,
            ..Default::default()
        };
        for orientation in [Orientation::Portrait, Orientation::Landscape] {
            spec.orientation = orientation;
            let sheet = generate(&mesh, &spec)?;
            let texts = sheet
                .primitives
                .iter()
                .filter_map(|p| {
                    if let Primitive::Text { value, .. } = p {
                        Some(value.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert!(texts.contains(&"CORTE A-A") && texts.contains(&"CORTE C-C"));
            assert!(!texts.contains(&"CORTE B-B"));
            assert_eq!(
                sheet.size,
                if orientation == Orientation::Portrait {
                    [210., 297.]
                } else {
                    [297., 210.]
                }
            );
            let image = png(&sheet, spec.png_dpi)?;
            let width = u32::from_be_bytes([image[16], image[17], image[18], image[19]]);
            let height = u32::from_be_bytes([image[20], image[21], image[22], image[23]]);
            assert_eq!(width, (sheet.size[0] * 96. / 25.4).round() as u32);
            assert_eq!(height, (sheet.size[1] * 96. / 25.4).round() as u32);
        }
        spec.section = None;
        assert!(spec.has_sections());
        let sheet = generate(&mesh, &spec)?;
        assert!(
            sheet
                .primitives
                .iter()
                .any(|p| matches!(p,Primitive::Text {value,..} if value=="CORTE C-C"))
        );
        Ok(())
    }
    #[test]
    fn invalid_section_identifiers_normals_and_png_resolution_are_rejected() {
        let view = SectionView {
            id: 1,
            section: Section {
                origin: [0.; 3],
                normal: [0., 0., 1.],
            },
        };
        let mut spec = Specification {
            additional_sections: vec![view.clone(), view],
            ..Default::default()
        };
        assert!(spec.validate().is_err());
        spec.additional_sections.pop();
        spec.additional_sections[0].id = u32::MAX;
        assert_eq!(spec.section_views()[0].1, '?');
        assert!(spec.validate().is_err());
        spec.additional_sections[0].id = 1;
        spec.additional_sections[0].section.normal = [f64::MAX; 3];
        assert!(spec.validate().is_err());
        spec.additional_sections.clear();
        spec.png_dpi = 301;
        assert!(spec.validate().is_err());
    }
    #[test]
    fn sheet_preserves_scale_dimensions_and_pdf_vectors() -> Result<()> {
        let mesh = nator_geometry::reference_bracket().map_err(|e| DrawingError(e.to_string()))?;
        let spec = Specification::default();
        let sheet = generate(&mesh, &spec)?;
        assert!(
            sheet
                .primitives
                .iter()
                .any(|p| matches!(p,Primitive::Text{value,..} if value=="60"))
        );
        let pdf = pdf(&sheet)?;
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.windows(5).any(|s| s == b"xref\n"));
        let png = png(&sheet, 96)?;
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        Ok(())
    }
    #[test]
    fn numbers_use_decimal_comma_without_trailing_zeros() {
        assert_eq!(format_mm(60.), "60");
        assert_eq!(format_mm(12.5), "12,5");
        assert_eq!(format_mm(0.1234), "0,123");
        assert_eq!(format_mm(-0.0001), "0");
    }

    #[test]
    fn section_view_has_closed_hatched_outline_and_labels() -> Result<()> {
        let mesh = nator_geometry::reference_bracket().map_err(|e| DrawingError(e.to_string()))?;
        // Corte horizontal a 3 mm: atravessa só a base, um retângulo 60 × 20.
        let section = Section {
            origin: [0., 0., 3.],
            normal: [0., 0., 1.],
        };
        let loops = section_loops(&section_segments(&mesh, &section)?);
        assert_eq!(loops.len(), 1);
        let (u, v, _) = section_basis(&section);
        let flat = loops[0]
            .iter()
            .map(|p| [dot(*p, u), dot(*p, v)])
            .collect::<Vec<_>>();
        let area = (0..flat.len())
            .map(|i| {
                let (a, b) = (flat[i], flat[(i + 1) % flat.len()]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            .abs()
            / 2.;
        assert!((area - 1200.).abs() < 1e-6);
        // Hachuras ficam dentro do contorno e somam o comprimento esperado.
        let lines = hatch(&[flat], 2.5);
        let total = lines
            .iter()
            .map(|[a, b]| (a[0] - b[0]).hypot(a[1] - b[1]))
            .sum::<f64>();
        assert!((total - 1200. / 2.5).abs() < 25.);
        let spec = Specification {
            section: Some(section),
            revision: "B".into(),
            date: "26/10/2026".into(),
            ..Specification::default()
        };
        let sheet = generate_with_notes(&mesh, &spec, &["Rosca M6 × 1 em 2 furos".into()])?;
        let texts = sheet
            .primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Text { value, .. } => Some(value.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(texts.contains(&"CORTE A-A"));
        assert!(texts.iter().filter(|t| **t == "A").count() >= 2);
        assert!(
            texts
                .iter()
                .any(|t| t.contains("REV B") && t.contains("26/10/2026"))
        );
        assert!(texts.iter().any(|t| t.contains("M6")));
        Ok(())
    }

    #[test]
    fn sections_lie_on_the_requested_plane() -> Result<()> {
        let mesh = nator_geometry::reference_bracket().map_err(|e| DrawingError(e.to_string()))?;
        let s = section_segments(
            &mesh,
            &Section {
                origin: [0., 0., 10.],
                normal: [0., 0., 1.],
            },
        )?;
        assert!(!s.is_empty());
        assert!(s.iter().flatten().all(|p| (p[2] - 10.).abs() < 1e-9));
        Ok(())
    }
}
