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
    let mut j = vec![vec![0.; x.len()]; r.len()];
    let mut candidate = s.clone();
    for column in 0..x.len() {
        let h = 1e-6 * x[column].abs().max(1.);
        let mut shifted = x.to_vec();
        shifted[column] += h;
        assign(&mut candidate, &shifted);
        let next = residuals(&candidate)?;
        for (row, value) in next.into_iter().enumerate() {
            j[row][column] = (value - r[row]) / h;
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
}
