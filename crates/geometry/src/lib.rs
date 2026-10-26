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

    /// Identificador de posição de cada vértice: vértices com coordenadas idênticas (bit a
    /// bit, com -0 igual a +0) recebem o mesmo identificador. Não há tolerância.
    fn point_ids(&self) -> Vec<u32> {
        let mut point_ids = HashMap::with_capacity(self.vertices.len());
        self.vertices
            .iter()
            .map(|p| {
                let next = point_ids.len() as u32;
                *point_ids.entry(position_key(*p)).or_insert(next)
            })
            .collect()
    }

    /// Funde vértices de coordenadas idênticas, sem tolerância. A geometria não muda: só os
    /// índices passam a compartilhar posições. STL repete cada vértice em cada triângulo
    /// (72 bytes por triângulo em f64); uma malha fechada fundida usa cerca de 12.
    pub fn welded(self) -> Mesh {
        let ids = self.point_ids();
        let count = ids.iter().map(|id| *id as usize + 1).max().unwrap_or(0);
        if count == self.vertices.len() {
            return self;
        }
        let mut vertices = vec![[0.; 3]; count];
        for (index, id) in ids.iter().enumerate() {
            vertices[*id as usize] = self.vertices[index];
        }
        let triangles = self
            .triangles
            .iter()
            .map(|t| t.map(|i| ids[i as usize]))
            .collect();
        Mesh {
            vertices,
            triangles,
            bounds: self.bounds,
        }
    }

    /// Imagem espelhada pelo plano (origem, normal). A reflexão inverte a orientação, então
    /// a ordem dos vértices de cada triângulo também é invertida para manter as normais
    /// apontando para fora.
    pub fn reflected(&self, origin: Point3, normal: Point3) -> Result<Mesh, GeometryError> {
        let length = norm(normal);
        if !length.is_finite() || length < 1e-12 || origin.iter().any(|v| !v.is_finite()) {
            return Err(GeometryError("Plano de espelhamento inválido.".into()));
        }
        let n = normal.map(|v| v / length);
        let vertices = self
            .vertices
            .iter()
            .map(|p| {
                let distance = dot(sub(*p, origin), n);
                std::array::from_fn(|i| p[i] - 2. * distance * n[i])
            })
            .collect();
        let triangles = self
            .triangles
            .iter()
            .map(|[a, b, c]| [*a, *c, *b])
            .collect();
        Mesh::new(vertices, triangles)
    }

    /// Usa coordenadas exatas para reconhecer vértices repetidos no STL, sem solda por tolerância.
    /// Uma borda quase coincidente permanece aberta e precisa de reparo explícito.
    pub fn topology(&self) -> Topology {
        self.edge_analysis(None).0
    }

    /// Topologia e arestas vivas em uma única passada pelas arestas.
    ///
    /// A máscara tem um byte por triângulo: o bit k marca a aresta oposta ao canto k (entre
    /// os cantos k+1 e k+2), a convenção das coordenadas baricêntricas do shader. Uma aresta
    /// é viva quando as normais dos dois triângulos vizinhos diferem mais que `crease_rad`,
    /// quando é borda aberta ou quando é compartilhada por mais de dois triângulos.
    pub fn edge_analysis(&self, crease_rad: Option<f64>) -> (Topology, Vec<u8>) {
        let ids = self.point_ids();
        let limit = crease_rad.map(f64::cos);
        let normals = limit.map(|_| {
            self.triangles
                .iter()
                .map(|t| {
                    let [a, b, c] = self.triangle_points(*t);
                    let n = cross(sub(b, a), sub(c, a));
                    n.map(|v| v / norm(n))
                })
                .collect::<Vec<_>>()
        });
        let mut mask = vec![
            0_u8;
            if limit.is_some() {
                self.triangles.len()
            } else {
                0
            }
        ];
        let bit = |local: usize| 1_u8 << ((local + 2) % 3);
        // Por aresta: primeira ocorrência (triângulo·3 + aresta local), contagem e saldo de
        // sentido (+1/-1), que é zero quando os dois triângulos têm orientação coerente.
        let mut edges: HashMap<u64, (u32, u32, i32)> =
            HashMap::with_capacity(self.triangles.len() * 2);
        for (index, t) in self.triangles.iter().enumerate() {
            for local in 0..3 {
                let a = ids[t[local] as usize];
                let b = ids[t[(local + 1) % 3] as usize];
                let key = (u64::from(a.min(b)) << 32) | u64::from(a.max(b));
                let entry = edges
                    .entry(key)
                    .or_insert(((index * 3 + local) as u32, 0, 0));
                entry.1 += 1;
                entry.2 += if a < b { 1 } else { -1 };
                if entry.1 > 1
                    && let (Some(limit), Some(normals)) = (limit, &normals)
                {
                    let first = (entry.0 / 3) as usize;
                    // Dobra acentuada ou aresta não manifold: os dois lados são marcados.
                    if entry.1 > 2 || dot(normals[first], normals[index]) < limit {
                        mask[first] |= bit((entry.0 % 3) as usize);
                        mask[index] |= bit(local);
                    }
                }
            }
        }
        let mut topology = Topology {
            boundary_edges: 0,
            non_manifold_edges: 0,
            inconsistent_edges: 0,
        };
        for (first, count, direction) in edges.into_values() {
            match count {
                1 => {
                    topology.boundary_edges += 1;
                    if limit.is_some() {
                        mask[(first / 3) as usize] |= bit((first % 3) as usize);
                    }
                }
                2 if direction != 0 => topology.inconsistent_edges += 1,
                2 => {}
                _ => topology.non_manifold_edges += 1,
            }
        }
        (topology, mask)
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

/// Chave de igualdade exata de posição; -0 e +0 são a mesma coordenada.
fn position_key(p: Point3) -> [u64; 3] {
    p.map(|x| if x == 0.0 { 0 } else { x.to_bits() })
}

pub fn add(a: Point3, b: Point3) -> Point3 {
    std::array::from_fn(|i| a[i] + b[i])
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

    #[test]
    fn reflection_keeps_volume_and_outward_orientation() -> Result<(), Box<dyn Error>> {
        let mesh = reference_bracket()?;
        let mirrored = mesh.reflected([0.; 3], [1., 0., 0.])?;
        assert_eq!(mirrored.bounds().min, [-60., 0., 0.]);
        assert!((mirrored.signed_volume() - mesh.signed_volume()).abs() < 1e-9);
        assert!(mirrored.topology().has_closed_oriented_edges());
        assert!(mesh.reflected([0.; 3], [0.; 3]).is_err());
        Ok(())
    }

    /// Como chega de um STL: três vértices próprios por triângulo.
    fn triangle_soup(mesh: &Mesh) -> Result<Mesh, GeometryError> {
        let vertices = mesh
            .triangles()
            .iter()
            .flat_map(|t| mesh.triangle_points(*t))
            .collect();
        let triangles = (0..mesh.triangles().len() as u32)
            .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
            .collect();
        Mesh::new(vertices, triangles)
    }

    #[test]
    fn welding_shares_identical_positions_without_changing_geometry() -> Result<(), Box<dyn Error>>
    {
        let soup = triangle_soup(&reference_bracket()?)?;
        assert_eq!(soup.vertices().len(), 60);
        let welded = soup.clone().welded();
        // O prisma em L tem 12 cantos distintos.
        assert_eq!(welded.vertices().len(), 12);
        assert_eq!(welded.triangles().len(), soup.triangles().len());
        assert_eq!(welded.signed_volume(), soup.signed_volume());
        assert!(welded.topology().has_closed_oriented_edges());
        // Fundir de novo não altera nada.
        assert_eq!(welded.clone().welded().vertices(), welded.vertices());
        Ok(())
    }

    #[test]
    fn creases_mark_sharp_edges_on_both_sides_and_skip_coplanar_diagonals()
    -> Result<(), Box<dyn Error>> {
        let mesh = reference_bracket()?;
        let (topology, mask) = mesh.edge_analysis(Some(30_f64.to_radians()));
        assert!(topology.has_closed_oriented_edges());
        // 18 arestas do prisma (6 em cada tampa e 6 laterais), marcadas nos dois triângulos.
        // As diagonais das tampas e das laterais são coplanares e não aparecem.
        let bits = mask.iter().map(|m| m.count_ones()).sum::<u32>();
        assert_eq!(bits, 36);
        // A máscara não depende de os vértices estarem fundidos.
        assert_eq!(
            triangle_soup(&mesh)?
                .edge_analysis(Some(30_f64.to_radians()))
                .1,
            mask
        );
        // Uma face removida deixa bordas abertas, que também são desenhadas.
        let mut triangles = mesh.triangles().to_vec();
        triangles.pop();
        let open = Mesh::new(mesh.vertices().to_vec(), triangles)?;
        let (topology, mask) = open.edge_analysis(Some(30_f64.to_radians()));
        assert!(topology.boundary_edges > 0);
        assert!(mask.iter().map(|m| m.count_ones()).sum::<u32>() > 30);
        Ok(())
    }
}
