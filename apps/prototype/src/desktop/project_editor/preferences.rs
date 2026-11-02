//! Preferências do aplicativo, pessoais e fora dos projetos: tema, idioma e atalhos.
//! Ficam num JSON na pasta de estado, ao lado da recuperação; arquivo ausente ou
//! ilegível vale os padrões, porque nada aqui guarda trabalho.
use super::theme;
use crate::i18n::Language;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(super) struct AppPreferences {
    pub theme: theme::Choice,
    /// `None` segue o idioma do sistema.
    pub language: Option<Language>,
    /// Atalhos trocados pelo usuário: nome estável da ação → tecla. Ausentes valem o padrão.
    pub shortcuts: BTreeMap<String, String>,
}

pub(super) fn path() -> Option<PathBuf> {
    nator_project_io::recovery_directory()
        .parent()
        .map(|p| p.join("preferences.json"))
}

impl AppPreferences {
    pub(super) fn load(path: &std::path::Path) -> Self {
        nator_project_io::read_preference(path)
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }
    pub(super) fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        nator_project_io::write_preference(path, &bytes).map_err(|e| e.to_string())
    }
}
