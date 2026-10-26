//! Fluxo paramétrico completo com o núcleo real: rascunho cotado por parâmetro, extrusão,
//! arredondamento, mudança de cota, salvamento e reabertura. Executado com `--features occt`.
#![cfg(feature = "occt")]

use nator_commands::{Command, Effect, Session};
use nator_document::{
    BodyId, Document, PartId, SketchId,
    features::{EdgeRef, Operation, RegionRef, body_owner, signature},
    parameters::Dimension,
};
use nator_kernel::Solid;
use nator_sketch::{ConstraintKind, Curve, Plane, Sketch};
use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Suporte em L no plano XZ, totalmente restrito: base 60 mm, altura 30 mm e as duas
/// espessuras cotadas. Retorna o rascunho, as cotas de espessura e as linhas do canto interno.
struct Bracket {
    sketch: Sketch,
    thickness: [u64; 2],
    inner: [u64; 2],
}

fn bracket(t: f64) -> Result<Bracket> {
    let mut sketch = Sketch::new("Perfil do suporte", Plane::xz());
    let p = [[0., 0.], [60., 0.], [60., t], [t, t], [t, 30.], [0., 30.]]
        .map(|q| sketch.add_point(q))
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut lines = Vec::new();
    for i in 0..6 {
        lines.push(sketch.add_curve(
            Curve::Line {
                start: p[i],
                end: p[(i + 1) % 6],
            },
            false,
        )?);
    }
    sketch.add_constraint(
        ConstraintKind::Fixed {
            point: p[0],
            position: [0., 0.],
        },
        false,
    )?;
    for (i, line) in lines.iter().enumerate() {
        let kind = if i % 2 == 0 {
            ConstraintKind::Horizontal { line: *line }
        } else {
            ConstraintKind::Vertical { line: *line }
        };
        sketch.add_constraint(kind, false)?;
    }
    for (a, b, value, horizontal) in [(p[0], p[1], 60., true), (p[0], p[5], 30., false)] {
        sketch.add_constraint(
            if horizontal {
                ConstraintKind::DeltaX { a, b, value }
            } else {
                ConstraintKind::DeltaY { a, b, value }
            },
            false,
        )?;
    }
    let base = sketch.add_constraint(
        ConstraintKind::DeltaY {
            a: p[1],
            b: p[2],
            value: t,
        },
        false,
    )?;
    let wall = sketch.add_constraint(
        ConstraintKind::DeltaX {
            a: p[5],
            b: p[4],
            value: t,
        },
        false,
    )?;
    Ok(Bracket {
        sketch,
        thickness: [base, wall],
        inner: [lines[2], lines[3]],
    })
}

fn solid(session: &Session, body: BodyId) -> Result<Solid> {
    let body = session.document().body(body).ok_or("Corpo ausente")?;
    let bytes = body.brep.as_ref().ok_or("Corpo sem B-rep")?;
    Ok(Solid::from_brep(bytes, Some(&body.face_names))?)
}

fn imported(effect: Effect) -> Result<BodyId> {
    match effect {
        Effect::Imported { body, .. } => Ok(body),
        _ => Err("A operação não criou um corpo".into()),
    }
}

fn set_thickness(session: &mut Session, part: PartId, value: &str) -> Result<()> {
    session.execute(Command::SetParameter {
        part,
        name: "espessura".into(),
        expression: Some(value.into()),
        dimension: Dimension::Length,
    })?;
    Ok(())
}

/// Monta a peça até o arredondamento. A aresta é escolhida por índice, como na interface.
fn build() -> Result<(Session, PartId, SketchId, BodyId, BodyId, [u64; 2])> {
    let Bracket {
        sketch,
        thickness,
        inner,
    } = bracket(6.)?;
    let mut session = Session::new(Document::new("Suporte paramétrico")?)?;
    let Effect::SketchCreated { part, sketch: id } = session.execute(Command::AddSketch {
        destination: None,
        sketch: sketch.clone(),
    })?
    else {
        return Err("Rascunho não criado".into());
    };
    set_thickness(&mut session, part, "6 mm")?;
    let mut bound = sketch;
    for constraint in thickness {
        bound
            .dimension_expressions
            .insert(constraint, "espessura".into());
    }
    session.execute(Command::UpdateSketch { id, sketch: bound })?;
    let extrusion = imported(session.execute(Command::AddFeature {
        part,
        name: "Extrusão".into(),
        operation: Operation::Extrude {
            sketch: id,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "20 mm".into(),
        },
    })?)?;
    let owner = body_owner(extrusion);
    let pair = [
        format!("{owner}:e{}", inner[0]),
        format!("{owner}:e{}", inner[1]),
    ];
    let index = solid(&session, extrusion)?.edge_between(&pair)?;
    let stamp = signature(
        session
            .document()
            .body(extrusion)
            .and_then(|b| b.brep.as_ref())
            .ok_or("B-rep ausente")?,
    );
    let fillet = imported(session.execute(Command::AddFeature {
        part,
        name: "Arredondamento".into(),
        operation: Operation::Fillet {
            base: extrusion,
            edge: EdgeRef::Index {
                index,
                signature: stamp,
            },
            radius: "3 mm".into(),
            additional_edges: Vec::new(),
        },
    })?)?;
    Ok((session, part, id, extrusion, fillet, inner))
}

/// Volume do suporte com espessura `t`, 20 mm de largura e arredondamento interno de 3 mm.
fn expected_volume(t: f64) -> f64 {
    let area = 60. * t + t * (30. - t);
    area * 20. + (1. - std::f64::consts::FRAC_PI_4) * 9. * 20.
}

#[test]
fn index_selections_become_stable_references() -> Result<()> {
    let (session, part, _, extrusion, _, inner) = build()?;
    let features = &session
        .document()
        .part(part)
        .ok_or("Peça ausente")?
        .features;
    assert!(matches!(
        &features[0].operation,
        Operation::Extrude {
            region: RegionRef::Boundary(_),
            ..
        }
    ));
    let owner = body_owner(extrusion);
    let Operation::Fillet { edge, .. } = &features[1].operation else {
        return Err("Segunda operação deveria ser o arredondamento".into());
    };
    let EdgeRef::Between(faces) = edge else {
        return Err(format!("Referência não estabilizada: {edge:?}").into());
    };
    let mut faces = faces.clone();
    faces.sort();
    let mut expected = [
        format!("{owner}:e{}", inner[0]),
        format!("{owner}:e{}", inner[1]),
    ];
    expected.sort();
    assert_eq!(faces, expected);
    Ok(())
}

#[test]
fn changing_the_thickness_rebuilds_the_fillet_on_the_same_edge() -> Result<()> {
    let (mut session, part, _, _, fillet, _) = build()?;
    assert!((solid(&session, fillet)?.volume()? - expected_volume(6.)).abs() < 1e-3);
    set_thickness(&mut session, part, "9 mm")?;
    assert!((solid(&session, fillet)?.volume()? - expected_volume(9.)).abs() < 1e-3);
    // Desfazer devolve a geometria anterior sem recalcular.
    assert!(session.undo());
    assert!((solid(&session, fillet)?.volume()? - expected_volume(6.)).abs() < 1e-3);
    Ok(())
}

#[test]
fn saved_project_keeps_references_for_later_edits() -> Result<()> {
    let (session, part, _, _, fillet, _) = build()?;
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("suporte.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let mut reopened = Session::new(nator_project_io::read_project(&path)?)?;
    set_thickness(&mut reopened, part, "8 mm")?;
    assert!((solid(&reopened, fillet)?.volume()? - expected_volume(8.)).abs() < 1e-3);
    Ok(())
}

#[test]
fn a_removed_corner_is_reported_instead_of_filleting_another_edge() -> Result<()> {
    let (mut session, _, id, _, _, inner) = build()?;
    let revision = session.revision();
    // Substituir o canto interno por uma diagonal elimina as faces referenciadas.
    let mut sketch = session
        .document()
        .sketch(id)
        .ok_or("Rascunho ausente")?
        .sketch
        .clone();
    let corner = sketch
        .entities
        .iter()
        .find(|e| e.id == inner[0])
        .and_then(|e| match e.curve {
            Curve::Line { end, .. } => Some(end),
            _ => None,
        })
        .ok_or("Canto ausente")?;
    let [start, end] = [inner[0], inner[1]].map(|line| {
        sketch
            .entities
            .iter()
            .find(|e| e.id == line)
            .and_then(|e| match e.curve {
                Curve::Line { start, end } => Some((start, end)),
                _ => None,
            })
    });
    let (Some((a, _)), Some((_, b))) = (start, end) else {
        return Err("Linhas do canto ausentes".into());
    };
    sketch.entities.retain(|e| !inner.contains(&e.id));
    sketch.constraints.retain(|c| match c.kind {
        ConstraintKind::Horizontal { line } | ConstraintKind::Vertical { line } => {
            !inner.contains(&line)
        }
        ConstraintKind::DeltaY { b, .. } | ConstraintKind::DeltaX { b, .. } => b != corner,
        _ => true,
    });
    sketch.dimension_expressions.clear();
    sketch.add_curve(Curve::Line { start: a, end: b }, false)?;
    sketch.prune_unused_points();
    let error = session
        .execute(Command::UpdateSketch { id, sketch })
        .err()
        .ok_or("O recálculo deveria falhar com referência perdida")?;
    assert!(error.to_string().contains("Referência perdida"), "{error}");
    // A falha preserva o documento: nenhuma revisão foi publicada.
    assert_eq!(session.revision(), revision);
    Ok(())
}

/// Um clique perto do canto interno, sobre a face da base, escolhe a aresta do canto pelos
/// nomes das faces, sem índice topológico, e o arredondamento resultante é o esperado.
#[test]
fn clicking_near_the_inner_corner_selects_its_edge() -> Result<()> {
    use nator_document::assembly::Pick;
    let (mut session, part, _, extrusion, _, inner) = build()?;
    let owner = body_owner(extrusion);
    let body = session.document().body(extrusion).ok_or("Corpo ausente")?;
    let face = body
        .face_names
        .iter()
        .position(|n| *n == format!("{owner}:e{}", inner[0]))
        .ok_or("Face da base ausente")? as u32;
    let triangle = body
        .geometry
        .triangle_faces()
        .ok_or("Malha sem faces de origem")?
        .iter()
        .position(|f| *f == face)
        .ok_or("Triângulo da face ausente")?;
    // Base com espessura 6: a face superior interna fica em z = 6 e o canto em x = 6.
    let pick = Pick {
        instance: session.document().instances()[0].id,
        part,
        body: extrusion,
        triangle,
        point: [6.2, -10., 6.],
        normal: [0., 0., 1.],
        vertices: [[0.; 3]; 3],
        placement: nator_geometry::Placement::default(),
    };
    let edge = session.document().edge_at(&pick)?;
    let mut expected = [
        format!("{owner}:e{}", inner[0]),
        format!("{owner}:e{}", inner[1]),
    ];
    let EdgeRef::Between(mut faces) = edge.clone() else {
        return Err("Seleção gráfica deveria produzir uma referência por nomes".into());
    };
    faces.sort();
    expected.sort();
    assert_eq!(faces, expected);
    let rounded = match session.execute(Command::AddFeature {
        part,
        name: "Arredondamento clicado".into(),
        operation: Operation::Fillet {
            base: extrusion,
            edge,
            radius: "3 mm".into(),
            additional_edges: Vec::new(),
        },
    })? {
        Effect::Imported { body, .. } => body,
        _ => return Err("Arredondamento não criado".into()),
    };
    // Sem o furo: só o arredondamento acrescenta material ao perfil extrudado.
    let profile = (60. * 6. + 6. * 24.) * 20.;
    let fillet = (1. - std::f64::consts::FRAC_PI_4) * 9. * 20.;
    assert!((solid(&session, rounded)?.volume()? - (profile + fillet)).abs() < 1e-3);
    Ok(())
}
