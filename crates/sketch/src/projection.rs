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

fn point(sketch: &mut Sketch, position: Point2, face: bool) -> Result<u64> {
    if let Some(point) = sketch
        .points
        .iter_mut()
        .find(|p| p.projected && distance(p.position, position) < EPSILON)
    {
        point.face |= face;
        return Ok(point.id);
    }
    let id = sketch.add_point(position)?;
    if let Some(point) = sketch.points.iter_mut().find(|p| p.id == id) {
        point.projected = true;
        point.face = face;
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
    insert_marked(sketch, curves, vertices, false)
}

/// Linhas da face onde o rascunho está: fixas, em amarelo, sem graus de liberdade.
/// Bordas compartilhadas por duas faces do mesmo plano entram uma vez só.
pub fn insert_face(sketch: &mut Sketch, curves: &[Segment]) -> Result<bool> {
    insert_marked(sketch, curves, &[], true)
}

fn insert_marked(
    sketch: &mut Sketch,
    curves: &[Segment],
    vertices: &[Point2],
    face: bool,
) -> Result<bool> {
    let mut next = sketch.clone();
    let before = (next.points.len(), next.entities.len());
    for vertex in vertices {
        point(&mut next, *vertex, face)?;
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
                    start: point(&mut next, a, face)?,
                    end: point(&mut next, b, face)?,
                }
            }
            Segment::Circle { center, radius } => Curve::Circle {
                center: point(&mut next, center, face)?,
                radius,
            },
            Segment::Arc {
                center,
                a,
                b,
                clockwise,
            } => Curve::Arc {
                center: point(&mut next, center, face)?,
                start: point(&mut next, a, face)?,
                end: point(&mut next, b, face)?,
                clockwise,
            },
        };
        let id = next.add_curve(shape, false)?;
        if let Some(entity) = next.entities.iter_mut().find(|e| e.id == id) {
            entity.projected = true;
            entity.face = face;
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

#[cfg(test)]
mod face_lines {
    use super::*;
    use crate::{edit, junctions, movement, solver, tools};

    /// Quadrado da face e um traço desenhado sobre ele.
    fn face_with_a_stroke() -> Result<(Sketch, u64)> {
        let mut sketch = Sketch::new("Face", Plane::xy());
        let square = [[0., 0.], [20., 0.], [20., 20.], [0., 20.]];
        let curves = (0..4)
            .map(|i| Segment::Line {
                a: square[i],
                b: square[(i + 1) % 4],
            })
            .collect::<Vec<_>>();
        assert!(insert_face(&mut sketch, &curves)?);
        let stroke = sketch.add_line([8., 0.], [8., 20.], false)?;
        Ok((sketch, stroke))
    }

    #[test]
    fn face_lines_are_fixed_yellow_references_that_delimit_areas() -> Result<()> {
        let (sketch, _) = face_with_a_stroke()?;
        let face = sketch
            .entities
            .iter()
            .filter(|e| e.face)
            .collect::<Vec<_>>();
        assert_eq!(face.len(), 4);
        assert!(face.iter().all(|e| e.projected && !e.construction));
        assert!(sketch.points.iter().filter(|p| p.face).all(|p| p.projected));
        // O traço sobre a face já separa duas áreas, sem refazer o contorno.
        assert_eq!(crate::areas::regions(&sketch)?.len(), 2);
        // Só a face: nada a resolver, zero graus de liberdade.
        let mut only = Sketch::new("Só a face", Plane::xy());
        insert_face(
            &mut only,
            &[Segment::Circle {
                center: [0., 0.],
                radius: 5.,
            }],
        )?;
        assert!(matches!(
            solver::solve(&only)?.status,
            solver::Status::FullyConstrained
        ));
        // Repetir não duplica as linhas.
        let mut again = sketch.clone();
        let square = [[0., 0.], [20., 0.]];
        assert!(!insert_face(
            &mut again,
            &[Segment::Line {
                a: square[0],
                b: square[1],
            }]
        )?);
        Ok(())
    }

    #[test]
    fn face_lines_cannot_be_trimmed_deleted_moved_or_rounded() -> Result<()> {
        let (sketch, stroke) = face_with_a_stroke()?;
        let face = sketch
            .entities
            .iter()
            .find(|e| e.face)
            .map(|e| e.id)
            .ok_or_else(|| SketchError("Linha da face ausente.".into()))?;
        let options = edit::Options {
            value_mm: 2.,
            reference: false,
        };
        for action in [
            edit::Action::Delete,
            edit::Action::Move,
            edit::Action::Construction,
        ] {
            let mut copy = sketch.clone();
            assert!(edit::apply(&mut copy, action, &[face], options).is_err());
            assert_eq!(copy.entities, sketch.entities);
        }
        let mut copy = sketch.clone();
        assert!(tools::trim(&mut copy, face, [4., 0.]).is_err());
        let corner = sketch
            .entities
            .iter()
            .filter(|e| e.face)
            .take(2)
            .map(|e| e.id)
            .collect::<Vec<_>>();
        assert!(edit::apply(&mut sketch.clone(), edit::Action::Fillet, &corner, options).is_err());
        let corner_point = sketch
            .points
            .iter()
            .find(|p| p.face)
            .map(|p| p.id)
            .ok_or_else(|| SketchError("Vértice da face ausente.".into()))?;
        assert!(movement::dragged(&sketch, &[corner_point], &[], [1., 1.]).is_err());
        let mut copy = sketch.clone();
        assert!(!junctions::dissolve(&mut copy, [0., 0.])?);
        // O traço desenhado continua livre para editar.
        let mut copy = sketch.clone();
        assert!(edit::apply(&mut copy, edit::Action::Delete, &[stroke], options).is_ok());
        Ok(())
    }

    /// As linhas da face servem de apoio às restrições: o traço se ajusta e elas não mudam.
    #[test]
    fn face_lines_anchor_constraints_and_dimensions() -> Result<()> {
        let (mut sketch, _) = face_with_a_stroke()?;
        // Lado inferior (y = 0) e esquerdo (x = 0) do quadrado.
        let side = |s: &Sketch, horizontal: bool| -> Result<u64> {
            for e in s.entities.iter().filter(|e| e.face) {
                let (a, b) = s.line(e.id)?;
                let axis = usize::from(horizontal);
                if a[axis].abs() < 1e-9 && b[axis].abs() < 1e-9 {
                    return Ok(e.id);
                }
            }
            Err(SketchError("Lado ausente.".into()))
        };
        let (bottom, left) = (side(&sketch, true)?, side(&sketch, false)?);
        let before = sketch.line(bottom)?;
        let line = sketch.add_line([3., 2.], [15., 6.], false)?;
        let start = sketch.entity(line)?.point_ids()[0];
        sketch.add_constraint(ConstraintKind::Parallel { a: line, b: bottom }, false)?;
        sketch.add_constraint(
            ConstraintKind::PointOnLine {
                point: start,
                line: left,
            },
            false,
        )?;
        sketch.add_constraint(
            ConstraintKind::PointLineDistance {
                point: start,
                line: bottom,
                value: 5.,
            },
            false,
        )?;
        let solved = solver::solve(&sketch)?;
        assert_ne!(solved.status, solver::Status::Conflict);
        let (a, b) = solved.sketch.line(line)?;
        assert!(a[0].abs() < 1e-6 && (a[1] - 5.).abs() < 1e-6, "{a:?}");
        assert!((a[1] - b[1]).abs() < 1e-6);
        assert_eq!(solved.sketch.line(bottom)?, before);
        Ok(())
    }
}
