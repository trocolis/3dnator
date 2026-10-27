//! Extrusão com o núcleo real: modos automáticos, faces, áreas e volume da prévia.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session, extrusion};
use nator_document::{
    BodyId, Document, PartId, SketchId,
    features::{BooleanMode, Combine, FaceRef, Operation, RegionRef, UntilFace, body_owner},
    parameters::Dimension,
};
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch};
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
                mode: BooleanMode::Join
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
                mode: BooleanMode::Cut
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
    let result = created(s.execute(Command::AddFeature {
        part,
        name: "Até face".into(),
        operation: Operation::Extrude {
            sketch: id,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "1 mm".into(),
            combine: None,
            until: Some(UntilFace {
                body: imported,
                face: FaceRef::Named(format!("{}:f{index}", body_owner(imported))),
            }),
        },
    })?)?;
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
            }),
            until: None,
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
