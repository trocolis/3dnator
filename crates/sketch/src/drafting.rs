//! Medidas de desenho no plano local: mm para comprimentos e radianos para ângulos.
//! A prévia não altera o rascunho; os campos travados têm precedência sobre o cursor.
use crate::{Point2, Result, SketchError, add, distance, profiles::Segment, scale, sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    Line,
    Rectangle,
    Circle,
    Arc,
}
impl Primitive {
    pub fn field_count(self) -> usize {
        if self == Self::Circle { 1 } else { 2 }
    }
    pub fn angular(self, field: usize) -> bool {
        field == 1 && matches!(self, Self::Line | Self::Arc)
    }
}

#[derive(Debug, Clone)]
pub struct Preview {
    pub point: Point2,
    pub values: [f64; 2],
    pub path: Vec<Point2>,
    /// O início do arco acompanha mudanças no raio durante a terceira etapa.
    pub arc_start: Option<Point2>,
}

pub fn preview(
    primitive: Primitive,
    anchors: &[Point2],
    cursor: Point2,
    locked: [Option<f64>; 2],
) -> Result<Preview> {
    preview_with_reference(primitive, anchors, cursor, locked, 0.)
}

/// A direção da linha de origem é capturada no primeiro clique. As demais
/// primitivas conservam suas convenções: arco usa direção inicial/abertura.
pub fn preview_with_reference(
    primitive: Primitive,
    anchors: &[Point2],
    cursor: Point2,
    locked: [Option<f64>; 2],
    reference: f64,
) -> Result<Preview> {
    if !reference.is_finite() {
        return Err(SketchError("Referência angular inválida.".into()));
    }
    let first = *anchors
        .first()
        .ok_or_else(|| SketchError("Escolha o primeiro ponto.".into()))?;
    if first.iter().chain(cursor.iter()).any(|v| !v.is_finite()) {
        return Err(SketchError("Coordenada inválida.".into()));
    }
    let delta = sub(cursor, first);
    let direction = delta[1].atan2(delta[0]);
    let start_angle = anchors.get(1).map(|p| {
        let d = sub(*p, first);
        d[1].atan2(d[0])
    });
    let mut values = if primitive == Primitive::Rectangle {
        [delta[0].abs(), delta[1].abs()]
    } else if primitive == Primitive::Arc && anchors.len() == 2 {
        [
            distance(first, anchors[1]),
            (direction - start_angle.unwrap_or(0.)).rem_euclid(std::f64::consts::TAU),
        ]
    } else {
        [
            distance(first, cursor),
            if primitive == Primitive::Line {
                crate::lines::relative_angle(direction, reference)
            } else {
                direction
            },
        ]
    };
    for field in 0..primitive.field_count() {
        if let Some(value) = locked[field] {
            if !value.is_finite() || (!primitive.angular(field) && !(1e-9..=1e9).contains(&value)) {
                return Err(SketchError(
                    "Use uma medida positiva de até 1.000.000.000 mm ou um ângulo finito.".into(),
                ));
            }
            if primitive == Primitive::Arc
                && anchors.len() == 2
                && field == 1
                && !(1e-9..std::f64::consts::TAU - 1e-9).contains(&value)
            {
                return Err(SketchError(
                    "A abertura do arco deve estar entre 0° e 360°, sem incluir os limites.".into(),
                ));
            }
            values[field] = value;
        }
    }
    let angle = if primitive == Primitive::Arc && anchors.len() == 2 {
        start_angle.unwrap_or(0.) + values[1]
    } else if primitive == Primitive::Circle {
        direction
    } else if primitive == Primitive::Line {
        reference + values[1]
    } else {
        values[1]
    };
    let point = if primitive == Primitive::Rectangle {
        // Dimensões são positivas; o quadrante continua sendo escolhido pelo mouse.
        add(
            first,
            [
                values[0] * if delta[0] < 0. { -1. } else { 1. },
                values[1] * if delta[1] < 0. { -1. } else { 1. },
            ],
        )
    } else {
        add(first, scale([angle.cos(), angle.sin()], values[0]))
    };
    let arc_start =
        start_angle.map(|angle| add(first, scale([angle.cos(), angle.sin()], values[0])));
    let path = match primitive {
        Primitive::Line => vec![first, point],
        Primitive::Rectangle => vec![
            first,
            [point[0], first[1]],
            point,
            [first[0], point[1]],
            first,
        ],
        Primitive::Circle => Segment::Circle {
            center: first,
            radius: values[0],
        }
        .sample(96),
        Primitive::Arc => match arc_start {
            Some(start) => Segment::Arc {
                center: first,
                a: start,
                b: point,
                clockwise: false,
            }
            .sample(96),
            None => vec![first, point],
        },
    };
    Ok(Preview {
        point,
        values,
        path,
        arc_start,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn line_angles_are_relative_to_the_captured_reference_only() -> Result<()> {
        let reference = std::f64::consts::FRAC_PI_4;
        let p = preview_with_reference(
            Primitive::Line,
            &[[0., 0.]],
            [0., 10.],
            [None, None],
            reference,
        )?;
        assert!((p.values[1] - reference).abs() < 1e-10);
        let p = preview_with_reference(
            Primitive::Line,
            &[[0., 0.]],
            [10., 0.],
            [Some(20.), Some(std::f64::consts::FRAC_PI_2)],
            reference,
        )?;
        assert!((p.point[0] + 2_f64.sqrt() * 10.).abs() < 1e-10);
        assert!((p.point[1] - 2_f64.sqrt() * 10.).abs() < 1e-10);
        let p = preview(Primitive::Line, &[[0., 0.]], [0., 10.], [None, None])?;
        assert!((p.values[1] - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
        Ok(())
    }
    #[test]
    fn locked_values_override_cursor_without_losing_quadrants_or_arc_radius() -> Result<()> {
        let p = preview(
            Primitive::Line,
            &[[0., 0.]],
            [3., 4.],
            [Some(20.), Some(std::f64::consts::FRAC_PI_2)],
        )?;
        assert!(distance(p.point, [0., 20.]) < 1e-10);
        let p = preview(
            Primitive::Rectangle,
            &[[10., 10.]],
            [0., 0.],
            [Some(30.), Some(20.)],
        )?;
        assert_eq!(p.point, [-20., -10.]);
        let p = preview(Primitive::Circle, &[[1., 2.]], [4., 6.], [Some(10.), None])?;
        assert!(distance(p.point, [7., 10.]) < 1e-10);
        let p = preview(
            Primitive::Arc,
            &[[0., 0.], [5., 0.]],
            [0., 1.],
            [Some(10.), Some(std::f64::consts::FRAC_PI_2)],
        )?;
        assert_eq!(p.arc_start, Some([10., 0.]));
        assert!(distance(p.point, [0., 10.]) < 1e-10);
        assert!(p.path.len() > 2);
        for value in [f64::NAN, 0., -1., 1e10] {
            assert!(
                preview(
                    Primitive::Circle,
                    &[[0., 0.]],
                    [3., 4.],
                    [Some(value), None]
                )
                .is_err()
            );
        }
        Ok(())
    }
}
