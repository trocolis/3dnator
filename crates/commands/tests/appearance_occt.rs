//! Aparência precisa, recortes com furos e histórico preservam a geometria CAD.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session};
use nator_document::{
    Document, PaintMask, SurfacePaint,
    features::{FaceRef, Operation, RegionRef, body_owner},
};
use nator_geometry::RgbColor;
use nator_sketch::{Plane, Sketch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn painted_area(doc: &Document, body: nator_document::BodyId, material: u8) -> Result<f64> {
    let appearance = doc.body_appearance(body).ok_or("Dado de pintura ausente")?;
    let mesh = appearance
        .geometry
        .as_ref()
        .ok_or("Dado de pintura ausente")?
        .mesh();
    Ok(mesh
        .triangles()
        .iter()
        .zip(appearance.materials.iter())
        .filter(|(_, m)| **m == material)
        .map(|(t, _)| {
            let [a, b, c] = t.map(|i| mesh.vertices()[i as usize]);
            nator_geometry::norm(nator_geometry::cross(
                nator_geometry::sub(b, a),
                nator_geometry::sub(c, a),
            )) / 2.
        })
        .sum())
}
#[test]
fn paint_masks_preserve_solid_support_projected_profiles_recalculate_and_roundtrip() -> Result<()> {
    let mut profile = Sketch::new("Placa", Plane::xy());
    profile.add_rectangle([0., 0.], [20., 20.], false)?;
    let mut doc = Document::new("Pintura")?;
    let (part, sketch) = doc.add_sketch(None, profile)?;
    let mut session = Session::new(doc)?;
    let Effect::Imported { body, .. } = session.execute(Command::AddFeature {
        part,
        name: "Placa".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "5 mm".into(),
            combine: None,
            until: None,
        },
    })?
    else {
        return Err("Corpo ausente".into());
    };
    let mut mask = Sketch::new("Delimitação", Plane::xy());
    let points = [[2., 2.], [12., 2.], [12., 12.], [2., 12.]];
    let curves = (0..4)
        .map(|i| nator_sketch::profiles::Segment::Line {
            a: points[i],
            b: points[(i + 1) % 4],
        })
        .collect::<Vec<_>>();
    nator_sketch::projection::insert(&mut mask, &curves, &[])?;
    mask.add_circle([7., 7.], 2., false)?;
    let regions = nator_sketch::areas::regions(&mask)?;
    let region = regions
        .iter()
        .find(|r| r.holes.len() == 1)
        .ok_or("Área com furo ausente")?;
    let reference = RegionRef::Area(nator_sketch::areas::signature(&mask, region)?);
    let Effect::SketchCreated {
        sketch: mask_id, ..
    } = session.execute(Command::AddSketch {
        destination: Some(part),
        sketch: mask,
    })?
    else {
        return Err("Máscara ausente".into());
    };
    let bytes = session
        .document()
        .body(body)
        .ok_or("Dado de pintura ausente")?
        .brep
        .clone()
        .ok_or("Dado de pintura ausente")?;
    let geometry = session
        .document()
        .body(body)
        .ok_or("Dado de pintura ausente")?
        .geometry
        .clone();
    let face = FaceRef::Named(format!("{}:end", body_owner(body)));
    session.execute(Command::PaintSurface {
        body,
        paint: SurfacePaint {
            face: face.clone(),
            mask: None,
            color: RgbColor::new([50, 50, 220]),
        },
    })?;
    assert!((painted_area(session.document(), body, 1)? - 400.).abs() < 1e-6);
    session.execute(Command::PaintSurface {
        body,
        paint: SurfacePaint {
            face: face.clone(),
            mask: Some(PaintMask {
                sketch: mask_id,
                region: reference,
            }),
            color: RgbColor::new([220, 50, 50]),
        },
    })?;
    let area = 100. - 4. * std::f64::consts::PI;
    assert!((painted_area(session.document(), body, 2)? - area).abs() < 0.1);
    assert!((painted_area(session.document(), body, 1)? - (400. - area)).abs() < 0.1);
    assert_eq!(
        session
            .document()
            .body(body)
            .ok_or("Dado de pintura ausente")?
            .brep
            .as_ref()
            .ok_or("Dado de pintura ausente")?,
        &bytes
    );
    assert!(std::sync::Arc::ptr_eq(
        &session
            .document()
            .body(body)
            .ok_or("Dado de pintura ausente")?
            .geometry,
        &geometry
    ));
    assert!(session.undo());
    assert_eq!(
        session
            .document()
            .body_appearance(body)
            .ok_or("Dado de pintura ausente")?
            .layers
            .len(),
        1
    );
    assert!(session.redo());
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("pintura.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    assert_eq!(
        reopened
            .body_appearance(body)
            .ok_or("Dado de pintura ausente")?
            .layers,
        session
            .document()
            .body_appearance(body)
            .ok_or("Dado de pintura ausente")?
            .layers
    );
    assert!((painted_area(&reopened, body, 2)? - area).abs() < 0.1);
    let mut session = Session::new(reopened)?;
    let mut changed = session
        .document()
        .sketch(mask_id)
        .ok_or("Dado de pintura ausente")?
        .sketch
        .clone();
    let circle = changed
        .entities
        .iter()
        .find(|e| matches!(e.curve, nator_sketch::Curve::Circle { .. }))
        .ok_or("Dado de pintura ausente")?
        .id;
    if let nator_sketch::Curve::Circle { radius, .. } = &mut changed
        .entities
        .iter_mut()
        .find(|e| e.id == circle)
        .ok_or("Dado de pintura ausente")?
        .curve
    {
        *radius = 3.;
    }
    session.execute(Command::UpdateSketch {
        id: mask_id,
        sketch: changed,
    })?;
    assert!(
        (painted_area(session.document(), body, 2)? - (100. - 9. * std::f64::consts::PI)).abs()
            < 0.15
    );
    assert!(
        session
            .execute(Command::RemoveSketch { id: mask_id })
            .is_err()
    );
    // Uma máscara que não alcança a face deve falhar sem publicar cache parcial.
    let revision = session.revision();
    let mut outside = session
        .document()
        .sketch(mask_id)
        .ok_or("Máscara ausente")?
        .sketch
        .clone();
    for point in &mut outside.points {
        point.position[0] += 100.;
    }
    assert!(
        session
            .execute(Command::UpdateSketch {
                id: mask_id,
                sketch: outside
            })
            .is_err()
    );
    assert_eq!(session.revision(), revision);
    assert!(
        (painted_area(session.document(), body, 2)? - (100. - 9. * std::f64::consts::PI)).abs()
            < 0.15
    );
    let appearance = session
        .document()
        .body_appearance(body)
        .ok_or("Pintura ausente")?;
    let mut invalid = appearance.clone();
    invalid.materials = std::sync::Arc::new(vec![255; invalid.materials.len()]);
    let mut candidate = session.document().clone();
    assert!(candidate.set_surface_appearance(invalid).is_err());
    let scene = session.document().scene()?.ok_or("Cena ausente")?;
    assert!(scene.colors.contains(&RgbColor::new([220, 50, 50])));
    assert!(scene.colors.contains(&RgbColor::new([50, 50, 220])));
    if let Some(directory) = std::env::var_os("NATOR_PAINT_CAPTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory)?;
        nator_project_io::write_project(&directory.join("paint.3dnator"), session.document())?;
    }
    let cloned = session.document().clone();
    let mut cloned = cloned;
    cloned.clone_part(part, "Cópia".into())?;
    cloned.validate()?;
    assert_eq!(cloned.appearances().len(), 2);
    session.execute(Command::ClearSurfacePaint { body, face })?;
    assert!(session.document().body_appearance(body).is_none());
    Ok(())
}

#[test]
fn curved_surface_paint_is_partitioned_without_modifying_the_solid() -> Result<()> {
    let solid = nator_kernel::Solid::cylinder([0.; 3], 5., 10.)?;
    let side = (0..solid.face_count()?)
        .find(|i| {
            solid
                .face_measure(*i)
                .is_ok_and(|m| matches!(m.kind, nator_kernel::FaceKind::Cylinder { .. }))
        })
        .ok_or("Face cilíndrica ausente")?;
    let mask = nator_kernel::Solid::cuboid([0., -10., -1.], [10., 20., 12.])?;
    let volume = solid.volume()?;
    let (mesh, materials) = solid.surface_appearance(&[(side, Some(&mask))])?;
    let area = mesh
        .triangles()
        .iter()
        .zip(materials)
        .filter(|(_, m)| *m == 1)
        .map(|(t, _)| {
            let [a, b, c] = t.map(|i| mesh.vertices()[i as usize]);
            assert!((a[0] + b[0] + c[0]) / 3. >= -1e-6);
            nator_geometry::norm(nator_geometry::cross(
                nator_geometry::sub(b, a),
                nator_geometry::sub(c, a),
            )) / 2.
        })
        .sum::<f64>();
    assert!((area - 50. * std::f64::consts::PI).abs() < 1.);
    assert_eq!(solid.volume()?, volume);
    Ok(())
}
