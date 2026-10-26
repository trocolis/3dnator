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
    fn from_matrix(translation: Point3, matrix: [[f64; 3]; 3]) -> Result<Self, GeometryError> {
        let y = (-matrix[2][0]).clamp(-1., 1.).asin();
        let (x, z) = if y.cos().abs() > 1e-10 {
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
