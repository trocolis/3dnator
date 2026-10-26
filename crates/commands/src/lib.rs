//! Transações de documento, separadas do futuro histórico de operações paramétricas.
mod features;
use nator_document::{
    BodyId, Document, DocumentError, InstanceId, MeshAsset, PartId, SketchId, validate_name,
};
use nator_geometry::Placement;
use std::{collections::VecDeque, sync::Arc};

#[derive(Debug, Clone)]
pub enum Command {
    SetSheet {
        id: Option<nator_document::SheetId>,
        target: Option<PartId>,
        specification: nator_drawing::Specification,
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
}

impl Command {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SetSheet { .. } => "Salvar folha técnica associada",
            Self::ImportSolid { .. } => "Importar sólido CAD",
            Self::AddFeature { .. } => "Adicionar operação paramétrica",
            Self::EditFeature { .. } => "Editar operação paramétrica",
            Self::RemoveFeature { .. } => "Remover operação paramétrica",
            Self::AddInstance { .. } => "Adicionar instância",
            Self::CloneInstance { .. } => "Clonar peça independente",
            Self::ReparentInstance { .. } => "Organizar subpeça",
            Self::RemoveInstance { .. } => "Remover instância",
            Self::CreateGroup { .. } => "Criar grupo organizacional",
            Self::MoveGroup { .. } => "Mover grupo",
            Self::RemoveGroup { .. } => "Desagrupar elementos",
            Self::SetParameter { .. } => "Alterar parâmetro da peça",
            Self::AddSketch { .. } => "Criar rascunho",
            Self::UpdateSketch { .. } => "Editar rascunho",
            Self::ImportMesh { .. } => "Importar corpo de malha",
            Self::RenamePart { .. } => "Renomear peça",
            Self::RenameBody { .. } => "Renomear corpo",
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
            Self::SetSheet {
                id,
                target,
                specification,
            } => {
                doc.set_sheet(id, target, specification)?;
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
                // O identificador vem antes da construção porque os nomes das faces levam
                // o corpo dono; em caso de falha, a cópia candidata é descartada inteira.
                let output = doc.allocate_body_id()?;
                let operation = features::normalize(doc, part, operation)?;
                let built = features::construct(doc, part, output, &operation)?;
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
                    },
                )?;
                for input in operation.inputs() {
                    doc.body_mut(input)?.visible = false;
                }
                doc.part_mut(part)?
                    .features
                    .push(nator_document::features::Feature {
                        name,
                        output,
                        operation,
                    });
                return Ok(Effect::Imported { part, body: output });
            }
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
                doc.add_instance(part, source.parent, source.placement)?;
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
                    .any(|f| f.output == id || f.operation.inputs().contains(&id))
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
