//! Adaptação da tesselação de exibição à malha validada do aplicativo.
//! Superfícies CAD válidas podem produzir triângulos colapsados nas singularidades
//! e costuras. Só esses elementos de área exatamente zero são descartados: não há
//! tolerância que apague detalhes pequenos, nem alteração da geometria B-rep.
use crate::KernelError;
use nator_geometry::{Mesh, cross, norm, sub};

pub(crate) fn display_mesh(
    positions: &[f64],
    faces: Vec<i32>,
) -> Result<(Mesh, Vec<u32>), KernelError> {
    if positions.is_empty()
        || positions.len() > 18_000_000
        || !positions.len().is_multiple_of(9)
        || faces.len() != positions.len() / 9
    {
        return Err(KernelError(
            "Tesselação nativa com tamanho ou faces inconsistentes.".into(),
        ));
    }
    let mut vertices = Vec::with_capacity(positions.len() / 3);
    let mut triangles = Vec::with_capacity(faces.len());
    let mut kept_faces = Vec::with_capacity(faces.len());
    for (coordinates, face) in positions.as_chunks::<9>().0.iter().zip(faces) {
        let face =
            u32::try_from(face).map_err(|_| KernelError("Face de triângulo inválida.".into()))?;
        // Coordenadas inválidas são erros, mesmo que o triângulo também esteja
        // colapsado. Limpar a apresentação não pode mascarar falhas numéricas.
        if coordinates.iter().any(|value| !value.is_finite()) {
            return Err(KernelError(
                "Coordenada não finita na tesselação do sólido.".into(),
            ));
        }
        let points = [
            [coordinates[0], coordinates[1], coordinates[2]],
            [coordinates[3], coordinates[4], coordinates[5]],
            [coordinates[6], coordinates[7], coordinates[8]],
        ];
        // Norma do produto vetorial: dobro da área, em mm².
        let double_area = norm(cross(sub(points[1], points[0]), sub(points[2], points[0])));
        if !double_area.is_finite() {
            return Err(KernelError(
                "Triângulo da tesselação fora do intervalo numérico.".into(),
            ));
        }
        if double_area == 0. {
            continue;
        }
        let first = u32::try_from(vertices.len())
            .map_err(|_| KernelError("Tesselação com vértices demais.".into()))?;
        vertices.extend(points);
        triangles.push([first, first + 1, first + 2]);
        // A face acompanha o triângulo aceito, preservando a seleção e os nomes
        // estáveis mesmo quando um elemento intermediário foi descartado.
        kept_faces.push(face);
    }
    if triangles.is_empty() {
        return Err(KernelError(
            "A tesselação não contém triângulos de área positiva.".into(),
        ));
    }
    let mesh = Mesh::new(vertices, triangles)
        .map_err(|e| KernelError(e.to_string()))?
        .welded();
    Ok((mesh, kept_faces))
}

#[cfg(test)]
mod tests {
    use super::*;
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn collapsed_triangles_are_removed_together_with_their_face_indices() -> Result<()> {
        let triangles = [
            [[0.; 3], [1., 0., 0.], [0.; 3]],
            [[0.; 3], [2., 0., 0.], [0., 1., 0.]],
            [[1.; 3], [2.; 3], [3.; 3]],
            [[0., 0., 1.], [0., 1., 1.], [2., 0., 1.]],
            [[2.; 3]; 3],
        ];
        let positions = triangles
            .into_iter()
            .flatten()
            .flatten()
            .collect::<Vec<_>>();
        let (mesh, faces) = display_mesh(&positions, vec![40, 7, 99, 3, 20])?;
        assert_eq!(faces, [7, 3]);
        assert_eq!(mesh.triangles().len(), 2);
        for (actual, expected) in mesh.triangles().iter().zip([triangles[1], triangles[3]]) {
            assert_eq!(mesh.triangle_points(*actual), expected);
        }
        Ok(())
    }

    #[test]
    fn small_positive_triangles_are_preserved_without_an_area_tolerance() -> Result<()> {
        let (mesh, faces) = display_mesh(&[0., 0., 0., 1e-100, 0., 0., 0., 1e-100, 0.], vec![5])?;
        assert_eq!(mesh.triangles().len(), 1);
        assert_eq!(faces, [5]);
        assert_eq!(mesh.vertices()[1], [1e-100, 0., 0.]);
        Ok(())
    }

    #[test]
    fn invalid_numbers_indices_and_empty_results_remain_errors() {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(display_mesh(&[invalid, 0., 0., 1., 0., 0., 0., 1., 0.], vec![0]).is_err());
        }
        assert!(
            display_mesh(
                &[f64::MAX, 0., 0., -f64::MAX, 0., 0., 0., f64::MAX, 0.],
                vec![0]
            )
            .is_err()
        );
        assert!(display_mesh(&[0.; 9], vec![0]).is_err());
        assert!(display_mesh(&[0.; 9], vec![-1]).is_err());
        assert!(display_mesh(&[0.; 8], vec![0]).is_err());
        assert!(display_mesh(&[0.; 9], vec![0, 1]).is_err());
    }
}
