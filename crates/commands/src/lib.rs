//! Transações do documento e coordenação do histórico paramétrico, com desfazer/refazer.
mod assembly;
pub mod drawing;
pub mod export;
mod features;
pub mod measure;
use nator_document::{
    BodyId, Document, DocumentError, InstanceId, MeshAsset, PartId, SketchId, validate_name,
};
use nator_geometry::Placement;
use std::{collections::VecDeque, sync::Arc};

#[derive(Debug, Clone)]
pub enum Command {
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
    RemoveFeature {
        part: PartId,
        output: BodyId,
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
    CloneInstance {
        id: InstanceId,
        name: String,
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
}

#[derive(Debug, Clone, Copy)]
pub enum Effect {
    Changed,
    Imported { part: PartId, body: BodyId },
    SketchCreated { part: PartId, sketch: SketchId },
    SheetSaved { id: nator_document::SheetId },
}

impl Command {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ImportStep { .. } => "Importar montagem STEP",
            Self::SetSheet { .. } => "Salvar folha técnica associada",
            Self::ImportSolid { .. } => "Importar sólido CAD",
            Self::AddFeature { .. } => "Adicionar operação paramétrica",
            Self::EditFeature { .. } => "Editar operação paramétrica",
            Self::RemoveFeature { .. } => "Remover operação paramétrica",
            Self::AddInstance { .. } => "Adicionar instância",
            Self::PatternInstance { .. } => "Padrão de instâncias",
            Self::MirrorInstance { .. } => "Espelhar instância como nova peça",
            Self::CloneInstance { .. } => "Clonar peça independente",
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
            Self::MoveBody { .. } => "Posicionar corpo",
            Self::MoveInstance { .. } => "Posicionar instância",
            Self::RemoveBody { .. } => "Excluir corpo",
        }
    }
    fn apply(self, doc: &mut Document) -> Result<Effect, DocumentError> {
        match self {
            Self::ImportStep {
                path,
                destination,
                name,
            } => return assembly::import_step(doc, &path, destination, name),
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
                    ));
                }
                let (part, body) = doc.add_mesh(destination, name, geometry)?;
                let imported = doc.body_mut(body)?;
                imported.brep = Some(brep);
                // A geometria importada nunca é recalculada: nomes posicionais são estáveis.
                imported.face_names = Arc::new(nator_kernel::indexed_face_names(
                    &nator_document::features::body_owner(body),
                    faces,
                ));
                return Ok(Effect::Imported { part, body });
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
                features::rebuild(doc, part)?;
            }
            Self::RemoveFeature { part, output } => features::remove(doc, part, output)?,
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
                        ));
                    }
                    record.sketch = report.sketch;
                }
                features::rebuild(doc, part_id)?;
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
                    ));
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
                    ));
                }
                doc.update_sketch(id, sketch)?;
                let part = doc
                    .parts()
                    .iter()
                    .find(|p| p.sketches.iter().any(|s| s.id == id))
                    .map(|p| p.id)
                    .ok_or_else(|| DocumentError("Peça do rascunho ausente.".into()))?;
                features::rebuild(doc, part)?;
            }
            Self::ImportMesh {
                destination,
                name,
                geometry,
            } => {
                let (part, body) = doc.add_mesh(destination, name, geometry)?;
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
            Self::MoveBody { id, placement } => {
                doc.body_mut(id)?.placement = placement;
                let part = doc
                    .parts()
                    .iter()
                    .find(|p| p.bodies.iter().any(|b| b.id == id))
                    .map(|p| p.id)
                    .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
                features::rebuild(doc, part)?;
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
                    ));
                }
                doc.remove_body(id)?;
            }
        }
        Ok(Effect::Changed)
    }
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
    let operation = features::normalize(doc, part, operation)?;
    let built = features::construct(doc, part, output, &operation)?;
    // Refinamentos herdam a aparência da entrada principal. Uma edição explícita da cor
    // de um resultado permanece independente e não é sobrescrita pelo recálculo.
    let color = operation
        .inputs()
        .first()
        .and_then(|id| doc.body(*id))
        .and_then(|b| b.color);
    doc.add_body(
        part,
        nator_document::Body {
            id: output,
            name: name.clone(),
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
    doc.part_mut(part)?
        .features
        .push(nator_document::features::Feature {
            name,
            output,
            operation,
        });
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
    ) -> Result<Self, DocumentError> {
        let label = command.label().to_owned();
        let effect = command.apply(&mut document)?;
        document.validate()?;
        Ok(Self {
            document,
            revision,
            effect,
            label,
        })
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
    pub fn execute(&mut self, command: Command) -> Result<Effect, DocumentError> {
        let prepared = PreparedCommand::new(self.document.clone(), self.revision, command)?;
        self.commit(prepared)
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
