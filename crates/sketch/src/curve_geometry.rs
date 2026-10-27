//! Consultas analíticas compartilhadas pelas áreas, capturas e pelo aparador.
//! Parâmetros percorrem a curva orientada entre 0 e 1, sem discretização.
use crate::{profiles::Segment, *};
const EPS: f64 = 1e-6;

pub(crate) fn radial(s: &Segment) -> Option<(Point2, f64)> {
    match *s {
        Segment::Circle { center, radius } => Some((center, radius)),
        Segment::Arc { center, a, .. } => Some((center, distance(center, a))),
        _ => None,
    }
}
pub(crate) fn point(s: &Segment, t: f64) -> Point2 {
    match *s {
        Segment::Line { a, b } => add(a, scale(sub(b, a), t)),
        Segment::Circle { center, radius } => add(
            center,
            scale(
                [
                    (t * std::f64::consts::TAU).cos(),
                    (t * std::f64::consts::TAU).sin(),
                ],
                radius,
            ),
        ),
        Segment::Arc {
            center,
            a,
            b,
            clockwise,
        } => {
            let d = sub(a, center);
            let end = sub(b, center);
            let sign = if clockwise { -1. } else { 1. };
            let start = d[1].atan2(d[0]);
            let sweep = ((end[1].atan2(end[0]) - start) * sign).rem_euclid(std::f64::consts::TAU);
            let angle = start + sign * sweep * t;
            add(
                center,
                scale([angle.cos(), angle.sin()], distance(center, a)),
            )
        }
    }
}
pub(crate) fn parameter(s: &Segment, p: Point2) -> Option<f64> {
    match *s {
        Segment::Line { a, b } => {
            let d = sub(b, a);
            let t = dot(sub(p, a), d) / dot(d, d);
            ((-EPS..=1. + EPS).contains(&t) && distance(point(s, t), p) < EPS)
                .then_some(t.clamp(0., 1.))
        }
        Segment::Circle { center, radius } => {
            let d = sub(p, center);
            ((distance(center, p) - radius).abs() < EPS).then_some(
                d[1].atan2(d[0]).rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU,
            )
        }
        Segment::Arc {
            center,
            a,
            b,
            clockwise,
        } => {
            if distance(p, a) < EPS {
                return Some(0.);
            }
            if distance(p, b) < EPS {
                return Some(1.);
            }
            let sign = if clockwise { -1. } else { 1. };
            let first = sub(a, center);
            let last = sub(b, center);
            let d = sub(p, center);
            let start = first[1].atan2(first[0]);
            let sweep = ((last[1].atan2(last[0]) - start) * sign).rem_euclid(std::f64::consts::TAU);
            let travel = ((d[1].atan2(d[0]) - start) * sign).rem_euclid(std::f64::consts::TAU);
            (sweep > 1e-12
                && travel <= sweep + 1e-10
                && (distance(center, p) - distance(center, a)).abs() < EPS)
                .then_some((travel / sweep).clamp(0., 1.))
        }
    }
}
fn candidates(a: &Segment, b: &Segment) -> Vec<Point2> {
    match (a, b) {
        (Segment::Line { a, b }, Segment::Line { a: c, b: d }) => {
            if let Some((p, _, _)) = profiles::line_intersection(*a, *b, *c, *d) {
                vec![p]
            } else {
                vec![*a, *b, *c, *d]
            }
        }
        (Segment::Line { a, b }, curve) | (curve, Segment::Line { a, b }) => {
            let Some((center, radius)) = radial(curve) else {
                return Vec::new();
            };
            let d = sub(*b, *a);
            let offset = sub(*a, center);
            let aa = dot(d, d);
            let bb = 2. * dot(d, offset);
            let cc = dot(offset, offset) - radius * radius;
            let discriminant = bb * bb - 4. * aa * cc;
            if discriminant < -1e-9 * aa {
                return Vec::new();
            }
            let root = discriminant.max(0.).sqrt();
            [-bb - root, -bb + root]
                .map(|v| add(*a, scale(d, v / (2. * aa))))
                .to_vec()
        }
        _ => {
            let Some((c, r)) = radial(a) else {
                return Vec::new();
            };
            let Some((d, s)) = radial(b) else {
                return Vec::new();
            };
            let length = distance(c, d);
            if length < EPS {
                // Círculos coincidentes não têm um encontro isolado: a costura
                // arbitrária não vira vértice. Em arcos sobrepostos, só seus
                // extremos podem delimitar trechos finitos para o aparador.
                return [a, b]
                    .into_iter()
                    .flat_map(|curve| match *curve {
                        Segment::Arc { a, b, .. } => vec![a, b],
                        _ => Vec::new(),
                    })
                    .collect();
            }
            if length > r + s + EPS || length < (r - s).abs() - EPS {
                return Vec::new();
            }
            let x = (r * r - s * s + length * length) / (2. * length);
            let h = (r * r - x * x).max(0.).sqrt();
            let u = scale(sub(d, c), 1. / length);
            let middle = add(c, scale(u, x));
            vec![
                add(middle, [-u[1] * h, u[0] * h]),
                add(middle, [u[1] * h, -u[0] * h]),
            ]
        }
    }
}
/// Somente encontros pertencentes aos dois trechos finitos; tangências não duplicam nós.
pub(crate) fn intersections(a: &Segment, b: &Segment) -> Vec<Point2> {
    let mut points = Vec::new();
    for p in candidates(a, b) {
        if parameter(a, p).is_some()
            && parameter(b, p).is_some()
            && !points.iter().any(|q| distance(*q, p) < EPS)
        {
            points.push(p);
        }
    }
    points
}

/// Projeta o clique na curva exata. Em arcos, projeções fora da abertura ficam no
/// extremo mais próximo, em vez de alcançar a continuação imaginária do círculo.
pub(crate) fn closest_parameter(s: &Segment, p: Point2) -> f64 {
    match *s {
        Segment::Line { a, b } => {
            let d = sub(b, a);
            (dot(sub(p, a), d) / dot(d, d)).clamp(0., 1.)
        }
        Segment::Circle { center, .. } => {
            let d = sub(p, center);
            d[1].atan2(d[0]).rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU
        }
        Segment::Arc { center, a, b, .. } => {
            let d = sub(p, center);
            let length = distance(p, center);
            if length > 1e-12
                && let Some(t) = parameter(s, add(center, scale(d, distance(center, a) / length)))
            {
                return t;
            }
            if distance(p, a) < distance(p, b) {
                0.
            } else {
                1.
            }
        }
    }
}
pub(crate) fn slice(s: &Segment, from: f64, to: f64) -> Segment {
    match *s {
        Segment::Line { .. } => Segment::Line {
            a: point(s, from),
            b: point(s, to),
        },
        Segment::Circle { center, .. } => Segment::Arc {
            center,
            a: point(s, from),
            b: point(s, to),
            clockwise: false,
        },
        Segment::Arc {
            center, clockwise, ..
        } => Segment::Arc {
            center,
            a: point(s, from),
            b: point(s, to),
            clockwise,
        },
    }
}
