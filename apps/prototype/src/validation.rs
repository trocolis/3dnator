//! Casos analíticos pequenos, executados somente com o núcleo CAD real.
use std::{error::Error, path::Path};

#[cfg(not(feature = "occt"))]
pub fn run(_directory: &Path) -> Result<(), Box<dyn Error>> {
    nator_kernel::require_available()?;
    Err("Validação OCCT indisponível nesta compilação.".into())
}

#[cfg(feature = "occt")]
pub fn run(directory: &Path) -> Result<(), Box<dyn Error>> {
    use nator_kernel::Solid;
    use serde_json::{Value, json};
    use std::{
        fs,
        time::{Instant, SystemTime, UNIX_EPOCH},
    };

    fn check_close(actual: f64, expected: f64, tolerance: f64) -> Result<(), Box<dyn Error>> {
        if !actual.is_finite() || (actual - expected).abs() > tolerance {
            return Err(
                format!("Valor inesperado: {actual}; esperado {expected} ± {tolerance}.").into(),
            );
        }
        Ok(())
    }

    fn memory() -> Value {
        // O mesmo relatório roda em Windows; a coleta de memória desse sistema fica pendente.
        #[cfg(target_os = "linux")]
        {
            let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
            let field = |name: &str| {
                status
                    .lines()
                    .find(|line| line.starts_with(name))
                    .and_then(|line| line.split_whitespace().nth(1))
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(|kb| kb * 1024)
            };
            json!({"rss_bytes":field("VmRSS:"),"peak_rss_bytes":field("VmHWM:")})
        }
        #[cfg(not(target_os = "linux"))]
        {
            json!({"rss_bytes":null,"peak_rss_bytes":null})
        }
    }

    fn measured<T>(
        steps: &mut Vec<Value>,
        name: &str,
        task: impl FnOnce() -> Result<T, Box<dyn Error>>,
    ) -> Result<T, Box<dyn Error>> {
        let start = Instant::now();
        match task() {
            Ok(value) => {
                steps.push(json!({"name":name,"status":"passed","elapsed_ms":start.elapsed().as_secs_f64()*1000.,"memory":memory()}));
                Ok(value)
            }
            Err(error) => {
                steps.push(json!({"name":name,"status":"failed","elapsed_ms":start.elapsed().as_secs_f64()*1000.,"error":error.to_string(),"memory":memory()}));
                Err(error)
            }
        }
    }

    // Um diretório novo impede misturar evidências de execuções ou sobrescrever arquivos.
    if let Some(parent) = directory.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(directory)?;
    let mut steps = Vec::new();
    let start = Instant::now();
    let result = (|| -> Result<(), Box<dyn Error>> {
        let block = measured(&mut steps, "cuboid_volume", || {
            let body = Solid::cuboid([0.; 3], [60., 20., 30.])?;
            check_close(body.volume()?, 36_000., 1e-7)?;
            Ok(body)
        })?;
        let bracket = measured(&mut steps, "cut_bracket", || {
            let tool = Solid::cuboid([6., 0., 6.], [54., 20., 24.])?;
            let body = block.cut(&tool)?;
            check_close(body.volume()?, 10_080., 1e-6)?;
            check_close(block.volume()?, 36_000., 1e-7)?;
            Ok(body)
        })?;
        let mut drilled = measured(&mut steps, "cylindrical_cut", || {
            let tool = Solid::cylinder([30., 10., -1.], 3., 8.)?;
            let body = bracket.cut(&tool)?;
            check_close(
                body.volume()?,
                10_080. - std::f64::consts::PI * 9. * 6.,
                1e-5,
            )?;
            Ok(body)
        })?;
        measured(&mut steps, "fillet", || {
            let original = Solid::cuboid([0.; 3], [20., 30., 40.])?;
            let mut body = original.fillet(0, 2.)?;
            if body.volume()? >= original.volume()? {
                return Err("O arredondamento não reduziu o volume do bloco.".into());
            }
            nator_project_io::write_stl(
                &directory.join("fillet.stl"),
                &body.tessellate(0.05, 0.25)?,
            )?;
            Ok(())
        })?;
        measured(&mut steps, "chamfer", || {
            let original = Solid::cuboid([0.; 3], [20., 30., 40.])?;
            let mut body = original.chamfer(0, 2.)?;
            if body.volume()? >= original.volume()? {
                return Err("O chanfro não reduziu o volume do bloco.".into());
            }
            nator_project_io::write_stl(
                &directory.join("chamfer.stl"),
                &body.tessellate(0.05, 0.25)?,
            )?;
            Ok(())
        })?;
        measured(&mut steps, "invalid_operation_preserves_source", || {
            let body = Solid::cuboid([0.; 3], [20., 30., 40.])?;
            if body.fillet(usize::MAX, 2.).is_ok() {
                return Err("Uma aresta inválida foi aceita.".into());
            }
            if Solid::cuboid([0.; 3], [-1., 1., 1.]).is_ok() {
                return Err("Dimensão negativa foi aceita.".into());
            }
            check_close(body.volume()?, 24_000., 1e-7)?;
            Ok(())
        })?;
        let mesh = measured(&mut steps, "tessellation_and_bounds", || {
            let mesh = drilled.tessellate(0.05, 0.25)?;
            for (actual, expected) in mesh.bounds().size().into_iter().zip([60., 20., 30.]) {
                check_close(actual, expected, 1e-6)?;
            }
            Ok(mesh)
        })?;
        measured(&mut steps, "step_round_trip", || {
            let path = directory.join("bracket.step");
            drilled.write_step(&path)?;
            let mut imported = Solid::read_step(&path)?;
            check_close(imported.volume()?, drilled.volume()?, 1e-5)?;
            let imported_mesh = imported.tessellate(0.05, 0.25)?;
            for (actual, expected) in imported_mesh
                .bounds()
                .size()
                .into_iter()
                .zip(mesh.bounds().size())
            {
                check_close(actual, expected, 1e-5)?;
            }
            Ok(())
        })?;
        measured(&mut steps, "stl_round_trip", || {
            let path = directory.join("bracket.stl");
            nator_project_io::write_stl(&path, &mesh)?;
            let imported = nator_project_io::read_stl(&path, 1.)?;
            if !imported.topology().has_closed_oriented_edges() {
                return Err("STL reimportado possui arestas abertas ou inconsistentes.".into());
            }
            check_close(
                imported.signed_volume(),
                drilled.volume()?,
                drilled.volume()? * 0.005,
            )?;
            for (actual, expected) in imported
                .bounds()
                .size()
                .into_iter()
                .zip(mesh.bounds().size())
            {
                check_close(actual, expected, 1e-4)?;
            }
            Ok(())
        })?;
        Ok(())
    })();
    let report = json!({
        "prototype_version":env!("CARGO_PKG_VERSION"),"occt_version":nator_kernel::version(),
        "os":std::env::consts::OS,"architecture":std::env::consts::ARCH,
        "unix_timestamp":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        "status":if result.is_ok() {"passed"} else {"failed"},
        "elapsed_ms":start.elapsed().as_secs_f64()*1000.,"steps":steps,
        "linear_deflection_mm":0.05,"angular_deflection_rad":0.25,
        "limitations":["STEP colors and assembly hierarchy not tested","mesh self-intersections not tested","not a benchmark on the 8 GB reference computer"],
        "error":result.as_ref().err().map(ToString::to_string),
    });
    fs::write(
        directory.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("Relatório: {}", directory.join("report.json").display());
    result
}
