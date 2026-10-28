//! Solução amortecida com jacobiano por diferenças finitas e análise de posto.
//! O rascunho original nunca é alterado; o chamador decide publicar ou descartar o resultado.
#![allow(clippy::needless_range_loop)] // Índices representam pivôs e colunas de matrizes pequenas.
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    UnderConstrained { degrees_of_freedom: usize },
    FullyConstrained,
    Conflict,
}
#[derive(Debug, Clone)]
pub struct Report {
    pub sketch: Sketch,
    pub status: Status,
    pub max_error: f64,
    pub redundant: bool,
    pub conflicting: Vec<u64>,
}

pub(crate) fn constraint_residuals(s: &Sketch, c: &ConstraintKind) -> Result<Vec<f64>> {
    let direction = |line| -> Result<Point2> {
        let (a, b) = s.line(line)?;
        let d = sub(b, a);
        let length = distance(a, b);
        if length < 1e-12 {
            return Err(SketchError("Linha degenerada na restrição.".into()));
        }
        Ok(scale(d, 1. / length))
    };
    let checked = |value: f64| {
        if value.is_finite() {
            Ok(value)
        } else {
            Err(SketchError("Valor de restrição não finito.".into()))
        }
    };
    let result = match *c {
        ConstraintKind::Tangent { line, arc, point } => {
            if let Curve::Circle { center, radius } = s.entity(arc)?.curve {
                // Círculos completos não têm extremos. O campo `point` identifica
                // seu centro; a distância à reta suporte deve ser igual ao raio.
                if point != center {
                    return Err(SketchError(
                        "Centro inválido na tangência do círculo.".into(),
                    ));
                }
                let (a, _) = s.line(line)?;
                return checked(cross(sub(s.point(center)?, a), direction(line)?).abs() - radius)
                    .map(|value| vec![value]);
            }
            if !s.entity(line)?.point_ids().contains(&point)
                || !s.entity(arc)?.point_ids().contains(&point)
            {
                return Err(SketchError(
                    "Tangência exige uma extremidade compartilhada.".into(),
                ));
            }
            let center = s.radial(arc)?.0;
            vec![dot(direction(line)?, sub(s.point(point)?, center))]
        }
        ConstraintKind::Fixed { point, position } => {
            let d = sub(s.point(point)?, position);
            vec![d[0], d[1]]
        }
        ConstraintKind::Coincident { a, b } => {
            let d = sub(s.point(a)?, s.point(b)?);
            vec![d[0], d[1]]
        }
        ConstraintKind::Horizontal { line } => {
            let (a, b) = s.line(line)?;
            vec![b[1] - a[1]]
        }
        ConstraintKind::Vertical { line } => {
            let (a, b) = s.line(line)?;
            vec![b[0] - a[0]]
        }
        ConstraintKind::Distance { a, b, value } => {
            if checked(value)? < 0. {
                return Err(SketchError("Distância negativa.".into()));
            }
            vec![distance(s.point(a)?, s.point(b)?) - value]
        }
        ConstraintKind::DeltaX { a, b, value } => {
            vec![s.point(b)?[0] - s.point(a)?[0] - checked(value)?]
        }
        ConstraintKind::DeltaY { a, b, value } => {
            vec![s.point(b)?[1] - s.point(a)?[1] - checked(value)?]
        }
        ConstraintKind::Radius { circle, value } => {
            if checked(value)? <= 0. {
                return Err(SketchError("Raio de cota inválido.".into()));
            }
            vec![s.radial(circle)?.1 - value]
        }
        ConstraintKind::Angle { a, b, value } => {
            let d = sub(s.point(b)?, s.point(a)?);
            if distance(s.point(a)?, s.point(b)?) < 1e-12 {
                return Err(SketchError("Ângulo exige dois pontos distintos.".into()));
            }
            // O resíduo circular mantém a direção (inclusive 180°), sem aceitar
            // a solução invertida de uma simples tangente. O jacobiano desembrulha ±π.
            let target = checked(value)?;
            let target = target.sin().atan2(target.cos());
            let delta = d[1].atan2(d[0]) - target;
            vec![delta.sin().atan2(delta.cos())]
        }
        ConstraintKind::LineAngle {
            line,
            reference,
            value,
            ..
        } => {
            if line == reference {
                return Err(SketchError(
                    "A referência angular deve ser outra linha.".into(),
                ));
            }
            let measured = s.measure(c)?.unwrap_or_default();
            vec![lines::relative_angle(measured, checked(value)?)]
        }
        ConstraintKind::Diameter { circle, value } => {
            if checked(value)? <= 0. {
                return Err(SketchError("Diâmetro de cota inválido.".into()));
            }
            vec![2. * s.radial(circle)?.1 - value]
        }
        ConstraintKind::PointLineDistance { point, line, value } => {
            if s.entity(line)?.point_ids().contains(&point) {
                return Err(SketchError(
                    "A distância à linha exige um ponto fora dela.".into(),
                ));
            }
            if checked(value)? < 0. {
                return Err(SketchError("Distância negativa.".into()));
            }
            vec![s.measure(c)?.unwrap_or_default() - value]
        }
        ConstraintKind::LineCircleDistance {
            line,
            circle,
            value,
            ..
        } => {
            s.line(line)?;
            s.radial(circle)?;
            if checked(value)? < 0. {
                return Err(SketchError("Distância negativa.".into()));
            }
            vec![s.measure(c)?.unwrap_or_default() - value]
        }
        ConstraintKind::Parallel { a, b } => vec![cross(direction(a)?, direction(b)?)],
        ConstraintKind::Perpendicular { a, b } => vec![dot(direction(a)?, direction(b)?)],
        ConstraintKind::EqualLength { a, b } => {
            let (a0, a1) = s.line(a)?;
            let (b0, b1) = s.line(b)?;
            vec![distance(a0, a1) - distance(b0, b1)]
        }
        ConstraintKind::Midpoint { point, line } => {
            let (a, b) = s.line(line)?;
            let d = sub(s.point(point)?, scale(add(a, b), 0.5));
            vec![d[0], d[1]]
        }
        ConstraintKind::PointOnLine { point, line } => {
            let (a, _) = s.line(line)?;
            vec![cross(sub(s.point(point)?, a), direction(line)?)]
        }
        ConstraintKind::Concentric { a, b } => {
            let d = sub(s.circle(a)?.0, s.circle(b)?.0);
            vec![d[0], d[1]]
        }
    };
    if result.iter().any(|v| !v.is_finite()) {
        return Err(SketchError(
            "Restrição resultou em valor não finito.".into(),
        ));
    }
    Ok(result)
}
pub(crate) fn residuals(s: &Sketch) -> Result<Vec<f64>> {
    let mut r = Vec::new();
    for c in s.constraints.iter().filter(|c| !c.reference) {
        r.extend(constraint_residuals(s, &c.kind)?);
    }
    for entity in &s.entities {
        if let Curve::Arc {
            center, start, end, ..
        } = entity.curve
        {
            r.push(
                distance(s.point(center)?, s.point(start)?)
                    - distance(s.point(center)?, s.point(end)?),
            );
        }
    }
    Ok(r)
}
pub fn max_error(s: &Sketch) -> Result<f64> {
    Ok(residuals(s)?
        .into_iter()
        .map(f64::abs)
        .fold(0_f64, f64::max))
}
fn variables(s: &Sketch) -> Vec<f64> {
    s.points
        .iter()
        .flat_map(|p| p.position)
        .chain(s.entities.iter().filter_map(|e| {
            if let Curve::Circle { radius, .. } = e.curve {
                Some(radius)
            } else {
                None
            }
        }))
        .collect()
}
fn assign(s: &mut Sketch, x: &[f64]) {
    for (i, p) in s.points.iter_mut().enumerate() {
        p.position = [x[i * 2], x[i * 2 + 1]];
    }
    let mut offset = s.points.len() * 2;
    for e in &mut s.entities {
        if let Curve::Circle { radius, .. } = &mut e.curve {
            *radius = x[offset];
            offset += 1;
        }
    }
}
fn jacobian(s: &Sketch, x: &[f64], r: &[f64]) -> Result<Vec<Vec<f64>>> {
    let mut angular = vec![false; r.len()];
    let mut offset = 0;
    for constraint in s.constraints.iter().filter(|c| !c.reference) {
        if matches!(
            constraint.kind,
            ConstraintKind::Angle { .. } | ConstraintKind::LineAngle { .. }
        ) {
            angular[offset] = true;
        }
        offset += constraint_residuals(s, &constraint.kind)?.len();
    }
    let mut j = vec![vec![0.; x.len()]; r.len()];
    let mut candidate = s.clone();
    for column in 0..x.len() {
        let h = 1e-6 * x[column].abs().max(1.);
        let mut shifted = x.to_vec();
        shifted[column] += h;
        assign(&mut candidate, &shifted);
        let next = residuals(&candidate)?;
        for (row, value) in next.into_iter().enumerate() {
            let difference = value - r[row];
            // Uma perturbação infinitesimal pode atravessar o corte de atan2.
            // Derivar a volta de 2π produziria um pivô artificial e travaria 0°→180°.
            j[row][column] = if angular[row] {
                difference.sin().atan2(difference.cos()) / h
            } else {
                difference / h
            };
        }
    }
    Ok(j)
}
fn linear_solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for column in 0..n {
        let pivot =
            (column..n).max_by(|&i, &k| a[i][column].abs().total_cmp(&a[k][column].abs()))?;
        if a[pivot][column].abs() < 1e-18 {
            return None;
        }
        a.swap(column, pivot);
        b.swap(column, pivot);
        let denominator = a[column][column];
        for value in &mut a[column][column..] {
            *value /= denominator;
        }
        b[column] /= denominator;
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = a[row][column];
            for k in column..n {
                a[row][k] -= factor * a[column][k];
            }
            b[row] -= factor * b[column];
        }
    }
    Some(b)
}
fn rank(mut matrix: Vec<Vec<f64>>, columns: usize) -> usize {
    let mut row = 0;
    for column in 0..columns {
        let Some(pivot) = (row..matrix.len())
            .max_by(|&a, &b| matrix[a][column].abs().total_cmp(&matrix[b][column].abs()))
        else {
            break;
        };
        if matrix[pivot][column].abs() < 1e-7 {
            continue;
        }
        matrix.swap(row, pivot);
        let denominator = matrix[row][column];
        for value in &mut matrix[row][column..] {
            *value /= denominator;
        }
        for i in row + 1..matrix.len() {
            let factor = matrix[i][column];
            for k in column..columns {
                matrix[i][k] -= factor * matrix[row][k];
            }
        }
        row += 1;
        if row == matrix.len() {
            break;
        }
    }
    row
}
pub fn solve(source: &Sketch) -> Result<Report> {
    source.validate()?;
    let mut sketch = source.clone();
    let mut x = variables(&sketch);
    if x.len() > 256 {
        return Err(SketchError(
            "Este solucionador aceita até 256 variáveis por rascunho.".into(),
        ));
    }
    let mut r = residuals(&sketch)?;
    let mut lambda = 1e-4;
    for _ in 0..64 {
        if r.iter().all(|v| v.abs() < 1e-8) {
            break;
        }
        let j = jacobian(&sketch, &x, &r)?;
        let n = x.len();
        let mut a = vec![vec![0.; n]; n];
        let mut b = vec![0.; n];
        for (row, error) in j.iter().zip(&r) {
            for i in 0..n {
                b[i] -= row[i] * error;
                for k in 0..n {
                    a[i][k] += row[i] * row[k];
                }
            }
        }
        for (i, row) in a.iter_mut().enumerate() {
            row[i] += lambda;
        }
        let Some(delta) = linear_solve(a, b) else {
            lambda *= 10.;
            continue;
        };
        let next = x
            .iter()
            .zip(delta)
            .map(|(value, delta)| value + delta)
            .collect::<Vec<_>>();
        let mut candidate = sketch.clone();
        assign(&mut candidate, &next);
        let Ok(errors) = residuals(&candidate) else {
            lambda *= 10.;
            continue;
        };
        let cost = |values: &[f64]| values.iter().map(|v| v * v).sum::<f64>();
        if cost(&errors) < cost(&r) {
            x = next;
            r = errors;
            sketch = candidate;
            lambda = (lambda * 0.2).max(1e-12);
        } else {
            lambda = (lambda * 10.).min(1e12);
        }
    }
    let max_error = r.iter().copied().map(f64::abs).fold(0_f64, f64::max);
    let matrix = jacobian(&sketch, &x, &r)?;
    let rank = rank(matrix, x.len());
    let status = if max_error > 1e-6 {
        Status::Conflict
    } else if rank == x.len() {
        Status::FullyConstrained
    } else {
        Status::UnderConstrained {
            degrees_of_freedom: x.len() - rank,
        }
    };
    let conflicting = sketch
        .constraints
        .iter()
        .filter(|c| !c.reference)
        .filter_map(|c| {
            constraint_residuals(&sketch, &c.kind)
                .ok()
                .filter(|r| r.iter().any(|v| v.abs() > 1e-6))
                .map(|_| c.id)
        })
        .collect();
    sketch.refresh_reference_dimensions()?;
    Ok(Report {
        sketch,
        status,
        max_error,
        redundant: r.len() > rank,
        conflicting,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn angular_dimensions_keep_direction_across_the_pi_boundary() -> Result<()> {
        for (initial, target) in [(179_f64, -179_f64), (0., 180.), (45., -30.)] {
            let mut s = Sketch::new("Ângulo", Plane::xy());
            let line = s.add_line(
                [0., 0.],
                [
                    20. * initial.to_radians().cos(),
                    20. * initial.to_radians().sin(),
                ],
                false,
            )?;
            let Curve::Line { start, end } = s.entity(line)?.curve else {
                return Err(SketchError("Linha ausente".into()));
            };
            s.add_constraint(
                ConstraintKind::Fixed {
                    point: start,
                    position: [0., 0.],
                },
                false,
            )?;
            s.add_constraint(
                ConstraintKind::Distance {
                    a: start,
                    b: end,
                    value: 20.,
                },
                false,
            )?;
            s.add_constraint(
                ConstraintKind::Angle {
                    a: start,
                    b: end,
                    value: target.to_radians(),
                },
                false,
            )?;
            let report = solve(&s)?;
            assert_ne!(report.status, Status::Conflict, "{initial} → {target}");
            let (a, b) = report.sketch.line(line)?;
            let d = sub(b, a);
            assert!(
                distance(
                    d,
                    [
                        20. * target.to_radians().cos(),
                        20. * target.to_radians().sin()
                    ]
                ) < 1e-5
            );
            let reference = s.add_constraint(
                ConstraintKind::Angle {
                    a: start,
                    b: end,
                    value: 0.,
                },
                true,
            )?;
            s.refresh_reference_dimensions()?;
            assert!(
                matches!(s.constraints.iter().find(|c| c.id == reference).map(|c| &c.kind), Some(ConstraintKind::Angle {value,..}) if (*value-initial.to_radians()).abs() < 1e-10)
            );
        }
        Ok(())
    }
    #[test]
    fn dimensional_rectangle_is_solved_and_fully_constrained() -> Result<()> {
        let mut s = Sketch::new("Base", Plane::xy());
        s.add_rectangle([0., 0.], [12., 8.], false)?;
        let p = s.points.iter().map(|p| p.id).collect::<Vec<_>>();
        s.add_constraint(
            ConstraintKind::Fixed {
                point: p[0],
                position: [0., 0.],
            },
            false,
        )?;
        s.add_constraint(
            ConstraintKind::DeltaX {
                a: p[0],
                b: p[1],
                value: 60.,
            },
            false,
        )?;
        s.add_constraint(
            ConstraintKind::DeltaY {
                a: p[0],
                b: p[3],
                value: 20.,
            },
            false,
        )?;
        let report = solve(&s)?;
        assert_eq!(report.status, Status::FullyConstrained);
        assert!(distance(report.sketch.point(p[2])?, [60., 20.]) < 1e-6);
        assert_eq!(s.point(p[2])?, [12., 8.]);
        Ok(())
    }
    #[test]
    fn conflicting_dimensions_preserve_source_and_are_identified() -> Result<()> {
        let mut s = Sketch::new("Conflito", Plane::xy());
        let line = s.add_line([0., 0.], [10., 0.], false)?;
        let Curve::Line { start, end } = s.entity(line)?.curve else {
            return Err(SketchError("Linha ausente".into()));
        };
        s.add_constraint(
            ConstraintKind::Distance {
                a: start,
                b: end,
                value: 10.,
            },
            false,
        )?;
        s.add_constraint(
            ConstraintKind::Distance {
                a: start,
                b: end,
                value: 20.,
            },
            false,
        )?;
        let report = solve(&s)?;
        assert_eq!(report.status, Status::Conflict);
        assert!(!report.conflicting.is_empty());
        assert_eq!(s.point(end)?, [10., 0.]);
        Ok(())
    }

    /// Cotas da ferramenta Cota: diâmetro, ponto–linha, linha–círculo (folga e
    /// penetração) e ângulo entre lados escolhidos das linhas. Cada uma leva a geometria
    /// ao valor e `measure` devolve o mesmo valor depois da solução.
    #[test]
    fn dimension_tool_kinds_drive_and_measure_the_geometry() -> Result<()> {
        let fixed = |s: &mut Sketch, point: u64| -> Result<()> {
            let position = s.point(point)?;
            s.add_constraint(ConstraintKind::Fixed { point, position }, false)?;
            Ok(())
        };
        let mut s = Sketch::new("Cotas", Plane::xy());
        let base = s.add_line([0., 0.], [40., 0.], false)?;
        let Curve::Line { start, end } = s.entity(base)?.curve else {
            return Err(SketchError("Linha ausente".into()));
        };
        fixed(&mut s, start)?;
        fixed(&mut s, end)?;
        let circle = s.add_circle([20., 15.], 4., false)?;
        let Curve::Circle { center, .. } = s.entity(circle)?.curve else {
            return Err(SketchError("Círculo ausente".into()));
        };
        let point = s.add_point([5., 9.])?;
        let kinds = [
            ConstraintKind::Diameter { circle, value: 12. },
            ConstraintKind::PointLineDistance {
                point,
                line: base,
                value: 3.,
            },
            ConstraintKind::LineCircleDistance {
                line: base,
                circle,
                value: 2.,
                crossing: false,
            },
        ];
        for kind in kinds.iter().cloned() {
            s.add_constraint(kind, false)?;
        }
        let report = solve(&s)?;
        assert_ne!(report.status, Status::Conflict);
        let solved = &report.sketch;
        for kind in &kinds {
            let measured = solved
                .measure(kind)?
                .ok_or(SketchError("Sem medida".into()))?;
            let expected = kind
                .dimension_value()
                .ok_or(SketchError("Sem valor".into()))?;
            assert!((measured - expected).abs() < 1e-6, "{kind:?}: {measured}");
        }
        // Raio 6 e folga 2: o centro fica a 8 mm da linha.
        assert!((solved.point(center)?[1] - 8.).abs() < 1e-6);
        // Com a linha cortando o círculo, a cota mede quanto a borda passa da reta.
        let crossing = ConstraintKind::LineCircleDistance {
            line: base,
            circle,
            value: 0.,
            crossing: true,
        };
        assert!((solved.measure(&crossing)?.unwrap_or_default() + 2.).abs() < 1e-6);
        // Um ponto da própria linha não define distância.
        let own = ConstraintKind::PointLineDistance {
            point: start,
            line: base,
            value: 1.,
        };
        assert!(constraint_residuals(solved, &own).is_err());
        Ok(())
    }

    #[test]
    fn line_angle_sides_choose_the_measured_wedge() -> Result<()> {
        let mut s = Sketch::new("Setores", Plane::xy());
        let horizontal = s.add_line([0., 0.], [10., 0.], false)?;
        let inclined = s.add_line([0., 0.], [10., 10.], false)?;
        let angle = |flip_line, flip_reference| ConstraintKind::LineAngle {
            line: inclined,
            reference: horizontal,
            value: 0.,
            flip_line,
            flip_reference,
        };
        let degrees = |kind: &ConstraintKind| -> Result<f64> {
            Ok(s.measure(kind)?.unwrap_or_default().to_degrees())
        };
        assert!((degrees(&angle(false, false))? - 45.).abs() < 1e-9);
        // Inverter a referência mede o setor suplementar (sentido horário: -135°).
        assert!((degrees(&angle(false, true))? + 135.).abs() < 1e-9);
        assert!((degrees(&angle(true, true))? - 45.).abs() < 1e-9);
        // Dirigida no setor suplementar, a linha gira até ele ter 120°.
        let mut driven = s.clone();
        let Curve::Line { start, .. } = driven.entity(horizontal)?.curve else {
            return Err(SketchError("Linha ausente".into()));
        };
        let position = driven.point(start)?;
        driven.add_constraint(
            ConstraintKind::Fixed {
                point: start,
                position,
            },
            false,
        )?;
        driven.add_constraint(ConstraintKind::Horizontal { line: horizontal }, false)?;
        driven.add_constraint(
            ConstraintKind::LineAngle {
                line: horizontal,
                reference: inclined,
                value: 120_f64.to_radians(),
                flip_line: true,
                flip_reference: false,
            },
            false,
        )?;
        let solved = solve(&driven)?.sketch;
        let incline = lines::angle(&solved, inclined)?.to_degrees();
        assert!((incline.abs() - 60.).abs() < 1e-5, "{incline}");
        Ok(())
    }

    #[test]
    fn dimension_labels_follow_their_constraints() -> Result<()> {
        let mut s = Sketch::new("Textos", Plane::xy());
        let line = s.add_line([0., 0.], [10., 0.], false)?;
        let Curve::Line { start, end } = s.entity(line)?.curve else {
            return Err(SketchError("Linha ausente".into()));
        };
        let id = s.add_constraint(
            ConstraintKind::Distance {
                a: start,
                b: end,
                value: 10.,
            },
            false,
        )?;
        s.dimension_labels.insert(id, [0., 6.]);
        s.validate()?;
        let mut orphan = s.clone();
        orphan.dimension_labels.insert(999, [0., 1.]);
        assert!(orphan.validate().is_err());
        let mut infinite = s.clone();
        infinite.dimension_labels.insert(id, [f64::NAN, 0.]);
        assert!(infinite.validate().is_err());
        // Apagar a linha remove a cota e o texto dela.
        s.entities.clear();
        s.prune_unused_points();
        assert!(s.dimension_labels.is_empty());
        s.validate()?;
        Ok(())
    }
}
