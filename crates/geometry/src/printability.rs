//! Análise de imprimibilidade FDM sobre a malha já orientada na mesa (Z para cima). São
//! estimativas geométricas, sem fatiar: balanços pela normal, espessuras e frestas por raios.
use crate::{Mesh, Point3, add, cross, dot, norm, position_key, sub};
use std::collections::HashMap;

/// Um raio só mede espessura ou fresta se encontrar uma superfície de frente: em cantos e
/// chanfros a parede vizinha está de lado e não forma parede fina nem fresta.
const FACING: f64 = -0.7;
/// Acima disto, a face é tratada como horizontal: sua espessura depende das camadas.
const HORIZONTAL: f64 = 0.7;
/// Distância mínima do raio: evita reencontrar a própria superfície por arredondamento.
const RAY_START: f64 = 1e-7;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrintLimits {
    /// Ângulo máximo a partir da vertical: 0° é parede, 90° é teto.
    pub max_overhang_deg: f64,
    /// Parede mínima em faces laterais, em mm.
    pub min_wall_mm: f64,
    /// Espessura mínima em faces quase horizontais, em mm.
    pub min_floor_mm: f64,
    /// Fresta mínima, em mm: abaixo disto o bico a preenche.
    pub min_gap_mm: f64,
    /// Regiões menores que isto, em mm², são ruído da triangulação.
    pub min_region_mm2: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Issue {
    Overhang,
    ThinWall,
    Gap,
}
impl Issue {
    pub const ALL: [Self; 3] = [Self::Gap, Self::ThinWall, Self::Overhang];
    fn bit(self) -> u8 {
        match self {
            Self::Overhang => 1,
            Self::ThinWall => 2,
            Self::Gap => 4,
        }
    }
}

/// Trecho contínuo com o mesmo problema. `worst` é o ângulo em graus nos balanços e a menor
/// medida em mm nas paredes e frestas.
#[derive(Debug, Clone, PartialEq)]
pub struct IssueRegion {
    pub issue: Issue,
    pub triangles: Vec<u32>,
    pub area: f64,
    pub worst: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Printability {
    /// Máscara de problemas por triângulo; a ordem é a da malha analisada.
    masks: Vec<u8>,
    /// Regiões da maior para a menor, por problema.
    pub regions: Vec<IssueRegion>,
    /// Área apoiada na mesa, em mm².
    pub contact_area: f64,
}
impl Printability {
    pub fn has(&self, triangle: usize, issue: Issue) -> bool {
        self.masks
            .get(triangle)
            .is_some_and(|mask| mask & issue.bit() != 0)
    }
}

/// Árvore de caixas sobre os triângulos: os raios são locais e curtos, então só visitam a
/// vizinhança da origem, mesmo em corpos de centenas de milhares de triângulos.
struct Tree {
    nodes: Vec<Node>,
    order: Vec<u32>,
}
struct Node {
    min: Point3,
    max: Point3,
    /// Folha com `count` triângulos a partir de `start`; ramo com filhos em `start` e `start + 1`.
    start: u32,
    count: u32,
}
const LEAF: usize = 4;

impl Tree {
    fn new(mesh: &Mesh) -> Self {
        let triangles = mesh.triangles();
        let boxes = triangles
            .iter()
            .map(|t| {
                let [a, b, c] = mesh.triangle_points(*t);
                (
                    std::array::from_fn::<f64, 3, _>(|i| a[i].min(b[i]).min(c[i])),
                    std::array::from_fn::<f64, 3, _>(|i| a[i].max(b[i]).max(c[i])),
                )
            })
            .collect::<Vec<_>>();
        let mut tree = Self {
            nodes: Vec::with_capacity(triangles.len()),
            order: (0..triangles.len() as u32).collect(),
        };
        tree.nodes.push(Node {
            min: [0.; 3],
            max: [0.; 3],
            start: 0,
            count: 0,
        });
        tree.split(0, 0, triangles.len(), &boxes);
        tree
    }
    fn split(&mut self, node: usize, start: usize, end: usize, boxes: &[(Point3, Point3)]) {
        let (mut min, mut max) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for id in &self.order[start..end] {
            let (low, high) = boxes[*id as usize];
            min = std::array::from_fn(|i| min[i].min(low[i]));
            max = std::array::from_fn(|i| max[i].max(high[i]));
        }
        self.nodes[node].min = min;
        self.nodes[node].max = max;
        if end - start <= LEAF {
            self.nodes[node].start = start as u32;
            self.nodes[node].count = (end - start) as u32;
            return;
        }
        let axis = (0..3)
            .max_by(|a, b| (max[*a] - min[*a]).total_cmp(&(max[*b] - min[*b])))
            .unwrap_or(0);
        let middle = start + (end - start) / 2;
        let center = |id: &u32| {
            let (low, high) = boxes[*id as usize];
            low[axis] + high[axis]
        };
        self.order[start..end]
            .select_nth_unstable_by(middle - start, |a, b| center(a).total_cmp(&center(b)));
        let left = self.nodes.len();
        for _ in 0..2 {
            self.nodes.push(Node {
                min: [0.; 3],
                max: [0.; 3],
                start: 0,
                count: 0,
            });
        }
        self.nodes[node].start = left as u32;
        self.split(left, start, middle, boxes);
        self.split(left + 1, middle, end, boxes);
    }
    /// Triângulo mais próximo a menos de `limit` da origem, ignorando `own`.
    fn cast(
        &self,
        mesh: &Mesh,
        origin: Point3,
        direction: Point3,
        limit: f64,
        own: u32,
    ) -> Option<(u32, f64)> {
        let mut best = None;
        let mut reach = limit;
        let mut stack = vec![0_usize];
        while let Some(index) = stack.pop() {
            let node = &self.nodes[index];
            if !reaches(node, origin, direction, reach) {
                continue;
            }
            if node.count == 0 {
                stack.extend([node.start as usize, node.start as usize + 1]);
                continue;
            }
            let leaf = node.start as usize..(node.start + node.count) as usize;
            for id in &self.order[leaf] {
                if *id == own {
                    continue;
                }
                let points = mesh.triangle_points(mesh.triangles()[*id as usize]);
                if let Some(t) = intersect(origin, direction, points)
                    && t < reach
                {
                    reach = t;
                    best = Some((*id, t));
                }
            }
        }
        best
    }
}

/// Teste de lajes: o raio cruza a caixa antes de `reach`?
fn reaches(node: &Node, origin: Point3, direction: Point3, reach: f64) -> bool {
    let (mut near, mut far) = (0_f64, reach);
    for axis in 0..3 {
        if direction[axis].abs() < 1e-12 {
            if origin[axis] < node.min[axis] || origin[axis] > node.max[axis] {
                return false;
            }
            continue;
        }
        let a = (node.min[axis] - origin[axis]) / direction[axis];
        let b = (node.max[axis] - origin[axis]) / direction[axis];
        near = near.max(a.min(b));
        far = far.min(a.max(b));
        if near > far {
            return false;
        }
    }
    true
}

/// Möller–Trumbore: distância ao longo do raio, dos dois lados do triângulo.
fn intersect(origin: Point3, direction: Point3, [a, b, c]: [Point3; 3]) -> Option<f64> {
    let (u, v) = (sub(b, a), sub(c, a));
    let p = cross(direction, v);
    let determinant = dot(u, p);
    if determinant.abs() < 1e-14 {
        return None;
    }
    let offset = sub(origin, a);
    let s = dot(offset, p) / determinant;
    if !(-1e-9..=1. + 1e-9).contains(&s) {
        return None;
    }
    let q = cross(offset, u);
    let t = dot(direction, q) / determinant;
    if t < -1e-9 || s + t > 1. + 1e-9 {
        return None;
    }
    Some(dot(v, q) / determinant).filter(|distance| *distance > RAY_START)
}

/// Quatro amostras por triângulo: o centro e o meio de cada canto. Um só raio chamaria de
/// fina uma face longa que só estreita na ponta.
fn samples([a, b, c]: [Point3; 3]) -> [Point3; 4] {
    let mix = |p: Point3, q: Point3, r: Point3| -> Point3 {
        std::array::from_fn(|i| (2. * p[i] + q[i] + r[i]) / 4.)
    };
    [
        std::array::from_fn(|i| (a[i] + b[i] + c[i]) / 3.),
        mix(a, b, c),
        mix(b, c, a),
        mix(c, a, b),
    ]
}

/// Menor distância, em ao menos duas amostras, até uma superfície de frente.
fn opposing(
    tree: &Tree,
    mesh: &Mesh,
    id: u32,
    points: [Point3; 3],
    normal: Point3,
    direction: Point3,
    limit: f64,
) -> Option<f64> {
    let triangles = mesh.triangles();
    let mut hits = 0;
    let mut least = f64::INFINITY;
    for sample in samples(points) {
        let Some((other, distance)) = tree.cast(mesh, sample, direction, limit, id) else {
            continue;
        };
        let facing = unit_normal(mesh.triangle_points(triangles[other as usize]));
        if facing.is_some_and(|facing| dot(facing, normal) < FACING) {
            hits += 1;
            least = least.min(distance);
        }
    }
    (hits >= 2).then_some(least)
}

fn unit_normal([a, b, c]: [Point3; 3]) -> Option<Point3> {
    let normal = cross(sub(b, a), sub(c, a));
    let length = norm(normal);
    (length > 1e-14).then(|| normal.map(|v| v / length))
}

/// Agrupa triângulos marcados que compartilham aresta.
fn connected(mesh: &Mesh, marked: &[u32]) -> Vec<Vec<u32>> {
    let mut edges: HashMap<([u64; 3], [u64; 3]), Vec<usize>> = HashMap::new();
    for (index, id) in marked.iter().enumerate() {
        let [a, b, c] = mesh
            .triangle_points(mesh.triangles()[*id as usize])
            .map(position_key);
        for (p, q) in [(a, b), (b, c), (c, a)] {
            edges.entry((p.min(q), p.max(q))).or_default().push(index);
        }
    }
    let mut parent = (0..marked.len()).collect::<Vec<_>>();
    fn root(parent: &mut [usize], mut node: usize) -> usize {
        while parent[node] != node {
            parent[node] = parent[parent[node]];
            node = parent[node];
        }
        node
    }
    for group in edges.values() {
        for pair in group.windows(2) {
            let (a, b) = (root(&mut parent, pair[0]), root(&mut parent, pair[1]));
            parent[a] = b;
        }
    }
    let mut groups: HashMap<usize, Vec<u32>> = HashMap::new();
    for (index, id) in marked.iter().enumerate() {
        groups
            .entry(root(&mut parent, index))
            .or_default()
            .push(*id);
    }
    groups.into_values().collect()
}

/// Analisa `mesh`, com a mesa em Z = 0 e o material acima dela.
pub fn analyze(mesh: &Mesh, limits: PrintLimits) -> Printability {
    let triangles = mesh.triangles();
    let tree = Tree::new(mesh);
    let bed = mesh.bounds().min[2];
    let overhang_sine = limits.max_overhang_deg.to_radians().sin();
    let reach = limits.min_wall_mm.max(limits.min_floor_mm);
    let mut marked: [(Issue, Vec<u32>); 3] = Issue::ALL.map(|issue| (issue, Vec::new()));
    let mut worst = vec![[f64::INFINITY; 3]; triangles.len()];
    let mut contact_area = 0.;
    for (index, triangle) in triangles.iter().enumerate() {
        let points = mesh.triangle_points(*triangle);
        let Some(normal) = unit_normal(points) else {
            continue;
        };
        let id = index as u32;
        let flat = normal[2].abs() > HORIZONTAL;
        let lowest = points.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min);
        let highest = points
            .iter()
            .map(|p| p[2])
            .fold(f64::NEG_INFINITY, f64::max);
        let resting = normal[2] < -0.999 && highest - bed < 1e-6;
        if resting {
            contact_area += norm(cross(sub(points[1], points[0]), sub(points[2], points[0]))) / 2.;
        }
        for (slot, (issue, list)) in marked.iter_mut().enumerate() {
            let measured = match issue {
                Issue::Gap => opposing(&tree, mesh, id, points, normal, normal, limits.min_gap_mm),
                Issue::ThinWall => {
                    let limit = if flat {
                        limits.min_floor_mm
                    } else {
                        limits.min_wall_mm
                    };
                    let inward = normal.map(|v| -v);
                    opposing(&tree, mesh, id, points, normal, inward, limit.min(reach))
                }
                Issue::Overhang => (!resting && lowest >= bed && -normal[2] > overhang_sine + 1e-6)
                    .then(|| (-normal[2]).clamp(-1., 1.).asin().to_degrees()),
            };
            if let Some(value) = measured {
                list.push(id);
                worst[index][slot] = value;
            }
        }
    }
    let mut masks = vec![0_u8; triangles.len()];
    let mut regions = Vec::new();
    for (slot, (issue, list)) in marked.iter().enumerate() {
        let mut found = connected(mesh, list)
            .into_iter()
            .map(|group| {
                let area = group
                    .iter()
                    .map(|id| {
                        let [a, b, c] = mesh.triangle_points(triangles[*id as usize]);
                        norm(cross(sub(b, a), sub(c, a))) / 2.
                    })
                    .sum::<f64>();
                let values = group.iter().map(|id| worst[*id as usize][slot]);
                let value = if *issue == Issue::Overhang {
                    values.fold(0., f64::max)
                } else {
                    values.fold(f64::INFINITY, f64::min)
                };
                IssueRegion {
                    issue: *issue,
                    triangles: group,
                    area,
                    worst: value,
                }
            })
            .filter(|region| region.area >= limits.min_region_mm2)
            .collect::<Vec<_>>();
        found.sort_by(|a, b| b.area.total_cmp(&a.area));
        for region in &found {
            for id in &region.triangles {
                masks[*id as usize] |= issue.bit();
            }
        }
        regions.extend(found);
    }
    Printability {
        masks,
        regions,
        contact_area,
    }
}

/// Soma de vetores para quem acumula pontos ponderados.
pub fn weighted_center(mesh: &Mesh, triangles: &[u32]) -> Option<Point3> {
    let (mut sum, mut total) = ([0.; 3], 0.);
    for id in triangles {
        let [a, b, c] = mesh.triangle_points(*mesh.triangles().get(*id as usize)?);
        let area = norm(cross(sub(b, a), sub(c, a))) / 2.;
        let center: Point3 = std::array::from_fn(|i| (a[i] + b[i] + c[i]) / 3.);
        sum = add(sum, center.map(|v| v * area));
        total += area;
    }
    (total > 0.).then(|| sum.map(|v| v / total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    const LIMITS: PrintLimits = PrintLimits {
        max_overhang_deg: 45.,
        min_wall_mm: 0.8,
        min_floor_mm: 0.4,
        min_gap_mm: 0.4,
        min_region_mm2: 0.16,
    };

    /// Junta caixas (origem, tamanho) em uma malha, cada uma um volume fechado.
    fn boxes(list: &[(Point3, Point3)]) -> Result<Mesh, Box<dyn Error>> {
        let (mut vertices, mut triangles) = (Vec::new(), Vec::new());
        for ([x, y, z], [w, d, h]) in list.iter().copied() {
            let base = vertices.len() as u32;
            vertices.extend([
                [x, y, z],
                [x + w, y, z],
                [x + w, y + d, z],
                [x, y + d, z],
                [x, y, z + h],
                [x + w, y, z + h],
                [x + w, y + d, z + h],
                [x, y + d, z + h],
            ]);
            let faces: [[u32; 3]; 12] = [
                [0, 2, 1],
                [0, 3, 2],
                [4, 5, 6],
                [4, 6, 7],
                [0, 1, 5],
                [0, 5, 4],
                [1, 2, 6],
                [1, 6, 5],
                [2, 3, 7],
                [2, 7, 6],
                [3, 0, 4],
                [3, 4, 7],
            ];
            triangles.extend(faces.map(|f| f.map(|i| i + base)));
        }
        Ok(Mesh::new(vertices, triangles)?)
    }
    fn count(result: &Printability, issue: Issue) -> usize {
        result.regions.iter().filter(|r| r.issue == issue).count()
    }

    #[test]
    fn a_plain_block_has_no_issues_and_rests_on_the_bed() -> Result<(), Box<dyn Error>> {
        let result = analyze(&boxes(&[([0.; 3], [20., 30., 10.])])?, LIMITS);
        assert!(result.regions.is_empty(), "{:?}", result.regions);
        assert!((result.contact_area - 600.).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn floating_underside_is_an_overhang_but_the_base_is_not() -> Result<(), Box<dyn Error>> {
        let mesh = boxes(&[([0.; 3], [5., 5., 10.]), ([0., 0., 10.], [30., 5., 5.])])?;
        let result = analyze(&mesh, LIMITS);
        assert_eq!(count(&result, Issue::Overhang), 1);
        let region = &result.regions[0];
        assert!((region.worst - 90.).abs() < 1e-9);
        assert!((region.area - 150.).abs() < 1e-9);
        assert!((result.contact_area - 25.).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn walls_thinner_than_two_nozzles_are_measured() -> Result<(), Box<dyn Error>> {
        let thin = analyze(&boxes(&[([0.; 3], [0.5, 30., 10.])])?, LIMITS);
        assert_eq!(count(&thin, Issue::ThinWall), 2);
        assert!((thin.regions[0].worst - 0.5).abs() < 1e-9);
        let thick = analyze(&boxes(&[([0.; 3], [1.2, 30., 10.])])?, LIMITS);
        assert_eq!(count(&thick, Issue::ThinWall), 0);
        // Chão de 0,6 mm é aceitável: em Z valem as camadas, não o bico.
        let floor = analyze(&boxes(&[([0.; 3], [30., 30., 0.6])])?, LIMITS);
        assert_eq!(count(&floor, Issue::ThinWall), 0);
        Ok(())
    }

    #[test]
    fn slits_narrower_than_the_nozzle_are_gaps_but_wide_ones_are_not() -> Result<(), Box<dyn Error>>
    {
        let close = boxes(&[([0.; 3], [5., 20., 10.]), ([5.2, 0., 0.], [5., 20., 10.])])?;
        let result = analyze(&close, LIMITS);
        assert_eq!(count(&result, Issue::Gap), 2);
        assert!((result.regions[0].worst - 0.2).abs() < 1e-9);
        let far = boxes(&[([0.; 3], [5., 20., 10.]), ([6., 0., 0.], [5., 20., 10.])])?;
        assert_eq!(count(&analyze(&far, LIMITS), Issue::Gap), 0);
        Ok(())
    }

    /// O canto interno de um L tem paredes a 90°: elas se veem, mas não formam fresta.
    #[test]
    fn inner_corners_are_not_gaps() -> Result<(), Box<dyn Error>> {
        let (mut vertices, mut triangles) = (Vec::new(), Vec::new());
        // Perfil em L extrudado, com faixas estreitas junto ao canto.
        let outline: [[f64; 2]; 8] = [
            [0., 0.],
            [20., 0.],
            [20., 5.],
            [5.1, 5.],
            [5., 5.],
            [5., 5.1],
            [5., 20.],
            [0., 20.],
        ];
        for z in [0., 10.] {
            vertices.extend(outline.iter().map(|p| [p[0], p[1], z]));
        }
        for i in 0..8_u32 {
            let j = (i + 1) % 8;
            triangles.push([i, j, j + 8]);
            triangles.push([i, j + 8, i + 8]);
        }
        for t in [
            [0, 2, 1],
            [0, 3, 2],
            [0, 4, 3],
            [0, 5, 4],
            [0, 6, 5],
            [0, 7, 6],
        ] {
            triangles.push(t);
            triangles.push([t[0] + 8, t[2] + 8, t[1] + 8]);
        }
        let result = analyze(&Mesh::new(vertices, triangles)?, LIMITS);
        assert_eq!(count(&result, Issue::Gap), 0, "{:?}", result.regions);
        Ok(())
    }

    #[test]
    fn large_meshes_finish_because_rays_stay_local() -> Result<(), Box<dyn Error>> {
        let cells = (0..2000)
            .map(|i| {
                (
                    [f64::from(i % 50) * 3., f64::from(i / 50) * 3., 0.],
                    [2., 2., 5.],
                )
            })
            .collect::<Vec<_>>();
        let result = analyze(&boxes(&cells)?, LIMITS);
        assert!(result.regions.is_empty());
        assert!((result.contact_area - 8000.).abs() < 1e-6);
        Ok(())
    }
}
