//! Pacote ZIP autocontido, com manifesto JSON e malhas binárias f64.
//! O leitor nunca extrai caminhos do arquivo para o sistema de arquivos.
use super::ImportError;
use nator_document::{
    Body, BodyId, Document, Instance, InstanceId, MAX_ELEMENTS, MAX_SCENE_TRIANGLES, MeshAsset,
    Part, PartId, SketchId, SketchRecord, WorkPlaneId, WorkPlaneRecord,
};
use nator_geometry::{Mesh, Placement, Point3, RgbColor};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

/// Versão 15: visibilidade das pastas da árvore (`folders` no projeto e em cada peça).
/// Ausente, vale o padrão: tudo visível, exceto a origem das peças.
/// Versão 14: rascunhos de referência da montagem (`reference_sketches`), fora das peças.
/// Versão 13: configurações do documento (casas decimais exibidas) e miniatura PNG opcional
/// (`thumbnail.png`). Versões anteriores usam as configurações padrão e não têm miniatura.
/// Versão 12: operações suprimidas e marcador do histórico por peça. Versões anteriores
/// abrem com todas as operações ativas e sem marcador (campos ausentes).
/// Versão 11: cotas associativas das folhas, com caminhos e referências topológicas estáveis.
/// Projetos anteriores usam a cor neutra;
/// referências topológicas das versões anteriores a 7 ainda passam por `migrate_to_7`.
/// Versão 18: retas infinitas de construção e cotas angulares relativas a linhas.
/// Versão 19: tangência de círculo completo com reta suporte (Tangent usa seu centro).
/// Versão 20: planos de trabalho por peça (`work_planes`) e visibilidade da pasta Planos
/// (`folders.planes`). Ausentes, a peça não tem planos e a pasta fica visível.
pub const FORMAT_VERSION: u32 = 20;
const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_EXPANDED_BYTES: u64 = 512 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
const MESH_MAGIC: &[u8; 8] = b"3DMSH001";
/// Faces de origem dos triângulos de um sólido: assinatura, contagem u32 e um u32 por
/// triângulo, todos little-endian.
const FACES_MAGIC: &[u8; 8] = b"3DFAC001";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    schema_version: u32,
    app_version: String,
    #[serde(default)]
    recovery: bool,
    document_id: String,
    name: String,
    next_id: u64,
    parts: Vec<PartRecord>,
    instances: Vec<InstanceRecord>,
    #[serde(default)]
    groups: Vec<GroupRecord>,
    #[serde(default)]
    sheets: Vec<nator_document::SheetRecord>,
    #[serde(default)]
    settings: nator_document::Settings,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    reference_sketches: Vec<SketchRecordData>,
    #[serde(default, skip_serializing_if = "is_default")]
    folders: nator_document::ProjectFolders,
}
/// Campos opcionais com valor padrão não são gravados.
fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PartRecord {
    id: u64,
    name: String,
    bodies: Vec<BodyRecord>,
    #[serde(default)]
    sketches: Vec<SketchRecordData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    work_planes: Vec<WorkPlaneRecordData>,
    #[serde(default)]
    parameters: nator_document::parameters::Parameters,
    #[serde(default)]
    features: Vec<nator_document::features::Feature>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    history_marker: Option<usize>,
    #[serde(default, skip_serializing_if = "is_default")]
    folders: nator_document::PartFolders,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SketchRecordData {
    id: u64,
    sketch: nator_sketch::Sketch,
    visible: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkPlaneRecordData {
    id: u64,
    name: String,
    /// Plano-base e deslocamento em mm, nas coordenadas da peça.
    plane: nator_document::WorkPlane,
    visible: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BodyRecord {
    id: u64,
    name: String,
    visible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color_srgb: Option<[u8; 3]>,
    placement: PlacementRecord,
    mesh_path: String,
    #[serde(default)]
    brep_path: Option<String>,
    /// Um nome por face do B-rep, na ordem topológica; vazio em corpos de malha.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    face_names: Vec<String>,
    /// Face de origem de cada triângulo da malha de um sólido (`faces/<id>.bin`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    faces_path: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupRecord {
    id: u64,
    name: String,
    members: Vec<u64>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstanceRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color_srgb: Option<[u8; 3]>,
    #[serde(default)]
    parent: Option<u64>,
    id: u64,
    part: u64,
    name: String,
    visible: bool,
    placement: PlacementRecord,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlacementRecord {
    translation_mm: Point3,
    rotation_rad: Point3,
}
impl From<Placement> for PlacementRecord {
    fn from(p: Placement) -> Self {
        Self {
            translation_mm: p.translation_mm(),
            rotation_rad: p.rotation_rad(),
        }
    }
}
impl PlacementRecord {
    fn into_placement(self) -> Result<Placement, ImportError> {
        Placement::new(self.translation_mm, self.rotation_rad)
            .map_err(|e| ImportError(e.to_string()))
    }
}

fn error(e: impl std::fmt::Display) -> ImportError {
    ImportError(e.to_string())
}
fn faces_path(id: u64) -> String {
    format!("faces/{id}.bin")
}

fn decode_faces(bytes: &[u8], triangles: usize) -> Result<Vec<u32>, ImportError> {
    let invalid = || ImportError("Faces de origem dos triângulos inválidas.".into());
    if bytes.get(..8) != Some(FACES_MAGIC.as_slice()) {
        return Err(invalid());
    }
    let count = bytes
        .get(8..12)
        .and_then(|b| <[u8; 4]>::try_from(b).ok())
        .map(u32::from_le_bytes)
        .ok_or_else(invalid)? as usize;
    if count != triangles || bytes.len() != 12 + count * 4 {
        return Err(invalid());
    }
    Ok(bytes[12..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect())
}

fn mesh_path(id: u64) -> String {
    format!("meshes/{id}.bin")
}

fn read_entry(
    archive: &mut ZipArchive<File>,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>, ImportError> {
    let entry = archive.by_name(name).map_err(error)?;
    if entry.size() > limit {
        return Err(ImportError(format!(
            "Entrada do projeto excede o limite: {name}"
        )));
    }
    let mut bytes = Vec::new();
    entry.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(ImportError("Conteúdo descompactado excessivo.".into()));
    }
    Ok(bytes)
}

fn decode_mesh(bytes: &[u8]) -> Result<Mesh, ImportError> {
    if bytes.get(..8) != Some(MESH_MAGIC.as_slice()) {
        return Err(ImportError(
            "Cabeçalho de malha do projeto inválido.".into(),
        ));
    }
    let number = |start: usize| -> Result<u32, ImportError> {
        let value = bytes
            .get(start..start + 4)
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| ImportError("Malha de projeto truncada.".into()))?;
        Ok(u32::from_le_bytes(value))
    };
    let vertices = number(8)? as usize;
    let triangles = number(12)? as usize;
    if triangles == 0
        || triangles > MAX_SCENE_TRIANGLES
        || vertices == 0
        || vertices > MAX_SCENE_TRIANGLES * 3
    {
        return Err(ImportError(
            "Contagens de malha do projeto inválidas ou excessivas.".into(),
        ));
    }
    let expected = 16_u64 + vertices as u64 * 24 + triangles as u64 * 12;
    if bytes.len() as u64 != expected {
        return Err(ImportError(
            "Tamanho da malha do projeto não corresponde ao cabeçalho.".into(),
        ));
    }
    let vertex_end = 16 + vertices * 24;
    let vertices = bytes[16..vertex_end]
        .as_chunks::<24>()
        .0
        .iter()
        .map(|p| {
            let mut point = [0.; 3];
            for (axis, value) in p.as_chunks::<8>().0.iter().enumerate() {
                point[axis] = f64::from_le_bytes(*value);
            }
            point
        })
        .collect();
    let triangles = bytes[vertex_end..]
        .as_chunks::<12>()
        .0
        .iter()
        .map(|t| {
            let mut indices = [0; 3];
            for (i, value) in t.as_chunks::<4>().0.iter().enumerate() {
                indices[i] = u32::from_le_bytes(*value);
            }
            indices
        })
        .collect();
    Mesh::new(vertices, triangles).map_err(error)
}

pub fn read_project(path: &Path) -> Result<Document, ImportError> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > MAX_ARCHIVE_BYTES {
        return Err(ImportError(
            "Projeto deve ser um arquivo de até 256 MiB.".into(),
        ));
    }
    let mut archive = ZipArchive::new(file).map_err(error)?;
    if archive.len() > MAX_ELEMENTS + 1 {
        return Err(ImportError("Projeto contém entradas demais.".into()));
    }
    let expanded = archive
        .decompressed_size()
        .ok_or_else(|| ImportError("Tamanho expandido do projeto inválido.".into()))?;
    if expanded > u128::from(MAX_EXPANDED_BYTES) {
        return Err(ImportError("Projeto descompactado excede 512 MiB.".into()));
    }
    let bytes = read_entry(&mut archive, "manifest.json", MAX_MANIFEST_BYTES)?;
    #[derive(Deserialize)]
    struct Header {
        format: String,
        schema_version: u32,
    }
    let header: Header = serde_json::from_slice(&bytes).map_err(error)?;
    if header.format != "3dnator" || !(1..=FORMAT_VERSION).contains(&header.schema_version) {
        return Err(ImportError(format!(
            "Formato ou versão de projeto não suportada: {} / {}.",
            header.format, header.schema_version
        )));
    }
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).map_err(error)?;
    if header.schema_version < 7 {
        migrate_to_7(&mut value)?;
    }
    let manifest: Manifest = serde_json::from_value(value).map_err(error)?;
    let mut expected_names = HashSet::from(["manifest.json".to_owned()]);
    let mut parts = Vec::new();
    let mut total_triangles = 0_usize;
    for part in manifest.parts {
        let mut bodies = Vec::new();
        for body in part.bodies {
            // Só aceitamos nomes derivados do identificador; nenhum caminho externo é seguido.
            if body.mesh_path != mesh_path(body.id)
                || !expected_names.insert(body.mesh_path.clone())
            {
                return Err(ImportError(
                    "Caminho de malha inválido ou repetido no projeto.".into(),
                ));
            }
            let bytes = read_entry(&mut archive, &body.mesh_path, MAX_EXPANDED_BYTES)?;
            let mesh = decode_mesh(&bytes)?;
            total_triangles = total_triangles
                .checked_add(mesh.triangles().len())
                .ok_or_else(|| ImportError("Contagem geométrica excessiva.".into()))?;
            if total_triangles > MAX_SCENE_TRIANGLES {
                return Err(ImportError(
                    "Projeto excede dois milhões de triângulos.".into(),
                ));
            }
            let brep = if let Some(path) = body.brep_path {
                if path != format!("breps/{}.brep", body.id) || !expected_names.insert(path.clone())
                {
                    return Err(ImportError("Caminho B-rep inválido ou repetido.".into()));
                }
                Some(Arc::new(read_entry(&mut archive, &path, 64 * 1024 * 1024)?))
            } else {
                None
            };
            if !body.face_names.is_empty() && brep.is_none() {
                return Err(ImportError("Nomes de faces sem B-rep no projeto.".into()));
            }
            let geometry = if let Some(path) = body.faces_path {
                if path != faces_path(body.id) || !expected_names.insert(path.clone()) {
                    return Err(ImportError(
                        "Caminho de faces inválido ou repetido no projeto.".into(),
                    ));
                }
                let faces = decode_faces(
                    &read_entry(&mut archive, &path, MAX_EXPANDED_BYTES)?,
                    mesh.triangles().len(),
                )?;
                MeshAsset::with_faces(mesh, faces).map_err(error)?
            } else {
                MeshAsset::new(mesh)
            };
            bodies.push(Body {
                color: body.color_srgb.map(RgbColor::new),
                brep,
                face_names: Arc::new(body.face_names),
                id: BodyId(body.id),
                name: body.name,
                visible: body.visible,
                placement: body.placement.into_placement()?,
                geometry: Arc::new(geometry),
            });
        }
        parts.push(Part {
            id: PartId(part.id),
            name: part.name,
            bodies,
            sketches: part
                .sketches
                .into_iter()
                .map(|s| SketchRecord {
                    id: SketchId(s.id),
                    sketch: s.sketch,
                    visible: s.visible,
                })
                .collect(),
            work_planes: part
                .work_planes
                .into_iter()
                .map(|w| WorkPlaneRecord {
                    id: WorkPlaneId(w.id),
                    name: w.name,
                    plane: w.plane,
                    visible: w.visible,
                })
                .collect(),
            parameters: part.parameters,
            features: part.features,
            history_marker: part.history_marker,
            folders: part.folders,
        });
    }
    // A miniatura é opcional e só é conferida quanto ao tamanho e à assinatura PNG.
    if archive.index_for_name(super::thumbnail::PATH).is_some() {
        let bytes = read_entry(
            &mut archive,
            super::thumbnail::PATH,
            super::thumbnail::MAX_BYTES,
        )?;
        if !super::thumbnail::is_png(&bytes) {
            return Err(ImportError("Miniatura do projeto inválida.".into()));
        }
        expected_names.insert(super::thumbnail::PATH.to_owned());
    }
    if archive.len() != expected_names.len()
        || archive.file_names().any(|name| match name {
            Ok(name) => !expected_names.contains(name.as_ref()),
            Err(_) => true,
        })
    {
        return Err(ImportError(
            "Projeto contém entradas não declaradas ou duplicadas.".into(),
        ));
    }
    let instances = manifest
        .instances
        .into_iter()
        .map(|i| {
            Ok(Instance {
                color: i.color_srgb.map(RgbColor::new),
                parent: i.parent.map(PartId),
                id: InstanceId(i.id),
                part: PartId(i.part),
                name: i.name,
                visible: i.visible,
                placement: i.placement.into_placement()?,
            })
        })
        .collect::<Result<Vec<_>, ImportError>>()?;
    let mut document = Document::restore(
        manifest.document_id,
        manifest.name,
        manifest.next_id,
        parts,
        instances,
    )
    .map_err(error)?;
    document
        .restore_groups(
            manifest
                .groups
                .into_iter()
                .map(|g| nator_document::assembly::Group {
                    id: nator_document::GroupId(g.id),
                    name: g.name,
                    members: g.members.into_iter().map(InstanceId).collect(),
                })
                .collect(),
        )
        .map_err(error)?;
    document.restore_sheets(manifest.sheets).map_err(error)?;
    document.set_settings(manifest.settings).map_err(error)?;
    document
        .restore_reference_sketches(
            manifest
                .reference_sketches
                .into_iter()
                .map(|s| SketchRecord {
                    id: SketchId(s.id),
                    sketch: s.sketch,
                    visible: s.visible,
                })
                .collect(),
        )
        .map_err(error)?;
    document.restore_folders(manifest.folders);
    Ok(document)
}

/// Miniatura PNG gravada no projeto, se houver. Não lê a geometria.
pub fn read_thumbnail(path: &Path) -> Result<Option<Vec<u8>>, ImportError> {
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_ARCHIVE_BYTES {
        return Err(ImportError("Projeto excede 256 MiB.".into()));
    }
    let mut archive = ZipArchive::new(file).map_err(error)?;
    if archive.index_for_name(super::thumbnail::PATH).is_none() {
        return Ok(None);
    }
    let bytes = read_entry(
        &mut archive,
        super::thumbnail::PATH,
        super::thumbnail::MAX_BYTES,
    )?;
    Ok(super::thumbnail::is_png(&bytes).then_some(bytes))
}

/// Migração 6 → 7 sobre o JSON do manifesto, antes da desserialização tipada.
///
/// Até a versão 6, extrusões/revoluções guardavam a região como posição na lista ordenada
/// por área, e arredondamento/chanfro/casca guardavam índice + assinatura do B-rep em campos
/// soltos. Essas referências são preservadas literalmente nas variantes `Index`, com o mesmo
/// significado de antes; a próxima edição da operação as converte em referências estáveis.
fn migrate_to_7(manifest: &mut serde_json::Value) -> Result<(), ImportError> {
    use serde_json::{Value, json};
    let invalid = || ImportError("Histórico de versão anterior com formato inesperado.".into());
    let Some(parts) = manifest.get_mut("parts").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for part in parts {
        let Some(features) = part.get_mut("features").and_then(Value::as_array_mut) else {
            continue;
        };
        for feature in features {
            let Some(operation) = feature.get_mut("operation").and_then(Value::as_object_mut)
            else {
                return Err(invalid());
            };
            for (kind, data) in operation.iter_mut() {
                let data = data.as_object_mut().ok_or_else(invalid)?;
                match kind.as_str() {
                    "Extrude" | "Revolve" => {
                        let index = data
                            .get("region")
                            .and_then(Value::as_u64)
                            .ok_or_else(invalid)?;
                        data.insert("region".into(), json!({ "Index": index }));
                    }
                    "Fillet" | "Chamfer" | "Shell" => {
                        let field = if kind == "Shell" { "face" } else { "edge" };
                        let index = data
                            .get(field)
                            .and_then(Value::as_u64)
                            .ok_or_else(invalid)?;
                        let signature = data
                            .remove("signature")
                            .and_then(|s| s.as_u64())
                            .ok_or_else(invalid)?;
                        data.insert(
                            field.into(),
                            json!({ "Index": { "index": index, "signature": signature } }),
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

pub fn write_project(path: &Path, document: &Document) -> Result<(), ImportError> {
    write_archive(path, document, false)
}

pub fn write_recovery(path: &Path, document: &Document) -> Result<(), ImportError> {
    if path.exists() && !is_recovery(path)? {
        return Err(ImportError(
            "A recuperação não pode substituir um projeto normal.".into(),
        ));
    }
    write_archive(path, document, true)
}

fn write_archive(path: &Path, document: &Document, recovery: bool) -> Result<(), ImportError> {
    document.validate().map_err(error)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    // O temporário fica no mesmo volume. A persistência substitui o destino atomicamente.
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    let manifest = Manifest {
        format: "3dnator".into(),
        schema_version: FORMAT_VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        recovery,
        document_id: document.id().into(),
        name: document.name().into(),
        next_id: document.next_id(),
        parts: document
            .parts()
            .iter()
            .map(|p| PartRecord {
                id: p.id.0,
                name: p.name.clone(),
                parameters: p.parameters.clone(),
                features: p.features.clone(),
                history_marker: p.history_marker,
                folders: p.folders,
                sketches: p
                    .sketches
                    .iter()
                    .map(|s| SketchRecordData {
                        id: s.id.0,
                        sketch: s.sketch.clone(),
                        visible: s.visible,
                    })
                    .collect(),
                work_planes: p
                    .work_planes
                    .iter()
                    .map(|w| WorkPlaneRecordData {
                        id: w.id.0,
                        name: w.name.clone(),
                        plane: w.plane,
                        visible: w.visible,
                    })
                    .collect(),
                bodies: p
                    .bodies
                    .iter()
                    .map(|b| BodyRecord {
                        color_srgb: b.color.map(RgbColor::channels),
                        id: b.id.0,
                        name: b.name.clone(),
                        visible: b.visible,
                        placement: b.placement.into(),
                        mesh_path: mesh_path(b.id.0),
                        brep_path: b.brep.as_ref().map(|_| format!("breps/{}.brep", b.id.0)),
                        face_names: b.face_names.to_vec(),
                        faces_path: b.geometry.triangle_faces().map(|_| faces_path(b.id.0)),
                    })
                    .collect(),
            })
            .collect(),
        sheets: document.sheets().to_vec(),
        settings: document.settings(),
        folders: document.folders(),
        reference_sketches: document
            .reference_sketches()
            .iter()
            .map(|s| SketchRecordData {
                id: s.id.0,
                sketch: s.sketch.clone(),
                visible: s.visible,
            })
            .collect(),
        groups: document
            .groups()
            .iter()
            .map(|g| GroupRecord {
                id: g.id.0,
                name: g.name.clone(),
                members: g.members.iter().map(|i| i.0).collect(),
            })
            .collect(),
        instances: document
            .instances()
            .iter()
            .map(|i| InstanceRecord {
                color_srgb: i.color.map(RgbColor::channels),
                parent: i.parent.map(|p| p.0),
                id: i.id.0,
                part: i.part.0,
                name: i.name.clone(),
                visible: i.visible,
                placement: i.placement.into(),
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&manifest).map_err(error)?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(ImportError("Manifesto do projeto excede o limite.".into()));
    }
    {
        let mut archive = ZipWriter::new(temporary.as_file_mut());
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(3));
        archive
            .start_file("manifest.json", options)
            .map_err(error)?;
        archive.write_all(&bytes)?;
        if let Some(png) = super::thumbnail::render(document)? {
            // PNG já é comprimido: armazenar sem nova compressão.
            archive
                .start_file(
                    super::thumbnail::PATH,
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .map_err(error)?;
            archive.write_all(&png)?;
        }
        for body in document.parts().iter().flat_map(|p| &p.bodies) {
            if let Some(bytes) = &body.brep {
                archive
                    .start_file(format!("breps/{}.brep", body.id.0), options)
                    .map_err(error)?;
                archive.write_all(bytes)?;
            }
            if let Some(faces) = body.geometry.triangle_faces() {
                archive
                    .start_file(faces_path(body.id.0), options)
                    .map_err(error)?;
                let mut buffer = Vec::with_capacity(12 + faces.len() * 4);
                buffer.extend_from_slice(FACES_MAGIC);
                buffer.extend_from_slice(&(faces.len() as u32).to_le_bytes());
                for face in faces {
                    buffer.extend_from_slice(&face.to_le_bytes());
                }
                archive.write_all(&buffer)?;
            }
            archive
                .start_file(mesh_path(body.id.0), options)
                .map_err(error)?;
            let mesh = body.geometry.mesh();
            archive.write_all(MESH_MAGIC)?;
            archive.write_all(&(mesh.vertices().len() as u32).to_le_bytes())?;
            archive.write_all(&(mesh.triangles().len() as u32).to_le_bytes())?;
            // Agrupar o payload evita uma chamada ao compressor por coordenada.
            let mut buffer = Vec::with_capacity(64 * 1024);
            for point in mesh.vertices() {
                for coordinate in point {
                    buffer.extend_from_slice(&coordinate.to_le_bytes());
                }
                if buffer.len() >= 64 * 1024 {
                    archive.write_all(&buffer)?;
                    buffer.clear();
                }
            }
            for triangle in mesh.triangles() {
                for index in triangle {
                    buffer.extend_from_slice(&index.to_le_bytes());
                }
                if buffer.len() >= 64 * 1024 {
                    archive.write_all(&buffer)?;
                    buffer.clear();
                }
            }
            archive.write_all(&buffer)?;
        }
        archive.finish().map_err(error)?;
    }
    temporary.as_file_mut().sync_all()?;
    // Ler o pacote temporário verifica CRC, referências e invariantes antes da substituição.
    read_project(temporary.path())?;
    temporary.persist(path).map_err(error)?;
    Ok(())
}

pub fn recovery_directory() -> PathBuf {
    if let Some(path) = std::env::var_os("NATOR_RECOVERY_DIR").filter(|p| !p.is_empty()) {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "windows")]
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(path).join("3dnator/recovery");
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(path) = std::env::var_os("XDG_STATE_HOME").filter(|p| !p.is_empty()) {
            return PathBuf::from(path).join("3dnator/recovery");
        }
        if let Some(path) = std::env::var_os("HOME") {
            return PathBuf::from(path).join(".local/state/3dnator/recovery");
        }
    }
    std::env::temp_dir().join("3dnator-recovery")
}

fn is_recovery(path: &Path) -> Result<bool, ImportError> {
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_ARCHIVE_BYTES {
        return Ok(false);
    }
    let mut archive = ZipArchive::new(file).map_err(error)?;
    let bytes = read_entry(&mut archive, "manifest.json", MAX_MANIFEST_BYTES)?;
    #[derive(Deserialize)]
    struct Marker {
        format: String,
        schema_version: u32,
        #[serde(default)]
        recovery: bool,
    }
    let marker: Marker = serde_json::from_slice(&bytes).map_err(error)?;
    Ok(marker.format == "3dnator"
        && (1..=FORMAT_VERSION).contains(&marker.schema_version)
        && marker.recovery)
}

/// A limpeza nunca remove um arquivo só por sua extensão ou por estar na pasta de recuperação.
pub fn remove_recovery(path: &Path) -> Result<(), ImportError> {
    if !path.exists() {
        return Ok(());
    }
    if is_recovery(path)? {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn recovery_files(directory: &Path) -> Result<Vec<PathBuf>, ImportError> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut files = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "3dnator"))
        .collect::<Vec<_>>();
    files
        .sort_by_cached_key(|p| std::cmp::Reverse(fs::metadata(p).and_then(|m| m.modified()).ok()));
    files.truncate(1000);
    files.retain(|path| is_recovery(path).unwrap_or(false));
    files.truncate(50);
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn migrates_version_one_projects_without_losing_bodies() -> Result<(), Box<dyn Error>> {
        let folder = tempfile::tempdir()?;
        let current = folder.path().join("current.3dnator");
        let old = folder.path().join("old.3dnator");
        let doc = document()?;
        write_project(&current, &doc)?;
        let mut reader = ZipArchive::new(File::open(current)?)?;
        let mut manifest: serde_json::Value = serde_json::from_slice(&read_entry(
            &mut reader,
            "manifest.json",
            MAX_MANIFEST_BYTES,
        )?)?;
        manifest["schema_version"] = serde_json::json!(1);
        for part in manifest["parts"].as_array_mut().ok_or("Peças ausentes")? {
            part.as_object_mut()
                .ok_or("Peça inválida")?
                .remove("sketches");
        }
        let mut writer = ZipWriter::new(File::create(&old)?);
        writer.start_file("manifest.json", SimpleFileOptions::default())?;
        writer.write_all(&serde_json::to_vec(&manifest)?)?;
        for body in doc.parts().iter().flat_map(|p| &p.bodies) {
            let name = mesh_path(body.id.0);
            let bytes = read_entry(&mut reader, &name, MAX_EXPANDED_BYTES)?;
            writer.start_file(name, SimpleFileOptions::default())?;
            writer.write_all(&bytes)?;
        }
        writer.finish()?;
        let loaded = read_project(&old)?;
        assert_eq!(loaded.parts()[0].bodies.len(), 2);
        assert!(loaded.parts()[0].sketches.is_empty());
        Ok(())
    }

    #[test]
    fn migrates_index_references_from_version_six() -> Result<(), Box<dyn Error>> {
        use nator_document::features::{EdgeRef, FaceRef, Feature, Operation, RegionRef};
        let mut manifest = serde_json::json!({"parts": [{"features": [
            {"name": "Extrusão", "output": 5, "operation": {"Extrude":
                {"sketch": 3, "region": 1, "from": "0 mm", "to": "10 mm"}}},
            {"name": "Arredondamento", "output": 6, "operation": {"Fillet":
                {"base": 5, "edge": 7, "signature": 42, "radius": "2 mm"}}},
            {"name": "Casca", "output": 8, "operation": {"Shell":
                {"base": 6, "face": 2, "signature": 9, "thickness": "1 mm"}}}
        ]}]});
        migrate_to_7(&mut manifest)?;
        let features: Vec<Feature> =
            serde_json::from_value(manifest["parts"][0]["features"].clone())?;
        assert!(matches!(
            &features[0].operation,
            Operation::Extrude {
                region: RegionRef::Index(1),
                ..
            }
        ));
        assert!(matches!(
            &features[1].operation,
            Operation::Fillet {
                edge: EdgeRef::Index {
                    index: 7,
                    signature: 42
                },
                ..
            }
        ));
        assert!(matches!(
            &features[2].operation,
            Operation::Shell {
                face: FaceRef::Index {
                    index: 2,
                    signature: 9
                },
                ..
            }
        ));
        // Um histórico corrompido não é migrado pela metade.
        let mut broken = serde_json::json!({"parts": [{"features": [
            {"name": "X", "output": 1, "operation": {"Fillet": {"base": 2, "edge": 3}}}
        ]}]});
        assert!(migrate_to_7(&mut broken).is_err());
        Ok(())
    }

    #[test]
    fn face_names_round_trip_with_the_brep() -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("nomes.3dnator");
        let mut doc = document()?;
        let body = doc.parts()[0].bodies[0].id;
        let names = vec!["b9:start".to_owned(), String::new(), "b9:e4".to_owned()];
        let mesh = doc
            .body(body)
            .ok_or("Corpo ausente")?
            .geometry
            .mesh()
            .clone();
        let faces = (0..mesh.triangles().len() as u32)
            .map(|i| i % 3)
            .collect::<Vec<_>>();
        {
            let target = doc.body_mut(body)?;
            target.brep = Some(Arc::new(b"conteudo-brep".to_vec()));
            target.face_names = Arc::new(names.clone());
            target.geometry = Arc::new(MeshAsset::with_faces(mesh, faces.clone())?);
        }
        write_project(&path, &doc)?;
        let loaded = read_project(&path)?;
        let restored = loaded.body(body).ok_or("Corpo ausente")?;
        assert_eq!(restored.face_names.as_slice(), names.as_slice());
        assert_eq!(restored.geometry.triangle_faces(), Some(faces.as_slice()));
        // Nomes sem B-rep são rejeitados pela validação do documento.
        doc.body_mut(body)?.brep = None;
        assert!(write_project(&directory.path().join("invalido.3dnator"), &doc).is_err());
        Ok(())
    }

    #[test]
    fn recovery_cleanup_never_removes_regular_projects() -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let manual = directory.path().join("manual.3dnator");
        let automatic = directory.path().join("automatic.3dnator");
        let doc = document()?;
        write_project(&manual, &doc)?;
        write_recovery(&automatic, &doc)?;
        assert_eq!(recovery_files(directory.path())?, vec![automatic.clone()]);
        assert!(write_recovery(&manual, &doc).is_err());
        remove_recovery(&manual)?;
        assert!(manual.exists());
        remove_recovery(&automatic)?;
        assert!(!automatic.exists());
        Ok(())
    }
    fn document() -> Result<Document, Box<dyn Error>> {
        let mut doc = Document::new("Projeto de ensaio")?;
        let asset = Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?));
        let (part, _) = doc.add_mesh(None, "Base".into(), Arc::clone(&asset))?;
        let (_, second) = doc.add_mesh(Some(part), "Tampa".into(), asset)?;
        doc.body_mut(second)?.placement = Placement::new([3., 4., 5.], [0., 0., 0.25])?;
        Ok(doc)
    }
    #[test]
    fn round_trip_is_self_contained_and_replaces_existing_project() -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("protótipo.3dnator");
        let doc = document()?;
        write_project(&path, &doc)?;
        let loaded = read_project(&path)?;
        assert_eq!(loaded.id(), doc.id());
        assert_eq!(loaded.parts()[0].bodies.len(), 2);
        assert_eq!(
            loaded.parts()[0].bodies[1].placement,
            doc.parts()[0].bodies[1].placement
        );
        let mut changed = loaded.clone();
        changed.part_mut(loaded.parts()[0].id)?.name = "Variação".into();
        write_project(&path, &changed)?;
        assert_eq!(read_project(&path)?.parts()[0].name, "Variação");
        assert_eq!(
            loaded.parts()[0].bodies[0].geometry.mesh().vertices(),
            doc.parts()[0].bodies[0].geometry.mesh().vertices()
        );
        Ok(())
    }
    #[test]
    fn reference_sketches_round_trip() -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("referencias.3dnator");
        let mut doc = document()?;
        let mut layout = nator_sketch::Sketch::new("Layout", nator_sketch::Plane::xz());
        layout.add_point([10., 20.])?;
        let id = doc.add_reference_sketch(layout)?;
        write_project(&path, &doc)?;
        let reopened = read_project(&path)?;
        assert!(reopened.is_reference_sketch(id));
        assert_eq!(
            reopened
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .points
                .len(),
            1
        );
        Ok(())
    }

    #[test]
    fn angular_sketch_dimensions_round_trip_with_their_expressions() -> Result<(), Box<dyn Error>> {
        use nator_sketch::{ConstraintKind, Curve, Plane, Sketch};
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("angular.3dnator");
        let mut doc = Document::new("Cota angular")?;
        let mut sketch = Sketch::new("Linha", Plane::xy());
        let line = sketch.add_line([0., 0.], [10., 10.], false)?;
        let Curve::Line { start, end } = sketch.entity(line)?.curve else {
            return Err("Linha ausente".into());
        };
        let constraint = sketch.add_constraint(
            ConstraintKind::Angle {
                a: start,
                b: end,
                value: std::f64::consts::FRAC_PI_4,
            },
            false,
        )?;
        sketch
            .dimension_expressions
            .insert(constraint, "45 deg".into());
        let (_, id) = doc.add_sketch(None, sketch)?;
        write_project(&path, &doc)?;
        let reopened = read_project(&path)?;
        let sketch = &reopened.sketch(id).ok_or("Rascunho ausente")?.sketch;
        assert!(
            matches!(sketch.constraints[0].kind,ConstraintKind::Angle {value,..} if (value-std::f64::consts::FRAC_PI_4).abs() < 1e-12)
        );
        assert_eq!(
            sketch
                .dimension_expressions
                .get(&constraint)
                .map(String::as_str),
            Some("45 deg")
        );
        sketch.validate()?;
        Ok(())
    }

    #[test]
    fn infinite_lines_and_relative_angles_round_trip_without_changing_legacy_lines()
    -> Result<(), Box<dyn Error>> {
        use nator_sketch::{ConstraintKind, LineType, Plane, Sketch};
        let mut sketch = Sketch::new("Referências lineares", Plane::xy());
        let reference = sketch.add_line_with_type([0., 0.], [1., 1.], LineType::Infinite)?;
        let line = sketch.add_line_with_type([10., 0.], [0., 10.], LineType::Construction)?;
        let constraint = sketch.add_constraint(
            ConstraintKind::LineAngle {
                line,
                reference,
                value: std::f64::consts::FRAC_PI_2,
            },
            false,
        )?;
        sketch
            .dimension_expressions
            .insert(constraint, "90 deg".into());
        let mut document = Document::new("Linhas de referência")?;
        let (_, id) = document.add_sketch(None, sketch)?;
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("linhas.3dnator");
        write_project(&path, &document)?;
        let reopened = read_project(&path)?;
        let sketch = &reopened.sketch(id).ok_or("Rascunho ausente")?.sketch;
        assert_eq!(sketch.entity(reference)?.line_type(), LineType::Infinite);
        assert_eq!(sketch.entity(line)?.line_type(), LineType::Construction);
        assert!(
            matches!(sketch.constraints[0].kind,ConstraintKind::LineAngle { line:a,reference:b,..} if a==line && b==reference)
        );
        assert_eq!(
            sketch
                .dimension_expressions
                .get(&constraint)
                .map(String::as_str),
            Some("90 deg")
        );
        let mut legacy = Sketch::new("Linha antiga", Plane::xy());
        let old = legacy.add_line([0., 0.], [1., 0.], false)?;
        let value = serde_json::to_value(&legacy)?;
        assert!(value["entities"][0].get("infinite").is_none());
        let decoded: Sketch = serde_json::from_value(value)?;
        assert_eq!(decoded.entity(old)?.line_type(), LineType::Normal);
        Ok(())
    }

    #[test]
    fn folder_visibility_round_trips_and_defaults_when_absent() -> Result<(), Box<dyn Error>> {
        use nator_document::Folder;
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("pastas.3dnator");
        let mut doc = document()?;
        let part = doc.parts()[0].id;
        // Padrão: tudo visível, exceto a origem das peças; nada é gravado.
        assert!(!doc.folder_visible(Folder::PartOrigin(part))?);
        write_project(&path, &doc)?;
        let mut archive = ZipArchive::new(File::open(&path)?)?;
        let mut manifest = String::new();
        archive
            .by_name("manifest.json")?
            .read_to_string(&mut manifest)?;
        assert!(!manifest.contains("\"folders\""));
        doc.set_folder_visible(Folder::Bodies(part), false)?;
        doc.set_folder_visible(Folder::PartOrigin(part), true)?;
        doc.set_folder_visible(Folder::ReferenceSketches, false)?;
        let second = directory.path().join("pastas-2.3dnator");
        write_project(&second, &doc)?;
        let reopened = read_project(&second)?;
        assert!(!reopened.folder_visible(Folder::Bodies(part))?);
        assert!(reopened.folder_visible(Folder::PartOrigin(part))?);
        assert!(!reopened.folder_visible(Folder::ReferenceSketches)?);
        assert!(reopened.folder_visible(Folder::Parts)?);
        Ok(())
    }

    #[test]
    fn work_planes_round_trip_with_their_visibility() -> Result<(), Box<dyn Error>> {
        use nator_document::{Folder, WorkPlane};
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("planos.3dnator");
        let mut doc = document()?;
        let part = doc.parts()[0].id;
        // Projetos sem planos não gravam o campo nem a visibilidade padrão da pasta.
        write_project(&path, &doc)?;
        let mut manifest = String::new();
        ZipArchive::new(File::open(&path)?)?
            .by_name("manifest.json")?
            .read_to_string(&mut manifest)?;
        assert!(!manifest.contains("work_planes") && !manifest.contains("\"planes\""));
        assert!(read_project(&path)?.parts()[0].work_planes.is_empty());
        let base = nator_sketch::Plane::from_normal([1., 2., 3.], [0.3, -0.4, 0.5])?;
        let shown = doc.add_work_plane(
            part,
            "Plano 1".into(),
            WorkPlane {
                base,
                offset: 12.345,
            },
        )?;
        let hidden = doc.add_work_plane(
            part,
            "Plano 2".into(),
            WorkPlane {
                base: nator_sketch::Plane::yz(),
                offset: -7.,
            },
        )?;
        doc.work_plane_mut(hidden)?.visible = false;
        doc.set_folder_visible(Folder::WorkPlanes(part), false)?;
        let second = directory.path().join("planos-2.3dnator");
        write_project(&second, &doc)?;
        let reopened = read_project(&second)?;
        assert_eq!(reopened.parts()[0].work_planes, doc.parts()[0].work_planes);
        assert_eq!(reopened.work_plane_part(shown), Some(part));
        assert!(!reopened.folder_visible(Folder::WorkPlanes(part))?);
        assert!(reopened.next_id() > hidden.0);
        Ok(())
    }

    #[test]
    fn settings_and_thumbnail_round_trip() -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("miniatura.3dnator");
        let mut doc = document()?;
        doc.set_settings(nator_document::Settings {
            display_decimals: 4,
        })?;
        write_project(&path, &doc)?;
        assert_eq!(read_project(&path)?.settings().display_decimals, 4);
        let png = read_thumbnail(&path)?.ok_or("Miniatura ausente")?;
        assert!(png.starts_with(b"\x89PNG"));
        // A miniatura não é preta nem vazia: há pixels do fundo e do modelo.
        let decoder = png::Decoder::new(std::io::Cursor::new(&png));
        let mut reader = decoder.read_info()?;
        let mut pixels = vec![0; reader.output_buffer_size().ok_or("Tamanho ausente")?];
        reader.next_frame(&mut pixels)?;
        let distinct = pixels
            .as_chunks::<3>()
            .0
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert!(distinct > 10);
        // Um documento vazio é salvo sem miniatura.
        let empty = directory.path().join("vazio.3dnator");
        write_project(&empty, &Document::new("Vazio")?)?;
        assert!(read_thumbnail(&empty)?.is_none());
        assert!(
            doc.set_settings(nator_document::Settings {
                display_decimals: 9
            })
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn invalid_document_does_not_destroy_previous_file() -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("project.3dnator");
        let mut doc = document()?;
        write_project(&path, &doc)?;
        let before = fs::read(&path)?;
        let id = doc.parts()[0].id;
        doc.part_mut(id)?.name.clear();
        assert!(write_project(&path, &doc).is_err());
        assert_eq!(fs::read(&path)?, before);
        let mut truncated = before;
        truncated.truncate(20);
        fs::write(directory.path().join("broken.3dnator"), truncated)?;
        assert!(read_project(&directory.path().join("broken.3dnator")).is_err());
        Ok(())
    }
    #[test]
    fn rejects_future_schema_without_following_paths() -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("future.3dnator");
        let mut archive = ZipWriter::new(File::create(&path)?);
        archive.start_file("manifest.json", SimpleFileOptions::default())?;
        archive.write_all(br#"{"format":"3dnator","schema_version":999}"#)?;
        archive.finish()?;
        let error = read_project(&path)
            .err()
            .ok_or("Formato futuro foi aceito")?;
        assert!(error.to_string().contains("versão"));
        Ok(())
    }
}
