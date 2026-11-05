//! Medições: consultas ao documento e ao núcleo que nunca alteram a geometria.
//!
//! Sólidos CAD são medidos sobre o B-rep exato; corpos de malha, sobre a malha, com o
//! resultado marcado como aproximado. Coordenadas são as da peça (origem local), como na
//! construção; quem mede em contexto de montagem compõe a posição da instância.
//! Sem materiais definidos, o centro de massa supõe densidade uniforme.
mod sketch;
pub use sketch::sketch_entity;

use nator_document::{BodyId, Document, DocumentError, assembly::Pick};
use nator_geometry::{Point3, cross, dot, norm, sub};

/// Volume, área e centro de massa de um corpo em coordenadas da peça.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyProperties {
    pub volume_mm3: f64,
    pub area_mm2: f64,
    pub center_mm: Point3,
    /// Medido no B-rep exato (sólido CAD) em vez da malha de exibição.
    pub exact: bool,
}

/// Medida de uma face ou aresta selecionada em um sólido.
#[derive(Debug, Clone, PartialEq)]
pub enum Topology {
    Face {
        name: String,
        area_mm2: f64,
        /// Raio de faces cilíndricas, esféricas ou toroidais (raio maior).
        radius_mm: Option<f64>,
        planar: bool,
    },
    Edge {
        faces: [String; 2],
        length_mm: f64,
        radius_mm: Option<f64>,
    },
}

/// Plano exato da face clicada, em coordenadas de montagem. A captura é
/// independente do corpo de origem: alterações futuras não criam vínculo externo.
#[cfg(feature = "occt")]
pub fn sketch_plane(doc: &Document, pick: &Pick) -> Result<nator_sketch::Plane, DocumentError> {
    let body = doc
        .body(pick.body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let bytes = body.brep.as_ref().ok_or_else(|| {
        DocumentError(
            "Selecione uma face plana de um sólido CAD; malhas não definem planos exatos.".into(),
        )
    })?;
    // O suporte guarda somente o plano, sem referência associativa. Faces de transição
    // do chanfro podem não ter nome (ou compartilhar o mesmo papel); isso não impede
    // consultar sua superfície exata. O índice vem da malha desta mesma revisão e nunca
    // é persistido. Consultamos antes de transformar para preservar a correspondência
    // entre os índices da tesselação e os do B-rep gravado, inclusive em projetos antigos.
    let index = doc.face_index_at(pick)?;
    let solid = nator_kernel::Solid::from_brep(bytes, None).map_err(error)?;
    let measured = solid.face_measure(index).map_err(error)?;
    let nator_kernel::FaceKind::Plane { normal, offset } = measured.kind else {
        return Err(DocumentError(
            "A face escolhida é curva. Selecione uma face plana ou um plano da origem.".into(),
        ));
    };
    let placement = pick.placement.compose(body.placement).map_err(error)?;
    let local = placement.inverse_apply(pick.point);
    let distance = dot(normal, local) - offset;
    let origin = std::array::from_fn(|i| local[i] - normal[i] * distance);
    let mut world_normal = placement.apply_vector(normal);
    if dot(world_normal, pick.normal) < 0. {
        world_normal = world_normal.map(|v| -v);
    }
    nator_sketch::Plane::from_normal(placement.apply(origin), world_normal).map_err(error)
}
#[cfg(not(feature = "occt"))]
pub fn sketch_plane(_doc: &Document, _pick: &Pick) -> Result<nator_sketch::Plane, DocumentError> {
    Err(DocumentError(
        "Seleção de face plana requer o núcleo OCCT.".into(),
    ))
}

/// Ângulo entre duas faces clicadas e, se forem paralelas, a distância entre elas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacePair {
    pub angle_rad: f64,
    pub parallel_distance_mm: Option<f64>,
}

fn error(e: impl std::fmt::Display) -> DocumentError {
    DocumentError(e.to_string())
}

/// Propriedades do corpo. Malhas precisam ser fechadas para ter volume significativo.
pub fn body_properties(doc: &Document, body: BodyId) -> Result<BodyProperties, DocumentError> {
    let record = doc
        .body(body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    #[cfg(feature = "occt")]
    if let Some(bytes) = &record.brep {
        let properties = nator_kernel::Solid::from_brep(bytes, None)
            .and_then(|s| s.transformed(record.placement))
            .and_then(|s| s.properties())
            .map_err(error)?;
        return Ok(BodyProperties {
            volume_mm3: properties.volume,
            area_mm2: properties.area,
            center_mm: properties.center,
            exact: true,
        });
    }
    let mesh = record.geometry.mesh();
    if !record.geometry.topology().has_closed_oriented_edges() {
        return Err(DocumentError(
            "A malha está aberta ou com orientação inconsistente; volume e centro de massa não têm significado."
                .into(),
        ));
    }
    let (center, volume) = mesh.mass_center().map_err(error)?;
    let area = mesh
        .triangles()
        .iter()
        .map(|t| {
            let [a, b, c] = mesh.triangle_points(*t);
            norm(cross(sub(b, a), sub(c, a))) / 2.
        })
        .sum();
    Ok(BodyProperties {
        volume_mm3: volume,
        area_mm2: area,
        center_mm: record.placement.apply(center),
        exact: false,
    })
}

/// Face ou aresta sob o clique, medida no B-rep exato.
#[cfg(feature = "occt")]
pub fn topology(doc: &Document, pick: &Pick, edge: bool) -> Result<Topology, DocumentError> {
    use nator_document::features::{EdgeRef, FaceRef};
    use nator_kernel::FaceKind;
    let record = doc
        .body(pick.body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let bytes = record
        .brep
        .as_ref()
        .ok_or_else(|| DocumentError("Medidas de faces e arestas exigem sólido CAD.".into()))?;
    let solid = nator_kernel::Solid::from_brep(bytes, Some(&record.face_names))
        .and_then(|s| s.transformed(record.placement))
        .map_err(error)?;
    if edge {
        let reference = crate::topology::edge_at(doc, pick)?;
        let index = match reference {
            EdgeRef::Between(ref names) => solid.edge_between(names).map_err(error)?,
            EdgeRef::Index { index, .. } => index,
        };
        let faces = solid
            .edge_face_names(index)
            .unwrap_or_else(|_| [format!("#{}", index + 1), String::new()]);
        let measure = solid.edge_measure(index).map_err(error)?;
        Ok(Topology::Edge {
            faces,
            length_mm: measure.length,
            radius_mm: measure.radius,
        })
    } else {
        let (index, name) = match doc.face_at(pick)? {
            FaceRef::Named(name) => (solid.face_named(&name).map_err(error)?, name),
            FaceRef::Index { index, .. } => (index, format!("#{}", index + 1)),
        };
        let measure = solid.face_measure(index).map_err(error)?;
        let (radius_mm, planar) = match measure.kind {
            FaceKind::Plane { .. } => (None, true),
            FaceKind::Cylinder { radius } | FaceKind::Sphere { radius } => (Some(radius), false),
            FaceKind::Torus { major_radius } => (Some(major_radius), false),
            FaceKind::Cone | FaceKind::Other => (None, false),
        };
        Ok(Topology::Face {
            name,
            area_mm2: measure.area,
            radius_mm,
            planar,
        })
    }
}

#[cfg(not(feature = "occt"))]
pub fn topology(_doc: &Document, _pick: &Pick, _edge: bool) -> Result<Topology, DocumentError> {
    nator_kernel::require_available().map_err(error)?;
    Err(DocumentError(
        "Medição de faces e arestas indisponível.".into(),
    ))
}

/// Ângulo entre as normais de duas faces clicadas (coordenadas de mundo). Faces paralelas,
/// dentro de 0,01°, informam também a distância entre os planos que passam pelos cliques.
pub fn face_pair(first: &Pick, second: &Pick) -> Result<FacePair, DocumentError> {
    let (a, b) = (norm(first.normal), norm(second.normal));
    if a < 1e-12 || b < 1e-12 {
        return Err(DocumentError("Face sem normal definida.".into()));
    }
    let n = first.normal.map(|v| v / a);
    let cosine = (dot(n, second.normal) / b).clamp(-1., 1.);
    let angle_rad = cosine.acos();
    let parallel = angle_rad < 0.01_f64.to_radians() || angle_rad > (180. - 0.01_f64).to_radians();
    Ok(FacePair {
        angle_rad,
        parallel_distance_mm: parallel.then(|| dot(sub(second.point, first.point), n).abs()),
    })
}

/// Menor distância entre os sólidos de dois cliques, em coordenadas de mundo.
#[cfg(feature = "occt")]
pub fn body_distance(doc: &Document, first: &Pick, second: &Pick) -> Result<f64, DocumentError> {
    let solid = |pick: &Pick| -> Result<nator_kernel::Solid, DocumentError> {
        let record = doc
            .body(pick.body)
            .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
        let bytes = record
            .brep
            .as_ref()
            .ok_or_else(|| DocumentError("Distância entre corpos exige sólidos CAD.".into()))?;
        let pose = pick.placement.compose(record.placement).map_err(error)?;
        nator_kernel::Solid::from_brep(bytes, None)
            .and_then(|s| s.transformed(pose))
            .map_err(error)
    };
    Ok(solid(first)?.distance(&solid(second)?).map_err(error)?.0)
}

#[cfg(not(feature = "occt"))]
pub fn body_distance(_doc: &Document, _first: &Pick, _second: &Pick) -> Result<f64, DocumentError> {
    nator_kernel::require_available().map_err(error)?;
    Err(DocumentError("Distância entre corpos indisponível.".into()))
}

/// Elemento escolhido pela ferramenta Medir, em coordenadas de mundo. Só guarda a
/// geometria necessária para medir e desenhar a cota; não há vínculo com o corpo.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Vertex {
        point: Point3,
    },
    Edge {
        /// Polilinha da aresta; uma reta tem só os dois extremos.
        points: Vec<Point3>,
        length_mm: f64,
        /// Centro e raio de arestas circulares.
        circle: Option<(Point3, f64)>,
    },
    Face {
        /// Ponto clicado sobre a face.
        point: Point3,
        normal: Point3,
        planar: bool,
        area_mm2: Option<f64>,
        radius_mm: Option<f64>,
    },
}
impl Target {
    /// Ponto usado para posicionar o realce e o texto.
    pub fn anchor(&self) -> Point3 {
        match self {
            Self::Vertex { point } | Self::Face { point, .. } => *point,
            Self::Edge { points, .. } => along(points, 0.5),
        }
    }
    fn straight(&self) -> Option<(Point3, Point3)> {
        match self {
            Self::Edge {
                points,
                circle: None,
                ..
            } if points.len() == 2 => Some((points[0], points[1])),
            _ => None,
        }
    }
    fn plane(&self) -> Option<(Point3, Point3)> {
        match self {
            Self::Face {
                point,
                normal,
                planar: true,
                ..
            } => unit(*normal).map(|n| (*point, n)),
            _ => None,
        }
    }
}

/// Uma leitura desenhada na cena. Comprimentos em mm e ângulos em radianos.
#[derive(Debug, Clone, PartialEq)]
pub enum Reading {
    /// Coordenadas de um vértice.
    Point {
        at: Point3,
    },
    Length {
        mm: f64,
        path: Vec<Point3>,
    },
    Radius {
        mm: f64,
        center: Point3,
        at: Point3,
    },
    Area {
        mm2: f64,
        at: Point3,
    },
    Distance {
        mm: f64,
        from: Point3,
        to: Point3,
    },
    /// Componentes de uma distância nos três eixos principais.
    Axes {
        from: Point3,
        to: Point3,
    },
    Angle {
        rad: f64,
        at: Point3,
    },
}

/// Abaixo disto (0,01°), duas direções são paralelas.
const PARALLEL_RAD: f64 = 0.01 * std::f64::consts::PI / 180.;

fn unit(v: Point3) -> Option<Point3> {
    let length = norm(v);
    (length > 1e-12).then(|| v.map(|c| c / length))
}

fn add(a: Point3, b: Point3) -> Point3 {
    std::array::from_fn(|i| a[i] + b[i])
}

fn scale(a: Point3, k: f64) -> Point3 {
    a.map(|v| v * k)
}

/// Ponto a uma fração do comprimento de uma polilinha.
fn along(points: &[Point3], fraction: f64) -> Point3 {
    let total = points
        .windows(2)
        .map(|w| norm(sub(w[1], w[0])))
        .sum::<f64>();
    let mut left = total * fraction;
    for w in points.windows(2) {
        let length = norm(sub(w[1], w[0]));
        if left <= length && length > 0. {
            return add(w[0], scale(sub(w[1], w[0]), left / length));
        }
        left -= length;
    }
    points.last().copied().unwrap_or_default()
}

/// Ponto mais próximo de `p` no segmento `a`–`b`.
fn on_segment(p: Point3, a: Point3, b: Point3) -> Point3 {
    let d = sub(b, a);
    let length = dot(d, d);
    if length < 1e-24 {
        return a;
    }
    add(a, scale(d, (dot(sub(p, a), d) / length).clamp(0., 1.)))
}

/// Pontos mais próximos entre dois segmentos (busca ternária dupla não é necessária:
/// a solução fechada clássica, com os casos degenerados presos aos extremos).
fn segments(a0: Point3, a1: Point3, b0: Point3, b1: Point3) -> (Point3, Point3) {
    let (d1, d2, r) = (sub(a1, a0), sub(b1, b0), sub(a0, b0));
    let (a, e, f) = (dot(d1, d1), dot(d2, d2), dot(d2, r));
    let (s, t) = if a < 1e-24 && e < 1e-24 {
        (0., 0.)
    } else if a < 1e-24 {
        (0., (f / e).clamp(0., 1.))
    } else {
        let c = dot(d1, r);
        if e < 1e-24 {
            ((-c / a).clamp(0., 1.), 0.)
        } else {
            let b = dot(d1, d2);
            let denominator = a * e - b * b;
            let mut s = if denominator > 1e-24 {
                ((b * f - c * e) / denominator).clamp(0., 1.)
            } else {
                0.
            };
            let mut t = (b * s + f) / e;
            if t < 0. {
                t = 0.;
                s = (-c / a).clamp(0., 1.);
            } else if t > 1. {
                t = 1.;
                s = ((b - c) / a).clamp(0., 1.);
            }
            (s, t)
        }
    };
    (add(a0, scale(d1, s)), add(b0, scale(d2, t)))
}

/// Pares de pontos mais próximos entre os elementos, sem faces.
fn closest_points(first: &Target, second: &Target) -> (Point3, Point3) {
    let pieces = |t: &Target| -> Vec<(Point3, Point3)> {
        match t {
            Target::Vertex { point } | Target::Face { point, .. } => vec![(*point, *point)],
            Target::Edge { points, .. } if points.len() < 2 => {
                points.first().map(|p| vec![(*p, *p)]).unwrap_or_default()
            }
            Target::Edge { points, .. } => points.windows(2).map(|w| (w[0], w[1])).collect(),
        }
    };
    let mut best = (f64::INFINITY, ([0.; 3], [0.; 3]));
    for (a0, a1) in pieces(first) {
        for (b0, b1) in pieces(second) {
            let (p, q) = segments(a0, a1, b0, b1);
            let gap = norm(sub(q, p));
            if gap < best.0 {
                best = (gap, (p, q));
            }
        }
    }
    best.1
}

/// Ângulo agudo entre duas direções sem sentido (arestas), em radianos.
fn acute(a: Point3, b: Point3) -> Option<f64> {
    let (a, b) = (unit(a)?, unit(b)?);
    Some(dot(a, b).abs().clamp(0., 1.).acos())
}

/// Leituras de um elemento ou de um par. Com `axes`, a distância do par também é
/// decomposta nos eixos X, Y e Z (duplo clique no segundo alvo).
pub fn readings(first: &Target, second: Option<&Target>, axes: bool) -> Vec<Reading> {
    let Some(second) = second else {
        return match first {
            Target::Vertex { point } => vec![Reading::Point { at: *point }],
            Target::Edge {
                points,
                length_mm,
                circle,
            } => {
                let mut result = vec![Reading::Length {
                    mm: *length_mm,
                    path: points.clone(),
                }];
                if let Some((center, radius)) = circle {
                    result.push(Reading::Radius {
                        mm: *radius,
                        center: *center,
                        at: along(points, 0.5),
                    });
                }
                result
            }
            Target::Face {
                point,
                area_mm2,
                radius_mm,
                ..
            } => area_mm2
                .map(|mm2| Reading::Area { mm2, at: *point })
                .into_iter()
                .chain(radius_mm.map(|mm| Reading::Radius {
                    mm,
                    center: *point,
                    at: *point,
                }))
                .collect(),
        };
    };
    let distance = |from: Point3, to: Point3| {
        let mut result = vec![Reading::Distance {
            mm: norm(sub(to, from)),
            from,
            to,
        }];
        if axes {
            result.push(Reading::Axes { from, to });
        }
        result
    };
    // Plano e o que medir até ele: ponto (vértice/face curva) ou aresta.
    let to_plane = |other: &Target, (origin, normal): (Point3, Point3), flip: bool| {
        let foot = |p: Point3| sub(p, scale(normal, dot(sub(p, origin), normal)));
        let from = match other {
            Target::Edge { points, .. } => points
                .iter()
                .copied()
                .min_by(|a, b| {
                    dot(sub(*a, origin), normal)
                        .abs()
                        .total_cmp(&dot(sub(*b, origin), normal).abs())
                })
                .unwrap_or_else(|| other.anchor()),
            _ => other.anchor(),
        };
        let to = foot(from);
        if flip {
            distance(to, from)
        } else {
            distance(from, to)
        }
    };
    match (
        first.straight(),
        second.straight(),
        first.plane(),
        second.plane(),
    ) {
        // Duas retas: paralelas medem a distância entre elas; senão, o ângulo.
        (Some((a0, a1)), Some((b0, b1)), _, _) => {
            let angle = acute(sub(a1, a0), sub(b1, b0)).unwrap_or(0.);
            if angle < PARALLEL_RAD {
                let middle = scale(add(a0, a1), 0.5);
                let d = sub(b1, b0);
                let foot = add(b0, scale(d, dot(sub(middle, b0), d) / dot(d, d)));
                distance(middle, foot)
            } else {
                let (p, q) = segments(a0, a1, b0, b1);
                vec![Reading::Angle {
                    rad: angle,
                    at: scale(add(p, q), 0.5),
                }]
            }
        }
        // Dois planos: paralelos medem a distância; senão, o ângulo entre as normais.
        (_, _, Some(a), Some(b)) => {
            let angle = dot(a.1, b.1).clamp(-1., 1.).acos();
            if angle < PARALLEL_RAD || std::f64::consts::PI - angle < PARALLEL_RAD {
                to_plane(first, b, false)
            } else {
                vec![Reading::Angle {
                    rad: angle,
                    at: scale(add(a.0, b.0), 0.5),
                }]
            }
        }
        // Reta e plano: paralela mede a distância; senão, o ângulo com o plano.
        (Some((a0, a1)), _, _, Some(plane)) | (_, Some((a0, a1)), Some(plane), _) => {
            let along_normal = unit(sub(a1, a0)).map_or(0., |d| dot(d, plane.1).abs());
            let angle = along_normal.clamp(0., 1.).asin();
            if angle < PARALLEL_RAD {
                let edge = if first.straight().is_some() {
                    first
                } else {
                    second
                };
                to_plane(edge, plane, edge != first)
            } else {
                vec![Reading::Angle {
                    rad: angle,
                    at: scale(add(a0, a1), 0.5),
                }]
            }
        }
        (_, _, None, Some(plane)) => to_plane(first, plane, false),
        (_, _, Some(plane), None) => to_plane(second, plane, true),
        _ => {
            let (from, to) = closest_points(first, second);
            distance(from, to)
        }
    }
}

/// Elemento sob o clique: vértice a menos de `reach` mm, senão aresta a menos de
/// `reach`, senão a face. Sólidos usam o B-rep exato; malhas, o triângulo atingido.
pub fn target_at(doc: &Document, pick: &Pick, reach: f64) -> Result<Target, DocumentError> {
    let record = doc
        .body(pick.body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    if record.brep.is_some() {
        return solid_target(doc, pick, reach);
    }
    // Malha: vértices e lados do triângulo atingido, em coordenadas de mundo.
    let near = |p: Point3| norm(sub(p, pick.point));
    if let Some(point) = pick
        .vertices
        .iter()
        .copied()
        .filter(|p| near(*p) <= reach)
        .min_by(|a, b| near(*a).total_cmp(&near(*b)))
    {
        return Ok(Target::Vertex { point });
    }
    let [a, b, c] = pick.vertices;
    if let Some((p, q)) = [(a, b), (b, c), (c, a)]
        .into_iter()
        .filter(|(p, q)| near(on_segment(pick.point, *p, *q)) <= reach)
        .min_by(|x, y| {
            near(on_segment(pick.point, x.0, x.1))
                .total_cmp(&near(on_segment(pick.point, y.0, y.1)))
        })
    {
        return Ok(Target::Edge {
            points: vec![p, q],
            length_mm: norm(sub(q, p)),
            circle: None,
        });
    }
    Ok(Target::Face {
        point: pick.point,
        normal: pick.normal,
        planar: true,
        area_mm2: None,
        radius_mm: None,
    })
}

#[cfg(feature = "occt")]
fn solid_target(doc: &Document, pick: &Pick, reach: f64) -> Result<Target, DocumentError> {
    use nator_kernel::{EdgeGeometry, FaceKind};
    let (solid, _) = crate::features::source(doc, pick.part, pick.body)?;
    // `source` devolve o sólido na peça; a ocorrência leva ao mundo.
    let world = |p: Point3| pick.placement.apply(p);
    let click = pick.placement.inverse_apply(pick.point);
    let near = |p: Point3| norm(sub(p, click));
    let mut vertex: Option<(f64, Point3)> = None;
    // Distância ao clique, índice, polilinha e círculo (centro e raio) da aresta mais próxima.
    type Near = (f64, usize, Vec<Point3>, Option<(Point3, f64)>);
    let mut edge: Option<Near> = None;
    for index in 0..solid.edge_count().map_err(error)? {
        let (points, circle, ends) = match solid.edge_geometry(index).map_err(error)? {
            EdgeGeometry::Line(a, b) => (vec![a, b], None, vec![a, b]),
            EdgeGeometry::Circle {
                center,
                radius,
                start,
                end,
                closed,
                ..
            } => {
                let points = solid.edge_polyline(index, 0.01).map_err(error)?;
                let ends = if closed { Vec::new() } else { vec![start, end] };
                (points, Some((center, radius)), ends)
            }
            EdgeGeometry::Curve(points) => {
                let ends = match (points.first(), points.last()) {
                    (Some(a), Some(b)) => vec![*a, *b],
                    _ => Vec::new(),
                };
                (points, None, ends)
            }
        };
        for end in ends {
            let gap = near(end);
            if gap <= reach && vertex.is_none_or(|(best, _)| gap < best) {
                vertex = Some((gap, end));
            }
        }
        let gap = points
            .windows(2)
            .map(|w| near(on_segment(click, w[0], w[1])))
            .fold(f64::INFINITY, f64::min);
        if gap <= reach && edge.as_ref().is_none_or(|(best, ..)| gap < *best) {
            edge = Some((gap, index, points, circle));
        }
    }
    if let Some((_, point)) = vertex {
        return Ok(Target::Vertex {
            point: world(point),
        });
    }
    if let Some((_, index, points, circle)) = edge {
        let length_mm = solid.edge_measure(index).map_err(error)?.length;
        return Ok(Target::Edge {
            points: points.into_iter().map(world).collect(),
            length_mm,
            circle: circle.map(|(center, radius)| (world(center), radius)),
        });
    }
    let index = doc.face_index_at(pick)?;
    let measured = solid.face_measure(index).map_err(error)?;
    let (planar, radius_mm) = match measured.kind {
        FaceKind::Plane { .. } => (true, None),
        FaceKind::Cylinder { radius } | FaceKind::Sphere { radius } => (false, Some(radius)),
        FaceKind::Torus { major_radius } => (false, Some(major_radius)),
        FaceKind::Cone | FaceKind::Other => (false, None),
    };
    Ok(Target::Face {
        point: pick.point,
        normal: pick.normal,
        planar,
        area_mm2: Some(measured.area),
        radius_mm,
    })
}

#[cfg(not(feature = "occt"))]
fn solid_target(_: &Document, _: &Pick, _: f64) -> Result<Target, DocumentError> {
    nator_kernel::require_available().map_err(error)?;
    Err(DocumentError("Medição de sólidos indisponível.".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nator_document::{MeshAsset, assembly::Pick};
    use nator_geometry::Placement;
    use std::{error::Error, sync::Arc};

    fn pick(point: Point3, normal: Point3) -> Pick {
        Pick {
            instance: nator_document::InstanceId(1),
            part: nator_document::PartId(1),
            body: BodyId(1),
            triangle: 0,
            point,
            normal,
            vertices: [[0.; 3]; 3],
            placement: Placement::default(),
        }
    }

    #[test]
    fn mesh_properties_are_approximate_and_follow_the_body_position() -> Result<(), Box<dyn Error>>
    {
        let mut doc = Document::new("Medição")?;
        let (_, body) = doc.add_mesh(
            None,
            "Suporte".into(),
            Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
        )?;
        doc.body_mut(body)?.placement = Placement::new([100., 0., 0.], [0.; 3])?;
        let properties = body_properties(&doc, body)?;
        assert!(!properties.exact);
        assert!((properties.volume_mm3 - 10_080.).abs() < 1e-6);
        // L de 60 × 20 × 30 com 6 de espessura: área total analítica.
        let area = 2. * 504. + 20. * (60. + 6. + 54. + 24. + 6. + 30.);
        assert!((properties.area_mm2 - area).abs() < 1e-6);
        assert!(properties.center_mm[0] > 100.);
        Ok(())
    }

    #[test]
    fn parallel_faces_report_their_distance() -> Result<(), Box<dyn Error>> {
        let pair = face_pair(
            &pick([0., 0., 0.], [0., 0., -1.]),
            &pick([5., 3., 12.], [0., 0., 1.]),
        )?;
        assert!((pair.angle_rad - std::f64::consts::PI).abs() < 1e-12);
        assert_eq!(pair.parallel_distance_mm, Some(12.));
        let square = face_pair(&pick([0.; 3], [1., 0., 0.]), &pick([0.; 3], [0., 1., 0.]))?;
        assert!((square.angle_rad - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert_eq!(square.parallel_distance_mm, None);
        Ok(())
    }

    fn edge(a: Point3, b: Point3) -> Target {
        Target::Edge {
            points: vec![a, b],
            length_mm: norm(sub(b, a)),
            circle: None,
        }
    }

    fn face(point: Point3, normal: Point3) -> Target {
        Target::Face {
            point,
            normal,
            planar: true,
            area_mm2: Some(1.),
            radius_mm: None,
        }
    }

    fn only_distance(readings: &[Reading]) -> Option<(f64, Point3, Point3)> {
        match readings {
            [Reading::Distance { mm, from, to }] => Some((*mm, *from, *to)),
            _ => None,
        }
    }

    #[test]
    fn one_element_shows_its_own_measure() {
        assert_eq!(
            readings(&edge([0.; 3], [3., 4., 0.]), None, false),
            vec![Reading::Length {
                mm: 5.,
                path: vec![[0.; 3], [3., 4., 0.]],
            }]
        );
        let point = Target::Vertex {
            point: [1., 2., 3.],
        };
        assert_eq!(
            readings(&point, None, false),
            vec![Reading::Point { at: [1., 2., 3.] }]
        );
    }

    #[test]
    fn parallel_edges_give_their_distance_and_others_the_angle() -> Result<(), Box<dyn Error>> {
        let a = edge([0., 0., 0.], [10., 0., 0.]);
        let b = edge([2., 7., 0.], [30., 7., 0.]);
        let (mm, from, to) = only_distance(&readings(&a, Some(&b), false)).ok_or("distância")?;
        assert!((mm - 7.).abs() < 1e-12, "{mm}");
        assert_eq!(from, [5., 0., 0.]);
        assert_eq!(to, [5., 7., 0.]);
        // Sem se tocar, a 30° e em outra altura: só o ângulo, sempre agudo.
        let tilted = edge(
            [0., 3., 2.],
            [
                -30_f64.to_radians().cos(),
                3. - 30_f64.to_radians().sin(),
                2.,
            ],
        );
        let [Reading::Angle { rad, .. }] = readings(&a, Some(&tilted), false)[..] else {
            return Err("ângulo esperado".into());
        };
        assert!((rad.to_degrees() - 30.).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn vertices_and_edges_measure_the_shortest_gap_and_its_axes() -> Result<(), Box<dyn Error>> {
        let a = Target::Vertex {
            point: [0., 0., 0.],
        };
        let b = Target::Vertex {
            point: [3., 4., 12.],
        };
        let pair = readings(&a, Some(&b), true);
        assert_eq!(pair.len(), 2);
        assert!(matches!(pair[0], Reading::Distance { mm, .. } if (mm - 13.).abs() < 1e-12));
        assert_eq!(
            pair[1],
            Reading::Axes {
                from: [0.; 3],
                to: [3., 4., 12.],
            }
        );
        // Vértice até aresta: o pé da perpendicular dentro do segmento.
        let line = edge([-5., 2., 0.], [5., 2., 0.]);
        let (mm, _, to) = only_distance(&readings(&a, Some(&line), false)).ok_or("distância")?;
        assert!((mm - 2.).abs() < 1e-12);
        assert_eq!(to, [0., 2., 0.]);
        Ok(())
    }

    #[test]
    fn faces_measure_plane_gaps_angles_and_edges_against_them() -> Result<(), Box<dyn Error>> {
        let bottom = face([0., 0., 0.], [0., 0., -1.]);
        let top = face([5., 3., 12.], [0., 0., 1.]);
        let (mm, ..) = only_distance(&readings(&bottom, Some(&top), false)).ok_or("distância")?;
        assert!((mm - 12.).abs() < 1e-12);
        let side = face([0., 0., 0.], [1., 0., 0.]);
        assert!(matches!(
            readings(&bottom, Some(&side), false)[..],
            [Reading::Angle { rad, .. }] if (rad - std::f64::consts::FRAC_PI_2).abs() < 1e-12
        ));
        // Aresta paralela ao plano: a distância; inclinada: o ângulo com o plano.
        let flat = edge([0., 0., 4.], [9., 0., 4.]);
        let (mm, ..) = only_distance(&readings(&flat, Some(&bottom), false)).ok_or("distância")?;
        assert!((mm - 4.).abs() < 1e-12);
        let rising = edge([0., 0., 0.], [1., 0., 1.]);
        assert!(matches!(
            readings(&bottom, Some(&rising), false)[..],
            [Reading::Angle { rad, .. }] if (rad.to_degrees() - 45.).abs() < 1e-9
        ));
        // Vértice até o plano da face.
        let corner = Target::Vertex {
            point: [7., 7., 9.],
        };
        let (mm, ..) =
            only_distance(&readings(&corner, Some(&bottom), false)).ok_or("distância")?;
        assert!((mm - 9.).abs() < 1e-12);
        Ok(())
    }
}
