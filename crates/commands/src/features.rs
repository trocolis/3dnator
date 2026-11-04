//! Construção ocorre no worker. Somente bytes B-rep, nomes de faces e malhas chegam ao
//! documento; objetos nativos do núcleo nunca saem da thread que os criou.
#[cfg(feature = "occt")]
use nator_document::features::{
    BooleanMode, EdgeRef, ExtrusionLimit, FaceRef, RegionRef, UntilFace, body_owner, signature,
};
#[cfg(feature = "occt")]
use nator_document::parameters::Dimension;
use nator_document::{BodyId, Document, DocumentError, MeshAsset, PartId, features::Operation};
#[cfg(feature = "occt")]
use nator_kernel::Solid;
use std::sync::Arc;

#[cfg(feature = "occt")]
fn hole_key(wire: &nator_sketch::profiles::Wire) -> Vec<u64> {
    let mut key = wire.entities.clone();
    key.sort_unstable();
    key.dedup();
    key
}
#[cfg(feature = "occt")]
fn order_holes(
    region: &mut nator_sketch::profiles::Region,
    keys: &[Vec<u64>],
) -> Result<(), DocumentError> {
    if region.holes.len() != keys.len() {
        return Err(DocumentError(
            "A quantidade de furos mudou; atualize a correspondência na transição.".into(),
        ));
    }
    let mut ordered = vec![];
    for key in keys {
        if region.holes.iter().filter(|w| hole_key(w) == *key).count() != 1 {
            return Err(DocumentError(
                "Identidade do furo perdida ou ambígua; atualize a correspondência da transição."
                    .into(),
            ));
        }
        let index = region
            .holes
            .iter()
            .position(|w| hole_key(w) == *key)
            .ok_or_else(|| {
                DocumentError(
                    "Um furo da transição foi substituído; atualize sua correspondência.".into(),
                )
            })?;
        ordered.push(region.holes.remove(index));
    }
    region.holes = ordered;
    Ok(())
}

#[cfg(feature = "occt")]
fn inherited_colors(
    doc: &Document,
    part: PartId,
    operation: &Operation,
    result: &Solid,
    owner: &str,
) -> Result<Vec<Option<nator_geometry::RgbColor>>, DocumentError> {
    use std::collections::HashMap;
    let mut colors = vec![None; result.face_count().map_err(error)?];
    let mut inputs = operation.inputs();
    if let Operation::ExtrudeFace { base, .. } = operation {
        inputs.push(*base);
    }
    inputs.sort_unstable_by_key(|id| id.0);
    inputs.dedup();
    for input in inputs {
        let Some(body) = doc.body(input).filter(|b| !b.intrinsic_colors.is_empty()) else {
            continue;
        };
        // Uma pintura uniforme explícita já substituiu as cores próprias.
        if doc
            .body_appearance(input)
            .is_some_and(|a| a.base_color.is_some())
        {
            continue;
        }
        let (source, bytes) = source(doc, part, input)?;
        let mut names: HashMap<&str, Option<nator_geometry::RgbColor>> = HashMap::new();
        for (name, color) in source.face_names().iter().zip(body.intrinsic_colors.iter()) {
            if name.is_empty() {
                continue;
            }
            names
                .entry(name)
                .and_modify(|old| {
                    if *old != *color {
                        *old = None
                    }
                })
                .or_insert(*color);
        }
        let carry = if let Operation::ExtrudeFace { base, face, .. } = operation {
            if *base == input {
                body.intrinsic_colors
                    .get(face_index(&source, &bytes, face)?)
                    .copied()
                    .flatten()
            } else {
                None
            }
        } else {
            None
        };
        let owner = format!("{owner}:");
        for (index, color) in colors.iter_mut().enumerate() {
            if let Some(inherited) = names
                .get(result.face_names()[index].as_str())
                .copied()
                .flatten()
            {
                *color = Some(inherited);
                continue;
            }
            if carry.is_some() && result.face_names()[index].starts_with(&owner) {
                *color = carry;
                continue;
            }
            for (original, candidate) in body.intrinsic_colors.iter().enumerate() {
                if let Some(candidate) = candidate
                    && source
                        .face_common_area(original, result, index)
                        .map_err(error)?
                        > 1e-7
                {
                    *color = Some(*candidate);
                    break;
                }
            }
        }
    }
    if colors.iter().all(Option::is_none) {
        colors.clear();
    }
    Ok(colors)
}

/// Resultado CAD de uma operação: malha de exibição, B-rep e nomes estáveis das faces.
pub(super) struct Built {
    pub mesh: Arc<MeshAsset>,
    pub brep: Arc<Vec<u8>>,
    pub faces: Arc<Vec<String>>,
    pub intrinsic_colors: Arc<Vec<Option<nator_geometry::RgbColor>>>,
}

/// Recalcula na ordem as operações ativas da peça. Operações suprimidas, dependentes de uma
/// suprimida ou posteriores ao marcador do histórico mantêm a última geometria, oculta. Uma
/// falha interrompe o recálculo identificando a operação; como os comandos operam sobre uma
/// cópia candidata, o documento anterior é preservado.
pub(super) fn rebuild(doc: &mut Document, part: PartId) -> Result<(), super::CommandError> {
    let piece = doc
        .part(part)
        .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
    let states = nator_document::features::states(piece);
    for (position, state) in states.into_iter().enumerate() {
        if !state.is_active() {
            continue;
        }
        // Lido a cada etapa: refazer um corpo atualiza as referências das operações seguintes.
        let feature = doc
            .part(part)
            .and_then(|p| p.features.get(position))
            .ok_or_else(|| DocumentError("Operação inexistente.".into()))?
            .clone();
        let built = construct(doc, part, feature.output, &feature.operation).map_err(|e| {
            let dependents = doc
                .part(part)
                .map(|p| nator_document::features::dependents(p, feature.output))
                .unwrap_or_default();
            super::CommandError {
                message: format!("{}: {}", feature.name, e.0),
                failure: Some(super::RebuildFailure {
                    part,
                    output: feature.output,
                    name: feature.name.clone(),
                    message: e.0,
                    dependents,
                }),
            }
        })?;
        let body = doc.body_mut(feature.output)?;
        let bytes = body.brep.replace(built.brep);
        let names = std::mem::replace(&mut body.face_names, built.faces);
        body.geometry = built.mesh;
        body.intrinsic_colors = built.intrinsic_colors;
        if let Some(bytes) = bytes {
            retarget(doc, part, feature.output, &bytes, &names);
        }
    }
    crate::appearance::rebuild(doc, part)?;
    Ok(())
}

/// Abaixo disto é o mesmo lugar: geometria refeita varia só no ruído numérico.
#[cfg(feature = "occt")]
const SAME_PLACE_MM: f64 = 1e-6;

/// Ponto exato sobre a face, longe da borda: o centro do maior triângulo da malha.
#[cfg(feature = "occt")]
fn point_on_face(
    solid: &mut Solid,
    index: usize,
) -> Result<Option<nator_geometry::Point3>, nator_kernel::KernelError> {
    use nator_geometry::{cross, norm, sub};
    let (mesh, faces) = solid.tessellate_with_faces(0.05, 0.25)?;
    let vertices = mesh.vertices();
    let largest = mesh
        .triangles()
        .iter()
        .zip(&faces)
        .filter(|(_, face)| **face as usize == index)
        .map(|(triangle, _)| triangle.map(|v| vertices[v as usize]))
        .max_by(|p, q| {
            let area =
                |t: &[nator_geometry::Point3; 3]| norm(cross(sub(t[1], t[0]), sub(t[2], t[0])));
            area(p).total_cmp(&area(q))
        });
    largest
        .map(|[a, b, c]| {
            let center = std::array::from_fn(|i| (a[i] + b[i] + c[i]) / 3.);
            Ok(solid.face_nearest(index, center)?.1)
        })
        .transpose()
}

/// A mesma face no corpo refeito. Só vale um resultado único: sem ele, a referência fica como
/// está e a operação falha com erro explícito, em vez de agir sobre outra geometria.
#[cfg(feature = "occt")]
fn same_face(
    old: &mut Solid,
    index: usize,
    new: &Solid,
) -> Result<Option<usize>, nator_kernel::KernelError> {
    let name = old.face_names().get(index).cloned().unwrap_or_default();
    let candidates = new
        .face_names()
        .iter()
        .enumerate()
        .filter(|(_, other)| **other == name)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if let ([only], false) = (candidates.as_slice(), name.is_empty()) {
        return Ok(Some(*only));
    }
    let Some(point) = point_on_face(old, index)? else {
        return Ok(None);
    };
    let mut found = Vec::new();
    for candidate in candidates {
        if new.face_nearest(candidate, point)?.0 < SAME_PLACE_MM {
            found.push(candidate);
        }
    }
    Ok(if let [only] = found.as_slice() {
        Some(*only)
    } else {
        None
    })
}

/// A mesma aresta no corpo refeito, pelo ponto no meio dela.
#[cfg(feature = "occt")]
fn same_edge(
    old: &Solid,
    index: usize,
    new: &Solid,
) -> Result<Option<usize>, nator_kernel::KernelError> {
    let point = old.edge_anchors(index)?.midpoint;
    let mut found = Vec::new();
    for candidate in 0..new.edge_count()? {
        let other = new.edge_anchors(candidate)?.midpoint;
        if nator_geometry::norm(nator_geometry::sub(other, point)) < SAME_PLACE_MM {
            found.push(candidate);
        }
    }
    Ok(if let [only] = found.as_slice() {
        Some(*only)
    } else {
        None
    })
}

/// Atualiza as referências por índice ao corpo que acabou de ser refeito. A assinatura compara
/// bytes do B-rep, que mudam com ruído na última casa decimal mesmo com a face no mesmo lugar;
/// sem isso, qualquer recálculo invalidaria uma referência correta.
#[cfg(feature = "occt")]
fn retarget(doc: &mut Document, part: PartId, body: BodyId, bytes: &[u8], names: &[String]) {
    let Some(fresh) = doc.body(body).and_then(|b| b.brep.clone()) else {
        return;
    };
    let Some(piece) = doc.part(part) else {
        return;
    };
    if fresh.as_slice() == bytes {
        return;
    }
    let (stale, stamp) = (signature(bytes), signature(&fresh));
    let new_names = doc.body(body).map(|b| b.face_names.clone());
    let mut features = piece.features.clone();
    let mut solids = None;
    let mut changed = false;
    for feature in &mut features {
        for (owner, face) in feature.operation.face_refs_mut() {
            let FaceRef::Index { index, signature } = face else {
                continue;
            };
            if owner != body || *signature != stale {
                continue;
            }
            let found = load_pair(&mut solids, bytes, names, &fresh, new_names.as_deref())
                .and_then(|(old, new)| same_face(old, *index, new).ok().flatten());
            if let Some(index) = found {
                *face = FaceRef::Index {
                    index,
                    signature: stamp,
                };
                changed = true;
            }
        }
        for (owner, edge) in feature.operation.edge_refs_mut() {
            let EdgeRef::Index { index, signature } = edge else {
                continue;
            };
            if owner != body || *signature != stale {
                continue;
            }
            let found = load_pair(&mut solids, bytes, names, &fresh, new_names.as_deref())
                .and_then(|(old, new)| same_edge(old, *index, new).ok().flatten());
            if let Some(index) = found {
                *edge = EdgeRef::Index {
                    index,
                    signature: stamp,
                };
                changed = true;
            }
        }
    }
    if changed && let Ok(piece) = doc.part_mut(part) {
        piece.features = features;
    }
    // A pintura por índice segue o mesmo critério. Sem isso, qualquer recálculo do corpo
    // invalidaria a camada e recusaria a edição inteira. O cache volta vazio e é refeito
    // em `appearance::rebuild`; face que sumiu continua falhando de forma explícita.
    let Some(appearance) = doc.body_appearance(body).cloned() else {
        return;
    };
    let mut layers = appearance.layers.clone();
    let mut moved = false;
    for layer in &mut layers {
        let FaceRef::Index { index, signature } = &mut layer.face else {
            continue;
        };
        if *signature != stale {
            continue;
        }
        let found = load_pair(&mut solids, bytes, names, &fresh, new_names.as_deref())
            .and_then(|(old, new)| same_face(old, *index, new).ok().flatten());
        if let Some(found) = found {
            *index = found;
            *signature = stamp;
            moved = true;
        }
    }
    if moved {
        let _ = doc.set_surface_appearance(nator_document::appearance::SurfaceAppearance {
            layers,
            source_signature: stamp,
            geometry: None,
            materials: Arc::new(Vec::new()),
            ..appearance
        });
    }
}

/// Carrega os dois sólidos só se alguma referência precisar deles.
#[cfg(feature = "occt")]
fn load_pair<'a>(
    solids: &'a mut Option<(Solid, Solid)>,
    bytes: &[u8],
    names: &[String],
    fresh: &[u8],
    new_names: Option<&Vec<String>>,
) -> Option<(&'a mut Solid, &'a Solid)> {
    if solids.is_none() {
        let old = Solid::from_brep(bytes, (!names.is_empty()).then_some(names)).ok()?;
        let new = Solid::from_brep(fresh, new_names.map(Vec::as_slice)).ok()?;
        *solids = Some((old, new));
    }
    solids.as_mut().map(|(old, new)| (old, &*new))
}

#[cfg(not(feature = "occt"))]
fn retarget(_doc: &mut Document, _part: PartId, _body: BodyId, _bytes: &[u8], _names: &[String]) {}

#[cfg(feature = "occt")]
fn error(e: impl std::fmt::Display) -> DocumentError {
    DocumentError(e.to_string())
}

/// Corpo de entrada já posicionado na peça, com os nomes de faces gravados.
#[cfg(feature = "occt")]
pub(super) fn source(
    doc: &Document,
    part: PartId,
    id: BodyId,
) -> Result<(Solid, Arc<Vec<u8>>), DocumentError> {
    let body = doc
        .part(part)
        .and_then(|p| p.bodies.iter().find(|b| b.id == id))
        .ok_or_else(|| DocumentError("Corpo da operação inexistente nesta peça.".into()))?;
    let bytes = body.brep.clone().ok_or_else(|| {
        DocumentError("Corpos de malha não participam de operações CAD precisas.".into())
    })?;
    let names = (!body.face_names.is_empty()).then_some(body.face_names.as_slice());
    let solid = Solid::from_brep(&bytes, names)
        .and_then(|s| s.transformed(body.placement))
        .map_err(error)?;
    Ok((solid, bytes))
}

#[cfg(feature = "occt")]
pub(super) fn sketch_of(
    doc: &Document,
    part: PartId,
    id: nator_document::SketchId,
) -> Result<&nator_sketch::Sketch, DocumentError> {
    doc.part(part)
        .and_then(|p| p.sketches.iter().find(|r| r.id == id))
        .map(|r| &r.sketch)
        .ok_or_else(|| DocumentError("Rascunho inexistente nesta peça.".into()))
}

/// Arestas exatas de uma face, no plano do rascunho. Curvas livres viram polilinha fina.
#[cfg(feature = "occt")]
fn face_segments(
    solid: &Solid,
    face: usize,
    plane: nator_sketch::Plane,
) -> Result<Vec<nator_sketch::profiles::Segment>, DocumentError> {
    use nator_kernel::EdgeGeometry;
    use nator_sketch::profiles::Segment;
    let local = |p: nator_geometry::Point3| plane.project(p);
    let mut segments = Vec::new();
    for edge in solid.face_edges(face).map_err(error)? {
        match solid.edge_geometry(edge).map_err(error)? {
            EdgeGeometry::Line(a, b) => segments.push(Segment::Line {
                a: local(a),
                b: local(b),
            }),
            EdgeGeometry::Circle {
                center,
                radius,
                closed: true,
                ..
            } => segments.push(Segment::Circle {
                center: local(center),
                radius,
            }),
            EdgeGeometry::Circle {
                center,
                start,
                middle,
                end,
                ..
            } => {
                let (c, a, b, m) = (local(center), local(start), local(end), local(middle));
                let angle = |p: nator_sketch::Point2| (p[1] - c[1]).atan2(p[0] - c[0]);
                let turn = |p| (angle(p) - angle(a)).rem_euclid(std::f64::consts::TAU);
                segments.push(Segment::Arc {
                    center: c,
                    a,
                    b,
                    clockwise: turn(m) > turn(b),
                });
            }
            EdgeGeometry::Curve(points) => {
                for pair in points.windows(2) {
                    let (a, b) = (local(pair[0]), local(pair[1]));
                    if nator_sketch::distance(a, b) > 1e-9 {
                        segments.push(Segment::Line { a, b });
                    }
                }
            }
        }
    }
    Ok(segments)
}

/// Linhas de todas as faces planas que estão no plano do rascunho: só as do corpo
/// indicado ou, sem ele, as dos corpos exibidos da peça.
#[cfg(feature = "occt")]
pub fn face_lines(
    doc: &Document,
    part: PartId,
    plane: nator_sketch::Plane,
    body: Option<BodyId>,
) -> Result<Vec<nator_sketch::profiles::Segment>, DocumentError> {
    let piece = doc
        .part(part)
        .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
    let bodies = match body {
        Some(id) => vec![id],
        None => piece
            .shown_bodies()
            .filter(|b| b.brep.is_some())
            .map(|b| b.id)
            .collect(),
    };
    let normal = plane.normal();
    let mut segments = Vec::new();
    for id in bodies {
        if doc.body(id).is_none_or(|b| b.brep.is_none()) {
            continue;
        }
        let (solid, _) = source(doc, part, id)?;
        for face in 0..solid.face_count().map_err(error)? {
            let nator_kernel::FaceKind::Plane {
                normal: face_normal,
                offset,
            } = solid.face_measure(face).map_err(error)?.kind
            else {
                continue;
            };
            if nator_geometry::norm(nator_geometry::cross(face_normal, normal)) < 1e-7
                && (nator_geometry::dot(face_normal, plane.origin) - offset).abs() < 1e-6
            {
                segments.extend(face_segments(&solid, face, plane)?);
            }
        }
    }
    Ok(segments)
}
#[cfg(not(feature = "occt"))]
pub fn face_lines(
    _: &Document,
    _: PartId,
    _: nator_sketch::Plane,
    _: Option<BodyId>,
) -> Result<Vec<nator_sketch::profiles::Segment>, DocumentError> {
    Ok(Vec::new())
}

/// Rascunho acrescido do contorno exato da face: as linhas do corpo passam a delimitar
/// áreas como se fizessem parte do desenho, sem alterar o rascunho guardado.
#[cfg(feature = "occt")]
fn with_face_outline(
    sketch: &nator_sketch::Sketch,
    solid: &Solid,
    face: usize,
) -> Result<nator_sketch::Sketch, DocumentError> {
    use nator_sketch::{Curve, profiles::Segment};
    let fail = |e: nator_sketch::SketchError| DocumentError(e.to_string());
    let mut outlined = sketch.clone();
    for segment in face_segments(solid, face, sketch.plane)? {
        match segment {
            Segment::Line { a, b } => {
                outlined.add_line(a, b, false).map_err(fail)?;
            }
            Segment::Circle { center, radius } => {
                outlined.add_circle(center, radius, false).map_err(fail)?;
            }
            Segment::Arc {
                center,
                a,
                b,
                clockwise,
            } => {
                let curve = Curve::Arc {
                    center: outlined.add_point(center).map_err(fail)?,
                    start: outlined.add_point(a).map_err(fail)?,
                    end: outlined.add_point(b).map_err(fail)?,
                    clockwise,
                };
                outlined.add_curve(curve, false).map_err(fail)?;
            }
        }
    }
    Ok(outlined)
}

/// Traços do rascunho que delimitam uma região do arranjo com o contorno da face.
#[cfg(feature = "occt")]
fn drawn_entities(
    sketch: &nator_sketch::Sketch,
    outlined: &nator_sketch::Sketch,
    region: &nator_sketch::profiles::Region,
) -> Result<Vec<(u64, bool)>, DocumentError> {
    let mut ids = nator_sketch::areas::signature(outlined, region)
        .map_err(error)?
        .into_iter()
        .filter(|(id, _)| sketch.entity(*id).is_ok())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// Parte de uma face no arranjo do rascunho com o contorno dela.
#[cfg(feature = "occt")]
pub(super) struct FaceCell {
    pub outlined: nator_sketch::Sketch,
    pub region: nator_sketch::profiles::Region,
    /// Traços do rascunho que delimitam a parte, com o lado em que ela fica.
    pub drawn: Vec<(u64, bool)>,
}

/// Parte da face que contém `seed`.
#[cfg(feature = "occt")]
pub(super) fn face_cell(
    sketch: &nator_sketch::Sketch,
    solid: &Solid,
    face: usize,
    seed: nator_sketch::Point2,
) -> Result<FaceCell, DocumentError> {
    let outlined = with_face_outline(sketch, solid, face)?;
    let regions = nator_sketch::areas::regions(&outlined).map_err(error)?;
    let region = regions
        .into_iter()
        .filter(|r| r.outer.contains(seed) && !r.holes.iter().any(|h| h.contains(seed)))
        .min_by(|a, b| a.area().total_cmp(&b.area()))
        .ok_or_else(|| DocumentError("A área escolhida não existe mais na face.".into()))?;
    let drawn = drawn_entities(sketch, &outlined, &region)?;
    Ok(FaceCell {
        outlined,
        region,
        drawn,
    })
}

/// Rascunho e região de uma condição de área. Uma parte de face dividida por traços
/// abertos usa o rascunho com o contorno; se os traços mudaram, falha em vez de trocar
/// silenciosamente para a parte vizinha.
#[cfg(feature = "occt")]
pub(super) fn mask_area(
    sketch: &nator_sketch::Sketch,
    reference: &RegionRef,
    solid: &Solid,
    face: usize,
) -> Result<(nator_sketch::Sketch, nator_sketch::profiles::Region), DocumentError> {
    let RegionRef::Cell { seed, entities } = reference else {
        return Ok((sketch.clone(), region(sketch, reference)?));
    };
    let cell = face_cell(sketch, solid, face, seed.map(|v| v as f64 * 1e-6))?;
    if cell.drawn != *entities {
        return Err(DocumentError(
            "A área escolhida mudou: os traços que a delimitam não são mais os mesmos.".into(),
        ));
    }
    Ok((cell.outlined, cell.region))
}

#[cfg(feature = "occt")]
pub(super) fn region(
    sketch: &nator_sketch::Sketch,
    reference: &RegionRef,
) -> Result<nator_sketch::profiles::Region, DocumentError> {
    if let RegionRef::Area(signature) = reference {
        let mut regions = nator_sketch::areas::regions(sketch).map_err(error)?;
        let index = nator_sketch::areas::find(sketch, &regions, signature).map_err(error)?;
        return Ok(regions.swap_remove(index));
    }
    use nator_sketch::profiles;
    let mut regions = profiles::regions(sketch).map_err(error)?;
    let index = match reference {
        RegionRef::Boundary(ids) => profiles::find_region(&regions, ids).map_err(error)?,
        RegionRef::Index(index) => *index,
        RegionRef::Area(_) | RegionRef::Multiple(_) | RegionRef::Cell { .. } => {
            return Err(DocumentError(
                "Escolha uma única área para esta consulta.".into(),
            ));
        }
    };
    if index >= regions.len() {
        return Err(DocumentError("Região do rascunho inexistente.".into()));
    }
    Ok(regions.swap_remove(index))
}

/// Junção automática de áreas: cada face nova da extrusão (nome do dono) se une às
/// vizinhas no mesmo plano, numa só face. Faces antigas vizinhas entre si continuam
/// separadas, como as divisões feitas de propósito pela extrusão zero.
#[cfg(feature = "occt")]
fn merge_new_coplanar(solid: Solid, owner: &str) -> Result<Solid, DocumentError> {
    let count = solid.face_count().map_err(error)?;
    let planes = (0..count)
        .map(|face| {
            Ok(match solid.face_measure(face).map_err(error)?.kind {
                nator_kernel::FaceKind::Plane { normal, offset } => Some((normal, offset)),
                _ => None,
            })
        })
        .collect::<Result<Vec<_>, DocumentError>>()?;
    let same = |a: usize, b: usize| match (planes[a], planes[b]) {
        (Some((n, d)), Some((m, e))) => {
            let cosine = nator_geometry::dot(n, m);
            cosine.abs() > 1. - 1e-9 && (d - e * cosine.signum()).abs() < 1e-6
        }
        _ => false,
    };
    let new = |face: usize| solid.face_names()[face].starts_with(owner);
    // Faces de cada aresta, para achar as vizinhas.
    let mut by_edge = std::collections::HashMap::<usize, Vec<usize>>::new();
    for face in 0..count {
        for edge in solid.face_edges(face).map_err(error)? {
            by_edge.entry(edge).or_default().push(face);
        }
    }
    let mut parent = (0..count).collect::<Vec<_>>();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for faces in by_edge.values() {
        if let [a, b] = faces[..]
            && a != b
            && (new(a) || new(b))
            && same(a, b)
        {
            let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
            parent[ra] = rb;
        }
    }
    let mut groups = std::collections::HashMap::<usize, Vec<usize>>::new();
    for face in 0..count {
        let r = root(&mut parent, face);
        groups.entry(r).or_default().push(face);
    }
    let chosen = groups
        .into_values()
        .filter(|g| g.len() > 1)
        .flatten()
        .collect::<Vec<_>>();
    if chosen.len() < 2 {
        return Ok(solid);
    }
    solid.merge_faces(&chosen, owner).map_err(error)
}

#[cfg(feature = "occt")]
pub(super) fn boolean_mode(mode: BooleanMode) -> u32 {
    match mode {
        BooleanMode::Join => 0,
        BooleanMode::Cut => 1,
        BooleanMode::Intersect => 2,
    }
}

/// Eixo de revolução a partir de uma linha do rascunho, em coordenadas da peça.
#[cfg(feature = "occt")]
fn sketch_axis(
    sketch: &nator_sketch::Sketch,
    line: u64,
) -> Result<([f64; 3], [f64; 3]), DocumentError> {
    let (a, b) = sketch.line(line).map_err(|_| {
        DocumentError(
            "A linha do eixo não existe mais no rascunho; escolha o eixo novamente.".into(),
        )
    })?;
    let (a, b) = (sketch.plane.world(a), sketch.plane.world(b));
    Ok((a, nator_geometry::sub(b, a)))
}

/// Medidas avaliadas de um furo; `through` é a profundidade usada em furos passantes.
#[cfg(feature = "occt")]
fn hole_geometry(
    spec: &nator_document::features::HoleSpec,
    parameters: &nator_document::parameters::Parameters,
    through: f64,
) -> Result<nator_kernel::hole::HoleGeometry, DocumentError> {
    use nator_document::features::HoleHeadSpec;
    use nator_kernel::hole::{HoleGeometry, HoleHead};
    let length = |e: &str| parameters.evaluate(e, Dimension::Length);
    let angle = |e: &str| parameters.evaluate(e, Dimension::Angle);
    let head = match &spec.head {
        HoleHeadSpec::Simple => HoleHead::Simple,
        HoleHeadSpec::Counterbore { diameter, depth } => HoleHead::Counterbore {
            diameter: length(diameter)?,
            depth: length(depth)?,
        },
        HoleHeadSpec::Countersink {
            diameter,
            angle: cone,
        } => HoleHead::Countersink {
            diameter: length(diameter)?,
            angle: angle(cone)?,
        },
        HoleHeadSpec::Nut {
            across_flats,
            depth,
        } => HoleHead::Nut {
            across_flats: length(across_flats)?,
            depth: length(depth)?,
        },
    };
    let (depth, tip_angle) = match &spec.depth {
        Some(depth) => (
            length(depth)?,
            spec.tip_angle.as_deref().map(angle).transpose()?,
        ),
        None => (through, None),
    };
    Ok(HoleGeometry {
        diameter: length(&spec.diameter)?,
        depth,
        head,
        tip_angle,
        teardrop: spec.teardrop,
    })
}

/// Uma referência por índice só vale para o B-rep exato em que foi escolhida.
#[cfg(feature = "occt")]
fn same_geometry(bytes: &[u8], expected: u64) -> Result<(), DocumentError> {
    if signature(bytes) != expected {
        return Err(DocumentError(
            "A geometria mudou: selecione novamente a aresta/face desta operação no histórico."
                .into(),
        ));
    }
    Ok(())
}

#[cfg(feature = "occt")]
fn edge_index(solid: &Solid, bytes: &[u8], edge: &EdgeRef) -> Result<usize, DocumentError> {
    match edge {
        EdgeRef::Between(faces) => solid.edge_between(faces).map_err(error),
        EdgeRef::Index { index, signature } => {
            same_geometry(bytes, *signature)?;
            Ok(*index)
        }
    }
}

#[cfg(feature = "occt")]
pub(super) fn face_index(
    solid: &Solid,
    bytes: &[u8],
    face: &FaceRef,
) -> Result<usize, DocumentError> {
    match face {
        FaceRef::Named(name) => solid.face_named(name).map_err(error),
        FaceRef::Index { index, signature } => {
            same_geometry(bytes, *signature)?;
            Ok(*index)
        }
    }
}

/// Converte seleções por índice em referências estáveis antes de gravá-las no histórico.
/// A interface escolhe regiões, arestas e faces pela posição atual; aqui essa posição é
/// traduzida para a identidade de origem. Quando não há nome estável (por exemplo, face sem
/// origem conhecida), a referência por índice é mantida com sua assinatura de geometria.
#[cfg(feature = "occt")]
pub(super) fn normalize(
    doc: &Document,
    part: PartId,
    operation: Operation,
) -> Result<Operation, DocumentError> {
    let operation = if matches!(operation, Operation::Extrude { .. }) {
        let guide = crate::extrusion::guide(doc, part, &operation)?;
        if (guide.to - guide.from).abs() < 1e-7 {
            crate::extrusion::imprint_target(doc, part, operation, None)?
        } else {
            operation
        }
    } else {
        operation
    };
    let stable_one = |sketch, reference: RegionRef| -> Result<RegionRef, DocumentError> {
        Ok(match reference {
            RegionRef::Index(_) => {
                RegionRef::Boundary(region(sketch_of(doc, part, sketch)?, &reference)?.boundary())
            }
            stable => stable,
        })
    };
    let stable_region = |sketch, reference: RegionRef| -> Result<RegionRef, DocumentError> {
        if let RegionRef::Multiple(regions) = reference {
            Ok(RegionRef::Multiple(
                regions
                    .into_iter()
                    .map(|r| stable_one(sketch, r))
                    .collect::<Result<_, _>>()?,
            ))
        } else {
            stable_one(sketch, reference)
        }
    };
    Ok(match operation {
        Operation::ExtrudeFace {
            base,
            face,
            from,
            to,
            combine,
            until,
            boundaries,
        } => Operation::ExtrudeFace {
            face: stable_face(doc, part, base, face)?,
            base,
            from,
            to,
            combine,
            boundaries,
            until: until
                .map(|limit| {
                    Ok::<_, DocumentError>(match limit {
                        ExtrusionLimit::Face(limit) => UntilFace {
                            face: stable_face(doc, part, limit.body, limit.face)?,
                            body: limit.body,
                        }
                        .into(),
                        plane => plane,
                    })
                })
                .transpose()?,
        },
        Operation::Extrude {
            sketch,
            region,
            from,
            to,
            combine,
            until,
        } => Operation::Extrude {
            region: stable_region(sketch, region)?,
            until: until
                .map(|limit| {
                    Ok::<_, DocumentError>(match limit {
                        ExtrusionLimit::Face(limit) => UntilFace {
                            face: stable_face(doc, part, limit.body, limit.face)?,
                            body: limit.body,
                        }
                        .into(),
                        plane => plane,
                    })
                })
                .transpose()?,
            sketch,
            from,
            to,
            combine,
        },
        Operation::Label {
            base,
            face,
            mut spec,
        } => {
            let key = spec.shape_key();
            if spec.outline.is_empty() || spec.outline_key != key {
                spec.outline = Arc::new(
                    Solid::text_outline(
                        &spec.text,
                        &spec.font_family,
                        spec.bold,
                        spec.italic,
                        spec.underline,
                    )
                    .map_err(error)?,
                );
                spec.outline_key = key;
            }
            let face = stable_face(doc, part, base, face)?;
            let (solid, bytes) = source(doc, part, base)?;
            let frame = solid
                .surface_frame(face_index(&solid, &bytes, &face)?, spec.position, spec.uv)
                .map_err(error)?;
            spec.position = frame.origin;
            spec.normal = nator_geometry::cross(frame.u, frame.v);
            spec.uv = Some(frame.uv);
            Operation::Label { base, face, spec }
        }
        Operation::Pipe {
            sketch,
            entities,
            diameter,
            thickness,
        } => {
            let entities = if entities.len() == 1 {
                nator_sketch::connected::chain(sketch_of(doc, part, sketch)?, entities[0])
                    .map_err(error)?
            } else {
                entities
            };
            Operation::Pipe {
                sketch,
                entities,
                diameter,
                thickness,
            }
        }
        Operation::Sweep {
            start,
            end,
            route,
            holes,
        } => {
            let route = match route {
                nator_document::features::SweepRoute::Sketch { sketch, entities }
                    if entities.len() == 1 =>
                {
                    let source = sketch_of(doc, part, sketch)?;
                    nator_document::features::SweepRoute::Sketch {
                        sketch,
                        entities: nator_sketch::connected::chain(source, entities[0])
                            .map_err(error)?,
                    }
                }
                other => other,
            };
            let Operation::Loft {
                start, end, holes, ..
            } = normalize(
                doc,
                part,
                Operation::Loft {
                    start,
                    end,
                    holes,
                    ruled: false,
                    developable: false,
                },
            )?
            else {
                return Err(DocumentError("Perfis inválidos.".into()));
            };
            Operation::Sweep {
                start,
                end,
                route,
                holes,
            }
        }
        Operation::Loft {
            start,
            end,
            ruled,
            developable,
            holes,
        } => {
            let profile = |p: nator_document::features::ProfileRef| -> Result<_, DocumentError> {
                Ok(nator_document::features::ProfileRef {
                    region: stable_region(p.sketch, p.region)?,
                    sketch: p.sketch,
                })
            };
            let start = profile(start)?;
            let end = profile(end)?;
            let holes = match holes {
                Some(pairs) => pairs,
                None => {
                    let a = region(sketch_of(doc, part, start.sketch)?, &start.region)?;
                    let b = region(sketch_of(doc, part, end.sketch)?, &end.region)?;
                    if a.holes.len() != b.holes.len() {
                        return Err(DocumentError(
                            "Os perfis da transição precisam ter os mesmos furos.".into(),
                        ));
                    }
                    let keys = |r: &nator_sketch::profiles::Region| {
                        let mut keys = r.holes.iter().map(hole_key).collect::<Vec<_>>();
                        keys.sort();
                        keys
                    };
                    keys(&a)
                        .into_iter()
                        .zip(keys(&b))
                        .map(|(start, end)| nator_document::features::HolePair { start, end })
                        .collect()
                }
            };
            Operation::Loft {
                start,
                end,
                ruled,
                developable,
                holes: Some(holes),
            }
        }
        Operation::Revolve {
            sketch,
            region,
            axis_origin,
            axis_direction,
            angle,
            axis_line,
            axis_sketch,
            additional_profiles,
        } => Operation::Revolve {
            region: stable_region(sketch, region)?,
            sketch,
            axis_origin,
            axis_direction,
            angle,
            axis_line,
            axis_sketch,
            additional_profiles: additional_profiles
                .into_iter()
                .map(|p| {
                    Ok(nator_document::features::ProfileRef {
                        sketch: p.sketch,
                        region: stable_region(p.sketch, p.region)?,
                    })
                })
                .collect::<Result<_, DocumentError>>()?,
        },
        Operation::Fillet {
            base,
            edge,
            radius,
            additional_edges,
        } => Operation::Fillet {
            edge: stable_edge(doc, part, base, edge)?,
            additional_edges: additional_edges
                .into_iter()
                .map(|e| stable_edge(doc, part, base, e))
                .collect::<Result<_, _>>()?,
            base,
            radius,
        },
        Operation::Chamfer {
            base,
            edge,
            distance,
            angle,
            additional_edges,
        } => Operation::Chamfer {
            edge: stable_edge(doc, part, base, edge)?,
            additional_edges: additional_edges
                .into_iter()
                .map(|e| stable_edge(doc, part, base, e))
                .collect::<Result<_, _>>()?,
            base,
            distance,
            angle,
        },
        Operation::Shell {
            base,
            face,
            thickness,
            additional_faces,
            outward,
            through,
            exit_face,
        } => Operation::Shell {
            face: stable_face(doc, part, base, face)?,
            additional_faces: additional_faces
                .into_iter()
                .map(|f| stable_face(doc, part, base, f))
                .collect::<Result<_, _>>()?,
            base,
            thickness,
            outward,
            through,
            exit_face: exit_face
                .map(|f| stable_face(doc, part, base, f))
                .transpose()?,
        },
        Operation::MergeFaces { base, faces } => Operation::MergeFaces {
            faces: faces
                .into_iter()
                .map(|face| stable_face(doc, part, base, face))
                .collect::<Result<_, _>>()?,
            base,
        },
        Operation::Lip { base, face, spec } => Operation::Lip {
            face: stable_face(doc, part, base, face)?,
            base,
            spec,
        },
        Operation::Split {
            base,
            origin,
            normal,
            cutting_plane,
            reverse,
        } => Operation::Split {
            base,
            origin,
            normal,
            reverse,
            cutting_plane: cutting_plane
                .map(|plane| match plane {
                    ExtrusionLimit::Face(UntilFace { body, face }) => {
                        Ok(ExtrusionLimit::Face(UntilFace {
                            body,
                            face: stable_face(doc, part, body, face)?,
                        }))
                    }
                    other => Ok(other),
                })
                .transpose()?,
        },
        other => other,
    })
}

#[cfg(feature = "occt")]
fn stable_face(
    doc: &Document,
    part: PartId,
    base: BodyId,
    face: FaceRef,
) -> Result<FaceRef, DocumentError> {
    let FaceRef::Index { index, signature } = face else {
        return Ok(face);
    };
    let (solid, bytes) = source(doc, part, base)?;
    same_geometry(&bytes, signature)?;
    Ok(match solid.face_name(index) {
        Ok(name) if solid.face_named(name).is_ok_and(|found| found == index) => {
            FaceRef::Named(name.to_owned())
        }
        _ => FaceRef::Index { index, signature },
    })
}

#[cfg(feature = "occt")]
fn stable_edge(
    doc: &Document,
    part: PartId,
    base: BodyId,
    edge: EdgeRef,
) -> Result<EdgeRef, DocumentError> {
    let EdgeRef::Index { index, signature } = edge else {
        return Ok(edge);
    };
    let (solid, bytes) = source(doc, part, base)?;
    same_geometry(&bytes, signature)?;
    Ok(match solid.edge_face_names(index) {
        Ok(faces) if solid.edge_between(&faces).is_ok() => EdgeRef::Between(faces),
        // Par sem nome ou repetido: só o índice identifica a aresta nesta geometria.
        _ => EdgeRef::Index { index, signature },
    })
}

#[cfg(not(feature = "occt"))]
pub(super) fn normalize(
    _doc: &Document,
    _part: PartId,
    operation: Operation,
) -> Result<Operation, DocumentError> {
    Ok(operation)
}

#[cfg(feature = "occt")]
pub(super) fn construct(
    doc: &Document,
    part: PartId,
    output: BodyId,
    operation: &Operation,
) -> Result<Built, DocumentError> {
    let piece = doc
        .part(part)
        .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
    let value = |expression: &str| piece.parameters.evaluate(expression, Dimension::Length);
    let owner = body_owner(output);
    let mut label_colors = None;
    let mut solid = match operation {
        operation @ (Operation::Extrude { combine, .. }
        | Operation::ExtrudeFace { combine, .. }) => {
            let guide = crate::extrusion::guide(doc, part, operation)?;
            if (guide.to - guide.from).abs() < 1e-7 {
                crate::extrusion::imprint(doc, part, operation, &owner)?
            } else {
                let extrusion = crate::extrusion::tool(doc, part, operation, &owner, None)?;
                match combine {
                    // O corpo existente é a base: o corte remove a extrusão dele.
                    Some(combine) => {
                        let joined = source(doc, part, combine.target)?
                            .0
                            .boolean(&extrusion, boolean_mode(combine.mode), &owner)
                            .map_err(error)?;
                        if combine.merge_coplanar && combine.mode == BooleanMode::Join {
                            merge_new_coplanar(joined, &owner)?
                        } else {
                            joined
                        }
                    }
                    None => extrusion,
                }
            }
        }
        Operation::Label { base, face, spec } => {
            let (solid, bytes) = source(doc, part, *base)?;
            let (result, faces) = solid
                .label(
                    face_index(&solid, &bytes, face)?,
                    &spec.outline,
                    nator_kernel::TextPlacement {
                        point: spec.position,
                        uv: spec.uv,
                        size_mm: value(&spec.size)?,
                        angle_rad: piece
                            .parameters
                            .evaluate(&spec.rotation, Dimension::Angle)?,
                        depth_mm: value(&spec.depth)?,
                        mode: match spec.mode {
                            nator_document::features::LabelMode::Surface => {
                                nator_kernel::TextMode::Surface
                            }
                            nator_document::features::LabelMode::Raised => {
                                nator_kernel::TextMode::Raised
                            }
                            nator_document::features::LabelMode::Engraved => {
                                nator_kernel::TextMode::Engraved
                            }
                        },
                    },
                    &owner,
                )
                .map_err(error)?;
            label_colors = Some((faces, spec.color));
            result
        }
        Operation::Pipe {
            sketch,
            entities,
            diameter,
            thickness,
        } => crate::pipe::build(
            sketch_of(doc, part, *sketch)?,
            entities,
            value(diameter)?,
            thickness.as_ref().map(|t| value(t)).transpose()?,
            &owner,
        )?,
        Operation::Sweep {
            start,
            end,
            route,
            holes,
        } => {
            let a = sketch_of(doc, part, start.sketch)?;
            let b = sketch_of(doc, part, end.sketch)?;
            if nator_document::features::validate_coplanar(a.plane, b.plane).is_ok() {
                return Err(DocumentError(
                    "Escolha planos diferentes para a transição guiada.".into(),
                ));
            }
            let mut first = region(a, &start.region)?;
            let mut last = region(b, &end.region)?;
            if let Some(pairs) = holes {
                order_holes(
                    &mut first,
                    &pairs.iter().map(|p| p.start.clone()).collect::<Vec<_>>(),
                )?;
                order_holes(
                    &mut last,
                    &pairs.iter().map(|p| p.end.clone()).collect::<Vec<_>>(),
                )?;
            }
            let curves = crate::sweep::curves(doc, part, (a, &first), (b, &last), route)?;
            Solid::guided_transition((a, &first), (b, &last), &curves, &owner).map_err(error)?
        }
        Operation::Loft {
            start,
            end,
            ruled,
            developable,
            holes,
        } => {
            let a = sketch_of(doc, part, start.sketch)?;
            let b = sketch_of(doc, part, end.sketch)?;
            if nator_document::features::validate_coplanar(a.plane, b.plane).is_ok() {
                return Err(DocumentError(
                    "Escolha perfis em planos diferentes para criar a transição.".into(),
                ));
            }
            let mut first = region(a, &start.region)?;
            let mut last = region(b, &end.region)?;
            if let Some(pairs) = holes {
                order_holes(
                    &mut first,
                    &pairs.iter().map(|p| p.start.clone()).collect::<Vec<_>>(),
                )?;
                order_holes(
                    &mut last,
                    &pairs.iter().map(|p| p.end.clone()).collect::<Vec<_>>(),
                )?;
            }
            Solid::loft((a, &first), (b, &last), *ruled, *developable, &owner).map_err(error)?
        }
        Operation::Revolve {
            sketch: primary_id,
            region: reference,
            axis_origin,
            axis_direction,
            angle,
            axis_line,
            axis_sketch,
            additional_profiles,
        } => {
            let sketch = sketch_of(doc, part, *primary_id)?;
            let (origin, direction) = match axis_line {
                Some(line) => {
                    let axis = sketch_of(doc, part, axis_sketch.unwrap_or(*primary_id))?;
                    nator_document::features::validate_coplanar(sketch.plane, axis.plane)?;
                    sketch_axis(axis, *line)?
                }
                None => (*axis_origin, *axis_direction),
            };
            let angle = piece.parameters.evaluate(angle, Dimension::Angle)?;
            let mut result: Option<Solid> = None;
            let profiles = std::iter::once((*primary_id, reference))
                .chain(additional_profiles.iter().map(|p| (p.sketch, &p.region)));
            for (id, reference) in profiles {
                let profile = sketch_of(doc, part, id)?;
                nator_document::features::validate_coplanar(sketch.plane, profile.plane)?;
                for selection in reference.selections() {
                    let mut solid = Solid::revolve(
                        profile,
                        &region(profile, selection)?,
                        origin,
                        direction,
                        angle,
                        &owner,
                    )
                    .map_err(error)?;
                    // IDs de entidades são locais a cada rascunho. Não atribuir
                    // nomes ambíguos às faces de outro rascunho: essas seleções
                    // usam índice + assinatura até existir um dono composto.
                    if id != *primary_id {
                        solid.map_face_names(|_| String::new());
                    }
                    result = Some(match result {
                        None => solid,
                        Some(previous) => previous.boolean(&solid, 0, &owner).map_err(error)?,
                    });
                }
            }
            result.ok_or_else(|| DocumentError("Escolha ao menos uma área de revolução.".into()))?
        }
        Operation::Boolean { base, tool, mode } => source(doc, part, *base)?
            .0
            .boolean(&source(doc, part, *tool)?.0, boolean_mode(*mode), &owner)
            .map_err(error)?,
        Operation::ComponentCut {
            base,
            outline,
            clearance,
            floor,
            depth,
        } => crate::fit::cut(
            doc,
            part,
            *base,
            outline,
            value(clearance)?,
            value(floor)?,
            depth.as_deref().map(value).transpose()?,
            &owner,
        )?,
        Operation::MergeFaces { base, faces } => {
            let (solid, bytes) = source(doc, part, *base)?;
            let indices = faces
                .iter()
                .map(|face| face_index(&solid, &bytes, face))
                .collect::<Result<Vec<_>, _>>()?;
            let mut plane = None;
            for index in &indices {
                let nator_kernel::FaceKind::Plane { normal, offset } =
                    solid.face_measure(*index).map_err(error)?.kind
                else {
                    return Err(DocumentError("Só áreas planas podem ser unidas.".into()));
                };
                let (reference, height) = *plane.get_or_insert((normal, offset));
                if nator_geometry::norm(nator_geometry::sub(normal, reference)) > 1e-7
                    || (offset - height).abs() > 1e-6
                {
                    return Err(DocumentError(
                        "As áreas a unir precisam estar no mesmo plano.".into(),
                    ));
                }
            }
            solid.merge_faces(&indices, &owner).map_err(error)?
        }
        Operation::Lip { base, face, spec } => {
            let (solid, bytes) = source(doc, part, *base)?;
            let index = face_index(&solid, &bytes, face)?;
            crate::fastening::lip(&solid, index, spec, &piece.parameters, &owner)?
        }
        Operation::ScrewBoss {
            base,
            sketch,
            points,
            spec,
        } => crate::fastening::bosses(
            source(doc, part, *base)?.0,
            sketch_of(doc, part, *sketch)?,
            points,
            spec,
            &piece.parameters,
            &owner,
        )?,
        Operation::SnapFit {
            base,
            sketch,
            points,
            spec,
        } => crate::fastening::snaps(
            source(doc, part, *base)?.0,
            sketch_of(doc, part, *sketch)?,
            points,
            spec,
            &piece.parameters,
            &owner,
        )?,
        Operation::RepairContacts {
            base,
            contacts,
            mode,
            radius,
        } => crate::solid_check::repair(
            source(doc, part, *base)?.0,
            contacts,
            *mode,
            value(radius)?,
            &owner,
        )?,
        Operation::Fillet {
            base,
            edge,
            radius,
            additional_edges,
        } => {
            let (solid, bytes) = source(doc, part, *base)?;
            let edges = std::iter::once(edge)
                .chain(additional_edges)
                .map(|e| edge_index(&solid, &bytes, e))
                .collect::<Result<Vec<_>, _>>()?;
            solid
                .fillet(&edges, value(radius)?, &owner)
                .map_err(error)?
        }
        Operation::Chamfer {
            base,
            edge,
            distance,
            angle,
            additional_edges,
        } => {
            let (solid, bytes) = source(doc, part, *base)?;
            let edges = std::iter::once(edge)
                .chain(additional_edges)
                .map(|e| edge_index(&solid, &bytes, e))
                .collect::<Result<Vec<_>, _>>()?;
            let angle = angle
                .as_deref()
                .map(|a| piece.parameters.evaluate(a, Dimension::Angle))
                .transpose()?;
            solid
                .chamfer(&edges, value(distance)?, angle, &owner)
                .map_err(error)?
        }
        Operation::Shell {
            base,
            face,
            thickness,
            additional_faces,
            outward,
            through,
            exit_face,
        } => {
            let (solid, bytes) = source(doc, part, *base)?;
            let mut faces = std::iter::once(face)
                .chain(additional_faces)
                .map(|f| face_index(&solid, &bytes, f))
                .collect::<Result<Vec<_>, _>>()?;
            if *through {
                let exit = match exit_face {
                    Some(face) => face_index(&solid, &bytes, face)?,
                    None => solid.opposite_face(faces[0]).map_err(error)?,
                };
                if faces.contains(&exit) {
                    return Err(DocumentError(
                        "A saída deve ser diferente das faces de entrada.".into(),
                    ));
                }
                faces.push(exit);
            }
            solid
                .shell(&faces, value(thickness)?, *outward, &owner)
                .map_err(error)?
        }
        Operation::Hole {
            base,
            sketch,
            points,
            spec,
        } => {
            let sketch = sketch_of(doc, part, *sketch)?;
            let (mut solid, _) = source(doc, part, *base)?;
            let body = doc
                .body(*base)
                .ok_or_else(|| DocumentError("Corpo do furo inexistente.".into()))?;
            let bounds = body.geometry.mesh().bounds();
            let center = body.placement.apply(bounds.center());
            let diagonal = nator_geometry::norm(bounds.size());
            let normal = sketch.plane.normal();
            let direction = if spec.reverse {
                normal
            } else {
                normal.map(|v| -v)
            };
            for (index, point) in points.iter().enumerate() {
                let position = sketch.plane.world(sketch.point(*point).map_err(|_| {
                    DocumentError(
                        "Um ponto do furo não existe mais no rascunho; selecione os pontos novamente."
                            .into(),
                    )
                })?);
                // Passante: profundidade que certamente atravessa o corpo a partir do ponto.
                let through =
                    2. * (diagonal + nator_geometry::norm(nator_geometry::sub(position, center)));
                let geometry = hole_geometry(spec, &piece.parameters, through)?;
                let tool = Solid::hole_tool(position, direction, &geometry, &owner, index)
                    .map_err(error)?;
                solid = solid.boolean(&tool, 1, &owner).map_err(error)?;
            }
            solid
        }
        Operation::Pattern { base, layout } => {
            let placements =
                nator_document::features::pattern_placements(layout, &piece.parameters)?;
            source(doc, part, *base)?
                .0
                .pattern(&placements, &owner)
                .map_err(error)?
        }
        Operation::Mirror {
            base,
            origin,
            normal,
            keep_original,
        } => {
            let (solid, _) = source(doc, part, *base)?;
            let mut image = solid.mirrored(*origin, *normal).map_err(error)?;
            if *keep_original {
                // A imagem recebe sufixo para que suas faces não repitam os nomes originais.
                image.map_face_names(|name| format!("{name}:m"));
                solid.boolean(&image, 0, &owner).map_err(error)?
            } else {
                image
            }
        }
        operation @ Operation::Split { base, .. } => {
            let (origin, normal) = division_plane(doc, part, operation)?;
            source(doc, part, *base)?
                .0
                .split(origin, normal, &owner)
                .map_err(error)?
        }
        Operation::SplitRemainder { base, division } => {
            let primary = piece
                .features
                .iter()
                .find(|f| f.output == *division)
                .ok_or_else(|| DocumentError("Divisão primária inexistente.".into()))?;
            if !matches!(primary.operation,Operation::Split {base:source,..} if source==*base) {
                return Err(DocumentError(
                    "A divisão complementar exige a mesma base da etapa primária.".into(),
                ));
            }
            let (origin, normal) = division_plane(doc, part, &primary.operation)?;
            source(doc, part, *base)?
                .0
                .split(origin, normal.map(|v| -v), &owner)
                .map_err(error)?
        }
    };
    solid.volume().map_err(error)?;
    let brep = Arc::new(solid.to_brep().map_err(error)?);
    let faces = Arc::new(solid.face_names().to_vec());
    let (mesh, triangle_faces) = solid.tessellate_with_faces(0.05, 0.25).map_err(error)?;
    let mesh = Arc::new(MeshAsset::with_faces(mesh, triangle_faces)?);
    let mut intrinsic_colors = inherited_colors(doc, part, operation, &solid, &owner)?;
    if let Some((faces, color)) = label_colors {
        intrinsic_colors.resize(solid.face_count().map_err(error)?, None);
        for face in faces {
            if let Some(value) = intrinsic_colors.get_mut(face) {
                *value = Some(color);
            }
        }
    }
    let intrinsic_colors = Arc::new(intrinsic_colors);
    Ok(Built {
        mesh,
        brep,
        faces,
        intrinsic_colors,
    })
}
#[cfg(not(feature = "occt"))]
pub(super) fn construct(
    _doc: &Document,
    _part: PartId,
    _output: BodyId,
    _operation: &Operation,
) -> Result<Built, DocumentError> {
    nator_kernel::require_available().map_err(|e| DocumentError(e.to_string()))?;
    Err(DocumentError("Construção CAD indisponível.".into()))
}

/// Excluir uma operação nunca deixa referências silenciosamente apontando para outra geometria:
/// sem `with_dependents`, a existência de dependentes é um erro que os nomeia; com ele, todas
/// as dependentes são excluídas na mesma ação.
pub(super) fn remove(
    doc: &mut Document,
    part: PartId,
    body: BodyId,
    with_dependents: bool,
) -> Result<(), DocumentError> {
    use nator_document::features;
    let piece = doc.part_mut(part)?;
    if !piece.features.iter().any(|f| f.output == body) {
        return Err(DocumentError("Operação inexistente.".into()));
    }
    let dependents = features::dependents(piece, body);
    if !dependents.is_empty() && !with_dependents {
        let names = piece
            .features
            .iter()
            .filter(|f| dependents.contains(&f.output))
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(DocumentError(format!(
            "Operações dependem deste resultado: {names}. Exclua-as também ou cancele."
        )));
    }
    let removed = std::iter::once(body)
        .chain(dependents)
        .collect::<std::collections::HashSet<_>>();
    let before = features::hidden_by_history(piece);
    // O marcador conta operações: descontar as excluídas que estavam antes dele.
    if let Some(marker) = piece.history_marker {
        let earlier = piece.features[..marker]
            .iter()
            .filter(|f| removed.contains(&f.output))
            .count();
        piece.history_marker = Some(marker - earlier);
    }
    piece.features.retain(|f| !removed.contains(&f.output));
    piece.bodies.retain(|b| !removed.contains(&b.id));
    if piece
        .history_marker
        .is_some_and(|marker| marker >= piece.features.len())
    {
        piece.history_marker = None;
    }
    features::sync_visibility(piece, &before);
    Ok(())
}

/// Move a operação `output` para a posição `index`, conferindo que tudo o que ela usa
/// continue antes dela e que tudo o que usa seu resultado continue depois. Com a ordem válida,
/// a visibilidade derivada do histórico é ajustada (o marcador conta posições).
pub(super) fn reorder(
    piece: &mut nator_document::Part,
    output: BodyId,
    index: usize,
) -> Result<(), DocumentError> {
    use nator_document::features;
    let from = piece
        .features
        .iter()
        .position(|f| f.output == output)
        .ok_or_else(|| DocumentError("Operação inexistente.".into()))?;
    if index >= piece.features.len() {
        return Err(DocumentError("Posição do histórico inválida.".into()));
    }
    let before = features::hidden_by_history(piece);
    let feature = piece.features.remove(from);
    piece.features.insert(index, feature);
    let producers = piece
        .features
        .iter()
        .map(|f| (f.output, f.name.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let mut built = std::collections::HashSet::new();
    for feature in &piece.features {
        for reference in feature.operation.references() {
            if let Some(producer) = producers.get(&reference)
                && !built.contains(&reference)
            {
                return Err(DocumentError(format!(
                    "“{}” usa o resultado de “{producer}”, que ficaria depois dela no histórico.",
                    feature.name
                )));
            }
        }
        built.insert(feature.output);
    }
    features::sync_visibility(piece, &before);
    Ok(())
}

/// O segundo corpo sempre consulta o plano da divisão primária, inclusive após
/// mover seu plano de trabalho ou uma face de apoio.
#[cfg(feature = "occt")]
fn division_plane(
    doc: &Document,
    part: PartId,
    operation: &Operation,
) -> Result<([f64; 3], [f64; 3]), DocumentError> {
    let Operation::Split {
        origin,
        normal,
        cutting_plane,
        reverse,
        ..
    } = operation
    else {
        return Err(DocumentError(
            "A operação referenciada não é uma divisão.".into(),
        ));
    };
    match cutting_plane {
        Some(plane) => {
            let (origin, normal) = crate::extrusion::limit_plane(doc, part, plane)?;
            Ok((origin, if *reverse { normal.map(|v| -v) } else { normal }))
        }
        None => Ok((*origin, *normal)),
    }
}
