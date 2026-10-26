//! Operações da etapa 3 pelo histórico do documento, com o núcleo real.
#![cfg(feature = "occt")]

use nator_commands::{Command, Effect, Session};
use nator_document::{
    BodyId, Document, PartId, SketchId,
    features::{HoleHeadSpec, HoleSpec, LinearAxis, Operation, PatternLayout, RegionRef},
    parameters::Dimension,
};
use nator_kernel::Solid;
use nator_sketch::{Curve, Plane, Sketch};
use std::{error::Error, f64::consts::PI};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn created(effect: Effect) -> Result<BodyId> {
    match effect {
        Effect::Imported { body, .. } => Ok(body),
        _ => Err("A operação não criou um corpo".into()),
    }
}

fn sketch(
    session: &mut Session,
    part: Option<PartId>,
    sketch: Sketch,
) -> Result<(PartId, SketchId)> {
    match session.execute(Command::AddSketch {
        destination: part,
        sketch,
    })? {
        Effect::SketchCreated { part, sketch } => Ok((part, sketch)),
        _ => Err("Rascunho não criado".into()),
    }
}

fn volume(session: &Session, body: BodyId) -> Result<f64> {
    let body = session.document().body(body).ok_or("Corpo ausente")?;
    let bytes = body.brep.as_ref().ok_or("Corpo sem B-rep")?;
    Ok(Solid::from_brep(bytes, Some(&body.face_names))?.volume()?)
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

/// Placa 60 × 40 com espessura `espessura`, extrudada de z = 0 para z = espessura.
fn plate(session: &mut Session) -> Result<(PartId, BodyId)> {
    let mut outline = Sketch::new("Placa", Plane::xy());
    outline.add_rectangle([0., 0.], [60., 40.], false)?;
    let (part, id) = sketch(session, None, outline)?;
    session.execute(Command::SetParameter {
        part,
        name: "espessura".into(),
        expression: Some("8 mm".into()),
        dimension: Dimension::Length,
    })?;
    let body = feature(
        session,
        part,
        "Placa",
        Operation::Extrude {
            sketch: id,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "espessura".into(),
        },
    )?;
    Ok((part, body))
}

#[test]
fn counterbored_holes_follow_sketch_points_and_parameters() -> Result<()> {
    let mut session = Session::new(Document::new("Placa furada")?)?;
    let (part, body) = plate(&mut session)?;
    // Rascunho dos furos na face inferior (z = 0), furando para cima (a favor da normal):
    // os furos passantes acompanham a espessura, com o rebaixo do lado de baixo.
    let mut positions = Sketch::new("Furos", Plane::xy());
    let a = positions.add_circle([15., 20.], 3., false)?;
    let b = positions.add_circle([45., 20.], 3., false)?;
    let centers = [a, b].map(|id| match positions.entity(id).map(|e| e.curve.clone()) {
        Ok(Curve::Circle { center, .. }) => Ok(center),
        _ => Err("Centro ausente"),
    });
    let points = centers
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let (_, holes_sketch) = sketch(&mut session, Some(part), positions)?;
    let drilled = feature(
        &mut session,
        part,
        "Furos",
        Operation::Hole {
            base: body,
            sketch: holes_sketch,
            points,
            spec: HoleSpec {
                diameter: "6.6 mm".into(),
                depth: None,
                head: HoleHeadSpec::Counterbore {
                    diameter: "11 mm".into(),
                    depth: "4 mm".into(),
                },
                tip_angle: None,
                reverse: true,
                thread: None,
            },
        },
    )?;
    let removed = |t: f64| 2. * (PI * 5.5 * 5.5 * 4. + PI * 3.3 * 3.3 * (t - 4.));
    assert!((volume(&session, drilled)? - (60. * 40. * 8. - removed(8.))).abs() < 1e-3);
    // Engrossar a placa recalcula os furos passantes pela nova espessura.
    session.execute(Command::SetParameter {
        part,
        name: "espessura".into(),
        expression: Some("12 mm".into()),
        dimension: Dimension::Length,
    })?;
    assert!((volume(&session, drilled)? - (60. * 40. * 12. - removed(12.))).abs() < 1e-3);
    let names = &session
        .document()
        .body(drilled)
        .ok_or("Corpo ausente")?
        .face_names;
    assert!(
        names.iter().any(|n| n.ends_with(":hole1:cbore_floor")),
        "{names:?}"
    );
    Ok(())
}

#[test]
fn pattern_mirror_split_and_revolve_about_a_sketch_line() -> Result<()> {
    let mut session = Session::new(Document::new("Padrões")?)?;
    let (part, body) = plate(&mut session)?;
    let plate_volume = 60. * 40. * 8.;
    let row = feature(
        &mut session,
        part,
        "Fileira",
        Operation::Pattern {
            base: body,
            layout: PatternLayout::Linear {
                first: LinearAxis {
                    direction: [1., 0., 0.],
                    count: 3,
                    spacing: "70 mm".into(),
                },
                second: None,
            },
        },
    )?;
    assert!((volume(&session, row)? - 3. * plate_volume).abs() < 1e-3);
    let symmetric = feature(
        &mut session,
        part,
        "Simetria",
        Operation::Mirror {
            base: row,
            origin: [-5., 0., 0.],
            normal: [1., 0., 0.],
            keep_original: true,
        },
    )?;
    assert!((volume(&session, symmetric)? - 6. * plate_volume).abs() < 1e-3);
    let half = feature(
        &mut session,
        part,
        "Metade",
        Operation::Split {
            base: body,
            origin: [0., 0., 2.],
            normal: [0., 0., 1.],
        },
    )?;
    assert!((volume(&session, half)? - 60. * 40. * 6.).abs() < 1e-3);
    // Mudar a espessura propaga para padrão, simetria e divisão.
    session.execute(Command::SetParameter {
        part,
        name: "espessura".into(),
        expression: Some("10 mm".into()),
        dimension: Dimension::Length,
    })?;
    assert!((volume(&session, symmetric)? - 6. * 60. * 40. * 10.).abs() < 1e-3);
    assert!((volume(&session, half)? - 60. * 40. * 8.).abs() < 1e-3);

    // Revolução em torno de uma linha de construção cotada: o eixo acompanha a cota.
    let mut profile = Sketch::new("Anel", Plane::xz());
    profile.add_rectangle([10., 0.], [15., 4.], false)?;
    let axis = profile.add_line([0., -5.], [0., 5.], true)?;
    let (_, ring_sketch) = sketch(&mut session, Some(part), profile)?;
    let ring = feature(
        &mut session,
        part,
        "Anel",
        Operation::Revolve {
            sketch: ring_sketch,
            region: RegionRef::Index(0),
            axis_origin: [0.; 3],
            axis_direction: [0., 0., 1.],
            angle: "360 deg".into(),
            axis_line: Some(axis),
        },
    )?;
    let expected = PI * (15. * 15. - 10. * 10.) * 4.;
    assert!((volume(&session, ring)? - expected).abs() < 1e-3);
    // Deslocar a linha do eixo 5 mm para a esquerda aumenta os raios do anel.
    let mut moved = session
        .document()
        .sketch(ring_sketch)
        .ok_or("Rascunho ausente")?
        .sketch
        .clone();
    if let Some(Curve::Line { start, end }) = moved
        .entities
        .iter()
        .find(|e| e.id == axis)
        .map(|e| e.curve.clone())
    {
        for point in moved
            .points
            .iter_mut()
            .filter(|p| p.id == start || p.id == end)
        {
            point.position[0] = -5.;
        }
    }
    session.execute(Command::UpdateSketch {
        id: ring_sketch,
        sketch: moved,
    })?;
    let expected = PI * (20. * 20. - 15. * 15.) * 4.;
    assert!((volume(&session, ring)? - expected).abs() < 1e-3);
    Ok(())
}
