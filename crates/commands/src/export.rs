//! Planejamento da exportação STL para impressão 3D, sem gravar nada.
//!
//! Regras do README:
//! - a unidade padrão é o corpo: um arquivo por corpo, em coordenadas locais da peça;
//! - a exportação conjunta gera um único arquivo com as posições de montagem, sem união
//!   booleana entre os corpos;
//! - cada corpo de uma definição compartilhada é exportado uma vez, informando quantas
//!   ocorrências ele tem; opcionalmente, todas as ocorrências nas posições de montagem;
//! - para sólidos CAD, a malha vem da geometria precisa com a qualidade escolhida, nunca
//!   da malha de exibição; corpos de malha são exportados como estão;
//! - nomes de arquivo combinam peça e corpo e nunca repetem um nome do mesmo plano.
use nator_document::{BodyId, Document, DocumentError, PartId};
use nator_geometry::{Bounds, Mesh, Placement, Point3, add, cross, dot, sub};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
pub enum StepScope {
    Body(BodyId),
    Part(PartId),
    Assembly,
}

/// Constrói o intercâmbio sobre B-reps precisos. Malhas visíveis geram erro explícito;
/// omiti-las ou convertê-las silenciosamente mudaria o conteúdo exportado.
#[cfg(feature = "occt")]
pub fn step_scene(
    doc: &Document,
    scope: StepScope,
) -> Result<nator_kernel::StepAssembly, DocumentError> {
    use nator_kernel::{StepAssembly, StepOccurrence, StepPart};
    let body_data = |body: &nator_document::Body| crate::appearance::step_body(doc, body.id);
    if let StepScope::Body(id) = scope {
        let body = doc
            .body(id)
            .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
        return Ok(StepAssembly {
            parts: vec![StepPart {
                name: body.name.clone(),
                bodies: vec![body_data(body)?],
            }],
            instances: vec![StepOccurrence {
                part: 0,
                parent: None,
                name: body.name.clone(),
                placement: Placement::default(),
                color: None,
            }],
        });
    }
    doc.validate()?;
    fn keep(
        doc: &Document,
        part: PartId,
        retained: &mut HashMap<PartId, bool>,
    ) -> Result<bool, DocumentError> {
        if let Some(found) = retained.get(&part) {
            return Ok(*found);
        }
        let piece = doc
            .part(part)
            .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
        let mut visible = piece.shown_bodies().next().is_some();
        for child in doc
            .instances()
            .iter()
            .filter(|i| i.parent == Some(part) && doc.shows_instance(i))
        {
            visible |= keep(doc, child.part, retained)?;
        }
        retained.insert(part, visible);
        Ok(visible)
    }
    let roots = match scope {
        StepScope::Part(part) => vec![(
            part,
            Placement::default(),
            doc.part(part)
                .ok_or_else(|| DocumentError("Peça inexistente.".into()))?
                .name
                .clone(),
            None,
        )],
        _ => doc
            .instances()
            .iter()
            .filter(|i| i.parent.is_none() && doc.shows_instance(i))
            .map(|i| (i.part, i.placement, i.name.clone(), i.color))
            .collect(),
    };
    let mut retained = HashMap::new();
    for (part, _, _, _) in &roots {
        keep(doc, *part, &mut retained)?;
    }
    let selected = doc
        .parts()
        .iter()
        .filter(|p| retained.get(&p.id) == Some(&true))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(DocumentError("O escopo não contém corpos visíveis.".into()));
    }
    let indices = selected
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id, i))
        .collect::<HashMap<_, _>>();
    let parts = selected
        .iter()
        .map(|p| {
            Ok(StepPart {
                name: p.name.clone(),
                bodies: p
                    .shown_bodies()
                    .map(body_data)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, DocumentError>>()?;
    let mut instances = Vec::new();
    for (part, placement, name, color) in roots {
        if let Some(index) = indices.get(&part) {
            instances.push(StepOccurrence {
                part: *index,
                parent: None,
                name,
                placement,
                color,
            });
        }
    }
    for i in doc.instances().iter().filter(|i| doc.shows_instance(i)) {
        if let Some(parent) = i.parent.and_then(|p| indices.get(&p))
            && let Some(part) = indices.get(&i.part)
        {
            instances.push(StepOccurrence {
                part: *part,
                parent: Some(*parent),
                name: i.name.clone(),
                placement: i.placement,
                color: i.color,
            });
        }
    }
    Ok(StepAssembly { parts, instances })
}

/// Tolerâncias de tesselação da exportação.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quality {
    Draft,
    Standard,
    High,
    /// Desvio linear (mm) e angular (rad) informados pelo usuário.
    Custom {
        linear_mm: f64,
        angular_rad: f64,
    },
}
impl Quality {
    /// Desvio linear em mm e angular em rad.
    pub fn tolerances(self) -> (f64, f64) {
        match self {
            Self::Draft => (0.1, 0.5),
            Self::Standard => (0.02, 0.25),
            Self::High => (0.005, 0.1),
            Self::Custom {
                linear_mm,
                angular_rad,
            } => (linear_mm, angular_rad),
        }
    }
}

/// O que exportar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scope {
    /// Um corpo, em coordenadas locais da peça.
    Body(BodyId),
    /// Cada corpo visível da peça em arquivo próprio, em coordenadas locais da peça.
    Part(PartId),
    /// Cada corpo visível de cada definição, uma vez, em coordenadas locais da peça.
    Definitions,
    /// Todos os corpos visíveis em um único arquivo, nas posições de montagem.
    Joined,
    /// Um arquivo por ocorrência de cada corpo, nas posições de montagem.
    AllOccurrences,
}

/// Arquivo planejado.
#[derive(Debug, Clone)]
pub struct File {
    pub name: String,
    pub mesh: Mesh,
    /// Quantas vezes a geometria aparece na montagem (1 para ocorrências e conjunto).
    pub occurrences: usize,
    /// Direção que aponta para a mesa no referencial de `mesh`, se o corpo tiver uma.
    pub down: Option<Point3>,
    /// Corpo sem B-rep: malha ou superfície importada, que pode não fechar um volume.
    pub surface: bool,
}

/// Distância entre objetos na mesa, em mm.
const BED_GAP: f64 = 5.;

/// Arquivo pronto para a mesa: base em Z = 0 e canto mínimo em XY na origem da malha.
/// `items` traz a posição de cada cópia na mesa.
#[derive(Debug, Clone)]
pub struct Placed {
    pub name: String,
    pub mesh: Mesh,
    pub items: Vec<Point3>,
    pub size: Point3,
}
impl Placed {
    /// Malha na posição da primeira cópia: formatos sem cópias, como o STL.
    pub fn positioned(&self) -> Result<Mesh, DocumentError> {
        let offset = self.items.first().copied().unwrap_or([0.; 3]);
        Mesh::new(
            self.mesh
                .vertices()
                .iter()
                .map(|p| add(*p, offset))
                .collect(),
            self.mesh.triangles().to_vec(),
        )
        .map_err(|e| DocumentError(e.to_string()))
    }
}

/// Resultado da distribuição. Excessos são avisos: quem decide é o usuário, e a mesa do
/// perfil pode ser apenas uma das que ele tem.
#[derive(Debug, Clone)]
pub struct BedLayout {
    pub files: Vec<Placed>,
    /// Objetos que não cabem na mesa nem sozinhos.
    pub oversized: Vec<String>,
    /// O conjunto ocupa mais que uma mesa.
    pub overflow: bool,
}

/// O que será gravado: `layout` traz os corpos imprimíveis, e `skipped` os que ficaram de fora.
#[derive(Debug, Clone)]
pub struct PrintPlan {
    pub files: Vec<File>,
    pub layout: BedLayout,
    /// Superfícies abertas, que não têm volume para imprimir.
    pub skipped: Vec<String>,
    /// Nome e tamanho do conjunto, quando os corpos formam uma única peça na mesa.
    pub group: Option<(String, Point3)>,
}

/// Rotação mínima que leva `down` a -Z. Direção oposta gira meia-volta em X.
fn onto_bed(down: Point3, point: Point3) -> Point3 {
    let target = [0., 0., -1.];
    let cosine = dot(down, target);
    if cosine > 1. - 1e-12 {
        point
    } else if cosine < -1. + 1e-12 {
        [point[0], -point[1], -point[2]]
    } else {
        let axis = cross(down, target);
        let along = dot(axis, point) / (1. + cosine);
        add(
            add(point.map(|v| v * cosine), cross(axis, point)),
            axis.map(|v| v * along),
        )
    }
}

/// Malha girada e apoiada: sem `down`, conserva a orientação da modelagem.
pub(crate) fn on_bed(mesh: &Mesh, down: Option<Point3>) -> Result<Mesh, DocumentError> {
    let error = |e: nator_geometry::GeometryError| DocumentError(e.to_string());
    let turned = Mesh::new(
        mesh.vertices()
            .iter()
            .map(|p| down.map_or(*p, |d| onto_bed(d, *p)))
            .collect(),
        mesh.triangles().to_vec(),
    )
    .map_err(error)?;
    let corner = turned.bounds().min;
    Mesh::new(
        turned.vertices().iter().map(|p| sub(*p, corner)).collect(),
        turned.triangles().to_vec(),
    )
    .map_err(error)
}

/// Apoia cada arquivo e distribui as cópias em fileiras. Com mesa, o conjunto fica centrado
/// nela; sem mesa, na origem, onde os fatiadores recebem modelos sem posição definida.
pub fn prepare_for_bed(files: &[File], bed: Option<Point3>) -> Result<BedLayout, DocumentError> {
    let width = bed.map_or(f64::INFINITY, |b| b[0]);
    let (mut x, mut y, mut row) = (0_f64, 0_f64, 0_f64);
    let mut extent = [0_f64; 2];
    let mut placed = Vec::with_capacity(files.len());
    let mut oversized = Vec::new();
    for file in files {
        let mesh = on_bed(&file.mesh, file.down)?;
        let size = mesh.bounds().size();
        if bed.is_some_and(|b| (0..3).any(|i| size[i] > b[i])) {
            oversized.push(file.name.clone());
        }
        let mut items = Vec::with_capacity(file.occurrences);
        for _ in 0..file.occurrences.max(1) {
            if x > 0. && x + size[0] > width {
                (x, y, row) = (0., y + row + BED_GAP, 0.);
            }
            items.push([x, y, 0.]);
            x += size[0] + BED_GAP;
            row = row.max(size[1]);
            extent = [extent[0].max(x - BED_GAP), extent[1].max(y + row)];
        }
        placed.push(Placed {
            name: file.name.clone(),
            mesh,
            items,
            size,
        });
    }
    let offset = match bed {
        Some(b) => [(b[0] - extent[0]) / 2., (b[1] - extent[1]) / 2., 0.],
        None => [-extent[0] / 2., -extent[1] / 2., 0.],
    };
    for file in &mut placed {
        file.items.iter_mut().for_each(|p| *p = add(*p, offset));
    }
    Ok(BedLayout {
        files: placed,
        oversized,
        overflow: bed.is_some_and(|b| extent[0] > b[0] || extent[1] > b[1]),
    })
}

/// Corpos juntos na mesa, sem união nem mudança de posição relativa. Só a translação do
/// conjunto muda: cada corpo continua sendo um sólido próprio.
fn prepare_group(files: &[File], name: &str, bed: Option<Point3>) -> BedLayout {
    let (mut min, mut max) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for bounds in files.iter().map(|f| f.mesh.bounds()) {
        min = std::array::from_fn(|i| min[i].min(bounds.min[i]));
        max = std::array::from_fn(|i| max[i].max(bounds.max[i]));
    }
    let size = sub(max, min);
    let target = match bed {
        Some(b) => [(b[0] - size[0]) / 2., (b[1] - size[1]) / 2., 0.],
        None => [-size[0] / 2., -size[1] / 2., 0.],
    };
    let oversized = bed.is_some_and(|b| (0..3).any(|i| size[i] > b[i]));
    BedLayout {
        files: unchanged(files, Some(sub(target, min))),
        oversized: oversized.then(|| name.to_owned()).into_iter().collect(),
        overflow: false,
    }
}

fn unchanged(files: &[File], offset: Option<Point3>) -> Vec<Placed> {
    files
        .iter()
        .map(|file| Placed {
            name: file.name.clone(),
            mesh: file.mesh.clone(),
            items: vec![offset.unwrap_or([0.; 3])],
            size: file.mesh.bounds().size(),
        })
        .collect()
}

/// Planeja a exportação para impressão. A montagem mantém um arquivo por corpo por baixo:
/// juntar tudo em uma malha faria de cada encosto entre peças um falso defeito. Superfícies
/// abertas ficam de fora e são informadas; um sólido com defeito continua sendo erro de quem
/// grava, nunca é omitido.
pub fn print_plan(
    doc: &Document,
    scope: Scope,
    quality: Quality,
    prepare: bool,
    bed: Option<Point3>,
) -> Result<PrintPlan, DocumentError> {
    print_plan_of(doc, scope, quality, prepare, bed, None)
}

/// Como `print_plan`, limitado a um trecho quando `excerpt` for informado.
pub fn print_plan_of(
    doc: &Document,
    scope: Scope,
    quality: Quality,
    prepare: bool,
    bed: Option<Point3>,
    excerpt: Option<Excerpt>,
) -> Result<PrintPlan, DocumentError> {
    let joined = scope == Scope::Joined;
    let (planned, meshes) = match excerpt {
        Some(excerpt) => excerpt_files(doc, scope, quality, excerpt)?,
        None => (
            plan(
                doc,
                if joined { Scope::AllOccurrences } else { scope },
                quality,
            )?,
            Vec::new(),
        ),
    };
    let (open, files): (Vec<_>, Vec<_>) = planned.into_iter().partition(|file| {
        file.surface && (file.mesh.topology().boundary_edges > 0 || file.mesh.signed_volume() <= 0.)
    });
    let mut skipped = open.into_iter().map(|file| file.name).collect::<Vec<_>>();
    skipped.extend(meshes);
    if files.is_empty() {
        return Err(DocumentError(if skipped.is_empty() {
            "Não há corpos visíveis para exportar.".into()
        } else {
            format!(
                "Só há superfícies abertas, sem volume para imprimir: {}.",
                skipped.join(", ")
            )
        }));
    }
    let name = file_stem(doc.name());
    let layout = match (prepare, joined) {
        (true, true) => prepare_group(&files, &name, bed),
        (true, false) => prepare_for_bed(&files, bed)?,
        (false, _) => BedLayout {
            files: unchanged(&files, None),
            oversized: Vec::new(),
            overflow: false,
        },
    };
    let group = joined.then(|| {
        let (mut min, mut max) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for bounds in files.iter().map(|f| f.mesh.bounds()) {
            min = std::array::from_fn(|i| min[i].min(bounds.min[i]));
            max = std::array::from_fn(|i| max[i].max(bounds.max[i]));
        }
        (name, sub(max, min))
    });
    Ok(PrintPlan {
        files,
        layout,
        skipped,
        group,
    })
}

/// Tamanho de `body` com a face escolhida na mesa, sem tesselar de novo.
pub fn size_on_bed(doc: &Document, body: BodyId, down: Option<Point3>) -> Option<Point3> {
    let record = doc.body(body)?;
    let local = transformed(record.geometry.mesh(), record.placement).ok()?;
    let down = down.map(|d| record.placement.apply_vector(d));
    Some(on_bed(&local, down).ok()?.bounds().size())
}

fn is_surface(doc: &Document, body: BodyId) -> bool {
    doc.body(body).is_some_and(|record| record.brep.is_none())
}

/// Direção da mesa no referencial da peça.
fn body_down(doc: &Document, body: BodyId) -> Option<Point3> {
    let record = doc.body(body)?;
    record.print_down.map(|d| record.placement.apply_vector(d))
}

/// Resumo exibido antes de exportar.
#[derive(Debug, Clone, Copy)]
pub struct Summary {
    pub files: usize,
    pub triangles: usize,
    /// Tamanho exato dos STL binários: 84 bytes de cabeçalho mais 50 por triângulo.
    pub bytes: u64,
    pub bounds: Option<Bounds>,
}

pub fn summarize(files: &[File]) -> Summary {
    let triangles = files.iter().map(|f| f.mesh.triangles().len()).sum();
    let bounds = files.iter().map(|f| f.mesh.bounds()).reduce(|a, b| Bounds {
        min: std::array::from_fn(|i| a.min[i].min(b.min[i])),
        max: std::array::from_fn(|i| a.max[i].max(b.max[i])),
    });
    Summary {
        files: files.len(),
        triangles,
        bytes: files
            .iter()
            .map(|f| 84 + 50 * f.mesh.triangles().len() as u64)
            .sum(),
        bounds,
    }
}

/// Nome de arquivo seguro em Linux e Windows, sem separadores nem caracteres reservados.
fn file_stem(text: &str) -> String {
    let cleaned = text
        .chars()
        .map(|c| {
            if c.is_control() || "/\\<>:\"|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect::<String>();
    let trimmed = cleaned.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        "corpo".into()
    } else {
        trimmed.chars().take(120).collect()
    }
}

/// Garante nomes distintos dentro do plano, sem diferenciar maiúsculas (Windows).
fn unique(name: String, used: &mut HashSet<String>) -> String {
    let mut candidate = name.clone();
    let mut counter = 2;
    while !used.insert(candidate.to_lowercase()) {
        candidate = format!("{name}_{counter}");
        counter += 1;
    }
    candidate
}

/// Trecho de uma peça para testar um encaixe sem imprimir tudo: caixa em coordenadas da peça.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Excerpt {
    pub min: Point3,
    pub max: Point3,
}

/// Caixa envolvente do corpo ou da peça, ponto de partida do trecho.
pub fn scope_bounds(doc: &Document, scope: Scope) -> Option<Excerpt> {
    let bodies = match scope {
        Scope::Body(body) => vec![doc.body(body)?],
        Scope::Part(part) => doc.part(part)?.shown_bodies().collect(),
        _ => return None,
    };
    let (mut min, mut max) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for body in bodies {
        let bounds = transformed(body.geometry.mesh(), body.placement)
            .ok()?
            .bounds();
        min = std::array::from_fn(|i| min[i].min(bounds.min[i]));
        max = std::array::from_fn(|i| max[i].max(bounds.max[i]));
    }
    min.iter()
        .all(|v| v.is_finite())
        .then_some(Excerpt { min, max })
}

/// Arquivos do trecho e os corpos de malha deixados de fora: sem B-rep não há corte preciso.
fn excerpt_files(
    doc: &Document,
    scope: Scope,
    quality: Quality,
    excerpt: Excerpt,
) -> Result<(Vec<File>, Vec<String>), DocumentError> {
    if (0..3).any(|i| excerpt.max[i] <= excerpt.min[i]) {
        return Err(DocumentError("O trecho precisa ter tamanho.".into()));
    }
    let part_of = |body: BodyId| {
        doc.parts()
            .iter()
            .find(|p| p.bodies.iter().any(|b| b.id == body))
    };
    let (part, bodies) = match scope {
        Scope::Body(body) => (part_of(body), vec![body]),
        Scope::Part(part) => {
            let part = doc.part(part);
            let bodies = part.map(|p| p.shown_bodies().map(|b| b.id).collect());
            (part, bodies.unwrap_or_default())
        }
        _ => {
            return Err(DocumentError(
                "O trecho vale para um corpo ou uma peça.".into(),
            ));
        }
    };
    let part = part.ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
    let (mut used, mut files, mut meshes) = (HashSet::new(), Vec::new(), Vec::new());
    for body in bodies {
        let record = doc
            .body(body)
            .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
        let name = file_stem(&format!("{}_{}_trecho", part.name, record.name));
        if record.brep.is_none() {
            meshes.push(name);
            continue;
        }
        if let Some(mesh) = excerpt_mesh(record, quality, excerpt)? {
            files.push(File {
                name: unique(name, &mut used),
                mesh,
                occurrences: 1,
                down: body_down(doc, body),
                surface: false,
            });
        }
    }
    if files.is_empty() {
        return Err(DocumentError("O trecho não alcança nenhum corpo.".into()));
    }
    Ok((files, meshes))
}

#[cfg(feature = "occt")]
fn excerpt_mesh(
    record: &nator_document::Body,
    quality: Quality,
    excerpt: Excerpt,
) -> Result<Option<Mesh>, DocumentError> {
    use nator_kernel::{KernelError, Solid};
    /// Volume comum abaixo disto é um corpo que só encosta na caixa.
    const EMPTY_MM3: f64 = 1e-6;
    let (linear, angular) = quality.tolerances();
    let clipped = || -> Result<Option<Mesh>, KernelError> {
        let bytes = record
            .brep
            .as_ref()
            .ok_or_else(|| KernelError("Corpo sem sólido.".into()))?;
        let solid = Solid::from_brep(bytes, None)?.transformed(record.placement)?;
        let size = std::array::from_fn(|i| excerpt.max[i] - excerpt.min[i]);
        let frame = Solid::cuboid(excerpt.min, size)?;
        if solid.common_volume(&frame)? < EMPTY_MM3 {
            return Ok(None);
        }
        let mut piece = solid.boolean(&frame, 2, "b0")?;
        piece.tessellate(linear, angular).map(Some)
    };
    clipped().map_err(|e| DocumentError(e.to_string()))
}
#[cfg(not(feature = "occt"))]
fn excerpt_mesh(
    _record: &nator_document::Body,
    _quality: Quality,
    _excerpt: Excerpt,
) -> Result<Option<Mesh>, DocumentError> {
    nator_kernel::require_available().map_err(|e| DocumentError(e.to_string()))?;
    Ok(None)
}

/// Malha de exportação de um corpo, em coordenadas locais da peça.
fn body_mesh(doc: &Document, body: BodyId, quality: Quality) -> Result<Mesh, DocumentError> {
    let record = doc
        .body(body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let (linear, angular) = quality.tolerances();
    if !(linear.is_finite() && angular.is_finite() && linear > 0. && angular > 0.) {
        return Err(DocumentError(
            "Tolerâncias de exportação devem ser positivas.".into(),
        ));
    }
    let local = match &record.brep {
        #[cfg(feature = "occt")]
        Some(bytes) => {
            let mut solid = nator_kernel::Solid::from_brep(bytes, None)
                .map_err(|e| DocumentError(e.to_string()))?;
            solid
                .tessellate(linear, angular)
                .map_err(|e| DocumentError(e.to_string()))?
        }
        _ => record.geometry.mesh().clone(),
    };
    transformed(&local, record.placement)
}

pub(crate) fn transformed(mesh: &Mesh, placement: Placement) -> Result<Mesh, DocumentError> {
    Mesh::new(
        mesh.vertices()
            .iter()
            .map(|p| placement.apply(*p))
            .collect(),
        mesh.triangles().to_vec(),
    )
    .map_err(|e| DocumentError(e.to_string()))
}

fn join(meshes: &[Mesh]) -> Result<Mesh, DocumentError> {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for mesh in meshes {
        let base = u32::try_from(vertices.len())
            .map_err(|_| DocumentError("Malha conjunta grande demais.".into()))?;
        vertices.extend_from_slice(mesh.vertices());
        triangles.extend(mesh.triangles().iter().map(|t| t.map(|i| i + base)));
    }
    Mesh::new(vertices, triangles).map_err(|e| DocumentError(e.to_string()))
}

/// Planeja os arquivos de uma exportação. Nada é gravado.
pub fn plan(doc: &Document, scope: Scope, quality: Quality) -> Result<Vec<File>, DocumentError> {
    let mut used = HashSet::new();
    let mut files = Vec::new();
    let occurrences = doc.occurrences()?;
    // Ocorrências de cada definição na montagem (instâncias visíveis em toda a hierarquia).
    let mut count: HashMap<PartId, usize> = HashMap::new();
    for occurrence in &occurrences {
        if let Some(instance) = doc.instance(occurrence.instance) {
            *count.entry(instance.part).or_default() += 1;
        }
    }
    let name_of = |part: PartId, body: BodyId| -> String {
        let part = doc.part(part).map(|p| p.name.as_str()).unwrap_or("peça");
        let body = doc.body(body).map(|b| b.name.as_str()).unwrap_or("corpo");
        file_stem(&format!("{part}_{body}"))
    };
    let visible_bodies = |part: PartId| -> Vec<BodyId> {
        doc.part(part)
            .map(|p| p.shown_bodies().map(|b| b.id).collect())
            .unwrap_or_default()
    };
    match scope {
        Scope::Body(body) => {
            let part = doc
                .parts()
                .iter()
                .find(|p| p.bodies.iter().any(|b| b.id == body))
                .map(|p| p.id)
                .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
            files.push(File {
                name: unique(name_of(part, body), &mut used),
                mesh: body_mesh(doc, body, quality)?,
                occurrences: count.get(&part).copied().unwrap_or(0).max(1),
                down: body_down(doc, body),
                surface: is_surface(doc, body),
            });
        }
        Scope::Part(part) => {
            for body in visible_bodies(part) {
                files.push(File {
                    name: unique(name_of(part, body), &mut used),
                    mesh: body_mesh(doc, body, quality)?,
                    occurrences: count.get(&part).copied().unwrap_or(0).max(1),
                    down: body_down(doc, body),
                    surface: is_surface(doc, body),
                });
            }
        }
        Scope::Definitions => {
            for part in doc.parts().iter().filter(|p| count.contains_key(&p.id)) {
                for body in visible_bodies(part.id) {
                    files.push(File {
                        name: unique(name_of(part.id, body), &mut used),
                        mesh: body_mesh(doc, body, quality)?,
                        occurrences: count[&part.id],
                        down: body_down(doc, body),
                        surface: is_surface(doc, body),
                    });
                }
            }
        }
        Scope::Joined | Scope::AllOccurrences => {
            // Cada definição é tesselada uma vez e reposicionada em cada ocorrência.
            let mut cache: HashMap<BodyId, Mesh> = HashMap::new();
            let mut placed = Vec::new();
            for occurrence in &occurrences {
                let Some(instance) = doc.instance(occurrence.instance) else {
                    continue;
                };
                for body in visible_bodies(instance.part) {
                    let local = match cache.get(&body) {
                        Some(mesh) => mesh.clone(),
                        None => {
                            let mesh = body_mesh(doc, body, quality)?;
                            cache.insert(body, mesh.clone());
                            mesh
                        }
                    };
                    placed.push((
                        instance.part,
                        body,
                        transformed(&local, occurrence.placement)?,
                        body_down(doc, body).map(|d| occurrence.placement.apply_vector(d)),
                    ));
                }
            }
            if scope == Scope::Joined {
                if placed.is_empty() {
                    return Err(DocumentError(
                        "Não há corpos visíveis para exportar.".into(),
                    ));
                }
                let meshes = placed.into_iter().map(|(_, _, m, _)| m).collect::<Vec<_>>();
                files.push(File {
                    name: unique(file_stem(doc.name()), &mut used),
                    mesh: join(&meshes)?,
                    occurrences: 1,
                    down: None,
                    surface: false,
                });
            } else {
                for (part, body, mesh, down) in placed {
                    files.push(File {
                        name: unique(name_of(part, body), &mut used),
                        mesh,
                        occurrences: 1,
                        down,
                        surface: is_surface(doc, body),
                    });
                }
            }
        }
    }
    if files.is_empty() {
        return Err(DocumentError(
            "Não há corpos visíveis para exportar.".into(),
        ));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nator_document::MeshAsset;
    use std::{error::Error, sync::Arc};

    fn assembly() -> Result<(Document, PartId), Box<dyn Error>> {
        let mut doc = Document::new("Montagem: teste/1")?;
        let asset = Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?));
        let (part, _) = doc.add_mesh(None, "Suporte".into(), Arc::clone(&asset))?;
        doc.add_mesh(Some(part), "Suporte".into(), asset)?;
        doc.add_instance(part, None, Placement::new([100., 0., 0.], [0.; 3])?)?;
        Ok((doc, part))
    }

    fn block(size: Point3) -> Result<Mesh, Box<dyn Error>> {
        let [x, y, z] = size;
        Ok(Mesh::new(
            vec![
                [0., 0., 0.],
                [x, 0., 0.],
                [x, y, 0.],
                [0., y, 0.],
                [0., 0., z],
                [x, 0., z],
                [x, y, z],
                [0., y, z],
            ],
            vec![
                [0, 2, 1],
                [0, 3, 2],
                [4, 5, 6],
                [4, 6, 7],
                [0, 1, 5],
                [0, 5, 4],
                [1, 2, 6],
                [1, 6, 5],
                [2, 3, 7],
                [2, 7, 6],
                [3, 0, 4],
                [3, 4, 7],
            ],
        )?)
    }
    fn file(name: &str, mesh: Mesh, occurrences: usize, down: Option<Point3>) -> File {
        File {
            name: name.into(),
            mesh,
            occurrences,
            down,
            surface: false,
        }
    }

    /// Dois blocos encostados e uma superfície aberta, como na montagem real.
    fn assembly_with_surface() -> Result<Document, Box<dyn Error>> {
        let mut doc = Document::new("Conjunto")?;
        let a = Arc::new(MeshAsset::new(block([10., 10., 10.])?));
        let (first, _) = doc.add_mesh(None, "A".into(), a.clone())?;
        let (second, _) = doc.add_mesh(None, "B".into(), a)?;
        // Peças novas nascem separadas: esta encosta a segunda na face da primeira.
        for (part, x) in [(first, 0.), (second, 10.)] {
            let id = doc
                .instances()
                .iter()
                .find(|i| i.part == part)
                .ok_or("Instância")?
                .id;
            doc.instance_mut(id)?.placement = Placement::new([x, 0., 0.], [0.; 3])?;
        }
        let open = Mesh::new(
            vec![[0., 0., 20.], [10., 0., 20.], [0., 10., 20.]],
            vec![[0, 1, 2]],
        )?;
        doc.add_mesh(None, "Serigrafia".into(), Arc::new(MeshAsset::new(open)))?;
        Ok(doc)
    }

    #[test]
    fn assembly_keeps_touching_bodies_separate_and_leaves_open_surfaces_out()
    -> Result<(), Box<dyn Error>> {
        let doc = assembly_with_surface()?;
        // A união em uma malha acusa o encosto, embora cada corpo seja um sólido fechado.
        let united = plan(&doc, Scope::Joined, Quality::Standard)?.remove(0);
        assert!(!united.mesh.topology().has_closed_oriented_edges());
        let bed = Some([200., 200., 100.]);
        let print = print_plan(&doc, Scope::Joined, Quality::Standard, true, bed)?;
        assert_eq!(print.skipped.len(), 1);
        assert!(print.skipped[0].contains("Serigrafia"));
        assert_eq!(print.layout.files.len(), 2);
        assert!(
            print
                .layout
                .files
                .iter()
                .all(|f| f.mesh.topology().has_closed_oriented_edges())
        );
        // A distância entre os corpos não muda, e o conjunto fica na mesa.
        let [a, b] = [&print.layout.files[0], &print.layout.files[1]];
        let world = |f: &Placed| add(f.mesh.bounds().min, f.items[0]);
        assert!((world(b)[0] - world(a)[0]).abs() - 10. < 1e-9);
        assert!(world(a)[2].abs() < 1e-9);
        let (_, size) = print.group.as_ref().ok_or("Conjunto")?;
        assert!((size[0] - 20.).abs() < 1e-9);
        assert!((world(a)[0].min(world(b)[0]) - 90.).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn only_open_surfaces_are_a_clear_error() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Superfície")?;
        let open = Mesh::new(
            vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            vec![[0, 1, 2]],
        )?;
        let (_, body) = doc.add_mesh(None, "Serigrafia".into(), Arc::new(MeshAsset::new(open)))?;
        let message = print_plan(&doc, Scope::Body(body), Quality::Standard, true, None)
            .err()
            .ok_or("Sem erro")?
            .to_string();
        assert!(message.contains("superfícies abertas"), "{message}");
        Ok(())
    }

    #[test]
    fn chosen_face_rests_on_the_bed_keeping_the_solid_closed() -> Result<(), Box<dyn Error>> {
        let mesh = block([10., 20., 30.])?;
        let volume = mesh.signed_volume();
        // Cada direção vira a dimensão correspondente em altura.
        for (down, height) in [
            ([0., 0., -1.], 30.),
            ([0., 0., 1.], 30.),
            ([1., 0., 0.], 10.),
            ([0., -1., 0.], 20.),
        ] {
            let layout = prepare_for_bed(&[file("a", mesh.clone(), 1, Some(down))], None)?;
            let placed = &layout.files[0];
            assert!((placed.size[2] - height).abs() < 1e-9, "{down:?}");
            assert!(placed.mesh.bounds().min[2].abs() < 1e-12);
            assert!(placed.mesh.topology().has_closed_oriented_edges());
            assert!((placed.mesh.signed_volume() - volume).abs() < 1e-6);
        }
        // Face inclinada 45°: a rotação é própria, sem espelhar o sólido.
        let tilted = prepare_for_bed(
            &[file(
                "a",
                mesh,
                1,
                Some([
                    0.,
                    std::f64::consts::FRAC_1_SQRT_2,
                    -std::f64::consts::FRAC_1_SQRT_2,
                ]),
            )],
            None,
        )?;
        assert!((tilted.files[0].mesh.signed_volume() - volume).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn copies_fill_rows_and_excess_is_reported() -> Result<(), Box<dyn Error>> {
        let bed = Some([100., 100., 50.]);
        let layout = prepare_for_bed(&[file("a", block([30., 20., 10.])?, 4, None)], bed)?;
        let items = &layout.files[0].items;
        assert_eq!(items.len(), 4);
        assert!(!layout.overflow && layout.oversized.is_empty());
        // Três por fileira: 30 + 5 + 30 + 5 + 30 = 100.
        assert!((items[0][1] - items[2][1]).abs() < 1e-9);
        assert!((items[3][1] - items[0][1] - 25.).abs() < 1e-9);
        assert!(items.iter().all(|p| p[0] >= 0. && p[0] + 30. <= 100.));
        let tall = prepare_for_bed(&[file("alta", block([10., 10., 60.])?, 1, None)], bed)?;
        assert_eq!(tall.oversized, vec!["alta".to_owned()]);
        let many = prepare_for_bed(&[file("a", block([60., 60., 10.])?, 2, None)], bed)?;
        assert!(many.overflow && many.oversized.is_empty());
        // Sem mesa nada é avaliado, e o modelo fica centrado na origem.
        let free = prepare_for_bed(&[file("a", block([500., 20., 10.])?, 1, None)], None)?;
        assert!(!free.overflow);
        assert!((free.files[0].items[0][0] + 250.).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn separate_files_use_local_coordinates_and_unique_names() -> Result<(), Box<dyn Error>> {
        let (doc, part) = assembly()?;
        let files = plan(&doc, Scope::Part(part), Quality::Standard)?;
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "Suporte_Suporte");
        assert_eq!(files[1].name, "Suporte_Suporte_2");
        assert_eq!(files[0].occurrences, 2);
        assert_eq!(files[0].mesh.bounds().min, [0.; 3]);
        let definitions = plan(&doc, Scope::Definitions, Quality::Standard)?;
        assert_eq!(definitions.len(), 2);
        Ok(())
    }

    #[test]
    fn joined_and_occurrence_exports_use_assembly_positions() -> Result<(), Box<dyn Error>> {
        let (doc, _) = assembly()?;
        let joined = plan(&doc, Scope::Joined, Quality::Draft)?;
        assert_eq!(joined.len(), 1);
        // Nome do documento sem os caracteres reservados ':' e '/'.
        assert_eq!(joined[0].name, "Montagem_ teste_1");
        assert_eq!(joined[0].mesh.bounds().max[0], 160.);
        assert_eq!(joined[0].mesh.triangles().len(), 4 * 20);
        let all = plan(&doc, Scope::AllOccurrences, Quality::Draft)?;
        assert_eq!(all.len(), 4);
        let summary = summarize(&all);
        assert_eq!(summary.triangles, 80);
        assert_eq!(summary.bytes, 4 * 84 + 80 * 50);
        assert_eq!(summary.bounds.ok_or("Caixa ausente")?.max[0], 160.);
        assert!(
            plan(
                &doc,
                Scope::Joined,
                Quality::Custom {
                    linear_mm: 0.,
                    angular_rad: 0.1
                }
            )
            .is_err()
        );
        Ok(())
    }
}
