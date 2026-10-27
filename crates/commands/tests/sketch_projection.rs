//! Apoios do modelo no plano do rascunho: coordenadas exatas, visibilidade e ocorrências.
use nator_commands::sketch_projection;
use nator_document::{Document, MeshAsset, SketchId};
use nator_geometry::{Mesh, Placement};
use nator_sketch::{Plane, Sketch};
use std::sync::Arc;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
