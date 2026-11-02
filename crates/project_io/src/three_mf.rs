//! Exportação 3MF Core: um objeto por corpo, em milímetros, com posições na mesa e sem
//! extensões ou configurações de fatiador, para abrir em qualquer aplicativo.
use crate::{ImportError, MAX_TRIANGLES, require_closed};
use nator_geometry::{Mesh, Point3};
use std::{
    fs,
    io::{BufWriter, Seek, Write},
    path::Path,
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
</Types>
"#;
const RELATIONSHIPS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>
</Relationships>
"#;
/// Casas decimais em mm: um nanômetro, bem abaixo do que uma impressora resolve.
const DECIMALS: usize = 6;

/// Corpo e suas cópias na mesa: `items` traz a translação, em mm, de cada uma.
pub struct ThreeMfObject<'a> {
    pub name: &'a str,
    pub mesh: &'a Mesh,
    pub items: &'a [Point3],
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Malha como será lida do arquivo: arredondar e fundir pode colapsar frestas, então a
/// validação precisa ver essas mesmas coordenadas.
fn rounded(mesh: &Mesh) -> Result<Mesh, ImportError> {
    let scale = 10_f64.powi(DECIMALS as i32);
    mesh.quantized(|v| (v * scale).round() / scale + 0.)
        .map(Mesh::welded)
        .map_err(|e| ImportError(e.to_string()))
}

pub fn encode_3mf(
    objects: &[ThreeMfObject<'_>],
    output: impl Write + Seek,
) -> Result<(), ImportError> {
    if objects.is_empty() || objects.iter().any(|o| o.items.is_empty()) {
        return Err(ImportError("Não há objetos para exportar.".into()));
    }
    let triangles = objects
        .iter()
        .map(|o| o.mesh.triangles().len())
        .sum::<usize>();
    if triangles > MAX_TRIANGLES {
        return Err(ImportError(
            "Malha excede dois milhões de triângulos.".into(),
        ));
    }
    // Validar tudo antes de escrever: um objeto ruim não deixa pacote pela metade.
    let meshes = objects
        .iter()
        .map(|object| {
            let mesh = rounded(object.mesh)?;
            require_closed(&mesh).map_err(|e| ImportError(format!("{}: {e}", object.name)))?;
            Ok(mesh)
        })
        .collect::<Result<Vec<_>, ImportError>>()?;
    let failure = |e: zip::result::ZipError| ImportError(e.to_string());
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut archive = ZipWriter::new(output);
    archive
        .start_file("[Content_Types].xml", options)
        .map_err(failure)?;
    archive.write_all(CONTENT_TYPES.as_bytes())?;
    archive
        .start_file("_rels/.rels", options)
        .map_err(failure)?;
    archive.write_all(RELATIONSHIPS.as_bytes())?;
    archive
        .start_file("3D/3dmodel.model", options)
        .map_err(failure)?;
    {
        let mut model = BufWriter::new(&mut archive);
        writeln!(model, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
        writeln!(
            model,
            r#"<model unit="millimeter" xml:lang="pt-BR" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">"#
        )?;
        writeln!(
            model,
            r#"  <metadata name="Application">3dnator {}</metadata>"#,
            env!("CARGO_PKG_VERSION")
        )?;
        writeln!(model, "  <resources>")?;
        for (index, (object, mesh)) in objects.iter().zip(&meshes).enumerate() {
            writeln!(
                model,
                r#"    <object id="{}" type="model" name="{}"><mesh><vertices>"#,
                index + 1,
                escape(object.name)
            )?;
            for [x, y, z] in mesh.vertices() {
                writeln!(
                    model,
                    r#"<vertex x="{x:.DECIMALS$}" y="{y:.DECIMALS$}" z="{z:.DECIMALS$}"/>"#
                )?;
            }
            writeln!(model, "</vertices><triangles>")?;
            for [v1, v2, v3] in mesh.triangles() {
                writeln!(model, r#"<triangle v1="{v1}" v2="{v2}" v3="{v3}"/>"#)?;
            }
            writeln!(model, "</triangles></mesh></object>")?;
        }
        writeln!(model, "  </resources>\n  <build>")?;
        for (index, object) in objects.iter().enumerate() {
            for [x, y, z] in object.items {
                writeln!(
                    model,
                    r#"    <item objectid="{}" transform="1 0 0 0 1 0 0 0 1 {x:.DECIMALS$} {y:.DECIMALS$} {z:.DECIMALS$}"/>"#,
                    index + 1
                )?;
            }
        }
        writeln!(model, "  </build>\n</model>")?;
        model.flush()?;
    }
    archive.finish().map_err(failure)?;
    Ok(())
}

/// Grava em arquivo temporário e publica sem sobrescrever: um destino existente é erro.
/// Limite do pacote 3MF lido, compactado e de cada parte expandida.
const MAX_3MF_BYTES: u64 = 256 * 1024 * 1024;

/// Atributos de uma tag XML, sem o prefixo de namespace no nome da tag.
struct Tag<'a> {
    name: &'a str,
    closing: bool,
    attributes: Vec<(&'a str, String)>,
}

fn unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Percorre as tags do documento. Texto, comentários e instruções são ignorados: o 3MF
/// Core guarda toda a geometria em atributos.
fn tags(xml: &str) -> impl Iterator<Item = Tag<'_>> {
    let mut rest = xml;
    std::iter::from_fn(move || {
        loop {
            let start = rest.find('<')?;
            let end = start + rest[start..].find('>')?;
            let body = &rest[start + 1..end];
            rest = &rest[end + 1..];
            if body.starts_with('?') || body.starts_with('!') {
                continue;
            }
            let closing = body.starts_with('/');
            let body = body.trim_start_matches('/').trim_end_matches('/');
            let (name, mut attributes) = match body.find(char::is_whitespace) {
                Some(i) => (&body[..i], &body[i..]),
                None => (body, ""),
            };
            let name = name.rsplit(':').next().unwrap_or(name);
            let mut parsed = Vec::new();
            while let Some(equal) = attributes.find('=') {
                let key = attributes[..equal].trim();
                let after = attributes[equal + 1..].trim_start();
                let Some(quote) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
                    break;
                };
                let Some(close) = after[1..].find(quote) else {
                    break;
                };
                parsed.push((
                    key.rsplit(':').next().unwrap_or(key),
                    unescape(&after[1..1 + close]),
                ));
                attributes = &after[close + 2..];
            }
            return Some(Tag {
                name,
                closing,
                attributes: parsed,
            });
        }
    })
}

impl Tag<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
    }
    fn number(&self, key: &str) -> Result<f64, ImportError> {
        self.get(key)
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .ok_or_else(|| ImportError(format!("Atributo {key} inválido no 3MF.")))
    }
    fn index(&self, key: &str) -> Result<usize, ImportError> {
        self.get(key)
            .and_then(|v| v.trim().parse::<usize>().ok())
            .ok_or_else(|| ImportError(format!("Atributo {key} inválido no 3MF.")))
    }
}

/// Matriz 3MF (4 × 3, por linhas): `p' = [x y z 1] · M`.
type Transform = [f64; 12];
const IDENTITY: Transform = [1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0.];

fn transform(tag: &Tag<'_>) -> Result<Transform, ImportError> {
    let Some(text) = tag.get("transform") else {
        return Ok(IDENTITY);
    };
    let values = text
        .split_whitespace()
        .map(|v| v.parse::<f64>().ok().filter(|v| v.is_finite()))
        .collect::<Option<Vec<_>>>()
        .filter(|v| v.len() == 12)
        .ok_or_else(|| ImportError("Transformação inválida no 3MF.".into()))?;
    Ok(std::array::from_fn(|i| values[i]))
}

fn apply(m: &Transform, p: Point3) -> Point3 {
    std::array::from_fn(|c| p[0] * m[c] + p[1] * m[3 + c] + p[2] * m[6 + c] + m[9 + c])
}

/// `a` depois de `b`.
fn compose(a: &Transform, b: &Transform) -> Transform {
    let mut out = [0.; 12];
    for row in 0..4 {
        for c in 0..3 {
            let mut value = if row == 3 { a[9 + c] } else { 0. };
            for k in 0..3 {
                value += b[row * 3 + k] * a[k * 3 + c];
            }
            out[row * 3 + c] = value;
        }
    }
    out
}

#[derive(Default)]
struct Object {
    vertices: Vec<Point3>,
    triangles: Vec<[usize; 3]>,
    components: Vec<(String, Transform)>,
}

/// Lê um 3MF Core como uma malha só, em milímetros: os objetos da mesa nas suas posições,
/// sem cores, materiais nem montagem.
pub fn read_3mf(path: &Path) -> Result<Mesh, ImportError> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > MAX_3MF_BYTES {
        return Err(ImportError("3MF excede o limite de 256 MiB.".into()));
    }
    let mut archive = zip::ZipArchive::new(file).map_err(|e| ImportError(e.to_string()))?;
    let mut read = |name: &str| -> Result<Option<String>, ImportError> {
        let Ok(entry) = archive.by_name(name.trim_start_matches('/')) else {
            return Ok(None);
        };
        let mut text = String::new();
        use std::io::Read;
        entry.take(MAX_3MF_BYTES + 1).read_to_string(&mut text)?;
        if text.len() as u64 > MAX_3MF_BYTES {
            return Err(ImportError("Modelo 3MF grande demais.".into()));
        }
        Ok(Some(text))
    };
    // O modelo principal é o alvo da relação do pacote; o caminho usual é o padrão.
    let target = read("_rels/.rels")?
        .and_then(|rels| {
            tags(&rels)
                .find(|t| {
                    t.name == "Relationship"
                        && t.get("Type").is_some_and(|k| k.ends_with("/3dmodel"))
                })
                .and_then(|t| t.get("Target").map(str::to_owned))
        })
        .unwrap_or_else(|| "3D/3dmodel.model".into());
    let xml =
        read(&target)?.ok_or_else(|| ImportError("Pacote 3MF sem o modelo principal.".into()))?;
    decode_3mf_model(&xml)
}

/// Converte o XML do modelo 3MF numa malha em milímetros.
pub fn decode_3mf_model(xml: &str) -> Result<Mesh, ImportError> {
    let mut scale = 1.;
    let mut objects = std::collections::HashMap::<String, Object>::new();
    let mut items = Vec::new();
    let mut current: Option<(String, Object)> = None;
    for tag in tags(xml) {
        match (tag.name, tag.closing) {
            ("model", false) => {
                scale = match tag.get("unit").unwrap_or("millimeter") {
                    "micron" => 0.001,
                    "millimeter" => 1.,
                    "centimeter" => 10.,
                    "inch" => 25.4,
                    "foot" => 304.8,
                    "meter" => 1000.,
                    other => {
                        return Err(ImportError(format!("Unidade 3MF desconhecida: {other}.")));
                    }
                };
            }
            ("object", false) => {
                let id = tag
                    .get("id")
                    .ok_or_else(|| ImportError("Objeto 3MF sem id.".into()))?;
                current = Some((id.to_owned(), Object::default()));
            }
            ("object", true) => {
                if let Some((id, object)) = current.take() {
                    objects.insert(id, object);
                }
            }
            ("vertex", false) => {
                let (_, object) = current
                    .as_mut()
                    .ok_or_else(|| ImportError("Vértice 3MF fora de um objeto.".into()))?;
                object
                    .vertices
                    .push([tag.number("x")?, tag.number("y")?, tag.number("z")?]);
            }
            ("triangle", false) => {
                let (_, object) = current
                    .as_mut()
                    .ok_or_else(|| ImportError("Triângulo 3MF fora de um objeto.".into()))?;
                object
                    .triangles
                    .push([tag.index("v1")?, tag.index("v2")?, tag.index("v3")?]);
                if object.triangles.len() > MAX_TRIANGLES {
                    return Err(ImportError("3MF excede dois milhões de triângulos.".into()));
                }
            }
            ("component", false) => {
                let (_, object) = current
                    .as_mut()
                    .ok_or_else(|| ImportError("Componente 3MF fora de um objeto.".into()))?;
                let id = tag
                    .get("objectid")
                    .ok_or_else(|| ImportError("Componente 3MF sem objeto.".into()))?;
                object.components.push((id.to_owned(), transform(&tag)?));
            }
            ("item", false) => {
                let id = tag
                    .get("objectid")
                    .ok_or_else(|| ImportError("Item 3MF sem objeto.".into()))?;
                items.push((id.to_owned(), transform(&tag)?));
            }
            _ => {}
        }
    }
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    // Componentes aninhados até uma profundidade segura; ciclos são recusados.
    fn emit(
        objects: &std::collections::HashMap<String, Object>,
        id: &str,
        placement: &Transform,
        depth: usize,
        vertices: &mut Vec<Point3>,
        triangles: &mut Vec<[u32; 3]>,
    ) -> Result<(), ImportError> {
        if depth > 16 {
            return Err(ImportError(
                "Componentes 3MF aninhados demais ou em ciclo.".into(),
            ));
        }
        let object = objects
            .get(id)
            .ok_or_else(|| ImportError(format!("Objeto 3MF {id} inexistente.")))?;
        let offset = vertices.len();
        vertices.extend(object.vertices.iter().map(|p| apply(placement, *p)));
        for t in &object.triangles {
            if t.iter().any(|i| *i >= object.vertices.len()) {
                return Err(ImportError(
                    "Triângulo 3MF cita vértice inexistente.".into(),
                ));
            }
            triangles.push(t.map(|i| (offset + i) as u32));
        }
        if triangles.len() > MAX_TRIANGLES {
            return Err(ImportError("3MF excede dois milhões de triângulos.".into()));
        }
        for (child, local) in &object.components {
            emit(
                objects,
                child,
                &compose(placement, local),
                depth + 1,
                vertices,
                triangles,
            )?;
        }
        Ok(())
    }
    for (id, placement) in &items {
        emit(&objects, id, placement, 0, &mut vertices, &mut triangles)?;
    }
    for p in &mut vertices {
        *p = p.map(|v| v * scale);
    }
    // Como no STL, triângulos de área zero não têm superfície e ficam de fora.
    triangles.retain(|t| {
        let [a, b, c] = t.map(|i| vertices[i as usize]);
        nator_geometry::norm(nator_geometry::cross(
            nator_geometry::sub(b, a),
            nator_geometry::sub(c, a),
        )) > 0.
    });
    if triangles.is_empty() {
        return Err(ImportError(
            "O 3MF não tem objetos com malha na mesa.".into(),
        ));
    }
    Ok(Mesh::new(vertices, triangles)
        .map_err(|e| ImportError(e.to_string()))?
        .welded())
}

pub fn write_3mf(path: &Path, objects: &[ThreeMfObject<'_>]) -> Result<(), ImportError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !parent.is_dir() {
        return Err(ImportError("A pasta de destino não existe.".into()));
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    encode_3mf(objects, temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(path)
        .map_err(|e| ImportError(e.to_string()))?;
    // O temporário nasce privado; o arquivo final segue as permissões de um STL exportado.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o644))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{error::Error, io::Read};
    use zip::ZipArchive;

    fn tetrahedron(flip: bool) -> Result<Mesh, Box<dyn Error>> {
        let mut faces = vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]];
        if flip {
            faces.iter_mut().for_each(|f| f.swap(1, 2));
        }
        Ok(Mesh::new(
            vec![[0., 0., 0.], [10., 0., 0.], [0., 10., 0.], [0., 0., 10.]],
            faces,
        )?)
    }
    fn entry(bytes: Vec<u8>, name: &str) -> Result<String, Box<dyn Error>> {
        let mut archive = ZipArchive::new(std::io::Cursor::new(bytes))?;
        let mut text = String::new();
        archive.by_name(name)?.read_to_string(&mut text)?;
        Ok(text)
    }

    #[test]
    fn package_has_core_parts_units_escaped_names_and_build_items() -> Result<(), Box<dyn Error>> {
        let mesh = tetrahedron(false)?;
        let mut bytes = std::io::Cursor::new(Vec::new());
        encode_3mf(
            &[ThreeMfObject {
                name: "Peça <A> & \"B\"",
                mesh: &mesh,
                items: &[[0., 0., 0.], [15., 0., 0.]],
            }],
            &mut bytes,
        )?;
        let bytes = bytes.into_inner();
        assert!(entry(bytes.clone(), "[Content_Types].xml")?.contains("3dmodel+xml"));
        assert!(entry(bytes.clone(), "_rels/.rels")?.contains("/3D/3dmodel.model"));
        let model = entry(bytes, "3D/3dmodel.model")?;
        assert!(model.contains(r#"unit="millimeter""#));
        assert!(model.contains("Peça &lt;A&gt; &amp; &quot;B&quot;"));
        assert_eq!(model.matches("<vertex ").count(), 4);
        assert_eq!(model.matches("<triangle ").count(), 4);
        assert_eq!(model.matches("<item ").count(), 2);
        assert!(model.contains("1 0 0 0 1 0 0 0 1 15.000000 0.000000 0.000000"));
        Ok(())
    }

    #[test]
    fn written_packages_read_back_with_every_item_in_place() -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("ida-e-volta.3mf");
        let mesh = tetrahedron(false)?;
        write_3mf(
            &path,
            &[ThreeMfObject {
                name: "Corpo",
                mesh: &mesh,
                items: &[[0., 0., 0.], [15., 0., 0.]],
            }],
        )?;
        let read = read_3mf(&path)?;
        assert_eq!(read.triangles().len(), 8);
        let bounds = read.bounds();
        assert_eq!(bounds.min, [0., 0., 0.]);
        assert_eq!(bounds.max, [25., 10., 10.]);
        Ok(())
    }

    #[test]
    fn units_and_components_become_millimeters_in_place() -> Result<(), Box<dyn Error>> {
        let model = r#"<?xml version="1.0"?>
<model unit="inch" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
 <resources>
  <object id="1" type="model"><mesh>
   <vertices><vertex x="0" y="0" z="0"/><vertex x="1" y="0" z="0"/>
    <vertex x="0" y="1" z="0"/><vertex x="0" y="0" z="1"/></vertices>
   <triangles><triangle v1="0" v2="2" v3="1"/><triangle v1="0" v2="1" v3="3"/>
    <triangle v1="1" v2="2" v3="3"/><triangle v1="0" v2="3" v3="2"/>
    <triangle v1="0" v2="0" v3="1"/></triangles>
  </mesh></object>
  <object id="2" type="model"><components>
   <component objectid="1" transform="1 0 0 0 1 0 0 0 1 2 0 0"/>
  </components></object>
 </resources>
 <build><item objectid="2" transform="1 0 0 0 1 0 0 0 1 0 0 1"/></build>
</model>"#;
        let mesh = decode_3mf_model(model)?;
        // O triângulo de área zero sai; o resto chega em mm, deslocado pelo item e pelo
        // componente.
        assert_eq!(mesh.triangles().len(), 4);
        let bounds = mesh.bounds();
        let close = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
        assert!(close(bounds.min, [50.8, 0., 25.4]), "{:?}", bounds.min);
        assert!(close(bounds.max, [76.2, 25.4, 50.8]), "{:?}", bounds.max);
        assert!(decode_3mf_model("<model unit=\"parsec\"></model>").is_err());
        Ok(())
    }

    #[test]
    fn open_or_inverted_bodies_are_refused_with_their_name() -> Result<(), Box<dyn Error>> {
        let open = Mesh::new(
            vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            vec![[0, 1, 2]],
        )?;
        let inverted = tetrahedron(true)?;
        for mesh in [&open, &inverted] {
            let message = encode_3mf(
                &[ThreeMfObject {
                    name: "Corpo",
                    mesh,
                    items: &[[0.; 3]],
                }],
                std::io::Cursor::new(Vec::new()),
            )
            .err()
            .ok_or("Sem erro")?
            .to_string();
            assert!(message.starts_with("Corpo: "), "{message}");
        }
        Ok(())
    }

    #[test]
    fn existing_destinations_are_never_overwritten() -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("modelo.3mf");
        let mesh = tetrahedron(false)?;
        let objects = [ThreeMfObject {
            name: "Corpo",
            mesh: &mesh,
            items: &[[0.; 3]],
        }];
        write_3mf(&path, &objects)?;
        let first = fs::read(&path)?;
        assert!(write_3mf(&path, &objects).is_err());
        assert_eq!(fs::read(&path)?, first);
        assert_eq!(fs::read_dir(directory.path())?.count(), 1);
        Ok(())
    }
}
