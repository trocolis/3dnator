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

/// Espelhar a instância de um sólido cria uma peça independente cujo histórico termina em
/// um espelhamento: ela continua paramétrica e não altera a original.
#[test]
fn mirrored_solid_instance_keeps_its_own_parametric_history() -> Result<()> {
    let mut session = Session::new(Document::new("Esquerda e direita")?)?;
    let (part, _) = plate(&mut session)?;
    let instance = session.document().instances()[0].id;
    session.execute(Command::MirrorInstance {
        id: instance,
        origin: [-10., 0., 0.],
        normal: [1., 0., 0.],
        name: "Placa espelhada".into(),
    })?;
    let doc = session.document();
    let mirrored = doc
        .parts()
        .iter()
        .find(|p| p.id != part)
        .ok_or("Peça espelhada ausente")?;
    let last = mirrored.features.last().ok_or("Histórico vazio")?;
    assert!(matches!(last.operation, Operation::Mirror { .. }));
    let image = last.output;
    let clone = mirrored.id;
    assert!((volume(&session, image)? - 60. * 40. * 8.).abs() < 1e-3);
    // A imagem fica em x ∈ [-80, -20]: reflexão de [0, 60] pelo plano x = -10.
    let bounds = session
        .document()
        .body(image)
        .ok_or("Corpo ausente")?
        .geometry
        .mesh()
        .bounds();
    assert!((bounds.min[0] + 80.).abs() < 1e-6 && (bounds.max[0] + 20.).abs() < 1e-6);
    // O clone tem parâmetros próprios: engrossá-lo não altera a placa original.
    session.execute(Command::SetParameter {
        part: clone,
        name: "espessura".into(),
        expression: Some("15 mm".into()),
        dimension: Dimension::Length,
    })?;
    assert!((volume(&session, image)? - 60. * 40. * 15.).abs() < 1e-3);
    let original = session.document().part(part).ok_or("Original ausente")?;
    let output = original.features[0].output;
    assert!((volume(&session, output)? - 60. * 40. * 8.).abs() < 1e-3);
    Ok(())
}

/// Medidas exatas no B-rep: propriedades de massa, raio de um furo e distância entre peças.
#[test]
fn exact_measurements_on_solids() -> Result<()> {
    use nator_commands::measure::{self, Topology};
    use nator_document::assembly::Pick;
    let mut session = Session::new(Document::new("Medição")?)?;
    let (part, body) = plate(&mut session)?;
    let properties = measure::body_properties(session.document(), body)?;
    assert!(properties.exact);
    assert!((properties.volume_mm3 - 60. * 40. * 8.).abs() < 1e-6);
    assert!((properties.area_mm2 - (2. * 2400. + 2. * 480. + 2. * 320.)).abs() < 1e-6);
    for (actual, expected) in properties.center_mm.iter().zip([30., 20., 4.]) {
        assert!((actual - expected).abs() < 1e-9);
    }
    // Furo passante Ø6 no centro: a face cilíndrica mede raio 3.
    let mut positions = Sketch::new("Furo", Plane::xy());
    let circle = positions.add_circle([30., 20.], 3., false)?;
    let center = match positions.entity(circle)?.curve {
        Curve::Circle { center, .. } => center,
        _ => return Err("Centro ausente".into()),
    };
    let (_, holes) = sketch(&mut session, Some(part), positions)?;
    let drilled = feature(
        &mut session,
        part,
        "Furo",
        Operation::Hole {
            base: body,
            sketch: holes,
            points: vec![center],
            spec: HoleSpec {
                diameter: "6 mm".into(),
                depth: None,
                head: HoleHeadSpec::Simple,
                tip_angle: None,
                reverse: true,
                thread: None,
            },
        },
    )?;
    let doc = session.document();
    let record = doc.body(drilled).ok_or("Corpo ausente")?;
    let wall = record
        .face_names
        .iter()
        .position(|n| n.ends_with(":hole0:wall"))
        .ok_or("Parede do furo ausente")? as u32;
    let triangle = record
        .geometry
        .triangle_faces()
        .ok_or("Malha sem faces")?
        .iter()
        .position(|f| *f == wall)
        .ok_or("Triângulo da parede ausente")?;
    let pick = |point: [f64; 3]| Pick {
        instance: doc.instances()[0].id,
        part,
        body: drilled,
        triangle,
        point,
        normal: [1., 0., 0.],
        vertices: [[0.; 3]; 3],
        placement: nator_geometry::Placement::default(),
    };
    match measure::topology(doc, &pick([33., 20., 4.]), false)? {
        Topology::Face {
            radius_mm,
            planar,
            area_mm2,
            ..
        } => {
            assert!(!planar);
            assert!((radius_mm.ok_or("Raio ausente")? - 3.).abs() < 1e-9);
            assert!((area_mm2 - 2. * PI * 3. * 8.).abs() < 1e-6);
        }
        other => return Err(format!("Medida inesperada: {other:?}").into()),
    }
    // A aresta circular da boca do furo, perto do topo (z = 8), tem raio 3.
    match measure::topology(doc, &pick([33., 20., 7.9]), true)? {
        Topology::Edge {
            radius_mm,
            length_mm,
            ..
        } => {
            assert!((radius_mm.ok_or("Raio ausente")? - 3.).abs() < 1e-9);
            assert!((length_mm - 2. * PI * 3.).abs() < 1e-6);
        }
        other => return Err(format!("Medida inesperada: {other:?}").into()),
    }
    // Duas instâncias da placa afastadas 100 mm em X ficam a 40 mm uma da outra.
    session.execute(Command::AddInstance {
        part,
        parent: None,
        placement: nator_geometry::Placement::new([100., 0., 0.], [0.; 3])?,
    })?;
    let doc = session.document();
    let at = |instance: usize| Pick {
        instance: doc.instances()[instance].id,
        part,
        body,
        triangle: 0,
        point: [0.; 3],
        normal: [0., 0., 1.],
        vertices: [[0.; 3]; 3],
        placement: doc.instances()[instance].placement,
    };
    assert!((measure::body_distance(doc, &at(0), &at(1))? - 40.).abs() < 1e-9);
    Ok(())
}
