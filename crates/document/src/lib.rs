//! Documento independente da interface. Corpos mantêm malhas locais compartilhadas por Arc.
//! Instâncias referenciam definições de peças; nunca duplicam a geometria automaticamente.
use nator_geometry::{Mesh, Placement, RgbColor, Topology};
use std::{collections::HashSet, error::Error, fmt, sync::Arc};
pub mod assembly;
pub mod drawing;
pub mod features;
pub mod parameters;
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SheetRecord {
    pub id: SheetId,
    pub target: Option<PartId>,
    pub specification: nator_drawing::Specification,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<drawing::DrawingAnnotation>,
}

/// Configurações do documento, salvas no projeto. Só afetam a apresentação: a precisão
/// interna continua sendo a de `f64` em milímetros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    /// Casas decimais exibidas em medidas, propriedades e cotas da interface (0 a 6).
    pub display_decimals: u8,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            display_decimals: 2,
        }
    }
}
/// Limite de casas decimais exibidas.
pub const MAX_DISPLAY_DECIMALS: u8 = 6;

#[derive(Debug, Clone)]
pub struct SketchRecord {
    pub id: SketchId,
    pub sketch: nator_sketch::Sketch,
    pub visible: bool,
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
    pub placement: Placement,
    pub visible: bool,
    /// Cor sólida definida pelo usuário ou importada; ausência usa a aparência neutra.
    pub color: Option<RgbColor>,
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
}
impl Part {
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
}
impl Default for PartFolders {
    fn default() -> Self {
        Self {
            bodies: true,
            subparts: true,
            sketches: true,
            planes: true,
            origin: false,
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
}
impl Default for ProjectFolders {
    fn default() -> Self {
        Self {
            project: true,
            parts: true,
            sketches: true,
            origin: true,
        }
    }
}

/// Nó da árvore com visibilidade própria, além de peças, corpos e rascunhos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Folder {
    Project,
    Parts,
    ReferenceSketches,
    Origin,
    Bodies(PartId),
    Subparts(PartId),
    Sketches(PartId),
    WorkPlanes(PartId),
    PartOrigin(PartId),
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
    parts: Vec<Part>,
    instances: Vec<Instance>,
    groups: Vec<assembly::Group>,
    sheets: Vec<SheetRecord>,
    settings: Settings,
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
            parts: Vec::new(),
            instances: Vec::new(),
            groups: Vec::new(),
            sheets: Vec::new(),
            settings: Settings::default(),
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
        let document = Self {
            id,
            name,
            next_id,
            parts,
            instances,
            groups: Vec::new(),
            sheets: Vec::new(),
            settings: Settings::default(),
            reference_sketches: Vec::new(),
            folders: ProjectFolders::default(),
        };
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
            Folder::Origin => self.folders.origin,
            Folder::Bodies(id) => part(id)?.bodies,
            Folder::Subparts(id) => part(id)?.subparts,
            Folder::Sketches(id) => part(id)?.sketches,
            Folder::WorkPlanes(id) => part(id)?.planes,
            Folder::PartOrigin(id) => part(id)?.origin,
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
            Folder::Origin => &mut self.folders.origin,
            Folder::Bodies(id) => &mut self.part_mut(id)?.folders.bodies,
            Folder::Subparts(id) => &mut self.part_mut(id)?.folders.subparts,
            Folder::Sketches(id) => &mut self.part_mut(id)?.folders.sketches,
            Folder::WorkPlanes(id) => &mut self.part_mut(id)?.folders.planes,
            Folder::PartOrigin(id) => &mut self.part_mut(id)?.folders.origin,
        };
        *flag = visible;
        Ok(())
    }
    /// Instância exibida no seu nível: visível e com a pasta que a contém visível (Peças,
    /// na raiz, dentro do projeto; Subpeças, dentro de uma peça). Os níveis acima são
    /// conferidos ao percorrer a montagem (`occurrences`).
    pub fn shows_instance(&self, instance: &Instance) -> bool {
        instance.visible
            && match instance.parent {
                None => self.folders.project && self.folders.parts,
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
    pub fn set_settings(&mut self, settings: Settings) -> Result<(), DocumentError> {
        if settings.display_decimals > MAX_DISPLAY_DECIMALS {
            return Err(DocumentError(
                "As casas decimais exibidas devem ficar entre 0 e 6.".into(),
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
        sketch: nator_sketch::Sketch,
    ) -> Result<SketchId, DocumentError> {
        sketch
            .validate()
            .map_err(|e| DocumentError(e.to_string()))?;
        let id = SketchId(self.allocate_id()?);
        self.reference_sketches.push(SketchRecord {
            id,
            sketch,
            visible: true,
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
    pub fn remove_sketch(&mut self, id: SketchId) -> Result<(), DocumentError> {
        if self.is_reference_sketch(id) {
            self.reference_sketches.retain(|s| s.id != id);
            return Ok(());
        }
        let part = self
            .parts
            .iter_mut()
            .find(|p| p.sketches.iter().any(|s| s.id == id))
            .ok_or_else(|| DocumentError("Rascunho inexistente.".into()))?;
        if part
            .features
            .iter()
            .any(|f| f.operation.sketch() == Some(id))
        {
            return Err(DocumentError(
                "O rascunho é perfil de operações do histórico; exclua-as antes.".into(),
            ));
        }
        part.sketches.retain(|s| s.id != id);
        Ok(())
    }
    pub fn create_part(&mut self, name: String) -> Result<PartId, DocumentError> {
        validate_name(&name)?;
        let id = PartId(self.allocate_id()?);
        let instance = InstanceId(self.allocate_id()?);
        self.parts.push(Part {
            id,
            name: name.clone(),
            bodies: Vec::new(),
            sketches: Vec::new(),
            work_planes: Vec::new(),
            parameters: parameters::Parameters::default(),
            features: Vec::new(),
            history_marker: None,
            folders: PartFolders::default(),
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
        sketch: nator_sketch::Sketch,
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
        self.part_mut(part)?.sketches.push(SketchRecord {
            id,
            sketch,
            visible: true,
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
            None => {
                let id = PartId(self.allocate_id()?);
                let instance = InstanceId(self.allocate_id()?);
                self.parts.push(Part {
                    id,
                    name: name.clone(),
                    bodies: Vec::new(),
                    sketches: Vec::new(),
                    work_planes: Vec::new(),
                    parameters: parameters::Parameters::default(),
                    features: Vec::new(),
                    history_marker: None,
                    folders: PartFolders::default(),
                });
                self.instances.push(Instance {
                    parent: None,
                    id: instance,
                    part: id,
                    name: name.clone(),
                    placement: Placement::default(),
                    visible: true,
                    color: None,
                });
                id
            }
        };
        let body = self.allocate_body_id()?;
        self.part_mut(part)?.bodies.push(Body {
            id: body,
            name,
            geometry,
            brep: None,
            face_names: Arc::default(),
            placement: Placement::default(),
            visible: true,
            color: None,
        });
        Ok((part, body))
    }
    /// Reserva o identificador do próximo corpo. Operações paramétricas precisam dele antes
    /// de construir o sólido, porque os nomes das faces levam o dono (`b{id}`).
    pub fn allocate_body_id(&mut self) -> Result<BodyId, DocumentError> {
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
        Ok(())
    }

    pub fn validate(&self) -> Result<(), DocumentError> {
        validate_name(&self.name)?;
        if self.next_id == 0 {
            return Err(DocumentError(
                "Contador de identificadores inválido.".into(),
            ));
        }
        uuid::Uuid::parse_str(&self.id)
            .map_err(|_| DocumentError("Identificador do projeto inválido.".into()))?;
        let count = self.parts.len()
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
                let mesh = body.geometry.mesh();
                let base = u32::try_from(group.vertices.len())
                    .map_err(|_| DocumentError("Cena com vértices excessivos.".into()))?;
                group.vertices.extend(
                    mesh.vertices()
                        .iter()
                        .map(|point| occurrence.placement.apply(body.placement.apply(*point))),
                );
                group
                    .triangles
                    .extend(mesh.triangles().iter().map(|t| t.map(|i| i + base)));
                group.creases.extend_from_slice(body.geometry.creases());
                group.colors.extend(std::iter::repeat_n(
                    occurrence_color.or(body.color).unwrap_or(RgbColor::NEUTRAL),
                    mesh.triangles().len(),
                ));
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
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
