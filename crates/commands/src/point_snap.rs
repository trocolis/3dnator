//! Apoios para movimentação ponto a ponto, consultados sem alterar o documento.
//! Centros circulares e de polígonos pertencem ao próprio plano da superfície, inclusive
//! no vazio. Triângulos usam o baricentro e retângulos, o encontro das diagonais;
//! furos assimétricos não deslocam esses apoios como deslocariam o centro de massa.
use nator_document::{Body, DocumentError};
use nator_geometry::Point3;
use nator_kernel::CircleAnchor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointKind {
    Vertex,
    EdgeMidpoint,
    CircleCenter,
    ArcCenter,
    CircleBoundary,
    FaceCenter,
}

#[derive(Debug, Clone)]
pub struct PointAnchor {
    /// Coordenadas locais do corpo; a interface compõe corpo e ocorrência na projeção.
    pub point: Point3,
    pub kind: PointKind,
    /// Presente somente no centro, para descobrir o apoio no disco e no vazio do furo.
    pub circle: Option<CircleAnchor>,
}

/// Apoios fixos de um rascunho de referência, no plano local e em mm. A projeção
/// e o alcance em tela pertencem à interface; as regras das curvas ficam aqui.
pub fn reference_points(
    sketch: &nator_sketch::Sketch,
) -> Result<Vec<nator_sketch::Point2>, DocumentError> {
    use nator_sketch::profiles::{Segment, segment};
    let mut points = sketch.points.iter().map(|p| p.position).collect::<Vec<_>>();
    for entity in &sketch.entities {
        match segment(sketch, entity).map_err(|e| DocumentError(e.to_string()))? {
            Segment::Line { a, b } => points.push([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]),
            Segment::Circle { center, radius } => points.extend([
                [center[0] + radius, center[1]],
                [center[0], center[1] + radius],
                [center[0] - radius, center[1]],
                [center[0], center[1] - radius],
            ]),
            arc @ Segment::Arc { .. } => {
                // Duas subdivisões angulares fornecem exatamente a metade do
                // arco, preservando o sentido e a passagem por 0/2π. Centro e
                // extremos já estão nos pontos originais do rascunho.
                if let Some(midpoint) = arc.sample(2).get(1) {
                    points.push(*midpoint);
                }
            }
        }
    }
    let references =
        nator_sketch::topology::references(sketch).map_err(|e| DocumentError(e.to_string()))?;
    points.extend(references.vertices);
    points.extend(references.centers);
    Ok(points)
}

/// Consulta potencialmente demorada: chamar fora da thread da interface e reaproveitar
/// o resultado enquanto a geometria não mudar. Malhas oferecem vértices e pontos
/// médios das arestas vivas, sem inventar círculos para contornos aproximados.
pub fn body_anchors(body: &Body) -> Result<Vec<PointAnchor>, DocumentError> {
    #[cfg(feature = "occt")]
    if let Some(bytes) = &body.brep {
        let query = || -> Result<Vec<PointAnchor>, nator_kernel::KernelError> {
            let solid = nator_kernel::Solid::from_brep(bytes, None)?;
            let mut vertices = Vec::new();
            let mut anchors = Vec::new();
            for face in 0..solid.face_count()? {
                if let Some(point) = solid.face_polygon_center(face)? {
                    anchors.push(PointAnchor {
                        point,
                        kind: PointKind::FaceCenter,
                        circle: None,
                    });
                }
            }
            for edge in 0..solid.edge_count()? {
                let supports = solid.edge_anchors(edge)?;
                if let Some(circle) = supports.circle {
                    anchors.push(PointAnchor {
                        point: circle.center,
                        kind: if circle.closed {
                            PointKind::CircleCenter
                        } else {
                            PointKind::ArcCenter
                        },
                        circle: Some(circle),
                    });
                    if circle.closed {
                        // Os quadrantes pertencem ao plano do círculo, acompanhando
                        // a geometria local e as poses posteriores do corpo/instância.
                        let y = nator_geometry::cross(circle.normal, circle.x_direction);
                        for direction in [
                            circle.x_direction,
                            y,
                            circle.x_direction.map(|v| -v),
                            y.map(|v| -v),
                        ] {
                            anchors.push(PointAnchor {
                                point: std::array::from_fn(|i| {
                                    circle.center[i] + direction[i] * circle.radius_mm
                                }),
                                kind: PointKind::CircleBoundary,
                                circle: None,
                            });
                        }
                    }
                }
                // Um círculo fechado tem uma costura topológica arbitrária, que
                // não deve competir com o centro como se fosse um canto da peça.
                if !supports.seam && !supports.circle.is_some_and(|c| c.closed) {
                    vertices.extend([supports.start, supports.end]);
                    anchors.push(PointAnchor {
                        point: supports.midpoint,
                        kind: PointKind::EdgeMidpoint,
                        circle: None,
                    });
                }
            }
            vertices.sort_by(|a, b| {
                a.iter()
                    .zip(b)
                    .map(|(a, b)| a.total_cmp(b))
                    .find(|o| !o.is_eq())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            vertices.dedup();
            anchors.extend(vertices.into_iter().map(|point| PointAnchor {
                point,
                kind: PointKind::Vertex,
                circle: None,
            }));
            Ok(anchors)
        };
        return query().map_err(|e| DocumentError(e.to_string()));
    }
    let mut anchors = body
        .geometry
        .mesh()
        .vertices()
        .iter()
        .map(|point| PointAnchor {
            point: *point,
            kind: PointKind::Vertex,
            circle: None,
        })
        .collect::<Vec<_>>();
    let mesh = body.geometry.mesh();
    anchors.extend(
        nator_geometry::planar_polygon_centers(mesh)
            .into_iter()
            .map(|point| PointAnchor {
                point,
                kind: PointKind::FaceCenter,
                circle: None,
            }),
    );
    let mut midpoints = std::collections::HashSet::new();
    for (triangle, mask) in mesh.triangles().iter().zip(body.geometry.creases()) {
        let points = mesh.triangle_points(*triangle);
        for corner in 0..3 {
            if mask & (1 << corner) == 0 {
                continue;
            }
            let a = points[(corner + 1) % 3];
            let b = points[(corner + 2) % 3];
            let point = std::array::from_fn(|i| (a[i] + b[i]) * 0.5);
            // STL pode repetir os vértices de cada triângulo. A chave geométrica
            // evita duplicar o apoio dos dois lados, normalizando também −0.
            let key = point.map(|v| if v == 0. { 0 } else { v.to_bits() });
            if midpoints.insert(key) {
                anchors.push(PointAnchor {
                    point,
                    kind: PointKind::EdgeMidpoint,
                    circle: None,
                });
            }
        }
    }
    Ok(anchors)
}

#[cfg(all(test, feature = "occt"))]
mod tests {
    use super::*;
    use nator_document::{Document, MeshAsset};
    use nator_kernel::Solid;
    use std::sync::Arc;

    fn anchors(mut solid: Solid) -> Result<Vec<PointAnchor>, Box<dyn std::error::Error>> {
        let (mesh, faces) = solid.tessellate_with_faces(0.5, 0.5)?;
        let mut doc = Document::new("Apoios")?;
        let (_, id) = doc.add_mesh(
            None,
            "Corpo".into(),
            Arc::new(MeshAsset::with_faces(mesh, faces)?),
        )?;
        doc.body_mut(id)?.brep = Some(Arc::new(solid.to_brep()?));
        Ok(body_anchors(doc.body(id).ok_or("Corpo ausente")?)?)
    }

    #[test]
    fn washer_centers_keep_both_surface_heights_without_arbitrary_seam_vertices()
    -> Result<(), Box<dyn std::error::Error>> {
        let solid = Solid::cylinder([0., 0., 3.], 20., 5.)?.cut(&Solid::cylinder(
            [0., 0., 2.],
            10.,
            7.,
        )?)?;
        let anchors = anchors(solid)?;
        assert_eq!(
            anchors.len(),
            20,
            "Quatro centros e dezesseis quadrantes, sem costuras"
        );
        for (radius, height) in [(10., 3.), (20., 3.), (10., 8.), (20., 8.)] {
            assert!(anchors.iter().any(|a| a.circle.is_some_and(|c| {
                c.closed
                    && (c.radius_mm - radius).abs() < 1e-9
                    && nator_geometry::norm(nator_geometry::sub(c.center, [0., 0., height])) < 1e-9
                    && (nator_geometry::norm(c.normal) - 1.).abs() < 1e-9
            })));
        }
        assert_eq!(
            anchors
                .iter()
                .filter(|a| a.kind == PointKind::CircleCenter)
                .count(),
            4
        );
        assert_eq!(
            anchors
                .iter()
                .filter(|a| a.kind == PointKind::CircleBoundary)
                .count(),
            16
        );
        for height in [3., 8.] {
            for radius in [10., 20.] {
                for point in [
                    [radius, 0., height],
                    [0., radius, height],
                    [-radius, 0., height],
                    [0., -radius, height],
                ] {
                    assert!(anchors.iter().any(|a| a.kind == PointKind::CircleBoundary
                        && nator_geometry::norm(nator_geometry::sub(a.point, point)) < 1e-9));
                }
            }
        }
        Ok(())
    }

    #[test]
    fn cuboid_offers_the_midpoint_of_each_real_edge() -> Result<(), Box<dyn std::error::Error>> {
        let anchors = anchors(Solid::cuboid([1., 2., 3.], [20., 10., 4.])?)?;
        assert_eq!(
            anchors
                .iter()
                .filter(|a| a.kind == PointKind::Vertex)
                .count(),
            8
        );
        assert_eq!(
            anchors
                .iter()
                .filter(|a| a.kind == PointKind::EdgeMidpoint)
                .count(),
            12
        );
        for point in [[11., 2., 7.], [21., 7., 7.], [1., 2., 5.]] {
            assert!(anchors.iter().any(|a| a.kind == PointKind::EdgeMidpoint
                && nator_geometry::norm(nator_geometry::sub(a.point, point)) < 1e-9));
        }
        let centers = anchors
            .iter()
            .filter(|a| a.kind == PointKind::FaceCenter)
            .collect::<Vec<_>>();
        assert_eq!(centers.len(), 6);
        for point in [[11., 7., 7.], [11., 7., 3.], [21., 7., 5.]] {
            assert!(
                centers
                    .iter()
                    .any(|a| nator_geometry::norm(nator_geometry::sub(a.point, point)) < 1e-9)
            );
        }
        Ok(())
    }

    #[test]
    fn triangular_face_centers_are_centroids_on_each_surface()
    -> Result<(), Box<dyn std::error::Error>> {
        use nator_sketch::{Plane, Sketch, areas};
        let mut sketch = Sketch::new("Triângulo", Plane::xy());
        for (a, b) in [
            ([0., 0.], [12., 0.]),
            ([12., 0.], [0., 6.]),
            ([0., 6.], [0., 0.]),
        ] {
            sketch.add_line(a, b, false)?;
        }
        let region = areas::regions(&sketch)?.remove(0);
        let anchors = anchors(Solid::extrude(&sketch, &region, 0., 2., "triangle")?)?;
        let centers = anchors
            .iter()
            .filter(|a| a.kind == PointKind::FaceCenter)
            .collect::<Vec<_>>();
        assert_eq!(centers.len(), 5);
        for point in [[4., 2., 0.], [4., 2., 2.]] {
            assert!(
                centers
                    .iter()
                    .any(|a| nator_geometry::norm(nator_geometry::sub(a.point, point)) < 1e-9)
            );
        }
        // O centro de caixa [6,3] não substitui o baricentro [4,2].
        assert!(!centers.iter().any(|a| a.point == [6., 3., 2.]));
        Ok(())
    }

    #[test]
    fn chamfer_corner_centers_match_the_eight_triangle_centroids()
    -> Result<(), Box<dyn std::error::Error>> {
        let cube = Solid::cuboid([0.; 3], [20.; 3])?;
        let edges = (0..cube.edge_count()?).collect::<Vec<_>>();
        let chamfered = cube.chamfer(&edges, 2., None, "chamfer")?;
        let mut corners = Vec::new();
        for face in 0..chamfered.face_count()? {
            // No cubo de 20 mm com chanfro de 2 mm, só os oito triângulos
            // possuem área inferior a 8 mm². A medida de massa superficial
            // fornece uma comparação independente da média dos vértices.
            if chamfered.face_measure(face)?.area < 8. {
                corners.push(chamfered.face_anchors(face)?.centroid);
            }
        }
        assert_eq!(corners.len(), 8);
        let anchors = anchors(chamfered)?;
        for center in corners {
            assert!(anchors.iter().any(|a| a.kind == PointKind::FaceCenter
                && nator_geometry::norm(nator_geometry::sub(a.point, center)) < 1e-9));
        }
        Ok(())
    }

    #[test]
    fn rectangle_with_an_off_center_hole_keeps_its_geometric_center()
    -> Result<(), Box<dyn std::error::Error>> {
        let solid = Solid::cuboid([0.; 3], [20., 10., 4.])?.cut(&Solid::cylinder(
            [4., 5., -1.],
            1.,
            6.,
        )?)?;
        let anchors = anchors(solid)?;
        for point in [[10., 5., 0.], [10., 5., 4.]] {
            assert!(anchors.iter().any(|a| a.kind == PointKind::FaceCenter
                && nator_geometry::norm(nator_geometry::sub(a.point, point)) < 1e-9));
        }
        Ok(())
    }

    #[test]
    fn arc_midpoint_stays_on_the_curve_instead_of_its_chord()
    -> Result<(), Box<dyn std::error::Error>> {
        use nator_sketch::{Curve, Plane, Sketch, areas};
        let mut sketch = Sketch::new("Segmento circular", Plane::xy());
        let center = sketch.add_point([0., 0.])?;
        let start = sketch.add_point([10., 0.])?;
        let end = sketch.add_point([0., 10.])?;
        sketch.add_curve(
            Curve::Arc {
                center,
                start,
                end,
                clockwise: false,
            },
            false,
        )?;
        sketch.add_curve(
            Curve::Line {
                start: end,
                end: start,
            },
            false,
        )?;
        let region = areas::regions(&sketch)?
            .into_iter()
            .next()
            .ok_or("Região ausente")?;
        let anchors = anchors(Solid::extrude(&sketch, &region, 0., 2., "arc")?)?;
        let radial = 10. / 2_f64.sqrt();
        for point in [[radial, radial, 2.], [5., 5., 2.]] {
            assert!(anchors.iter().any(|a| a.kind == PointKind::EdgeMidpoint
                && nator_geometry::norm(nator_geometry::sub(a.point, point)) < 1e-8));
        }
        for point in [[0., 0., 0.], [0., 0., 2.]] {
            assert!(anchors.iter().any(|a| a.kind == PointKind::ArcCenter
                && nator_geometry::norm(nator_geometry::sub(a.point, point)) < 1e-8));
        }
        Ok(())
    }
}

#[cfg(test)]
mod mesh_tests {
    use super::*;
    use nator_document::{Document, MeshAsset};
    use nator_geometry::Mesh;
    use std::sync::Arc;

    #[test]
    fn mesh_midpoints_exclude_coplanar_triangulation_diagonals()
    -> Result<(), Box<dyn std::error::Error>> {
        let mesh = Mesh::new(
            vec![[0., 0., 0.], [20., 0., 0.], [20., 10., 0.], [0., 10., 0.]],
            vec![[0, 1, 2], [0, 2, 3]],
        )?;
        let mut doc = Document::new("Plano triangulado")?;
        let (_, id) = doc.add_mesh(None, "Plano".into(), Arc::new(MeshAsset::new(mesh)))?;
        let anchors = body_anchors(doc.body(id).ok_or("Corpo ausente")?)?;
        let midpoints = anchors
            .iter()
            .filter(|a| a.kind == PointKind::EdgeMidpoint)
            .map(|a| a.point)
            .collect::<Vec<_>>();
        assert_eq!(midpoints.len(), 4);
        for point in [[10., 0., 0.], [20., 5., 0.], [10., 10., 0.], [0., 5., 0.]] {
            assert!(midpoints.contains(&point));
        }
        assert!(
            !midpoints.contains(&[10., 5., 0.]),
            "A diagonal do retângulo não é uma aresta geométrica"
        );
        assert_eq!(
            anchors
                .iter()
                .filter(|a| a.kind == PointKind::FaceCenter)
                .count(),
            1
        );
        assert!(
            anchors
                .iter()
                .any(|a| a.kind == PointKind::FaceCenter && a.point == [10., 5., 0.])
        );
        Ok(())
    }
}
