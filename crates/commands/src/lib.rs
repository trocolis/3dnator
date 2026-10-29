//! Transações do documento e coordenação do histórico paramétrico, com desfazer/refazer.
mod appearance;
mod assembly;
pub mod drawing;
pub mod export;
pub mod extrusion;
mod features;
pub mod measure;
pub mod point_snap;
pub mod sketch_projection;
use nator_document::{
    BodyId, Document, DocumentError, InstanceId, MeshAsset, PartId, SketchId, WorkPlaneId,
    validate_name,
};
use nator_geometry::Placement;
use std::{collections::VecDeque, sync::Arc};

#[derive(Debug, Clone)]
pub enum Command {
    PaintSurface {
        body: BodyId,
        paint: nator_document::SurfacePaint,
    },
    ClearSurfacePaint {
        body: BodyId,
        face: nator_document::features::FaceRef,
    },
    AddReferenceImage {
        image: nator_document::ReferenceImage,
    },
    UpdateReferenceImage {
        image: nator_document::ReferenceImage,
    },
    RemoveReferenceImage {
        id: nator_document::ImageId,
    },
    /// Escolhe o modo pelo contato geométrico antes de publicar uma operação concreta.
    /// A sugestão não é persistida como regra implícita: editar o histórico mantém o modo.
    AddSmartExtrusion {
        part: PartId,
        name: String,
        operation: nator_document::features::Operation,
        preferred: Option<BodyId>,
    },
    /// A leitura e a tesselação ocorrem na preparação do comando, na thread de trabalho.
    ImportStep {
        path: std::path::PathBuf,
        destination: Option<PartId>,
        name: String,
    },
    SetSheet {
        id: Option<nator_document::SheetId>,
        target: Option<PartId>,
        specification: nator_drawing::Specification,
        annotations: Vec<nator_document::drawing::DrawingAnnotation>,
    },
    ImportSolid {
        destination: Option<PartId>,
        name: String,
        geometry: Arc<MeshAsset>,
        brep: Arc<Vec<u8>>,
        /// Quantidade de faces do B-rep; recebem nomes posicionais com o dono do novo corpo.
        faces: usize,
    },
    AddFeature {
        part: PartId,
        name: String,
        operation: nator_document::features::Operation,
    },
    EditFeature {
        part: PartId,
        output: BodyId,
        operation: nator_document::features::Operation,
    },
    /// Exclui a operação e seu resultado. Com `dependents`, exclui também as operações que
    /// dependem dele; sem isso, a existência de dependentes é um erro explícito.
    RemoveFeature {
        part: PartId,
        output: BodyId,
        dependents: bool,
    },
    /// Suprime ou reativa uma operação, preservando parâmetros e referências.
    SetFeatureSuppressed {
        part: PartId,
        output: BodyId,
        suppressed: bool,
    },
    RenameFeature {
        part: PartId,
        output: BodyId,
        name: String,
    },
    /// Move a operação para `index` no histórico, desde que tudo o que ela usa continue
    /// antes dela e tudo o que usa o seu resultado continue depois.
    MoveFeature {
        part: PartId,
        output: BodyId,
        index: usize,
    },
    /// Quantidade de operações construídas; `None` (ou o fim) reativa todo o histórico.
    SetHistoryMarker {
        part: PartId,
        marker: Option<usize>,
    },
    /// Comandos aplicados em uma só ação de desfazer. Os recálculos ficam para o fim, de modo
    /// que uma edição e a supressão ou correção de outra operação sejam avaliadas juntas.
    Batch {
        label: &'static str,
        commands: Vec<Command>,
    },
    /// Instâncias adicionais da mesma definição, em padrão linear ou circular a partir da
    /// posição da instância de origem, reunidas em um grupo.
    PatternInstance {
        id: InstanceId,
        layout: nator_document::features::PatternLayout,
    },
    /// Versão espelhada de uma instância como peça nova e independente. O plano é dado nas
    /// coordenadas do pai da instância.
    MirrorInstance {
        id: InstanceId,
        origin: [f64; 3],
        normal: [f64; 3],
        name: String,
    },
    AddInstance {
        part: PartId,
        parent: Option<PartId>,
        placement: Placement,
    },
    /// Peça nova e independente a partir da instância, posicionada ao lado dela.
    CloneInstance {
        id: InstanceId,
        name: String,
    },
    /// Nova instância da mesma definição, posicionada ao lado da original.
    DuplicateInstance {
        id: InstanceId,
    },
    ReparentInstance {
        id: InstanceId,
        parent: Option<PartId>,
    },
    RemoveInstance {
        id: InstanceId,
    },
    CreateGroup {
        name: String,
        members: Vec<InstanceId>,
    },
    MoveGroup {
        id: nator_document::GroupId,
        delta: Placement,
    },
    RemoveGroup {
        id: nator_document::GroupId,
    },
    SetParameter {
        part: PartId,
        name: String,
        expression: Option<String>,
        dimension: nator_document::parameters::Dimension,
    },
    AddSketch {
        destination: Option<PartId>,
        sketch: nator_sketch::Sketch,
    },
    UpdateSketch {
        id: SketchId,
        sketch: nator_sketch::Sketch,
    },
    ImportMesh {
        destination: Option<PartId>,
        name: String,
        geometry: Arc<MeshAsset>,
    },
    RenamePart {
        id: PartId,
        name: String,
    },
    RenameBody {
        id: BodyId,
        name: String,
    },
    SetBodyColor {
        id: BodyId,
        color: Option<nator_geometry::RgbColor>,
    },
    SetSketchVisibility {
        id: SketchId,
        visible: bool,
    },
    SetBodyVisibility {
        id: BodyId,
        visible: bool,
    },
    /// Visibilidade de uma pasta da árvore (peças, rascunhos, corpos, subpeças, origens).
    SetFolderVisibility {
        folder: nator_document::Folder,
        visible: bool,
    },
    SetInstanceVisibility {
        id: InstanceId,
        visible: bool,
    },
    MoveBody {
        id: BodyId,
        placement: Placement,
    },
    MoveInstance {
        id: InstanceId,
        placement: Placement,
    },
    RemoveBody {
        id: BodyId,
    },
    /// Cópia independente do corpo na mesma peça, sem histórico, na posição local dada.
    CopyBody {
        id: BodyId,
        placement: Placement,
    },
    RenameInstance {
        id: InstanceId,
        name: String,
    },
    RenameGroup {
        id: nator_document::GroupId,
        name: String,
    },
    /// Rascunho de referência da montagem, no plano dado em coordenadas da montagem.
    AddReferenceSketch {
        sketch: nator_sketch::Sketch,
    },
    /// Exclui um rascunho de referência, ou de peça que nenhuma operação usa.
    RemoveSketch {
        id: SketchId,
    },
    RenameSketch {
        id: SketchId,
        name: String,
    },
    /// Plano de trabalho na peça: um plano-base, em coordenadas da peça, deslocado ao
    /// longo da sua normal.
    AddWorkPlane {
        part: PartId,
        name: String,
        plane: nator_document::WorkPlane,
    },
    /// Nova definição do plano (plano-base e distância). Os rascunhos apoiados nele são
    /// movidos juntos e o histórico da peça é recalculado na mesma transação.
    UpdateWorkPlane {
        id: WorkPlaneId,
        plane: nator_document::WorkPlane,
    },
    SetWorkPlaneVisibility {
        id: WorkPlaneId,
        visible: bool,
    },
    /// Exclui o plano. Rascunhos criados nele permanecem onde estão, sem o vínculo.
    RemoveWorkPlane {
        id: WorkPlaneId,
    },
    /// Rascunho novo apoiado no plano de trabalho, na peça dona dele. Usar o plano o oculta
    /// na mesma transação, como uma operação oculta o perfil que consome.
    AddSketchOnWorkPlane {
        id: WorkPlaneId,
        name: String,
    },
    /// Peça nova e vazia, com uma instância na raiz da montagem, ao lado das existentes.
    CreatePart {
        name: String,
    },
    CreatePartIn {
        name: String,
        parent: Option<PartId>,
    },
    /// Configurações do documento (apresentação).
    SetSettings {
        settings: nator_document::Settings,
    },
    /// Cópia independente de uma definição de outro documento aberto, inserida na raiz da
    /// montagem ou como subpeça de `parent`, em `placement` ou ao lado, se sobrepuser.
    InsertPart {
        source: Arc<Document>,
        part: PartId,
        parent: Option<PartId>,
        placement: Placement,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Effect {
    Changed,
    Imported { part: PartId, body: BodyId },
    SketchCreated { part: PartId, sketch: SketchId },
    WorkPlaneCreated { part: PartId, plane: WorkPlaneId },
    ReferenceSketchCreated { sketch: SketchId },
    PartCreated { part: PartId },
    SheetSaved { id: nator_document::SheetId },
}

/// Falha do recálculo atribuída a uma operação do histórico. Permite oferecer a correção da
/// operação, sua supressão ou o cancelamento da edição que a provocou.
#[derive(Debug, Clone, PartialEq)]
pub struct RebuildFailure {
    pub part: PartId,
    pub output: BodyId,
    pub name: String,
    pub message: String,
    /// Operações que dependem da que falhou, direta ou indiretamente.
    pub dependents: Vec<BodyId>,
}

/// Erro de um comando. Falhas de recálculo identificam a operação responsável.
#[derive(Debug, Clone)]
pub struct CommandError {
    pub message: String,
    pub failure: Option<RebuildFailure>,
}
impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for CommandError {}
impl From<DocumentError> for CommandError {
    fn from(error: DocumentError) -> Self {
        Self {
            message: error.0,
            failure: None,
        }
    }
}

/// Recálculos pedidos pelos comandos: imediatos, ou reunidos até o fim de um lote.
struct Rebuilds {
    deferred: Option<Vec<PartId>>,
}
impl Rebuilds {
    fn immediate() -> Self {
        Self { deferred: None }
    }
    fn deferred() -> Self {
        Self {
            deferred: Some(Vec::new()),
        }
    }
    fn request(&mut self, doc: &mut Document, part: PartId) -> Result<(), CommandError> {
        match &mut self.deferred {
            Some(parts) => {
                if !parts.contains(&part) {
                    parts.push(part);
                }
                Ok(())
            }
            None => features::rebuild(doc, part),
        }
    }
    fn into_parts(self) -> Vec<PartId> {
        self.deferred.unwrap_or_default()
    }
}

impl Command {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PaintSurface { .. } => "Pintar superfície",
            Self::ClearSurfacePaint { .. } => "Restaurar aparência da superfície",
            Self::AddReferenceImage { .. } => "Adicionar imagem de referência",
            Self::UpdateReferenceImage { .. } => "Editar imagem de referência",
            Self::RemoveReferenceImage { .. } => "Excluir imagem de referência",
            Self::ImportStep { .. } => "Importar montagem STEP",
            Self::SetSheet { .. } => "Salvar folha técnica associada",
            Self::ImportSolid { .. } => "Importar sólido CAD",
            Self::AddFeature { .. } | Self::AddSmartExtrusion { .. } => {
                "Adicionar operação paramétrica"
            }
            Self::EditFeature { .. } => "Editar operação paramétrica",
            Self::RemoveFeature { .. } => "Remover operação paramétrica",
            Self::SetFeatureSuppressed {
                suppressed: true, ..
            } => "Suprimir operação",
            Self::SetFeatureSuppressed { .. } => "Reativar operação",
            Self::RenameFeature { .. } => "Renomear operação",
            Self::MoveFeature { .. } => "Reordenar histórico",
            Self::SetHistoryMarker { .. } => "Mover marcador do histórico",
            Self::Batch { label, .. } => label,
            Self::AddInstance { .. } => "Adicionar instância",
            Self::PatternInstance { .. } => "Padrão de instâncias",
            Self::MirrorInstance { .. } => "Espelhar instância como nova peça",
            Self::CloneInstance { .. } => "Clonar peça independente",
            Self::DuplicateInstance { .. } => "Nova instância",
            Self::ReparentInstance { .. } => "Organizar subpeça",
            Self::RemoveInstance { .. } => "Remover instância",
            Self::CreateGroup { .. } => "Criar grupo organizacional",
            Self::MoveGroup { .. } => "Mover grupo",
            Self::RemoveGroup { .. } => "Desagrupar elementos",
            Self::SetParameter { .. } => "Alterar parâmetro da peça",
            Self::AddSketch { .. } => "Criar rascunho",
            Self::UpdateSketch { .. } => "Editar rascunho",
            Self::SetSketchVisibility { .. } => "Alterar visibilidade do rascunho",
            Self::ImportMesh { .. } => "Importar corpo de malha",
            Self::RenamePart { .. } => "Renomear peça",
            Self::RenameBody { .. } => "Renomear corpo",
            Self::SetBodyColor { .. } => "Alterar cor do corpo",
            Self::SetBodyVisibility { .. } | Self::SetInstanceVisibility { .. } => {
                "Alterar visibilidade"
            }
            Self::SetFolderVisibility { .. } => "Alterar visibilidade da pasta",
            Self::MoveBody { .. } => "Posicionar corpo",
            Self::MoveInstance { .. } => "Posicionar instância",
            Self::RemoveBody { .. } => "Excluir corpo",
            Self::CopyBody { .. } => "Copiar corpo",
            Self::RenameInstance { .. } => "Renomear instância",
            Self::RenameGroup { .. } => "Renomear grupo",
            Self::SetSettings { .. } => "Alterar configurações do documento",
            Self::AddReferenceSketch { .. } => "Criar rascunho de referência",
            Self::RemoveSketch { .. } => "Excluir rascunho",
            Self::RenameSketch { .. } => "Renomear rascunho",
            Self::AddWorkPlane { .. } => "Criar plano deslocado",
            Self::UpdateWorkPlane { .. } => "Editar plano deslocado",
            Self::SetWorkPlaneVisibility { .. } => "Alterar visibilidade do plano",
            Self::RemoveWorkPlane { .. } => "Excluir plano de trabalho",
            Self::AddSketchOnWorkPlane { .. } => "Criar rascunho no plano",
            Self::CreatePart { .. } => "Criar peça",
            Self::CreatePartIn { .. } => "Criar peça na pasta",
            Self::InsertPart { .. } => "Inserir peça de outro projeto",
        }
    }
    fn apply(self, doc: &mut Document, rebuilds: &mut Rebuilds) -> Result<Effect, CommandError> {
        match self {
            Self::PaintSurface { body, paint } => {
                let mut layers = doc
                    .body_appearance(body)
                    .map(|a| a.layers.clone())
                    .unwrap_or_default();
                if layers.len() >= 64 {
                    return Err(DocumentError("Limite de pinturas atingido.".into()).into());
                }
                layers.push(paint);
                let appearance = appearance::build(doc, body, layers)?;
                doc.set_surface_appearance(appearance)?;
            }
            Self::ClearSurfacePaint { body, face } => {
                let layers = doc
                    .body_appearance(body)
                    .map(|a| {
                        a.layers
                            .iter()
                            .filter(|l| l.face != face)
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if layers.is_empty() {
                    doc.remove_surface_appearance(body);
                } else {
                    let appearance = appearance::build(doc, body, layers)?;
                    doc.set_surface_appearance(appearance)?;
                }
            }

            Self::AddReferenceImage { image } => {
                doc.add_image(image)?;
            }
            Self::UpdateReferenceImage { image } => doc.update_image(image)?,
            Self::RemoveReferenceImage { id } => doc.remove_image(id)?,
            Self::ImportStep {
                path,
                destination,
                name,
            } => {
                // As raízes importadas vão juntas para o lado das ocorrências existentes.
                let before = doc.instances().iter().map(|i| i.id).collect::<Vec<_>>();
                let effect = assembly::import_step(doc, &path, destination, name)?;
                let roots = doc
                    .instances()
                    .iter()
                    .filter(|i| i.parent == destination && !before.contains(&i.id))
                    .map(|i| i.id)
                    .collect::<Vec<_>>();
                doc.place_beside(&roots)?;
                return Ok(effect);
            }
            Self::SetSheet {
                id,
                target,
                specification,
                annotations,
            } => {
                let id = doc.set_sheet(id, target, specification, annotations)?;
                return Ok(Effect::SheetSaved { id });
            }
            Self::ImportSolid {
                destination,
                name,
                geometry,
                brep,
                faces,
            } => {
                if faces == 0 || faces > nator_document::MAX_FACES_PER_BODY {
                    return Err(DocumentError(
                        "Quantidade de faces do sólido importado inválida.".into(),
                    )
                    .into());
                }
                let (part, body) = doc.add_mesh(destination, name, geometry)?;
                place_new_part(doc, destination, part)?;
                let imported = doc.body_mut(body)?;
                imported.brep = Some(brep);
                // A geometria importada nunca é recalculada: nomes posicionais são estáveis.
                imported.face_names = Arc::new(nator_kernel::indexed_face_names(
                    &nator_document::features::body_owner(body),
                    faces,
                ));
                return Ok(Effect::Imported { part, body });
            }
            Self::AddSmartExtrusion {
                part,
                name,
                operation,
                preferred,
            } => {
                let operation = extrusion::automatic(doc, part, operation, preferred)?;
                let output = add_feature(doc, part, name, operation)?;
                return Ok(Effect::Imported { part, body: output });
            }
            Self::AddFeature {
                part,
                name,
                operation,
            } => {
                let output = add_feature(doc, part, name, operation)?;
                return Ok(Effect::Imported { part, body: output });
            }
            Self::PatternInstance { id, layout } => {
                let source = doc
                    .instance(id)
                    .ok_or_else(|| DocumentError("Instância inexistente.".into()))?
                    .clone();
                // Montagens não têm parâmetros: medidas com unidade são avaliadas diretamente.
                let steps = nator_document::features::pattern_placements(
                    &layout,
                    &nator_document::parameters::Parameters::default(),
                )?;
                let mut members = vec![id];
                for step in steps.iter().skip(1) {
                    let placement = step
                        .compose(source.placement)
                        .map_err(|e| DocumentError(e.to_string()))?;
                    let copy = doc.add_instance(source.part, source.parent, placement)?;
                    doc.instance_mut(copy)?.color = source.color;
                    members.push(copy);
                }
                // O grupo move o padrão inteiro; desagrupar libera cada posição.
                doc.add_group(format!("Padrão de {}", source.name), members)?;
            }
            Self::MirrorInstance {
                id,
                origin,
                normal,
                name,
            } => assembly::mirror_instance(doc, id, origin, normal, name)?,
            Self::EditFeature {
                part,
                output,
                operation,
            } => {
                let operation = features::normalize(doc, part, operation)?;
                let feature = doc
                    .part_mut(part)?
                    .features
                    .iter_mut()
                    .find(|f| f.output == output)
                    .ok_or_else(|| DocumentError("Operação inexistente.".into()))?;
                feature.operation = operation;
                nator_document::features::validate(
                    doc.part(part)
                        .ok_or_else(|| DocumentError("Peça inexistente.".into()))?,
                )?;
                rebuilds.request(doc, part)?;
            }
            Self::RemoveFeature {
                part,
                output,
                dependents,
            } => features::remove(doc, part, output, dependents)?,
            Self::SetFeatureSuppressed {
                part,
                output,
                suppressed,
            } => {
                let piece = doc.part_mut(part)?;
                let before = nator_document::features::hidden_by_history(piece);
                piece
                    .features
                    .iter_mut()
                    .find(|f| f.output == output)
                    .ok_or_else(|| DocumentError("Operação inexistente.".into()))?
                    .suppressed = suppressed;
                nator_document::features::sync_visibility(piece, &before);
                // Suprimir não altera as operações que continuam ativas: nenhuma delas usa o
                // resultado suprimido. Reativar exige reconstruir o que volta ao histórico.
                if !suppressed {
                    rebuilds.request(doc, part)?;
                }
            }
            Self::RenameFeature { part, output, name } => {
                validate_name(&name)?;
                doc.part_mut(part)?
                    .features
                    .iter_mut()
                    .find(|f| f.output == output)
                    .ok_or_else(|| DocumentError("Operação inexistente.".into()))?
                    .name = name;
            }
            Self::MoveFeature {
                part,
                output,
                index,
            } => {
                features::reorder(doc.part_mut(part)?, output, index)?;
                rebuilds.request(doc, part)?;
            }
            Self::SetHistoryMarker { part, marker } => {
                let piece = doc.part_mut(part)?;
                let count = piece.features.len();
                let previous = piece.history_marker.unwrap_or(count);
                let marker = marker.filter(|m| *m < count);
                let before = nator_document::features::hidden_by_history(piece);
                piece.history_marker = marker;
                nator_document::features::sync_visibility(piece, &before);
                // Recuar o marcador não muda as operações anteriores a ele. Avançar traz de
                // volta operações cuja geometria pode estar desatualizada.
                if marker.unwrap_or(count) > previous {
                    rebuilds.request(doc, part)?;
                }
            }
            Self::Batch { commands, .. } => {
                let mut inner = Rebuilds::deferred();
                let mut effect = Effect::Changed;
                for command in commands {
                    effect = command.apply(doc, &mut inner)?;
                }
                for part in inner.into_parts() {
                    rebuilds.request(doc, part)?;
                }
                return Ok(effect);
            }
            Self::AddInstance {
                part,
                parent,
                placement,
            } => {
                doc.add_instance(part, parent, placement)?;
            }
            Self::CloneInstance { id, name } => {
                let source = doc
                    .instance(id)
                    .ok_or_else(|| DocumentError("Instância inexistente.".into()))?
                    .clone();
                let part = doc.clone_part(source.part, name)?;
                let copy = doc.add_instance(part, source.parent, source.placement)?;
                doc.instance_mut(copy)?.color = source.color;
                doc.place_beside(&[copy])?;
            }
            Self::DuplicateInstance { id } => {
                let source = doc
                    .instance(id)
                    .ok_or_else(|| DocumentError("Instância inexistente.".into()))?
                    .clone();
                let copy = doc.add_instance(source.part, source.parent, source.placement)?;
                doc.instance_mut(copy)?.color = source.color;
                doc.place_beside(&[copy])?;
            }
            Self::ReparentInstance { id, parent } => doc.reparent(id, parent)?,
            Self::RemoveInstance { id } => doc.remove_instance(id)?,
            Self::CreateGroup { name, members } => {
                doc.add_group(name, members)?;
            }
            Self::MoveGroup { id, delta } => doc.move_group(id, delta)?,
            Self::RemoveGroup { id } => doc.remove_group(id)?,
            Self::SetParameter {
                part,
                name,
                expression,
                dimension,
            } => {
                let part_id = part;
                let part = doc.part_mut(part)?;
                if let Some(expression) = expression {
                    part.parameters.set(name, expression, dimension)?;
                } else {
                    part.parameters.entries.remove(&name);
                    part.parameters.validate()?;
                }
                for record in &mut part.sketches {
                    nator_document::parameters::resolve_sketch(
                        &mut record.sketch,
                        &part.parameters,
                    )?;
                    let report = nator_sketch::solver::solve(&record.sketch)
                        .map_err(|e| DocumentError(e.to_string()))?;
                    if report.status == nator_sketch::solver::Status::Conflict {
                        return Err(DocumentError(
                            "A alteração do parâmetro conflita com as restrições do rascunho."
                                .into(),
                        )
                        .into());
                    }
                    record.sketch = report.sketch;
                }
                rebuilds.request(doc, part_id)?;
            }
            Self::AddSketch {
                destination,
                sketch,
            } => {
                if nator_sketch::solver::max_error(&sketch)
                    .map_err(|e| DocumentError(e.to_string()))?
                    > 1e-6
                {
                    return Err(DocumentError(
                        "Resolva as restrições antes de criar o rascunho.".into(),
                    )
                    .into());
                }
                let (part, sketch) = doc.add_sketch(destination, sketch)?;
                return Ok(Effect::SketchCreated { part, sketch });
            }
            Self::UpdateSketch { id, sketch } => {
                if nator_sketch::solver::max_error(&sketch)
                    .map_err(|e| DocumentError(e.to_string()))?
                    > 1e-6
                {
                    return Err(DocumentError(
                        "Resolva as restrições antes de confirmar o rascunho.".into(),
                    )
                    .into());
                }
                doc.update_sketch(id, sketch)?;
                // Rascunhos de referência não alimentam operações: nada a recalcular.
                if let Some(part) = doc.sketch_part(id) {
                    rebuilds.request(doc, part)?;
                }
            }
            Self::ImportMesh {
                destination,
                name,
                geometry,
            } => {
                let (part, body) = doc.add_mesh(destination, name, geometry)?;
                place_new_part(doc, destination, part)?;
                return Ok(Effect::Imported { part, body });
            }
            Self::RenamePart { id, name } => {
                validate_name(&name)?;
                doc.part_mut(id)?.name = name;
            }
            Self::RenameBody { id, name } => {
                validate_name(&name)?;
                doc.body_mut(id)?.name = name;
            }
            Self::SetBodyColor { id, color } => doc.body_mut(id)?.color = color,
            Self::SetSketchVisibility { id, visible } => doc.sketch_mut(id)?.visible = visible,
            Self::SetBodyVisibility { id, visible } => doc.body_mut(id)?.visible = visible,
            Self::SetInstanceVisibility { id, visible } => doc.instance_mut(id)?.visible = visible,
            Self::SetFolderVisibility { folder, visible } => {
                doc.set_folder_visible(folder, visible)?;
            }
            Self::MoveBody { id, placement } => {
                doc.body_mut(id)?.placement = placement;
                let part = doc
                    .parts()
                    .iter()
                    .find(|p| p.bodies.iter().any(|b| b.id == id))
                    .map(|p| p.id)
                    .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
                rebuilds.request(doc, part)?;
            }
            Self::MoveInstance { id, placement } => doc.instance_mut(id)?.placement = placement,
            Self::RemoveBody { id } => {
                let part = doc
                    .parts()
                    .iter()
                    .find(|p| p.bodies.iter().any(|b| b.id == id))
                    .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
                if part
                    .features
                    .iter()
                    .any(|f| f.output == id || f.operation.references().contains(&id))
                {
                    return Err(DocumentError(
                        "O corpo participa do histórico; remova a operação correspondente.".into(),
                    )
                    .into());
                }
                doc.remove_body(id)?;
            }
            Self::CopyBody { id, placement } => {
                let part = doc
                    .parts()
                    .iter()
                    .find(|p| p.bodies.iter().any(|b| b.id == id))
                    .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
                if nator_document::features::inactive_outputs(part).contains(&id) {
                    return Err(DocumentError(
                        "O resultado de uma operação inativa não pode ser copiado.".into(),
                    )
                    .into());
                }
                let part = part.id;
                let body = doc.copy_body(id, placement)?;
                crate::appearance::rebuild(doc, part)?;
                return Ok(Effect::Imported { part, body });
            }
            Self::RenameInstance { id, name } => {
                validate_name(&name)?;
                doc.instance_mut(id)?.name = name;
            }
            Self::RenameGroup { id, name } => doc.rename_group(id, name)?,
            Self::SetSettings { settings } => doc.set_settings(settings)?,
            Self::AddReferenceSketch { sketch } => {
                if nator_sketch::solver::max_error(&sketch)
                    .map_err(|e| DocumentError(e.to_string()))?
                    > 1e-6
                {
                    return Err(DocumentError(
                        "Resolva as restrições antes de criar o rascunho.".into(),
                    )
                    .into());
                }
                let sketch = doc.add_reference_sketch(sketch)?;
                return Ok(Effect::ReferenceSketchCreated { sketch });
            }
            Self::RemoveSketch { id } => doc.remove_sketch(id)?,
            Self::RenameSketch { id, name } => {
                nator_document::validate_name(&name)?;
                doc.sketch_mut(id)?.sketch.name = name;
            }
            Self::AddWorkPlane { part, name, plane } => {
                let plane = doc.add_work_plane(part, name, plane)?;
                return Ok(Effect::WorkPlaneCreated { part, plane });
            }
            Self::UpdateWorkPlane { id, plane } => {
                let (part, moved) = doc.update_work_plane(id, plane)?;
                // Sem rascunhos apoiados, nenhuma operação depende do plano.
                if moved {
                    rebuilds.request(doc, part)?;
                }
            }
            Self::SetWorkPlaneVisibility { id, visible } => {
                doc.work_plane_mut(id)?.visible = visible;
            }
            Self::RemoveWorkPlane { id } => doc.remove_work_plane(id)?,
            Self::AddSketchOnWorkPlane { id, name } => {
                let (part, sketch) = doc.add_sketch_on_work_plane(id, name)?;
                return Ok(Effect::SketchCreated { part, sketch });
            }
            Self::CreatePart { name } => {
                let part = doc.create_part(name)?;
                place_new_part(doc, None, part)?;
                return Ok(Effect::PartCreated { part });
            }
            Self::CreatePartIn { name, parent } => {
                let part = doc.create_part(name)?;
                if let Some(parent) = parent {
                    let instance = doc
                        .instances()
                        .iter()
                        .find(|i| i.part == part && i.parent.is_none())
                        .map(|i| i.id)
                        .ok_or_else(|| DocumentError("Instância da peça nova ausente.".into()))?;
                    doc.reparent(instance, Some(parent))?;
                }
                place_new_part(doc, parent, part)?;
                return Ok(Effect::PartCreated { part });
            }
            Self::InsertPart {
                source,
                part,
                parent,
                placement,
            } => {
                if source.id() == doc.id() {
                    return Err(DocumentError(
                        "Para repetir uma peça do mesmo projeto, use uma nova instância.".into(),
                    )
                    .into());
                }
                let copy = doc.import_part(&source, part)?;
                let instance = doc.add_instance(copy, parent, placement)?;
                doc.place_beside(&[instance])?;
            }
        }
        Ok(Effect::Changed)
    }
}

/// Peça nova na raiz (sem destino de construção): sua instância vai para o lado das
/// ocorrências existentes. Corpos acrescentados a uma peça ficam onde foram lidos.
fn place_new_part(
    doc: &mut Document,
    destination: Option<PartId>,
    part: PartId,
) -> Result<(), DocumentError> {
    if destination.is_some() {
        return Ok(());
    }
    let instances = doc
        .instances()
        .iter()
        .filter(|i| i.part == part)
        .map(|i| i.id)
        .collect::<Vec<_>>();
    doc.place_beside(&instances)
}

/// Acrescenta uma operação ao histórico da peça e devolve o corpo resultante.
fn add_feature(
    doc: &mut Document,
    part: PartId,
    name: String,
    operation: nator_document::features::Operation,
) -> Result<BodyId, DocumentError> {
    // O identificador vem antes da construção porque os nomes das faces levam o corpo dono;
    // em caso de falha, a cópia candidata do documento é descartada inteira.
    let output = doc.allocate_body_id()?;
    let piece = doc
        .part(part)
        .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
    // A nova operação entra no marcador do histórico e só pode usar resultados construídos.
    let inactive = nator_document::features::inactive_outputs(piece);
    if operation.references().iter().any(|b| inactive.contains(b)) {
        return Err(DocumentError(
            "A operação usa o resultado de uma operação suprimida ou posterior ao marcador do histórico."
                .into(),
        ));
    }
    let operation = features::normalize(doc, part, operation)?;
    let built = features::construct(doc, part, output, &operation)?;
    // O resultado é uma nova etapa geométrica, não um novo corpo para o usuário:
    // refinamentos preservam o nome e a aparência da entrada principal. O nome da
    // operação pertence à linha do tempo; criações sem entrada ganham seu próprio nome.
    // Edições posteriores de nome/cor do resultado não são sobrescritas pelo recálculo.
    let source = operation.inputs().first().and_then(|id| doc.body(*id));
    let body_name = source.map(|b| b.name.clone()).unwrap_or_else(|| {
        format!(
            "Corpo {}",
            doc.next_name_number(nator_document::ElementKind::Body) - 1
        )
    });
    let color = source.and_then(|b| b.color);
    doc.add_body(
        part,
        nator_document::Body {
            id: output,
            name: body_name,
            geometry: built.mesh,
            brep: Some(built.brep),
            face_names: built.faces,
            placement: Placement::default(),
            visible: true,
            color,
        },
    )?;
    for input in operation.inputs() {
        doc.body_mut(input)?.visible = false;
    }
    // Ocultar apenas na criação bem-sucedida da operação. Recalcular não deve
    // apagar a escolha posterior do usuário de mostrar o perfil novamente.
    if let Some(sketch) = operation.sketch() {
        doc.sketch_mut(sketch)?.visible = false;
    }
    let piece = doc.part_mut(part)?;
    let feature = nator_document::features::Feature {
        name,
        output,
        operation,
        suppressed: false,
    };
    match piece.history_marker {
        Some(marker) => {
            piece.features.insert(marker, feature);
            piece.history_marker = Some(marker + 1);
        }
        None => piece.features.push(feature),
    }
    Ok(output)
}

#[derive(Debug, Clone)]
struct Snapshot {
    document: Document,
    state: u64,
    label: String,
}

/// Preparação pode calcular parâmetros/soluções em outra thread, sem alterar a sessão.
pub struct PreparedCommand {
    document: Document,
    revision: u64,
    effect: Effect,
    label: String,
}
impl PreparedCommand {
    pub fn new(
        mut document: Document,
        revision: u64,
        command: Command,
    ) -> Result<Self, CommandError> {
        let label = command.label().to_owned();
        let effect = command.apply(&mut document, &mut Rebuilds::immediate())?;
        document.validate()?;
        Ok(Self {
            document,
            revision,
            effect,
            label,
        })
    }
    /// Documento candidato, para pré-visualizar o resultado antes de publicá-lo.
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn effect(&self) -> Effect {
        self.effect
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

/// Snapshots compartilham as malhas. Desfazer não mantém cópias completas dos vértices.
pub struct Session {
    document: Document,
    undo: VecDeque<Snapshot>,
    redo: Vec<Snapshot>,
    revision: u64,
    state: u64,
    next_state: u64,
    saved_state: Option<u64>,
}

impl Session {
    pub fn new(document: Document) -> Result<Self, DocumentError> {
        document.validate()?;
        Ok(Self {
            document,
            undo: VecDeque::new(),
            redo: Vec::new(),
            revision: 0,
            state: 0,
            next_state: 1,
            saved_state: Some(0),
        })
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn state(&self) -> u64 {
        self.state
    }
    pub fn is_dirty(&self) -> bool {
        self.saved_state != Some(self.state)
    }
    pub fn mark_saved(&mut self, state: u64) {
        self.saved_state = Some(state);
    }
    pub fn mark_unsaved(&mut self) {
        self.saved_state = None;
    }
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|s| s.label.as_str())
    }
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|s| s.label.as_str())
    }
    pub fn execute(&mut self, command: Command) -> Result<Effect, CommandError> {
        let prepared = PreparedCommand::new(self.document.clone(), self.revision, command)?;
        Ok(self.commit(prepared)?)
    }
    pub fn commit(&mut self, prepared: PreparedCommand) -> Result<Effect, DocumentError> {
        if prepared.revision != self.revision || prepared.document.id() != self.document.id() {
            return Err(DocumentError(
                "A edição preparada pertence a outro documento ou revisão.".into(),
            ));
        }
        let PreparedCommand {
            document: candidate,
            effect,
            label,
            ..
        } = prepared;
        let next_state = self
            .next_state
            .checked_add(1)
            .ok_or_else(|| DocumentError("Estados da sessão esgotados.".into()))?;
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| DocumentError("Revisões esgotadas.".into()))?;
        // Somente a partir daqui a edição é publicada: falhas anteriores não alteram o documento.
        self.undo.push_back(Snapshot {
            document: std::mem::replace(&mut self.document, candidate),
            state: self.state,
            label,
        });
        if self.undo.len() > 100 {
            self.undo.pop_front();
        }
        self.redo.clear();
        self.state = self.next_state;
        self.next_state = next_state;
        self.revision = next_revision;
        Ok(effect)
    }
    pub fn undo(&mut self) -> bool {
        let Some(next_revision) = self.revision.checked_add(1) else {
            return false;
        };
        let Some(previous) = self.undo.pop_back() else {
            return false;
        };
        self.redo.push(Snapshot {
            document: std::mem::replace(&mut self.document, previous.document),
            state: self.state,
            label: previous.label,
        });
        self.state = previous.state;
        self.revision = next_revision;
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(next_revision) = self.revision.checked_add(1) else {
            return false;
        };
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push_back(Snapshot {
            document: std::mem::replace(&mut self.document, next.document),
            state: self.state,
            label: next.label,
        });
        self.state = next.state;
        self.revision = next_revision;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn sketch_visibility_is_an_independent_undoable_edit() -> Result<(), Box<dyn Error>> {
        let mut session = Session::new(Document::new("Visibilidade")?)?;
        let Effect::SketchCreated { sketch: id, .. } = session.execute(Command::AddSketch {
            destination: None,
            sketch: nator_sketch::Sketch::new("Perfil", nator_sketch::Plane::xy()),
        })?
        else {
            return Err("Rascunho ausente".into());
        };
        session.mark_saved(session.state());
        session.execute(Command::SetSketchVisibility { id, visible: false })?;
        assert!(session.is_dirty());
        assert!(
            !session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .visible
        );
        assert!(session.undo());
        assert!(!session.is_dirty());
        assert!(
            session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .visible
        );
        assert!(session.redo());
        assert!(
            !session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .visible
        );
        assert!(
            session
                .execute(Command::SetSketchVisibility {
                    id: SketchId(u64::MAX),
                    visible: true
                })
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn sketches_on_work_planes_follow_the_plane_and_hide_it_in_one_undo()
    -> Result<(), Box<dyn Error>> {
        use nator_sketch::Plane;
        let mut session = Session::new(Document::new("Plano deslocado")?)?;
        let Effect::PartCreated { part } = session.execute(Command::CreatePart {
            name: "Suporte".into(),
        })?
        else {
            return Err("Peça ausente".into());
        };
        let base = Plane::from_normal([10., 0., 0.], [1., 0., 0.])?;
        let Effect::WorkPlaneCreated { part: owner, plane } =
            session.execute(Command::AddWorkPlane {
                part,
                name: "Plano 1".into(),
                plane: nator_document::WorkPlane { base, offset: 15. },
            })?
        else {
            return Err("Plano ausente".into());
        };
        assert_eq!(owner, part);
        assert!(
            session
                .document()
                .work_plane(plane)
                .is_some_and(|w| w.visible)
        );
        let Effect::SketchCreated {
            part: owner,
            sketch,
        } = session.execute(Command::AddSketchOnWorkPlane {
            id: plane,
            name: "Perfil".into(),
        })?
        else {
            return Err("Rascunho ausente".into());
        };
        assert_eq!(owner, part);
        let support = session
            .document()
            .sketch(sketch)
            .ok_or("Rascunho ausente")?
            .sketch
            .plane;
        assert_eq!(support.origin, [25., 0., 0.]);
        assert_eq!((support.u, support.v), (base.u, base.v));
        assert!(
            session
                .document()
                .work_plane(plane)
                .is_some_and(|w| !w.visible)
        );
        // Desfazer remove o rascunho e devolve a visibilidade numa única ação.
        assert!(session.undo());
        assert!(session.document().sketch(sketch).is_none());
        assert!(
            session
                .document()
                .work_plane(plane)
                .is_some_and(|w| w.visible)
        );
        assert!(session.redo());
        // Editar a distância leva o rascunho apoiado junto, em uma ação de desfazer.
        session.execute(Command::UpdateWorkPlane {
            id: plane,
            plane: nator_document::WorkPlane { base, offset: 40. },
        })?;
        let moved = |session: &Session| {
            session
                .document()
                .sketch(sketch)
                .map(|record| record.sketch.plane.origin)
        };
        assert_eq!(moved(&session), Some([50., 0., 0.]));
        assert!(session.undo());
        assert_eq!(moved(&session), Some([25., 0., 0.]));
        assert!(session.redo());
        // Excluir o plano preserva o rascunho na última posição, sem o vínculo.
        session.execute(Command::RemoveWorkPlane { id: plane })?;
        assert_eq!(moved(&session), Some([50., 0., 0.]));
        assert!(session.document().work_plane(plane).is_none());
        assert!(session.document().sketch(sketch).is_some());
        assert!(
            session
                .execute(Command::AddSketchOnWorkPlane {
                    id: plane,
                    name: "Outro".into(),
                })
                .is_err()
        );
        assert!(
            session
                .execute(Command::AddWorkPlane {
                    part,
                    name: "Inválido".into(),
                    plane: nator_document::WorkPlane {
                        base,
                        offset: f64::NAN,
                    },
                })
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn unresolved_sketch_edits_do_not_change_the_document() -> Result<(), Box<dyn Error>> {
        use nator_sketch::{ConstraintKind, Curve, Plane, Sketch};
        let mut session = Session::new(Document::new("Ensaio")?)?;
        let mut sketch = Sketch::new("Cotas", Plane::xy());
        let line = sketch.add_line([0., 0.], [10., 0.], false)?;
        let Curve::Line { start, end } = sketch.entity(line)?.curve else {
            return Err("Linha ausente".into());
        };
        sketch.add_constraint(
            ConstraintKind::Distance {
                a: start,
                b: end,
                value: 20.,
            },
            false,
        )?;
        assert!(
            session
                .execute(Command::AddSketch {
                    destination: None,
                    sketch: sketch.clone()
                })
                .is_err()
        );
        assert!(session.document().parts().is_empty());
        let sketch = nator_sketch::solver::solve(&sketch)?.sketch;
        session.execute(Command::AddSketch {
            destination: None,
            sketch,
        })?;
        assert!(session.undo());
        assert!(session.document().parts().is_empty());
        Ok(())
    }

    #[test]
    fn prepared_results_reject_stale_revisions() -> Result<(), Box<dyn Error>> {
        let mut session = Session::new(Document::new("Ensaio")?)?;
        let prepared =
            PreparedCommand::new(session.document().clone(), session.revision(), import()?)?;
        session.execute(import()?)?;
        assert!(session.commit(prepared).is_err());
        assert_eq!(session.document().parts().len(), 1);
        Ok(())
    }
    #[test]
    fn a_late_save_only_marks_its_own_snapshot_clean() -> Result<(), Box<dyn Error>> {
        let mut session = Session::new(Document::new("Ensaio")?)?;
        session.execute(import()?)?;
        let saved = session.state();
        let id = session.document().parts()[0].bodies[0].id;
        session.execute(Command::RenameBody {
            id,
            name: "Nova edição".into(),
        })?;
        session.mark_saved(saved);
        assert!(session.is_dirty());
        assert!(session.undo());
        assert!(!session.is_dirty());
        Ok(())
    }

    #[test]
    fn body_color_survives_saving_cloning_and_undo() -> Result<(), Box<dyn Error>> {
        let mut session = Session::new(Document::new("Cores")?)?;
        session.execute(import()?)?;
        let body = session.document().parts()[0].bodies[0].id;
        let instance = session.document().instances()[0].id;
        let color = nator_geometry::RgbColor::new([220, 35, 70]);
        session.execute(Command::SetBodyColor {
            id: body,
            color: Some(color),
        })?;
        assert!(
            session
                .document()
                .scene()?
                .ok_or("Cena ausente")?
                .colors
                .iter()
                .all(|c| *c == color)
        );
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("cores.3dnator");
        nator_project_io::write_project(&path, session.document())?;
        assert_eq!(
            nator_project_io::read_project(&path)?
                .body(body)
                .ok_or("Corpo ausente")?
                .color,
            Some(color)
        );
        session.execute(Command::CloneInstance {
            id: instance,
            name: "Variação".into(),
        })?;
        assert_eq!(session.document().parts()[1].bodies[0].color, Some(color));
        assert!(session.undo());
        assert!(session.undo());
        assert_eq!(
            session.document().body(body).ok_or("Corpo ausente")?.color,
            None
        );
        assert!(session.redo());
        assert_eq!(
            session.document().body(body).ok_or("Corpo ausente")?.color,
            Some(color)
        );
        Ok(())
    }
    #[test]
    fn sheet_layout_and_stable_section_ids_survive_saving_and_undo() -> Result<(), Box<dyn Error>> {
        use nator_drawing::{Orientation, Section, SectionView, Specification};
        let mut session = Session::new(Document::new("Cortes")?)?;
        session.execute(import()?)?;
        let part = session.document().parts()[0].id;
        let spec = Specification {
            orientation: Orientation::Portrait,
            png_dpi: 96,
            additional_sections: vec![SectionView {
                id: 3,
                section: Section {
                    origin: [0., 0., 3.],
                    normal: [0., 0., 1.],
                },
            }],
            ..Default::default()
        };
        session.execute(Command::SetSheet {
            id: None,
            target: Some(part),
            specification: spec,
            annotations: Vec::new(),
        })?;
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("cortes.3dnator");
        nator_project_io::write_project(&path, session.document())?;
        let reopened = nator_project_io::read_project(&path)?;
        let spec = &reopened.sheets()[0].specification;
        assert_eq!(spec.orientation, Orientation::Portrait);
        assert_eq!(spec.png_dpi, 96);
        assert_eq!(spec.additional_sections[0].id, 3);
        let sheet = crate::drawing::generate(&reopened, Some(part), spec)?;
        assert!(
            sheet.primitives.iter().any(
                |p| matches!(p,nator_drawing::Primitive::Text {value,..} if value=="CORTE D-D")
            )
        );
        assert!(session.undo());
        assert!(session.document().sheets().is_empty());
        assert!(session.redo());
        assert_eq!(
            session.document().sheets()[0]
                .specification
                .additional_sections[0]
                .id,
            3
        );
        Ok(())
    }
    fn import() -> Result<Command, Box<dyn Error>> {
        Ok(Command::ImportMesh {
            destination: None,
            name: "Suporte".into(),
            geometry: Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
        })
    }
    #[test]
    fn undo_redo_restore_saved_state_and_share_geometry() -> Result<(), Box<dyn Error>> {
        let mut session = Session::new(Document::new("Ensaio")?)?;
        session.execute(import()?)?;
        session.mark_saved(session.state());
        let body = session.document().parts()[0].bodies[0].id;
        let asset = Arc::clone(
            &session
                .document()
                .body(body)
                .ok_or("Corpo ausente")?
                .geometry,
        );
        session.execute(Command::RenameBody {
            id: body,
            name: "Base".into(),
        })?;
        assert!(session.is_dirty());
        assert!(session.undo());
        assert!(!session.is_dirty());
        assert!(Arc::ptr_eq(
            &asset,
            &session
                .document()
                .body(body)
                .ok_or("Corpo ausente")?
                .geometry
        ));
        assert!(session.redo());
        assert_eq!(
            session.document().body(body).ok_or("Corpo ausente")?.name,
            "Base"
        );
        assert!(session.undo());
        session.execute(Command::RemoveBody { id: body })?;
        assert!(!session.redo());
        assert!(session.undo());
        assert!(!session.is_dirty());
        Ok(())
    }
    #[test]
    fn instance_patterns_share_the_definition_inside_a_group() -> Result<(), Box<dyn Error>> {
        use nator_document::features::{LinearAxis, PatternLayout};
        let mut session = Session::new(Document::new("Montagem")?)?;
        session.execute(import()?)?;
        let instance = session.document().instances()[0].id;
        session.execute(Command::PatternInstance {
            id: instance,
            layout: PatternLayout::Linear {
                first: LinearAxis {
                    direction: [1., 0., 0.],
                    count: 3,
                    spacing: "100 mm".into(),
                },
                second: None,
            },
        })?;
        let doc = session.document();
        assert_eq!(doc.parts().len(), 1);
        assert_eq!(doc.instances().len(), 3);
        assert_eq!(doc.groups()[0].members.len(), 3);
        let scene = doc.scene_mesh()?.ok_or("Cena ausente")?;
        assert_eq!(scene.bounds().max[0], 260.);
        assert!(session.undo());
        assert_eq!(session.document().instances().len(), 1);
        Ok(())
    }

    #[test]
    fn mirrored_mesh_instance_is_an_independent_reflected_part() -> Result<(), Box<dyn Error>> {
        let mut session = Session::new(Document::new("Montagem")?)?;
        session.execute(import()?)?;
        let instance = session.document().instances()[0].id;
        session.execute(Command::MoveInstance {
            id: instance,
            placement: Placement::new([10., 0., 0.], [0., 0., 0.3])?,
        })?;
        session.execute(Command::MirrorInstance {
            id: instance,
            origin: [0.; 3],
            normal: [1., 0., 0.],
            name: "Suporte esquerdo".into(),
        })?;
        let doc = session.document();
        assert_eq!(doc.parts().len(), 2);
        let original = doc.parts()[0].bodies[0].geometry.mesh();
        let mirrored = doc.parts()[1].bodies[0].geometry.mesh();
        assert!((mirrored.signed_volume() - original.signed_volume()).abs() < 1e-9);
        assert!(mirrored.topology().has_closed_oriented_edges());
        // A cena da imagem é a reflexão da cena original pelo plano x = 0.
        let world = |part: usize| -> Result<nator_geometry::Bounds, Box<dyn Error>> {
            let mut copy = doc.clone();
            let keep = copy.instances()[part].id;
            for other in copy.instances().iter().map(|i| i.id).collect::<Vec<_>>() {
                copy.instance_mut(other)?.visible = other == keep;
            }
            Ok(copy.scene_mesh()?.ok_or("Cena ausente")?.bounds())
        };
        let (a, b) = (world(0)?, world(1)?);
        assert!((a.min[0] + b.max[0]).abs() < 1e-9 && (a.max[0] + b.min[0]).abs() < 1e-9);
        assert!((a.min[1] - b.min[1]).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn copied_bodies_and_renamed_occurrences_are_independent_and_undoable()
    -> Result<(), Box<dyn Error>> {
        let mut session = Session::new(Document::new("Cópias")?)?;
        session.execute(import()?)?;
        let part = session.document().parts()[0].id;
        let body = session.document().parts()[0].bodies[0].id;
        let instance = session.document().instances()[0].id;
        let placement = Placement::new([0., 40., 0.], [0.; 3])?;
        let Effect::Imported { body: copy, .. } = session.execute(Command::CopyBody {
            id: body,
            placement,
        })?
        else {
            return Err("Cópia sem corpo novo".into());
        };
        let doc = session.document();
        assert_eq!(doc.part(part).ok_or("Peça ausente")?.bodies.len(), 2);
        let copied = doc.body(copy).ok_or("Cópia ausente")?;
        assert_eq!(copied.placement, placement);
        assert!(Arc::ptr_eq(
            &copied.geometry,
            &doc.body(body).ok_or("Corpo ausente")?.geometry
        ));
        session.execute(Command::RenameInstance {
            id: instance,
            name: "Suporte esquerdo".into(),
        })?;
        assert_eq!(
            session
                .document()
                .instance(instance)
                .ok_or("Instância ausente")?
                .name,
            "Suporte esquerdo"
        );
        // A definição mantém o nome; renomear a ocorrência não renomeia a peça.
        assert_eq!(
            session.document().part(part).ok_or("Peça ausente")?.name,
            "Suporte"
        );
        let second = session.document().instances()[0].id;
        session.execute(Command::AddInstance {
            part,
            parent: None,
            placement: Placement::new([100., 0., 0.], [0.; 3])?,
        })?;
        let third = session.document().instances()[1].id;
        session.execute(Command::CreateGroup {
            name: "Pares".into(),
            members: vec![second, third],
        })?;
        let group = session.document().groups()[0].id;
        session.execute(Command::RenameGroup {
            id: group,
            name: "Laterais".into(),
        })?;
        assert_eq!(session.document().groups()[0].name, "Laterais");
        assert!(session.undo());
        assert_eq!(session.document().groups()[0].name, "Pares");
        Ok(())
    }

    #[test]
    fn parts_from_another_project_are_inserted_as_independent_copies() -> Result<(), Box<dyn Error>>
    {
        let mut source = Session::new(Document::new("Origem")?)?;
        source.execute(import()?)?;
        let part = source.document().parts()[0].id;
        let mut target = Session::new(Document::new("Destino")?)?;
        target.execute(import()?)?;
        let host = target.document().parts()[0].id;
        let shared = Arc::new(source.document().clone());
        target.execute(Command::InsertPart {
            source: Arc::clone(&shared),
            part,
            parent: Some(host),
            placement: Placement::new([0., 0., 40.], [0.; 3])?,
        })?;
        let doc = target.document();
        assert_eq!(doc.parts().len(), 2);
        let inserted = doc
            .instances()
            .iter()
            .find(|i| i.parent == Some(host))
            .ok_or("Subpeça ausente")?;
        assert_ne!(inserted.part, part);
        assert!(doc.scene_mesh()?.ok_or("Cena ausente")?.bounds().max[2] > 40.);
        // A origem não é alterada e o mesmo projeto não insere a si mesmo.
        assert_eq!(source.document().parts().len(), 1);
        assert!(
            source
                .execute(Command::InsertPart {
                    source: shared,
                    part,
                    parent: None,
                    placement: Placement::default(),
                })
                .is_err()
        );
        assert!(target.undo());
        assert_eq!(target.document().parts().len(), 1);
        Ok(())
    }

    #[test]
    fn reference_sketches_and_new_parts_are_undoable_assembly_edits() -> Result<(), Box<dyn Error>>
    {
        use nator_sketch::{Plane, Sketch};
        let mut session = Session::new(Document::new("Montagem")?)?;
        let mut layout = Sketch::new("Layout", Plane::xy());
        layout.add_line([0., 0.], [50., 0.], false)?;
        let Effect::ReferenceSketchCreated { sketch } =
            session.execute(Command::AddReferenceSketch {
                sketch: layout.clone(),
            })?
        else {
            return Err("Rascunho de referência ausente".into());
        };
        // Nenhuma peça é criada para o rascunho de referência.
        assert!(session.document().parts().is_empty());
        layout.add_point([10., 10.])?;
        session.execute(Command::UpdateSketch {
            id: sketch,
            sketch: layout,
        })?;
        let Effect::PartCreated { part } = session.execute(Command::CreatePart {
            name: "Base".into(),
        })?
        else {
            return Err("Peça ausente".into());
        };
        assert_eq!(session.document().instances()[0].part, part);
        session.execute(Command::RemoveSketch { id: sketch })?;
        assert!(session.document().sketch(sketch).is_none());
        assert!(session.undo());
        assert!(session.document().is_reference_sketch(sketch));
        Ok(())
    }

    #[test]
    fn folder_visibility_is_an_undoable_edit() -> Result<(), Box<dyn Error>> {
        use nator_document::Folder;
        let mut session = Session::new(Document::new("Pastas")?)?;
        session.execute(import()?)?;
        let part = session.document().parts()[0].id;
        session.execute(Command::SetFolderVisibility {
            folder: Folder::PartOrigin(part),
            visible: true,
        })?;
        assert!(
            session
                .document()
                .folder_visible(Folder::PartOrigin(part))?
        );
        assert!(session.undo());
        assert!(
            !session
                .document()
                .folder_visible(Folder::PartOrigin(part))?
        );
        assert!(session.redo());
        assert!(
            session
                .document()
                .folder_visible(Folder::PartOrigin(part))?
        );
        // Pasta de uma peça inexistente é recusada sem mudar o documento.
        let revision = session.revision();
        assert!(
            session
                .execute(Command::SetFolderVisibility {
                    folder: Folder::Bodies(PartId(999)),
                    visible: false,
                })
                .is_err()
        );
        assert_eq!(session.revision(), revision);
        Ok(())
    }

    #[test]
    fn failed_command_preserves_document_and_redo() -> Result<(), Box<dyn Error>> {
        let mut session = Session::new(Document::new("Ensaio")?)?;
        session.execute(import()?)?;
        assert!(session.undo());
        let revision = session.revision();
        assert!(
            session
                .execute(Command::RenamePart {
                    id: PartId(999),
                    name: "Inválida".into()
                })
                .is_err()
        );
        assert_eq!(session.revision(), revision);
        assert!(session.redo());
        Ok(())
    }
}
