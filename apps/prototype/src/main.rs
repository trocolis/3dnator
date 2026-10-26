//! Entrada do protótipo: CLI reproduzível e visualizador desktop opcional.
use nator_geometry::{Mesh, reference_bracket};
use std::{
    error::Error,
    path::{Path, PathBuf},
    process::ExitCode,
};

#[cfg(feature = "desktop")]
mod benchmark;
#[cfg(feature = "desktop")]
mod desktop;
mod editor_validation;
#[cfg(feature = "desktop")]
mod i18n;
mod part_validation;
#[cfg(feature = "desktop")]
mod preview;
mod project_validation;
#[cfg(feature = "occt")]
mod reference_part;
mod sketch_validation;
mod validation;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Erro: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => {
            #[cfg(feature = "desktop")]
            {
                desktop::run()?;
                Ok(())
            }
            #[cfg(not(feature = "desktop"))]
            {
                Err("Este binário foi compilado sem desktop. Use --help para as opções CLI.".into())
            }
        }
        [command] if command == "--help" || command == "-h" => {
            println!(
                "3dnator — protótipo {}\n\nSem argumentos: abrir editor de projetos.\n--inspect ARQUIVO.stl: inspecionar malha (coordenadas assumidas em mm).\n--export-reference ARQUIVO.stl: exportar suporte em L de referência.\n--validate DIRETORIO_NOVO: validar OCCT e gerar relatório e arquivos.\n--validate-project DIRETORIO_NOVO: validar salvamento, recuperação e comandos.\n--validate-sketch DIRETORIO_NOVO: validar restrições, expressões e recálculo.\n--validate-part DIRETORIO_NOVO: validar a peça de referência com OCCT (cotas, recálculo, STEP/STL).\n--validate-editor DIRETORIO_NOVO: validar montagem e folhas PNG/PDF.\n--render-project-preview PROJETO IMAGEM.png: capturar o editor real.\n--inspect-project ARQUIVO.3dnator: inspecionar projeto.\n--render-preview ARQUIVO.png: capturar renderizador (requer desktop).\n--benchmark DIRETORIO_NOVO: medir abertura, memória, navegação e recálculo (use --release).\n--version: informar versão e disponibilidade do núcleo.\n\nSTL aceita ASCII ou binário. STEP e operações CAD exigem --features occt.\nRascunhos, parâmetros, montagem e folhas disponíveis no protótipo.",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
        [command] if command == "--version" => {
            println!(
                "3dnator {} | OCCT: {}",
                env!("CARGO_PKG_VERSION"),
                if nator_kernel::is_available() {
                    "habilitado"
                } else {
                    "indisponível"
                }
            );
            Ok(())
        }
        [command, path] if command == "--inspect" => {
            let mesh = load_model(Path::new(path), 1.)?;
            let topology = mesh.topology();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "source":path.to_string_lossy(),"body_type":"mesh","assumed_unit":"mm",
                    "bounds_mm":{"min":mesh.bounds().min,"max":mesh.bounds().max,"size":mesh.bounds().size()},
                    "triangles":mesh.triangles().len(),"boundary_edges":topology.boundary_edges,
                    "non_manifold_edges":topology.non_manifold_edges,"inconsistent_edges":topology.inconsistent_edges,
                    "signed_mesh_volume_mm3":mesh.signed_volume(),"self_intersections_checked":false
                }))?
            );
            Ok(())
        }
        [command, path] if command == "--export-reference" => {
            let mesh = reference_bracket()?;
            nator_project_io::write_stl(Path::new(path), &mesh)?;
            println!(
                "Suporte de referência exportado: {} (60 × 20 × 30 mm).",
                path.to_string_lossy()
            );
            Ok(())
        }
        [command, path] if command == "--validate" => validation::run(&PathBuf::from(path)),
        [command, path] if command == "--validate-project" => {
            project_validation::run(Path::new(path))
        }
        [command, path] if command == "--validate-editor" => {
            editor_validation::run(Path::new(path))
        }
        [command, path] if command == "--validate-part" => part_validation::run(Path::new(path)),
        [command, path] if command == "--validate-sketch" => {
            sketch_validation::run(Path::new(path))
        }
        [command, path] if command == "--inspect-project" => {
            project_validation::inspect(Path::new(path))
        }
        #[cfg(feature = "desktop")]
        [command, path] if command == "--render-preview" => preview::render(Path::new(path)),
        #[cfg(feature = "desktop")]
        [command, path] if command == "--benchmark" => benchmark::run(Path::new(path)),
        #[cfg(feature = "desktop")]
        [command, name, path] if command == "--benchmark-scenario" => {
            benchmark::scenario(&name.to_string_lossy(), Path::new(path))
        }
        #[cfg(feature = "desktop")]
        [command, project, output] if command == "--render-project-preview" => {
            desktop::render_project(Path::new(project), Path::new(output))
        }
        _ => Err("Argumentos inválidos. Use --help.".into()),
    }
}

fn load_model(path: &Path, scale: f64) -> Result<Mesh, Box<dyn Error>> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("stl") => Ok(nator_project_io::read_stl(path, scale)?),
        Some("step" | "stp") => {
            #[cfg(feature = "occt")]
            {
                Ok(nator_kernel::Solid::read_step(path)?.tessellate(0.05, 0.25)?)
            }
            #[cfg(not(feature = "occt"))]
            {
                nator_kernel::require_available()?;
                Err("Leitura STEP indisponível.".into())
            }
        }
        _ => Err("Formato não suportado. Escolha STL, STEP ou STP.".into()),
    }
}
