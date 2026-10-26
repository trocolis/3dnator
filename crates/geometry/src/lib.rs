//! Tipos próprios compartilhados: a malha de apresentação não substitui o sólido CAD.

use std::{collections::HashMap, error::Error, fmt};

mod placement;
pub mod query;
pub use placement::Placement;

pub type Point3 = [f64; 3];

#[derive(Debug, Clone, PartialEq)]
pub struct GeometryError(pub String);

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for GeometryError {}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min: Point3,
    pub max: Point3,
}

impl Bounds {
    pub fn size(self) -> Point3 {
        std::array::from_fn(|i| self.max[i] - self.min[i])
    }
    pub fn center(self) -> Point3 {
        std::array::from_fn(|i| self.min[i] * 0.5 + self.max[i] * 0.5)
    }
}

/// Dados privados impedem criar índices inválidos depois da validação.
#[derive(Debug, Clone)]
pub struct Mesh {
    vertices: Vec<Point3>,
    triangles: Vec<[u32; 3]>,
    bounds: Bounds,
}

impl Mesh {
    pub fn new(vertices: Vec<Point3>, triangles: Vec<[u32; 3]>) -> Result<Self, GeometryError> {
        let first = vertices
            .first()
            .copied()
            .ok_or_else(|| GeometryError("A malha não tem vértices.".into()))?;
        if triangles.is_empty() {
            return Err(GeometryError("A malha não tem triângulos.".into()));
        }
        let mut bounds = Bounds {
            min: first,
            max: first,
        };
        for point in &vertices {
            for (axis, coordinate) in point.iter().copied().enumerate() {
                if !coordinate.is_finite() {
                    return Err(GeometryError("Coordenada não finita na malha.".into()));
                }
                bounds.min[axis] = bounds.min[axis].min(coordinate);
                bounds.max[axis] = bounds.max[axis].max(coordinate);
            }
        }
        if bounds.size().iter().any(|n| !n.is_finite()) {
            return Err(GeometryError(
                "Dimensões fora do intervalo numérico.".into(),
            ));
        }
        for triangle in &triangles {
            let [a, b, c] = triangle.map(|index| vertices.get(index as usize));
            let (Some(a), Some(b), Some(c)) = (a, b, c) else {
                return Err(GeometryError("Índice de triângulo inválido.".into()));
            };
            let normal = cross(sub(*b, *a), sub(*c, *a));
            let length = norm(normal);
            if !length.is_finite() || length <= 0.0 {
                return Err(GeometryError(
                    "Triângulo degenerado ou fora do intervalo numérico.".into(),
                ));
            }
        }
        Ok(Self {
            vertices,
            triangles,
            bounds,
        })
    }

    pub fn vertices(&self) -> &[Point3] {
        &self.vertices
    }
    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }
    pub fn bounds(&self) -> Bounds {
        self.bounds
    }
    pub fn triangle_points(&self, triangle: [u32; 3]) -> [Point3; 3] {
        // Os índices já foram validados e as coleções não podem ser alteradas externamente.
        triangle.map(|index| self.vertices[index as usize])
    }

    /// Usa coordenadas exatas para reconhecer vértices repetidos no STL, sem solda por tolerância.
    /// Uma borda quase coincidente permanece aberta e precisa de reparo explícito.
    pub fn topology(&self) -> Topology {
        let mut point_ids = HashMap::new();
        let mut next_id = 0_u64;
        let ids: Vec<u64> = self
            .vertices
            .iter()
            .map(|p| {
                let key = p.map(|x| if x == 0.0 { 0 } else { x.to_bits() });
                *point_ids.entry(key).or_insert_with(|| {
                    let id = next_id;
                    next_id += 1;
                    id
                })
            })
            .collect();
        let mut edges: HashMap<(u64, u64), (usize, i64)> = HashMap::new();
        for t in &self.triangles {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let (a, b) = (ids[a as usize], ids[b as usize]);
                let entry = edges.entry((a.min(b), a.max(b))).or_default();
                entry.0 += 1;
                entry.1 += if a < b { 1 } else { -1 };
            }
        }
        Topology {
            boundary_edges: edges.values().filter(|(count, _)| *count == 1).count(),
            non_manifold_edges: edges.values().filter(|(count, _)| *count > 2).count(),
            inconsistent_edges: edges
                .values()
                .filter(|(count, direction)| *count == 2 && *direction != 0)
                .count(),
        }
    }

    /// Centroide volumétrico da malha fechada, calculado por tetraedros orientados.
    pub fn mass_center(&self) -> Result<(Point3, f64), GeometryError> {
        let origin = self.bounds.center();
        let mut moment = [0.; 3];
        let mut volume = 0.;
        for t in &self.triangles {
            let [a, b, c] = self.triangle_points(*t).map(|p| sub(p, origin));
            let v = dot(a, cross(b, c)) / 6.;
            volume += v;
            for i in 0..3 {
                moment[i] += (a[i] + b[i] + c[i]) * v / 4.;
            }
        }
        if !volume.is_finite() || volume.abs() < 1e-12 {
            return Err(GeometryError(
                "Volume nulo ou inválido para centro de massa.".into(),
            ));
        }
        Ok((
            std::array::from_fn(|i| origin[i] + moment[i] / volume),
            volume,
        ))
    }
    pub fn signed_volume(&self) -> f64 {
        // Transladar a referência reduz cancelamento numérico em modelos longe da origem.
        let origin = self.bounds.center();
        self.triangles
            .iter()
            .map(|t| {
                let [a, b, c] = self.triangle_points(*t).map(|p| sub(p, origin));
                dot(a, cross(b, c)) / 6.0
            })
            .sum()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Topology {
    pub boundary_edges: usize,
    pub non_manifold_edges: usize,
    pub inconsistent_edges: usize,
}

impl Topology {
    /// Verificação local das arestas; não detecta auto-interseções ou cascas sobrepostas.
    pub fn has_closed_oriented_edges(self) -> bool {
        self.boundary_edges == 0 && self.non_manifold_edges == 0 && self.inconsistent_edges == 0
    }
}

pub fn sub(a: Point3, b: Point3) -> Point3 {
    std::array::from_fn(|i| a[i] - b[i])
}
pub fn dot(a: Point3, b: Point3) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
pub fn cross(a: Point3, b: Point3) -> Point3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn norm(v: Point3) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

/// Suporte em L de referência, explicitamente uma malha, sem operações CAD simuladas.
pub fn reference_bracket() -> Result<Mesh, GeometryError> {
    let profile = [
        [0., 0.],
        [60., 0.],
        [60., 6.],
        [6., 6.],
        [6., 30.],
        [0., 30.],
    ];
    let vertices = [0., 20.]
        .into_iter()
        .flat_map(|y| profile.map(|[x, z]| [x, y, z]))
        .collect();
    let caps = [[0, 1, 2], [0, 2, 3], [0, 3, 5], [3, 4, 5]];
    let mut triangles = Vec::new();
    for [a, b, c] in caps {
        triangles.push([a, b, c]);
        triangles.push([c + 6, b + 6, a + 6]);
    }
    for a in 0..6 {
        let b = (a + 1) % 6;
        triangles.push([a, a + 6, b + 6]);
        triangles.push([a, b + 6, b]);
    }
    Mesh::new(vertices, triangles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracket_is_closed_and_has_analytical_volume() -> Result<(), Box<dyn Error>> {
        let mesh = reference_bracket()?;
        assert_eq!(mesh.bounds().size(), [60., 20., 30.]);
        assert!(mesh.topology().has_closed_oriented_edges());
        assert!((mesh.signed_volume() - 10_080.).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn rejects_invalid_indices_and_degenerate_triangles() {
        assert!(Mesh::new(vec![[0.; 3]], vec![[0, 1, 2]]).is_err());
        assert!(Mesh::new(vec![[0.; 3]], vec![[0, 0, 0]]).is_err());
    }

    #[test]
    fn detects_open_and_reversed_faces() -> Result<(), Box<dyn Error>> {
        let mesh = reference_bracket()?;
        let mut triangles = mesh.triangles().to_vec();
        triangles.pop();
        let open = Mesh::new(mesh.vertices().to_vec(), triangles)?;
        assert!(open.topology().boundary_edges > 0);
        let mut triangles = mesh.triangles().to_vec();
        triangles[0].swap(0, 1);
        let reversed = Mesh::new(mesh.vertices().to_vec(), triangles)?;
        assert!(reversed.topology().inconsistent_edges > 0);
        Ok(())
    }
}
