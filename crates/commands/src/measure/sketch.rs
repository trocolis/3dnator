//! Medidas analíticas dos elementos de rascunhos, inclusive em planos transformados.
use super::{Target, error};
use nator_document::DocumentError;
use nator_geometry::Placement;
use nator_sketch::{Curve, Sketch, distance, profiles};

pub fn sketch_entity(
    sketch: &Sketch,
    entity: u64,
    placement: Placement,
) -> Result<Target, DocumentError> {
    let entity = sketch.entity(entity).map_err(error)?;
    if entity.infinite {
        return Err(error("Uma reta infinita não tem comprimento finito."));
    }
    let world = |p| placement.apply(sketch.plane.world(p));
    let (length_mm, circle) = match entity.curve {
        Curve::Line { start, end } => {
            let a = sketch.point(start).map_err(error)?;
            let b = sketch.point(end).map_err(error)?;
            (distance(a, b), None)
        }
        Curve::Circle { center, radius } => (
            std::f64::consts::TAU * radius,
            Some((world(sketch.point(center).map_err(error)?), radius)),
        ),
        Curve::Arc {
            center,
            start,
            end,
            clockwise,
        } => {
            let c = sketch.point(center).map_err(error)?;
            let a = nator_sketch::sub(sketch.point(start).map_err(error)?, c);
            let b = nator_sketch::sub(sketch.point(end).map_err(error)?, c);
            let first = a[1].atan2(a[0]);
            let last = b[1].atan2(b[0]);
            let sweep = if clockwise {
                first - last
            } else {
                last - first
            };
            let radius = a[0].hypot(a[1]);
            (
                sweep.rem_euclid(std::f64::consts::TAU) * radius,
                Some((world(c), radius)),
            )
        }
    };
    let points = profiles::segment(sketch, entity)
        .map_err(error)?
        .sample(256)
        .into_iter()
        .map(world)
        .collect();
    Ok(Target::Edge {
        points,
        length_mm,
        circle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sketch_lengths_are_analytic_and_follow_plane_and_occurrence()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut sketch = Sketch::new(
            "Medidas",
            nator_sketch::Plane {
                origin: [10., 20., 30.],
                ..Default::default()
            },
        );
        let line = sketch.add_line([0., 0.], [3., 4.], false)?;
        let circle = sketch.add_circle([8., 8.], 2., false)?;
        let center = sketch.add_point([0., 0.])?;
        let start = sketch.add_point([2., 0.])?;
        let end = sketch.add_point([0., 2.])?;
        let arc = sketch.add_curve(
            Curve::Arc {
                center,
                start,
                end,
                clockwise: false,
            },
            false,
        )?;
        let pose = Placement::new([100., 0., 0.], [0., 0., std::f64::consts::FRAC_PI_2])?;
        for (id, expected) in [
            (line, 5.),
            (circle, 4. * std::f64::consts::PI),
            (arc, std::f64::consts::PI),
        ] {
            let Target::Edge {
                points, length_mm, ..
            } = sketch_entity(&sketch, id, pose)?
            else {
                return Err("Alvo incorreto".into());
            };
            assert!((length_mm - expected).abs() < 1e-12);
            assert!(points.iter().all(|p| (p[2] - 30.).abs() < 1e-12));
        }
        let Target::Edge { points, .. } = sketch_entity(&sketch, line, pose)? else {
            return Err("Reta ausente".into());
        };
        assert!((points[0][0] - 80.).abs() < 1e-12 && (points[0][1] - 10.).abs() < 1e-12);
        Ok(())
    }
}
