//! Edição do histórico paramétrico com o núcleo real: supressão, marcador, reordenação,
//! exclusão com dependentes e falhas de recálculo atribuídas à operação responsável.
#![cfg(feature = "occt")]

use nator_commands::{Command, Effect, Session};
use nator_document::{
    BodyId, Document, PartId,
    features::{
        EdgeRef, FeatureState, LinearAxis, Operation, PatternLayout, RegionRef, body_owner, states,
    },
    parameters::Dimension,
};
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch};
use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn created(effect: Effect) -> Result<BodyId> {
    match effect {
        Effect::Imported { body, .. } => Ok(body),
        _ => Err("A operação não criou um corpo".into()),
    }
}

fn volume(session: &Session, body: BodyId) -> Result<f64> {
    let body = session.document().body(body).ok_or("Corpo ausente")?;
    let bytes = body.brep.as_ref().ok_or("Corpo sem B-rep")?;
    Ok(Solid::from_brep(bytes, Some(&body.face_names))?.volume()?)
}

fn visible(session: &Session, body: BodyId) -> Result<bool> {
    Ok(session
        .document()
        .body(body)
        .ok_or("Corpo ausente")?
        .visible)
}

fn feature(
    session: &mut Session,
    part: PartId,
    name: &str,
    operation: Operation,
) -> Result<BodyId> {
    created(session.execute(Command::AddFeature {
        part,
        name: name.into(),
        operation,
    })?)
}

fn translate(base: BodyId, x: f64) -> Operation {
    Operation::Transform {
        base,
        translation: [x, 0., 0.],
        rotation: [0.; 3],
    }
}

/// Placa 60 × 40 × `espessura` (A), deslocada (B) e repetida duas vezes (C).
struct Chain {
    session: Session,
    part: PartId,
    /// Linhas do contorno da placa, que dão nome às faces laterais.
    lines: Vec<u64>,
    a: BodyId,
    b: BodyId,
    c: BodyId,
}

fn chain() -> Result<Chain> {
    let mut session = Session::new(Document::new("Histórico")?)?;
    let mut outline = Sketch::new("Placa", Plane::xy());
    let lines = outline.add_rectangle([0., 0.], [60., 40.], false)?;
    let Effect::SketchCreated { part, sketch } = session.execute(Command::AddSketch {
        destination: None,
        sketch: outline,
    })?
    else {
        return Err("Rascunho não criado".into());
    };
    session.execute(Command::SetParameter {
        part,
        name: "espessura".into(),
        expression: Some("8 mm".into()),
        dimension: Dimension::Length,
    })?;
    let a = feature(
        &mut session,
        part,
        "Placa",
        Operation::Extrude {
            sketch,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "espessura".into(),
            combine: None,
            until: None,
        },
    )?;
    let b = feature(&mut session, part, "Deslocamento", translate(a, 10.))?;
    let c = feature(
        &mut session,
        part,
        "Repetição",
        Operation::Pattern {
            base: b,
            layout: PatternLayout::Linear {
                first: LinearAxis {
                    direction: [1., 0., 0.],
                    count: 2,
                    spacing: "70 mm".into(),
                },
                second: None,
            },
        },
    )?;
    Ok(Chain {
        session,
        part,
        lines,
        a,
        b,
        c,
    })
}

fn part_states(session: &Session, part: PartId) -> Result<Vec<FeatureState>> {
    Ok(states(session.document().part(part).ok_or("Peça ausente")?))
}

#[test]
fn suppression_hides_dependents_and_reactivation_rebuilds_them() -> Result<()> {
    let Chain {
        mut session,
        part,
        a,
        b,
        c,
        ..
    } = chain()?;
    session.execute(Command::SetFeatureSuppressed {
        part,
        output: b,
        suppressed: true,
    })?;
    assert_eq!(
        part_states(&session, part)?,
        [
            FeatureState::Active,
            FeatureState::Suppressed,
            FeatureState::Dependent
        ]
    );
    // A placa volta a ser o resultado visível; os resultados inativos ficam ocultos.
    assert!(visible(&session, a)? && !visible(&session, b)? && !visible(&session, c)?);
    assert!(
        session
            .execute(Command::SetBodyVisibility {
                id: c,
                visible: true
            })
            .is_err()
    );
    // Com a operação suprimida, alterar a espessura recalcula só o que está ativo.
    session.execute(Command::SetParameter {
        part,
        name: "espessura".into(),
        expression: Some("10 mm".into()),
        dimension: Dimension::Length,
    })?;
    assert!((volume(&session, a)? - 60. * 40. * 10.).abs() < 1e-6);
    // Reativar reconstrói as operações com a espessura atual.
    session.execute(Command::SetFeatureSuppressed {
        part,
        output: b,
        suppressed: false,
    })?;
    assert!(part_states(&session, part)?.iter().all(|s| s.is_active()));
    assert!((volume(&session, c)? - 2. * 60. * 40. * 10.).abs() < 1e-6);
    assert!(!visible(&session, a)? && !visible(&session, b)? && visible(&session, c)?);
    // Desfazer a reativação volta ao histórico suprimido, com a mesma visibilidade.
    assert!(session.undo());
    assert!(visible(&session, a)? && !visible(&session, c)?);
    Ok(())
}

#[test]
fn history_marker_rolls_back_inserts_and_persists() -> Result<()> {
    let Chain {
        mut session,
        part,
        a,
        b,
        c,
        ..
    } = chain()?;
    session.execute(Command::SetHistoryMarker {
        part,
        marker: Some(1),
    })?;
    assert!(visible(&session, a)? && !visible(&session, b)? && !visible(&session, c)?);
    // Resultados depois do marcador não podem ser usados pela nova operação.
    assert!(
        session
            .execute(Command::AddFeature {
                part,
                name: "Inválida".into(),
                operation: translate(c, 5.),
            })
            .is_err()
    );
    // A nova operação entra no marcador, que avança junto.
    let d = feature(&mut session, part, "Elevação", translate(a, -100.))?;
    let piece = session.document().part(part).ok_or("Peça ausente")?;
    let order = piece.features.iter().map(|f| f.output).collect::<Vec<_>>();
    assert_eq!(order, [a, d, b, c]);
    assert_eq!(piece.history_marker, Some(2));
    // Salvar e reabrir preservam o marcador e o estado de cada operação.
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("marcador.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    assert_eq!(
        reopened.part(part).ok_or("Peça ausente")?.history_marker,
        Some(2)
    );
    // Levar o marcador ao fim reativa e reconstrói as operações posteriores.
    session.execute(Command::SetHistoryMarker { part, marker: None })?;
    assert!(part_states(&session, part)?.iter().all(|s| s.is_active()));
    assert!(!visible(&session, a)? && visible(&session, d)? && visible(&session, c)?);
    assert!((volume(&session, c)? - 2. * 60. * 40. * 8.).abs() < 1e-6);
    Ok(())
}

#[test]
fn reordering_keeps_dependencies_before_their_users() -> Result<()> {
    let Chain {
        mut session,
        part,
        a,
        b,
        c,
        ..
    } = chain()?;
    let d = feature(&mut session, part, "Cópia lateral", translate(a, -100.))?;
    let error = session
        .execute(Command::MoveFeature {
            part,
            output: c,
            index: 1,
        })
        .err()
        .ok_or("Reordenação inválida aceita")?;
    assert!(error.to_string().contains("Deslocamento"));
    session.execute(Command::MoveFeature {
        part,
        output: d,
        index: 1,
    })?;
    let order = session
        .document()
        .part(part)
        .ok_or("Peça ausente")?
        .features
        .iter()
        .map(|f| f.output)
        .collect::<Vec<_>>();
    assert_eq!(order, [a, d, b, c]);
    Ok(())
}

#[test]
fn removal_names_dependents_and_removes_them_on_request() -> Result<()> {
    let Chain {
        mut session,
        part,
        a,
        b,
        c,
        ..
    } = chain()?;
    session.execute(Command::SetHistoryMarker {
        part,
        marker: Some(2),
    })?;
    let error = session
        .execute(Command::RemoveFeature {
            part,
            output: b,
            dependents: false,
        })
        .err()
        .ok_or("Exclusão com dependentes aceita")?;
    assert!(error.to_string().contains("Repetição"));
    session.execute(Command::RemoveFeature {
        part,
        output: b,
        dependents: true,
    })?;
    let piece = session.document().part(part).ok_or("Peça ausente")?;
    assert_eq!(piece.features.len(), 1);
    assert_eq!(piece.history_marker, None);
    assert!(session.document().body(b).is_none() && session.document().body(c).is_none());
    assert!(visible(&session, a)?);
    Ok(())
}

#[test]
fn failed_rebuild_names_the_operation_and_a_batch_resolves_it() -> Result<()> {
    let Chain {
        mut session,
        part,
        lines,
        a,
        ..
    } = chain()?;
    session.execute(Command::SetParameter {
        part,
        name: "raio".into(),
        expression: Some("2 mm".into()),
        dimension: Dimension::Length,
    })?;
    let owner = body_owner(a);
    let rounded = feature(
        &mut session,
        part,
        "Arredondamento",
        Operation::Fillet {
            base: a,
            edge: EdgeRef::Between([format!("{owner}:e{}", lines[0]), format!("{owner}:end")]),
            radius: "raio".into(),
            additional_edges: Vec::new(),
        },
    )?;
    let moved = feature(&mut session, part, "Afastamento", translate(rounded, 200.))?;
    let too_large = Command::SetParameter {
        part,
        name: "raio".into(),
        expression: Some("12 mm".into()),
        dimension: Dimension::Length,
    };
    let revision = session.revision();
    let error = session
        .execute(too_large.clone())
        .err()
        .ok_or("Raio maior que a espessura foi aceito")?;
    let failure = error.failure.ok_or("Falha sem operação identificada")?;
    assert_eq!(failure.output, rounded);
    assert_eq!(failure.name, "Arredondamento");
    assert_eq!(failure.dependents, [moved]);
    assert_eq!(session.revision(), revision);
    // Suprimir a operação que falhou junto com a edição aplica as duas de uma vez.
    session.execute(Command::Batch {
        label: "Alterar parâmetro e suprimir falha",
        commands: vec![
            too_large.clone(),
            Command::SetFeatureSuppressed {
                part,
                output: rounded,
                suppressed: true,
            },
        ],
    })?;
    let piece = session.document().part(part).ok_or("Peça ausente")?;
    assert_eq!(
        piece
            .parameters
            .entries
            .get("raio")
            .map(|p| p.expression.as_str()),
        Some("12 mm")
    );
    assert_eq!(
        &part_states(&session, part)?[3..],
        [FeatureState::Suppressed, FeatureState::Dependent]
    );
    // Desfazer remove o lote inteiro; corrigir a referência junto da edição também resolve.
    assert!(session.undo());
    session.execute(Command::Batch {
        label: "Alterar parâmetro e corrigir operação",
        commands: vec![
            too_large,
            Command::EditFeature {
                part,
                output: rounded,
                operation: Operation::Fillet {
                    base: a,
                    edge: EdgeRef::Between([
                        format!("{owner}:e{}", lines[0]),
                        format!("{owner}:end"),
                    ]),
                    radius: "3 mm".into(),
                    additional_edges: Vec::new(),
                },
            },
        ],
    })?;
    assert!(part_states(&session, part)?.iter().all(|s| s.is_active()));
    Ok(())
}
