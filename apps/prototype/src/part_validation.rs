//! Critério de entrega da etapa 2 do ROADMAP, executado com o núcleo real:
//! desenhar o suporte em L com medidas, extrudar, furar e arredondar, alterar uma cota com
//! recálculo, salvar, reabrir e exportar para STEP e STL, verificando dimensões e integridade.
//! A peça e seu volume analítico estão em `reference_part`.
use std::{error::Error, path::Path};

#[cfg(not(feature = "occt"))]
pub fn run(_directory: &Path) -> Result<(), Box<dyn Error>> {
    nator_kernel::require_available()?;
    Err("Validação da peça indisponível nesta compilação.".into())
}

#[cfg(feature = "occt")]
pub fn run(directory: &Path) -> Result<(), Box<dyn Error>> {
    use crate::reference_part::{self, ReferencePart, body_solid, expected_volume, set_length};
    use nator_commands::Session;
    use nator_document::features::{EdgeRef, Operation, RegionRef};
    use nator_kernel::Solid;
    use serde_json::{Value, json};
    use std::time::Instant;

    fn check_close(actual: f64, expected: f64, tolerance: f64) -> Result<(), Box<dyn Error>> {
        if !actual.is_finite() || (actual - expected).abs() > tolerance {
            return Err(format!("Valor {actual}; esperado {expected} ± {tolerance}.").into());
        }
        Ok(())
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

    // 1. Rascunho cotado por parâmetros, extrusão, furo e arredondamento.
    let t0 = Instant::now();
    let ReferencePart {
        mut session,
        part,
        fillet,
    } = reference_part::build()?;
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
    step(
        "dimensioned sketch, extrude, drill and fillet with stable references",
        t0,
    );

    // 2. Alterar cotas recalcula todo o histórico, inclusive o arredondamento.
    let t0 = Instant::now();
    set_length(&mut session, part, "espessura", "8 mm")?;
    set_length(&mut session, part, "largura", "25 mm")?;
    check_close(
        body_solid(&session, fillet)?.volume()?,
        expected_volume(8., 25.),
        1e-3,
    )?;
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

    // 3. Salvar, reabrir e recalcular a partir do arquivo.
    let t0 = Instant::now();
    let project = directory.join("bracket.3dnator");
    nator_project_io::write_project(&project, session.document())?;
    let mut reopened = Session::new(nator_project_io::read_project(&project)?)?;
    check_close(
        body_solid(&reopened, fillet)?.volume()?,
        expected_volume(8., 25.),
        1e-3,
    )?;
    set_length(&mut reopened, part, "espessura", "7 mm")?;
    check_close(
        body_solid(&reopened, fillet)?.volume()?,
        expected_volume(7., 25.),
        1e-3,
    )?;
    step("save, reopen and rebuild from file", t0);

    // 4. Exportar STEP e STL e conferir a ida e volta.
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
