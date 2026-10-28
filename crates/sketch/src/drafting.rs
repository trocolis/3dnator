//! Medidas de desenho no plano local: mm para comprimentos e radianos para ângulos.
//! A prévia não altera o rascunho; os campos travados têm precedência sobre o cursor.
use crate::{Point2, Result, SketchError, add, distance, profiles::Segment, scale, sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    Line,
    SymmetricLine,
    Rectangle,
    CenterRectangle,
    InclinedRectangle,
    Circle,
    DiameterCircle,
    TangentCircle,
    Arc,
}
impl Primitive {
    pub fn field_count(self) -> usize {
        if matches!(
            self,
            Self::Circle | Self::DiameterCircle | Self::TangentCircle
        ) {
            1
        } else {
            2
        }
    }
    pub fn angular(self, field: usize) -> bool {
        field == 1
            && matches!(
                self,
                Self::Line | Self::SymmetricLine | Self::Arc | Self::InclinedRectangle
            )
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

/// Abertura contínua a partir do segundo clique. Desembrulhar cada passo do
/// cursor evita trocar de sentido ao cruzar ±180°; uma volta exige percorrer
/// os quadrantes, em vez de surgir ao passar poucos graus abaixo do início.
#[derive(Debug, Default, Clone)]
pub struct ArcSweep {
    previous: Option<f64>,
    value: f64,
}
impl ArcSweep {
    pub fn update(&mut self, center: Point2, start: Point2, cursor: Point2) {
        if distance(center, cursor) < 1e-7 {
            return;
        }
        let angle = sub(cursor, center);
        let angle = angle[1].atan2(angle[0]);
        let from = sub(start, center);
        let previous = self.previous.unwrap_or_else(|| from[1].atan2(from[0]));
        self.value += crate::lines::relative_angle(angle, previous);
        self.value = self
            .value
            .clamp(-std::f64::consts::TAU + 1e-7, std::f64::consts::TAU - 1e-7);
        self.previous = Some(angle);
    }
    pub fn value(&self) -> f64 {
        self.value
    }
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
    preview_with_sweep(primitive, anchors, cursor, locked, reference, None)
}

pub fn preview_with_sweep(
    primitive: Primitive,
    anchors: &[Point2],
    cursor: Point2,
    locked: [Option<f64>; 2],
    reference: f64,
    sweep: Option<f64>,
) -> Result<Preview> {
    if !reference.is_finite() {
        return Err(SketchError("Referência angular inválida.".into()));
    }
    let first = *anchors
        .first()
        .ok_or_else(|| SketchError("Escolha o primeiro ponto.".into()))?;
    if anchors
        .iter()
        .flatten()
        .chain(cursor.iter())
        .any(|v| !v.is_finite())
    {
        return Err(SketchError("Coordenada inválida.".into()));
    }
    // Os modos derivados reaproveitam a validação e as medidas das primitivas
    // básicas. A geometria retornada é também a usada na confirmação do desenho.
    match primitive {
        Primitive::SymmetricLine => {
            let mut result =
                preview_with_reference(Primitive::Line, anchors, cursor, locked, reference)?;
            result.path.insert(0, sub(scale(first, 2.), result.point));
            return Ok(result);
        }
        Primitive::CenterRectangle => {
            let corner = add(first, scale(sub(cursor, first), 2.));
            let mut result = preview(Primitive::Rectangle, anchors, corner, locked)?;
            let half = scale(sub(result.point, first), 0.5);
            for point in &mut result.path {
                *point = sub(*point, half);
            }
            result.point = sub(result.point, half);
            return Ok(result);
        }
        Primitive::InclinedRectangle if anchors.len() == 1 => {
            return preview_with_reference(Primitive::Line, anchors, cursor, locked, reference);
        }
        Primitive::InclinedRectangle => {
            let side = sub(anchors[1], first);
            let length = distance(first, anchors[1]);
            if length < 1e-9 {
                return Err(SketchError(
                    "O lado do retângulo precisa ter comprimento positivo.".into(),
                ));
            }
            let normal = scale([-side[1], side[0]], 1. / length);
            let signed_width = crate::dot(sub(cursor, anchors[1]), normal);
            // Nesta etapa o único campo editável é a largura perpendicular ao lado.
            let width = preview(
                Primitive::Circle,
                &[first],
                add(first, scale(normal, signed_width)),
                [locked[0], None],
            )?
            .values[0];
            let offset = scale(normal, width * if signed_width < 0. { -1. } else { 1. });
            let point = add(anchors[1], offset);
            return Ok(Preview {
                point,
                values: [width, 0.],
                path: vec![first, anchors[1], point, add(first, offset), first],
                arc_start: None,
            });
        }
        Primitive::DiameterCircle => {
            let mut result = preview(Primitive::Circle, anchors, cursor, locked)?;
            let center = scale(add(first, result.point), 0.5);
            result.path = Segment::Circle {
                center,
                radius: result.values[0] * 0.5,
            }
            .sample(96);
            return Ok(result);
        }
        Primitive::TangentCircle => {
            if anchors.len() < 3 {
                return Err(SketchError("Selecione duas linhas perpendiculares.".into()));
            }
            let u = sub(anchors[1], first);
            let v = sub(anchors[2], first);
            let delta = sub(cursor, first);
            let x = crate::dot(delta, u);
            let y = crate::dot(delta, v);
            let radius = (x.abs() + y.abs()) * 0.5;
            let radius = preview(
                Primitive::Circle,
                &[first],
                add(first, [radius, 0.]),
                [locked[0], None],
            )?
            .values[0];
            let center = add(
                first,
                add(
                    scale(u, radius * if x < 0. { -1. } else { 1. }),
                    scale(v, radius * if y < 0. { -1. } else { 1. }),
                ),
            );
            return Ok(Preview {
                point: center,
                values: [radius, 0.],
                path: Segment::Circle { center, radius }.sample(96),
                arc_start: None,
            });
        }
        _ => {}
    }
    let delta = sub(cursor, first);
    let direction = delta[1].atan2(delta[0]);
    let start_angle = anchors.get(1).map(|p| {
        let d = sub(*p, first);
        d[1].atan2(d[0])
    });
    let sweep =
        sweep.unwrap_or_else(|| crate::lines::relative_angle(direction, start_angle.unwrap_or(0.)));
    let mut values = if primitive == Primitive::Rectangle {
        [delta[0].abs(), delta[1].abs()]
    } else if primitive == Primitive::Arc && anchors.len() == 2 {
        [distance(first, anchors[1]), sweep.abs()]
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
        start_angle.unwrap_or(0.) + values[1] * if sweep < 0. { -1. } else { 1. }
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
            Some(start) if distance(start, point) < 1e-9 => vec![start],
            Some(start) => Segment::Arc {
                center: first,
                a: start,
                b: point,
                clockwise: sweep < 0.,
            }
            .sample(96),
            None => vec![first, point],
        },
        _ => return Err(SketchError("Modo de desenho inválido.".into())),
    };
    Ok(Preview {
        point,
        values,
        path,
        arc_start,
    })
}

/// Detecta fechamento apoiado também em entidades anteriores à cadeia. A consulta
/// completa de áreas pode envolver muitos cruzamentos e deve rodar no trabalhador.
pub fn creates_area(before: &crate::Sketch, after: &crate::Sketch) -> Result<bool> {
    Ok(crate::areas::regions(after)?.len() > crate::areas::regions(before)?.len())
}

/// Encerrar a cadeia quando o último segmento formar um laço de área positiva,
/// inclusive ao cruzar o meio de um trecho anterior. Voltar sobre a mesma reta
/// não fecha área. A tolerância está em mm², no plano local do rascunho.
pub fn closes_chain(points: &[Point2], end: Point2) -> bool {
    let Some(start) = points.last() else {
        return false;
    };
    if points.len() < 3 {
        return false;
    }
    for index in 0..points.len() - 2 {
        let Some((intersection, t, u)) =
            crate::profiles::line_intersection(points[index], points[index + 1], *start, end)
        else {
            continue;
        };
        if !(-1e-8..=1. + 1e-8).contains(&t) || !(1e-8..=1. + 1e-8).contains(&u) {
            continue;
        }
        let mut polygon = vec![intersection];
        polygon.extend_from_slice(&points[index + 1..]);
        polygon.push(intersection);
        let area = polygon
            .windows(2)
            .map(|pair| crate::cross(sub(pair[0], intersection), sub(pair[1], intersection)))
            .sum::<f64>()
            .abs()
            * 0.5;
        if area > 1e-9 {
            return true;
        }
    }
    false
}

/// Referencial ortonormal das retas de apoio, em coordenadas locais do rascunho.
/// Os segmentos são tratados como retas suporte; tangências podem ficar além dos
/// extremos, como nas demais restrições de linha do solucionador.
pub fn tangent_frame(sketch: &crate::Sketch, first: u64, second: u64) -> Result<[Point2; 3]> {
    let (a, b) = sketch.line(first)?;
    let (c, d) = sketch.line(second)?;
    let ab = sub(b, a);
    let cd = sub(d, c);
    let length_u = distance(a, b);
    let length_v = distance(c, d);
    if first == second
        || !length_u.is_finite()
        || !length_v.is_finite()
        || length_u < 1e-9
        || length_v < 1e-9
    {
        return Err(SketchError(
            "Selecione duas linhas diferentes e perpendiculares.".into(),
        ));
    }
    let u = scale(ab, 1. / length_u);
    let v = scale(cd, 1. / length_v);
    if crate::dot(u, v).abs() > 1e-7 {
        return Err(SketchError(
            "As duas linhas devem ser perpendiculares.".into(),
        ));
    }
    let t = crate::cross(sub(c, a), v) / crate::cross(u, v);
    let origin = add(a, scale(u, t));
    Ok([origin, add(origin, u), add(origin, v)])
}

/// Cria os quatro lados compartilhando os vértices. As restrições conservam a
/// ortogonalidade no recálculo; retas infinitas continuam apenas auxiliares.
pub fn add_rectangle(
    sketch: &mut crate::Sketch,
    corners: [Point2; 4],
    kind: crate::LineType,
    inclined: bool,
) -> Result<Vec<u64>> {
    use crate::{ConstraintKind, Curve, LineType};
    for index in 0..4 {
        let edge = sub(corners[(index + 1) % 4], corners[index]);
        let next = sub(corners[(index + 2) % 4], corners[(index + 1) % 4]);
        if edge.iter().any(|v| !v.is_finite())
            || distance(corners[index], corners[(index + 1) % 4]) < 1e-9
            || crate::dot(edge, next).abs()
                > 1e-7 * distance([0., 0.], edge) * distance([0., 0.], next)
        {
            return Err(SketchError(
                "O retângulo precisa ter lados positivos e perpendiculares.".into(),
            ));
        }
    }
    let mut candidate = sketch.clone();
    let points = corners
        .map(|point| candidate.add_point(point))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let mut lines = Vec::new();
    for index in 0..4 {
        let id = candidate.add_curve(
            Curve::Line {
                start: points[index],
                end: points[(index + 1) % 4],
            },
            kind != LineType::Normal,
        )?;
        if kind == LineType::Infinite
            && let Some(entity) = candidate.entities.iter_mut().find(|e| e.id == id)
        {
            entity.infinite = true;
        }
        lines.push(id);
    }
    for index in 0..4 {
        let constraint = if inclined {
            // Três ângulos retos bastam; o quarto seria redundante.
            if index == 3 {
                continue;
            }
            ConstraintKind::Perpendicular {
                a: lines[index],
                b: lines[index + 1],
            }
        } else if index % 2 == 0 {
            ConstraintKind::Horizontal { line: lines[index] }
        } else {
            ConstraintKind::Vertical { line: lines[index] }
        };
        candidate.add_constraint(constraint, false)?;
    }
    *sketch = candidate;
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arc_follows_initial_motion_and_keeps_its_direction_across_half_turns() -> Result<()> {
        for sign in [-1., 1.] {
            let mut sweep = ArcSweep::default();
            for degree in [10_f64, 90., 170., 190., 270.] {
                let angle = sign * degree.to_radians();
                let cursor = [10. * angle.cos(), 10. * angle.sin()];
                sweep.update([0., 0.], [10., 0.], cursor);
                assert!((sweep.value() - angle).abs() < 1e-9);
                let p = preview_with_sweep(
                    Primitive::Arc,
                    &[[0., 0.], [10., 0.]],
                    cursor,
                    [None, None],
                    0.,
                    Some(sweep.value()),
                )?;
                assert!(distance(p.point, cursor) < 1e-9);
                assert!((p.values[1] - angle.abs()).abs() < 1e-9);
            }
            let p = preview_with_sweep(
                Primitive::Arc,
                &[[0., 0.], [10., 0.]],
                [0., sign * 10.],
                [None, Some(std::f64::consts::FRAC_PI_2)],
                0.,
                Some(sign),
            )?;
            assert!(distance(p.point, [0., sign * 10.]) < 1e-9);
        }
        Ok(())
    }
    #[test]
    fn derived_previews_keep_centers_quadrants_and_full_dimensions() -> Result<()> {
        let center = [10., 20.];
        for cursor in [[14., 23.], [6., 23.], [6., 17.], [14., 17.]] {
            let result = preview(
                Primitive::CenterRectangle,
                &[center],
                cursor,
                [Some(40.), Some(20.)],
            )?;
            assert_eq!(result.values, [40., 20.]);
            assert_eq!(scale(add(result.path[0], result.path[2]), 0.5), center);
            assert_eq!(
                result.point,
                [
                    if cursor[0] < 10. { -10. } else { 30. },
                    if cursor[1] < 20. { 10. } else { 30. }
                ]
            );
        }
        let result = preview(
            Primitive::SymmetricLine,
            &[center],
            [13., 24.],
            [Some(10.), None],
        )?;
        assert!(distance(result.path[0], [4., 12.]) < 1e-9);
        assert!(distance(result.point, [16., 28.]) < 1e-9);
        let result = preview(
            Primitive::DiameterCircle,
            &[[2., 3.]],
            [8., 11.],
            [Some(20.), None],
        )?;
        let circle_center = scale(add([2., 3.], result.point), 0.5);
        assert!(distance(circle_center, [8., 11.]) < 1e-9);
        assert!(
            result
                .path
                .iter()
                .all(|point| (distance(*point, circle_center) - 10.).abs() < 1e-9)
        );
        Ok(())
    }

    #[test]
    fn inclined_preview_projects_width_to_the_perpendicular_on_both_sides() -> Result<()> {
        for (cursor, expected) in [([1., 15.], [-2., 14.]), ([11., 1.], [14., 2.])] {
            let result = preview(
                Primitive::InclinedRectangle,
                &[[0., 0.], [6., 8.]],
                cursor,
                [Some(10.), None],
            )?;
            assert!(distance(result.point, expected) < 1e-9);
            assert!(
                (crate::dot(
                    sub(result.path[1], result.path[0]),
                    sub(result.path[2], result.path[1])
                ))
                .abs()
                    < 1e-9
            );
        }
        assert!(
            preview(
                Primitive::InclinedRectangle,
                &[[0., 0.], [0., 0.]],
                [5., 5.],
                [None, None]
            )
            .is_err()
        );
        assert!(
            preview(
                Primitive::InclinedRectangle,
                &[[0., 0.], [f64::NAN, 0.]],
                [5., 5.],
                [None, None]
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn tangent_preview_uses_rotated_translated_lines_and_all_four_quadrants() -> Result<()> {
        let mut sketch = crate::Sketch::new("Tangentes", crate::Plane::xy());
        let first = sketch.add_line([10., 20.], [13., 24.], false)?;
        let second = sketch.add_line([10., 20.], [6., 23.], false)?;
        let frame = tangent_frame(&sketch, first, second)?;
        assert!(distance(frame[0], [10., 20.]) < 1e-9);
        for x in [-1., 1.] {
            for y in [-1., 1.] {
                let expected = add(
                    frame[0],
                    add(
                        scale(sub(frame[1], frame[0]), 5. * x),
                        scale(sub(frame[2], frame[0]), 5. * y),
                    ),
                );
                let result = preview(Primitive::TangentCircle, &frame, expected, [Some(5.), None])?;
                assert!(distance(result.point, expected) < 1e-9);
                assert!(
                    result
                        .path
                        .iter()
                        .all(|point| (distance(*point, expected) - 5.).abs() < 1e-9)
                );
            }
        }
        let parallel = sketch.add_line([0., 0.], [3., 4.], false)?;
        assert!(tangent_frame(&sketch, first, first).is_err());
        assert!(tangent_frame(&sketch, first, parallel).is_err());
        let circle = sketch.add_circle([0., 0.], 5., false)?;
        assert!(tangent_frame(&sketch, first, circle).is_err());
        Ok(())
    }

    #[test]
    fn closing_chain_requires_area_and_detects_crossings_between_vertices() -> Result<()> {
        assert!(closes_chain(&[[0., 0.], [20., 0.], [20., 20.]], [0., 0.]));
        assert!(closes_chain(
            &[[0., 0.], [20., 0.], [20., 20.]],
            [10., -10.]
        ));
        assert!(!closes_chain(&[[0., 0.], [10., 0.], [20., 0.]], [0., 0.]));
        assert!(!closes_chain(&[[0., 0.], [20., 0.], [20., 20.]], [0., 20.]));
        assert!(!closes_chain(&[], [0., 0.]));
        // Um único segmento também pode fechar uma área sobre apoios já existentes.
        let mut before = crate::Sketch::new("Apoios", crate::Plane::xy());
        before.add_line([0., 0.], [20., 0.], false)?;
        before.add_line([20., 0.], [20., 20.], false)?;
        let mut after = before.clone();
        after.add_line([20., 20.], [0., 10.], false)?;
        assert!(!creates_area(&before, &after)?);
        after = before.clone();
        after.add_line([20., 20.], [0., 0.], false)?;
        assert!(creates_area(&before, &after)?);
        Ok(())
    }

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
