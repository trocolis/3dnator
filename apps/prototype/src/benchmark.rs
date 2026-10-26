//! Medições de leveza da etapa 1 do ROADMAP: tempo de abertura, memória em repouso e
//! durante operações, resposta da navegação e recálculo, nos cenários de peça simples, malha
//! STL importada e instâncias repetidas.
//!
//! O orquestrador (`--benchmark`) executa cada cenário em um processo filho
//! (`--benchmark-scenario`), para que o pico de memória de um cenário não contamine o
//! seguinte. A GPU de baixo consumo (integrada) é preferida por padrão, por ser a referência
//! do ROADMAP; `NATOR_BENCH_GPU=high` escolhe a de alto desempenho.
//!
//! Os números dependem do perfil de compilação: medições de referência usam `--release`.
use nator_commands::{Command, Effect, Session};
use nator_document::{Document, MeshAsset};
use nator_geometry::{Mesh, Placement};
use nator_renderer::{PreparedMesh, Viewport, wgpu};
use serde_json::{Value, json};
use std::{error::Error, path::Path, sync::Arc, time::Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Cenários na ordem de execução.
const SCENARIOS: [&str; 4] = ["simple_part", "imported_mesh", "instances", "recalculation"];
/// Resolução da área 3D usada na medição de quadros.
const VIEWPORT: [u32; 2] = [1280, 800];
const WARMUP_FRAMES: usize = 5;
const MEASURED_FRAMES: usize = 120;

pub fn run(directory: &Path) -> Result<()> {
    if let Some(parent) = directory.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(directory)?;
    let executable = std::env::current_exe()?;
    let mut results = Vec::new();
    for name in SCENARIOS {
        let output = std::process::Command::new(&executable)
            .arg("--benchmark-scenario")
            .arg(name)
            .arg(directory)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "Cenário {name} falhou: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into());
        }
        results.push(serde_json::from_slice::<Value>(&output.stdout)?);
    }
    let report = json!({
        "app_version": env!("CARGO_PKG_VERSION"),
        "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "system": system(),
        "viewport_px": VIEWPORT,
        "measured_frames": MEASURED_FRAMES,
        "scenarios": results,
        "notes": [
            "Each scenario runs in its own process; peak RSS is per scenario.",
            "Frame time includes command encoding and waiting for GPU completion.",
            "On integrated GPUs, RSS includes buffers in shared memory."
        ],
    });
    std::fs::write(
        directory.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Executa um cenário e escreve seu resultado JSON no stdout.
pub fn scenario(name: &str, directory: &Path) -> Result<()> {
    let result = match name {
        "simple_part" => render_scenario(name, directory, simple_part()?)?,
        "imported_mesh" => imported_mesh(directory)?,
        "instances" => render_scenario(name, directory, instances(20)?)?,
        "recalculation" => recalculation()?,
        _ => return Err(format!("Cenário desconhecido: {name}").into()),
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

// ----- Cenários --------------------------------------------------------------------------

/// Malha da peça de referência: o suporte paramétrico do OCCT quando disponível.
fn part_mesh() -> Result<(Mesh, &'static str)> {
    #[cfg(feature = "occt")]
    {
        let part = crate::reference_part::build()?;
        let body = part
            .session
            .document()
            .body(part.fillet)
            .ok_or("Corpo ausente")?;
        Ok((
            body.geometry.mesh().clone(),
            "parametric bracket (OCCT, fillet and hole)",
        ))
    }
    #[cfg(not(feature = "occt"))]
    {
        Ok((
            nator_geometry::reference_bracket()?,
            "reference bracket mesh (no OCCT)",
        ))
    }
}

fn document_with(mesh: Mesh, name: &str) -> Result<(Session, nator_document::PartId)> {
    let mut session = Session::new(Document::new(name)?)?;
    let Effect::Imported { part, .. } = session.execute(Command::ImportMesh {
        destination: None,
        name: name.into(),
        geometry: Arc::new(MeshAsset::new(mesh)),
    })?
    else {
        return Err("Corpo não importado.".into());
    };
    Ok((session, part))
}

struct Scene {
    session: Session,
    source: &'static str,
    extra: Value,
}

fn simple_part() -> Result<Scene> {
    let (mesh, source) = part_mesh()?;
    let (session, _) = document_with(mesh, "Peça simples")?;
    Ok(Scene {
        session,
        source,
        extra: json!({}),
    })
}

/// `side × side` instâncias da peça de referência em grade, compartilhando a definição.
fn instances(side: usize) -> Result<Scene> {
    let (mesh, source) = part_mesh()?;
    let (mut session, part) = document_with(mesh, "Instâncias")?;
    for i in 1..side * side {
        let placement = Placement::new(
            [(i % side) as f64 * 80., (i / side) as f64 * 40., 0.],
            [0.; 3],
        )?;
        session.execute(Command::AddInstance {
            part,
            parent: None,
            placement,
        })?;
    }
    Ok(Scene {
        session,
        source,
        extra: json!({"instances": side * side}),
    })
}

/// Toro fechado com `2·u·v` triângulos, gravado como STL binário e importado de volta.
fn torus(u: usize, v: usize) -> Result<Mesh> {
    use std::f64::consts::TAU;
    let (major, minor) = (40., 15.);
    let mut vertices = Vec::with_capacity(u * v);
    for i in 0..u {
        let a = TAU * i as f64 / u as f64;
        for j in 0..v {
            let b = TAU * j as f64 / v as f64;
            let r = major + minor * b.cos();
            vertices.push([r * a.cos(), r * a.sin(), minor * b.sin()]);
        }
    }
    let index = |i: usize, j: usize| ((i % u) * v + (j % v)) as u32;
    let mut triangles = Vec::with_capacity(2 * u * v);
    for i in 0..u {
        for j in 0..v {
            let (a, b, c, d) = (
                index(i, j),
                index(i + 1, j),
                index(i + 1, j + 1),
                index(i, j + 1),
            );
            triangles.push([a, b, c]);
            triangles.push([a, c, d]);
        }
    }
    Ok(Mesh::new(vertices, triangles)?)
}

fn imported_mesh(directory: &Path) -> Result<Value> {
    let path = directory.join("torus.stl");
    if !path.exists() {
        nator_project_io::write_stl(&path, &torus(1000, 500)?)?;
    }
    let bytes = std::fs::metadata(&path)?.len();
    let start = Instant::now();
    let mesh = nator_project_io::read_stl(&path, 1.)?;
    let import_ms = elapsed(start);
    let start = Instant::now();
    let (session, _) = document_with(mesh, "Malha importada")?;
    let analysis_ms = elapsed(start);
    let mut result = render_scenario(
        "imported_mesh",
        directory,
        Scene {
            session,
            source: "synthetic torus written as binary STL",
            extra: json!({"stl_bytes": bytes}),
        },
    )?;
    result["stl_import_ms"] = json!(import_ms);
    result["topology_analysis_ms"] = json!(analysis_ms);
    Ok(result)
}

#[cfg(feature = "occt")]
fn recalculation() -> Result<Value> {
    use crate::reference_part::{self, set_length};
    let start = Instant::now();
    let mut part = reference_part::build()?;
    let build_ms = elapsed(start);
    let mut samples = Vec::new();
    for value in ["8 mm", "6 mm"].iter().cycle().take(20) {
        let start = Instant::now();
        set_length(&mut part.session, part.part, "espessura", value)?;
        samples.push(elapsed(start));
    }
    Ok(json!({
        "name": "recalculation",
        "source": "parametric bracket: sketch, extrude, hole, boolean cut, fillet",
        "history_operations": part.session.document().part(part.part).map(|p| p.features.len()),
        "initial_build_ms": build_ms,
        "parameter_change_ms": statistics(&mut samples),
        "memory": memory(),
    }))
}

#[cfg(not(feature = "occt"))]
fn recalculation() -> Result<Value> {
    Ok(json!({"name": "recalculation", "skipped": "OCCT não habilitado nesta compilação"}))
}

// ----- Medição comum ---------------------------------------------------------------------

fn render_scenario(name: &str, directory: &Path, scene: Scene) -> Result<Value> {
    // Abertura: gravar o projeto e medir a leitura completa, com validação.
    let project = directory.join(format!("{name}.3dnator"));
    nator_project_io::write_project(&project, scene.session.document())?;
    let project_bytes = std::fs::metadata(&project)?.len();
    let start = Instant::now();
    let document = nator_project_io::read_project(&project)?;
    let open_ms = elapsed(start);
    let memory_after_open = memory();

    // Mesmo caminho do editor: cena derivada e dados de GPU preparados na CPU.
    let start = Instant::now();
    let derived = document.scene()?.ok_or("Cena vazia")?;
    let scene_ms = elapsed(start);
    let start = Instant::now();
    let prepared = PreparedMesh::with_creases(&derived.mesh, &derived.creases)?;
    let prepare_ms = elapsed(start);
    let gpu_bytes = prepared.gpu_bytes();
    let scene_triangles = derived.mesh.triangles().len();
    // Como no editor, a cena derivada é descartada depois de preparada.
    drop(derived);

    let gpu = Gpu::new()?;
    let start = Instant::now();
    let mut viewport = Viewport::new(&gpu.device, &nator_geometry::reference_bracket()?)?;
    viewport.resize(&gpu.device, VIEWPORT);
    viewport.set_prepared_mesh(&gpu.device, &prepared)?;
    gpu.wait()?;
    let upload_ms = elapsed(start);
    drop(prepared);
    let memory_at_rest = memory();

    // Navegação: órbita contínua, um quadro completo por iteração.
    let mut samples = Vec::with_capacity(MEASURED_FRAMES);
    for frame in 0..WARMUP_FRAMES + MEASURED_FRAMES {
        let start = Instant::now();
        viewport.camera.orbit([3., 0.5]);
        viewport.render(&gpu.device, &gpu.queue);
        gpu.wait()?;
        if frame >= WARMUP_FRAMES {
            samples.push(elapsed(start));
        }
    }
    let mut result = json!({
        "name": name,
        "source": scene.source,
        "scene_triangles": scene_triangles,
        "gpu_vertex_bytes": gpu_bytes,
        "project_bytes": project_bytes,
        "open_project_ms": open_ms,
        "scene_mesh_ms": scene_ms,
        "gpu_data_prepare_ms": prepare_ms,
        "gpu_upload_ms": upload_ms,
        "frame_ms": statistics(&mut samples),
        "adapter": gpu.adapter,
        "memory_after_open": memory_after_open,
        "memory_at_rest": memory_at_rest,
        "memory_final": memory(),
    });
    if let (Some(target), Some(extra)) = (result.as_object_mut(), scene.extra.as_object()) {
        target.extend(extra.clone());
    }
    Ok(result)
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: Value,
}
impl Gpu {
    fn new() -> Result<Self> {
        let preference = match std::env::var("NATOR_BENCH_GPU").as_deref() {
            Ok("high") => wgpu::PowerPreference::HighPerformance,
            _ => wgpu::PowerPreference::LowPower,
        };
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: preference,
            ..Default::default()
        }))?;
        let info = adapter.get_info();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
        Ok(Self {
            device,
            queue,
            adapter: json!({
                "name": info.name,
                "backend": format!("{:?}", info.backend),
                "device_type": format!("{:?}", info.device_type),
                "driver": info.driver,
                "driver_info": info.driver_info,
                "power_preference": format!("{preference:?}"),
            }),
        })
    }
    fn wait(&self) -> Result<()> {
        self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })?;
        Ok(())
    }
}

fn elapsed(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.
}

/// Média, percentil 95 e máximo em milissegundos.
fn statistics(samples: &mut [f64]) -> Value {
    if samples.is_empty() {
        return Value::Null;
    }
    samples.sort_by(f64::total_cmp);
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let p95 = samples[((samples.len() as f64 * 0.95).ceil() as usize).clamp(1, samples.len()) - 1];
    json!({"mean": mean, "p95": p95, "max": samples[samples.len() - 1]})
}

/// RSS atual e pico do processo (Linux). Em outros sistemas a coleta ainda não existe.
fn memory() -> Value {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        let field = |name: &str| {
            status
                .lines()
                .find(|line| line.starts_with(name))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
                .map(|kb| kb * 1024)
        };
        json!({"rss_bytes": field("VmRSS:"), "peak_rss_bytes": field("VmHWM:")})
    }
    #[cfg(not(target_os = "linux"))]
    {
        json!({"rss_bytes": null, "peak_rss_bytes": null})
    }
}

fn system() -> Value {
    #[cfg(target_os = "linux")]
    let (cpu, memory_total) = {
        let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let cpu = cpuinfo
            .lines()
            .find(|l| l.starts_with("model name"))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| s.trim().to_owned());
        let total = meminfo
            .lines()
            .find(|l| l.starts_with("MemTotal:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .map(|kb| kb * 1024);
        (cpu, total)
    };
    #[cfg(not(target_os = "linux"))]
    let (cpu, memory_total): (Option<String>, Option<u64>) = (None, None);
    json!({
        "os": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "cpu": cpu,
        "logical_cpus": std::thread::available_parallelism().map(|n| n.get()).ok(),
        "memory_total_bytes": memory_total,
        "occt": nator_kernel::is_available(),
    })
}
