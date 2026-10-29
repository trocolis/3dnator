//! Preferência local da paleta. Independe dos projetos e de seu desfazer/refazer.
use crate::ImportError;
use std::{io::Write, path::Path};
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
    let parent = path
        .parent()
        .ok_or_else(|| ImportError("Caminho da preferência inválido.".into()))?;
    std::fs::create_dir_all(parent)?;
    // Mesmo volume e substituição atômica, inclusive no Windows: uma interrupção
    // nunca deixa um JSON pela metade no lugar da última cor válida.
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&serde_json::to_vec(&color).map_err(|e| ImportError(e.to_string()))?)?;
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
