//! Materialização de referências projetadas. A captura é um retrato da geometria
//! de origem; não guarda índices CAD nem depende da vida do componente de origem.
//! Pontos e raios recebem restrições explícitas para não se moverem ao cotar.
use crate::{profiles::Segment, *};

const EPSILON: f64 = 1e-8;

/// Distância analítica para captura, sem amostrar novamente círculos e arcos a
/// cada movimento do cursor. Também respeita os extremos finitos dos arcos.
pub fn distance_to(curve: &Segment, cursor: Point2) -> f64 {
    let parameter = crate::curve_geometry::closest_parameter(curve, cursor);
    distance(crate::curve_geometry::point(curve, parameter), cursor)
}

fn point(sketch: &mut Sketch, position: Point2) -> Result<u64> {
    if let Some(point) = sketch
        .points
        .iter()
        .find(|p| p.projected && distance(p.position, position) < EPSILON)
    {
        return Ok(point.id);
    }
    let id = sketch.add_point(position)?;
    if let Some(point) = sketch.points.iter_mut().find(|p| p.id == id) {
        point.projected = true;
    }
    sketch.add_constraint(
        ConstraintKind::Fixed {
            point: id,
            position,
        },
        false,
    )?;
    Ok(id)
}

fn same(a: &Segment, b: &Segment) -> bool {
    let close = |a, b| distance(a, b) < EPSILON;
    match (a, b) {
        (Segment::Line { a, b }, Segment::Line { a: c, b: d }) => {
            (close(*a, *c) && close(*b, *d)) || (close(*a, *d) && close(*b, *c))
        }
        (
            Segment::Circle {
                center: a,
                radius: r,
            },
            Segment::Circle {
                center: b,
                radius: s,
            },
        ) => close(*a, *b) && (r - s).abs() < EPSILON,
        (
            Segment::Arc {
                center: a,
                a: a0,
                b: a1,
                clockwise: ac,
            },
            Segment::Arc {
                center: b,
                a: b0,
                b: b1,
                clockwise: bc,
            },
        ) => {
            close(*a, *b)
                && ((ac == bc && close(*a0, *b0) && close(*a1, *b1))
                    || (ac != bc && close(*a0, *b1) && close(*a1, *b0)))
        }
        _ => false,
    }
}

/// Insere toda a seleção ou não altera nada. Reprojetar uma referência existente
/// é inócuo; apoios coincidentes são compartilhados para fechar os contornos.
/// O chamador publica o resultado por comando, oferecendo desfazer/refazer.
pub fn insert(sketch: &mut Sketch, curves: &[Segment], vertices: &[Point2]) -> Result<bool> {
    let mut next = sketch.clone();
    let before = (next.points.len(), next.entities.len());
    for vertex in vertices {
        point(&mut next, *vertex)?;
    }
    for curve in curves {
        if next
            .entities
            .iter()
            .filter(|e| e.projected)
            .any(|e| profiles::segment(&next, e).is_ok_and(|existing| same(curve, &existing)))
        {
            continue;
        }
        let shape = match *curve {
            Segment::Line { a, b } => {
                if distance(a, b) < EPSILON {
                    continue;
                }
                Curve::Line {
                    start: point(&mut next, a)?,
                    end: point(&mut next, b)?,
                }
            }
            Segment::Circle { center, radius } => Curve::Circle {
                center: point(&mut next, center)?,
                radius,
            },
            Segment::Arc {
                center,
                a,
                b,
                clockwise,
            } => Curve::Arc {
                center: point(&mut next, center)?,
                start: point(&mut next, a)?,
                end: point(&mut next, b)?,
                clockwise,
            },
        };
        let id = next.add_curve(shape, false)?;
        if let Some(entity) = next.entities.iter_mut().find(|e| e.id == id) {
            entity.projected = true;
        }
        if let Segment::Circle { radius, .. } = *curve {
            next.add_constraint(
                ConstraintKind::Radius {
                    circle: id,
                    value: radius,
                },
                false,
            )?;
        }
    }
    next.validate()?;
    let changed = before != (next.points.len(), next.entities.len());
    if changed {
        *sketch = next;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projected_contours_are_closed_fixed_and_idempotent() -> Result<()> {
        let mut sketch = Sketch::new("Projetado", Plane::xy());
        let curves = vec![
            Segment::Line {
                a: [0., 0.],
                b: [10., 0.],
            },
            Segment::Line {
                a: [10., 0.],
                b: [10., 10.],
            },
            Segment::Line {
                a: [10., 10.],
                b: [0., 10.],
            },
            Segment::Line {
                a: [0., 10.],
                b: [0., 0.],
            },
            Segment::Circle {
                center: [5., 5.],
                radius: 2.,
            },
        ];
        assert!(insert(&mut sketch, &curves, &[[20., 20.]])?);
        assert_eq!(sketch.points.len(), 6);
        assert_eq!(areas::regions(&sketch)?.len(), 2);
        assert_eq!(
            solver::solve(&sketch)?.status,
            solver::Status::FullyConstrained
        );
        assert!(!insert(&mut sketch, &curves, &[[20., 20.]])?);
        assert!(!insert(&mut sketch, &[curves[0].reversed()], &[])?);
        assert!(sketch.points.iter().all(|p| p.projected));
        assert!(sketch.entities.iter().all(|e| e.projected));
        Ok(())
    }

    #[test]
    fn invalid_projection_does_not_leave_partial_geometry_or_identifiers() -> Result<()> {
        let mut sketch = Sketch::new("Atômico", Plane::xy());
        let before = sketch.next_id;
        assert!(
            insert(
                &mut sketch,
                &[Segment::Circle {
                    center: [1., 2.],
                    radius: -1.
                }],
                &[[4., 5.]]
            )
            .is_err()
        );
        assert!(sketch.points.is_empty());
        assert!(sketch.entities.is_empty());
        assert_eq!(sketch.next_id, before);
        Ok(())
    }

    #[test]
    fn dense_references_do_not_consume_free_variables_or_move_under_dimensions() -> Result<()> {
        let mut sketch = Sketch::new("Referências densas", Plane::xy());
        let curves = (0..150)
            .map(|i| Segment::Line {
                a: [i as f64, 0.],
                b: [i as f64 + 1., 0.],
            })
            .collect::<Vec<_>>();
        insert(&mut sketch, &curves, &[])?;
        let fixed = sketch.points[0].id;
        let moving = sketch.add_point([0., 10.])?;
        sketch.add_constraint(
            ConstraintKind::Distance {
                a: fixed,
                b: moving,
                value: 20.,
            },
            false,
        )?;
        let report = solver::solve_preserving(&sketch, &[moving], &[])?;
        assert_ne!(report.status, solver::Status::Conflict);
        assert_eq!(report.sketch.point(fixed)?, [0., 0.]);
        assert!(
            (distance(report.sketch.point(fixed)?, report.sketch.point(moving)?) - 20.).abs()
                < 1e-6
        );
        assert_eq!(
            report.status,
            solver::Status::UnderConstrained {
                degrees_of_freedom: 1
            }
        );
        assert!(!report.redundant);
        Ok(())
    }
}
