//! Persistência de projetos e intercâmbio STL, sem dependência da interface.
//! Malhas abertas são aceitas como referência; não há reparo ou conversão implícita.

use nator_geometry::{Mesh, cross, norm, sub};
use std::{
    error::Error,
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

mod project;
pub use project::{
    FORMAT_VERSION, read_project, recovery_directory, recovery_files, remove_recovery,
    write_project, write_recovery,
};

pub const MAX_STL_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_TRIANGLES: usize = 2_000_000;

#[derive(Debug)]
pub struct ImportError(pub String);
impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for ImportError {}
impl From<std::io::Error> for ImportError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

pub fn read_stl(path: &Path, scale_to_mm: f64) -> Result<Mesh, ImportError> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(ImportError(
            "O caminho STL não é um arquivo regular.".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_STL_BYTES + 1).read_to_end(&mut bytes)?;
    decode_stl(&bytes, scale_to_mm)
}

pub fn decode_stl(bytes: &[u8], scale_to_mm: f64) -> Result<Mesh, ImportError> {
    if !scale_to_mm.is_finite() || scale_to_mm <= 0. {
        return Err(ImportError("A escala deve ser positiva e finita.".into()));
    }
    if bytes.len() as u64 > MAX_STL_BYTES {
        return Err(ImportError(
            "STL excede o limite de 128 MiB do protótipo.".into(),
        ));
    }
    // Cabeçalhos binários também podem começar com 'solid': o tamanho exato tem precedência.
    let binary_count = bytes
        .get(80..84)
        .and_then(|b| <[u8; 4]>::try_from(b).ok())
        .map(u32::from_le_bytes);
    let binary = binary_count.is_some_and(|n| 84_u64 + u64::from(n) * 50 == bytes.len() as u64);
    let vertices = if binary {
        let count =
            binary_count.ok_or_else(|| ImportError("Cabeçalho STL inválido.".into()))? as usize;
        if count == 0 || count > MAX_TRIANGLES {
            return Err(ImportError(
                "Quantidade de triângulos inválida ou superior a dois milhões.".into(),
            ));
        }
        let mut vertices = Vec::with_capacity(count * 3);
        for record in bytes[84..].as_chunks::<50>().0 {
            // Normais do arquivo são ignoradas: a apresentação usa a orientação dos vértices.
            for point in record[12..48].as_chunks::<12>().0 {
                let mut xyz = [0.; 3];
                for (axis, coordinate) in point.as_chunks::<4>().0.iter().enumerate() {
                    xyz[axis] = f64::from(f32::from_le_bytes(*coordinate)) * scale_to_mm;
                }
                vertices.push(xyz);
            }
        }
        vertices
    } else {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| ImportError("STL binário truncado ou texto ASCII inválido.".into()))?;
        decode_ascii(text, scale_to_mm)?
    };
    let triangles = (0..(vertices.len() / 3) as u32)
        .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
        .collect();
    Mesh::new(vertices, triangles).map_err(|e| ImportError(e.to_string()))
}

fn decode_ascii(text: &str, scale: f64) -> Result<Vec<[f64; 3]>, ImportError> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let header = lines
        .next()
        .ok_or_else(|| ImportError("STL vazio.".into()))?;
    if header.split_whitespace().next() != Some("solid") {
        return Err(ImportError(
            "Cabeçalho STL ASCII ausente ou tamanho binário incorreto.".into(),
        ));
    }
    let mut vertices = Vec::new();
    loop {
        let line = lines
            .next()
            .ok_or_else(|| ImportError("STL ASCII sem endsolid.".into()))?;
        if line.split_whitespace().next() == Some("endsolid") {
            if lines.next().is_some() {
                return Err(ImportError(
                    "Conteúdo adicional após endsolid não é suportado neste protótipo.".into(),
                ));
            }
            break;
        }
        if vertices.len() / 3 >= MAX_TRIANGLES {
            return Err(ImportError("STL excede dois milhões de triângulos.".into()));
        }
        let mut words = line.split_whitespace();
        if words.next() != Some("facet") || words.next() != Some("normal") {
            return Err(ImportError("Esperado facet normal no STL ASCII.".into()));
        }
        for _ in 0..3 {
            parse_number(words.next())?;
        }
        if words.next().is_some() {
            return Err(ImportError("Normal STL com dados adicionais.".into()));
        }
        expect_line(lines.next(), "outer loop")?;
        for _ in 0..3 {
            let mut words = lines
                .next()
                .ok_or_else(|| ImportError("Triângulo ASCII incompleto.".into()))?
                .split_whitespace();
            if words.next() != Some("vertex") {
                return Err(ImportError("Esperado vertex no STL ASCII.".into()));
            }
            let mut point = [0.; 3];
            for coordinate in &mut point {
                *coordinate = parse_number(words.next())? * scale;
            }
            if words.next().is_some() {
                return Err(ImportError("Vértice STL com dados adicionais.".into()));
            }
            vertices.push(point);
        }
        expect_line(lines.next(), "endloop")?;
        expect_line(lines.next(), "endfacet")?;
    }
    Ok(vertices)
}

fn expect_line(line: Option<&str>, expected: &str) -> Result<(), ImportError> {
    if line.is_some_and(|line| line.split_whitespace().eq(expected.split_whitespace())) {
        Ok(())
    } else {
        Err(ImportError(format!("Esperado '{expected}' no STL ASCII.")))
    }
}

fn parse_number(word: Option<&str>) -> Result<f64, ImportError> {
    let value = word
        .and_then(|w| w.parse::<f64>().ok())
        .filter(|v| v.is_finite());
    value.ok_or_else(|| ImportError("Número inválido ou não finito no STL.".into()))
}

pub fn encode_stl(mesh: &Mesh, mut output: impl Write) -> Result<(), ImportError> {
    if mesh.triangles().len() > MAX_TRIANGLES {
        return Err(ImportError(
            "Malha excede dois milhões de triângulos.".into(),
        ));
    }
    // A quantização do STL pode colapsar detalhes. Validá-la antes de começar a escrever.
    let quantized = Mesh::new(
        mesh.vertices()
            .iter()
            .map(|p| p.map(|v| f64::from(v as f32)))
            .collect(),
        mesh.triangles().to_vec(),
    )
    .map_err(|e| ImportError(format!("Precisão STL insuficiente: {e}")))?;
    if !quantized.topology().has_closed_oriented_edges() || quantized.signed_volume() <= 0. {
        return Err(ImportError("A exportação exige arestas fechadas, orientação consistente e volume positivo. A malha original foi preservada.".into()));
    }
    let mut header = [0_u8; 80];
    let label = b"3dnator - coordinates in millimeters";
    header[..label.len()].copy_from_slice(label);
    output.write_all(&header)?;
    output.write_all(&(quantized.triangles().len() as u32).to_le_bytes())?;
    for triangle in quantized.triangles() {
        let [a, b, c] = quantized.triangle_points(*triangle);
        let normal = cross(sub(b, a), sub(c, a));
        let normal = normal.map(|n| n / norm(normal));
        for point in [normal, a, b, c] {
            for value in point {
                output.write_all(&(value as f32).to_le_bytes())?;
            }
        }
        output.write_all(&0_u16.to_le_bytes())?;
    }
    Ok(())
}

pub fn write_stl(path: &Path, mesh: &Mesh) -> Result<(), ImportError> {
    // Nunca sobrescrever um arquivo existente sem uma escolha explícita do usuário.
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let result = encode_stl(mesh, &mut file).and_then(|()| file.sync_all().map_err(Into::into));
    drop(file);
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}

/// Exportação nativa publicada somente após sucesso e sem sobrescrever destinos.
pub fn export_step(
    path: &std::path::Path,
    write: impl FnOnce(&std::path::Path) -> Result<(), String>,
) -> Result<(), ImportError> {
    if path.exists() {
        return Err(ImportError("O destino STEP já existe.".into()));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let temporary = tempfile::Builder::new()
        .suffix(".step")
        .tempfile_in(parent)?;
    write(temporary.path()).map_err(ImportError)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(path)
        .map_err(|e| ImportError(e.to_string()))?;
    Ok(())
}

/// Imagens e documentos são publicados atomicamente, com destino novo.
pub fn write_export(path: &std::path::Path, bytes: &[u8]) -> Result<(), ImportError> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist_noclobber(path)
        .map_err(|e| ImportError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nator_geometry::reference_bracket;

    #[test]
    fn binary_round_trip_preserves_dimensions_orientation_and_volume() -> Result<(), Box<dyn Error>>
    {
        let mesh = reference_bracket()?;
        let mut bytes = Vec::new();
        encode_stl(&mesh, &mut bytes)?;
        assert_eq!(bytes.len(), 84 + mesh.triangles().len() * 50);
        let imported = decode_stl(&bytes, 1.)?;
        assert_eq!(imported.bounds().size(), mesh.bounds().size());
        assert!(imported.topology().has_closed_oriented_edges());
        assert!((imported.signed_volume() - mesh.signed_volume()).abs() < 1e-9);
        // 'solid' não distingue o STL binário do ASCII.
        bytes[..5].copy_from_slice(b"solid");
        assert!(decode_stl(&bytes, 1.).is_ok());
        Ok(())
    }

    #[test]
    fn reads_open_ascii_mesh_and_applies_scale() -> Result<(), Box<dyn Error>> {
        let ascii = b"solid sample\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid sample\n";
        let mesh = decode_stl(ascii, 25.4)?;
        assert_eq!(mesh.bounds().size(), [25.4, 25.4, 0.]);
        assert_eq!(mesh.topology().boundary_edges, 3);
        assert!(encode_stl(&mesh, Vec::new()).is_err());
        Ok(())
    }

    #[test]
    fn rejects_truncation_nonfinite_values_and_hostile_counts() -> Result<(), Box<dyn Error>> {
        let mut bytes = Vec::new();
        encode_stl(&reference_bracket()?, &mut bytes)?;
        assert!(decode_stl(&bytes[..bytes.len() - 1], 1.).is_err());
        let mut hostile = bytes.clone();
        hostile[80..84].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode_stl(&hostile, 1.).is_err());
        bytes[96..100].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(decode_stl(&bytes, 1.).is_err());
        assert!(decode_stl(b"", f64::INFINITY).is_err());
        assert!(decode_stl(b"solid x\nendsolid x", 1.).is_err());
        Ok(())
    }

    #[test]
    fn export_never_overwrites_and_removes_incomplete_output() -> Result<(), Box<dyn Error>> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("nator-stl-{}-{nonce}", std::process::id()));
        fs::create_dir(&directory)?;
        let result = (|| -> Result<(), Box<dyn Error>> {
            let path = directory.join("existing.stl");
            fs::write(&path, b"preservar")?;
            assert!(write_stl(&path, &reference_bracket()?).is_err());
            assert_eq!(fs::read(&path)?, b"preservar");
            let open = Mesh::new(
                vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
                vec![[0, 1, 2]],
            )?;
            let rejected = directory.join("rejected.stl");
            assert!(write_stl(&rejected, &open).is_err());
            assert!(!rejected.exists());
            Ok(())
        })();
        fs::remove_dir_all(directory)?;
        result
    }

    #[test]
    fn rejects_details_collapsed_by_stl_float_precision() -> Result<(), Box<dyn Error>> {
        let mesh = reference_bracket()?;
        let translated = Mesh::new(
            mesh.vertices()
                .iter()
                .map(|p| p.map(|v| v + 1_000_000_000.))
                .collect(),
            mesh.triangles().to_vec(),
        )?;
        assert!(encode_stl(&translated, Vec::new()).is_err());
        Ok(())
    }
}
