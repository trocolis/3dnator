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
    /// Largura da árvore do projeto (expandida e recolhida), ajustada pela borda direita.
    /// Ausente vale o padrão.
    pub tree_width: Option<f32>,
    /// Tamanho da faixa de ferramentas.
    pub ribbon: RibbonSize,
}

/// Maior mostra o nome de cada seção sob os botões; Normal, só os botões com legenda;
/// Compacto, uma barra baixa só com ícones.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum RibbonSize {
    Large,
    #[default]
    Normal,
    Compact,
}

/// Largura padrão e limites da árvore do projeto, em pontos.
pub(super) const TREE_WIDTH: f32 = 270.;
pub(super) const TREE_WIDTH_RANGE: std::ops::RangeInclusive<f32> = 200.0..=640.0;

pub(super) fn path() -> Option<PathBuf> {
    nator_project_io::recovery_directory()
        .parent()
        .map(|p| p.join("preferences.json"))
}

impl AppPreferences {
    /// Largura da árvore, dentro dos limites mesmo com um valor estranho no arquivo.
    pub(super) fn tree_width(&self) -> f32 {
        self.tree_width
            .filter(|w| w.is_finite())
            .unwrap_or(TREE_WIDTH)
            .clamp(*TREE_WIDTH_RANGE.start(), *TREE_WIDTH_RANGE.end())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Arquivo de versões anteriores (sem largura nem tamanho da faixa) abre com os padrões;
    /// larguras fora do limite ou inválidas não chegam à interface.
    #[test]
    fn older_files_get_defaults_and_tree_width_stays_in_range()
    -> Result<(), Box<dyn std::error::Error>> {
        let old: AppPreferences = serde_json::from_str(r#"{"theme":"Dark"}"#)?;
        assert_eq!(old.ribbon, RibbonSize::Normal);
        assert_eq!(old.tree_width(), TREE_WIDTH);
        for (stored, expected) in [
            (50., 200.),
            (5000., 640.),
            (f32::NAN, TREE_WIDTH),
            (320., 320.),
        ] {
            let preferences = AppPreferences {
                tree_width: Some(stored),
                ..Default::default()
            };
            assert_eq!(preferences.tree_width(), expected);
        }
        let saved = serde_json::to_string(&AppPreferences {
            ribbon: RibbonSize::Compact,
            tree_width: Some(300.),
            ..Default::default()
        })?;
        let reopened: AppPreferences = serde_json::from_str(&saved)?;
        assert_eq!(reopened.ribbon, RibbonSize::Compact);
        assert_eq!(reopened.tree_width(), 300.);
        Ok(())
    }
}
