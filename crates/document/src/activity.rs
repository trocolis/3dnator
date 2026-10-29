//! Registro semântico da linha do tempo: referências aos elementos atuais, sem
//! guardar cliques, movimentos da câmera ou cópias de malhas. A lista permite
//! localizar/editar elementos; não é um replay de ações de montagem/rascunho.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditContext {
    Keep,
    Assembly,
    Part(PartId),
    Sketch {
        part: Option<PartId>,
        sketch: SketchId,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityTarget {
    Part(PartId),
    Sketch {
        id: SketchId,
        entities: Vec<u64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        constraints: Vec<u64>,
    },
    Feature {
        part: PartId,
        output: BodyId,
    },
    Body(BodyId),
    Instance(InstanceId),
    Group(GroupId),
    WorkPlane(WorkPlaneId),
    Image(ImageId),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityKind {
    Part,
    Sketch,
    Geometry,
    Dimension,
    Relation,
    Modify,
    Projection,
    Feature,
    Paint,
    Move,
    Copy,
    Group,
    WorkPlane,
    Image,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub id: u64,
    pub context: EditContext,
    pub kind: ActivityKind,
    pub target: ActivityTarget,
}
impl Document {
    pub fn activities(&self) -> &[Activity] {
        &self.activities
    }
    pub fn restore_activities(&mut self, activities: Vec<Activity>) -> Result<(), DocumentError> {
        if activities.len() > MAX_ELEMENTS || activities.iter().any(|a| a.id == 0)
            || activities.windows(2).any(|a| a[0].id >= a[1].id)
            || activities.iter().any(|a| matches!(&a.target, ActivityTarget::Sketch { entities, constraints, .. } if entities.len() > MAX_ELEMENTS || constraints.len() > MAX_ELEMENTS)) {
            return Err(DocumentError("Registro de atividades inválido.".into()));
        }
        // Alvos excluídos podem permanecer como registro histórico. A interface
        // só oferece edição quando a referência ainda existe no documento atual.
        self.activities = activities;
        Ok(())
    }
    pub fn record_activity(
        &mut self,
        context: EditContext,
        kind: ActivityKind,
        target: ActivityTarget,
    ) -> Result<(), DocumentError> {
        if matches!(&target, ActivityTarget::Sketch { entities, constraints, .. }
            if entities.len() > MAX_ELEMENTS || constraints.len() > MAX_ELEMENTS)
        {
            return Err(DocumentError("Atividade de rascunho muito extensa.".into()));
        }
        let id = self
            .activities
            .last()
            .map(|a| a.id)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| DocumentError("Sequência de atividades esgotada.".into()))?;
        if self.activities.len() >= MAX_ELEMENTS {
            self.activities.remove(0);
        }
        self.activities.push(Activity {
            id,
            context,
            kind,
            target,
        });
        Ok(())
    }
    pub fn remove_group_activity(&mut self, group: GroupId) {
        self.activities
            .retain(|a| a.target != ActivityTarget::Group(group));
    }
}
