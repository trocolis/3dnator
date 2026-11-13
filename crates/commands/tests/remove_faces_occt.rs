//! Remover faces: as outras faces da mesma concordância são sugeridas, e com elas a remoção sai.
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

#[test]
fn a_chained_fillet_face_brings_the_rest_of_its_blend_and_then_comes_off() -> Result<()> {
    let (mut session, part, body) = chained()?;
    let record = session.document().body(body).ok_or("Corpo")?;
    let bytes = record.brep.clone().ok_or("B-rep")?;
    let solid = Solid::from_brep(&bytes, Some(&record.face_names))?;
    let reference = |index| FaceRef::Index {
        index,
        signature: signature(&bytes),
    };
    let radii = blend_radii(&solid)?;
    let contour = (0..radii.len())
        .filter(|i| radii[*i].is_some_and(|r| (r - 2.).abs() < 1e-6))
        .collect::<Vec<_>>();
    // O contorno de 2 mm passa pelo canto: cilindro, toro e cilindro.
    assert!(contour.len() >= 2, "{radii:?}");
    for &index in &contour {
        let chosen = vec![reference(index)];
        let revision = session.revision();
        // Sozinho, um trecho da cadeia não sai: o documento fica como estava.
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
        // A concordância traz o resto do contorno, e nada do canto de 5 mm.
        let mut chain = removal_dependents(session.document(), part, body, &chosen)?;
        let mut expected = contour
            .iter()
            .filter(|i| **i != index)
            .map(|i| reference(*i))
            .collect::<Vec<_>>();
        chain.sort_by_key(|f| format!("{f:?}"));
        expected.sort_by_key(|f| format!("{f:?}"));
        assert_eq!(chain, expected);
        let all = chosen.into_iter().chain(chain).collect::<Vec<_>>();
        let sharp = imported(session.execute(Command::AddFeature {
            part,
            name: "Com a concordância".into(),
            operation: Operation::RemoveFaces {
                base: body,
                faces: all,
            },
        })?)?;
        let result = session.document().body(sharp).ok_or("Resultado")?;
        assert!(result.face_names.len() < solid.face_count()?);
        assert!(session.undo());
    }
    // O canto de 5 mm é outra concordância: nenhuma face do contorno é sugerida.
    let corner = (0..radii.len())
        .find(|i| radii[*i].is_some_and(|r| (r - 5.).abs() < 1e-6))
        .ok_or("Canto")?;
    assert!(removal_dependents(session.document(), part, body, &[reference(corner)])?.is_empty());
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
    // Os quatro arredondamentos se encontram em quinas vivas, sem tangência: cada um sai só.
    assert!(removal_dependents(session.document(), part, body, &chosen)?.is_empty());
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
    assert!(removal_dependents(session.document(), part, body, &cap)?.is_empty());
    Ok(())
}
