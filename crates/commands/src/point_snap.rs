//! Apoios para movimentação ponto a ponto, consultados sem alterar o documento.
//! O núcleo fornece centros exatos no plano de cada borda circular. Não aproximamos
//! círculos por caixas envolventes nem confundimos o centro do furo com o centro de massa.
use nator_document::{Body, DocumentError};
use nator_geometry::Point3;
use nator_kernel::CircleAnchor;

#[derive(Debug, Clone)]
pub struct PointAnchor {
    /// Coordenadas locais do corpo; a interface compõe corpo e ocorrência na projeção.
    pub point: Point3,
    pub circle: Option<CircleAnchor>,
}

/// Consulta potencialmente demorada: chamar fora da thread da interface e reaproveitar
/// o resultado enquanto a geometria não mudar. Malhas oferecem seus vértices, sem
/// inventar centros analíticos para contornos apenas aproximadamente circulares.
pub fn body_anchors(body: &Body) -> Result<Vec<PointAnchor>, DocumentError> {
    #[cfg(feature = "occt")]
    if let Some(bytes) = &body.brep {
        let query = || -> Result<Vec<PointAnchor>, nator_kernel::KernelError> {
            let solid = nator_kernel::Solid::from_brep(bytes, None)?;
            let mut vertices = Vec::new();
            let mut anchors = Vec::new();
            for edge in 0..solid.edge_count()? {
                let supports = solid.edge_anchors(edge)?;
                if let Some(circle) = supports.circle {
                    anchors.push(PointAnchor {
                        point: circle.center,
                        circle: Some(circle),
                    });
                }
                // Um círculo fechado tem uma costura topológica arbitrária, que
                // não deve competir com o centro como se fosse um canto da peça.
                if !supports.seam && !supports.circle.is_some_and(|c| c.closed) {
                    vertices.extend([supports.start, supports.end]);
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
                circle: None,
            }));
            Ok(anchors)
        };
        return query().map_err(|e| DocumentError(e.to_string()));
    }
    Ok(body
        .geometry
        .mesh()
        .vertices()
        .iter()
        .map(|point| PointAnchor {
            point: *point,
            circle: None,
        })
        .collect())
}

#[cfg(all(test, feature = "occt"))]
mod tests {
    use super::*;
    use nator_document::{Document, MeshAsset};
    use nator_kernel::Solid;
    use std::sync::Arc;

    #[test]
    fn washer_centers_keep_both_surface_heights_without_arbitrary_seam_vertices()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut solid = Solid::cylinder([0., 0., 3.], 20., 5.)?.cut(&Solid::cylinder(
            [0., 0., 2.],
            10.,
            7.,
        )?)?;
        let (mesh, faces) = solid.tessellate_with_faces(0.5, 0.5)?;
        let mut doc = Document::new("Arruela")?;
        let (_, id) = doc.add_mesh(
            None,
            "Arruela".into(),
            Arc::new(MeshAsset::with_faces(mesh, faces)?),
        )?;
        doc.body_mut(id)?.brep = Some(Arc::new(solid.to_brep()?));
        let anchors = body_anchors(doc.body(id).ok_or("Corpo ausente")?)?;
        assert_eq!(
            anchors.len(),
            4,
            "Só as quatro bordas circulares, sem costuras"
        );
        for (radius, height) in [(10., 3.), (20., 3.), (10., 8.), (20., 8.)] {
            assert!(anchors.iter().any(|a| a.circle.is_some_and(|c| {
                c.closed
                    && (c.radius_mm - radius).abs() < 1e-9
                    && nator_geometry::norm(nator_geometry::sub(c.center, [0., 0., height])) < 1e-9
                    && (nator_geometry::norm(c.normal) - 1.).abs() < 1e-9
            })));
        }
        Ok(())
    }
}
