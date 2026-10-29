//! Apoios do modelo projetados ortogonalmente no plano local de um rascunho.
//! Consultar no trabalhador: os apoios CAD são exatos e não uma amostra da tesselação.
use crate::point_snap::{body_anchors, reference_points};
use nator_document::{Document, DocumentError, SketchId, assembly::Occurrence};
use nator_geometry::Placement;
use nator_sketch::profiles::Segment;
use nator_sketch::{Plane, Point2};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Vertex,
    Edge,
    Plane,
}

/// Geometria já no plano local de destino. Triângulos servem apenas para captura
/// e prévia de faces (incluindo furos); as curvas inseridas vêm do B-rep exato.
#[derive(Debug, Clone)]
pub struct Element {
    pub source: SourceGeometry,
    pub kind: Kind,
    pub curves: Vec<Segment>,
    pub vertices: Vec<Point2>,
    pub triangles: Vec<[Point2; 3]>,
    /// Contornos externos/furos de áreas de outros rascunhos, pela regra par-ímpar.
    pub contours: Vec<Vec<Point2>>,
}

/// Coordenadas de montagem da origem, separadas das coordenadas locais da
/// projeção. A seleção pela origem usa a câmera atual, sem supor planos paralelos.
#[derive(Debug, Clone, Default)]
pub struct SourceGeometry {
    pub name: String,
    pub vertices: Vec<nator_geometry::Point3>,
    pub curves: Vec<Vec<nator_geometry::Point3>>,
    pub triangles: Vec<[nator_geometry::Point3; 3]>,
    pub contours: Vec<Vec<nator_geometry::Point3>>,
}
pub fn elements(
    document: &Document,
    occurrences: &[Occurrence],
    plane: Plane,
    target: Placement,
) -> Result<Vec<Element>, DocumentError> {
    elements_for_sketch(document, occurrences, plane, target, None)
}

fn inside_contours(contours: &[Vec<Point2>], p: Point2) -> bool {
    let mut inside = false;
    for points in contours {
        for (a, b) in points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .take(points.len())
        {
            if (a[1] > p[1]) != (b[1] > p[1])
                && p[0] < a[0] + (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1])
            {
                inside = !inside;
            }
        }
    }
    inside
}

/// Faces planas da malha são agrupadas pelo suporte. A contagem de incidências
/// elimina diagonais internas, inclusive quando o STL repete coordenadas. Furos
/// permanecem vazios porque a captura usa os triângulos reais da superfície.
fn mesh_areas(
    mesh: &nator_geometry::Mesh,
    project: impl Fn(nator_geometry::Point3) -> Point2,
    world: impl Fn(nator_geometry::Point3) -> nator_geometry::Point3,
    name: &str,
) -> Vec<Element> {
    type Key = [u64; 3];
    #[derive(Default)]
    struct Face {
        triangles: Vec<[Point2; 3]>,
        source_triangles: Vec<[nator_geometry::Point3; 3]>,
        edges: HashMap<(Key, Key), (usize, [nator_geometry::Point3; 2])>,
    }
    let key = |p: nator_geometry::Point3| p.map(|v| if v == 0. { 0 } else { v.to_bits() });
    let mut groups: HashMap<[i64; 4], Vec<[nator_geometry::Point3; 3]>> = HashMap::new();
    for triangle in mesh.triangles() {
        let points = mesh.triangle_points(*triangle);
        let normal = nator_geometry::cross(
            nator_geometry::sub(points[1], points[0]),
            nator_geometry::sub(points[2], points[0]),
        );
        let length = nator_geometry::norm(normal);
        if length < 1e-12 {
            continue;
        }
        let mut normal = normal.map(|v| v / length);
        if normal
            .iter()
            .find(|v| v.abs() > 1e-9)
            .is_some_and(|v| *v < 0.)
        {
            normal = normal.map(|v| -v);
        }
        let plane = [
            (normal[0] * 1e8).round() as i64,
            (normal[1] * 1e8).round() as i64,
            (normal[2] * 1e8).round() as i64,
            (nator_geometry::dot(normal, points[0]) * 1e6).round() as i64,
        ];
        groups.entry(plane).or_default().push(points);
    }
    let mut groups = groups.into_iter().collect::<Vec<_>>();
    groups.sort_by_key(|(key, _)| *key);
    fn root(parents: &mut [usize], mut index: usize) -> usize {
        while parents[index] != index {
            parents[index] = parents[parents[index]];
            index = parents[index];
        }
        index
    }
    let mut result = Vec::new();
    for (_, triangles) in groups {
        // Superfícies desconectadas no mesmo plano continuam sendo escolhas
        // independentes; compartilhar apenas um vértice não une suas áreas.
        let mut parents = (0..triangles.len()).collect::<Vec<_>>();
        let mut owners = HashMap::new();
        for (index, points) in triangles.iter().enumerate() {
            for i in 0..3 {
                let (a, b) = (key(points[i]), key(points[(i + 1) % 3]));
                let edge = if a < b { (a, b) } else { (b, a) };
                if let Some(previous) = owners.insert(edge, index) {
                    let a = root(&mut parents, index);
                    let b = root(&mut parents, previous);
                    parents[a.max(b)] = a.min(b);
                }
            }
        }
        let mut components: HashMap<usize, Face> = HashMap::new();
        for (index, points) in triangles.into_iter().enumerate() {
            let face = components.entry(root(&mut parents, index)).or_default();
            face.triangles.push(points.map(&project));
            face.source_triangles.push(points.map(&world));
            for i in 0..3 {
                let pair = [points[i], points[(i + 1) % 3]];
                let (a, b) = (key(pair[0]), key(pair[1]));
                let edge = if a < b { (a, b) } else { (b, a) };
                face.edges
                    .entry(edge)
                    .and_modify(|(n, _)| *n += 1)
                    .or_insert((1, pair));
            }
        }
        let mut components = components.into_iter().collect::<Vec<_>>();
        components.sort_by_key(|(key, _)| *key);
        for (_, face) in components {
            let mut edges = face
                .edges
                .into_iter()
                .filter(|(_, (count, _))| *count == 1)
                .collect::<Vec<_>>();
            edges.sort_by_key(|(key, _)| *key);
            let curves = edges
                .into_iter()
                .filter_map(|(_, (_, pair))| {
                    let (a, b) = (project(pair[0]), project(pair[1]));
                    (nator_sketch::distance(a, b) > 1e-8).then_some(Segment::Line { a, b })
                })
                .collect::<Vec<_>>();
            if !curves.is_empty() {
                result.push(Element {
                    source: SourceGeometry {
                        name: name.into(),
                        triangles: face.source_triangles,
                        ..Default::default()
                    },
                    kind: Kind::Plane,
                    curves,
                    vertices: vec![],
                    triangles: face.triangles,
                    contours: vec![],
                });
            }
        }
    }
    result
}

fn sketch_elements(
    sketch: &nator_sketch::Sketch,
    source: Placement,
    plane: Plane,
    target: Placement,
) -> Result<Vec<Element>, DocumentError> {
    let error = |e: nator_sketch::SketchError| DocumentError(e.to_string());
    let world = |p| source.apply(sketch.plane.world(p));
    let project = |p| plane.project(target.inverse_apply(world(p)));
    let alignment = nator_geometry::dot(
        target.inverse_vector(source.apply_vector(sketch.plane.normal())),
        plane.normal(),
    );
    let curves = |curve: &Segment| -> Vec<Segment> {
        match *curve {
            Segment::Line { a, b } => {
                let (a, b) = (project(a), project(b));
                if nator_sketch::distance(a, b) < 1e-8 {
                    vec![]
                } else {
                    vec![Segment::Line { a, b }]
                }
            }
            Segment::Circle { center, radius } if alignment.abs() > 1. - 1e-9 => {
                vec![Segment::Circle {
                    center: project(center),
                    radius,
                }]
            }
            Segment::Arc {
                center,
                a,
                b,
                clockwise,
            } if alignment.abs() > 1. - 1e-9 => vec![Segment::Arc {
                center: project(center),
                a: project(a),
                b: project(b),
                clockwise: clockwise != (alignment < 0.),
            }],
            _ => {
                let radius = match *curve {
                    Segment::Circle { radius, .. } => radius,
                    Segment::Arc { center, a, .. } => nator_sketch::distance(center, a),
                    _ => 0.,
                };
                let steps = (std::f64::consts::TAU * (radius / 0.08).sqrt())
                    .ceil()
                    .clamp(16., 16384.) as usize;
                curve
                    .sample(steps)
                    .windows(2)
                    .filter_map(|p| {
                        let (a, b) = (project(p[0]), project(p[1]));
                        (nator_sketch::distance(a, b) >= 1e-8).then_some(Segment::Line { a, b })
                    })
                    .collect()
            }
        }
    };
    let mut result = Vec::new();
    for point in sketch.visible_points() {
        result.push(Element {
            source: SourceGeometry {
                name: sketch.name.clone(),
                vertices: vec![world(point.position)],
                ..Default::default()
            },
            kind: Kind::Vertex,
            vertices: vec![project(point.position)],
            curves: vec![],
            triangles: vec![],
            contours: vec![],
        });
    }
    for entity in sketch.entities.iter().filter(|e| !e.infinite) {
        result.push(Element {
            source: SourceGeometry {
                name: sketch.name.clone(),
                curves: vec![
                    nator_sketch::profiles::segment(sketch, entity)
                        .map_err(error)?
                        .sample(128)
                        .into_iter()
                        .map(world)
                        .collect(),
                ],
                ..Default::default()
            },
            kind: Kind::Edge,
            vertices: vec![],
            curves: curves(&nator_sketch::profiles::segment(sketch, entity).map_err(error)?),
            triangles: vec![],
            contours: vec![],
        });
    }
    for region in nator_sketch::areas::regions(sketch).map_err(error)? {
        let mut edges = Vec::new();
        let mut contours = Vec::new();
        for wire in std::iter::once(&region.outer).chain(&region.holes) {
            let projected = wire.segments.iter().flat_map(&curves).collect::<Vec<_>>();
            contours.push(projected.iter().flat_map(|s| s.sample(128)).collect());
            edges.extend(projected);
        }
        if !edges.is_empty() {
            result.push(Element {
                source: SourceGeometry {
                    name: sketch.name.clone(),
                    contours: std::iter::once(&region.outer)
                        .chain(&region.holes)
                        .map(|wire| wire.sample(128).into_iter().map(world).collect())
                        .collect(),
                    ..Default::default()
                },
                kind: Kind::Plane,
                curves: edges,
                vertices: vec![],
                triangles: vec![],
                contours,
            });
        }
    }
    Ok(result)
}

/// Prepara a seleção de Projetar fora da thread gráfica. Nenhum índice de origem
/// é persistido: confirmar captura as coordenadas nesta revisão do documento.
pub fn elements_for_sketch(
    document: &Document,
    occurrences: &[Occurrence],
    plane: Plane,
    target: Placement,
    edited: Option<SketchId>,
) -> Result<Vec<Element>, DocumentError> {
    plane.validate().map_err(|e| DocumentError(e.to_string()))?;
    let mut result = Vec::new();
    for occurrence in occurrences {
        let part = document
            .instance(occurrence.instance)
            .and_then(|i| document.part(i.part))
            .ok_or_else(|| DocumentError("Ocorrência de projeção inexistente.".into()))?;
        for body in part.shown_bodies() {
            let source = occurrence
                .placement
                .compose(body.placement)
                .map_err(|e| DocumentError(e.to_string()))?;
            let project = |point| plane.project(target.inverse_apply(source.apply(point)));
            if let Some(bytes) = &body.brep {
                #[cfg(feature = "occt")]
                {
                    let query = || -> Result<Vec<Element>, nator_kernel::KernelError> {
                        let mut result = Vec::new();
                        let mut solid = nator_kernel::Solid::from_brep(bytes, None)?;
                        let mut edges = Vec::new();
                        let mut world_edges: Vec<Vec<nator_geometry::Point3>> = Vec::new();
                        for index in 0..solid.edge_count()? {
                            let anchors = solid.edge_anchors(index)?;
                            if anchors.seam {
                                edges.push(Vec::new());
                                world_edges.push(Vec::new());
                                continue;
                            }
                            if !anchors.circle.is_some_and(|c| c.closed) {
                                for p in [anchors.start, anchors.end] {
                                    result.push(Element {
                                        source: SourceGeometry {
                                            name: body.name.clone(),
                                            vertices: vec![source.apply(p)],
                                            ..Default::default()
                                        },
                                        kind: Kind::Vertex,
                                        curves: vec![],
                                        vertices: vec![project(p)],
                                        triangles: vec![],
                                        contours: vec![],
                                    });
                                }
                            }
                            let curves =
                                edge_curves(&solid, index, anchors, plane, source, target)?;
                            let world_curve = solid
                                .edge_polyline(index, 0.01)?
                                .into_iter()
                                .map(|p| source.apply(p))
                                .collect::<Vec<_>>();
                            result.push(Element {
                                source: SourceGeometry {
                                    name: body.name.clone(),
                                    curves: vec![world_curve.clone()],
                                    ..Default::default()
                                },
                                kind: Kind::Edge,
                                vertices: if curves.is_empty() {
                                    vec![project(anchors.start)]
                                } else {
                                    vec![]
                                },
                                curves: curves.clone(),
                                triangles: vec![],
                                contours: vec![],
                            });
                            world_edges.push(world_curve);
                            edges.push(curves);
                        }
                        // Arquivos legados podem ter B-rep sem mapa de faces da malha.
                        // Nesse caso consultamos uma tesselação local no mesmo worker.
                        let fallback = if body.geometry.triangle_faces().is_none() {
                            Some(solid.tessellate_with_faces(0.02, 0.2)?)
                        } else {
                            None
                        };
                        let mesh = fallback
                            .as_ref()
                            .map(|(m, _)| m)
                            .unwrap_or_else(|| body.geometry.mesh());
                        let triangle_faces = fallback
                            .as_ref()
                            .map(|(_, f)| f.as_slice())
                            .or_else(|| body.geometry.triangle_faces());
                        let mut faces = Vec::new();
                        for face in 0..solid.face_count()? {
                            let face_edges = solid.face_edges(face)?;
                            let curves = face_edges
                                .iter()
                                .copied()
                                .flat_map(|i| edges[i].clone())
                                .collect::<Vec<_>>();
                            let triangles = mesh
                                .triangles()
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| {
                                    triangle_faces.is_some_and(|f| f[*i] as usize == face)
                                })
                                .map(|(_, t)| mesh.triangle_points(*t).map(project))
                                .collect();
                            if !curves.is_empty() {
                                faces.push(Element {
                                    source: SourceGeometry {
                                        name: body.name.clone(),
                                        curves: face_edges
                                            .iter()
                                            .map(|i| world_edges[*i].clone())
                                            .collect(),
                                        triangles: mesh
                                            .triangles()
                                            .iter()
                                            .enumerate()
                                            .filter(|(i, _)| {
                                                triangle_faces
                                                    .is_some_and(|f| f[*i] as usize == face)
                                            })
                                            .map(|(_, t)| {
                                                mesh.triangle_points(*t).map(|p| source.apply(p))
                                            })
                                            .collect(),
                                        ..Default::default()
                                    },
                                    kind: Kind::Plane,
                                    curves,
                                    vertices: vec![],
                                    triangles,
                                    contours: vec![],
                                });
                            }
                        }
                        result.extend(faces);
                        Ok(result)
                    };
                    result.extend(query().map_err(|e| DocumentError(e.to_string()))?);
                    continue;
                }
                #[cfg(not(feature = "occt"))]
                {
                    let _ = bytes;
                    return Err(DocumentError(
                        "Projetar geometria CAD exige o núcleo OCCT.".into(),
                    ));
                }
            }
            // Malhas conservam apenas vértices e arestas vivas; diagonais internas
            // de triangulação não são apresentadas como arestas do componente.
            let mesh = body.geometry.mesh();
            for p in mesh.vertices() {
                result.push(Element {
                    source: SourceGeometry {
                        name: body.name.clone(),
                        vertices: vec![source.apply(*p)],
                        ..Default::default()
                    },
                    kind: Kind::Vertex,
                    curves: vec![],
                    vertices: vec![project(*p)],
                    triangles: vec![],
                    contours: vec![],
                });
            }
            for (triangle, mask) in mesh.triangles().iter().zip(body.geometry.creases()) {
                let points = mesh.triangle_points(*triangle).map(project);
                for i in 0..3 {
                    if mask & (1 << i) != 0 {
                        result.push(Element {
                            source: SourceGeometry {
                                name: body.name.clone(),
                                curves: vec![vec![
                                    source.apply(mesh.triangle_points(*triangle)[(i + 1) % 3]),
                                    source.apply(mesh.triangle_points(*triangle)[(i + 2) % 3]),
                                ]],
                                ..Default::default()
                            },
                            kind: Kind::Edge,
                            curves: vec![Segment::Line {
                                a: points[(i + 1) % 3],
                                b: points[(i + 2) % 3],
                            }],
                            vertices: vec![],
                            triangles: vec![],
                            contours: vec![],
                        });
                    }
                }
            }
            result.extend(mesh_areas(mesh, project, |p| source.apply(p), &body.name));
        }
    }
    for occurrence in occurrences {
        if let Some(part) = document
            .instance(occurrence.instance)
            .and_then(|i| document.part(i.part))
        {
            for record in part
                .sketches
                .iter()
                .filter(|s| Some(s.id) != edited && part.shows_sketch(s))
            {
                result.extend(sketch_elements(
                    &record.sketch,
                    occurrence.placement,
                    plane,
                    target,
                )?);
            }
        }
    }
    for record in document
        .reference_sketches()
        .iter()
        .filter(|s| Some(s.id) != edited && document.shows_reference_sketch(s))
    {
        result.extend(sketch_elements(
            &record.sketch,
            Placement::default(),
            plane,
            target,
        )?);
    }
    Ok(result)
}

#[cfg(feature = "occt")]
fn edge_curves(
    solid: &nator_kernel::Solid,
    index: usize,
    anchors: nator_kernel::EdgeAnchors,
    plane: Plane,
    source: Placement,
    target: Placement,
) -> Result<Vec<Segment>, nator_kernel::KernelError> {
    let project = |p| plane.project(target.inverse_apply(source.apply(p)));
    let a = project(anchors.start);
    let b = project(anchors.end);
    if solid.edge_measure(index)?.straight {
        return Ok(if nator_sketch::distance(a, b) < 1e-8 {
            vec![]
        } else {
            vec![Segment::Line { a, b }]
        });
    }
    if let Some(circle) = anchors.circle {
        // Normais são vetores: não recebem a translação dos referenciais.
        let normal = target.inverse_vector(source.apply_vector(circle.normal));
        let alignment = nator_geometry::dot(normal, plane.normal());
        if alignment.abs() > 1. - 1e-9 {
            let center = project(circle.center);
            return Ok(vec![if circle.closed {
                Segment::Circle {
                    center,
                    radius: circle.radius_mm,
                }
            } else {
                Segment::Arc {
                    center,
                    a,
                    b,
                    clockwise: alignment < 0.,
                }
            }]);
        }
    }
    // Círculos inclinados viram elipses. O rascunho ainda não tem esse tipo:
    // aproximação explícita com desvio de 0,01 mm, sem inventar um raio circular.
    Ok(solid
        .edge_polyline(index, 0.01)?
        .windows(2)
        .filter_map(|pair| {
            let a = project(pair[0]);
            let b = project(pair[1]);
            (nator_sketch::distance(a, b) >= 1e-8).then_some(Segment::Line { a, b })
        })
        .collect())
}

/// Captura no plano, em mm: vértices precedem curvas e curvas precedem faces.
/// O filtro permite alcançar um contorno inteiro mesmo perto das suas arestas.
pub fn pick(
    elements: &[Element],
    cursor: Point2,
    tolerance: f64,
    filter: Option<Kind>,
) -> Option<usize> {
    let mut best: Option<(usize, u8, f64)> = None;
    for (index, element) in elements
        .iter()
        .enumerate()
        .filter(|(_, e)| filter.is_none_or(|k| e.kind == k))
    {
        let (priority, distance) = match element.kind {
            Kind::Vertex => (
                0,
                element
                    .vertices
                    .iter()
                    .map(|p| nator_sketch::distance(*p, cursor))
                    .fold(f64::INFINITY, f64::min),
            ),
            Kind::Edge => (
                1,
                element
                    .curves
                    .iter()
                    .map(|c| nator_sketch::projection::distance_to(c, cursor))
                    .fold(f64::INFINITY, f64::min),
            ),
            Kind::Plane => {
                if !inside_contours(&element.contours, cursor)
                    && !element.triangles.iter().any(|t| {
                        let sides = (0..3)
                            .map(|i| {
                                nator_sketch::cross(
                                    nator_sketch::sub(t[(i + 1) % 3], t[i]),
                                    nator_sketch::sub(cursor, t[i]),
                                )
                            })
                            .collect::<Vec<_>>();
                        let area = nator_sketch::cross(
                            nator_sketch::sub(t[1], t[0]),
                            nator_sketch::sub(t[2], t[0]),
                        )
                        .abs();
                        area > 1e-12
                            && (sides.iter().all(|s| *s >= -1e-9)
                                || sides.iter().all(|s| *s <= 1e-9))
                    })
                {
                    continue;
                }
                (2, 0.)
            }
        };
        if distance <= tolerance
            && best.is_none_or(|(_, p, d)| priority < p || (priority == p && distance < d))
        {
            best = Some((index, priority, distance));
        }
    }
    best.map(|(i, _, _)| i)
}

/// O chamador fornece as ocorrências permitidas pelo isolamento/bloqueio da interface.
/// Visibilidade de corpos e rascunhos continua sendo consultada no documento. O plano
/// está no referencial da peça editada; `target` posiciona esse referencial na montagem.
/// Nenhum desses apoios cria um vínculo paramétrico nem é persistido como índice CAD.
pub fn points(
    document: &Document,
    occurrences: &[Occurrence],
    edited: SketchId,
    plane: Plane,
    target: Placement,
) -> Result<Vec<Point2>, DocumentError> {
    plane.validate().map_err(|e| DocumentError(e.to_string()))?;
    let mut result = vec![[0., 0.]];
    let mut bodies = HashMap::new();
    let mut sketches = HashMap::new();
    let project =
        |point, source: Placement| plane.project(target.inverse_apply(source.apply(point)));
    for occurrence in occurrences {
        let part = document
            .instance(occurrence.instance)
            .and_then(|i| document.part(i.part))
            .ok_or_else(|| DocumentError("Ocorrência de projeção inexistente.".into()))?;
        for body in part.shown_bodies() {
            // Uma definição pode aparecer muitas vezes na montagem. Consultar a
            // geometria uma vez e compor cada ocorrência mantém os apoios exatos.
            if let std::collections::hash_map::Entry::Vacant(entry) = bodies.entry(body.id) {
                entry.insert(body_anchors(body)?);
            }
            let source = occurrence
                .placement
                .compose(body.placement)
                .map_err(|e| DocumentError(e.to_string()))?;
            if let Some(anchors) = bodies.get(&body.id) {
                result.extend(anchors.iter().map(|a| project(a.point, source)));
            }
        }
        for record in part
            .sketches
            .iter()
            .filter(|s| s.id != edited && part.shows_sketch(s))
        {
            if let std::collections::hash_map::Entry::Vacant(entry) = sketches.entry(record.id) {
                entry.insert(reference_points(&record.sketch)?);
            }
            if let Some(anchors) = sketches.get(&record.id) {
                result.extend(
                    anchors
                        .iter()
                        .map(|p| project(record.sketch.plane.world(*p), occurrence.placement)),
                );
            }
        }
    }
    for record in document
        .reference_sketches()
        .iter()
        .filter(|s| s.id != edited && document.shows_reference_sketch(s))
    {
        result.extend(
            reference_points(&record.sketch)?
                .into_iter()
                .map(|p| project(record.sketch.plane.world(p), Placement::default())),
        );
    }
    if result.iter().flatten().any(|v| !v.is_finite()) {
        return Err(DocumentError(
            "Ponto projetado fora do intervalo numérico.".into(),
        ));
    }
    // Faces em alturas distintas podem produzir o mesmo apoio em um plano paralelo.
    // A ordem não é identidade topológica: só elimina duplicações numéricas na captura.
    result.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    result.dedup_by(|a, b| nator_sketch::distance(*a, *b) < 1e-9);
    Ok(result)
}
