//! Posições geométricas dos indicadores, calculadas junto às referências do
//! rascunho. A interface escolhe ícones; nenhum símbolo participa do solver.
use crate::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Perpendicular,
    Tangent,
    Midpoint,
    Center,
}
#[derive(Debug, Clone, Copy)]
pub struct Indicator {
    pub point: Point2,
    pub kind: Kind,
}

fn push(result: &mut Vec<Indicator>, point: Point2, kind: Kind) {
    if !result
        .iter()
        .any(|i| i.kind == kind && distance(i.point, point) < 1e-6)
    {
        result.push(Indicator { point, kind });
    }
}

pub fn markers(sketch: &Sketch) -> Result<Vec<Indicator>> {
    let mut result = Vec::new();
    for entity in &sketch.entities {
        if matches!(entity.curve, Curve::Arc { .. })
            || (matches!(entity.curve, Curve::Circle { .. })
                && connected::center_is_linked(sketch, entity.id)?)
        {
            push(&mut result, sketch.radial(entity.id)?.0, Kind::Center);
        }
    }
    for constraint in &sketch.constraints {
        match constraint.kind {
            ConstraintKind::Midpoint { point, line } => {
                let (a, b) = sketch.line(line)?;
                let position = sketch.point(point)?;
                if distance(position, scale(add(a, b), 0.5)) < 1e-6 {
                    push(&mut result, position, Kind::Midpoint);
                }
            }
            ConstraintKind::PointOnCurve { point, curve }
                if matches!(sketch.entity(curve)?.curve, Curve::Arc { .. }) =>
            {
                let midpoint = crate::curve_geometry::point(
                    &profiles::segment(sketch, sketch.entity(curve)?)?,
                    0.5,
                );
                if distance(midpoint, sketch.point(point)?) < 1e-6 {
                    push(&mut result, midpoint, Kind::Midpoint);
                }
            }
            ConstraintKind::Perpendicular { a, b } if !constraint.reference => {
                intersection(sketch, a, b, &mut result)?;
            }
            ConstraintKind::Tangent { line, arc, point } if !constraint.reference => {
                let position = if matches!(sketch.entity(arc)?.curve, Curve::Circle { .. }) {
                    let center = sketch.radial(arc)?.0;
                    let (a, b) = sketch.line(line)?;
                    let d = sub(b, a);
                    add(a, scale(d, dot(sub(center, a), d) / dot(d, d)))
                } else {
                    sketch.point(point)?
                };
                push(&mut result, position, Kind::Tangent);
            }
            _ => {}
        }
    }
    // Retângulos usam Horizontal/Vertical por lado: o ângulo reto já está
    // garantido por essas duas regras, sem acrescentar Perpendicular redundante.
    let horizontal = sketch
        .constraints
        .iter()
        .filter(|c| !c.reference)
        .filter_map(|c| match c.kind {
            ConstraintKind::Horizontal { line } => Some(line),
            _ => None,
        })
        .collect::<Vec<_>>();
    let vertical = sketch
        .constraints
        .iter()
        .filter(|c| !c.reference)
        .filter_map(|c| match c.kind {
            ConstraintKind::Vertical { line } => Some(line),
            _ => None,
        })
        .collect::<Vec<_>>();
    for a in horizontal {
        for b in &vertical {
            intersection(sketch, a, *b, &mut result)?;
        }
    }
    Ok(result)
}

fn intersection(
    sketch: &Sketch,
    first: u64,
    second: u64,
    result: &mut Vec<Indicator>,
) -> Result<()> {
    let (a, b) = sketch.line(first)?;
    let (c, d) = sketch.line(second)?;
    if let Some((point, t, u)) = profiles::line_intersection(a, b, c, d)
        && (sketch.entity(first)?.infinite || (-1e-6..=1. + 1e-6).contains(&t))
        && (sketch.entity(second)?.infinite || (-1e-6..=1. + 1e-6).contains(&u))
    {
        push(result, point, Kind::Perpendicular);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markers_explain_right_angles_tangency_midpoints_and_centers() -> Result<()> {
        let mut sketch = Sketch::new("Símbolos", Plane::xy());
        sketch.add_rectangle([0., 0.], [20., 10.], false)?;
        let line = sketch.add_line([30., 0.], [50., 0.], false)?;
        let circle = sketch.add_circle([40., 5.], 5., false)?;
        let point = sketch.entity(circle)?.point_ids()[0];
        let end = sketch.add_point([50., 5.])?;
        sketch.add_curve(Curve::Line { start: point, end }, false)?;
        sketch.add_constraint(
            ConstraintKind::Tangent {
                line,
                arc: circle,
                point,
            },
            false,
        )?;
        let midpoint = sketch.add_point([40., 0.])?;
        sketch.add_constraint(
            ConstraintKind::Midpoint {
                point: midpoint,
                line,
            },
            false,
        )?;
        let markers = markers(&sketch)?;
        for kind in [
            Kind::Perpendicular,
            Kind::Tangent,
            Kind::Midpoint,
            Kind::Center,
        ] {
            assert!(markers.iter().any(|i| i.kind == kind));
        }
        assert_eq!(
            markers
                .iter()
                .filter(|i| i.kind == Kind::Perpendicular)
                .count(),
            4
        );
        assert!(
            markers
                .iter()
                .any(|i| i.kind == Kind::Tangent && i.point == [40., 0.])
        );
        Ok(())
    }
}
