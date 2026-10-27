//! Centros geométricos de triângulos e retângulos no próprio plano do contorno.
//! Nunca usamos o centro da caixa envolvente: ele é incorreto em faces inclinadas.
use crate::{Mesh, Point3, cross, dot, norm, sub};
use std::collections::HashMap;

pub fn polygon_center(vertices: &[Point3]) -> Option<Point3> {
    let mut points = vertices.to_vec();
    if points.iter().flatten().any(|v| !v.is_finite()) {
        return None;
    }
    // Segmentos colineares podem ter sido subdivididos por encontros ou pela malha.
    // Remover apenas pontos intermediários, sem apagar uma reversão do contorno.
    loop {
        let count = points.len();
        if count < 3 {
            return None;
        }
        let redundant = (0..count).find(|i| {
            let a = sub(points[*i], points[(i + count - 1) % count]);
            let b = sub(points[(i + 1) % count], points[*i]);
            norm(a) < 1e-9 || (dot(a, b) >= 0. && norm(cross(a, b)) <= 1e-9 * norm(a) * norm(b))
        });
        if let Some(i) = redundant {
            points.remove(i);
        } else {
            break;
        }
    }
    if points.len() != 3 && points.len() != 4 {
        return None;
    }
    let normal = cross(sub(points[1], points[0]), sub(points[2], points[0]));
    let length = norm(normal);
    if !length.is_finite() || length <= 1e-12 {
        return None;
    }
    if points.len() == 4 {
        if dot(sub(points[3], points[0]), normal).abs() > 1e-7 * length {
            return None;
        }
        for i in 0..4 {
            let a = sub(points[(i + 1) % 4], points[i]);
            let b = sub(points[(i + 2) % 4], points[(i + 1) % 4]);
            if dot(a, b).abs() > 1e-8 * norm(a) * norm(b) {
                return None;
            }
        }
    }
    Some(std::array::from_fn(|i| {
        points.iter().map(|p| p[i] / points.len() as f64).sum()
    }))
}

/// Reconstrói faces planas da malha antes de procurar centros. Diagonais de tesselação
/// não criam triângulos fictícios; componentes curvos ou contornos ramificados não passam.
/// Consulta para cache/worker, sem soldar bordas quase coincidentes.
pub fn planar_polygon_centers(mesh: &Mesh) -> Vec<Point3> {
    let welded = mesh.clone().welded();
    let triangles = welded.triangles();
    let normals = triangles
        .iter()
        .map(|t| {
            let [a, b, c] = welded.triangle_points(*t);
            let n = cross(sub(b, a), sub(c, a));
            n.map(|v| v / norm(n))
        })
        .collect::<Vec<_>>();
    let mut edges: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (i, triangle) in triangles.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (triangle[k], triangle[(k + 1) % 3]);
            edges.entry((a.min(b), a.max(b))).or_default().push(i);
        }
    }
    let mut neighbors = vec![Vec::new(); triangles.len()];
    for adjacent in edges.values() {
        if let [a, b] = adjacent.as_slice()
            && dot(normals[*a], normals[*b]) > 1. - 1e-12
        {
            neighbors[*a].push(*b);
            neighbors[*b].push(*a);
        }
    }
    let mut seen = vec![false; triangles.len()];
    let mut result = Vec::new();
    for first in 0..triangles.len() {
        if seen[first] {
            continue;
        }
        let mut stack = vec![first];
        seen[first] = true;
        let mut boundary = HashMap::<(u32, u32), usize>::new();
        while let Some(i) = stack.pop() {
            for k in 0..3 {
                let (a, b) = (triangles[i][k], triangles[i][(k + 1) % 3]);
                *boundary.entry((a.min(b), a.max(b))).or_default() += 1;
            }
            for &next in &neighbors[i] {
                if !seen[next] {
                    seen[next] = true;
                    stack.push(next);
                }
            }
        }
        let mut links = HashMap::<u32, Vec<u32>>::new();
        for ((a, b), count) in boundary {
            if count == 1 {
                links.entry(a).or_default().push(b);
                links.entry(b).or_default().push(a);
            }
        }
        if links.is_empty() || links.values().any(|v| v.len() != 2) {
            continue;
        }
        let Some(&start) = links.keys().min() else {
            continue;
        };
        let mut ordered = Vec::new();
        let (mut previous, mut current) = (u32::MAX, start);
        loop {
            ordered.push(welded.vertices()[current as usize]);
            let next = links[&current].iter().find(|v| **v != previous).copied();
            let Some(next) = next else {
                break;
            };
            previous = current;
            current = next;
            if current == start || ordered.len() > links.len() {
                break;
            }
        }
        // Mais de um contorno (furo) não vira um polígono preenchido por engano.
        if current == start
            && ordered.len() == links.len()
            && let Some(center) = polygon_center(&ordered)
        {
            result.push(center);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn triangle_centroid_and_rotated_rectangle_center_follow_their_planes() -> Result<()> {
        let pose = crate::Placement::new([5., -9., 12.], [0.3, -0.7, 0.4])?;
        for (vertices, expected) in [
            (
                vec![[0., 0., 0.], [12., 0., 0.], [0., 6., 0.]],
                [4., 2., 0.],
            ),
            (
                vec![
                    [0., 0., 0.],
                    [6., 0., 0.],
                    [12., 0., 0.],
                    [12., 6., 0.],
                    [0., 6., 0.],
                ],
                [6., 3., 0.],
            ),
        ] {
            let points = vertices
                .into_iter()
                .map(|p| pose.apply(p))
                .collect::<Vec<_>>();
            let center = polygon_center(&points).ok_or("Centro ausente")?;
            assert!(norm(sub(center, pose.apply(expected))) < 1e-9);
        }
        assert!(
            polygon_center(&[[0., 0., 0.], [12., 0., 0.], [9., 6., 0.], [0., 6., 0.]]).is_none()
        );
        assert!(
            polygon_center(&[[0., 0., 0.], [12., 0., 0.], [12., 6., 0.], [0., 6., 1.]]).is_none()
        );
        Ok(())
    }

    #[test]
    fn mesh_diagonals_do_not_invent_triangle_centers() -> Result<()> {
        let mesh = Mesh::new(
            vec![[0., 0., 0.], [12., 0., 0.], [12., 6., 0.], [0., 6., 0.]],
            vec![[0, 1, 2], [0, 2, 3]],
        )?;
        assert_eq!(planar_polygon_centers(&mesh), vec![[6., 3., 0.]]);
        let mesh = Mesh::new(
            vec![[0., 0., 0.], [12., 0., 0.], [0., 6., 0.]],
            vec![[0, 1, 2]],
        )?;
        assert_eq!(planar_polygon_centers(&mesh), vec![[4., 2., 0.]]);
        Ok(())
    }
}
