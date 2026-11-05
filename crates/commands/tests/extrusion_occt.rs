//! Extrusão com o núcleo real: modos automáticos, faces, áreas e volume da prévia.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session, extrusion};
use nator_document::{
    BodyId, Document, PartId, SketchId,
    features::{BooleanMode, Combine, FaceRef, Operation, RegionRef, UntilFace, body_owner},
    parameters::Dimension,
};
use nator_kernel::Solid;
use nator_sketch::{Curve, Plane, Sketch, areas};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn created(effect: Effect) -> Result<BodyId> {
    if let Effect::Imported { body, .. } = effect {
        Ok(body)
    } else {
        Err("Corpo ausente".into())
    }
}
fn add_sketch(s: &mut Session, part: Option<PartId>, sketch: Sketch) -> Result<(PartId, SketchId)> {
    if let Effect::SketchCreated { part, sketch } = s.execute(Command::AddSketch {
        destination: part,
        sketch,
    })? {
        Ok((part, sketch))
    } else {
        Err("Rascunho ausente".into())
    }
}
fn volume(s: &Session, id: BodyId) -> Result<f64> {
    let b = s.document().body(id).ok_or("Corpo ausente")?;
    Ok(Solid::from_brep(b.brep.as_ref().ok_or("B-rep ausente")?, Some(&b.face_names))?.volume()?)
}
fn fixture(hole: bool) -> Result<(Session, PartId, BodyId)> {
    let mut s = Session::new(Document::new("Extrusão")?)?;
    let mut profile = Sketch::new("Placa", Plane::xy());
    profile.add_rectangle([0., 0.], [20., 20.], false)?;
    if hole {
        profile.add_circle([10., 10.], 3., false)?;
    }
    let (part, sketch) = add_sketch(&mut s, None, profile)?;
    s.execute(Command::SetParameter {
        part,
        name: "altura".into(),
        expression: Some("10 mm".into()),
        dimension: Dimension::Length,
    })?;
    let body = created(s.execute(Command::AddFeature {
        part,
        name: "Placa".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "altura".into(),
            combine: None,
            until: None,
        },
    })?)?;
    Ok((s, part, body))
}
fn face(base: BodyId, to: &str) -> Operation {
    Operation::ExtrudeFace {
        base,
        face: FaceRef::Named(format!("{}:end", body_owner(base))),
        from: "0 mm".into(),
        to: to.into(),
        combine: None,
        until: None,
        boundaries: Vec::new(),
    }
}

#[test]
fn face_extrusions_preserve_holes_choose_join_cut_and_recalculate() -> Result<()> {
    for hole in [false, true] {
        let (mut s, part, base) = fixture(hole)?;
        let area = 400. - if hole { std::f64::consts::PI * 9. } else { 0. };
        let outward = extrusion::automatic(s.document(), part, face(base, "5 mm"), None)?;
        assert_eq!(
            extrusion::combination(&outward),
            Some(Combine {
                target: base,
                mode: BooleanMode::Join,
                merge_coplanar: true,
            })
        );
        let joined = created(s.execute(Command::AddSmartExtrusion {
            part,
            name: "Alongar face".into(),
            operation: face(base, "5 mm"),
            preferred: None,
        })?)?;
        assert!((volume(&s, joined)? - area * 15.).abs() < 1e-4);
        assert!(!s.document().body(base).ok_or("Base ausente")?.visible);
        s.execute(Command::SetParameter {
            part,
            name: "altura".into(),
            expression: Some("12 mm".into()),
            dimension: Dimension::Length,
        })?;
        assert!((volume(&s, joined)? - area * 17.).abs() < 1e-4);
        assert!(s.undo());
        assert!(s.undo());
        let inward = extrusion::automatic(s.document(), part, face(base, "-3 mm"), None)?;
        assert_eq!(
            extrusion::combination(&inward),
            Some(Combine {
                target: base,
                mode: BooleanMode::Cut,
                merge_coplanar: false,
            })
        );
        let removed =
            extrusion::removed(s.document(), part, &inward)?.ok_or("Sem volume de remoção")?;
        assert!((removed.signed_volume().abs() - area * 3.).abs() < 1.0);
        let cut = created(s.execute(Command::AddFeature {
            part,
            name: "Rebaixar face".into(),
            operation: inward,
        })?)?;
        assert!((volume(&s, cut)? - area * 7.).abs() < 1e-4);
        assert!(s.undo());
        assert!((volume(&s, base)? - area * 10.).abs() < 1e-4);
        assert!(s.redo());
    }
    Ok(())
}

#[test]
fn free_and_face_supported_sketches_have_distinct_automatic_defaults() -> Result<()> {
    let (mut s, part, base) = fixture(false)?;
    for (origin, to, expected, expected_removed) in [
        ([40., 0., 0.], "5 mm", None, 0.),
        ([0., 0., 10.], "5 mm", Some(BooleanMode::Join), 0.),
        ([0., 0., 10.], "-15 mm", Some(BooleanMode::Cut), 250.),
        ([0., 0., 20.], "-15 mm", Some(BooleanMode::Cut), 125.),
    ] {
        let mut profile = Sketch::new(
            "Ressalto",
            Plane {
                origin,
                ..Plane::xy()
            },
        );
        profile.add_rectangle([5., 5.], [10., 10.], false)?;
        let (_, id) = add_sketch(&mut s, Some(part), profile)?;
        let operation = Operation::Extrude {
            sketch: id,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: to.into(),
            combine: None,
            until: None,
        };
        let resolved = extrusion::automatic(s.document(), part, operation, None)?;
        assert_eq!(extrusion::combination(&resolved).map(|c| c.mode), expected);
        if expected == Some(BooleanMode::Cut) {
            let removed =
                extrusion::removed(s.document(), part, &resolved)?.ok_or("Sem remoção")?;
            // O prisma mede 375 mm³, mas só a parcela dentro da placa fica vermelha.
            assert!((removed.signed_volume().abs() - expected_removed).abs() < 1e-6);
        }
        assert!(s.document().body(base).ok_or("Base ausente")?.visible);
    }
    Ok(())
}

#[test]
fn every_cell_of_intersecting_circles_is_an_exact_extrusion_profile() -> Result<()> {
    let mut profile = Sketch::new("Círculos", Plane::xy());
    profile.add_circle([0., 0.], 10., false)?;
    profile.add_circle([10., 0.], 10., false)?;
    let areas = nator_sketch::areas::regions(&profile)?;
    assert_eq!(areas.len(), 3);
    let mut s = Session::new(Document::new("Áreas")?)?;
    let (part, id) = add_sketch(&mut s, None, profile.clone())?;
    let mut sum = 0.;
    for region in areas {
        let operation = Operation::Extrude {
            sketch: id,
            region: RegionRef::Area(nator_sketch::areas::signature(&profile, &region)?),
            from: "0 mm".into(),
            to: "3 mm".into(),
            combine: None,
            until: None,
        };
        let body = created(s.execute(Command::AddFeature {
            part,
            name: "Célula".into(),
            operation,
        })?)?;
        assert!((volume(&s, body)? - region.area() * 3.).abs() < 1e-4);
        sum += volume(&s, body)?;
    }
    let lens = 200. * std::f64::consts::PI / 3. - 50. * 3_f64.sqrt();
    assert!((sum - (200. * std::f64::consts::PI - lens) * 3.).abs() < 1e-4);
    Ok(())
}

#[test]
fn until_slanted_face_trims_the_whole_prism_instead_of_using_one_distance() -> Result<()> {
    let (mut s, part, base) = fixture(false)?;
    let body = s.document().body(base).ok_or("Base ausente")?;
    let mut target = Solid::from_brep(
        body.brep.as_ref().ok_or("B-rep ausente")?,
        Some(&body.face_names),
    )?
    .transformed(nator_geometry::Placement::new(
        [50., 0., 30.],
        [0., 0.3, 0.],
    )?)?;
    let index=(0..target.face_count()?).find(|i|matches!(target.face_measure(*i).map(|f|f.kind),Ok(nator_kernel::FaceKind::Plane {normal,..}) if normal[2]>0.9)).ok_or("Plano superior ausente")?;
    let nator_kernel::FaceKind::Plane { normal, offset } = target.face_measure(index)?.kind else {
        return Err("Plano ausente".into());
    };
    let bytes = target.to_brep()?;
    let mesh = std::sync::Arc::new(nator_document::MeshAsset::new(
        target.tessellate(0.05, 0.25)?,
    ));
    let imported = created(s.execute(Command::ImportSolid {
        destination: Some(part),
        name: "Limite inclinado".into(),
        geometry: mesh,
        brep: std::sync::Arc::new(bytes),
        faces: target.face_count()?,
    })?)?;
    let mut sketch = Sketch::new("Perfil", Plane::xy());
    sketch.add_rectangle([0., 0.], [20., 20.], false)?;
    let (_, id) = add_sketch(&mut s, Some(part), sketch)?;
    let result = created(
        s.execute(Command::AddFeature {
            part,
            name: "Até face".into(),
            operation: Operation::Extrude {
                sketch: id,
                region: RegionRef::Index(0),
                from: "0 mm".into(),
                to: "1 mm".into(),
                combine: None,
                until: Some(
                    UntilFace {
                        body: imported,
                        face: FaceRef::Named(format!("{}:f{index}", body_owner(imported))),
                    }
                    .into(),
                ),
            },
        })?,
    )?;
    let height = (offset - normal[0] * 10. - normal[1] * 10.) / normal[2];
    assert!((volume(&s, result)? - 400. * height).abs() < 1e-3);
    Ok(())
}

#[test]
fn face_and_area_references_round_trip_clone_and_allow_manual_new_bodies() -> Result<()> {
    let (mut s, part, base) = fixture(false)?;
    let joined = created(s.execute(Command::AddSmartExtrusion {
        part,
        name: "Alongar face".into(),
        operation: face(base, "5 mm"),
        preferred: None,
    })?)?;
    let new = created(s.execute(Command::AddFeature {
        part,
        name: "Corpo independente".into(),
        operation: face(joined, "2 mm"),
    })?)?;
    assert!(s.document().body(joined).ok_or("Corpo ausente")?.visible);
    assert!((volume(&s, new)? - 800.).abs() < 1e-5);
    // Uma lateral gerada por extrusão de face também precisa sobreviver a recálculo
    // e clonagem: sua identidade não pode ser a posição de uma aresta na tesselação.
    let side = s
        .document()
        .body(new)
        .ok_or("Corpo ausente")?
        .face_names
        .iter()
        .find(|name| name.starts_with(&format!("{}:face:", body_owner(new))))
        .ok_or("Lateral ausente")?
        .clone();
    let lateral = created(s.execute(Command::AddFeature {
        part,
        name: "Alongar lateral".into(),
        operation: Operation::ExtrudeFace {
            base: new,
            face: FaceRef::Named(side),
            from: "0 mm".into(),
            to: "1 mm".into(),
            combine: Some(Combine {
                target: new,
                mode: BooleanMode::Join,
                merge_coplanar: true,
            }),
            until: None,
            boundaries: Vec::new(),
        },
    })?)?;
    assert!((volume(&s, lateral)? - 840.).abs() < 1e-5);
    let mut profile = Sketch::new(
        "Disco interno",
        Plane {
            origin: [40., 0., 0.],
            ..Plane::xy()
        },
    );
    profile.add_rectangle([0., 0.], [20., 20.], false)?;
    let circle = profile.add_circle([10., 10.], 2., false)?;
    let regions = nator_sketch::areas::regions(&profile)?;
    let disk = regions
        .iter()
        .find(|r| r.outer.entities == vec![circle])
        .ok_or("Disco ausente")?;
    let reference = RegionRef::Area(nator_sketch::areas::signature(&profile, disk)?);
    let (_, id) = add_sketch(&mut s, Some(part), profile)?;
    let disk = created(s.execute(Command::AddFeature {
        part,
        name: "Disco".into(),
        operation: Operation::Extrude {
            sketch: id,
            region: reference,
            from: "0 mm".into(),
            to: "3 mm".into(),
            combine: None,
            until: None,
        },
    })?)?;
    assert!((volume(&s, disk)? - std::f64::consts::PI * 12.).abs() < 1e-5);
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("extrusao.3dnator");
    nator_project_io::write_project(&path, s.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    let mut s = Session::new(reopened)?;
    let instance = s
        .document()
        .instances()
        .iter()
        .find(|i| i.part == part)
        .ok_or("Instância ausente")?
        .id;
    s.execute(Command::CloneInstance {
        id: instance,
        name: "Cópia paramétrica".into(),
    })?;
    let copy = s
        .document()
        .parts()
        .iter()
        .find(|p| p.id != part)
        .ok_or("Peça copiada ausente")?
        .id;
    s.execute(Command::SetParameter {
        part: copy,
        name: "altura".into(),
        expression: Some("20 mm".into()),
        dimension: Dimension::Length,
    })?;
    let piece = s.document().part(copy).ok_or("Peça ausente")?;
    let output = piece
        .features
        .iter()
        .find(|f| f.name == "Alongar face")
        .ok_or("Operação copiada ausente")?
        .output;
    assert!((volume(&s, output)? - 10000.).abs() < 1e-5);
    assert!((volume(&s, joined)? - 6000.).abs() < 1e-5);
    s.document().validate()?;
    Ok(())
}

#[test]
fn sketch_boss_preserves_existing_support_holes_while_extending_the_outer_border() -> Result<()> {
    let (mut s, part, base) = fixture(true)?;
    let mut sketch = Sketch::new(
        "Saliência",
        Plane {
            origin: [0., 0., 10.],
            ..Plane::xy()
        },
    );
    sketch.add_rectangle([-2., -2.], [22., 22.], false)?;
    let (_, id) = add_sketch(&mut s, Some(part), sketch)?;
    let boss = created(s.execute(Command::AddSmartExtrusion {
        part,
        name: "Saliência sobre furo".into(),
        preferred: Some(base),
        operation: Operation::Extrude {
            sketch: id,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "5 mm".into(),
            combine: None,
            until: None,
        },
    })?)?;
    let expected =
        (400. - 9. * std::f64::consts::PI) * 10. + (576. - 9. * std::f64::consts::PI) * 5.;
    assert!(
        (volume(&s, boss)? - expected).abs() < 1e-4,
        "{}",
        volume(&s, boss)?
    );
    let body = s.document().body(boss).ok_or("Saliência ausente")?;
    assert!(
        body.geometry
            .mesh()
            .ray_hit(nator_geometry::query::Ray {
                origin: [10., 10., 100.],
                direction: [0., 0., -1.]
            })
            .is_none()
    );
    assert!(s.undo());
    assert!(s.redo());
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("saliência-furada.3dnator");
    nator_project_io::write_project(&path, s.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    assert!((volume(&Session::new(reopened)?, boss)? - expected).abs() < 1e-4);
    let profile = s.document().part(part).ok_or("Peça ausente")?.sketches[0].id;
    let mut updated = s
        .document()
        .sketch(profile)
        .ok_or("Perfil ausente")?
        .sketch
        .clone();
    for entity in &mut updated.entities {
        if let nator_sketch::Curve::Circle { radius, .. } = &mut entity.curve {
            *radius = 4.;
        }
    }
    s.execute(Command::UpdateSketch {
        id: profile,
        sketch: updated,
    })?;
    let expected =
        (400. - 16. * std::f64::consts::PI) * 10. + (576. - 16. * std::f64::consts::PI) * 5.;
    assert!((volume(&s, boss)? - expected).abs() < 1e-4);
    if let Some(directory) = std::env::var_os("NATOR_EXTRUSION_CAPTURE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory)?;
        nator_project_io::write_project(&directory.join("hole-preserved.3dnator"), s.document())?;
    }
    Ok(())
}

#[test]
fn zero_is_a_guide_only_and_sketch_and_work_planes_are_associative_limits() -> Result<()> {
    use nator_document::features::ExtrusionLimit;
    let mut s = Session::new(Document::new("Planos limite")?)?;
    let mut sketch = Sketch::new("Perfil", Plane::xy());
    sketch.add_rectangle([0., 0.], [4., 4.], false)?;
    let (part, id) = add_sketch(&mut s, None, sketch)?;
    let (_, limit) = add_sketch(
        &mut s,
        Some(part),
        Sketch::new(
            "Limite",
            Plane {
                origin: [0., 0., 10.],
                ..Plane::xy()
            },
        ),
    )?;
    let operation = Operation::Extrude {
        sketch: id,
        region: RegionRef::Index(0),
        from: "0 mm".into(),
        to: "0 mm".into(),
        combine: None,
        until: None,
    };
    let guide = extrusion::guide(s.document(), part, &operation)?;
    assert_eq!((guide.from, guide.to), (0., 0.));
    let revision = s.revision();
    assert!(
        s.execute(Command::AddSmartExtrusion {
            part,
            name: "Zero".into(),
            operation: operation.clone(),
            preferred: None
        })
        .is_err()
    );
    assert_eq!(s.revision(), revision);
    let mut bounded = operation.clone();
    if let Operation::Extrude { until, .. } = &mut bounded {
        *until = Some(ExtrusionLimit::Sketch { sketch: limit });
    }
    let body = created(s.execute(Command::AddFeature {
        part,
        name: "Até rascunho".into(),
        operation: bounded,
    })?)?;
    assert!((volume(&s, body)? - 160.).abs() < 1e-5);
    let mut moved = s
        .document()
        .sketch(limit)
        .ok_or("Limite ausente")?
        .sketch
        .clone();
    moved.plane.origin[2] = 20.;
    s.execute(Command::UpdateSketch {
        id: limit,
        sketch: moved,
    })?;
    assert!((volume(&s, body)? - 320.).abs() < 1e-5);
    assert!(s.execute(Command::RemoveSketch { id: limit }).is_err());
    let plane = s.document().clone();
    let mut plane = plane;
    let work = plane.add_work_plane(
        part,
        "Plano limite".into(),
        nator_document::WorkPlane {
            base: Plane::xy(),
            offset: 15.,
        },
    )?;
    let mut s = Session::new(plane)?;
    let mut operation = operation;
    if let Operation::Extrude { until, .. } = &mut operation {
        *until = Some(ExtrusionLimit::WorkPlane { work_plane: work });
    }
    let body = created(s.execute(Command::AddFeature {
        part,
        name: "Até plano".into(),
        operation,
    })?)?;
    assert!((volume(&s, body)? - 240.).abs() < 1e-5);
    s.execute(Command::UpdateWorkPlane {
        id: work,
        plane: nator_document::WorkPlane {
            base: Plane::xy(),
            offset: 25.,
        },
    })?;
    assert!((volume(&s, body)? - 400.).abs() < 1e-5);
    let mut doc = s.document().clone();
    doc.clone_part(part, "Cópia".into())?;
    doc.validate()?;
    Ok(())
}

#[test]
fn zero_extrusion_partitions_faces_preserves_holes_colors_and_rebuilds() -> Result<()> {
    use nator_commands::painting::Target;
    use nator_geometry::RgbColor;
    for hole in [false, true] {
        let (mut session, part, base) = fixture(hole)?;
        let red = RgbColor::new([180, 35, 60]);
        let blue = RgbColor::new([25, 80, 220]);
        session.execute(Command::SetBodyColor {
            id: base,
            color: Some(red),
        })?;
        session.execute(Command::PaintSurfaces {
            targets: vec![Target::Face {
                body: base,
                face: FaceRef::Named(format!("{}:end", body_owner(base))),
                boundaries: vec![],
            }],
            color: blue,
        })?;
        let mut profile = Sketch::new(
            "Divisão",
            Plane {
                origin: [0., 0., 10.],
                ..Plane::xy()
            },
        );
        profile.add_rectangle([2., 2.], [18., 18.], false)?;
        let (_, sketch) = add_sketch(&mut session, Some(part), profile.clone())?;
        let old_volume = volume(&session, base)?;
        let old_faces = session
            .document()
            .body(base)
            .ok_or("Base")?
            .face_names
            .len();
        let output = created(session.execute(Command::AddSmartExtrusion {
            part,
            name: "Dividir face".into(),
            preferred: None,
            operation: Operation::Extrude {
                sketch,
                region: RegionRef::Index(0),
                from: "0 mm".into(),
                to: "0 mm".into(),
                combine: None,
                until: None,
            },
        })?)?;
        assert!((volume(&session, output)? - old_volume).abs() < 1e-6);
        let body = session.document().body(output).ok_or("Divisão")?;
        assert!(body.face_names.len() > old_faces);
        assert_eq!(body.color, Some(red));
        let appearance = session
            .document()
            .body_appearance(output)
            .ok_or("Pintura herdada")?;
        assert!(appearance.layers.len() >= 2);
        assert!(appearance.layers.iter().all(|layer| layer.color == blue));
        let op = &session
            .document()
            .part(part)
            .ok_or("Peça")?
            .features
            .last()
            .ok_or("Etapa")?
            .operation;
        assert!(
            matches!(op, Operation::Extrude { combine: Some(Combine { target, .. }), region: RegionRef::Boundary(_), .. } if *target == base)
        );
        if hole {
            assert!(
                session
                    .document()
                    .pick(nator_geometry::query::Ray {
                        origin: [10., 10., 50.],
                        direction: [0., 0., -1.]
                    })?
                    .is_none()
            );
        }
        assert!(session.undo());
        assert!(session.document().body(base).ok_or("Base")?.visible);
        assert!(session.document().body(output).is_none());
        assert!(session.redo());
        let revision = session.revision();
        profile.plane.origin[2] = 11.;
        assert!(
            session
                .execute(Command::UpdateSketch {
                    id: sketch,
                    sketch: profile.clone()
                })
                .is_err()
        );
        assert_eq!(session.revision(), revision);
        profile.plane.origin[2] = 10.;
        for point in &mut profile.points {
            point.position[0] += 0.25;
        }
        session.execute(Command::UpdateSketch {
            id: sketch,
            sketch: profile,
        })?;
        assert!((volume(&session, output)? - old_volume).abs() < 1e-6);
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("divisao.3dnator");
        nator_project_io::write_project(&path, session.document())?;
        let reopened = Session::new(nator_project_io::read_project(&path)?)?;
        assert!((volume(&reopened, output)? - old_volume).abs() < 1e-6);
        let body = reopened.document().body(output).ok_or("Divisão")?;
        let solid = Solid::from_brep(body.brep.as_ref().ok_or("B-rep")?, Some(&body.face_names))?;
        let step = directory.path().join("divisao.step");
        solid.write_step(&step)?;
        let restored = Solid::read_step(&step)?;
        assert!((restored.volume()? - old_volume).abs() < 1e-5);
        assert_eq!(restored.face_count()?, solid.face_count()?);
        let mut cloned = reopened.document().clone();
        cloned.clone_part(part, "Cópia da divisão".into())?;
        cloned.validate()?;
    }
    Ok(())
}

#[test]
fn zero_extrusion_requires_unique_support_and_keeps_multiple_regions() -> Result<()> {
    let (mut session, part, base) = fixture(false)?;
    let mut profile = Sketch::new(
        "Duas áreas",
        Plane {
            origin: [0., 0., 10.],
            ..Plane::xy()
        },
    );
    profile.add_rectangle([1., 1.], [6., 6.], false)?;
    profile.add_rectangle([14., 14.], [19., 19.], false)?;
    let inner = profile.add_circle([3., 3.], 1., false)?;
    let regions = nator_sketch::profiles::regions(&profile)?
        .iter()
        .filter(|r| !r.outer.entities.contains(&inner))
        .map(|r| RegionRef::Boundary(r.boundary()))
        .collect();
    let (_, sketch) = add_sketch(&mut session, Some(part), profile)?;
    let operation = Operation::Extrude {
        sketch,
        region: RegionRef::Multiple(regions),
        from: "0 mm".into(),
        to: "0 mm".into(),
        combine: None,
        until: None,
    };
    let original = session.document().body(base).ok_or("Base")?.clone();
    session.document().validate()?;
    let output = created(session.execute(Command::AddFeature {
        part,
        name: "Divisão múltipla".into(),
        operation: operation.clone(),
    })?)?;
    let body = session.document().body(output).ok_or("Divisão")?;
    assert_eq!(body.face_names.len(), original.face_names.len() + 3);
    assert!((volume(&session, output)? - 4000.).abs() < 1e-6);
    assert!(session.undo());
    // Dois corpos coincidentes não podem ser escolhidos por ordem de criação.
    session.execute(Command::CopyBody {
        id: base,
        placement: nator_geometry::Placement::default(),
    })?;
    let revision = session.revision();
    assert!(
        session
            .execute(Command::AddSmartExtrusion {
                part,
                name: "Ambígua".into(),
                operation: operation.clone(),
                preferred: None
            })
            .is_err()
    );
    assert_eq!(session.revision(), revision);
    created(session.execute(Command::AddSmartExtrusion {
        part,
        name: "Base explícita".into(),
        operation,
        preferred: Some(base),
    })?)?;
    Ok(())
}

/// Pontas sem ligação explícita guardam a folga residual do solver: maior que a tolerância
/// do OCCT (1e-7 mm), mas dentro da fusão de nós das áreas (1e-6 mm), em linhas e arcos.
#[test]
fn area_with_residual_gaps_between_curves_is_an_exact_extrusion_profile() -> Result<()> {
    const GAP: f64 = 2e-7;
    let mut s = Session::new(Document::new("Folgas")?)?;
    let mut profile = Sketch::new("Folgas", Plane::xy());
    profile.add_line([0., 0.], [20., 0.], false)?;
    profile.add_line([20. + GAP, 0.], [20., 10.], false)?;
    let center = profile.add_point([10., 10.])?;
    let start = profile.add_point([20. + GAP, 10.])?;
    let end = profile.add_point([-GAP, 10.])?;
    profile.add_curve(
        Curve::Arc {
            center,
            start,
            end,
            clockwise: false,
        },
        false,
    )?;
    profile.add_line([0., 10.], [0., 0.], false)?;
    let regions = areas::regions(&profile)?;
    assert_eq!(regions.len(), 1);
    let signature = areas::signature(&profile, &regions[0])?;
    let (part, sketch) = add_sketch(&mut s, None, profile)?;
    let body = created(s.execute(Command::AddFeature {
        part,
        name: "Folgas".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Area(signature),
            from: "0 mm".into(),
            to: "4 mm".into(),
            combine: None,
            until: None,
        },
    })?)?;
    let expected = 4. * (200. + 50. * std::f64::consts::PI);
    assert!((volume(&s, body)? - expected).abs() < 1e-3);
    Ok(())
}

/// Placa de 20 × 20 × 10 com um quadrado de 10 × 10 desenhado no centro da face superior.
fn plate_with_contour() -> Result<(Session, PartId, BodyId, SketchId)> {
    let (mut s, part, base) = fixture(false)?;
    let mut contour = Sketch::new(
        "Contorno",
        Plane {
            origin: [0., 0., 10.],
            ..Plane::xy()
        },
    );
    contour.add_rectangle([5., 5.], [15., 15.], false)?;
    let (_, sketch) = add_sketch(&mut s, Some(part), contour)?;
    Ok((s, part, base, sketch))
}
/// Operação que o clique em (x, y) produz na face superior, olhando de cima.
fn clicked_area(s: &Session, base: BodyId, x: f64, y: f64, to: &str) -> Result<Operation> {
    let pick = s
        .document()
        .pick_all_with(
            nator_geometry::query::Ray {
                origin: [x, y, 100.],
                direction: [0., 0., -1.],
            },
            |_| true,
        )?
        .into_iter()
        .next()
        .ok_or("Clique fora da peça")?;
    Ok(Operation::ExtrudeFace {
        base,
        face: s.document().face_at(&pick)?,
        from: "0 mm".into(),
        to: to.into(),
        combine: None,
        until: None,
        boundaries: nator_commands::painting::boundaries(s.document(), &pick)?,
    })
}
fn smart(s: &mut Session, part: PartId, operation: Operation) -> Result<BodyId> {
    created(s.execute(Command::AddSmartExtrusion {
        part,
        name: "Área da face".into(),
        operation,
        preferred: None,
    })?)
}

#[test]
fn clicking_outside_a_contour_extrudes_only_that_cell_of_the_face() -> Result<()> {
    let (mut s, part, base, sketch) = plate_with_contour()?;
    let operation = clicked_area(&s, base, 2., 2., "5 mm")?;
    let Operation::ExtrudeFace { boundaries, .. } = &operation else {
        return Err("Operação inesperada".into());
    };
    assert_eq!(boundaries.len(), 1);
    assert_eq!(boundaries[0].mask.sketch, sketch);
    assert!(!boundaries[0].inside);
    let outside = smart(&mut s, part, operation)?;
    assert!((volume(&s, outside)? - (4000. + 300. * 5.)).abs() < 1e-4);
    // Só o anel sobe: o centro da face continua a 10 mm.
    let mesh = s.document().body(outside).ok_or("Corpo ausente")?;
    assert!((mesh.geometry.mesh().bounds().max[2] - 15.).abs() < 1e-6);
    // Como um perfil, o contorno usado sai de cena só depois da operação concluída.
    assert!(
        !s.document()
            .sketch(sketch)
            .ok_or("Rascunho ausente")?
            .visible
    );
    assert!(s.undo());

    // O mesmo clique dentro do contorno escolhe a célula complementar.
    let inside = clicked_area(&s, base, 10., 10., "5 mm")?;
    let inside = smart(&mut s, part, inside)?;
    assert!((volume(&s, inside)? - (4000. + 100. * 5.)).abs() < 1e-4);
    Ok(())
}

#[test]
fn a_face_without_a_visible_drawing_is_still_one_whole_area() -> Result<()> {
    let (mut s, part, base, sketch) = plate_with_contour()?;
    s.execute(Command::SetSketchVisibility {
        id: sketch,
        visible: false,
    })?;
    let operation = clicked_area(&s, base, 2., 2., "5 mm")?;
    assert!(
        matches!(&operation, Operation::ExtrudeFace { boundaries, .. } if boundaries.is_empty())
    );
    let whole = smart(&mut s, part, operation)?;
    assert!((volume(&s, whole)? - (4000. + 400. * 5.)).abs() < 1e-4);
    Ok(())
}

#[test]
fn a_face_cell_follows_its_sketch_and_protects_it() -> Result<()> {
    let (mut s, part, base, sketch) = plate_with_contour()?;
    let operation = clicked_area(&s, base, 2., 2., "5 mm")?;
    let outside = smart(&mut s, part, operation)?;
    assert!(s.execute(Command::RemoveSketch { id: sketch }).is_err());
    // Mover um vértice preserva as entidades e, portanto, a referência da área.
    let mut edited = s
        .document()
        .sketch(sketch)
        .ok_or("Rascunho ausente")?
        .sketch
        .clone();
    for point in &mut edited.points {
        point.position = point.position.map(|v| if v < 10. { 3. } else { 17. });
    }
    s.execute(Command::UpdateSketch {
        id: sketch,
        sketch: edited.clone(),
    })?;
    let current = s
        .document()
        .part(part)
        .and_then(|p| p.bodies.iter().rev().find(|b| b.visible))
        .map(|b| b.id)
        .ok_or("Resultado ausente")?;
    assert_eq!(current, outside);
    assert!((volume(&s, outside)? - (4000. + (400. - 196.) * 5.)).abs() < 1e-4);

    // Sem nada de fora do contorno, a célula sumiu: falha, nunca a face inteira.
    for point in &mut edited.points {
        point.position = point.position.map(|v| if v < 10. { -5. } else { 25. });
    }
    assert!(
        s.execute(Command::UpdateSketch {
            id: sketch,
            sketch: edited,
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn zero_on_a_cell_divides_the_face_and_a_whole_face_still_refuses() -> Result<()> {
    let (mut s, part, base, _) = plate_with_contour()?;
    let faces = s
        .document()
        .body(base)
        .ok_or("Base ausente")?
        .face_names
        .len();
    let operation = clicked_area(&s, base, 2., 2., "0 mm")?;
    let divided = smart(&mut s, part, operation)?;
    assert!((volume(&s, divided)? - 4000.).abs() < 1e-4);
    assert_eq!(
        s.document()
            .body(divided)
            .ok_or("Corpo ausente")?
            .face_names
            .len(),
        faces + 1
    );
    assert!(s.undo());
    assert!(extrusion::automatic(s.document(), part, face(base, "0 mm"), None).is_err());
    Ok(())
}

#[test]
fn preview_volume_of_a_cell_union_is_only_the_new_material() -> Result<()> {
    let (s, part, base, _) = plate_with_contour()?;
    let operation = clicked_area(&s, base, 2., 2., "5 mm")?;
    let operation = extrusion::automatic(s.document(), part, operation, None)?;
    assert_eq!(
        extrusion::combination(&operation).map(|c| c.mode),
        Some(BooleanMode::Join)
    );
    let added = extrusion::added(s.document(), part, &operation)?.ok_or("Volume ausente")?;
    let bounds = added.bounds();
    assert!((bounds.min[2] - 10.).abs() < 1e-6 && (bounds.max[2] - 15.).abs() < 1e-6);
    assert!(extrusion::removed(s.document(), part, &operation)?.is_none());
    Ok(())
}

#[test]
fn a_face_cell_survives_saving_and_parametric_cloning() -> Result<()> {
    let (mut s, part, base, _) = plate_with_contour()?;
    let operation = clicked_area(&s, base, 2., 2., "5 mm")?;
    let outside = smart(&mut s, part, operation)?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("celula.3dnator");
    nator_project_io::write_project(&path, s.document())?;
    let mut s = Session::new(nator_project_io::read_project(&path)?)?;
    let instance = s
        .document()
        .instances()
        .iter()
        .find(|i| i.part == part)
        .ok_or("Instância ausente")?
        .id;
    s.execute(Command::CloneInstance {
        id: instance,
        name: "Cópia".into(),
    })?;
    let copy = s
        .document()
        .parts()
        .iter()
        .find(|p| p.id != part)
        .ok_or("Peça copiada ausente")?
        .id;
    s.execute(Command::SetParameter {
        part: copy,
        name: "altura".into(),
        expression: Some("20 mm".into()),
        dimension: Dimension::Length,
    })?;
    let output = s
        .document()
        .part(copy)
        .and_then(|p| p.features.iter().find(|f| f.name == "Área da face"))
        .ok_or("Operação copiada ausente")?
        .output;
    // O rascunho do clone continua a 10 mm: o prisma atravessa até a nova face superior.
    assert!((volume(&s, output)? - (8000. + 300. * 5.)).abs() < 1e-4);
    assert!((volume(&s, outside)? - (4000. + 300. * 5.)).abs() < 1e-4);
    s.document().validate()?;
    Ok(())
}

/// Placa de 20 × 20 × 10 com um traço aberto no rascunho sobre a face superior.
fn plate_with_line(
    hole: bool,
    a: [f64; 2],
    b: [f64; 2],
) -> Result<(Session, PartId, BodyId, SketchId)> {
    let (mut s, part, base) = fixture(hole)?;
    let mut division = Sketch::new(
        "Divisão",
        Plane {
            origin: [0., 0., 10.],
            ..Plane::xy()
        },
    );
    division.add_line(a, b, false)?;
    let (_, sketch) = add_sketch(&mut s, Some(part), division)?;
    Ok((s, part, base, sketch))
}

#[test]
fn an_open_line_divides_the_face_into_two_areas_as_if_the_edges_were_drawn() -> Result<()> {
    for (a, b) in [([8., 0.], [8., 20.]), ([8., -5.], [8., 25.])] {
        let (mut s, part, base, _) = plate_with_line(false, a, b)?;
        let left = clicked_area(&s, base, 2., 2., "5 mm")?;
        let right = clicked_area(&s, base, 15., 15., "5 mm")?;
        let Operation::ExtrudeFace { boundaries, .. } = &left else {
            return Err("Operação inesperada".into());
        };
        assert_eq!(boundaries.len(), 1);
        assert!(matches!(boundaries[0].mask.region, RegionRef::Cell { .. }));
        let body = smart(&mut s, part, left)?;
        assert!((volume(&s, body)? - (4000. + 160. * 5.)).abs() < 1e-4);
        assert!(s.undo());
        let body = smart(&mut s, part, right)?;
        assert!((volume(&s, body)? - (4000. + 240. * 5.)).abs() < 1e-4);
    }
    Ok(())
}

#[test]
fn a_divided_face_keeps_curved_holes_and_a_line_that_stops_inside_divides_nothing() -> Result<()> {
    // Traço que passa pelo furo: a borda circular entra exata no contorno da parte.
    let (mut s, part, base, _) = plate_with_line(true, [10., 0.], [10., 20.])?;
    let left = clicked_area(&s, base, 2., 2., "5 mm")?;
    let body = smart(&mut s, part, left)?;
    let half = 200. - std::f64::consts::PI * 9. / 2.;
    assert!(
        (volume(&s, body)? - ((400. - std::f64::consts::PI * 9.) * 10. + half * 5.)).abs() < 1e-3
    );

    // Um traço que para no meio não separa nada: a face continua inteira.
    let (s, _, base, _) = plate_with_line(false, [8., 0.], [8., 10.])?;
    let operation = clicked_area(&s, base, 2., 2., "5 mm")?;
    assert!(
        matches!(&operation, Operation::ExtrudeFace { boundaries, .. } if boundaries.is_empty())
    );
    Ok(())
}

#[test]
fn a_face_part_follows_its_line_and_fails_if_the_line_passes_the_chosen_point() -> Result<()> {
    let (mut s, part, base, sketch) = plate_with_line(false, [8., 0.], [8., 20.])?;
    let operation = clicked_area(&s, base, 2., 2., "5 mm")?;
    let body = smart(&mut s, part, operation)?;
    let mut near = s
        .document()
        .sketch(sketch)
        .ok_or("Rascunho")?
        .sketch
        .clone();
    for point in &mut near.points {
        point.position[0] = 12.;
    }
    s.execute(Command::UpdateSketch {
        id: sketch,
        sketch: near.clone(),
    })?;
    assert!((volume(&s, body)? - (4000. + 240. * 5.)).abs() < 1e-4);
    // O traço passou por cima do ponto escolhido: a outra parte não entra no lugar.
    for point in &mut near.points {
        point.position[0] = 1.;
    }
    assert!(
        s.execute(Command::UpdateSketch {
            id: sketch,
            sketch: near
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn the_highlight_of_a_face_part_covers_only_that_part() -> Result<()> {
    let (s, _, base, _) = plate_with_line(false, [8., 0.], [8., 20.])?;
    let Operation::ExtrudeFace {
        face, boundaries, ..
    } = clicked_area(&s, base, 2., 2., "5 mm")?
    else {
        return Err("Operação inesperada".into());
    };
    let mesh = nator_commands::painting::cell_mesh(s.document(), base, &face, &boundaries)?;
    let area = mesh
        .triangles()
        .iter()
        .map(|t| {
            let [a, b, c] = mesh.triangle_points(*t);
            nator_geometry::norm(nator_geometry::cross(
                nator_geometry::sub(b, a),
                nator_geometry::sub(c, a),
            )) / 2.
        })
        .sum::<f64>();
    assert!((area - 160.).abs() < 1e-6, "{area}");
    Ok(())
}

#[test]
fn a_body_moves_to_another_part_in_the_same_place_and_its_history_stays_behind() -> Result<()> {
    let (mut s, part, base) = fixture(false)?;
    let mut other = Sketch::new("Outra", Plane::xy());
    other.add_rectangle([100., 0.], [110., 10.], false)?;
    let (destination, _) = add_sketch(&mut s, None, other)?;
    assert_ne!(destination, part);
    let instance = s
        .document()
        .instances()
        .iter()
        .find(|i| i.part == destination)
        .ok_or("Instância ausente")?
        .id;
    let offset = nator_geometry::Placement::new([30., 40., 0.], [0.; 3])?;
    s.execute(Command::MoveInstance {
        id: instance,
        placement: offset,
    })?;
    let moved = created(s.execute(Command::TransferBody {
        id: base,
        destination,
    })?)?;
    let document = s.document();
    let body = document.body(moved).ok_or("Corpo ausente")?;
    assert!(
        document
            .part(destination)
            .is_some_and(|p| p.bodies.iter().any(|b| b.id == moved))
    );
    // A posição local compensa a da peça de destino: nada anda no mundo.
    assert_eq!(body.placement.translation_mm(), [-30., -40., 0.]);
    assert!((volume(&s, moved)? - 4000.).abs() < 1e-5);
    // A origem depende do corpo no histórico: ele fica oculto, não some.
    assert!(!document.body(base).ok_or("Base ausente")?.visible);
    document.validate()?;
    assert!(s.undo());
    assert!(s.document().body(moved).is_none());
    assert!(s.document().body(base).ok_or("Base ausente")?.visible);
    Ok(())
}

#[test]
fn two_areas_of_a_face_become_one_and_the_dividing_line_disappears() -> Result<()> {
    let (mut s, part, base, _) = plate_with_line(false, [8., 0.], [8., 20.])?;
    let faces = s.document().body(base).ok_or("Base")?.face_names.len();
    let left = clicked_area(&s, base, 2., 2., "0 mm")?;
    let divided = smart(&mut s, part, left)?;
    let split = s.document().body(divided).ok_or("Corpo")?.face_names.len();
    assert_eq!(split, faces + 1);
    let click = |s: &Session, x: f64| -> Result<FaceRef> {
        let pick = s
            .document()
            .pick_all_with(
                nator_geometry::query::Ray {
                    origin: [x, 5., 100.],
                    direction: [0., 0., -1.],
                },
                |_| true,
            )?
            .into_iter()
            .next()
            .ok_or("Clique fora da peça")?;
        Ok(s.document().face_at(&pick)?)
    };
    let merge = Operation::MergeFaces {
        base: divided,
        faces: vec![click(&s, 2.)?, click(&s, 15.)?],
    };
    let merged = created(s.execute(Command::AddFeature {
        part,
        name: "Unir áreas".into(),
        operation: merge,
    })?)?;
    let body = s.document().body(merged).ok_or("Corpo")?;
    assert_eq!(body.face_names.len(), faces);
    assert!((volume(&s, merged)? - 4000.).abs() < 1e-6);
    assert!(s.undo());
    let side = s.document().body(divided).ok_or("Corpo")?;
    let lateral = side
        .face_names
        .iter()
        .find(|n| n.contains(":e"))
        .ok_or("Lateral ausente")?
        .clone();
    assert!(
        s.execute(Command::AddFeature {
            part,
            name: "Não coplanar".into(),
            operation: Operation::MergeFaces {
                base: divided,
                faces: vec![click(&s, 2.)?, FaceRef::Named(lateral)],
            },
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn face_lines_come_exact_into_the_sketch_and_their_areas_extrude() -> Result<()> {
    let (mut s, part, base) = fixture(true)?;
    let top = Plane {
        origin: [0., 0., 10.],
        ..Plane::xy()
    };
    let (_, sketch) = add_sketch(&mut s, Some(part), Sketch::new("Na face", top))?;
    s.execute(Command::AddFaceLines { id: sketch })?;
    let record = s
        .document()
        .sketch(sketch)
        .ok_or("Rascunho")?
        .sketch
        .clone();
    let face = record
        .entities
        .iter()
        .filter(|e| e.face)
        .collect::<Vec<_>>();
    // Quatro lados e o furo, este como círculo exato.
    assert_eq!(face.len(), 5);
    assert!(face.iter().any(|e| matches!(e.curve, Curve::Circle { .. })));
    // A placa em volta do furo e o disco do próprio círculo.
    let mut regions = areas::regions(&record)?;
    assert_eq!(regions.len(), 2);
    regions.sort_by(|a, b| b.area().total_cmp(&a.area()));
    assert!((regions[0].area() - (400. - std::f64::consts::PI * 9.)).abs() < 1e-6);
    // Repetir não duplica e o rascunho sem face no plano é recusado.
    assert!(s.execute(Command::AddFaceLines { id: sketch }).is_err());
    let (_, away) = add_sketch(
        &mut s,
        Some(part),
        Sketch::new(
            "Longe",
            Plane {
                origin: [0., 0., 50.],
                ..Plane::xy()
            },
        ),
    )?;
    assert!(s.execute(Command::AddFaceLines { id: away }).is_err());
    // A área delimitada pelas linhas da face extruda como qualquer região.
    let reference = RegionRef::Area(areas::signature(&record, &regions[0])?);
    let body = created(s.execute(Command::AddSmartExtrusion {
        part,
        name: "Sobre a face".into(),
        operation: Operation::Extrude {
            sketch,
            region: reference,
            from: "0 mm".into(),
            to: "5 mm".into(),
            combine: None,
            until: None,
        },
        preferred: Some(base),
    })?)?;
    let area = 400. - std::f64::consts::PI * 9.;
    assert!((volume(&s, body)? - area * 15.).abs() < 1e-3);
    Ok(())
}

/// Faces planas do corpo no plano de normal `axis` a `offset` mm da origem.
fn planar_faces(s: &Session, id: BodyId, axis: usize, offset: f64) -> Result<usize> {
    let b = s.document().body(id).ok_or("Corpo ausente")?;
    let solid = Solid::from_brep(b.brep.as_ref().ok_or("B-rep ausente")?, Some(&b.face_names))?;
    let mut count = 0;
    for face in 0..solid.face_count()? {
        if let nator_kernel::FaceKind::Plane { normal, offset: d } = solid.face_measure(face)?.kind
            && normal[axis].abs() > 1. - 1e-9
            && (d * normal[axis] - offset).abs() < 1e-6
        {
            count += 1;
        }
    }
    Ok(count)
}

#[test]
fn joined_extrusions_merge_new_faces_with_coplanar_neighbours_only() -> Result<()> {
    let (mut s, part, base) = fixture(false)?;
    let top = Plane {
        origin: [0., 0., 10.],
        ..Plane::xy()
    };
    // A divisão feita de propósito pela extrusão zero continua: duas faces no topo.
    let mut strip = Sketch::new("Faixa", top);
    strip.add_rectangle([0., 0.], [5., 20.], false)?;
    let (_, strip) = add_sketch(&mut s, Some(part), strip)?;
    let divided = created(s.execute(Command::AddSmartExtrusion {
        part,
        name: "Dividir".into(),
        operation: Operation::Extrude {
            sketch: strip,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "0 mm".into(),
            combine: None,
            until: None,
        },
        preferred: Some(base),
    })?)?;
    assert_eq!(planar_faces(&s, divided, 2, 10.)?, 2);
    // Ressalto sobre o topo, rente à lateral x = 20: as laterais novas continuam as
    // antigas no mesmo plano e viram uma face só.
    let mut boss = Sketch::new("Ressalto", top);
    boss.add_rectangle([10., 0.], [20., 20.], false)?;
    let (_, boss) = add_sketch(&mut s, Some(part), boss)?;
    let joined = created(s.execute(Command::AddSmartExtrusion {
        part,
        name: "Ressalto".into(),
        operation: Operation::Extrude {
            sketch: boss,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "5 mm".into(),
            combine: None,
            until: None,
        },
        preferred: Some(divided),
    })?)?;
    let feature = |s: &Session| -> Result<Operation> {
        Ok(s.document()
            .part(part)
            .and_then(|p| p.features.iter().find(|f| f.output == joined))
            .ok_or("Operação ausente")?
            .operation
            .clone())
    };
    let Operation::Extrude {
        sketch,
        region,
        from,
        to,
        combine: Some(combine),
        until,
    } = feature(&s)?
    else {
        return Err("União ausente".into());
    };
    assert_eq!(combine.mode, BooleanMode::Join);
    assert!(combine.merge_coplanar);
    for (axis, offset) in [(0, 20.), (1, 0.), (1, 20.)] {
        assert_eq!(
            planar_faces(&s, joined, axis, offset)?,
            1,
            "{axis} {offset}"
        );
    }
    // As duas faces antigas do topo, ambas sem nada novo, não se juntam.
    assert_eq!(planar_faces(&s, joined, 2, 10.)?, 2);
    assert!((volume(&s, joined)? - (20. * 20. * 10. + 10. * 20. * 5.)).abs() < 1e-6);
    // Desligada, as laterais ficam separadas como antes.
    s.execute(Command::EditFeature {
        part,
        output: joined,
        operation: Operation::Extrude {
            sketch,
            region,
            from,
            to,
            combine: Some(Combine {
                merge_coplanar: false,
                ..combine
            }),
            until,
        },
    })?;
    for (axis, offset) in [(0, 20.), (1, 0.), (1, 20.)] {
        assert_eq!(
            planar_faces(&s, joined, axis, offset)?,
            2,
            "{axis} {offset}"
        );
    }
    Ok(())
}

fn multiple_fixture() -> Result<(
    Session,
    PartId,
    Vec<nator_document::features::ExtrusionProfile>,
)> {
    use nator_document::features::{ExtrusionProfile, ProfileRef};
    let (mut s, part, first) = fixture(false)?;
    let mut bottom = Sketch::new("Segunda base", Plane::xy());
    bottom.add_rectangle([40., 0.], [60., 20.], false)?;
    let (_, sketch) = add_sketch(&mut s, Some(part), bottom)?;
    let second = created(s.execute(Command::AddFeature {
        part,
        name: "Segunda base".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "10 mm".into(),
            combine: None,
            until: None,
        },
    })?)?;
    let mut profiles = [first, second]
        .into_iter()
        .map(|base| ExtrusionProfile::Face {
            base,
            face: FaceRef::Named(format!("{}:end", body_owner(base))),
            boundaries: vec![],
        })
        .collect::<Vec<_>>();
    for (x, reversed) in [(80., false), (100., true)] {
        let mut plane = Plane {
            origin: [0., 0., 10.],
            ..Plane::xy()
        };
        if reversed {
            plane.v = [0., -1., 0.];
        }
        let mut profile = Sketch::new("Área independente", plane);
        profile.add_rectangle([x, 0.], [x + 10., if reversed { -10. } else { 10. }], false)?;
        let (_, sketch) = add_sketch(&mut s, Some(part), profile)?;
        profiles.push(ExtrusionProfile::Sketch(ProfileRef {
            sketch,
            region: RegionRef::Index(0),
        }));
    }
    Ok((s, part, profiles))
}
fn multiple_operation(
    profiles: Vec<nator_document::features::ExtrusionProfile>,
    to: &str,
    combine: Option<Combine>,
) -> Operation {
    Operation::ExtrudeMany {
        profiles,
        profile_keys: vec![],
        from: "0 mm".into(),
        to: to.into(),
        until: None,
        combine,
    }
}

#[test]
fn multiple_coplanar_faces_join_and_cut_both_bodies_with_exact_previews() -> Result<()> {
    for (to, mode, expected) in [
        ("5 mm", BooleanMode::Join, 12000.),
        ("-3 mm", BooleanMode::Cut, 5600.),
    ] {
        let (mut s, part, mut profiles) = multiple_fixture()?;
        profiles.truncate(2);
        let target = profiles[0].base().ok_or("Base")?;
        let sources = profiles.iter().filter_map(|p| p.base()).collect::<Vec<_>>();
        let operation = multiple_operation(
            profiles,
            to,
            Some(Combine {
                target,
                mode,
                merge_coplanar: mode == BooleanMode::Join,
            }),
        );
        let preview = if mode == BooleanMode::Cut {
            extrusion::removed(s.document(), part, &operation)?
        } else {
            extrusion::added(s.document(), part, &operation)?
        }
        .ok_or("Prévia")?;
        assert!((preview.signed_volume().abs() - (expected - 8000_f64).abs()).abs() < 1e-4);
        let count = s.document().part(part).ok_or("Peça")?.features.len();
        let output = created(s.execute(Command::AddFeature {
            part,
            name: "Extrusão conjunta".into(),
            operation,
        })?)?;
        assert!((volume(&s, output)? - expected).abs() < 1e-4);
        assert_eq!(
            s.document().part(part).ok_or("Peça")?.features.len(),
            count + 1
        );
        for source in &sources {
            assert!(!s.document().body(*source).ok_or("Corpo")?.visible);
        }
        assert!(s.undo());
        for source in &sources {
            assert!(s.document().body(*source).ok_or("Corpo")?.visible);
        }
        assert!(s.redo());
        assert!((volume(&s, output)? - expected).abs() < 1e-4);
    }
    Ok(())
}

#[test]
fn mixed_extrusion_keeps_opposite_normals_and_names_when_reordered_saved_and_cloned() -> Result<()>
{
    let (mut s, part, profiles) = multiple_fixture()?;
    let output = created(s.execute(Command::AddFeature {
        part,
        name: "Quatro perfis".into(),
        operation: multiple_operation(profiles, "5 mm", None),
    })?)?;
    assert!((volume(&s, output)? - 5000.).abs() < 1e-4);
    let bounds = s
        .document()
        .body(output)
        .ok_or("Corpo")?
        .geometry
        .mesh()
        .bounds();
    assert!((bounds.min[2] - 10.).abs() < 1e-6 && (bounds.max[2] - 15.).abs() < 1e-6);
    let names = s
        .document()
        .body(output)
        .ok_or("Corpo")?
        .face_names
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let mut operation = s
        .document()
        .part(part)
        .ok_or("Peça")?
        .features
        .last()
        .ok_or("Operação")?
        .operation
        .clone();
    if let Operation::ExtrudeMany {
        profiles,
        profile_keys,
        ..
    } = &mut operation
    {
        profiles.swap(0, 1);
        profile_keys.swap(0, 1);
    }
    s.execute(Command::EditFeature {
        part,
        output,
        operation,
    })?;
    assert_eq!(
        names,
        s.document()
            .body(output)
            .ok_or("Corpo")?
            .face_names
            .iter()
            .cloned()
            .collect()
    );
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("multipla.3dnator");
    nator_project_io::write_project(&path, s.document())?;
    let mut s = Session::new(nator_project_io::read_project(&path)?)?;
    let instance = s
        .document()
        .instances()
        .iter()
        .find(|i| i.part == part)
        .ok_or("Instância")?
        .id;
    s.execute(Command::CloneInstance {
        id: instance,
        name: "Cópia".into(),
    })?;
    let clone = s
        .document()
        .parts()
        .iter()
        .find(|p| p.id != part)
        .ok_or("Clone")?;
    let copy = clone.id;
    let copied_output = clone.features.last().ok_or("Operação")?.output;
    let original_keys = match &s
        .document()
        .part(part)
        .ok_or("Peça")?
        .features
        .last()
        .ok_or("Operação")?
        .operation
    {
        Operation::ExtrudeMany { profile_keys, .. } => profile_keys.clone(),
        _ => return Err("Operação".into()),
    };
    assert!(
        matches!(&clone.features.last().ok_or("Operação")?.operation,Operation::ExtrudeMany {profile_keys,..} if *profile_keys==original_keys)
    );
    s.execute(Command::SetParameter {
        part: copy,
        name: "teste".into(),
        expression: Some("1 mm".into()),
        dimension: Dimension::Length,
    })?;
    assert!((volume(&s, copied_output)? - 5000.).abs() < 1e-4);
    s.document().validate()?;
    Ok(())
}

#[test]
fn multiple_extrusion_rejects_non_coplanar_or_duplicate_profiles_atomically() -> Result<()> {
    let (mut s, part, profiles) = multiple_fixture()?;
    let mut invalid = profiles[..2].to_vec();
    if let nator_document::features::ExtrusionProfile::Face { base, face, .. } = &mut invalid[1] {
        *face = FaceRef::Named(format!("{}:start", body_owner(*base)));
    }
    for selection in [invalid, vec![profiles[0].clone(), profiles[0].clone()]] {
        let before = s.document().clone();
        assert!(
            s.execute(Command::AddFeature {
                part,
                name: "Inválida".into(),
                operation: multiple_operation(selection, "5 mm", None)
            })
            .is_err()
        );
        assert_eq!(s.document().parts().len(), before.parts().len());
        assert_eq!(
            s.document().part(part).ok_or("Peça")?.features.len(),
            before.part(part).ok_or("Peça")?.features.len()
        );
    }
    Ok(())
}

#[test]
fn multiple_extrusion_respects_face_cells_and_shared_associative_limit() -> Result<()> {
    use nator_document::features::{ExtrusionLimit, ExtrusionProfile, ProfileRef};
    let (mut s, part, base, _) = plate_with_contour()?;
    let Operation::ExtrudeFace {
        base,
        face,
        boundaries,
        ..
    } = clicked_area(&s, base, 2., 2., "5 mm")?
    else {
        return Err("Área".into());
    };
    let mut free = Sketch::new(
        "Livre",
        Plane {
            origin: [0., 0., 10.],
            ..Plane::xy()
        },
    );
    free.add_rectangle([40., 0.], [50., 10.], false)?;
    let (_, sketch) = add_sketch(&mut s, Some(part), free)?;
    let (_, limit) = add_sketch(
        &mut s,
        Some(part),
        Sketch::new(
            "Limite",
            Plane {
                origin: [0., 0., 17.],
                ..Plane::xy()
            },
        ),
    )?;
    let mut operation = multiple_operation(
        vec![
            ExtrusionProfile::Face {
                base,
                face,
                boundaries,
            },
            ExtrusionProfile::Sketch(ProfileRef {
                sketch,
                region: RegionRef::Index(0),
            }),
        ],
        "5 mm",
        None,
    );
    if let Operation::ExtrudeMany { until, .. } = &mut operation {
        *until = Some(ExtrusionLimit::Sketch { sketch: limit });
    }
    let output = created(s.execute(Command::AddFeature {
        part,
        name: "Célula e região".into(),
        operation,
    })?)?;
    assert!((volume(&s, output)? - 400. * 7.).abs() < 1e-4);
    assert!(!s.document().sketch(sketch).ok_or("Perfil")?.visible);
    assert!(s.document().sketch(limit).ok_or("Limite")?.visible);
    // Deslocar somente o perfil quebra a coplanaridade: o documento anterior é preservado.
    let before = volume(&s, output)?;
    let mut moved = s.document().sketch(sketch).ok_or("Perfil")?.sketch.clone();
    moved.plane.origin[2] = 11.;
    assert!(
        s.execute(Command::UpdateSketch {
            id: sketch,
            sketch: moved
        })
        .is_err()
    );
    assert!((volume(&s, output)? - before).abs() < 1e-6);
    Ok(())
}
