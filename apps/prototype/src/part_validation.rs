//! Critério de entrega da etapa 2 do ROADMAP, executado com o núcleo real:
//! desenhar o suporte em L com medidas, extrudar, furar e arredondar, alterar uma cota com
//! recálculo, salvar, reabrir e exportar para STEP e STL, verificando dimensões e integridade.
//!
//! Geometria (mm): rascunho no plano XZ, extrudado ao longo de -Y. Base de 60 × `largura`,
//! parede de 30 de altura, espessura `espessura`. Furo vertical Ø6 na base em (40, -largura/2)
//! e arredondamento interno R3. Volume analítico:
//! V = (60·t + t·(30 − t))·w + (1 − π/4)·3²·w − π·3²·t.
use std::{error::Error, path::Path};

#[cfg(not(feature = "occt"))]
pub fn run(_directory: &Path) -> Result<(), Box<dyn Error>> {
    nator_kernel::require_available()?;
    Err("Validação da peça indisponível nesta compilação.".into())
}

#[cfg(feature = "occt")]
pub fn run(directory: &Path) -> Result<(), Box<dyn Error>> {
    use nator_commands::{Command, Effect, Session};
    use nator_document::{
        BodyId, Document, PartId,
        features::{BooleanMode, EdgeRef, Operation, RegionRef, body_owner, signature},
        parameters::Dimension,
    };
    use nator_kernel::Solid;
    use nator_sketch::{ConstraintKind, Curve, Plane, Sketch};
    use serde_json::{Value, json};
    use std::{f64::consts::PI, time::Instant};

    fn expected_volume(t: f64, w: f64) -> f64 {
        (60. * t + t * (30. - t)) * w + (1. - PI / 4.) * 9. * w - PI * 9. * t
    }
    fn check_close(actual: f64, expected: f64, tolerance: f64) -> Result<(), Box<dyn Error>> {
        if !actual.is_finite() || (actual - expected).abs() > tolerance {
            return Err(format!("Valor {actual}; esperado {expected} ± {tolerance}.").into());
        }
        Ok(())
    }
    fn body_solid(session: &Session, id: BodyId) -> Result<Solid, Box<dyn Error>> {
        let body = session.document().body(id).ok_or("Corpo ausente")?;
        let bytes = body.brep.as_ref().ok_or("Corpo sem B-rep")?;
        Ok(Solid::from_brep(bytes, Some(&body.face_names))?.transformed(body.placement)?)
    }
    fn created(effect: Effect) -> Result<BodyId, Box<dyn Error>> {
        match effect {
            Effect::Imported { body, .. } => Ok(body),
            _ => Err("A operação não criou um corpo.".into()),
        }
    }
    fn parameter(
        session: &mut Session,
        part: PartId,
        name: &str,
        value: &str,
    ) -> Result<(), Box<dyn Error>> {
        session.execute(Command::SetParameter {
            part,
            name: name.into(),
            expression: Some(value.into()),
            dimension: Dimension::Length,
        })?;
        Ok(())
    }

    /// Perfil totalmente restrito, com as duas espessuras ligadas ao parâmetro `espessura`.
    fn profile(t: f64) -> Result<(Sketch, [u64; 2]), Box<dyn Error>> {
        let mut sketch = Sketch::new("Perfil do suporte", Plane::xz());
        let p = [[0., 0.], [60., 0.], [60., t], [t, t], [t, 30.], [0., 30.]]
            .map(|q| sketch.add_point(q))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
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

    if let Some(parent) = directory.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    // Um diretório novo impede misturar evidências de execuções diferentes.
    std::fs::create_dir(directory)?;
    let start = Instant::now();
    let mut steps: Vec<Value> = Vec::new();
    let mut step = |name: &str, since: Instant| {
        steps.push(json!({"name": name, "elapsed_ms": since.elapsed().as_secs_f64() * 1000.}));
    };

    // 1. Rascunho cotado e parâmetros.
    let t0 = Instant::now();
    let mut session = Session::new(Document::new("Suporte em L")?)?;
    let (sketch, inner) = profile(6.)?;
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
    parameter(&mut session, part, "espessura", "6 mm")?;
    parameter(&mut session, part, "largura", "20 mm")?;
    session.execute(Command::UpdateSketch { id, sketch })?;
    step("dimensioned sketch bound to parameters", t0);

    // 2. Extrusão, furo e arredondamento escolhido por índice, como faz a interface.
    let t0 = Instant::now();
    let extrusion = created(session.execute(Command::AddFeature {
        part,
        name: "Extrusão".into(),
        operation: Operation::Extrude {
            sketch: id,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: "largura".into(),
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
        },
    })?)?;
    check_close(
        body_solid(&session, fillet)?.volume()?,
        expected_volume(6., 20.),
        1e-3,
    )?;
    let stable = session
        .document()
        .part(part)
        .ok_or("Peça ausente")?
        .features
        .iter()
        .all(|f| match &f.operation {
            Operation::Extrude { region, .. } => matches!(region, RegionRef::Boundary(_)),
            Operation::Fillet { edge, .. } => matches!(edge, EdgeRef::Between(_)),
            _ => true,
        });
    if !stable {
        return Err("Seleções por índice não viraram referências estáveis.".into());
    }
    step("extrude, drill and fillet with stable references", t0);

    // 3. Alterar cotas recalcula todo o histórico, inclusive o arredondamento.
    let t0 = Instant::now();
    parameter(&mut session, part, "espessura", "8 mm")?;
    parameter(&mut session, part, "largura", "25 mm")?;
    let rebuilt = body_solid(&session, fillet)?.volume()?;
    check_close(rebuilt, expected_volume(8., 25.), 1e-3)?;
    if !session.undo() {
        return Err("Desfazer indisponível.".into());
    }
    check_close(
        body_solid(&session, fillet)?.volume()?,
        expected_volume(8., 20.),
        1e-3,
    )?;
    if !session.redo() {
        return Err("Refazer indisponível.".into());
    }
    step("parameter change rebuilds history; undo/redo", t0);

    // 4. Salvar, reabrir e recalcular a partir do arquivo.
    let t0 = Instant::now();
    let project = directory.join("bracket.3dnator");
    nator_project_io::write_project(&project, session.document())?;
    let mut reopened = Session::new(nator_project_io::read_project(&project)?)?;
    check_close(
        body_solid(&reopened, fillet)?.volume()?,
        expected_volume(8., 25.),
        1e-3,
    )?;
    parameter(&mut reopened, part, "espessura", "7 mm")?;
    check_close(
        body_solid(&reopened, fillet)?.volume()?,
        expected_volume(7., 25.),
        1e-3,
    )?;
    step("save, reopen and rebuild from file", t0);

    // 5. Exportar STEP e STL e conferir a ida e volta.
    let t0 = Instant::now();
    let mut final_solid = body_solid(&reopened, fillet)?;
    let volume = final_solid.volume()?;
    let step_path = directory.join("bracket.step");
    nator_project_io::export_step(&step_path, |temp| {
        final_solid.write_step(temp).map_err(|e| e.to_string())
    })?;
    let mut imported = Solid::read_step(&step_path)?;
    check_close(imported.volume()?, volume, 1e-4)?;
    let step_mesh = imported.tessellate(0.05, 0.25)?;
    for (actual, expected) in step_mesh.bounds().size().into_iter().zip([60., 25., 30.]) {
        check_close(actual, expected, 1e-5)?;
    }
    // A malha de exportação vem da geometria precisa, não da malha de exibição.
    let mesh = final_solid.tessellate(0.01, 0.1)?;
    let stl_path = directory.join("bracket.stl");
    nator_project_io::write_stl(&stl_path, &mesh)?;
    let stl = nator_project_io::read_stl(&stl_path, 1.)?;
    let topology = stl.topology();
    if !topology.has_closed_oriented_edges() {
        return Err(format!("STL com defeitos de topologia: {topology:?}").into());
    }
    check_close(stl.signed_volume(), volume, volume * 0.002)?;
    for (actual, expected) in stl.bounds().size().into_iter().zip([60., 25., 30.]) {
        check_close(actual, expected, 1e-4)?;
    }
    step("STEP and STL export round trip", t0);

    let report = json!({
        "status": "passed",
        "app_version": env!("CARGO_PKG_VERSION"),
        "occt_version": nator_kernel::version(),
        "format_version": nator_project_io::FORMAT_VERSION,
        "os": std::env::consts::OS,
        "final_parameters": {"espessura_mm": 7., "largura_mm": 25.},
        "final_volume_mm3": volume,
        "expected_volume_mm3": expected_volume(7., 25.),
        "stl_triangles": stl.triangles().len(),
        "stl_volume_mm3": stl.signed_volume(),
        "elapsed_ms": start.elapsed().as_secs_f64() * 1000.,
        "steps": steps,
        "files": ["bracket.3dnator", "bracket.step", "bracket.stl"],
        "limitations": [
            "STEP colors and assembly structure not exported",
            "mesh self-intersections not tested"
        ],
    });
    std::fs::write(
        directory.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
