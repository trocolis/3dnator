//! Apoios do modelo no plano do rascunho: coordenadas exatas, visibilidade e ocorrências.
use nator_commands::sketch_projection;
use nator_document::{Document, MeshAsset, SketchId};
use nator_geometry::{Mesh, Placement};
use nator_sketch::{Plane, Sketch};
use std::sync::Arc;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn visible_sketch_areas_edges_and_vertices_project_with_holes_and_exclude_the_target() -> Result<()>
{
    use sketch_projection::Kind;
    let mut source = Sketch::new("Origem", Plane::xy());
    source.add_rectangle([0., 0.], [20., 10.], false)?;
    source.add_circle([10., 5.], 2., false)?;
    let mut document = Document::new("Projetar rascunho")?;
    let (part, _) = document.add_sketch(None, source)?;
    let target = edited(
        &mut document,
        part,
        Plane {
            origin: [0., 0., 5.],
            ..Plane::xy()
        },
    )?;
    let elements = sketch_projection::elements_for_sketch(
        &document,
        &document.occurrences()?,
        Plane::xy(),
        Placement::default(),
        Some(target),
    )?;
    let area = sketch_projection::pick(&elements, [3., 3.], 0.1, Some(Kind::Plane))
        .ok_or("Área ausente")?;
    assert!(elements[area].contours.len() > 1);
    let vertex = sketch_projection::pick(&elements, [0., 0.], 0.1, Some(Kind::Vertex))
        .ok_or("Vértice ausente")?;
    assert!(!elements[vertex].vertices.is_empty());
    assert!(sketch_projection::pick(&elements, [5., 0.], 0.1, Some(Kind::Edge)).is_some());
    let mut projected = Sketch::new("Captura", Plane::xy());
    nator_sketch::projection::insert(&mut projected, &elements[area].curves, &[])?;
    assert!(
        nator_sketch::areas::regions(&projected)?
            .iter()
            .any(|r| r.holes.len() == 1)
    );
    Ok(())
}

#[test]
#[cfg(feature = "occt")]
fn multiple_projected_areas_extrude_as_one_operation_and_survive_roundtrip() -> Result<()> {
    use nator_commands::{Command, Effect, Session};
    use nator_document::features::{Operation, RegionRef};
    use nator_sketch::profiles::Segment;
    let mut sketch = Sketch::new("Projetado", Plane::xy());
    for (x, size) in [(0., 10.), (20., 5.)] {
        let corners = [[x, 0.], [x + size, 0.], [x + size, size], [x, size]];
        let curves = (0..4)
            .map(|i| Segment::Line {
                a: corners[i],
                b: corners[(i + 1) % 4],
            })
            .collect::<Vec<_>>();
        nator_sketch::projection::insert(&mut sketch, &curves, &[])?;
    }
    let regions = nator_sketch::areas::regions(&sketch)?;
    assert_eq!(regions.len(), 2);
    let selections = regions
        .iter()
        .map(|r| nator_sketch::areas::signature(&sketch, r).map(RegionRef::Area))
        .collect::<nator_sketch::Result<Vec<_>>>()?;
    let mut doc = Document::new("Áreas")?;
    let (part, id) = doc.add_sketch(None, sketch)?;
    let mut session = Session::new(doc)?;
    let Effect::Imported { body, .. } = session.execute(Command::AddFeature {
        part,
        name: "Extrusão".into(),
        operation: Operation::Extrude {
            sketch: id,
            region: RegionRef::Multiple(selections),
            from: "0 mm".into(),
            to: "4 mm".into(),
            combine: None,
            until: None,
        },
    })?
    else {
        return Err("Corpo ausente".into());
    };
    let solid = nator_kernel::Solid::from_brep(
        session
            .document()
            .body(body)
            .ok_or("Corpo ausente")?
            .brep
            .as_ref()
            .ok_or("B-rep ausente")?,
        None,
    )?;
    assert!((solid.volume()? - 500.).abs() < 1e-5);
    let original_names = session
        .document()
        .body(body)
        .ok_or("Corpo ausente")?
        .face_names
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let mut operation = session
        .document()
        .part(part)
        .ok_or("Peça ausente")?
        .features[0]
        .operation
        .clone();
    if let Operation::Extrude {
        region: RegionRef::Multiple(regions),
        ..
    } = &mut operation
    {
        regions.reverse();
    }
    session.execute(Command::EditFeature {
        part,
        output: body,
        operation,
    })?;
    let reordered_names = session
        .document()
        .body(body)
        .ok_or("Corpo ausente")?
        .face_names
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        original_names, reordered_names,
        "Reordenar áreas não pode trocar a identidade das faces."
    );
    assert!(session.undo());
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("areas.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    assert!(matches!(
        reopened.part(part).ok_or("Peça ausente")?.features[0].operation,
        Operation::Extrude {
            region: RegionRef::Multiple(_),
            ..
        }
    ));
    assert!(session.undo());
    assert!(session.document().body(body).is_none());
    assert!(session.redo());
    Ok(())
}

fn rectangle() -> Result<Arc<MeshAsset>> {
    Ok(Arc::new(MeshAsset::new(Mesh::new(
        vec![[0., 0., 7.], [20., 0., 7.], [20., 10., 7.], [0., 10., 7.]],
        vec![[0, 1, 2], [0, 2, 3]],
    )?)))
}
fn has(points: &[[f64; 2]], expected: [f64; 2]) -> bool {
    points
        .iter()
        .any(|p| nator_sketch::distance(*p, expected) < 1e-8)
}
fn edited(document: &mut Document, part: nator_document::PartId, plane: Plane) -> Result<SketchId> {
    Ok(document
        .add_sketch(Some(part), Sketch::new("Editado", plane))?
        .1)
}

#[test]
fn model_midpoints_and_polygon_centers_are_projected_onto_an_offset_sketch_plane() -> Result<()> {
    let mut document = Document::new("Projeção")?;
    let (part, _) = document.add_mesh(None, "Retângulo".into(), rectangle()?)?;
    document.add_mesh(
        Some(part),
        "Triângulo".into(),
        Arc::new(MeshAsset::new(Mesh::new(
            vec![[50., 0., 9.], [62., 0., 9.], [50., 6., 9.]],
            vec![[0, 1, 2]],
        )?)),
    )?;
    let plane = Plane {
        origin: [0., 0., 40.],
        ..Plane::xy()
    };
    let id = edited(&mut document, part, plane)?;
    let points = sketch_projection::points(
        &document,
        &document.occurrences()?,
        id,
        plane,
        Placement::default(),
    )?;
    for expected in [
        [10., 5.],
        [10., 0.],
        [20., 5.],
        [10., 10.],
        [0., 5.],
        [54., 2.],
        [56., 0.],
        [56., 3.],
    ] {
        assert!(has(&points, expected), "Apoio ausente: {expected:?}");
        assert_eq!(plane.world(expected)[2], 40.);
    }
    assert!(
        !has(&points, [60., 3.]),
        "A diagonal da malha não define o centro do triângulo"
    );
    assert!(
        document
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch
            .entities
            .is_empty()
    );
    Ok(())
}

#[test]
fn projection_composes_body_occurrence_and_edited_frame_before_dropping_height() -> Result<()> {
    let mut document = Document::new("Referenciais")?;
    let (part, body) = document.add_mesh(None, "Retângulo".into(), rectangle()?)?;
    let body_pose = Placement::new([4., -3., 9.], [0.3, 0.2, -0.4])?;
    let occurrence = Placement::new([100., -50., 30.], [-0.5, 0.6, 0.8])?;
    let target = Placement::new([-30., 12., 8.], [0.2, -0.7, 0.9])?;
    document.body_mut(body)?.placement = body_pose;
    let instance = document.instances()[0].id;
    document.instance_mut(instance)?.placement = occurrence;
    let plane = Plane::from_normal([3., 2., 40.], [1., 2., 3.])?;
    let id = edited(&mut document, part, plane)?;
    let points = sketch_projection::points(&document, &document.occurrences()?, id, plane, target)?;
    for local in [[10., 5., 7.], [10., 0., 7.], [0., 5., 7.]] {
        let part_point = target.inverse_apply(occurrence.apply(body_pose.apply(local)));
        let expected = plane.project(part_point);
        assert!(has(&points, expected));
        let foot = plane.world(expected);
        let displacement = nator_geometry::sub(part_point, foot);
        assert!(nator_geometry::dot(displacement, plane.u).abs() < 1e-8);
        assert!(nator_geometry::dot(displacement, plane.v).abs() < 1e-8);
    }
    Ok(())
}

#[test]
fn projection_includes_later_occurrences_and_respects_hidden_sources_and_the_edited_sketch()
-> Result<()> {
    let mut document = Document::new("Ocorrências")?;
    let (part, body) = document.add_mesh(None, "Retângulo".into(), rectangle()?)?;
    for i in 1..12 {
        document.add_instance(
            part,
            None,
            Placement::new([i as f64 * 100., 0., 0.], [0.; 3])?,
        )?;
    }
    let id = edited(&mut document, part, Plane::xy())?;
    let mut source = Sketch::new(
        "Auxiliar",
        Plane {
            origin: [0., 0., 20.],
            ..Plane::xy()
        },
    );
    source.add_rectangle([-40., -20.], [-20., 0.], true)?;
    let (_, source) = document.add_sketch(Some(part), source)?;
    let mut own = document
        .sketch(id)
        .ok_or("Rascunho ausente")?
        .sketch
        .clone();
    own.add_rectangle([-900., -900.], [-800., -800.], false)?;
    document.update_sketch(id, own)?;
    let occurrences = document.occurrences()?;
    let points = sketch_projection::points(
        &document,
        &occurrences,
        id,
        Plane::xy(),
        Placement::default(),
    )?;
    assert!(has(&points, [1110., 5.]));
    assert!(has(&points, [1110., 0.]));
    assert!(has(&points, [-30., -10.]));
    assert!(!has(&points, [-850., -850.]));
    let last = occurrences.last().ok_or("Ocorrência ausente")?;
    let points = sketch_projection::points(
        &document,
        std::slice::from_ref(last),
        id,
        Plane::xy(),
        Placement::default(),
    )?;
    assert!(has(&points, [1110., 5.]));
    assert!(!has(&points, [10., 5.]));
    document.body_mut(body)?.visible = false;
    document.sketch_mut(source)?.visible = false;
    assert_eq!(
        sketch_projection::points(
            &document,
            &occurrences,
            id,
            Plane::xy(),
            Placement::default()
        )?,
        vec![[0., 0.]]
    );
    Ok(())
}

#[cfg(feature = "occt")]
#[test]
fn cad_face_centers_and_true_edge_midpoints_reach_the_sketch_without_mesh_sampling() -> Result<()> {
    let mut outline = Sketch::new("Triângulo", Plane::xy());
    for (a, b) in [
        ([0., 0.], [12., 0.]),
        ([12., 0.], [0., 6.]),
        ([0., 6.], [0., 0.]),
    ] {
        outline.add_line(a, b, false)?;
    }
    let region = nator_sketch::areas::regions(&outline)?.remove(0);
    let mut solid = nator_kernel::Solid::extrude(&outline, &region, 0., 5., "triangle")?;
    let (mesh, faces) = solid.tessellate_with_faces(0.1, 0.2)?;
    let mut document = Document::new("CAD projetado")?;
    let (part, body) = document.add_mesh(
        None,
        "Prisma".into(),
        Arc::new(MeshAsset::with_faces(mesh, faces)?),
    )?;
    document.body_mut(body)?.brep = Some(Arc::new(solid.to_brep()?));
    document.body_mut(body)?.face_names = Arc::new(nator_kernel::indexed_face_names(
        &nator_document::features::body_owner(body),
        solid.face_count()?,
    ));
    let plane = Plane {
        origin: [0., 0., 30.],
        ..Plane::xy()
    };
    let id = edited(&mut document, part, plane)?;
    document.validate()?;
    let points = sketch_projection::points(
        &document,
        &document.occurrences()?,
        id,
        plane,
        Placement::default(),
    )?;
    for expected in [[4., 2.], [6., 0.], [6., 3.], [0., 3.]] {
        assert!(has(&points, expected), "{expected:?}");
    }
    assert_eq!(
        points
            .iter()
            .filter(|p| nator_sketch::distance(**p, [4., 2.]) < 1e-8)
            .count(),
        1
    );
    Ok(())
}

#[test]
fn project_tool_picks_vertices_before_edges_and_skips_mesh_diagonals() -> Result<()> {
    use sketch_projection::{Kind, elements, pick};
    let mut document = Document::new("Elementos")?;
    let (_, body) = document.add_mesh(None, "Retângulo".into(), rectangle()?)?;
    let occurrences = document.occurrences()?;
    let projected = elements(&document, &occurrences, Plane::xy(), Placement::default())?;
    let selected = pick(&projected, [0., 0.], 0.1, None).ok_or("Vértice ausente")?;
    assert_eq!(projected[selected].kind, Kind::Vertex);
    let selected = pick(&projected, [10., 0.], 0.1, None).ok_or("Aresta ausente")?;
    assert_eq!(projected[selected].kind, Kind::Edge);
    assert!(pick(&projected, [10., 5.], 0.1, Some(Kind::Edge)).is_none());
    let face =
        pick(&projected, [10., 5.], 0.1, Some(Kind::Plane)).ok_or("Área da malha ausente")?;
    assert_eq!(
        projected[face].curves.len(),
        4,
        "A diagonal de triangulação não pode virar contorno."
    );
    document.body_mut(body)?.visible = false;
    assert!(elements(&document, &occurrences, Plane::xy(), Placement::default())?.is_empty());
    Ok(())
}

#[test]
fn disconnected_coplanar_mesh_patches_remain_separate_projection_choices() -> Result<()> {
    let mut points = Vec::new();
    let mut triangles = Vec::new();
    for x in [0., 30.] {
        let base = points.len() as u32;
        points.extend([
            [x, 0., 7.],
            [x + 10., 0., 7.],
            [x + 10., 10., 7.],
            [x, 10., 7.],
        ]);
        triangles.extend([[base, base + 1, base + 2], [base, base + 2, base + 3]]);
    }
    let mut document = Document::new("Áreas da malha")?;
    document.add_mesh(
        None,
        "Malha".into(),
        Arc::new(MeshAsset::new(Mesh::new(points, triangles)?)),
    )?;
    let elements = sketch_projection::elements(
        &document,
        &document.occurrences()?,
        Plane::xy(),
        Placement::default(),
    )?;
    let areas = elements
        .iter()
        .filter(|e| e.kind == sketch_projection::Kind::Plane)
        .collect::<Vec<_>>();
    assert_eq!(areas.len(), 2);
    assert!(
        areas
            .iter()
            .all(|a| a.curves.len() == 4 && a.triangles.len() == 2)
    );
    Ok(())
}

#[test]
fn projected_geometry_round_trips_and_undoes_as_one_sketch_command() -> Result<()> {
    use nator_commands::{Command, Session};
    use nator_sketch::{profiles::Segment, projection, solver};
    let mut document = Document::new("Referências persistentes")?;
    let (_, id) = document.add_sketch(None, Sketch::new("Destino", Plane::xz()))?;
    let mut session = Session::new(document)?;
    let mut sketch = session
        .document()
        .sketch(id)
        .ok_or("Rascunho ausente")?
        .sketch
        .clone();
    projection::insert(
        &mut sketch,
        &[
            Segment::Circle {
                center: [10., 20.],
                radius: 3.,
            },
            Segment::Arc {
                center: [30., 20.],
                a: [35., 20.],
                b: [30., 25.],
                clockwise: false,
            },
        ],
        &[[50., 40.]],
    )?;
    session.execute(Command::UpdateSketch { id, sketch })?;
    assert!(session.undo());
    assert!(
        session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch
            .points
            .is_empty()
    );
    assert!(session.redo());
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("projecao.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    let loaded = &reopened.sketch(id).ok_or("Rascunho ausente")?.sketch;
    assert!(loaded.points.iter().all(|p| p.projected));
    assert!(loaded.entities.iter().all(|e| e.projected));
    let report = solver::solve(loaded)?;
    assert_eq!(report.status, solver::Status::FullyConstrained);
    assert!(!report.redundant);
    Ok(())
}

#[cfg(feature = "occt")]
fn cad_document(mut solid: nator_kernel::Solid) -> Result<Document> {
    let (mesh, faces) = solid.tessellate_with_faces(0.1, 0.2)?;
    let mut document = Document::new("Projetar CAD")?;
    let (_, body) = document.add_mesh(
        None,
        "Componente".into(),
        Arc::new(MeshAsset::with_faces(mesh, faces)?),
    )?;
    document.body_mut(body)?.brep = Some(Arc::new(solid.to_brep()?));
    document.body_mut(body)?.face_names = Arc::new(nator_kernel::indexed_face_names(
        &nator_document::features::body_owner(body),
        solid.face_count()?,
    ));
    Ok(document)
}

#[cfg(feature = "occt")]
#[test]
fn planar_faces_project_exact_boundaries_with_holes_and_component_placements() -> Result<()> {
    use nator_sketch::profiles::Segment;
    use sketch_projection::{Kind, elements, pick};
    let outer = nator_kernel::Solid::cylinder([0.; 3], 10., 5.)?;
    let inner = nator_kernel::Solid::cylinder([0., 0., -1.], 3., 7.)?;
    let mut document = cad_document(outer.cut(&inner)?)?;
    let body = document.parts()[0].bodies[0].id;
    let body_pose = Placement::new([4., -3., 9.], [0.3, 0.2, -0.4])?;
    let occurrence = Placement::new([100., -50., 30.], [-0.5, 0.6, 0.8])?;
    let target = occurrence.compose(body_pose)?;
    document.body_mut(body)?.placement = body_pose;
    let instance = document.instances()[0].id;
    document.instance_mut(instance)?.placement = occurrence;
    let plane = Plane {
        origin: [0., 0., 40.],
        ..Plane::xy()
    };
    let projected = elements(&document, &document.occurrences()?, plane, target)?;
    let index = pick(&projected, [7., 0.], 0.01, Some(Kind::Plane)).ok_or("Face ausente")?;
    let face = &projected[index];
    assert_eq!(face.curves.len(), 2);
    assert!(
        face.curves
            .iter()
            .all(|s| matches!(s, Segment::Circle { .. }))
    );
    assert!(
        pick(&projected, [0., 0.], 0.01, Some(Kind::Plane)).is_none(),
        "O furo não é face"
    );
    let mut sketch = Sketch::new("Face", plane);
    nator_sketch::projection::insert(&mut sketch, &face.curves, &[])?;
    let regions = nator_sketch::profiles::regions(&sketch)?;
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].holes.len(), 1);
    // Inclinada, a mesma borda vira uma polilinha em vez de um círculo falso.
    let inclined = elements(&document, &document.occurrences()?, Plane::xz(), target)?;
    assert!(
        inclined
            .iter()
            .any(|e| e.kind == Kind::Edge && e.curves.len() > 2)
    );
    assert!(
        !inclined
            .iter()
            .flat_map(|e| &e.curves)
            .any(|s| matches!(s, Segment::Circle { .. }))
    );
    Ok(())
}

#[cfg(feature = "occt")]
#[test]
fn cad_arcs_keep_their_radius_and_reverse_direction_with_the_target_normal() -> Result<()> {
    use nator_sketch::{Curve, profiles::Segment};
    let mut profile = Sketch::new("Semicírculo", Plane::xy());
    let center = profile.add_point([0., 0.])?;
    let start = profile.add_point([10., 0.])?;
    let end = profile.add_point([-10., 0.])?;
    profile.add_curve(
        Curve::Arc {
            center,
            start,
            end,
            clockwise: false,
        },
        false,
    )?;
    profile.add_curve(
        Curve::Line {
            start: end,
            end: start,
        },
        false,
    )?;
    let region = nator_sketch::areas::regions(&profile)?.remove(0);
    let document = cad_document(nator_kernel::Solid::extrude(
        &profile, &region, 0., 5., "arc",
    )?)?;
    for (plane, clockwise) in [
        (Plane::xy(), false),
        (
            Plane {
                v: [0., -1., 0.],
                ..Plane::xy()
            },
            true,
        ),
    ] {
        let elements = sketch_projection::elements(
            &document,
            &document.occurrences()?,
            plane,
            Placement::default(),
        )?;
        assert!(elements.iter().flat_map(|e| &e.curves).any(|c| matches!(c,
            Segment::Arc { center, a, clockwise: c, .. } if *c == clockwise && nator_sketch::distance(*center,*a) > 9.999)));
    }
    Ok(())
}

#[test]
#[cfg(feature = "occt")]
fn original_geometry_survives_a_perpendicular_projection_and_legacy_face_mesh() -> Result<()> {
    let mut solid = nator_kernel::Solid::cuboid([0., 0., 10.], [10., 10., 10.])?;
    let mesh = solid.tessellate(0.05, 0.2)?;
    let mut doc = Document::new("Origem fora do plano")?;
    let (_, body) = doc.add_mesh(None, "Bloco".into(), Arc::new(MeshAsset::new(mesh)))?;
    doc.body_mut(body)?.brep = Some(Arc::new(solid.to_brep()?));
    let elements =
        sketch_projection::elements(&doc, &doc.occurrences()?, Plane::xy(), Placement::default())?;
    // Arestas paralelas à normal colapsam em pontos; a origem continua selecionável.
    let edge = elements
        .iter()
        .find(|e| e.kind == sketch_projection::Kind::Edge && e.curves.is_empty())
        .ok_or("Aresta perpendicular ausente")?;
    assert_eq!(edge.vertices.len(), 1);
    assert!(!edge.source.curves[0].is_empty());
    let mut sketch = Sketch::new("Destino", Plane::xy());
    nator_sketch::projection::insert(&mut sketch, &edge.curves, &edge.vertices)?;
    assert_eq!(sketch.points.len(), 1);
    assert!(
        elements
            .iter()
            .filter(|e| e.kind == sketch_projection::Kind::Plane)
            .all(|e| !e.source.triangles.is_empty())
    );
    Ok(())
}
