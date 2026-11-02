//! Verificar sólido com o núcleo real: relatório, reparo no histórico, recálculo e STL.
#![cfg(feature = "occt")]
use nator_commands::{
    Command, Effect, Session,
    export::{self, Quality, Scope},
    solid_check,
};
use nator_document::{
    BodyId, Document, PartId, SketchId,
    features::{BooleanMode, Combine, ContactRepair, FaceRef, Operation, RegionRef},
};
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch, areas};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn sketch(s: &mut Session, part: Option<PartId>, sketch: Sketch) -> Result<(PartId, SketchId)> {
    let Effect::SketchCreated { part, sketch } = s.execute(Command::AddSketch {
        destination: part,
        sketch,
    })?
    else {
        return Err("Rascunho ausente".into());
    };
    Ok((part, sketch))
}
fn extrude(
    s: &mut Session,
    part: PartId,
    id: SketchId,
    from: &str,
    to: &str,
    combine: Option<Combine>,
) -> Result<BodyId> {
    let piece = s.document().part(part).ok_or("Peça")?;
    let profile = &piece
        .sketches
        .iter()
        .find(|r| r.id == id)
        .ok_or("Perfil")?
        .sketch;
    let signature = areas::signature(profile, &areas::regions(profile)?[0])?;
    let Effect::Imported { body, .. } = s.execute(Command::AddFeature {
        part,
        name: "Extrusão".into(),
        operation: Operation::Extrude {
            sketch: id,
            region: RegionRef::Area(signature),
            from: from.into(),
            to: to.into(),
            combine,
            until: None,
        },
    })?
    else {
        return Err("Corpo ausente".into());
    };
    Ok(body)
}
fn rectangle(name: &str, a: [f64; 2], b: [f64; 2]) -> Result<Sketch> {
    let mut sketch = Sketch::new(name, Plane::xy());
    sketch.add_rectangle(a, b, false)?;
    Ok(sketch)
}
/// Placa com dois entalhes rasos que se tocam em (20; 20), com origem na segunda operação
/// de corte. Devolve também o rascunho do último entalhe, usado para desfazer o contato.
fn contact_part() -> Result<(Session, PartId, SketchId, BodyId)> {
    let mut s = Session::new(Document::new("Contato")?)?;
    let (part, plate) = sketch(&mut s, None, rectangle("Placa", [0., 0.], [30., 30.])?)?;
    let base = extrude(&mut s, part, plate, "0 mm", "10 mm", None)?;
    let (_, first) = sketch(&mut s, Some(part), rectangle("A", [10., 20.], [20., 30.])?)?;
    let cut = |target| {
        Some(Combine {
            target,
            mode: BooleanMode::Cut,
            merge_coplanar: false,
        })
    };
    let once = extrude(&mut s, part, first, "5 mm", "10 mm", cut(base))?;
    let (_, second) = sketch(&mut s, Some(part), rectangle("B", [20., 10.], [30., 20.])?)?;
    let twice = extrude(&mut s, part, second, "5 mm", "10 mm", cut(once))?;
    Ok((s, part, second, twice))
}
fn repair(s: &mut Session, part: PartId, base: BodyId, mode: ContactRepair) -> Result<BodyId> {
    let report = solid_check::inspect(s.document(), part, base)?;
    let Effect::Imported { body, .. } = s.execute(Command::AddFeature {
        part,
        name: "Reparo".into(),
        operation: Operation::RepairContacts {
            base,
            contacts: report.contacts.into_iter().map(|c| c.faces).collect(),
            mode,
            radius: "0.05 mm".into(),
        },
    })?
    else {
        return Err("Corpo ausente".into());
    };
    Ok(body)
}
fn volume(s: &Session, id: BodyId) -> Result<f64> {
    let body = s.document().body(id).ok_or("Corpo")?;
    Ok(Solid::from_brep(body.brep.as_ref().ok_or("B-rep")?, Some(&body.face_names))?.volume()?)
}
fn stl(s: &Session, id: BodyId) -> Result<Vec<u8>> {
    let file = export::plan(s.document(), Scope::Body(id), Quality::Standard)?.remove(0);
    let mut bytes = Vec::new();
    nator_project_io::encode_stl(&file.mesh, &mut bytes)?;
    Ok(bytes)
}

#[test]
fn report_locates_the_contact_and_the_step_that_created_it() -> Result<()> {
    let (s, part, _, body) = contact_part()?;
    let report = solid_check::inspect(s.document(), part, body)?;
    assert_eq!(report.contacts.len(), 1);
    assert!(!report.unexplained_mesh_defects());
    assert!(report.mesh.non_manifold_edges > 0);
    let contact = &report.contacts[0];
    assert!((contact.length - 5.).abs() < 1e-9);
    let origin = contact.origin.as_ref().ok_or("Origem")?;
    assert_eq!(origin.step, 3);
    assert_eq!(origin.sketch.as_deref(), Some("B"));
    let clean = solid_check::inspect(
        s.document(),
        part,
        s.document().part(part).ok_or("Peça")?.features[0].output,
    )?;
    assert!(clean.contacts.is_empty());
    assert!(!clean.unexplained_mesh_defects());
    Ok(())
}

#[test]
fn stl_error_points_to_the_defect_and_repair_unblocks_the_export() -> Result<()> {
    let (mut s, part, _, body) = contact_part()?;
    let message = stl(&s, body).err().ok_or("Sem erro")?.to_string();
    assert!(message.contains("20,000; 20,000"), "{message}");
    assert!(message.contains("Verificar sólido"), "{message}");
    let before = volume(&s, body)?;
    for (mode, sign) in [(ContactRepair::Weld, 1.), (ContactRepair::Separate, -1.)] {
        let revision = s.revision();
        let repaired = repair(&mut s, part, body, mode)?;
        assert!(!stl(&s, repaired)?.is_empty());
        assert!(
            solid_check::inspect(s.document(), part, repaired)?
                .contacts
                .is_empty()
        );
        let change = (volume(&s, repaired)? - before) * sign;
        // O pino de 5 mm ocupa os dois quadrantes opostos em volta da aresta.
        let half_pin = std::f64::consts::PI * 0.05 * 0.05 * 5. / 2.;
        assert!((change - half_pin).abs() < 1e-6);
        assert!(s.undo());
        assert_eq!(s.revision(), revision + 2);
    }
    Ok(())
}

#[test]
fn repair_follows_the_history_and_fails_when_the_contact_disappears() -> Result<()> {
    let (mut s, part, second, body) = contact_part()?;
    let repaired = repair(&mut s, part, body, ContactRepair::Weld)?;
    // Mudar a altura preserva o contato: o reparo é refeito pelos nomes das faces.
    let record = s.document().part(part).ok_or("Peça")?;
    let moved = record
        .sketches
        .iter()
        .find(|r| r.id == second)
        .ok_or("B")?
        .sketch
        .clone();
    let mut wider = moved.clone();
    for point in &mut wider.points {
        if point.position[1] == 10. {
            point.position[1] = 5.;
        }
    }
    s.execute(Command::UpdateSketch {
        id: second,
        sketch: wider,
    })?;
    assert!(!stl(&s, repaired)?.is_empty());
    // Afastar o entalhe elimina o contato: falha explícita e documento preservado.
    let revision = s.revision();
    let mut away = moved;
    for point in &mut away.points {
        point.position[1] -= 1.;
    }
    let failure = s
        .execute(Command::UpdateSketch {
            id: second,
            sketch: away,
        })
        .err()
        .ok_or("Sem erro")?;
    assert!(failure.to_string().contains("não existe mais"), "{failure}");
    assert_eq!(s.revision(), revision);
    Ok(())
}

#[test]
fn repair_survives_saving_and_part_copies() -> Result<()> {
    let (mut s, part, _, body) = contact_part()?;
    let repaired = repair(&mut s, part, body, ContactRepair::Separate)?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("reparo.3dnator");
    nator_project_io::write_project(&path, s.document())?;
    let loaded = nator_project_io::read_project(&path)?;
    let feature = loaded
        .part(part)
        .ok_or("Peça")?
        .features
        .last()
        .ok_or("Operação")?;
    assert_eq!(feature.output, repaired);
    assert!(matches!(
        &feature.operation,
        Operation::RepairContacts { mode: ContactRepair::Separate, contacts, .. } if contacts.len() == 1
    ));
    Ok(())
}

#[test]
fn bed_face_comes_from_the_outward_normal_and_follows_the_history() -> Result<()> {
    let (mut s, part, _, body) = contact_part()?;
    let record = s.document().body(body).ok_or("Corpo")?;
    // Maior face plana com normal +Z no modelo: o topo da placa, em z = 5.
    let names = record.face_names.as_ref().clone();
    let mesh = record.geometry.mesh().clone();
    let faces = record.geometry.triangle_faces().ok_or("Faces")?.to_vec();
    let top = names
        .iter()
        .enumerate()
        .find(|(index, _)| {
            faces.iter().zip(mesh.triangles()).any(|(face, t)| {
                let [a, b, c] = mesh.triangle_points(*t);
                *face == *index as u32
                    && nator_geometry::cross(nator_geometry::sub(b, a), nator_geometry::sub(c, a))
                        [2]
                        > 0.
                    && a[2] == 10.
            })
        })
        .map(|(_, name)| name.clone())
        .ok_or("Topo")?;
    s.execute(Command::SetPrintFace {
        body,
        face: Some(FaceRef::Named(top)),
    })?;
    let record = s.document().body(body).ok_or("Corpo")?;
    assert_eq!(
        record.print_down.map(|d| d.map(f64::round)),
        Some([0., 0., 1.])
    );
    let file = export::plan(s.document(), Scope::Body(body), Quality::Standard)?.remove(0);
    let layout = export::prepare_for_bed(std::slice::from_ref(&file), None)?;
    // Virada de cabeça para baixo: os entalhes, antes em cima, ficam junto à mesa.
    assert!(layout.files[0].mesh.bounds().min[2].abs() < 1e-9);
    assert!((layout.files[0].size[2] - 10.).abs() < 1e-9);
    // A orientação passa para o corpo seguinte, e `None` a remove.
    let repaired = repair(&mut s, part, body, ContactRepair::Weld)?;
    assert!(
        s.document()
            .body(repaired)
            .ok_or("Reparo")?
            .print_down
            .is_some()
    );
    s.execute(Command::SetPrintFace {
        body: repaired,
        face: None,
    })?;
    assert!(
        s.document()
            .body(repaired)
            .ok_or("Reparo")?
            .print_down
            .is_none()
    );
    // Face curva não define apoio.
    let mut round = Session::new(Document::new("Curva")?)?;
    let mut disk = Sketch::new("Disco", Plane::xy());
    disk.add_circle([0., 0.], 5., false)?;
    let (part, id) = sketch(&mut round, None, disk)?;
    let cylinder = extrude(&mut round, part, id, "0 mm", "5 mm", None)?;
    let record = round.document().body(cylinder).ok_or("Cilindro")?;
    let wall = record
        .face_names
        .iter()
        .find(|name| !name.ends_with(":start") && !name.ends_with(":end"))
        .cloned()
        .ok_or("Parede")?;
    let failure = round
        .execute(Command::SetPrintFace {
            body: cylinder,
            face: Some(FaceRef::Named(wall)),
        })
        .err()
        .ok_or("Sem erro")?;
    assert!(failure.to_string().contains("face plana"), "{failure}");
    Ok(())
}
