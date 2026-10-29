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
            boundaries: Vec::new(),
            face: face.clone(),
            mask: None,
            color: RgbColor::new([50, 50, 220]),
        },
    })?;
    assert!((painted_area(session.document(), body, 1)? - 400.).abs() < 1e-6);
    session.execute(Command::PaintSurface {
        body,
        paint: SurfacePaint {
            boundaries: Vec::new(),
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

fn solid_fixture(
    duplicate_names: bool,
) -> Result<(Session, nator_document::PartId, nator_document::BodyId)> {
    let mut solid = nator_kernel::Solid::cuboid([0.; 3], [20., 20., 5.])?;
    let (mesh, faces) = solid.tessellate_with_faces(0.02, 0.2)?;
    let mut doc = Document::new("Balde de tinta")?;
    let (part, body) = doc.add_mesh(
        None,
        "Placa".into(),
        std::sync::Arc::new(nator_document::MeshAsset::with_faces(mesh, faces)?),
    )?;
    doc.body_mut(body)?.brep = Some(std::sync::Arc::new(solid.to_brep()?));
    doc.body_mut(body)?.face_names = std::sync::Arc::new(if duplicate_names {
        vec!["shared:face".into(); solid.face_count()?]
    } else {
        nator_kernel::indexed_face_names(&body_owner(body), solid.face_count()?)
    });
    Ok((Session::new(doc)?, part, body))
}
fn paint_pick(
    s: &Session,
    point: [f64; 3],
    whole: bool,
) -> Result<nator_commands::painting::Picked> {
    let pick = s
        .document()
        .pick(nator_geometry::query::Ray {
            origin: point,
            direction: [0., 0., -1.],
        })?
        .ok_or("Superfície ausente")?;
    Ok(nator_commands::painting::pick(s.document(), &pick, whole)?)
}
#[test]
fn automatic_cells_paint_inside_outside_and_holes_without_ambiguous_names() -> Result<()> {
    use nator_commands::painting::Target;
    let (mut s, part, body) = solid_fixture(true)?;
    let mut sketch = Sketch::new(
        "Delimitação",
        Plane {
            origin: [0., 0., 5.],
            ..Plane::xy()
        },
    );
    sketch.add_rectangle([2., 2.], [10., 10.], false)?;
    sketch.add_circle([6., 6.], 2., false)?;
    s.execute(Command::AddSketch {
        destination: Some(part),
        sketch,
    })?;
    let ring = paint_pick(&s, [3., 3., 50.], false)?.target;
    assert!(
        matches!(&ring,Target::Face{face:FaceRef::Index{..},boundaries,..} if boundaries.len()==1 && boundaries[0].inside)
    );
    s.execute(Command::PaintSurfaces {
        targets: vec![ring],
        color: RgbColor::new([0, 0, 255]),
    })?;
    let outside = paint_pick(&s, [15., 15., 50.], false)?.target;
    assert!(matches!(&outside,Target::Face{boundaries,..} if !boundaries[0].inside));
    s.execute(Command::PaintSurfaces {
        targets: vec![outside],
        color: RgbColor::new([255, 0, 0]),
    })?;
    let center = paint_pick(&s, [6., 6., 50.], false)?.target;
    s.execute(Command::PaintSurfaces {
        targets: vec![center],
        color: RgbColor::new([0, 255, 0]),
    })?;
    assert!((painted_area(s.document(), body, 1)? - (64. - 4. * std::f64::consts::PI)).abs() < 0.1);
    assert!((painted_area(s.document(), body, 2)? - 336.).abs() < 1e-6);
    assert!((painted_area(s.document(), body, 3)? - 4. * std::f64::consts::PI).abs() < 0.1);
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("cells.3dnator");
    nator_project_io::write_project(&path, s.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    assert_eq!(
        reopened
            .body_appearance(body)
            .ok_or("Pintura ausente")?
            .layers,
        s.document()
            .body_appearance(body)
            .ok_or("Pintura ausente")?
            .layers
    );
    let mut doc = reopened;
    doc.clone_part(part, "Cópia".into())?;
    doc.validate()?;
    let whole = paint_pick(&s, [6., 6., 50.], true)?.target;
    s.execute(Command::PaintSurfaces {
        targets: vec![whole],
        color: RgbColor::new([255, 180, 0]),
    })?;
    assert!(
        s.document().body_appearance(body).is_some_and(
            |a| a.layers.is_empty() && a.base_color == Some(RgbColor::new([255, 180, 0]))
        )
    );
    assert!(
        s.document()
            .scene()?
            .ok_or("Cena ausente")?
            .colors
            .iter()
            .all(|c| c.channels() == [255, 180, 0])
    );
    assert!(s.undo());
    assert_eq!(
        s.document()
            .body_appearance(body)
            .ok_or("Pintura ausente")?
            .layers
            .len(),
        3
    );
    Ok(())
}
#[test]
fn body_and_localized_colors_survive_extrusion_and_occurrence_colors() -> Result<()> {
    use nator_commands::painting::Target;
    let (mut s, part, base) = solid_fixture(false)?;
    let face = paint_pick(&s, [3., 3., 50.], false)?.target;
    s.execute(Command::PaintSurfaces {
        targets: vec![face],
        color: RgbColor::new([10, 40, 220]),
    })?;
    let mut profile = Sketch::new(
        "Saliência",
        Plane {
            origin: [0., 0., 5.],
            ..Plane::xy()
        },
    );
    profile.add_rectangle([2., 2.], [8., 8.], false)?;
    let Effect::SketchCreated { sketch, .. } = s.execute(Command::AddSketch {
        destination: Some(part),
        sketch: profile,
    })?
    else {
        return Err("Perfil ausente".into());
    };
    let operation = Operation::Extrude {
        sketch,
        region: RegionRef::Index(0),
        from: "0 mm".into(),
        to: "5 mm".into(),
        combine: Some(nator_document::features::Combine {
            target: base,
            mode: nator_document::features::BooleanMode::Join,
        }),
        until: None,
    };
    let Effect::Imported { body: raised, .. } = s.execute(Command::AddFeature {
        part,
        name: "Saliência".into(),
        operation,
    })?
    else {
        return Err("Saliência ausente".into());
    };
    let colors = s.document().scene()?.ok_or("Cena ausente")?.colors;
    assert!(colors.contains(&RgbColor::new([10, 40, 220])));
    assert!(
        s.document()
            .body_appearance(raised)
            .is_some_and(|a| !a.layers.is_empty())
    );
    s.execute(Command::PaintSurfaces {
        targets: vec![Target::Body(raised)],
        color: RgbColor::new([220, 40, 10]),
    })?;
    let mut doc = s.document().clone();
    let instance = doc.instances()[0].id;
    doc.instance_mut(instance)?.color = Some(RgbColor::new([0, 255, 0]));
    let mut s = Session::new(doc)?;
    let mut profile = Sketch::new(
        "Segunda saliência",
        Plane {
            origin: [0., 0., 10.],
            ..Plane::xy()
        },
    );
    profile.add_rectangle([3., 3.], [7., 7.], false)?;
    let Effect::SketchCreated { sketch, .. } = s.execute(Command::AddSketch {
        destination: Some(part),
        sketch: profile,
    })?
    else {
        return Err("Perfil ausente".into());
    };
    let Effect::Imported { body: result, .. } = s.execute(Command::AddSmartExtrusion {
        part,
        name: "Alongar corpo pintado".into(),
        preferred: Some(raised),
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
        return Err("Resultado ausente".into());
    };
    assert_eq!(
        s.document().body(result).ok_or("Resultado ausente")?.color,
        Some(RgbColor::new([220, 40, 10]))
    );
    assert!(
        s.document()
            .scene()?
            .ok_or("Cena ausente")?
            .colors
            .iter()
            .all(|c| c.channels() == [220, 40, 10])
    );
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("colored-extrusion.3dnator");
    nator_project_io::write_project(&path, s.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    assert!(
        reopened
            .scene()?
            .ok_or("Cena ausente")?
            .colors
            .iter()
            .all(|c| c.channels() == [220, 40, 10])
    );
    assert!(s.undo());
    assert!(s.redo());
    Ok(())
}

#[test]
fn automatic_intersections_and_complements_form_cells_and_batches_fail_atomically() -> Result<()> {
    use nator_commands::painting::Target;
    let (mut s, part, body) = solid_fixture(false)?;
    for x in [2., 7.] {
        let mut sketch = Sketch::new(
            "Divisão",
            Plane {
                origin: [0., 0., 5.],
                ..Plane::xy()
            },
        );
        sketch.add_rectangle([x, 2.], [x + 10., 12.], false)?;
        s.execute(Command::AddSketch {
            destination: Some(part),
            sketch,
        })?;
    }
    for (point, color) in [
        ([8., 5., 50.], [0, 0, 255]),
        ([3., 5., 50.], [255, 0, 0]),
        ([1., 1., 50.], [0, 255, 0]),
    ] {
        let target = paint_pick(&s, point, false)?.target;
        assert!(matches!(&target,Target::Face{boundaries,..} if boundaries.len()==2));
        s.execute(Command::PaintSurfaces {
            targets: vec![target],
            color: RgbColor::new(color),
        })?;
    }
    for (material, area) in [(1, 50.), (2, 50.), (3, 250.)] {
        assert!((painted_area(s.document(), body, material)? - area).abs() < 1e-5);
    }
    let revision = s.revision();
    let colors = s.document().scene()?.ok_or("Cena ausente")?.colors;
    assert!(
        s.execute(Command::PaintSurfaces {
            targets: vec![
                Target::Body(body),
                Target::Body(nator_document::BodyId(u64::MAX))
            ],
            color: RgbColor::new([255, 255, 0])
        })
        .is_err()
    );
    assert_eq!(s.revision(), revision);
    assert_eq!(s.document().scene()?.ok_or("Cena ausente")?.colors, colors);
    Ok(())
}

#[test]
fn cell_highlight_matches_paint_regions_holes_complements_and_world_placement() -> Result<()> {
    let (mut s, part, body) = solid_fixture(true)?;
    let mut sketch = Sketch::new(
        "Áreas visíveis",
        Plane {
            origin: [0., 0., 5.],
            ..Plane::xy()
        },
    );
    sketch.add_rectangle([2., 2.], [10., 10.], false)?;
    sketch.add_circle([6., 6.], 2., false)?;
    s.execute(Command::AddSketch {
        destination: Some(part),
        sketch,
    })?;
    let before = s.revision();
    let area = |mesh: &nator_geometry::Mesh| {
        mesh.triangles()
            .iter()
            .map(|t| {
                let [a, b, c] = mesh.triangle_points(*t);
                nator_geometry::norm(nator_geometry::cross(
                    nator_geometry::sub(b, a),
                    nator_geometry::sub(c, a),
                )) / 2.
            })
            .sum::<f64>()
    };
    for (point, expected) in [
        ([3., 3., 50.], 64. - 4. * std::f64::consts::PI),
        ([6., 6., 50.], 4. * std::f64::consts::PI),
        ([1., 1., 50.], 336.),
    ] {
        let pick = s
            .document()
            .pick(nator_geometry::query::Ray {
                origin: point,
                direction: [0., 0., -1.],
            })?
            .ok_or("Face ausente")?;
        let mesh =
            nator_commands::painting::highlight(s.document(), &pick)?.ok_or("Realce ausente")?;
        assert!((area(&mesh) - expected).abs() < 0.1);
        assert!(
            mesh.triangles()
                .iter()
                .flat_map(|t| mesh.triangle_points(*t))
                .all(|p| (p[2] - 5.).abs() < 1e-7)
        );
    }
    assert_eq!(s.revision(), before);
    assert!(s.document().body_appearance(body).is_none());
    let mut doc = s.document().clone();
    let instance = doc.instances()[0].id;
    let placement = nator_geometry::Placement::new([30., -12., 7.], [0.1, 0.2, 0.3])?;
    doc.instance_mut(instance)?.placement = placement;
    let ray = nator_geometry::query::Ray {
        origin: placement.apply([3., 3., 50.]),
        direction: placement.apply_vector([0., 0., -1.]),
    };
    let pick = doc.pick(ray)?.ok_or("Face posicionada ausente")?;
    let mesh =
        nator_commands::painting::highlight(&doc, &pick)?.ok_or("Realce posicionado ausente")?;
    assert!((area(&mesh) - (64. - 4. * std::f64::consts::PI)).abs() < 0.1);
    assert!(
        mesh.triangles()
            .iter()
            .flat_map(|t| mesh.triangle_points(*t))
            .all(|p| (placement.inverse_apply(p)[2] - 5.).abs() < 1e-6)
    );
    Ok(())
}
