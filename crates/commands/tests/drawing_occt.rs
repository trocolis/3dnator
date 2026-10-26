//! Vistas de B-rep: oclusão exata entre corpos e curvas independentes da tesselação.
#![cfg(feature = "occt")]
use nator_commands::{Command, Session, drawing};
use nator_document::{Document, MeshAsset};
use nator_drawing::{Primitive, Specification, View};
use nator_kernel::{Solid, projected_edges};
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
        ..Specification::default()
    };
    let sheet = drawing::generate(session.document(), Some(part), &spec)?;
    assert!(sheet.exact_projection);
    assert!(
        sheet
            .primitives
            .iter()
            .any(|p| matches!(p,Primitive::Text {value,..} if value.contains("HLR DO B-REP")))
    );
    assert!(nator_drawing::pdf(&sheet)?.starts_with(b"%PDF-1.4"));
    session.execute(Command::ImportMesh {
        destination: Some(part),
        name: "Referência".into(),
        geometry: Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
    })?;
    let approximate = drawing::generate(session.document(), Some(part), &spec)?;
    assert!(!approximate.exact_projection);
    assert!(
        approximate.primitives.iter().any(
            |p| matches!(p,Primitive::Text {value,..} if value.contains("DERIVADAS DA MALHA"))
        )
    );
    Ok(())
}
