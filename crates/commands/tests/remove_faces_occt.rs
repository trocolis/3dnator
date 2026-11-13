//! Remover faces: só as faces que precisam sair junto são sugeridas, confirmadas no núcleo.
#![cfg(feature = "occt")]
use nator_commands::{
    Command, Effect, Session,
    features::{Necessary, necessary_faces, probe_in_process, removal_plan},
};
use nator_document::{
    BodyId, Document, PartId,
    features::{EdgeRef, FaceRef, Operation, RegionRef, body_owner, signature},
};
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn imported(effect: Effect) -> Result<BodyId> {
    match effect {
        Effect::Imported { body, .. } => Ok(body),
        _ => Err("Corpo ausente".into()),
    }
}

/// Bloco 20 × 20 × 10 com as arestas indicadas (entre a tampa e cada lado) arredondadas.
fn rounded(edges: &[usize], radius: &str) -> Result<(Session, PartId, BodyId)> {
    let mut sketch = Sketch::new("Base", Plane::xy());
    let lines = sketch.add_rectangle([0., 0.], [20., 20.], false)?;
    let mut doc = Document::new("Remover")?;
    let (part, sketch) = doc.add_sketch(None, sketch)?;
    let mut session = Session::new(doc)?;
    let block = imported(session.execute(Command::AddFeature {
        part,
        name: "Bloco".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "10 mm".into(),
            combine: None,
            until: None,
        },
    })?)?;
    let owner = body_owner(block);
    let edge =
        |i: usize| EdgeRef::Between([format!("{owner}:end"), format!("{owner}:e{}", lines[i])]);
    let rounded = imported(session.execute(Command::AddFeature {
        part,
        name: "Arredondar".into(),
        operation: Operation::Fillet {
            base: block,
            edge: edge(edges[0]),
            radius: radius.into(),
            additional_edges: edges[1..].iter().map(|i| edge(*i)).collect(),
        },
    })?)?;
    Ok((session, part, rounded))
}

/// Bloco com um canto vertical arredondado e, depois, o contorno de cima arredondado: a
/// cadeia tangente do segundo arredondamento contorna o canto.
fn chained() -> Result<(Session, PartId, BodyId)> {
    let mut sketch = Sketch::new("Base", Plane::xy());
    let lines = sketch.add_rectangle([0., 0.], [20., 20.], false)?;
    let mut doc = Document::new("Remover")?;
    let (part, sketch) = doc.add_sketch(None, sketch)?;
    let mut session = Session::new(doc)?;
    let block = imported(session.execute(Command::AddFeature {
        part,
        name: "Bloco".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "10 mm".into(),
            combine: None,
            until: None,
        },
    })?)?;
    let owner = body_owner(block);
    let corner = imported(session.execute(Command::AddFeature {
        part,
        name: "Canto".into(),
        operation: Operation::Fillet {
            base: block,
            edge: EdgeRef::Between([
                format!("{owner}:e{}", lines[0]),
                format!("{owner}:e{}", lines[1]),
            ]),
            radius: "5 mm".into(),
            additional_edges: vec![],
        },
    })?)?;
    let top = imported(session.execute(Command::AddFeature {
        part,
        name: "Contorno".into(),
        operation: Operation::Fillet {
            base: corner,
            edge: EdgeRef::Between([format!("{owner}:end"), format!("{owner}:e{}", lines[0])]),
            radius: "2 mm".into(),
            additional_edges: vec![],
        },
    })?)?;
    Ok((session, part, top))
}

/// Índice de cada face do corpo com o raio da concordância (cilindro, esfera, toro pelo tubo).
fn blend_radii(solid: &Solid) -> Result<Vec<Option<f64>>> {
    let mut radii = Vec::new();
    for index in 0..solid.face_count()? {
        radii.push(match solid.face_measure(index)?.kind {
            nator_kernel::FaceKind::Cylinder { radius }
            | nator_kernel::FaceKind::Sphere { radius }
            | nator_kernel::FaceKind::Torus {
                minor_radius: radius,
                ..
            } => Some(radius),
            _ => None,
        });
    }
    Ok(radii)
}

/// A busca do aplicativo, com as sondagens no próprio processo (sem prazo).
fn necessary(
    session: &Session,
    part: PartId,
    body: BodyId,
    faces: &[FaceRef],
) -> Result<Necessary> {
    let plan = removal_plan(session.document(), part, body, faces)?;
    Ok(necessary_faces(
        &plan,
        &|faces, _| Some(probe_in_process(&plan.bytes, faces)),
        &|| false,
        1,
    ))
}

#[test]
fn a_chained_fillet_face_gets_exactly_the_faces_it_needs() -> Result<()> {
    let (mut session, part, body) = chained()?;
    let record = session.document().body(body).ok_or("Corpo")?;
    let bytes = record.brep.clone().ok_or("B-rep")?;
    let solid = Solid::from_brep(&bytes, Some(&record.face_names))?;
    let reference = |index| FaceRef::Index {
        index,
        signature: signature(&bytes),
    };
    let index = |face: &FaceRef| match face {
        FaceRef::Index { index, .. } => Ok(*index),
        _ => Err("Referência por índice"),
    };
    let radii = blend_radii(&solid)?;
    let contour = (0..radii.len())
        .filter(|i| radii[*i].is_some_and(|r| (r - 2.).abs() < 1e-6))
        .collect::<Vec<_>>();
    // O contorno de 2 mm passa pelo canto: cilindro, toro e cilindro.
    assert!(contour.len() >= 2, "{radii:?}");
    for &face in &contour {
        let chosen = vec![reference(face)];
        // Sozinho, um trecho da cadeia não sai.
        assert!(!probe_in_process(&bytes, &[face]));
        let Necessary::With(needed) = necessary(&session, part, body, &chosen)? else {
            return Err("Faces necessárias não encontradas".into());
        };
        let needed = needed
            .iter()
            .map(index)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        assert!(!needed.is_empty() && !needed.contains(&face));
        let all = std::iter::once(face)
            .chain(needed.iter().copied())
            .collect::<Vec<_>>();
        // Com elas a remoção sai, e sem qualquer uma delas não sai: nada é sugerido à toa.
        assert!(probe_in_process(&bytes, &all));
        for skip in &needed {
            let fewer = all
                .iter()
                .copied()
                .filter(|f| f != skip)
                .collect::<Vec<_>>();
            assert!(
                !probe_in_process(&bytes, &fewer),
                "{skip} dispensável em {all:?}"
            );
        }
        let sharp = imported(session.execute(Command::AddFeature {
            part,
            name: "Com as necessárias".into(),
            operation: Operation::RemoveFaces {
                base: body,
                faces: all.into_iter().map(reference).collect(),
            },
        })?)?;
        let result = session.document().body(sharp).ok_or("Resultado")?;
        assert!(result.face_names.len() < solid.face_count()?);
        assert!(session.undo());
    }
    Ok(())
}

#[test]
fn faces_that_leave_alone_have_no_dependents() -> Result<()> {
    let (session, part, body) = rounded(&[0, 1, 2, 3], "3 mm")?;
    let record = session.document().body(body).ok_or("Corpo")?;
    let bytes = record.brep.clone().ok_or("B-rep")?;
    let index = record
        .face_names
        .iter()
        .position(|n| n.ends_with(":round"))
        .ok_or("Arredondamento")?;
    let chosen = [FaceRef::Index {
        index,
        signature: signature(&bytes),
    }];
    assert_eq!(necessary(&session, part, body, &chosen)?, Necessary::Alone);
    // Uma tampa plana não é concordância: nada a sugerir.
    let cap = record
        .face_names
        .iter()
        .position(|n| n.ends_with(":start"))
        .ok_or("Tampa")?;
    let cap = [FaceRef::Index {
        index: cap,
        signature: signature(&bytes),
    }];
    assert!(!matches!(
        necessary(&session, part, body, &cap)?,
        Necessary::With(_)
    ));
    Ok(())
}
