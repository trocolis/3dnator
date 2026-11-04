//! Recorte de apresentação: tampas no plano, preservando furos e corpos separados.
//! Nenhum resultado deste módulo substitui o B-rep em operações ou exportações.
use crate::{Mesh, Point3, cross, dot, norm, silhouette::plane_basis, sub};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Debug)]
pub struct Cap {
    pub triangles: Vec<[Point3; 3]>,
    pub segments: Vec<[Point3; 2]>,
}

/// Interseção de uma malha de um único corpo com o plano. Contornos abertos ou
/// ramificados não recebem tampa. Faixas trapezoidais usam paridade, preservando
/// cavidades, ilhas e múltiplas cascas sem introduzir diagonais no contorno.
pub fn cap(mesh: &Mesh, range: std::ops::Range<usize>, origin: Point3, normal: Point3) -> Cap {
    let Some((u, v)) = plane_basis(normal) else {
        return Cap::default();
    };
    let normal = cross(u, v);
    let epsilon = (norm(mesh.bounds().size()) * 1e-10).max(1e-9);
    let key = |p: [f64; 2]| p.map(|x| (x / epsilon).round() as i64);
    let mut nodes = BTreeMap::new();
    let mut edges = BTreeSet::new();
    for triangle in &mesh.triangles()[range] {
        let p = mesh.triangle_points(*triangle);
        let d = p.map(|p| dot(sub(p, origin), normal));
        // Faces coplanares pertencem à superfície original; não duplicar sua área.
        if d.iter().all(|d| d.abs() <= epsilon) {
            continue;
        }
        let mut hits = Vec::new();
        for i in 0..3 {
            let j = (i + 1) % 3;
            if d[i].abs() <= epsilon {
                hits.push(p[i]);
            }
            if (d[i] < -epsilon && d[j] > epsilon) || (d[j] < -epsilon && d[i] > epsilon) {
                hits.push(std::array::from_fn(|k| {
                    p[i][k] + (p[j][k] - p[i][k]) * d[i] / (d[i] - d[j])
                }));
            }
        }
        let mut keys = Vec::new();
        for p in hits {
            let relative = sub(p, origin);
            let p = [dot(relative, u), dot(relative, v)];
            let k = key(p);
            nodes.entry(k).or_insert(p);
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        if let [a, b] = keys[..] {
            edges.insert((a.min(b), a.max(b)));
        }
    }
    let mut adjacency = BTreeMap::<_, Vec<_>>::new();
    for &(a, b) in &edges {
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
    }
    // Validar cada componente: uma malha aberta nunca inventa um fechamento.
    let mut seen = BTreeSet::new();
    let mut valid = BTreeSet::new();
    for &start in adjacency.keys() {
        if seen.contains(&start) {
            continue;
        }
        let mut pending = vec![start];
        let mut component = Vec::new();
        let mut closed = true;
        while let Some(at) = pending.pop() {
            if !seen.insert(at) {
                continue;
            }
            component.push(at);
            if let Some(next) = adjacency.get(&at) {
                closed &= next.len() == 2;
                pending.extend(next);
            }
        }
        if closed {
            valid.extend(component);
        }
    }
    let segments: Vec<_> = edges
        .into_iter()
        .filter(|(a, b)| valid.contains(a) && valid.contains(b))
        .map(|(a, b)| [nodes[&a], nodes[&b]])
        .collect();
    let world = |p: [f64; 2]| std::array::from_fn(|i| origin[i] + p[0] * u[i] + p[1] * v[i]);
    let mut result = Cap {
        segments: segments.iter().map(|s| s.map(world)).collect(),
        ..Cap::default()
    };
    let mut levels: Vec<_> = segments.iter().flatten().map(|p| p[1]).collect();
    levels.sort_by(f64::total_cmp);
    levels.dedup_by(|a, b| (*a - *b).abs() <= epsilon);
    for levels in levels.windows(2) {
        let [bottom, top] = [levels[0], levels[1]];
        let middle = (bottom + top) * 0.5;
        let x = |s: &[[f64; 2]; 2], y: f64| {
            s[0][0] + (s[1][0] - s[0][0]) * (y - s[0][1]) / (s[1][1] - s[0][1])
        };
        let mut active: Vec<_> = segments
            .iter()
            .filter(|s| middle > s[0][1].min(s[1][1]) && middle < s[0][1].max(s[1][1]))
            .collect();
        active.sort_by(|a, b| x(a, middle).total_cmp(&x(b, middle)));
        if active.len() % 2 != 0 {
            continue;
        }
        for pair in active.as_chunks::<2>().0 {
            let quad = [
                world([x(pair[0], bottom), bottom]),
                world([x(pair[1], bottom), bottom]),
                world([x(pair[1], top), top]),
                world([x(pair[0], top), top]),
            ];
            for t in [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]] {
                if norm(cross(sub(t[1], t[0]), sub(t[2], t[0]))) > epsilon * epsilon {
                    result.triangles.push(t);
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cap_keeps_an_annular_cavity_and_rejects_open_contours() -> Result<(), crate::GeometryError> {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        // Dois prismas concêntricos: a casca interna forma um furo por paridade.
        for radius in [4., 2.] {
            let base = vertices.len() as u32;
            for z in [-2., 2.] {
                for [x, y] in [
                    [-radius, -radius],
                    [radius, -radius],
                    [radius, radius],
                    [-radius, radius],
                ] {
                    vertices.push([x, y, z]);
                }
            }
            for i in 0..4 {
                let j = (i + 1) % 4;
                triangles.extend([
                    [base + i, base + j, base + j + 4],
                    [base + i, base + j + 4, base + i + 4],
                ]);
            }
        }
        let mesh = Mesh::new(vertices, triangles)?;
        let section = cap(&mesh, 0..mesh.triangles().len(), [0.; 3], [0., 0., 1.]);
        let area: f64 = section
            .triangles
            .iter()
            .map(|t| norm(cross(sub(t[1], t[0]), sub(t[2], t[0]))) * 0.5)
            .sum();
        assert!((area - 48.).abs() < 1e-8, "{area}");
        assert!(
            section
                .triangles
                .iter()
                .all(|t| t.iter().all(|p| p[2].abs() < 1e-12))
        );
        let open = cap(&mesh, 0..1, [0.; 3], [0., 0., 1.]);
        assert!(open.triangles.is_empty());
        Ok(())
    }
}
