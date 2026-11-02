//! Referências por índice no recálculo: faces com nome repetido só têm índice, e o recálculo
//! precisa reencontrá-las sem aceitar outra geometria no lugar.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session};
use nator_document::{
    BodyId, Document, PartId, SketchId,
    features::{BooleanMode, Combine, EdgeRef, FaceRef, Operation, RegionRef, signature},
};
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch};
use std::sync::Arc;
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
fn add(s: &mut Session, part: PartId, operation: Operation) -> Result<BodyId> {
    let Effect::Imported { body, .. } = s.execute(Command::AddFeature {
        part,
        name: "Operação".into(),
        operation,
    })?
    else {
        return Err("Corpo ausente".into());
    };
    Ok(body)
}
fn solid(s: &Session, id: BodyId) -> Result<Solid> {
    let body = s.document().body(id).ok_or("Corpo")?;
    Ok(Solid::from_brep(
        body.brep.as_ref().ok_or("B-rep")?,
        Some(&body.face_names),
    )?)
}
fn stamp(s: &Session, id: BodyId) -> Result<u64> {
    let body = s.document().body(id).ok_or("Corpo")?;
    Ok(signature(body.brep.as_ref().ok_or("B-rep")?))
}
fn slot_sketch(from: f64, to: f64) -> Result<Sketch> {
    let mut profile = Sketch::new("Canal", Plane::xy());
    profile.plane.origin = [0., 0., 10.];
    profile.add_rectangle([from, -1.], [to, 21.], false)?;
    Ok(profile)
}
fn slot(id: SketchId, base: BodyId, depth: &str) -> Operation {
    Operation::Extrude {
        sketch: id,
        region: RegionRef::Index(0),
        from: "0 mm".into(),
        to: format!("-({depth})"),
        combine: Some(Combine {
            target: base,
            mode: BooleanMode::Cut,
            merge_coplanar: false,
        }),
        until: None,
    }
}
struct Scene {
    session: Session,
    part: PartId,
    block: BodyId,
    slot: SketchId,
    cut: BodyId,
    top: f64,
}
/// Bloco de 40 × 20 × 10 mm com um canal fora do centro, que divide o topo em duas faces de
/// mesmo nome e áreas diferentes.
fn scene() -> Result<Scene> {
    let mut session = Session::new(Document::new("Referências")?)?;
    let mut profile = Sketch::new("Bloco", Plane::xy());
    profile.add_rectangle([0., 0.], [40., 20.], false)?;
    let (part, id) = sketch(&mut session, None, profile)?;
    let block = add(
        &mut session,
        part,
        Operation::Extrude {
            sketch: id,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "10 mm".into(),
            combine: None,
            until: None,
        },
    )?;
    let (_, slot_id) = sketch(&mut session, Some(part), slot_sketch(10., 14.)?)?;
    let cut = add(&mut session, part, slot(slot_id, block, "2 mm"))?;
    Ok(Scene {
        session,
        part,
        block,
        slot: slot_id,
        cut,
        top: 26. * 20.,
    })
}
/// Rebaixa 1 mm a metade do topo com a área pedida, guardada só por índice.
fn lower(scene: &mut Scene, area: f64) -> Result<BodyId> {
    let body = solid(&scene.session, scene.cut)?;
    let index = (0..body.face_count()?)
        .find(|i| {
            body.face_measure(*i)
                .is_ok_and(|m| (m.area - area).abs() < 1e-6)
        })
        .ok_or("Face")?;
    let signature = stamp(&scene.session, scene.cut)?;
    add(
        &mut scene.session,
        scene.part,
        Operation::ExtrudeFace {
            base: scene.cut,
            face: FaceRef::Index { index, signature },
            from: "0 mm".into(),
            to: "-(1 mm)".into(),
            combine: Some(Combine {
                target: scene.cut,
                mode: BooleanMode::Cut,
                merge_coplanar: false,
            }),
            until: None,
            boundaries: Vec::new(),
        },
    )
}
fn removed(scene: &Scene, body: BodyId) -> Result<f64> {
    Ok(solid(&scene.session, scene.cut)?.volume()? - solid(&scene.session, body)?.volume()?)
}
fn move_slot(scene: &mut Scene, from: f64, to: f64) -> Result<()> {
    scene.session.execute(Command::UpdateSketch {
        id: scene.slot,
        sketch: slot_sketch(from, to)?,
    })?;
    Ok(())
}

#[test]
fn a_repeated_name_keeps_the_face_by_index() -> Result<()> {
    let mut scene = scene()?;
    let area = scene.top;
    let lowered = lower(&mut scene, area)?;
    let feature = scene
        .session
        .document()
        .part(scene.part)
        .and_then(|p| p.features.iter().find(|f| f.output == lowered))
        .ok_or("Operação")?;
    assert!(matches!(
        feature.operation,
        Operation::ExtrudeFace {
            face: FaceRef::Index { .. },
            ..
        }
    ));
    assert!((removed(&scene, lowered)? - 520.).abs() < 1e-6);
    Ok(())
}

#[test]
fn a_change_elsewhere_keeps_the_chosen_face_and_can_repeat() -> Result<()> {
    let mut scene = scene()?;
    let area = scene.top;
    let lowered = lower(&mut scene, area)?;
    // Só a profundidade muda: o topo fica igual, mas o B-rep da base não.
    let (block, id) = (scene.block, scene.slot);
    let before = stamp(&scene.session, scene.cut)?;
    scene.session.execute(Command::EditFeature {
        part: scene.part,
        output: scene.cut,
        operation: slot(id, block, "3 mm"),
    })?;
    assert_ne!(stamp(&scene.session, scene.cut)?, before);
    assert!((removed(&scene, lowered)? - 520.).abs() < 1e-6);
    // A referência foi atualizada: um segundo recálculo funciona do mesmo jeito.
    scene.session.execute(Command::EditFeature {
        part: scene.part,
        output: scene.cut,
        operation: slot(id, block, "4 mm"),
    })?;
    assert!((removed(&scene, lowered)? - 520.).abs() < 1e-6);
    Ok(())
}

#[test]
fn the_face_grows_with_the_model_and_stays_the_chosen_side() -> Result<()> {
    let mut scene = scene()?;
    let area = scene.top;
    let lowered = lower(&mut scene, area)?;
    // O canal anda para a esquerda: a parte escolhida, à direita, cresce.
    move_slot(&mut scene, 6., 10.)?;
    assert!((removed(&scene, lowered)? - 30. * 20.).abs() < 1e-6);
    Ok(())
}

#[test]
fn a_face_that_left_its_place_is_an_explicit_error() -> Result<()> {
    let mut scene = scene()?;
    let area = scene.top;
    let lowered = lower(&mut scene, area)?;
    let before = removed(&scene, lowered)?;
    // O canal passa por cima do ponto da face: nenhuma das metades pode ser escolhida no lugar.
    let error = move_slot(&mut scene, 14., 38.).err().ok_or("Erro")?;
    assert!(error.to_string().contains("A geometria mudou"), "{error}");
    assert!((removed(&scene, lowered)? - before).abs() < 1e-9);
    Ok(())
}

#[test]
fn edges_are_found_again_after_a_change_elsewhere() -> Result<()> {
    let mut scene = scene()?;
    let body = solid(&scene.session, scene.cut)?;
    // Aresta superior da frente, à esquerda do canal: o par de faces se repete à direita.
    let edge = (0..body.edge_count()?)
        .find(|i| {
            body.edge_anchors(*i).is_ok_and(|a| {
                (a.midpoint[0] - 5.).abs() < 1e-6
                    && a.midpoint[1].abs() < 1e-6
                    && (a.midpoint[2] - 10.).abs() < 1e-6
            })
        })
        .ok_or("Aresta")?;
    let signature = stamp(&scene.session, scene.cut)?;
    let chamfered = add(
        &mut scene.session,
        scene.part,
        Operation::Chamfer {
            base: scene.cut,
            edge: EdgeRef::Index {
                index: edge,
                signature,
            },
            distance: "1 mm".into(),
            angle: None,
            additional_edges: Vec::new(),
        },
    )?;
    let expected = 0.5 * 10.;
    assert!((removed(&scene, chamfered)? - expected).abs() < 1e-6);
    let (block, id) = (scene.block, scene.slot);
    scene.session.execute(Command::EditFeature {
        part: scene.part,
        output: scene.cut,
        operation: slot(id, block, "3 mm"),
    })?;
    assert!((removed(&scene, chamfered)? - expected).abs() < 1e-6);
    Ok(())
}

#[test]
fn a_sketch_no_operation_uses_does_not_rebuild_the_part() -> Result<()> {
    let mut scene = scene()?;
    let area = scene.top;
    let lowered = lower(&mut scene, area)?;
    let mut empty = Sketch::new("Vazio", Plane::xy());
    let (_, id) = sketch(&mut scene.session, Some(scene.part), empty.clone())?;
    let brep = |s: &Session| s.document().body(lowered).and_then(|b| b.brep.clone());
    let before = brep(&scene.session).ok_or("B-rep")?;
    empty.add_line([0., 0.], [5., 5.], false)?;
    scene
        .session
        .execute(Command::UpdateSketch { id, sketch: empty })?;
    assert!(Arc::ptr_eq(&before, &brep(&scene.session).ok_or("B-rep")?));
    Ok(())
}
