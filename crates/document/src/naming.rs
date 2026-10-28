//! Sequências de criação por tipo, independentes dos nomes editáveis e dos IDs
//! globais. Renomear/excluir não libera números; desfazer restaura o documento.
use super::*;
use std::collections::BTreeMap;
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ElementKind {
    Part,
    Body,
    Sketch,
    WorkPlane,
    Image,
}
pub type NameCounters = BTreeMap<ElementKind, u64>;

impl Document {
    pub fn name_counters(&self) -> &NameCounters {
        &self.name_counters
    }
    pub fn next_name_number(&self, kind: ElementKind) -> u64 {
        self.name_counters
            .get(&kind)
            .copied()
            .unwrap_or(0)
            .saturating_add(1)
    }
    pub(crate) fn record_creation(&mut self, kind: ElementKind) -> Result<u64, DocumentError> {
        let value = self.name_counters.entry(kind).or_default();
        if *value >= 1_000_000_000_000 {
            return Err(DocumentError("Numeração de elementos esgotada.".into()));
        }
        *value = value
            .checked_add(1)
            .ok_or_else(|| DocumentError("Numeração de elementos esgotada.".into()))?;
        Ok(*value)
    }
    /// Projetos anteriores não guardavam sequências. A migração usa a quantidade
    /// existente e os sufixos padrão ainda disponíveis, preservando nomes próprios.
    pub fn restore_name_counters(&mut self, counters: NameCounters) -> Result<(), DocumentError> {
        let mut result = counters;
        let mut seed = |kind, prefix: &str, names: Vec<&str>| {
            let max = names
                .iter()
                .filter_map(|n| n.strip_prefix(prefix)?.trim().parse::<u64>().ok())
                .max()
                .unwrap_or(0);
            let value = result.entry(kind).or_default();
            *value = (*value).max(names.len() as u64).max(max);
        };
        seed(
            ElementKind::Part,
            "Peça",
            self.parts.iter().map(|p| p.name.as_str()).collect(),
        );
        seed(
            ElementKind::Body,
            "Corpo",
            self.parts
                .iter()
                .flat_map(|p| &p.bodies)
                .map(|b| b.name.as_str())
                .collect(),
        );
        seed(
            ElementKind::Sketch,
            "Rascunho",
            self.parts
                .iter()
                .flat_map(|p| &p.sketches)
                .chain(&self.reference_sketches)
                .map(|s| s.sketch.name.as_str())
                .collect(),
        );
        seed(
            ElementKind::WorkPlane,
            "Plano",
            self.parts
                .iter()
                .flat_map(|p| &p.work_planes)
                .map(|w| w.name.as_str())
                .collect(),
        );
        seed(
            ElementKind::Image,
            "Imagem",
            self.images.iter().map(|i| i.name.as_str()).collect(),
        );
        if result.values().any(|v| *v > 1_000_000_000_000) {
            return Err(DocumentError("Sequência de nomes fora do limite.".into()));
        }
        self.name_counters = result;
        Ok(())
    }
}
