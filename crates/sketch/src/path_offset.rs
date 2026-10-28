//! Deslocamento de cadeias abertas/fechadas. As curvas são orientadas ao longo
//! do caminho; junções são aparadas nas interseções de seus suportes exatos.
use crate::{profiles::Segment, *};
const EPS: f64 = 1e-6;

fn endpoints(segment: &Segment) -> Option<[Point2; 2]> {
    match *segment {
        Segment::Line { a, b } | Segment::Arc { a, b, .. } => Some([a, b]),
        _ => None,
    }
}
fn support(segment: &Segment) -> Segment {
    match *segment {
        Segment::Arc { center, a, .. } => Segment::Circle {
            center,
            radius: distance(center, a),
        },
        _ => segment.clone(),
    }
}
fn shifted(segment: &Segment, offset: f64) -> Result<Segment> {
    Ok(match *segment {
        Segment::Line { a, b } => {
            let d = sub(b, a);
            let n = scale([-d[1], d[0]], offset / distance(a, b));
            Segment::Line {
                a: add(a, n),
                b: add(b, n),
            }
        }
        Segment::Arc {
            center,
            a,
            b,
            clockwise,
        } => {
            let old = distance(center, a);
            let radius = old + if clockwise { offset } else { -offset };
            if radius <= 1e-9 {
                return Err(SketchError(
                    "O deslocamento ultrapassa o raio do contorno.".into(),
                ));
            }
            Segment::Arc {
                center,
                a: add(center, scale(sub(a, center), radius / old)),
                b: add(center, scale(sub(b, center), radius / old)),
                clockwise,
            }
        }
        Segment::Circle { .. } => {
            return Err(SketchError(
                "Um círculo completo deve ser deslocado separadamente.".into(),
            ));
        }
    })
}
fn set_end(segment: &mut Segment, end: usize, point: Point2) {
    match segment {
        Segment::Line { a, b } | Segment::Arc { a, b, .. } => {
            if end == 0 {
                *a = point;
            } else {
                *b = point;
            }
        }
        _ => {}
    }
}

pub(crate) fn append(sketch: &mut Sketch, selected: &[u64], offset: f64) -> Result<Vec<u64>> {
    let source = sketch.clone();
    let mut remaining = selected.to_vec();
    let mut created = Vec::new();
    while let Some(seed) = remaining.first().copied() {
        let entity = source.entity(seed)?;
        if entity.infinite || matches!(entity.curve, Curve::Circle { .. }) {
            created.push(tools::offset_entity(sketch, seed, offset)?);
            remaining.remove(0);
            continue;
        }
        let mut component = vec![seed];
        loop {
            let before = component.len();
            let ends = component
                .iter()
                .map(|id| profiles::segment(&source, source.entity(*id)?))
                .collect::<Result<Vec<_>>>()?
                .iter()
                .filter_map(endpoints)
                .flatten()
                .collect::<Vec<_>>();
            for id in &remaining {
                let e = source.entity(*id)?;
                if !e.infinite
                    && !component.contains(id)
                    && endpoints(&profiles::segment(&source, e)?).is_some_and(|p| {
                        p.iter()
                            .any(|p| ends.iter().any(|q| distance(*p, *q) < EPS))
                    })
                {
                    component.push(*id);
                }
            }
            if component.len() == before {
                break;
            }
        }
        remaining.retain(|id| !component.contains(id));
        if component.len() == 1 {
            created.push(tools::offset_entity(sketch, seed, offset)?);
            continue;
        }
        let originals = component
            .iter()
            .map(|id| profiles::segment(&source, source.entity(*id)?))
            .collect::<Result<Vec<_>>>()?;
        let all_ends = originals
            .iter()
            .filter_map(endpoints)
            .flatten()
            .collect::<Vec<_>>();
        let degree = |p: Point2| all_ends.iter().filter(|q| distance(p, **q) < EPS).count();
        if all_ends.iter().any(|p| degree(*p) > 2) {
            return Err(SketchError(
                "Escolha um trecho sem ramificações para deslocar.".into(),
            ));
        }
        let first_ends =
            endpoints(&originals[0]).ok_or_else(|| SketchError("Contorno inválido.".into()))?;
        let start = all_ends
            .iter()
            .find(|p| degree(**p) == 1)
            .copied()
            .unwrap_or(first_ends[0]);
        let mut point = start;
        let mut unused = (0..originals.len()).collect::<Vec<_>>();
        let mut ordered = Vec::new();
        while !unused.is_empty() {
            let index = unused
                .iter()
                .position(|i| {
                    endpoints(&originals[*i])
                        .is_some_and(|ends| ends.iter().any(|p| distance(*p, point) < EPS))
                })
                .ok_or_else(|| SketchError("O trecho selecionado não é contínuo.".into()))?;
            let index = unused.remove(index);
            let mut segment = originals[index].clone();
            let ends =
                endpoints(&segment).ok_or_else(|| SketchError("Contorno inválido.".into()))?;
            if distance(ends[0], point) >= EPS {
                segment = segment.reversed();
            }
            point = endpoints(&segment)
                .map(|e| e[1])
                .ok_or_else(|| SketchError("Contorno inválido.".into()))?;
            ordered.push(segment);
        }
        let closed = distance(point, start) < EPS;
        let mut output = ordered
            .iter()
            .map(|s| shifted(s, offset))
            .collect::<Result<Vec<_>>>()?;
        let count = output.len();
        for i in 0..if closed { count } else { count - 1 } {
            let j = (i + 1) % count;
            let a = endpoints(&output[i])
                .map(|e| e[1])
                .ok_or_else(|| SketchError("Contorno inválido.".into()))?;
            let b = endpoints(&output[j])
                .map(|e| e[0])
                .ok_or_else(|| SketchError("Contorno inválido.".into()))?;
            let intersection = if distance(a, b) < EPS {
                scale(add(a, b), 0.5)
            } else {
                let reference = endpoints(&ordered[i])
                    .map(|e| e[1])
                    .ok_or_else(|| SketchError("Contorno inválido.".into()))?;
                crate::curve_geometry::intersections_with_infinite(
                    &support(&output[i]),
                    &support(&output[j]),
                    true,
                    true,
                )
                .into_iter()
                .min_by(|a, b| distance(*a, reference).total_cmp(&distance(*b, reference)))
                .ok_or_else(|| {
                    SketchError("As curvas deslocadas não formam uma junção válida.".into())
                })?
            };
            set_end(&mut output[i], 1, intersection);
            set_end(&mut output[j], 0, intersection);
        }
        let mut points: Vec<(Point2, u64)> = Vec::new();
        for (original, segment) in ordered.iter().zip(output) {
            if let (Segment::Line { a, b }, Segment::Line { a: c, b: d }) = (original, &segment)
                && dot(sub(*b, *a), sub(*d, *c)) <= 1e-9
            {
                return Err(SketchError(
                    "O deslocamento faria o contorno se inverter.".into(),
                ));
            }
            let mut point_id = |position: Point2, sketch: &mut Sketch| -> Result<u64> {
                if let Some((_, id)) = points.iter().find(|(p, _)| distance(*p, position) < EPS) {
                    return Ok(*id);
                }
                let id = sketch.add_point(position)?;
                points.push((position, id));
                Ok(id)
            };
            let curve = match segment {
                Segment::Line { a, b } => Curve::Line {
                    start: point_id(a, sketch)?,
                    end: point_id(b, sketch)?,
                },
                Segment::Arc {
                    center,
                    a,
                    b,
                    clockwise,
                } => Curve::Arc {
                    center: sketch.add_point(center)?,
                    start: point_id(a, sketch)?,
                    end: point_id(b, sketch)?,
                    clockwise,
                },
                Segment::Circle { .. } => return Err(SketchError("Contorno inválido.".into())),
            };
            created.push(sketch.add_curve(curve, false)?);
        }
    }
    Ok(created)
}
