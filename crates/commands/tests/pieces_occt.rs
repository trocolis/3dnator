//! Um corpo é sempre um objeto só: partes desconectadas viram corpos próprios, que
//! acompanham o recálculo pela identidade, não pela posição na lista.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session};
use nator_document::{
    BodyId, Document, PartId,
    features::{Operation, RegionRef},
};
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn solid(session: &Session, id: BodyId) -> Result<Solid> {
    let b = session.document().body(id).ok_or("Corpo")?;
    Ok(Solid::from_brep(
        b.brep.as_ref().ok_or("B-rep")?,
        Some(&b.face_names),
    )?)
}

/// Partes geradas a partir de `base`, na ordem do histórico.
fn pieces(session: &Session, part: PartId, base: BodyId) -> Result<Vec<BodyId>> {
    Ok(session
        .document()
        .part(part)
        .ok_or("Peça")?
        .features
        .iter()
        .filter(|f| matches!(f.operation, Operation::Piece { base: b, .. } if b == base))
        .map(|f| f.output)
        .collect())
}

/// Perfil em U: dois braços de 10 × 15 mm sobre uma base de 30 × 5 mm.
fn u_shape() -> Result<Sketch> {
    let mut sketch = Sketch::new("U", Plane::xy());
    let points = [
        [0., 0.],
        [30., 0.],
        [30., 20.],
        [20., 20.],
        [20., 5.],
        [10., 5.],
        [10., 20.],
        [0., 20.],
    ];
    for (i, a) in points.iter().enumerate() {
        sketch.add_line(*a, points[(i + 1) % points.len()], false)?;
    }
    Ok(sketch)
}

#[test]
fn split_through_both_arms_gives_one_body_per_piece_and_follows_the_plane() -> Result<()> {
    let mut doc = Document::new("Partes")?;
    let (part, sketch) = doc.add_sketch(None, u_shape()?)?;
    let mut session = Session::new(doc)?;
    let Effect::Imported { body: base, .. } = session.execute(Command::AddFeature {
        part,
        name: "U".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "10 mm".into(),
            combine: None,
            until: None,
        },
    })?
    else {
        return Err("Base".into());
    };
    assert!(pieces(&session, part, base)?.is_empty());
    let split = |y: f64| Operation::Split {
        base,
        origin: [0., y, 0.],
        normal: [0., 1., 0.],
        cutting_plane: None,
        reverse: false,
    };
    let Effect::Imported { body: selected, .. } = session.execute(Command::SplitBody {
        part,
        name: "Divisão".into(),
        operation: split(10.),
    })?
    else {
        return Err("Divisão".into());
    };
    let p = session.document().part(part).ok_or("Peça")?;
    let division = p
        .features
        .iter()
        .find(|f| matches!(f.operation, Operation::Split { .. }))
        .ok_or("Divisão")?
        .output;
    let remainder = p
        .features
        .iter()
        .find(|f| matches!(f.operation, Operation::SplitRemainder { .. }))
        .ok_or("Complemento")?
        .output;
    // O lado de cima tem os dois braços soltos: cada um vira um corpo.
    let arms = pieces(&session, part, division)?;
    assert_eq!(arms.len(), 2);
    assert_eq!(selected, arms[0]);
    assert!(pieces(&session, part, remainder)?.is_empty());
    let shown = p.shown_bodies().map(|b| b.id).collect::<Vec<_>>();
    assert!(!shown.contains(&division));
    assert!(shown.contains(&remainder));
    for arm in &arms {
        assert!(shown.contains(arm));
        let s = solid(&session, *arm)?;
        assert_eq!(s.piece_count()?, 1);
        assert!((s.volume()? - 1000.).abs() < 1e-5);
    }
    let left = solid(&session, arms[0])?.properties()?.center[0];
    let right = solid(&session, arms[1])?.properties()?.center[0];
    assert!((left - 5.).abs() < 1e-6 && (right - 25.).abs() < 1e-6);

    // Mudar o plano recalcula as partes e cada corpo continua sendo o mesmo braço.
    session.execute(Command::EditFeature {
        part,
        output: division,
        operation: split(12.),
    })?;
    assert_eq!(pieces(&session, part, division)?, arms);
    for (arm, x) in arms.iter().zip([5., 25.]) {
        let s = solid(&session, *arm)?;
        assert!((s.volume()? - 800.).abs() < 1e-5);
        assert!((s.properties()?.center[0] - x).abs() < 1e-6);
    }

    // Abaixo da base o lado de cima é uma peça só: a parte que sumiu falha de forma
    // explícita, e o documento anterior é preservado.
    assert!(
        session
            .execute(Command::EditFeature {
                part,
                output: division,
                operation: split(2.),
            })
            .is_err()
    );
    assert_eq!(pieces(&session, part, division)?, arms);

    // As partes e suas identidades sobrevivem a salvar e reabrir.
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("partes.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let mut reopened = Session::new(nator_project_io::read_project(&path)?)?;
    reopened.execute(Command::EditFeature {
        part,
        output: division,
        operation: split(15.),
    })?;
    assert_eq!(pieces(&reopened, part, division)?, arms);
    for (arm, x) in arms.iter().zip([5., 25.]) {
        let s = solid(&reopened, *arm)?;
        assert!((s.volume()? - 500.).abs() < 1e-5);
        assert!((s.properties()?.center[0] - x).abs() < 1e-6);
    }

    // Desfazer a divisão remove também as partes.
    assert!(session.undo());
    assert!(session.undo());
    assert!(
        session
            .document()
            .part(part)
            .ok_or("Peça")?
            .features
            .iter()
            .all(|f| !matches!(f.operation, Operation::Piece { .. }))
    );
    Ok(())
}

#[test]
fn extrusion_of_two_separate_areas_gives_two_bodies() -> Result<()> {
    let mut sketch = Sketch::new("Ilhas", Plane::xy());
    sketch.add_rectangle([0., 0.], [10., 10.], false)?;
    sketch.add_rectangle([20., 0.], [30., 10.], false)?;
    let mut doc = Document::new("Ilhas")?;
    let (part, sketch) = doc.add_sketch(None, sketch)?;
    let mut session = Session::new(doc)?;
    session.execute(Command::AddFeature {
        part,
        name: "Extrusão".into(),
        operation: Operation::Extrude {
            sketch,
            region: RegionRef::Multiple(vec![RegionRef::Index(0), RegionRef::Index(1)]),
            from: "0 mm".into(),
            to: "5 mm".into(),
            combine: None,
            until: None,
        },
    })?;
    let p = session.document().part(part).ok_or("Peça")?;
    let shown = p.shown_bodies().map(|b| b.id).collect::<Vec<_>>();
    assert_eq!(shown.len(), 2);
    for body in shown {
        let s = solid(&session, body)?;
        assert_eq!(s.piece_count()?, 1);
        assert!((s.volume()? - 500.).abs() < 1e-5);
    }
    Ok(())
}
