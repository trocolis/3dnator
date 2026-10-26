//! Folhas vetoriais derivadas da geometria de apresentação, sem dependência da interface.
use nator_geometry::{Mesh, Point3, cross, dot, norm, sub};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specification {
    pub title: String,
    pub paper: Paper,
    pub style: Style,
    pub scale: f64,
    pub hidden_lines: bool,
    pub dimensions: bool,
    pub section: Option<Section>,
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
        }
    }
}
impl Specification {
    pub fn size(&self) -> [f64; 2] {
        match self.paper {
            Paper::A4 => [297., 210.],
            Paper::A3 => [420., 297.],
        }
    }
    pub fn validate(&self) -> Result<()> {
        if !self.scale.is_finite()
            || self.scale <= 0.
            || self.scale > 1000.
            || self.title.len() > 512
            || self.title.chars().any(char::is_control)
        {
            return Err(DrawingError("Título ou escala da folha inválidos.".into()));
        }
        if let Some(s) = &self.section
            && (s.origin.iter().chain(&s.normal).any(|v| !v.is_finite()) || norm(s.normal) < 1e-9)
        {
            return Err(DrawingError("Plano de corte inválido.".into()));
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
    spec.validate()?;
    if mesh.triangles().len() > 200_000 {
        return Err(DrawingError(
            "Folhas de malha limitadas a duzentos mil triângulos.".into(),
        ));
    }
    let size = spec.size();
    let mut sheet = Sheet {
        size,
        style: spec.style,
        primitives: Vec::new(),
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
            "3dnator | mm | ESCALA {}:1 | VISTAS DERIVADAS DA MALHA",
            spec.scale
        ),
    );
    let edges = feature_edges(mesh);
    if edges.len() > 600_000 {
        return Err(DrawingError(
            "Malha com arestas demais para a folha.".into(),
        ));
    }
    let cell = [(size[0] - 36.) / 2., (size[1] - 58.) / 2.];
    for (index, view) in [View::Front, View::Right, View::Top, View::Isometric]
        .into_iter()
        .enumerate()
    {
        let basis = view.basis();
        let points = mesh
            .vertices()
            .iter()
            .map(|p| [dot(*p, basis.0), dot(*p, basis.1)])
            .collect::<Vec<_>>();
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
            let count = (((a[0] - b[0]).hypot(a[1] - b[1]) / span * 500.).ceil() as usize).max(1);
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
        if let Some(section) = &spec.section {
            for [a, b] in section_segments(mesh, section)? {
                sheet.line(
                    map([dot(a, basis.0), dot(a, basis.1)]),
                    map([dot(b, basis.0), dot(b, basis.1)]),
                    0.7,
                    false,
                );
            }
        }
        if spec.dimensions && view != View::Isometric {
            let a = map(min);
            let b = map(max);
            sheet.dimension(a, [b[0], a[1]], [0., 7.], format!("{:.3}", extent[0]));
            sheet.dimension(a, [a[0], b[1]], [-7., 0.], format!("{:.3}", extent[1]));
        }
    }
    if sheet.primitives.len() > 500_000 {
        return Err(DrawingError("Folha contém elementos demais.".into()));
    }
    Ok(sheet)
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
    fn sheet_preserves_scale_dimensions_and_pdf_vectors() -> Result<()> {
        let mesh = nator_geometry::reference_bracket().map_err(|e| DrawingError(e.to_string()))?;
        let spec = Specification::default();
        let sheet = generate(&mesh, &spec)?;
        assert!(
            sheet
                .primitives
                .iter()
                .any(|p| matches!(p,Primitive::Text{value,..} if value=="60.000"))
        );
        let pdf = pdf(&sheet)?;
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.windows(5).any(|s| s == b"xref\n"));
        let png = png(&sheet, 96)?;
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
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
