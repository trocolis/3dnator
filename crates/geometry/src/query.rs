//! Consultas sobre malhas: medições e seleção não alteram a geometria precisa.
use crate::*;
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Point3,
    pub direction: Point3,
}
#[derive(Debug, Clone, Copy)]
pub struct Hit {
    pub point: Point3,
    pub normal: Point3,
    pub triangle: usize,
    pub distance: f64,
}
impl Mesh {
    pub fn ray_hit(&self, ray: Ray) -> Option<Hit> {
        let mut hit = None;
        let mut nearest = f64::INFINITY;
        for (index, t) in self.triangles.iter().enumerate() {
            let [a, b, c] = self.triangle_points(*t);
            let e1 = sub(b, a);
            let e2 = sub(c, a);
            let p = cross(ray.direction, e2);
            let determinant = dot(e1, p);
            if determinant.abs() < 1e-12 * norm(e1) * norm(e2) {
                continue;
            }
            let inverse = 1. / determinant;
            let s = sub(ray.origin, a);
            let u = dot(s, p) * inverse;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let q = cross(s, e1);
            let v = dot(ray.direction, q) * inverse;
            if v < 0. || u + v > 1. {
                continue;
            }
            let d = dot(e2, q) * inverse;
            if d < 0. || d >= nearest {
                continue;
            }
            nearest = d;
            let n = cross(e1, e2);
            hit = Some(Hit {
                point: std::array::from_fn(|i| ray.origin[i] + d * ray.direction[i]),
                normal: n.map(|v| v / norm(n)),
                triangle: index,
                distance: d,
            });
        }
        hit
    }
}
/// Ângulo menor entre vetores, em radianos.
pub fn angle_between(a: Point3, b: Point3) -> Result<f64, GeometryError> {
    let d = norm(a) * norm(b);
    if !d.is_finite() || d < 1e-12 {
        return Err(GeometryError("Ângulo exige dois vetores não nulos.".into()));
    }
    Ok((dot(a, b) / d).clamp(-1., 1.).acos())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ray_hits_nearest_surface_and_volume_centroid_is_finite() -> Result<(), GeometryError> {
        let mesh = reference_bracket()?;
        let hit = mesh
            .ray_hit(Ray {
                origin: [30., 10., 100.],
                direction: [0., 0., -1.],
            })
            .ok_or_else(|| GeometryError("Sem interseção".into()))?;
        assert!((hit.point[2] - 6.).abs() < 1e-9);
        let (c, v) = mesh.mass_center()?;
        assert!((v - 10080.).abs() < 1e-9);
        assert!(c.iter().all(|v| v.is_finite()));
        Ok(())
    }
}
