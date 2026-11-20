//! Regressão optativa com projetos externos, sem incorporar arquivos de terceiros.
#![cfg(feature = "occt")]
use nator_commands::export::{self, Quality, Scope};
use std::{error::Error, path::PathBuf};

#[test]
#[ignore = "Requer NATOR_EXPORT_PROJECT e NATOR_EXPORT_PART para um projeto externo."]
fn imported_part_exports_closed_stl_at_each_quality() -> Result<(), Box<dyn Error>> {
    let path = PathBuf::from(
        std::env::var_os("NATOR_EXPORT_PROJECT").ok_or("Informe NATOR_EXPORT_PROJECT.")?,
    );
    let name = std::env::var("NATOR_EXPORT_PART")?;
    let document = nator_project_io::read_project(&path)?;
    let part = document
        .parts()
        .iter()
        .find(|part| part.name == name)
        .ok_or("Peça não encontrada no projeto externo.")?;
    let directory = tempfile::tempdir()?;
    for (index, quality) in [Quality::Draft, Quality::Standard, Quality::High]
        .into_iter()
        .enumerate()
    {
        let files = export::plan(&document, Scope::Part(part.id), quality)?;
        assert!(!files.is_empty());
        for (body, file) in files.iter().enumerate() {
            nator_project_io::require_printable(&file.name, &file.mesh)?;
            let output = directory.path().join(format!("{index}-{body}.stl"));
            nator_project_io::write_stl(&output, &file.mesh)?;
            let reopened = nator_project_io::read_stl(&output, 1.)?;
            // O leitor descarta triângulos de área zero; a preparação STL recompõe
            // as vedações numéricas antes de verificar novamente o fechamento.
            assert!(
                reopened
                    .single_precision()?
                    .topology()
                    .has_closed_oriented_edges(),
                "{}: {quality:?}",
                file.name
            );
            assert!(reopened.signed_volume() > 0.);
        }
        eprintln!(
            "{quality:?}: {} corpos exportados e reabertos sem defeitos.",
            files.len()
        );
    }
    Ok(())
}
