//! Vistas de B-rep: oclusão exata entre corpos e curvas independentes da tesselação.
#![cfg(feature = "occt")]
use nator_commands::{Command, Session, drawing};
use nator_document::{
    Document, MeshAsset,
    drawing::{
        DrawingAnnotation, DrawingBodyRef, DrawingEdgeRef, DrawingFaceRef, DrawingMeasure,
        DrawingTopologyRef,
    },
    features::{EdgeRef, FaceRef},
};
use nator_drawing::{AnnotationId, Primitive, Section, Specification, View};
use nator_kernel::{FaceKind, Solid, projected_edges};
use std::{error::Error, sync::Arc};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
#[test]
fn hlr_hides_a_body_behind_another_without_mesh_depth_sampling() -> Result<()> {
    let large = Solid::cuboid([0.; 3], [10., 20., 30.])?;
    let small = Solid::cuboid([2., 40., 3.], [4., 10., 6.])?;
    let (right, _, direction) = View::Front.basis();
    let lines = projected_edges(&[large, small], right, direction, 0.01)?;
    let interior = |p: [f64; 2]| p[0] > 1.9 && p[0] < 6.1 && p[1] > 2.9 && p[1] < 9.1;
    assert!(
        lines
            .iter()
            .any(|l| l.hidden && interior(l.a) && interior(l.b))
    );
    assert!(
        !lines
            .iter()
            .any(|l| !l.hidden && interior(l.a) && interior(l.b))
    );
    let visible = lines
        .iter()
        .filter(|l| !l.hidden)
        .map(|l| (l.a[0] - l.b[0]).hypot(l.a[1] - l.b[1]))
        .sum::<f64>();
    assert!((visible - 80.).abs() < 1e-6, "Perímetro visível: {visible}");
    Ok(())
}
#[test]
fn hlr_extracts_a_circular_silhouette_with_an_explicit_curve_tolerance() -> Result<()> {
    let cylinder = Solid::cylinder([0.; 3], 5., 20.)?;
    let (right, _, direction) = View::Top.basis();
    let lines = projected_edges(&[cylinder], right, direction, 0.01)?;
    let visible = lines.iter().filter(|l| !l.hidden).collect::<Vec<_>>();
    assert!(visible.len() > 16);
    for line in visible {
        for p in [line.a, line.b] {
            assert!((p[0].hypot(p[1]) - 5.).abs() < 1e-7);
        }
        let middle = [(line.a[0] + line.b[0]) * 0.5, (line.a[1] + line.b[1]) * 0.5];
        assert!(5. - middle[0].hypot(middle[1]) < 0.011);
    }
    Ok(())
}
#[test]
fn sheets_use_the_brep_and_identify_the_mesh_fallback() -> Result<()> {
    let mut cylinder = Solid::cylinder([0.; 3], 5., 20.)?;
    let faces = cylinder.face_count()?;
    let brep = cylinder.to_brep()?;
    let (mesh, triangle_faces) = cylinder.tessellate_with_faces(1., 1.)?;
    let mut session = Session::new(Document::new("Folha precisa")?)?;
    let effect = session.execute(Command::ImportSolid {
        destination: None,
        name: "Cilindro".into(),
        geometry: Arc::new(MeshAsset::with_faces(mesh, triangle_faces)?),
        brep: Arc::new(brep),
        faces,
    })?;
    let nator_commands::Effect::Imported { part, .. } = effect else {
        return Err("Corpo não criado".into());
    };
    let spec = Specification {
        hidden_lines: true,
        section: Some(Section {
            origin: [0., 0., 10.],
            normal: [0., 0., 1.],
        }),
        ..Specification::default()
    };
    let sheet = drawing::generate(session.document(), Some(part), &spec)?;
    assert!(sheet.exact_projection);
    assert!(sheet.exact_sections);
    assert!(sheet.primitives.iter().any(
        |p| matches!(p,Primitive::Text {value,..} if value.contains("HLR E CORTES DO B-REP"))
    ));
    assert!(nator_drawing::pdf(&sheet)?.starts_with(b"%PDF-1.4"));
    session.execute(Command::ImportMesh {
        destination: Some(part),
        name: "Referência".into(),
        geometry: Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
    })?;
    let approximate = drawing::generate(session.document(), Some(part), &spec)?;
    assert!(!approximate.exact_projection);
    assert!(!approximate.exact_sections);
    assert!(
        approximate.primitives.iter().any(
            |p| matches!(p,Primitive::Text {value,..} if value.contains("DERIVADAS DA MALHA"))
        )
    );
    Ok(())
}

#[test]
fn precise_annular_section_preserves_holes_and_the_source_solid() -> Result<()> {
    let outer = Solid::cylinder([0.; 3], 10., 20.)?;
    let inner = Solid::cylinder([0.; 3], 3., 20.)?;
    let solid = outer.cut(&inner)?;
    let before = solid.volume()?;
    let segments = solid.section_segments([0., 0., 7.], [0., 0., 2.], 0.01)?;
    assert!(segments.iter().flatten().all(|p| (p[2] - 7.).abs() < 1e-8));
    let loops = nator_drawing::section_loops(&segments);
    assert_eq!(loops.len(), 2);
    let areas = loops
        .iter()
        .map(|l| {
            (0..l.len())
                .map(|i| {
                    let (a, b) = (l[i], l[(i + 1) % l.len()]);
                    a[0] * b[1] - b[0] * a[1]
                })
                .sum::<f64>()
                .abs()
                * 0.5
        })
        .collect::<Vec<_>>();
    let max = areas.iter().copied().fold(0., f64::max);
    let min = areas.iter().copied().fold(f64::INFINITY, f64::min);
    assert!((max - min - std::f64::consts::PI * 91.).abs() < 1.);
    assert!((solid.volume()? - before).abs() < 1e-9);
    assert!(
        solid
            .section_segments([0., 0., 50.], [0., 0., 1.], 0.01)?
            .is_empty()
    );
    assert!(
        solid
            .section_segments([0.; 3], [f64::MAX; 3], 0.01)
            .is_err()
    );
    Ok(())
}

#[test]
fn associative_annotations_measure_edges_radii_and_face_relations() -> Result<()> {
    let mut cylinder = Solid::cylinder([0.; 3], 5., 20.)?;
    let faces = cylinder.face_count()?;
    let brep = cylinder.to_brep()?;
    let (mesh, triangle_faces) = cylinder.tessellate_with_faces(1., 1.)?;
    let mut session = Session::new(Document::new("Cotas Associativas")?)?;
    let effect = session.execute(Command::ImportSolid {
        destination: None,
        name: "Cilindro".into(),
        geometry: Arc::new(MeshAsset::with_faces(mesh, triangle_faces)?),
        brep: Arc::new(brep),
        faces,
    })?;
    let nator_commands::Effect::Imported { part, body } = effect else {
        return Err("Corpo não criado".into());
    };
    let doc_body = session
        .document()
        .body(body)
        .ok_or("Corpo ausente no documento")?;
    let face_names = doc_body.face_names.clone();
    assert_eq!(face_names.len(), 3);
    let brep_bytes = doc_body.brep.as_ref().ok_or("B-rep ausente no corpo")?;
    let solid = Solid::from_brep(brep_bytes, Some(&face_names))?;
    let mut cyl_face = None;
    let mut cap_faces = Vec::new();
    for name in face_names.iter() {
        let i = solid.face_named(name)?;
        let m = solid.face_measure(i)?;
        match m.kind {
            FaceKind::Cylinder { .. } => cyl_face = Some(name.clone()),
            FaceKind::Plane { .. } => cap_faces.push(name.clone()),
            _ => {}
        }
    }
    let cyl_face = cyl_face.ok_or("Face cilíndrica não encontrada")?;
    assert_eq!(cap_faces.len(), 2);

    let circular_edge = solid.edge_between(&[cap_faces[0].clone(), cyl_face.clone()])?;
    let circular_edge_faces = solid.edge_face_names(circular_edge)?;

    let target = DrawingBodyRef {
        body,
        path: Vec::new(),
    };

    // 1. Cota de raio na face cilíndrica
    let a1 = DrawingAnnotation {
        id: AnnotationId(1),
        view: View::Front,
        measurement: DrawingMeasure::Radius {
            topology: DrawingTopologyRef::Face(DrawingFaceRef {
                target: target.clone(),
                face: FaceRef::Named(cyl_face),
            }),
            diameter: false,
        },
        offset_mm: [5., 5.],
    };

    // 2. Cota de diâmetro na aresta circular
    let a2 = DrawingAnnotation {
        id: AnnotationId(2),
        view: View::Top,
        measurement: DrawingMeasure::Radius {
            topology: DrawingTopologyRef::Edge(DrawingEdgeRef {
                target: target.clone(),
                edge: EdgeRef::Between(circular_edge_faces),
            }),
            diameter: true,
        },
        offset_mm: [10., -10.],
    };

    // 3. Cota de distância entre as faces planas (altura do cilindro = 20)
    let a3 = DrawingAnnotation {
        id: AnnotationId(3),
        view: View::Front,
        measurement: DrawingMeasure::FaceDistance {
            first: DrawingFaceRef {
                target: target.clone(),
                face: FaceRef::Named(cap_faces[0].clone()),
            },
            second: DrawingFaceRef {
                target: target.clone(),
                face: FaceRef::Named(cap_faces[1].clone()),
            },
        },
        offset_mm: [15., 0.],
    };

    let spec = Specification::default();
    let sheet =
        drawing::generate_with_annotations(session.document(), Some(part), &spec, &[a1, a2, a3])?;

    assert!(sheet.annotation_warnings.is_empty());
    assert_eq!(sheet.annotation_handles.len(), 3);
    assert!(sheet.annotation_handles.iter().any(|h| h.label == "R 5"));
    assert!(sheet.annotation_handles.iter().any(|h| h.label == "Ø 10"));
    assert!(sheet.annotation_handles.iter().any(|h| h.label == "20"));

    // Reposicionamento na folha sem alterar o documento
    let mut sheet_mut = sheet.clone();
    sheet_mut.reposition_annotation(AnnotationId(1), [25., 30.])?;
    let h1 = sheet_mut
        .annotation_handles
        .iter()
        .find(|h| h.id.0 == 1)
        .ok_or("Alça 1 não encontrada")?;
    assert_eq!(h1.offset_mm, [25., 30.]);

    Ok(())
}

#[test]
fn missing_geometry_generates_annotation_warnings_and_survives_save() -> Result<()> {
    let mut box_solid = Solid::cuboid([0.; 3], [10., 20., 30.])?;
    let faces = box_solid.face_count()?;
    let brep = box_solid.to_brep()?;
    let (mesh, triangle_faces) = box_solid.tessellate_with_faces(1., 1.)?;
    let mut session = Session::new(Document::new("Cotas Aviso")?)?;
    let effect = session.execute(Command::ImportSolid {
        destination: None,
        name: "Bloco".into(),
        geometry: Arc::new(MeshAsset::with_faces(mesh, triangle_faces)?),
        brep: Arc::new(brep),
        faces,
    })?;
    let nator_commands::Effect::Imported { part, body } = effect else {
        return Err("Corpo não criado".into());
    };

    let target = DrawingBodyRef {
        body,
        path: Vec::new(),
    };

    // Cota apontando para face inexistente
    let missing_annotation = DrawingAnnotation {
        id: AnnotationId(1),
        view: View::Front,
        measurement: DrawingMeasure::Radius {
            topology: DrawingTopologyRef::Face(DrawingFaceRef {
                target,
                face: FaceRef::Named("b1:face_inexistente".into()),
            }),
            diameter: false,
        },
        offset_mm: [5., 5.],
    };

    let spec = Specification::default();
    let sheet = drawing::generate_with_annotations(
        session.document(),
        Some(part),
        &spec,
        std::slice::from_ref(&missing_annotation),
    )?;

    assert_eq!(sheet.annotation_warnings.len(), 1);
    assert_eq!(sheet.annotation_warnings[0].0.0, 1);
    assert!(sheet.primitives.iter().any(|p| {
        matches!(p, Primitive::Text { value, .. } if value.contains("REVISAR COTA 1"))
    }));

    // Salvar folha com anotações e testar persistência e desfazer/refazer
    session.execute(Command::SetSheet {
        id: None,
        target: Some(part),
        specification: spec.clone(),
        annotations: vec![missing_annotation],
    })?;

    let dir = tempfile::tempdir()?;
    let path = dir.path().join("annotations.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    assert_eq!(reopened.sheets().len(), 1);
    assert_eq!(reopened.sheets()[0].annotations.len(), 1);
    assert_eq!(reopened.sheets()[0].annotations[0].id.0, 1);

    // Desfazer
    assert!(session.undo());
    assert!(session.document().sheets().is_empty());
    assert!(session.redo());
    assert_eq!(session.document().sheets().len(), 1);
    assert_eq!(session.document().sheets()[0].annotations.len(), 1);

    Ok(())
}
