//! Suporte em L paramétrico de referência, construído somente por comandos do documento,
//! como faria a interface. Usado pelo ensaio da etapa 2 (`--validate-part`) e pelas
//! medições de leveza (`--benchmark`).
//!
//! Geometria (mm): rascunho no plano XZ, extrudado ao longo de -Y. Base de 60 × `largura`,
//! parede de 30 de altura, espessura `espessura`. Furo vertical Ø6 na base em (40, -10)
//! e arredondamento interno R3. Volume analítico:
//! V = (60·t + t·(30 − t))·w + (1 − π/4)·3²·w − π·3²·t.
use nator_commands::{Command, Effect, Session};
use nator_document::{
    BodyId, Document, PartId,
    features::{BooleanMode, EdgeRef, Operation, RegionRef, body_owner, signature},
    parameters::Dimension,
};
use nator_kernel::Solid;
use nator_sketch::{ConstraintKind, Curve, Plane, Sketch};
use std::{error::Error, f64::consts::PI};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct ReferencePart {
    pub session: Session,
    pub part: PartId,
    /// Resultado final do histórico (arredondamento sobre o corpo furado).
    pub fillet: BodyId,
}

pub fn expected_volume(t: f64, w: f64) -> f64 {
    (60. * t + t * (30. - t)) * w + (1. - PI / 4.) * 9. * w - PI * 9. * t
}

/// Sólido de um corpo do documento, posicionado na peça e com seus nomes de faces.
pub fn body_solid(session: &Session, id: BodyId) -> Result<Solid> {
    let body = session.document().body(id).ok_or("Corpo ausente")?;
    let bytes = body.brep.as_ref().ok_or("Corpo sem B-rep")?;
    Ok(Solid::from_brep(bytes, Some(&body.face_names))?.transformed(body.placement)?)
}

pub fn set_length(session: &mut Session, part: PartId, name: &str, value: &str) -> Result<()> {
    session.execute(Command::SetParameter {
        part,
        name: name.into(),
        expression: Some(value.into()),
        dimension: Dimension::Length,
    })?;
    Ok(())
}

fn created(effect: Effect) -> Result<BodyId> {
    match effect {
        Effect::Imported { body, .. } => Ok(body),
        _ => Err("A operação não criou um corpo.".into()),
    }
}

/// Perfil totalmente restrito, com as duas espessuras ligadas ao parâmetro `espessura`.
/// Retorna também as linhas do canto interno, cuja aresta recebe o arredondamento.
fn profile(t: f64) -> Result<(Sketch, [u64; 2])> {
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
        sketch.add_constraint(
            if i % 2 == 0 {
                ConstraintKind::Horizontal { line: *line }
            } else {
                ConstraintKind::Vertical { line: *line }
            },
            false,
        )?;
    }
    sketch.add_constraint(
        ConstraintKind::DeltaX {
            a: p[0],
            b: p[1],
            value: 60.,
        },
        false,
    )?;
    sketch.add_constraint(
        ConstraintKind::DeltaY {
            a: p[0],
            b: p[5],
            value: 30.,
        },
        false,
    )?;
    for (a, b, horizontal) in [(p[1], p[2], false), (p[5], p[4], true)] {
        let id = sketch.add_constraint(
            if horizontal {
                ConstraintKind::DeltaX { a, b, value: t }
            } else {
                ConstraintKind::DeltaY { a, b, value: t }
            },
            false,
        )?;
        sketch.dimension_expressions.insert(id, "espessura".into());
    }
    Ok((sketch, [lines[2], lines[3]]))
}

/// Constrói o histórico com espessura 6 mm e largura 20 mm. As regiões e a aresta do
/// arredondamento são escolhidas por índice, como na interface, e o comando as estabiliza.
pub fn build() -> Result<ReferencePart> {
    let mut session = Session::new(Document::new("Suporte em L")?)?;
    let (sketch, inner) = profile(6.)?;
    // O rascunho entra sem expressões porque os parâmetros ainda não existem na peça.
    let Effect::SketchCreated { part, sketch: id } = session.execute(Command::AddSketch {
        destination: None,
        sketch: Sketch {
            dimension_expressions: Default::default(),
            ..sketch.clone()
        },
    })?
    else {
        return Err("Rascunho não criado.".into());
    };
    set_length(&mut session, part, "espessura", "6 mm")?;
    set_length(&mut session, part, "largura", "20 mm")?;
    session.execute(Command::UpdateSketch { id, sketch })?;
    let extrusion = created(session.execute(Command::AddFeature {
        part,
        name: "Extrusão".into(),
        operation: Operation::Extrude {
            sketch: id,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "largura".into(),
            combine: None,
            until: None,
        },
    })?)?;
    let mut hole = Sketch::new("Furo", Plane::xy());
    hole.add_circle([40., -10.], 3., false)?;
    let Effect::SketchCreated {
        sketch: hole_id, ..
    } = session.execute(Command::AddSketch {
        destination: Some(part),
        sketch: hole,
    })?
    else {
        return Err("Rascunho do furo não criado.".into());
    };
    let drill = created(session.execute(Command::AddFeature {
        part,
        name: "Ferramenta do furo".into(),
        operation: Operation::Extrude {
            sketch: hole_id,
            region: RegionRef::Index(0),
            from: "-1 mm".into(),
            to: "espessura + 1 mm".into(),
            combine: None,
            until: None,
        },
    })?)?;
    let drilled = created(session.execute(Command::AddFeature {
        part,
        name: "Furo".into(),
        operation: Operation::Boolean {
            base: extrusion,
            tool: drill,
            mode: BooleanMode::Cut,
        },
    })?)?;
    let owner = body_owner(extrusion);
    let pair = [
        format!("{owner}:e{}", inner[0]),
        format!("{owner}:e{}", inner[1]),
    ];
    let index = body_solid(&session, drilled)?.edge_between(&pair)?;
    let stamp = signature(
        session
            .document()
            .body(drilled)
            .and_then(|b| b.brep.as_ref())
            .ok_or("B-rep ausente")?,
    );
    let fillet = created(session.execute(Command::AddFeature {
        part,
        name: "Arredondamento".into(),
        operation: Operation::Fillet {
            base: drilled,
            edge: EdgeRef::Index {
                index,
                signature: stamp,
            },
            radius: "3 mm".into(),
            additional_edges: Vec::new(),
        },
    })?)?;
    Ok(ReferencePart {
        session,
        part,
        fillet,
    })
}
