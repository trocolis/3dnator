//! Compilação da guia em coordenadas da peça. Linhas/arcos mantêm geometria
//! exata; a guia automática é uma Bézier cúbica tangente aos dois perfis.
use nator_document::{Document, DocumentError, PartId, features::SweepRoute};
use nator_geometry::{Point3, add, cross, dot, norm, sub};
use nator_kernel::{GuideCurve, Solid};
use nator_sketch::{
    Plane, Sketch,
    profiles::{Region, Segment},
};
fn error(message: impl ToString) -> DocumentError {
    DocumentError(message.to_string())
}
fn unit(v: Point3) -> Result<Point3, DocumentError> {
    let length = norm(v);
    if !length.is_finite() || length < 1e-9 {
        return Err(error("Direção da trajetória inválida."));
    }
    Ok(v.map(|x| x / length))
}
fn plane_distance(plane: Plane, point: Point3) -> f64 {
    dot(sub(point, plane.origin), plane.normal()).abs()
}
fn tangent(plane: Plane, v: [f64; 2]) -> Point3 {
    std::array::from_fn(|i| plane.u[i] * v[0] + plane.v[i] * v[1])
}
fn checked_endpoints(
    first: Plane,
    last: Plane,
    start: Point3,
    end: Point3,
    a: Point3,
    b: Point3,
) -> Result<(), DocumentError> {
    if plane_distance(first, start) > 1e-5 || plane_distance(last, end) > 1e-5 {
        return Err(error(
            "A trajetória deve começar e terminar nos planos das áreas selecionadas.",
        ));
    }
    if dot(unit(a)?, first.normal()).abs() < 1. - 1e-5
        || dot(unit(b)?, last.normal()).abs() < 1. - 1e-5
    {
        return Err(error(
            "Os planos das áreas devem ser perpendiculares à trajetória nas extremidades.",
        ));
    }
    Ok(())
}
pub(super) struct Path {
    pub curves: Vec<GuideCurve>,
    pub start: Plane,
    pub end: Plane,
    pub smooth: bool,
}

pub(super) fn path(
    sketch: &Sketch,
    ids: &[u64],
    planes: Option<(Plane, Plane)>,
) -> Result<Path, DocumentError> {
    if ids.is_empty() || ids.len() > 1024 {
        return Err(error("Escolha uma cadeia aberta de até 1024 curvas."));
    }
    let mut segments = ids
        .iter()
        .map(|id| {
            let entity = sketch.entity(*id).map_err(error)?;
            if entity.infinite {
                return Err(error("Uma trajetória não pode conter retas infinitas."));
            }
            nator_sketch::profiles::segment(sketch, entity).map_err(error)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let ends = |s: &Segment| -> Result<([f64; 2], [f64; 2]), DocumentError> {
        match *s {
            Segment::Line { a, b } | Segment::Arc { a, b, .. } => Ok((a, b)),
            Segment::Circle { .. } => Err(error(
                "Use uma cadeia aberta de linhas/arcos entre os perfis, sem círculos fechados.",
            )),
        }
    };
    let all = segments.iter().map(ends).collect::<Result<Vec<_>, _>>()?;
    let mut terminals = vec![];
    for point in all.iter().flat_map(|(a, b)| [*a, *b]) {
        let count = all
            .iter()
            .flat_map(|(a, b)| [*a, *b])
            .filter(|p| nator_sketch::distance(*p, point) < 1e-6)
            .count();
        if count == 1 {
            terminals.push(point);
        } else if count > 2 {
            return Err(error(
                "A trajetória tem uma ramificação; escolha uma única cadeia.",
            ));
        }
    }
    if terminals.len() != 2 {
        return Err(error(
            "A trajetória precisa ter duas extremidades, sem ciclos ou trechos soltos.",
        ));
    }
    let world = |p| sketch.plane.world(p);
    let reverse = planes.is_some_and(|(first, last)| {
        let direct =
            plane_distance(first, world(terminals[0])) + plane_distance(last, world(terminals[1]));
        let reverse =
            plane_distance(first, world(terminals[1])) + plane_distance(last, world(terminals[0]));
        reverse < direct
    });
    let mut point = terminals[usize::from(reverse)];
    let start = point;
    let mut ordered = vec![];
    while !segments.is_empty() {
        let index = segments
            .iter()
            .position(|s| {
                ends(s).is_ok_and(|(a, b)| {
                    nator_sketch::distance(a, point) < 1e-6
                        || nator_sketch::distance(b, point) < 1e-6
                })
            })
            .ok_or_else(|| error("Há trechos desconectados na trajetória."))?;
        let mut segment = segments.remove(index);
        if nator_sketch::distance(ends(&segment)?.0, point) > 1e-6 {
            segment = segment.reversed();
        }
        point = ends(&segment)?.1;
        ordered.push(segment);
    }
    let direction = |s: &Segment, at_end: bool| -> Point3 {
        match *s {
            Segment::Line { a, b } => sub(world(b), world(a)),
            Segment::Arc {
                a,
                b,
                center,
                clockwise,
            } => {
                let radius = nator_sketch::sub(if at_end { b } else { a }, center);
                tangent(
                    sketch.plane,
                    if clockwise {
                        [radius[1], -radius[0]]
                    } else {
                        [-radius[1], radius[0]]
                    },
                )
            }
            Segment::Circle { .. } => [0.; 3],
        }
    };
    let a = ordered.first().ok_or_else(|| error("Trajetória vazia."))?;
    let b = ordered.last().ok_or_else(|| error("Trajetória vazia."))?;
    if let Some((first, last)) = planes {
        checked_endpoints(
            first,
            last,
            world(start),
            world(point),
            direction(a, false),
            direction(b, true),
        )?;
    }
    let start = Plane::from_normal(world(start), direction(a, false)).map_err(error)?;
    let end = Plane::from_normal(world(point), direction(b, true)).map_err(error)?;
    let mut smooth = true;
    for pair in ordered.windows(2) {
        smooth &= dot(
            unit(direction(&pair[0], true))?,
            unit(direction(&pair[1], false))?,
        ) > 1. - 1e-8;
    }
    let curves = ordered
        .iter()
        .map(|s| {
            Ok(match *s {
                Segment::Line { a, b } => GuideCurve::Line {
                    start: world(a),
                    end: world(b),
                },
                Segment::Arc { .. } => {
                    let samples = s.sample(4);
                    GuideCurve::Arc {
                        start: world(samples[0]),
                        middle: world(samples[samples.len() / 2]),
                        end: world(samples[samples.len() - 1]),
                    }
                }
                Segment::Circle { .. } => {
                    return Err(error("Uma trajetória entre perfis precisa ser aberta."));
                }
            })
        })
        .collect::<Result<_, DocumentError>>()?;
    Ok(Path {
        curves,
        start,
        end,
        smooth,
    })
}
pub(super) fn curves(
    doc: &Document,
    part: PartId,
    first: (&Sketch, &Region),
    last: (&Sketch, &Region),
    route: &SweepRoute,
) -> Result<Vec<GuideCurve>, DocumentError> {
    let a = Solid::profile_center(first.0, first.1).map_err(error)?;
    let b = Solid::profile_center(last.0, last.1).map_err(error)?;
    match route {
        SweepRoute::Sketch { sketch, entities } => path(
            crate::features::sketch_of(doc, part, *sketch)?,
            entities,
            Some((first.0.plane, last.0.plane)),
        )
        .map(|path| path.curves),
        SweepRoute::Automatic => {
            let chord = sub(b, a);
            let length = norm(chord);
            if length < 1e-7 {
                return Err(error(
                    "As áreas precisam ter centros distintos para uma guia automática.",
                ));
            }
            let direction = |normal: Point3| {
                if dot(normal, chord) < 0. {
                    normal.map(|v| -v)
                } else {
                    normal
                }
            };
            let start = direction(first.0.plane.normal());
            let end = direction(last.0.plane.normal());
            Ok(vec![GuideCurve::Cubic {
                poles: [
                    a,
                    add(a, start.map(|v| v * length * 0.55)),
                    sub(b, end.map(|v| v * length * 0.55)),
                    b,
                ],
            }])
        }
        SweepRoute::Axis {
            sketch,
            line,
            reverse,
        } => {
            let sketch = crate::features::sketch_of(doc, part, *sketch)?;
            nator_document::features::validate_coplanar(first.0.plane, sketch.plane)?;
            let (u, v) = sketch.line(*line).map_err(error)?;
            let origin = sketch.plane.world(u);
            let axis = unit(sub(sketch.plane.world(v), origin))?;
            let ca = dot(sub(a, origin), axis);
            let cb = dot(sub(b, origin), axis);
            let center = add(origin, axis.map(|x| x * ca));
            let ra = sub(a, center);
            let rb = sub(b, add(origin, axis.map(|x| x * cb)));
            let radius = norm(ra);
            if radius < 1e-7
                || (radius - norm(rb)).abs() > radius.max(1.) * 1e-5
                || (ca - cb).abs() > 1e-5
            {
                return Err(error(
                    "Para usar um eixo, os centros das áreas devem ter o mesmo raio e a mesma posição ao longo dele.",
                ));
            }
            let mut angle = dot(axis, cross(ra, rb)).atan2(dot(ra, rb));
            if angle.abs() < 1e-7 {
                return Err(error("O arco da trajetória precisa ter ângulo não nulo."));
            }
            if *reverse {
                angle -= angle.signum() * std::f64::consts::TAU;
            }
            let rotated = add(
                ra.map(|x| x * (angle * 0.5).cos()),
                cross(axis, ra).map(|x| x * (angle * 0.5).sin()),
            );
            checked_endpoints(
                first.0.plane,
                last.0.plane,
                a,
                b,
                cross(axis, ra),
                cross(axis, rb),
            )?;
            Ok(vec![GuideCurve::Arc {
                start: a,
                middle: add(center, rotated),
                end: b,
            }])
        }
    }
}
