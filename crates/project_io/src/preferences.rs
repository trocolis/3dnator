//! Preferências locais: paleta e biblioteca de perfis de impressão. Independem dos projetos
//! e de seu desfazer/refazer.
use crate::ImportError;
use nator_document::print_profile::PrintProfile;
use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};
/// Limite de leitura da biblioteca de perfis.
const MAX_PROFILES_BYTES: u64 = 1 << 20;
pub fn read_color_preference(path: &Path) -> Result<[u8; 3], ImportError> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(65)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 {
        return Err(ImportError("Preferência de cor inválida.".into()));
    }
    serde_json::from_slice(&bytes).map_err(|e| ImportError(e.to_string()))
}
pub fn write_color_preference(path: &Path, color: [u8; 3]) -> Result<(), ImportError> {
    let bytes = serde_json::to_vec(&color).map_err(|e| ImportError(e.to_string()))?;
    write_atomic(path, &bytes)
}
/// Escolhas da exportação que valem entre projetos. O comando do fatiador é pessoal e fica
/// fora do projeto: ele depende da máquina, não do modelo.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintExportPreferences {
    pub open_slicer: bool,
    pub slicer_command: String,
}
/// Sem arquivo ou com arquivo ilegível, valem os padrões: nada aqui guarda trabalho.
pub fn read_print_export(path: &Path) -> PrintExportPreferences {
    std::fs::read(path)
        .ok()
        .filter(|bytes| bytes.len() as u64 <= MAX_PROFILES_BYTES)
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}
pub fn write_print_export(
    path: &Path,
    preferences: &PrintExportPreferences,
) -> Result<(), ImportError> {
    let bytes = serde_json::to_vec_pretty(preferences).map_err(|e| ImportError(e.to_string()))?;
    write_atomic(path, &bytes)
}
/// Biblioteca pessoal de perfis. Sem arquivo, está vazia; um perfil inválido é erro, para
/// que a gravação seguinte não apague silenciosamente os demais.
pub fn read_print_profiles(path: &Path) -> Result<Vec<PrintProfile>, ImportError> {
    use std::io::Read;
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(MAX_PROFILES_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PROFILES_BYTES {
        return Err(ImportError("Biblioteca de perfis grande demais.".into()));
    }
    let profiles: Vec<PrintProfile> =
        serde_json::from_slice(&bytes).map_err(|e| ImportError(e.to_string()))?;
    for profile in &profiles {
        profile.validate().map_err(|e| ImportError(e.to_string()))?;
    }
    Ok(profiles)
}
pub fn write_print_profiles(path: &Path, profiles: &[PrintProfile]) -> Result<(), ImportError> {
    let bytes = serde_json::to_vec_pretty(profiles).map_err(|e| ImportError(e.to_string()))?;
    write_atomic(path, &bytes)
}
/// Grava um arquivo de preferências por inteiro, com substituição atômica.
pub fn write_preference(path: &Path, bytes: &[u8]) -> Result<(), ImportError> {
    if bytes.len() as u64 > MAX_PROFILES_BYTES {
        return Err(ImportError("Preferência grande demais.".into()));
    }
    write_atomic(path, bytes)
}
/// Lê um arquivo de preferências pequeno; ausente ou grande demais, `None`.
pub fn read_preference(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path)
        .ok()
        .filter(|bytes| bytes.len() as u64 <= MAX_PROFILES_BYTES)
}
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), ImportError> {
    let parent = path
        .parent()
        .ok_or_else(|| ImportError("Caminho da preferência inválido.".into()))?;
    std::fs::create_dir_all(parent)?;
    // Mesmo volume e substituição atômica, inclusive no Windows: uma interrupção
    // nunca deixa um JSON pela metade no lugar do último conteúdo válido.
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|e| ImportError(e.to_string()))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_preferences_round_trip_and_fall_back_to_defaults()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("preferences/print-export.json");
        assert_eq!(read_print_export(&path), Default::default());
        let preferences = PrintExportPreferences {
            open_slicer: true,
            slicer_command: "fatiador {arquivo}".into(),
        };
        write_print_export(&path, &preferences)?;
        assert_eq!(read_print_export(&path), preferences);
        std::fs::write(&path, b"nada")?;
        assert_eq!(read_print_export(&path), Default::default());
        Ok(())
    }
    #[test]
    fn print_profiles_round_trip_and_invalid_libraries_are_refused()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("preferences/print-profiles.json");
        assert!(read_print_profiles(&path)?.is_empty());
        let profile = PrintProfile {
            name: "Bico 0,6".into(),
            nozzle_mm: 0.6,
            bed_mm: Some([220., 220., 250.]),
            ..Default::default()
        };
        write_print_profiles(&path, std::slice::from_ref(&profile))?;
        assert_eq!(read_print_profiles(&path)?, vec![profile]);
        // Bibliotecas anteriores à tabela de fixadores ganham a tabela padrão.
        std::fs::write(&path, br#"[{"nozzle_mm":0.6}]"#)?;
        assert_eq!(read_print_profiles(&path)?[0].fasteners.len(), 5);
        std::fs::write(&path, br#"[{"nozzle_mm":-1}]"#)?;
        assert!(read_print_profiles(&path).is_err());
        Ok(())
    }
    #[test]
    fn custom_color_replaces_previous_value_atomically_and_reopens()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("preferences/paint.json");
        write_color_preference(&path, [18, 72, 172])?;
        assert_eq!(read_color_preference(&path)?, [18, 72, 172]);
        write_color_preference(&path, [255, 240, 128])?;
        assert_eq!(read_color_preference(&path)?, [255, 240, 128]);
        std::fs::write(&path, b"[256,0,0]")?;
        assert!(read_color_preference(&path).is_err());
        Ok(())
    }
}
