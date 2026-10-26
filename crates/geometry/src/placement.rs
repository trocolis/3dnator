//! Transformação rígida local. A ordem de rotação é X, depois Y, depois Z.
use crate::{GeometryError, Point3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    translation: Point3,
    angles: Point3,
    matrix: [[f64; 3]; 3],
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            translation: [0.; 3],
            angles: [0.; 3],
            matrix: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        }
    }
}

impl Placement {
    /// Matriz afim 3×4 em linhas para intercâmbio. Só aceita rotações próprias:
    /// escala, cisalhamento e reflexão precisam ser aplicados à geometria explicitamente.
    pub fn from_affine_rows(rows: [f64; 12]) -> Result<Self, GeometryError> {
        let matrix: [[f64; 3]; 3] =
            std::array::from_fn(|i| std::array::from_fn(|j| rows[i * 4 + j]));
        let valid = rows.iter().all(|v| v.is_finite())
            && (0..3).all(|i| {
                (0..3).all(|j| {
                    (crate::dot(matrix[i], matrix[j]) - if i == j { 1. } else { 0. }).abs() < 1e-8
                })
            })
            && (crate::dot(crate::cross(matrix[0], matrix[1]), matrix[2]) - 1.).abs() < 1e-8;
        if !valid {
            return Err(GeometryError(
                "A transformação importada não é uma posição rígida válida.".into(),
            ));
        }
        Self::from_matrix([rows[3], rows[7], rows[11]], matrix)
    }
    pub fn affine_rows(self) -> [f64; 12] {
        std::array::from_fn(|i| {
            if i % 4 == 3 {
                self.translation[i / 4]
            } else {
                self.matrix[i / 4][i % 4]
            }
        })
    }
    pub fn new(translation_mm: Point3, rotation_rad: Point3) -> Result<Self, GeometryError> {
        if translation_mm
            .into_iter()
            .chain(rotation_rad)
            .any(|n| !n.is_finite())
        {
            return Err(GeometryError(
                "Posição e rotação devem conter números finitos.".into(),
            ));
        }
        let [(sx, cx), (sy, cy), (sz, cz)] = rotation_rad.map(f64::sin_cos);
        Ok(Self {
            translation: translation_mm,
            angles: rotation_rad,
            matrix: [
                [cz * cy, cz * sy * sx - sz * cx, cz * sy * cx + sz * sx],
                [sz * cy, sz * sy * sx + cz * cx, sz * sy * cx - cz * sx],
                [-sy, cy * sx, cy * cx],
            ],
        })
    }
    /// Composição de origens: `parent.compose(child)` transforma do filho ao mundo.
    pub fn inverse(self) -> Result<Self, GeometryError> {
        let matrix = std::array::from_fn(|i| std::array::from_fn(|j| self.matrix[j][i]));
        let translation = std::array::from_fn(|i| -crate::dot(matrix[i], self.translation));
        Self::from_matrix(translation, matrix)
    }
    pub fn compose(self, child: Self) -> Result<Self, GeometryError> {
        let matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| (0..3).map(|k| self.matrix[i][k] * child.matrix[k][j]).sum())
        });
        Self::from_matrix(self.apply(child.translation), matrix)
    }

    /// Espelha o referencial no plano do pai, mantendo uma posição rígida destra.
    ///
    /// Uma reflexão isolada tem determinante -1 e não cabe em `Placement`. A imagem
    /// da geometria deve ser refletida também no plano local X = 0: se M é a reflexão
    /// no pai e S a reflexão local, esta posição representa M · self · S. As duas
    /// inversões se cancelam, preservando rotações próprias e o compartilhamento de
    /// uma mesma definição espelhada entre componentes com orientações diferentes.
    pub fn mirrored_frame(self, origin: Point3, normal: Point3) -> Result<Self, GeometryError> {
        let length = crate::norm(normal);
        if !length.is_finite() || length < 1e-12 || origin.iter().any(|v| !v.is_finite()) {
            return Err(GeometryError("Plano de espelhamento inválido.".into()));
        }
        let n = normal.map(|v| v / length);
        let distance = crate::dot(crate::sub(self.translation, origin), n);
        let translation = std::array::from_fn(|i| self.translation[i] - 2. * distance * n[i]);
        let matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let projection: f64 = (0..3).map(|k| n[k] * self.matrix[k][j]).sum();
                let reflected = self.matrix[i][j] - 2. * n[i] * projection;
                if j == 0 { -reflected } else { reflected }
            })
        });
        Self::from_matrix(translation, matrix)
    }
    fn from_matrix(translation: Point3, matrix: [[f64; 3]; 3]) -> Result<Self, GeometryError> {
        // atan2 conserva a distância à singularidade usando também a projeção do eixo X.
        // asin perde essa informação perto de ±90°: arredondamentos em R[2][0] podem
        // fazer uma rotação singular entrar no ramo regular com ângulos inconsistentes.
        let horizontal = matrix[0][0].hypot(matrix[1][0]);
        let y = (-matrix[2][0]).atan2(horizontal);
        let (x, z) = if horizontal > 1e-10 {
            (
                matrix[2][1].atan2(matrix[2][2]),
                matrix[1][0].atan2(matrix[0][0]),
            )
        } else {
            (0., (-matrix[0][1]).atan2(matrix[1][1]))
        };
        Self::new(translation, [x, y, z])
    }
    /// Rotação rígida pelo eixo escolhido; a origem se desloca ao redor do pivô.
    pub fn rotated_about(
        self,
        pivot: Point3,
        axis: Point3,
        angle: f64,
    ) -> Result<Self, GeometryError> {
        let length = crate::norm(axis);
        if !length.is_finite() || length < 1e-12 || !angle.is_finite() {
            return Err(GeometryError("Eixo ou ângulo de rotação inválido.".into()));
        }
        let a = axis.map(|v| v / length);
        let (s, c) = angle.sin_cos();
        let skew = [[0., -a[2], a[1]], [a[2], 0., -a[0]], [-a[1], a[0], 0.]];
        let matrix = std::array::from_fn(|i| {
            std::array::from_fn(
                |j| if i == j { c } else { 0. } + (1. - c) * a[i] * a[j] + s * skew[i][j],
            )
        });
        let delta = crate::sub(self.translation, pivot);
        let translation = std::array::from_fn(|i| pivot[i] + crate::dot(matrix[i], delta));
        let rotation = Self::from_matrix([0.; 3], matrix)?;
        let mut next = rotation.compose(self)?;
        next.translation = translation;
        Ok(next)
    }
    pub fn translated(self, delta: Point3) -> Result<Self, GeometryError> {
        Self::new(
            std::array::from_fn(|i| self.translation[i] + delta[i]),
            self.angles,
        )
    }
    pub fn translation_mm(self) -> Point3 {
        self.translation
    }
    pub fn rotation_rad(self) -> Point3 {
        self.angles
    }
    pub fn apply(self, point: Point3) -> Point3 {
        std::array::from_fn(|i| self.translation[i] + crate::dot(self.matrix[i], point))
    }
    pub fn inverse_apply(self, point: Point3) -> Point3 {
        let p = crate::sub(point, self.translation);
        std::array::from_fn(|i| {
            self.matrix[0][i] * p[0] + self.matrix[1][i] * p[1] + self.matrix[2][i] * p[2]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotates_locally_before_translating() -> Result<(), GeometryError> {
        let pose = Placement::new([10., 20., 30.], [0., 0., std::f64::consts::FRAC_PI_2])?;
        let point = pose.apply([2., 0., 0.]);
        for (actual, expected) in point.into_iter().zip([10., 22., 30.]) {
            assert!((actual - expected).abs() < 1e-10);
        }
        assert!(Placement::new([f64::NAN, 0., 0.], [0.; 3]).is_err());
        Ok(())
    }

    #[test]
    fn mirrored_frames_compensate_local_reflection_with_a_proper_rotation()
    -> Result<(), GeometryError> {
        let origin = [12., -5., 7.];
        let normal = [2., -3., 4.];
        let length = crate::norm(normal);
        let n = normal.map(|v| v / length);
        for angles in [
            [0.2, -0.7, 1.1],
            [0.3, std::f64::consts::FRAC_PI_2, -0.5],
            [0.3, -std::f64::consts::FRAC_PI_2, -0.5],
        ] {
            let source = Placement::new([80., -30., 45.], angles)?;
            let mirrored = source.mirrored_frame(origin, normal)?;
            for point in [[0.; 3], [2., 7., -4.], [-20., 3., 40.]] {
                let world = source.apply(point);
                let distance = crate::dot(crate::sub(world, origin), n);
                let expected: Point3 = std::array::from_fn(|i| world[i] - 2. * distance * n[i]);
                let actual = mirrored.apply([-point[0], point[1], point[2]]);
                assert!(crate::norm(crate::sub(expected, actual)) < 1e-8);
            }
            let x = crate::sub(mirrored.apply([1., 0., 0.]), mirrored.translation_mm());
            let y = crate::sub(mirrored.apply([0., 1., 0.]), mirrored.translation_mm());
            let z = crate::sub(mirrored.apply([0., 0., 1.]), mirrored.translation_mm());
            assert!((crate::dot(crate::cross(x, y), z) - 1.).abs() < 1e-12);
            // Espelhar novamente pelo mesmo plano desfaz a troca do referencial.
            let restored = mirrored.mirrored_frame(origin, normal)?;
            assert!(
                crate::norm(crate::sub(
                    source.apply([3., 4., 5.]),
                    restored.apply([3., 4., 5.])
                )) < 1e-8
            );
        }
        for invalid in [[0.; 3], [f64::NAN, 0., 0.], [f64::MAX; 3]] {
            assert!(
                Placement::default()
                    .mirrored_frame(origin, invalid)
                    .is_err()
            );
        }
        assert!(
            Placement::default()
                .mirrored_frame([f64::INFINITY; 3], normal)
                .is_err()
        );
        Ok(())
    }
}

#[cfg(test)]
mod pivot_tests {
    use super::*;
    #[test]
    fn rotation_preserves_pivot_and_inverse() -> Result<(), GeometryError> {
        let p = Placement::new([10., 0., 0.], [0.2, 0.3, 0.4])?;
        let next = p.rotated_about([0.; 3], [0., 0., 1.], std::f64::consts::FRAC_PI_2)?;
        let t = next.translation_mm();
        assert!(t[0].abs() < 1e-9);
        assert!((t[1] - 10.).abs() < 1e-9);
        let local = [2., 3., 4.];
        let restored = next.inverse_apply(next.apply(local));
        for i in 0..3 {
            assert!((restored[i] - local[i]).abs() < 1e-9);
        }
        let combined = p.compose(next)?;
        for (a, b) in combined
            .apply(local)
            .into_iter()
            .zip(p.apply(next.apply(local)))
        {
            assert!((a - b).abs() < 1e-9);
        }
        Ok(())
    }
}
