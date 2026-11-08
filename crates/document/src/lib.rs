//! Documento independente da interface. Corpos mantêm malhas locais compartilhadas por Arc.
//! Instâncias referenciam definições de peças; nunca duplicam a geometria automaticamente.
use nator_geometry::{Mesh, Placement, RgbColor, Topology};
use std::{collections::HashSet, error::Error, fmt, sync::Arc};
pub mod activity;
pub mod appearance;
pub mod assembly;
pub use appearance::{PaintBoundary, PaintMask, SurfaceAppearance, SurfacePaint};
pub mod drawing;
pub mod features;
pub mod images;
pub use images::{RasterAsset, ReferenceImage};
pub mod naming;
pub use naming::{ElementKind, NameCounters};
pub mod parameters;
pub mod print_profile;
pub mod selection;
pub mod work_planes;
pub use work_planes::{WorkPlane, WorkPlaneRecord};

pub const MAX_ELEMENTS: usize = 10_000;
pub const MAX_SCENE_TRIANGLES: usize = 2_000_000;

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub struct $name(pub u64);
    };
}
identifier!(PartId);
identifier!(BodyId);
identifier!(InstanceId);
identifier!(SketchId);
identifier!(GroupId);
identifier!(SheetId);
identifier!(WorkPlaneId);
identifier!(ImageId);
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SheetRecord {
    pub id: SheetId,
    pub target: Option<PartId>,
    pub specification: nator_drawing::Specification,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<drawing::DrawingAnnotation>,
}

/// Unidade usada nos textos de medida e nos números digitados sem unidade.
/// A geometria continua armazenada em milímetros. Projetos que usavam centímetro ou
/// metro abrem em milímetro: só a apresentação muda, nunca o modelo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LengthUnit {
    #[default]
    #[serde(alias = "Centimeter", alias = "Meter")]
    Millimeter,
    Inch,
}
impl LengthUnit {
    /// Milímetros por unidade.
    pub fn factor(self) -> f64 {
        match self {
            Self::Millimeter => 1.,
            Self::Inch => 25.4,
        }
    }
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Millimeter => "mm",
            Self::Inch => "in",
        }
    }
    pub fn area_suffix(self) -> &'static str {
        match self {
            Self::Millimeter => "mm²",
            Self::Inch => "in²",
        }
    }
    pub fn volume_suffix(self) -> &'static str {
        match self {
            Self::Millimeter => "mm³",
            Self::Inch => "in³",
        }
    }
    pub fn from_mm(self, value: f64) -> f64 {
        value / self.factor()
    }
    pub fn from_mm2(self, value: f64) -> f64 {
        value / self.factor().powi(2)
    }
    pub fn from_mm3(self, value: f64) -> f64 {
        value / self.factor().powi(3)
    }
    pub fn to_mm(self, value: f64) -> f64 {
        value * self.factor()
    }
    /// Potência de dez entre a unidade e o milímetro. Mantém a resolução dos textos: o mesmo
    /// número de casas diria muito menos em metros (1 mm³ seria 0,000000001 m³).
    fn magnitude(self) -> usize {
        match self {
            Self::Millimeter => 0,
            // 1 in = 25,4 mm: uma casa a mais mantém a resolução de 0,1 mm visível.
            Self::Inch => 1,
        }
    }
    /// Expressão de uma medida vinda do mouse. A escrita com ponto e sem arredondar a
    /// resolução do `f64` além do necessário preserva o valor ao ser avaliada de novo.
    pub fn expression(self, mm: f64) -> String {
        let value = (self.from_mm(mm) * 1e12).round() / 1e12;
        format!("{value} {}", self.suffix())
    }
    /// Torna explícita a unidade de uma conta sem parâmetros digitada sem unidade: o que
    /// o histórico guarda não pode mudar de medida se a unidade da interface for trocada.
    /// Expressões com unidades ou parâmetros são preservadas como foram escritas.
    pub fn normalize_expression(self, expression: &str) -> String {
        if self == Self::Millimeter {
            return expression.to_owned();
        }
        let trimmed = expression.trim();
        if parameters::Parameters::default()
            .evaluate(trimmed, parameters::Dimension::Scalar)
            .is_err()
        {
            trimmed.to_owned()
        } else if trimmed.replace(',', ".").parse::<f64>().is_ok() {
            format!("{trimmed} {}", self.suffix())
        } else {
            format!("({trimmed}) * 1 {}", self.suffix())
        }
    }
}

/// Configurações do documento, salvas no projeto. Só afetam a apresentação e a
/// entrada pelo mouse ou teclado: a geometria continua em `f64` e milímetros.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Casas decimais exibidas em medidas, propriedades e cotas da interface (0 a 6).
    pub display_decimals: u8,
    /// Precisão linear do mouse, em milímetros. Também define o espaçamento da grade.
    pub mouse_step_mm: f64,
    /// Precisão angular do mouse, em graus.
    pub mouse_angle_step_deg: f64,
    /// Unidade exibida e usada em números digitados sem unidade.
    pub unit: LengthUnit,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            display_decimals: 2,
            mouse_step_mm: DEFAULT_MOUSE_STEP_MM,
            mouse_angle_step_deg: 1.,
            unit: LengthUnit::Millimeter,
        }
    }
}
impl Settings {
    /// Encaixa uma medida do mouse na precisão linear configurada.
    pub fn snap_length(self, value: f64) -> f64 {
        nator_sketch::drafting::snap_to_step(value, self.mouse_step_mm)
    }
    /// Passos do desenho no rascunho.
    pub fn drafting_steps(self) -> nator_sketch::drafting::Steps {
        nator_sketch::drafting::Steps {
            length_mm: self.mouse_step_mm,
            angle_deg: self.mouse_angle_step_deg,
        }
    }
    /// Casas de uma medida: as configuradas, ou mais se forem necessárias para que a
    /// precisão do mouse continue visível (1 mm exibido em metros pede três casas).
    pub fn length_decimals(self) -> usize {
        let display = usize::from(self.display_decimals);
        // Um passo métrico em polegadas pode não ter representação curta; três casas a
        // mais já distinguem passos vizinhos sem encher os campos de algarismos.
        display.max(step_decimals(self.unit.from_mm(self.mouse_step_mm)).min(display + 3))
    }
    /// Casas necessárias para mostrar um ângulo na precisão do mouse.
    pub fn angle_step_decimals(self) -> usize {
        step_decimals(self.mouse_angle_step_deg)
    }
    /// Número de uma medida em milímetros, na unidade exibida e com casas fixas.
    pub fn format_length(self, mm: f64) -> String {
        nator_drawing::format_fixed(self.unit.from_mm(mm), self.length_decimals())
    }
    /// Número de uma medida em milímetros, na unidade exibida e sem zeros à direita.
    pub fn format_compact_length(self, mm: f64) -> String {
        let decimals = (3 + self.unit.magnitude()).max(self.length_decimals());
        nator_drawing::format_trimmed(self.unit.from_mm(mm), decimals)
    }
    /// Área em milímetros quadrados, com o símbolo da unidade exibida.
    pub fn area_label(self, mm2: f64) -> String {
        let decimals = usize::from(self.display_decimals) + 2 * self.unit.magnitude();
        format!(
            "{} {}",
            nator_drawing::format_fixed(self.unit.from_mm2(mm2), decimals),
            self.unit.area_suffix()
        )
    }
    /// Volume em milímetros cúbicos, com o símbolo da unidade exibida.
    pub fn volume_label(self, mm3: f64) -> String {
        let decimals = usize::from(self.display_decimals) + 3 * self.unit.magnitude();
        format!(
            "{} {}",
            nator_drawing::format_fixed(self.unit.from_mm3(mm3), decimals),
            self.unit.volume_suffix()
        )
    }
    /// Medida compacta seguida do símbolo da unidade exibida.
    pub fn length_label(self, mm: f64) -> String {
        format!("{} {}", self.format_compact_length(mm), self.unit.suffix())
    }
}
/// Menor quantidade de casas que representa `step` sem arredondar.
fn step_decimals(step: f64) -> usize {
    let limit = usize::from(MAX_DISPLAY_DECIMALS);
    (0..limit)
        .find(|decimals| {
            let scaled = step * 10_f64.powi(*decimals as i32);
            (scaled - scaled.round()).abs() < 1e-6
        })
        .unwrap_or(limit)
}
/// Limite de casas decimais exibidas.
pub const MAX_DISPLAY_DECIMALS: u8 = 6;
/// Limites da precisão do mouse, em milímetros e graus.
/// Passo linear inicial; projetos com valor explícito conservam sua configuração.
pub const DEFAULT_MOUSE_STEP_MM: f64 = 0.1;
pub const MIN_MOUSE_STEP_MM: f64 = 0.001;
pub const MAX_MOUSE_STEP_MM: f64 = 1000.;
pub const MIN_MOUSE_ANGLE_STEP_DEG: f64 = 0.01;
pub const MAX_MOUSE_ANGLE_STEP_DEG: f64 = 90.;

#[derive(Debug, Clone)]
pub struct SketchRecord {
    pub id: SketchId,
    pub sketch: nator_sketch::Sketch,
    pub visible: bool,
    /// Plano de trabalho da mesma peça que define o plano do rascunho: editar o plano
    /// move o rascunho e recalcula o histórico. `None` em rascunhos sobre a origem, sobre
    /// faces (plano capturado) e de referência da montagem.
    pub support: Option<WorkPlaneId>,
}

#[derive(Debug, Clone)]
pub struct DocumentError(pub String);
impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for DocumentError {}

/// Ângulo a partir do qual uma dobra da malha é exibida como aresta (30°).
pub const CREASE_ANGLE_RAD: f64 = std::f64::consts::FRAC_PI_6;

/// A análise das arestas ocorre uma vez por geometria, fora do redesenho da interface.
#[derive(Debug)]
pub struct MeshAsset {
    mesh: Mesh,
    topology: Topology,
    /// Arestas vivas por triângulo (ver `Mesh::edge_analysis`), usadas na exibição.
    creases: Vec<u8>,
    /// Face do B-rep de origem de cada triângulo, nos corpos sólidos. Permite selecionar
    /// faces e arestas na área 3D e convertê-las em referências por nome.
    triangle_faces: Option<Vec<u32>>,
}
impl MeshAsset {
    pub fn new(mesh: Mesh) -> Self {
        let (topology, creases) = mesh.edge_analysis(Some(CREASE_ANGLE_RAD));
        Self {
            mesh,
            topology,
            creases,
            triangle_faces: None,
        }
    }
    /// Malha de exibição de um sólido, com a face de origem de cada triângulo.
    pub fn with_faces(mesh: Mesh, faces: Vec<u32>) -> Result<Self, DocumentError> {
        if faces.len() != mesh.triangles().len() {
            return Err(DocumentError(
                "Faces de origem não correspondem aos triângulos da malha.".into(),
            ));
        }
        let mut asset = Self::new(mesh);
        asset.triangle_faces = Some(faces);
        Ok(asset)
    }
    pub fn triangle_faces(&self) -> Option<&[u32]> {
        self.triangle_faces.as_deref()
    }
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }
    pub fn topology(&self) -> Topology {
        self.topology
    }
    pub fn creases(&self) -> &[u8] {
        &self.creases
    }
}

/// Estilo de exibição de um corpo na cena. É um controle visual: não altera geometria,
/// histórico nem a visibilidade gravada no documento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Normal,
    /// Esmaecido, como as outras peças durante a construção de uma peça.
    Dimmed,
    Transparent,
    /// Selecionado: opaco, com destaque de cor.
    Selected,
}
impl Style {
    /// Estilos desenhados depois dos opacos, sem escrever profundidade.
    pub fn is_translucent(self) -> bool {
        matches!(self, Self::Dimmed | Self::Transparent)
    }
}

/// Acúmulo de uma parte da cena (opaca ou translúcida) antes da concatenação.
#[derive(Default)]
struct SceneGroup {
    vertices: Vec<nator_geometry::Point3>,
    triangles: Vec<[u32; 3]>,
    creases: Vec<u8>,
    styles: Vec<Style>,
    colors: Vec<RgbColor>,
    ranges: Vec<std::ops::Range<usize>>,
}

/// Cores de análise por triângulo da malha exibida de `body`. Só apresentação: não entra no
/// documento nem no projeto salvo.
#[derive(Debug, Clone, PartialEq)]
pub struct Inspection {
    pub body: BodyId,
    pub colors: Vec<Option<RgbColor>>,
}

/// Cena de exibição: malha em coordenadas de montagem, arestas vivas e estilo por triângulo.
/// Os triângulos opacos vêm primeiro (`opaque_triangles`), para que os translúcidos sejam
/// desenhados depois deles.
pub struct Scene {
    pub mesh: Mesh,
    pub creases: Vec<u8>,
    pub styles: Vec<Style>,
    pub colors: Vec<RgbColor>,
    pub opaque_triangles: usize,
    /// Faixas de triângulos por corpo/ocorrência, antes da agregação visual.
    pub body_ranges: Vec<std::ops::Range<usize>>,
}

#[derive(Debug, Clone)]
pub struct Body {
    pub id: BodyId,
    pub name: String,
    pub geometry: Arc<MeshAsset>,
    pub brep: Option<Arc<Vec<u8>>>,
    /// Nome estável de cada face do B-rep, na ordem topológica do núcleo. Vazio quando o
    /// corpo é malha ou veio de um projeto sem nomes; um nome vazio marca face sem origem.
    pub face_names: Arc<Vec<String>>,
    /// Cores próprias produzidas por operações; pinturas explícitas prevalecem.
    pub intrinsic_colors: Arc<Vec<Option<RgbColor>>>,
    pub placement: Placement,
    pub visible: bool,
    /// Cor sólida definida pelo usuário ou importada; ausência usa a aparência neutra.
    pub color: Option<RgbColor>,
    /// Direção unitária, no referencial local do corpo, que aponta para a mesa de impressão.
    /// Não altera a modelagem e passa de etapa em etapa, como o nome e a cor.
    pub print_down: Option<nator_geometry::Point3>,
}

impl Body {
    /// Direção que deixa `face` apoiada na mesa: sua normal externa, calculada na malha,
    /// onde a orientação dos triângulos sempre aponta para fora do sólido.
    pub fn print_direction(
        &self,
        face: &features::FaceRef,
    ) -> Result<nator_geometry::Point3, DocumentError> {
        use nator_geometry::{add, cross, dot, norm, sub};
        let faces = self.geometry.triangle_faces().ok_or_else(|| {
            DocumentError("Corpos de malha não têm faces para apoiar na mesa.".into())
        })?;
        let chosen = match face {
            features::FaceRef::Named(name) => self
                .face_names
                .iter()
                .enumerate()
                .filter(|(_, n)| *n == name)
                .map(|(i, _)| i as u32)
                .collect::<Vec<_>>(),
            features::FaceRef::Index { index, .. } => vec![*index as u32],
        };
        let mesh = self.geometry.mesh();
        let normals = faces
            .iter()
            .zip(mesh.triangles())
            .filter(|(id, _)| chosen.contains(id))
            .map(|(_, t)| {
                let [a, b, c] = mesh.triangle_points(*t);
                cross(sub(b, a), sub(c, a))
            })
            .collect::<Vec<_>>();
        if normals.is_empty() {
            return Err(DocumentError("Face inexistente neste corpo.".into()));
        }
        let not_flat = || DocumentError("Escolha uma face plana para apoiar na mesa.".into());
        let sum = normals.iter().fold([0.; 3], |total, n| add(total, *n));
        let length = norm(sum);
        // Em uma parede cilíndrica fechada as normais se anulam.
        if length < 1e-9 {
            return Err(not_flat());
        }
        let down = sum.map(|v| v / length);
        // Sem normalizar antes: triângulos minúsculos têm normal ruidosa e pesam pela área.
        if !normals.iter().all(|n| dot(*n, down) >= 0.9999 * norm(*n)) {
            return Err(not_flat());
        }
        Ok(down)
    }
}

/// Limite de faces nomeadas por corpo, coerente com o limite de topologia do núcleo.
pub const MAX_FACES_PER_BODY: usize = 1_000_000;
#[derive(Debug, Clone)]
pub struct Part {
    pub id: PartId,
    pub name: String,
    pub bodies: Vec<Body>,
    pub sketches: Vec<SketchRecord>,
    /// Planos de trabalho (suportes auxiliares de rascunhos), em coordenadas da peça.
    pub work_planes: Vec<WorkPlaneRecord>,
    pub parameters: parameters::Parameters,
    pub features: Vec<features::Feature>,
    /// Marcador do histórico: quantidade de operações construídas a partir do início.
    /// `None` constrói todas. As operações depois do marcador ficam inativas e novas
    /// operações são inseridas nessa posição, como ao voltar a uma etapa do histórico.
    pub history_marker: Option<usize>,
    /// Visibilidade das pastas da peça na árvore (corpos, subpeças, rascunhos, planos, origem).
    pub folders: PartFolders,
    /// Modelo importado só como referência da montagem: aparece em Referências, pode ser
    /// movido, ocultado e medido, mas não editado. "Criar peça" tira a marca.
    pub reference: bool,
}
impl Part {
    /// Alguma expressão da peça cita `name`. A busca textual pode acusar a mais, o que só
    /// custa um recálculo, mas nunca deixa de achar uma expressão.
    pub fn mentions_parameter(&self, name: &str) -> bool {
        let cited = |text: String| print_profile::mentions(&text, name);
        cited(format!("{:?}", self.parameters.entries))
            || self
                .sketches
                .iter()
                .any(|s| cited(format!("{:?}", s.sketch.dimension_expressions)))
            || self
                .features
                .iter()
                .any(|f| cited(format!("{:?}", f.operation)))
    }
    /// Corpos apresentados como elementos da peça, sem acumular as etapas ocultas do
    /// histórico. Um corpo independente ocultado pelo usuário continua listado para que
    /// possa ser reexibido. Uma etapa antiga explicitamente visível também permanece
    /// acessível; só os registros históricos ocultos são retirados da lista.
    ///
    /// A visibilidade da pasta não participa desta consulta: ocultar Corpos não pode
    /// apagar seus nós da árvore. As etapas suprimidas/recuadas continuam no histórico.
    pub fn listed_bodies(&self) -> impl Iterator<Item = &Body> {
        let historical = features::hidden_by_history(self);
        self.bodies
            .iter()
            .filter(move |b| b.visible || !historical.contains(&b.id))
    }

    /// Corpos exibidos: visíveis e com a pasta Corpos visível. A visibilidade própria de
    /// cada corpo, que o histórico também usa, não muda quando a pasta é ocultada.
    pub fn shown_bodies(&self) -> impl Iterator<Item = &Body> {
        let folder = self.folders.bodies;
        self.bodies.iter().filter(move |b| folder && b.visible)
    }
    /// Rascunho exibido: visível e com a pasta Rascunhos visível.
    pub fn shows_sketch(&self, record: &SketchRecord) -> bool {
        self.folders.sketches && record.visible
    }
}

/// Visibilidade das pastas de uma peça na árvore do projeto. A visibilidade efetiva de um
/// elemento combina a dele com a da pasta, e ocultar a pasta preserva a escolha de cada um.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartFolders {
    pub bodies: bool,
    pub subparts: bool,
    pub sketches: bool,
    /// Planos de trabalho; visíveis por padrão, inclusive em projetos sem o campo.
    pub planes: bool,
    /// Eixos da origem da peça, desenhados em cada ocorrência; ocultos por padrão.
    pub origin: bool,
    /// Modelos de referência posicionados dentro da peça (versão 51); visíveis por padrão.
    pub references: bool,
}
impl Default for PartFolders {
    fn default() -> Self {
        Self {
            bodies: true,
            subparts: true,
            sketches: true,
            planes: true,
            origin: false,
            references: true,
        }
    }
}

/// Visibilidade dos nós do projeto. `project` permanece na persistência por
/// compatibilidade; a raiz é sempre visível, e só os seus descendentes podem ser ocultos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectFolders {
    pub project: bool,
    pub parts: bool,
    pub sketches: bool,
    pub origin: bool,
    /// Modelos importados só como referência (versão 46).
    pub references: bool,
}
impl Default for ProjectFolders {
    fn default() -> Self {
        Self {
            project: true,
            parts: true,
            sketches: true,
            origin: true,
            references: true,
        }
    }
}

/// Nó da árvore com visibilidade própria, além de peças, corpos e rascunhos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Folder {
    Project,
    Parts,
    ReferenceSketches,
    References,
    Origin,
    Bodies(PartId),
    Subparts(PartId),
    Sketches(PartId),
    WorkPlanes(PartId),
    PartOrigin(PartId),
    /// Referências da peça: instâncias filhas cuja definição é referência.
    PartReferences(PartId),
}
#[derive(Debug, Clone)]
pub struct Instance {
    pub parent: Option<PartId>,
    pub id: InstanceId,
    pub part: PartId,
    pub name: String,
    pub placement: Placement,
    pub visible: bool,
    /// Cor de ocorrência vinda do STEP; sobrepõe a aparência da definição sem duplicá-la.
    pub color: Option<RgbColor>,
}

#[derive(Debug, Clone)]
pub struct Document {
    id: String,
    name: String,
    next_id: u64,
    name_counters: NameCounters,
    images: Vec<ReferenceImage>,
    appearances: Vec<SurfaceAppearance>,
    activities: Vec<activity::Activity>,
    parts: Vec<Part>,
    instances: Vec<Instance>,
    groups: Vec<assembly::Group>,
    sheets: Vec<SheetRecord>,
    settings: Settings,
    print_profile: print_profile::PrintProfile,
    /// Rascunhos de referência da montagem, em coordenadas da montagem. Não pertencem a
    /// nenhuma peça, então nenhuma operação pode usá-los como perfil: servem apenas de
    /// referência para posicionar componentes.
    reference_sketches: Vec<SketchRecord>,
    folders: ProjectFolders,
}

pub fn validate_name(name: &str) -> Result<(), DocumentError> {
    if name.trim().is_empty() || name.len() > 1024 || name.contains('\0') {
        Err(DocumentError(
            "O nome deve ter entre 1 e 1024 bytes e não pode conter NUL.".into(),
        ))
    } else {
        Ok(())
    }
}

impl Document {
    pub fn new(name: impl Into<String>) -> Result<Self, DocumentError> {
        let name = name.into();
        validate_name(&name)?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            next_id: 1,
            name_counters: NameCounters::default(),
            images: vec![],
            appearances: vec![],
            activities: vec![],
            parts: Vec::new(),
            instances: Vec::new(),
            groups: Vec::new(),
            sheets: Vec::new(),
            settings: Settings::default(),
            print_profile: Default::default(),
            reference_sketches: Vec::new(),
            folders: ProjectFolders::default(),
        })
    }
    /// Toda entrada de persistência passa pelas mesmas invariantes das edições.
    pub fn restore(
        id: String,
        name: String,
        next_id: u64,
        parts: Vec<Part>,
        instances: Vec<Instance>,
    ) -> Result<Self, DocumentError> {
        let mut document = Self {
            id,
            name,
            next_id,
            name_counters: NameCounters::default(),
            images: vec![],
            appearances: vec![],
            activities: vec![],
            parts,
            instances,
            groups: Vec::new(),
            sheets: Vec::new(),
            settings: Settings::default(),
            print_profile: Default::default(),
            reference_sketches: Vec::new(),
            folders: ProjectFolders::default(),
        };
        document.restore_name_counters(NameCounters::default())?;
        document.validate()?;
        Ok(document)
    }
    pub fn sheets(&self) -> &[SheetRecord] {
        &self.sheets
    }
    pub fn settings(&self) -> Settings {
        self.settings
    }
    pub fn folders(&self) -> ProjectFolders {
        self.folders
    }
    pub fn restore_folders(&mut self, folders: ProjectFolders) {
        // Projetos antigos podiam ocultar a raiz. A árvore atual não oferece
        // esse controle; normalizar evita carregar um projeto impossível de revelar.
        self.folders = ProjectFolders {
            project: true,
            ..folders
        };
    }
    /// Visibilidade própria do nó, sem considerar os nós acima dele.
    pub fn folder_visible(&self, folder: Folder) -> Result<bool, DocumentError> {
        let part = |id: PartId| {
            self.part(id)
                .map(|p| p.folders)
                .ok_or_else(|| DocumentError("Peça inexistente.".into()))
        };
        Ok(match folder {
            Folder::Project => self.folders.project,
            Folder::Parts => self.folders.parts,
            Folder::ReferenceSketches => self.folders.sketches,
            Folder::References => self.folders.references,
            Folder::Origin => self.folders.origin,
            Folder::Bodies(id) => part(id)?.bodies,
            Folder::Subparts(id) => part(id)?.subparts,
            Folder::Sketches(id) => part(id)?.sketches,
            Folder::WorkPlanes(id) => part(id)?.planes,
            Folder::PartOrigin(id) => part(id)?.origin,
            Folder::PartReferences(id) => part(id)?.references,
        })
    }
    pub fn set_folder_visible(
        &mut self,
        folder: Folder,
        visible: bool,
    ) -> Result<(), DocumentError> {
        if folder == Folder::Project && !visible {
            return Err(DocumentError(
                "A raiz do projeto deve permanecer visível.".into(),
            ));
        }
        let flag = match folder {
            Folder::Project => &mut self.folders.project,
            Folder::Parts => &mut self.folders.parts,
            Folder::ReferenceSketches => &mut self.folders.sketches,
            Folder::References => &mut self.folders.references,
            Folder::Origin => &mut self.folders.origin,
            Folder::Bodies(id) => &mut self.part_mut(id)?.folders.bodies,
            Folder::Subparts(id) => &mut self.part_mut(id)?.folders.subparts,
            Folder::Sketches(id) => &mut self.part_mut(id)?.folders.sketches,
            Folder::WorkPlanes(id) => &mut self.part_mut(id)?.folders.planes,
            Folder::PartOrigin(id) => &mut self.part_mut(id)?.folders.origin,
            Folder::PartReferences(id) => &mut self.part_mut(id)?.folders.references,
        };
        *flag = visible;
        Ok(())
    }
    /// Instância exibida no seu nível: visível e com a pasta que a contém visível (Peças ou
    /// Referências, na raiz, dentro do projeto; Subpeças ou Referências, dentro de uma
    /// peça). Os níveis acima são conferidos ao percorrer a montagem (`occurrences`).
    pub fn shows_instance(&self, instance: &Instance) -> bool {
        let reference = self.part(instance.part).is_some_and(|p| p.reference);
        instance.visible
            && match instance.parent {
                None if reference => self.folders.project && self.folders.references,
                None => self.folders.project && self.folders.parts,
                Some(parent) if reference => {
                    self.part(parent).is_some_and(|p| p.folders.references)
                }
                Some(parent) => self.part(parent).is_some_and(|p| p.folders.subparts),
            }
    }
    /// Rascunho de referência exibido: visível, com a pasta e o projeto visíveis.
    pub fn shows_reference_sketch(&self, record: &SketchRecord) -> bool {
        self.folders.project && self.folders.sketches && record.visible
    }
    /// Eixos da origem da montagem exibidos.
    pub fn shows_origin(&self) -> bool {
        self.folders.project && self.folders.origin
    }
    pub fn print_profile(&self) -> &print_profile::PrintProfile {
        &self.print_profile
    }
    /// Publica o perfil em todas as peças. Quem chama resolve rascunhos e histórico das
    /// peças afetadas, como ao mudar um parâmetro.
    pub fn set_print_profile(
        &mut self,
        profile: print_profile::PrintProfile,
    ) -> Result<(), DocumentError> {
        profile.validate()?;
        let inherited = profile.parameters();
        for part in &mut self.parts {
            part.parameters.inherited = inherited.clone();
        }
        self.print_profile = profile;
        Ok(())
    }
    pub fn set_settings(&mut self, settings: Settings) -> Result<(), DocumentError> {
        if settings.display_decimals > MAX_DISPLAY_DECIMALS {
            return Err(DocumentError(
                "As casas decimais exibidas devem ficar entre 0 e 6.".into(),
            ));
        }
        if !settings.mouse_step_mm.is_finite()
            || !(MIN_MOUSE_STEP_MM..=MAX_MOUSE_STEP_MM).contains(&settings.mouse_step_mm)
        {
            return Err(DocumentError(
                "A precisão linear do mouse deve ficar entre 0,001 e 1000 mm.".into(),
            ));
        }
        if !settings.mouse_angle_step_deg.is_finite()
            || !(MIN_MOUSE_ANGLE_STEP_DEG..=MAX_MOUSE_ANGLE_STEP_DEG)
                .contains(&settings.mouse_angle_step_deg)
        {
            return Err(DocumentError(
                "A precisão angular do mouse deve ficar entre 0,01 e 90 graus.".into(),
            ));
        }
        self.settings = settings;
        Ok(())
    }
    pub fn restore_sheets(&mut self, sheets: Vec<SheetRecord>) -> Result<(), DocumentError> {
        let mut next = self.clone();
        next.sheets = sheets;
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn set_sheet(
        &mut self,
        id: Option<SheetId>,
        target: Option<PartId>,
        specification: nator_drawing::Specification,
        annotations: Vec<drawing::DrawingAnnotation>,
    ) -> Result<SheetId, DocumentError> {
        if let Some(id) = id {
            let record = self
                .sheets
                .iter_mut()
                .find(|s| s.id == id)
                .ok_or_else(|| DocumentError("Folha inexistente.".into()))?;
            *record = SheetRecord {
                id,
                target,
                specification,
                annotations,
            };
            Ok(id)
        } else {
            let id = SheetId(self.allocate_id()?);
            self.sheets.push(SheetRecord {
                id,
                target,
                specification,
                annotations,
            });
            Ok(id)
        }
    }
    /// A folha de peça usa sua origem local e inclui as subpeças, sem colocar a montagem na peça.
    pub fn drawing_mesh(&self, target: Option<PartId>) -> Result<Mesh, DocumentError> {
        self.drawing_context(target)?
            .scene_mesh()?
            .ok_or_else(|| DocumentError("Não há corpos visíveis para gerar a folha.".into()))
    }
    /// Contexto independente para consultas de desenho. A origem da definição escolhida
    /// substitui as posições de suas ocorrências na montagem, incluindo suas subpeças.
    pub fn drawing_context(&self, target: Option<PartId>) -> Result<Self, DocumentError> {
        if let Some(part) = target {
            let mut local = self.clone();
            local.instances.retain(|i| i.parent.is_some());
            local.groups.clear();
            local.add_instance(part, None, Placement::default())?;
            Ok(local)
        } else {
            Ok(self.clone())
        }
    }
    /// Notas de documentação derivadas do histórico: roscas simplificadas dos furos da peça
    /// (ou de todas as peças, na folha de montagem). As medidas aparecem como digitadas.
    pub fn drawing_notes(&self, target: Option<PartId>) -> Vec<String> {
        use features::Operation;
        self.parts
            .iter()
            .filter(|p| target.is_none_or(|t| t == p.id))
            .flat_map(|part| &part.features)
            .filter_map(|feature| match &feature.operation {
                Operation::Hole { points, spec, .. } => spec.thread.as_ref().map(|thread| {
                    let depth = thread
                        .depth
                        .as_deref()
                        .map(|d| format!(", profundidade {d}"))
                        .unwrap_or_default();
                    format!(
                        "ROSCA {} (Ø {}, passo {}{depth}) EM {} FURO(S) - {}",
                        thread.designation,
                        thread.nominal,
                        thread.pitch,
                        points.len(),
                        feature.name
                    )
                }),
                _ => None,
            })
            .collect()
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn next_id(&self) -> u64 {
        self.next_id
    }
    pub fn parts(&self) -> &[Part] {
        &self.parts
    }
    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }
    pub fn part(&self, id: PartId) -> Option<&Part> {
        self.parts.iter().find(|p| p.id == id)
    }
    pub fn body(&self, id: BodyId) -> Option<&Body> {
        self.parts
            .iter()
            .flat_map(|p| &p.bodies)
            .find(|b| b.id == id)
    }
    pub fn instance(&self, id: InstanceId) -> Option<&Instance> {
        self.instances.iter().find(|i| i.id == id)
    }
    /// Rascunho de peça ou de referência da montagem.
    pub fn sketch(&self, id: SketchId) -> Option<&SketchRecord> {
        self.parts
            .iter()
            .flat_map(|p| &p.sketches)
            .chain(&self.reference_sketches)
            .find(|s| s.id == id)
    }
    /// Visibilidade do rascunho é estado do documento, persistido e reversível.
    pub fn sketch_mut(&mut self, id: SketchId) -> Result<&mut SketchRecord, DocumentError> {
        self.parts
            .iter_mut()
            .flat_map(|p| &mut p.sketches)
            .chain(&mut self.reference_sketches)
            .find(|s| s.id == id)
            .ok_or_else(|| DocumentError("Rascunho inexistente.".into()))
    }
    /// Peça dona do rascunho; `None` para rascunhos de referência da montagem.
    pub fn sketch_part(&self, id: SketchId) -> Option<PartId> {
        self.parts
            .iter()
            .find(|p| p.sketches.iter().any(|s| s.id == id))
            .map(|p| p.id)
    }
    pub fn reference_sketches(&self) -> &[SketchRecord] {
        &self.reference_sketches
    }
    pub fn is_reference_sketch(&self, id: SketchId) -> bool {
        self.reference_sketches.iter().any(|s| s.id == id)
    }
    /// Rascunho de referência novo, no plano dado em coordenadas da montagem.
    pub fn add_reference_sketch(
        &mut self,
        mut sketch: nator_sketch::Sketch,
    ) -> Result<SketchId, DocumentError> {
        sketch
            .validate()
            .map_err(|e| DocumentError(e.to_string()))?;
        let number = self.record_creation(ElementKind::Sketch)?;
        if matches!(sketch.name.as_str(), "Rascunho" | "Rascunho de referência") {
            sketch.name = format!("{} {number}", sketch.name);
        }
        let id = SketchId(self.allocate_id()?);
        self.reference_sketches.push(SketchRecord {
            id,
            sketch,
            visible: true,
            support: None,
        });
        Ok(id)
    }
    pub fn restore_reference_sketches(
        &mut self,
        sketches: Vec<SketchRecord>,
    ) -> Result<(), DocumentError> {
        let mut next = self.clone();
        next.reference_sketches = sketches;
        next.validate()?;
        *self = next;
        Ok(())
    }
    /// Exclui um rascunho de referência, ou de peça quando nenhuma operação o usa: o
    /// histórico nunca fica apontando para um perfil ausente.
    /// O rascunho delimita alguma pintura.
    fn masks_paint(&self, id: SketchId) -> bool {
        self.appearances
            .iter()
            .flat_map(|a| &a.layers)
            .any(|l| l.masks().any(|m| m.sketch == id))
    }
    /// O rascunho alimenta o histórico ou uma pintura: editá-lo pede recálculo da peça.
    pub fn sketch_drives_geometry(&self, id: SketchId) -> bool {
        let used = |p: &Part| p.features.iter().any(|f| f.operation.uses_sketch(id));
        self.masks_paint(id) || self.parts.iter().any(used)
    }
    pub fn remove_sketch(&mut self, id: SketchId) -> Result<(), DocumentError> {
        if self.masks_paint(id) {
            return Err(DocumentError(
                "O rascunho delimita uma pintura; remova essa pintura antes de excluí-lo.".into(),
            ));
        }
        if self.is_reference_sketch(id) {
            self.reference_sketches.retain(|s| s.id != id);
            self.images.retain(|i| i.sketch != id);
            return Ok(());
        }
        let part = self
            .parts
            .iter_mut()
            .find(|p| p.sketches.iter().any(|s| s.id == id))
            .ok_or_else(|| DocumentError("Rascunho inexistente.".into()))?;
        if part.features.iter().any(|f| f.operation.uses_sketch(id)) {
            return Err(DocumentError(
                "O rascunho é usado pelo histórico; altere suas referências ou exclua as operações antes.".into(),
            ));
        }
        part.sketches.retain(|s| s.id != id);
        self.images.retain(|i| i.sketch != id);
        Ok(())
    }
    pub fn create_part(&mut self, mut name: String) -> Result<PartId, DocumentError> {
        validate_name(&name)?;
        let number = self.record_creation(ElementKind::Part)?;
        // Nome padrão da interface, em qualquer dos idiomas dela, recebe o número.
        if ["Peça", "Part", "Pieza"].contains(&name.as_str()) {
            name = format!("{name} {number}");
        }
        let id = PartId(self.allocate_id()?);
        let instance = InstanceId(self.allocate_id()?);
        self.parts.push(Part {
            id,
            name: name.clone(),
            bodies: Vec::new(),
            sketches: Vec::new(),
            work_planes: Vec::new(),
            parameters: parameters::Parameters {
                inherited: self.print_profile.parameters(),
                ..Default::default()
            },
            features: Vec::new(),
            history_marker: None,
            folders: PartFolders::default(),
            reference: false,
        });
        self.instances.push(Instance {
            parent: None,
            id: instance,
            part: id,
            name,
            placement: Placement::default(),
            visible: true,
            color: None,
        });
        Ok(id)
    }
    pub fn add_sketch(
        &mut self,
        destination: Option<PartId>,
        mut sketch: nator_sketch::Sketch,
    ) -> Result<(PartId, SketchId), DocumentError> {
        sketch
            .validate()
            .map_err(|e| DocumentError(e.to_string()))?;
        let part = if let Some(id) = destination {
            self.part_mut(id)?;
            id
        } else {
            self.create_part("Peça".into())?
        };
        let id = SketchId(self.allocate_id()?);
        let number = self.record_creation(ElementKind::Sketch)?;
        if ["Rascunho", "Sketch", "Boceto"].contains(&sketch.name.as_str()) {
            sketch.name = format!("{} {number}", sketch.name);
        }
        self.part_mut(part)?.sketches.push(SketchRecord {
            id,
            sketch,
            visible: true,
            support: None,
        });
        Ok((part, id))
    }
    pub fn update_sketch(
        &mut self,
        id: SketchId,
        sketch: nator_sketch::Sketch,
    ) -> Result<(), DocumentError> {
        sketch
            .validate()
            .map_err(|e| DocumentError(e.to_string()))?;
        self.sketch_mut(id)?.sketch = sketch;
        for appearance in &mut self.appearances {
            if appearance
                .layers
                .iter()
                .any(|l| l.masks().any(|m| m.sketch == id))
            {
                appearance.geometry = None;
                appearance.materials = Arc::default();
            }
        }
        Ok(())
    }
    pub fn part_mut(&mut self, id: PartId) -> Result<&mut Part, DocumentError> {
        self.parts
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| DocumentError("Peça inexistente.".into()))
    }
    pub fn body_mut(&mut self, id: BodyId) -> Result<&mut Body, DocumentError> {
        self.parts
            .iter_mut()
            .flat_map(|p| &mut p.bodies)
            .find(|b| b.id == id)
            .ok_or_else(|| DocumentError("Corpo inexistente.".into()))
    }
    pub fn instance_mut(&mut self, id: InstanceId) -> Result<&mut Instance, DocumentError> {
        self.instances
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| DocumentError("Instância inexistente.".into()))
    }
    fn allocate_id(&mut self) -> Result<u64, DocumentError> {
        let id = self.next_id;
        self.next_id = id
            .checked_add(1)
            .ok_or_else(|| DocumentError("Identificadores esgotados.".into()))?;
        Ok(id)
    }
    pub fn add_mesh(
        &mut self,
        destination: Option<PartId>,
        name: String,
        geometry: Arc<MeshAsset>,
    ) -> Result<(PartId, BodyId), DocumentError> {
        validate_name(&name)?;
        if let Some(part) = destination {
            self.part(part)
                .ok_or_else(|| DocumentError("Peça de destino inexistente.".into()))?;
        }
        let part = match destination {
            Some(id) => id,
            None => self.create_part(name.clone())?,
        };
        let body = self.allocate_body_id()?;
        self.part_mut(part)?.bodies.push(Body {
            intrinsic_colors: Default::default(),
            id: body,
            name,
            geometry,
            brep: None,
            face_names: Arc::default(),
            placement: Placement::default(),
            visible: true,
            print_down: None,
            color: None,
        });
        Ok((part, body))
    }
    /// Reserva o identificador do próximo corpo. Operações paramétricas precisam dele antes
    /// de construir o sólido, porque os nomes das faces levam o dono (`b{id}`).
    pub fn allocate_body_id(&mut self) -> Result<BodyId, DocumentError> {
        self.record_creation(ElementKind::Body)?;
        Ok(BodyId(self.allocate_id()?))
    }
    /// Insere um corpo com identificador já reservado por `allocate_body_id`.
    pub fn add_body(&mut self, part: PartId, body: Body) -> Result<(), DocumentError> {
        validate_name(&body.name)?;
        if body.id.0 == 0 || body.id.0 >= self.next_id || self.body(body.id).is_some() {
            return Err(DocumentError(
                "Identificador de corpo não reservado ou repetido.".into(),
            ));
        }
        self.part_mut(part)?.bodies.push(body);
        Ok(())
    }
    pub fn remove_body(&mut self, id: BodyId) -> Result<(), DocumentError> {
        let part = self
            .parts
            .iter_mut()
            .find(|p| p.bodies.iter().any(|b| b.id == id))
            .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
        part.bodies.retain(|b| b.id != id);
        self.appearances.retain(|a| a.body != id);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), DocumentError> {
        self.validate_images()?;
        self.validate_appearances()?;
        validate_name(&self.name)?;
        if self.next_id == 0 {
            return Err(DocumentError(
                "Contador de identificadores inválido.".into(),
            ));
        }
        uuid::Uuid::parse_str(&self.id)
            .map_err(|_| DocumentError("Identificador do projeto inválido.".into()))?;
        let count = self.parts.len()
            + self.images.len()
            + self.instances.len()
            + self.groups.len()
            + self
                .sheets
                .iter()
                .map(|s| 1 + s.annotations.len())
                .sum::<usize>()
            + self
                .parts
                .iter()
                .map(|p| {
                    p.bodies.len()
                        + p.work_planes.len()
                        + p.sketches
                            .iter()
                            .map(|s| {
                                1 + s.sketch.points.len()
                                    + s.sketch.entities.len()
                                    + s.sketch.constraints.len()
                            })
                            .sum::<usize>()
                })
                .sum::<usize>()
            + self
                .reference_sketches
                .iter()
                .map(|s| {
                    1 + s.sketch.points.len() + s.sketch.entities.len() + s.sketch.constraints.len()
                })
                .sum::<usize>();
        if count > MAX_ELEMENTS {
            return Err(DocumentError(
                "O projeto excede dez mil elementos neste protótipo.".into(),
            ));
        }
        let mut ids = HashSet::new();
        let mut register = |id: u64| {
            if id == 0 || id >= self.next_id || !ids.insert(id) {
                Err(DocumentError(
                    "Identificadores duplicados ou contador inválido.".into(),
                ))
            } else {
                Ok(())
            }
        };
        let mut total_triangles = 0_usize;
        let mut total_vertices = 0_usize;
        // Rascunhos de referência não têm parâmetros de peça: cotas só com valores diretos.
        for record in &self.reference_sketches {
            register(record.id.0)?;
            if record.support.is_some() {
                return Err(DocumentError(
                    "Rascunho de referência não pode usar plano de trabalho.".into(),
                ));
            }
            record
                .sketch
                .validate()
                .map_err(|e| DocumentError(e.to_string()))?;
            let mut resolved = record.sketch.clone();
            parameters::resolve_sketch(&mut resolved, &parameters::Parameters::default())?;
        }
        for part in &self.parts {
            register(part.id.0)?;
            validate_name(&part.name)?;
            part.parameters.validate()?;
            features::validate(part)?;
            for record in &part.sketches {
                register(record.id.0)?;
                if record
                    .support
                    .is_some_and(|id| !part.work_planes.iter().any(|w| w.id == id))
                {
                    return Err(DocumentError(
                        "Rascunho apoiado em plano de trabalho inexistente na peça.".into(),
                    ));
                }
                record
                    .sketch
                    .validate()
                    .map_err(|e| DocumentError(e.to_string()))?;
                let mut resolved = record.sketch.clone();
                parameters::resolve_sketch(&mut resolved, &part.parameters)?;
                for (expected, stored) in
                    resolved.constraints.iter().zip(&record.sketch.constraints)
                {
                    if expected.kind != stored.kind {
                        return Err(DocumentError(
                            "Uma cota não corresponde à expressão armazenada.".into(),
                        ));
                    }
                }
            }
            for record in &part.work_planes {
                register(record.id.0)?;
                validate_name(&record.name)?;
                record.plane.validate()?;
            }
            for body in &part.bodies {
                register(body.id.0)?;
                if body
                    .brep
                    .as_ref()
                    .is_some_and(|b| b.is_empty() || b.len() > 64 * 1024 * 1024)
                {
                    return Err(DocumentError("B-rep vazio ou excessivo.".into()));
                }
                if body
                    .geometry
                    .triangle_faces()
                    .is_some_and(|faces| faces.iter().any(|f| *f as usize >= body.face_names.len()))
                {
                    return Err(DocumentError(
                        "Triângulo associado a uma face inexistente no corpo.".into(),
                    ));
                }
                if !body.face_names.is_empty()
                    && (body.brep.is_none()
                        || body.face_names.len() > MAX_FACES_PER_BODY
                        || body
                            .face_names
                            .iter()
                            .any(|n| !n.is_empty() && features::validate_face_name(n).is_err()))
                {
                    return Err(DocumentError(
                        "Nomes de faces inválidos ou sem B-rep correspondente.".into(),
                    ));
                }
                if !body.intrinsic_colors.is_empty()
                    && (body.brep.is_none() || body.intrinsic_colors.len() != body.face_names.len())
                {
                    return Err(DocumentError(
                        "Cores de faces incompatíveis com o sólido.".into(),
                    ));
                }
                validate_name(&body.name)?;
                total_triangles = total_triangles
                    .checked_add(body.geometry.mesh().triangles().len())
                    .ok_or_else(|| DocumentError("Contagem geométrica excessiva.".into()))?;
                total_vertices = total_vertices
                    .checked_add(body.geometry.mesh().vertices().len())
                    .ok_or_else(|| DocumentError("Contagem geométrica excessiva.".into()))?;
            }
        }
        if total_triangles > MAX_SCENE_TRIANGLES || total_vertices > MAX_SCENE_TRIANGLES * 3 {
            return Err(DocumentError(
                "O projeto excede os limites geométricos deste protótipo.".into(),
            ));
        }
        for instance in &self.instances {
            register(instance.id.0)?;
            validate_name(&instance.name)?;
            if self.part(instance.part).is_none() {
                return Err(DocumentError(
                    "Instância referencia uma peça inexistente.".into(),
                ));
            }
        }
        for sheet in &self.sheets {
            drawing::validate(&sheet.annotations, self.next_id)?;
            register(sheet.id.0)?;
            sheet
                .specification
                .validate()
                .map_err(|e| DocumentError(e.to_string()))?;
            if sheet.target.is_some_and(|id| self.part(id).is_none()) {
                return Err(DocumentError("Folha referencia peça inexistente.".into()));
            }
        }
        for group in &self.groups {
            register(group.id.0)?;
            validate_name(&group.name)?;
            let mut members = HashSet::new();
            let mut parent = None;
            for id in &group.members {
                if !members.insert(*id) {
                    return Err(DocumentError("Elemento repetido no grupo.".into()));
                }
                let i = self.instance(*id).ok_or_else(|| {
                    DocumentError("Grupo referencia instância inexistente.".into())
                })?;
                if let Some(p) = parent {
                    if p != i.parent {
                        return Err(DocumentError(
                            "Um grupo deve pertencer a um único nível da montagem.".into(),
                        ));
                    }
                } else {
                    parent = Some(i.parent);
                }
            }
        }
        self.validate_rigid_groups()?;
        self.validate_assembly()?;
        // Instâncias também amplificam vértices não utilizados: limitar ambos os custos.
        let mut scene_triangles = 0_usize;
        let mut scene_vertices = 0_usize;
        for occurrence in self.occurrences()? {
            let instance = self
                .instance(occurrence.instance)
                .ok_or_else(|| DocumentError("Instância ausente.".into()))?;
            let part = self
                .part(instance.part)
                .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
            for body in part.shown_bodies() {
                scene_triangles = scene_triangles
                    .checked_add(body.geometry.mesh().triangles().len())
                    .ok_or_else(|| DocumentError("Cena excede o limite numérico.".into()))?;
                scene_vertices = scene_vertices
                    .checked_add(body.geometry.mesh().vertices().len())
                    .ok_or_else(|| DocumentError("Cena excede o limite numérico.".into()))?;
            }
        }
        if scene_triangles > MAX_SCENE_TRIANGLES || scene_vertices > MAX_SCENE_TRIANGLES * 3 {
            return Err(DocumentError(
                "A cena excede os limites geométricos deste protótipo.".into(),
            ));
        }
        Ok(())
    }

    /// A cena é derivada. Posições de montagem nunca são gravadas nos vértices locais.
    pub fn scene_mesh(&self) -> Result<Option<Mesh>, DocumentError> {
        Ok(self.scene()?.map(|scene| scene.mesh))
    }

    /// Cena com as arestas vivas já analisadas em cada geometria: transformações rígidas
    /// preservam ângulos, então a máscara de cada corpo vale em qualquer posição.
    pub fn scene(&self) -> Result<Option<Scene>, DocumentError> {
        self.scene_styled(|_, _| Some(Style::Normal))
    }

    /// Cena com estilo escolhido por ocorrência e peça; `None` omite a ocorrência (isolamento),
    /// sem alterar a visibilidade gravada no documento.
    pub fn scene_styled(
        &self,
        style: impl Fn(&assembly::Occurrence, PartId) -> Option<Style>,
    ) -> Result<Option<Scene>, DocumentError> {
        self.scene_styled_bodies(|occurrence, part, _| style(occurrence, part))
    }

    /// Como `scene_styled`, com o estilo escolhido também por corpo (por exemplo, para
    /// destacar o corpo selecionado na peça em construção).
    pub fn scene_styled_bodies(
        &self,
        style: impl Fn(&assembly::Occurrence, PartId, BodyId) -> Option<Style>,
    ) -> Result<Option<Scene>, DocumentError> {
        self.scene_inspected(style, None)
    }

    /// Malha que a cena mostra para o corpo: a pintura pode subdividir a geometria, e as
    /// análises coloridas precisam dos mesmos triângulos.
    pub fn display_geometry(&self, body: BodyId) -> Option<&Arc<MeshAsset>> {
        self.body_appearance(body)
            .and_then(|a| a.geometry.as_ref())
            .or_else(|| self.body(body).map(|b| &b.geometry))
    }

    /// Como `scene_styled_bodies`, com as cores de `inspection` sobre o corpo analisado.
    /// Uma análise de outra revisão, com outros triângulos, é ignorada.
    pub fn scene_inspected(
        &self,
        style: impl Fn(&assembly::Occurrence, PartId, BodyId) -> Option<Style>,
        inspection: Option<&Inspection>,
    ) -> Result<Option<Scene>, DocumentError> {
        self.validate()?;
        // Dois grupos (opacos e translúcidos), concatenados no fim.
        let mut groups: [SceneGroup; 2] = Default::default();
        for occurrence in self.occurrences()? {
            let instance = self
                .instance(occurrence.instance)
                .ok_or_else(|| DocumentError("Instância ausente.".into()))?;
            let part = self
                .part(instance.part)
                .ok_or_else(|| DocumentError("Peça da instância inexistente.".into()))?;
            let occurrence_color = occurrence
                .path
                .iter()
                .filter_map(|id| self.instance(*id).and_then(|i| i.color))
                .next_back();
            for body in part.shown_bodies() {
                let Some(look) = style(&occurrence, part.id, body.id) else {
                    continue;
                };
                let group = &mut groups[usize::from(look.is_translucent())];
                let appearance = self
                    .body_appearance(body.id)
                    .filter(|a| a.geometry.is_some());
                let geometry = appearance
                    .and_then(|a| a.geometry.as_ref())
                    .unwrap_or(&body.geometry);
                let mesh = geometry.mesh();
                let base = u32::try_from(group.vertices.len())
                    .map_err(|_| DocumentError("Cena com vértices excessivos.".into()))?;
                group.vertices.extend(
                    mesh.vertices()
                        .iter()
                        .map(|point| occurrence.placement.apply(body.placement.apply(*point))),
                );
                group
                    .ranges
                    .push(group.triangles.len()..group.triangles.len() + mesh.triangles().len());
                group
                    .triangles
                    .extend(mesh.triangles().iter().map(|t| t.map(|i| i + base)));
                group.creases.extend_from_slice(geometry.creases());
                let painted_base = self.body_appearance(body.id).and_then(|a| a.base_color);
                let fallback = painted_base
                    .or(occurrence_color)
                    .or(body.color)
                    .unwrap_or(RgbColor::NEUTRAL);
                let first_color = group.colors.len();
                let intrinsic = |triangle: usize| {
                    geometry
                        .triangle_faces()
                        .and_then(|faces| faces.get(triangle))
                        .and_then(|face| body.intrinsic_colors.get(*face as usize))
                        .copied()
                        .flatten()
                };
                if let Some(appearance) = appearance {
                    group
                        .colors
                        .extend(
                            appearance
                                .materials
                                .iter()
                                .enumerate()
                                .map(|(triangle, m)| {
                                    if *m == 0 {
                                        painted_base
                                            .or_else(|| intrinsic(triangle))
                                            .unwrap_or(fallback)
                                    } else {
                                        appearance.layers[usize::from(*m) - 1].color
                                    }
                                }),
                        );
                } else {
                    group.colors.extend(
                        (0..mesh.triangles().len())
                            .map(|i| painted_base.or_else(|| intrinsic(i)).unwrap_or(fallback)),
                    );
                }
                if let Some(inspection) = inspection
                    .filter(|i| i.body == body.id && i.colors.len() == mesh.triangles().len())
                {
                    let painted = group.colors[first_color..].iter_mut();
                    for (color, analysis) in painted.zip(&inspection.colors) {
                        *color = analysis.unwrap_or(*color);
                    }
                }
                group
                    .styles
                    .extend(std::iter::repeat_n(look, mesh.triangles().len()));
            }
        }
        let [mut scene, translucent] = groups;
        let opaque_triangles = scene.triangles.len();
        let offset = u32::try_from(scene.vertices.len())
            .map_err(|_| DocumentError("Cena com vértices excessivos.".into()))?;
        scene.vertices.extend(translucent.vertices);
        scene
            .triangles
            .extend(translucent.triangles.iter().map(|t| t.map(|i| i + offset)));
        if scene.triangles.is_empty() {
            return Ok(None);
        }
        scene.ranges.extend(
            translucent
                .ranges
                .into_iter()
                .map(|r| r.start + opaque_triangles..r.end + opaque_triangles),
        );
        scene.creases.extend(translucent.creases);
        scene.styles.extend(translucent.styles);
        scene.colors.extend(translucent.colors);
        let mesh =
            Mesh::new(scene.vertices, scene.triangles).map_err(|e| DocumentError(e.to_string()))?;
        Ok(Some(Scene {
            mesh,
            creases: scene.creases,
            styles: scene.styles,
            colors: scene.colors,
            opaque_triangles,
            body_ranges: scene.ranges,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_names_are_numbered_in_every_interface_language() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Nomes")?;
        let english = doc.create_part("Part".into())?;
        let spanish = doc.create_part("Pieza".into())?;
        let named = doc.create_part("Tampa".into())?;
        assert_eq!(doc.part(english).ok_or("Peça")?.name, "Part 1");
        assert_eq!(doc.part(spanish).ok_or("Peça")?.name, "Pieza 2");
        assert_eq!(doc.part(named).ok_or("Peça")?.name, "Tampa");
        let (_, sketch) = doc.add_sketch(
            Some(english),
            nator_sketch::Sketch::new("Sketch", nator_sketch::Plane::xy()),
        )?;
        assert_eq!(
            doc.sketch(sketch).ok_or("Rascunho")?.sketch.name,
            "Sketch 1"
        );
        Ok(())
    }

    #[test]
    fn units_convert_presentation_without_changing_millimeters() {
        let unit = LengthUnit::Inch;
        assert_eq!(unit.to_mm(2.), 50.8);
        assert!((unit.from_mm(25.4) - 1.).abs() < 1e-12);
        assert!((unit.from_mm2(645.16) - 1.).abs() < 1e-12);
        assert_eq!(unit.expression(12.7), "0.5 in");
        assert_eq!(LengthUnit::Millimeter.expression(0.1 + 0.2), "0.3 mm");
    }

    #[test]
    fn numbers_typed_without_unit_are_stored_with_the_selected_unit() -> Result<(), DocumentError> {
        use parameters::{Dimension, Parameters};
        let unit = LengthUnit::Inch;
        assert_eq!(unit.normalize_expression(" 12,5 "), "12,5 in");
        assert_eq!(unit.normalize_expression("-3"), "-3 in");
        assert_eq!(unit.normalize_expression("2 + 3"), "(2 + 3) * 1 in");
        assert_eq!(unit.normalize_expression("5 mm"), "5 mm");
        assert_eq!(
            unit.normalize_expression("espessura + 1 mm"),
            "espessura + 1 mm"
        );
        assert_eq!(unit.normalize_expression(""), "");
        assert_eq!(LengthUnit::Millimeter.normalize_expression("12"), "12");
        let parameters = Parameters::default();
        for (typed, unit, mm) in [
            ("1", LengthUnit::Inch, 25.4),
            ("2 + 3", LengthUnit::Inch, 127.),
            ("0,5", LengthUnit::Inch, 12.7),
            ("7", LengthUnit::Millimeter, 7.),
        ] {
            let expression = unit.normalize_expression(typed);
            assert!((parameters.evaluate(&expression, Dimension::Length)? - mm).abs() < 1e-9);
        }
        Ok(())
    }

    #[test]
    fn displayed_decimals_never_hide_the_mouse_precision() {
        let mut settings = Settings {
            unit: LengthUnit::Inch,
            mouse_step_mm: 25.4 / 64.,
            ..Settings::default()
        };
        assert_eq!(Settings::default().mouse_step_mm, 0.1);
        // 1/64 in = 0,015625: as casas da precisão, limitadas a três além das exibidas.
        assert_eq!(settings.length_decimals(), 5);
        settings.display_decimals = 3;
        assert_eq!(settings.length_decimals(), 6);
        assert_eq!(settings.format_length(25.4 / 64.), "0,015625");
        settings.unit = LengthUnit::Millimeter;
        settings.display_decimals = 2;
        settings.mouse_step_mm = 0.01;
        assert_eq!(settings.length_decimals(), 2);
        settings.display_decimals = 4;
        assert_eq!(settings.format_length(12.5), "12,5000");
        assert_eq!(settings.format_compact_length(12.5), "12,5");
        settings.unit = LengthUnit::Inch;
        assert_eq!(settings.length_label(12.7), "0,5 in");
        settings.mouse_angle_step_deg = 0.25;
        assert_eq!(settings.angle_step_decimals(), 2);
    }

    #[test]
    fn mouse_precision_is_validated_with_the_document() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Precisão")?;
        doc.set_settings(Settings {
            mouse_step_mm: 0.01,
            mouse_angle_step_deg: 0.01,
            unit: LengthUnit::Inch,
            ..Settings::default()
        })?;
        assert_eq!(doc.settings().snap_length(1.2345), 1.23);
        for settings in [
            Settings {
                mouse_step_mm: 0.,
                ..Settings::default()
            },
            Settings {
                mouse_step_mm: f64::NAN,
                ..Settings::default()
            },
            Settings {
                mouse_angle_step_deg: 91.,
                ..Settings::default()
            },
        ] {
            assert!(doc.set_settings(settings).is_err());
        }
        assert_eq!(doc.settings().unit, LengthUnit::Inch);
        Ok(())
    }

    #[test]
    fn styled_scene_orders_opaque_first_and_omits_isolated_out() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Estilos")?;
        let asset = Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?));
        let (part, _) = doc.add_mesh(None, "Suporte".into(), Arc::clone(&asset))?;
        let second = doc.add_instance(part, None, Placement::new([100., 0., 0.], [0.; 3])?)?;
        let third = doc.add_instance(part, None, Placement::new([200., 0., 0.], [0.; 3])?)?;
        let scene = doc
            .scene_styled(|occurrence, _| match occurrence.instance {
                id if id == second => Some(Style::Transparent),
                id if id == third => None,
                _ => Some(Style::Normal),
            })?
            .ok_or("Cena ausente")?;
        assert_eq!(scene.mesh.triangles().len(), 40);
        assert_eq!(scene.opaque_triangles, 20);
        assert_eq!(scene.body_ranges, vec![0..20, 20..40]);
        assert_eq!(scene.styles[0], Style::Normal);
        assert_eq!(scene.styles[39], Style::Transparent);
        // A instância isolada fora da cena não entra na caixa envolvente.
        assert_eq!(scene.mesh.bounds().max[0], 160.);
        // O documento continua com as três instâncias visíveis.
        assert!(doc.instances().iter().all(|i| i.visible));
        Ok(())
    }

    #[test]
    fn prevents_vertex_amplification_by_instances() -> Result<(), Box<dyn Error>> {
        let mut vertices = vec![[0.; 3]; 1000];
        vertices[1] = [1., 0., 0.];
        vertices[2] = [0., 1., 0.];
        let asset = Arc::new(MeshAsset::new(Mesh::new(vertices, vec![[0, 1, 2]])?));
        let mut doc = Document::new("Ensaio")?;
        let (part, _) = doc.add_mesh(None, "Base".into(), asset)?;
        for id in 4..6004 {
            doc.instances.push(Instance {
                parent: None,
                id: InstanceId(id),
                part,
                name: format!("Instância {id}"),
                placement: Placement::default(),
                visible: true,
                color: None,
            });
        }
        doc.next_id = 6004;
        assert!(doc.validate().is_err());
        Ok(())
    }
    #[test]
    fn assembly_placement_preserves_body_coordinates() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Ensaio")?;
        let asset = Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?));
        let (_, body) = doc.add_mesh(None, "Suporte".into(), asset)?;
        let instance = doc.instances()[0].id;
        doc.instance_mut(instance)?.placement = Placement::new([100., 0., 0.], [0.; 3])?;
        let scene = doc.scene_mesh()?.ok_or("Cena ausente")?;
        assert_eq!(scene.bounds().min, [100., 0., 0.]);
        assert_eq!(
            doc.body(body)
                .ok_or("Corpo ausente")?
                .geometry
                .mesh()
                .bounds()
                .min,
            [0.; 3]
        );
        Ok(())
    }
    #[test]
    fn reference_sketches_live_outside_parts() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Referências")?;
        let mut layout = nator_sketch::Sketch::new("Layout", nator_sketch::Plane::xy());
        layout.add_line([0., 0.], [100., 0.], false)?;
        let id = doc.add_reference_sketch(layout)?;
        assert!(doc.is_reference_sketch(id));
        assert_eq!(doc.sketch_part(id), None);
        assert!(doc.parts().is_empty());
        doc.sketch_mut(id)?.visible = false;
        assert!(!doc.sketch(id).ok_or("Rascunho ausente")?.visible);
        doc.validate()?;
        doc.remove_sketch(id)?;
        assert!(doc.sketch(id).is_none());
        Ok(())
    }

    #[test]
    fn rejects_dangling_and_duplicate_identifiers() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Ensaio")?;
        let (part, _) = doc.add_mesh(
            None,
            "Suporte".into(),
            Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
        )?;
        let mut broken = doc.clone();
        broken.instances[0].part = PartId(u64::MAX);
        assert!(broken.validate().is_err());
        doc.parts
            .push(doc.part(part).ok_or("Peça ausente")?.clone());
        assert!(doc.validate().is_err());
        Ok(())
    }
}
