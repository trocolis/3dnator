//! Remover faces: faces que só saem junto com outras são encontradas e confirmadas no núcleo.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session, features::removal_dependents};
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

#[test]
fn a_face_that_cannot_leave_alone_gets_the_dependents_that_make_removal_work() -> Result<()> {
    let (mut session, part, body) = chained()?;
    let record = session.document().body(body).ok_or("Corpo")?;
    let bytes = record.brep.clone().ok_or("B-rep")?;
    let solid = Solid::from_brep(&bytes, Some(&record.face_names))?;
    let reference = |index| FaceRef::Index {
        index,
        signature: signature(&bytes),
    };
    let mut curved = Vec::new();
    for index in 0..solid.face_count()? {
        if !matches!(
            solid.face_measure(index)?.kind,
            nator_kernel::FaceKind::Plane { .. }
        ) {
            curved.push(index);
        }
    }
    assert!(!curved.is_empty());
    for index in curved {
        let chosen = vec![reference(index)];
        let revision = session.revision();
        // Sozinha, a face não sai: o documento fica como estava.
        assert!(
            session
                .execute(Command::AddFeature {
                    part,
                    name: "Sozinha".into(),
                    operation: Operation::RemoveFaces {
                        base: body,
                        faces: chosen.clone(),
                    },
                })
                .is_err()
        );
        assert_eq!(session.revision(), revision);
        let dependents = removal_dependents(session.document(), part, body, &chosen)?
            .ok_or("Nenhum conjunto encontrado")?;
        assert!(!dependents.is_empty());
        assert!(!dependents.contains(&reference(index)));
        // Com as dependentes, a remoção funciona e devolve as paredes planas do bloco.
        let all = chosen.into_iter().chain(dependents).collect::<Vec<_>>();
        let sharp = imported(session.execute(Command::AddFeature {
            part,
            name: "Com dependentes".into(),
            operation: Operation::RemoveFaces {
                base: body,
                faces: all,
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
    assert_eq!(
        removal_dependents(session.document(), part, body, &chosen)?,
        Some(Vec::new())
    );
    // Uma tampa nunca se resolve com vizinhas curvas: não há conjunto a sugerir.
    let cap = record
        .face_names
        .iter()
        .position(|n| n.ends_with(":start"))
        .ok_or("Tampa")?;
    let cap = [FaceRef::Index {
        index: cap,
        signature: signature(&bytes),
    }];
    assert_eq!(
        removal_dependents(session.document(), part, body, &cap)?,
        None
    );
    Ok(())
}
