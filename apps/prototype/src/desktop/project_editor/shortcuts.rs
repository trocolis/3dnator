//! Atalhos de teclado configuráveis. Cada ação tem um nome estável (gravado nas
//! preferências), um contexto e a tecla padrão. Os contextos seguem a lógica do rascunho:
//! uma letra, sem modificador, vale só onde a ferramenta existe; os atalhos do sistema
//! valem em toda parte. A mesma letra pode se repetir em contextos diferentes.
use super::{T, egui, sketch_editor};
use nator_sketch::edit::Action as Edit;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scope {
    System,
    /// Fora do rascunho: construção e montagem.
    Workspace,
    Sketch,
    Model,
    Assembly,
}
impl Scope {
    pub(super) fn label(self) -> T {
        match self {
            Self::System => T::ShortcutsSystem,
            Self::Workspace => T::ShortcutsWorkspace,
            Self::Sketch => T::ShortcutsSketch,
            Self::Model => T::ShortcutsModel,
            Self::Assembly => T::ShortcutsAssembly,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Target {
    Save,
    Undo,
    Redo,
    Search,
    Tool(sketch_editor::Tool),
    Edit(Edit),
    Delete,
    Stroke,
    Extrude,
    Revolve,
    Fillet,
    Chamfer,
    Move,
}
impl Target {
    pub(super) fn label(self) -> T {
        match self {
            Self::Save => T::Save,
            Self::Undo => T::Undo,
            Self::Redo => T::Redo,
            Self::Search => T::SearchTools,
            Self::Tool(tool) => tool.label(),
            Self::Edit(action) => sketch_editor::edit_label(action),
            Self::Delete => T::DeleteFeature,
            Self::Stroke => T::ShortcutStroke,
            Self::Extrude => T::Extrude,
            Self::Revolve => T::Revolve,
            Self::Fillet => T::Fillet,
            Self::Chamfer => T::Chamfer,
            Self::Move => T::MoveMenu,
        }
    }
}

pub(super) type Combo = (egui::Modifiers, egui::Key);

pub(super) struct Binding {
    pub id: &'static str,
    pub scope: Scope,
    pub target: Target,
    pub default: Combo,
}

const fn bind(id: &'static str, scope: Scope, target: Target, key: egui::Key) -> Binding {
    Binding {
        id,
        scope,
        target,
        default: (egui::Modifiers::NONE, key),
    }
}

pub(super) fn bindings() -> Vec<Binding> {
    use Scope::*;
    use egui::Key as K;
    use sketch_editor::Tool;
    let ctrl = |id, target, key| Binding {
        id,
        scope: System,
        target,
        default: (egui::Modifiers::CTRL, key),
    };
    vec![
        ctrl("save", Target::Save, K::S),
        ctrl("undo", Target::Undo, K::Z),
        ctrl("redo", Target::Redo, K::Y),
        bind("search", Workspace, Target::Search, K::S),
        bind("sketch.line", Sketch, Target::Tool(Tool::Line), K::L),
        bind(
            "sketch.rectangle",
            Sketch,
            Target::Tool(Tool::Rectangle),
            K::R,
        ),
        bind("sketch.circle", Sketch, Target::Tool(Tool::Circle), K::C),
        bind("sketch.arc", Sketch, Target::Tool(Tool::Arc), K::A),
        bind("sketch.project", Sketch, Target::Tool(Tool::Project), K::P),
        bind("sketch.trim", Sketch, Target::Tool(Tool::Trim), K::G),
        bind(
            "sketch.dimension",
            Sketch,
            Target::Tool(Tool::Dimension),
            K::D,
        ),
        bind("sketch.offset", Sketch, Target::Edit(Edit::Offset), K::S),
        bind("sketch.fillet", Sketch, Target::Edit(Edit::Fillet), K::O),
        bind("sketch.chamfer", Sketch, Target::Edit(Edit::Chamfer), K::H),
        bind("sketch.mirror", Sketch, Target::Edit(Edit::Mirror), K::E),
        bind("sketch.delete", Sketch, Target::Delete, K::Delete),
        bind("sketch.stroke", Sketch, Target::Stroke, K::X),
        bind("model.extrude", Model, Target::Extrude, K::E),
        bind("model.revolve", Model, Target::Revolve, K::R),
        bind("model.fillet", Model, Target::Fillet, K::F),
        bind("model.chamfer", Model, Target::Chamfer, K::C),
        bind("assembly.move", Assembly, Target::Move, K::M),
    ]
}

/// Texto da combinação, como aparece na aba Atalhos e no arquivo de preferências.
pub(super) fn format((modifiers, key): Combo) -> String {
    let mut parts = Vec::new();
    if modifiers.command || modifiers.ctrl {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    parts.push(key.name());
    parts.join("+")
}

pub(super) fn parse(text: &str) -> Option<Combo> {
    let mut modifiers = egui::Modifiers::NONE;
    let mut key = None;
    for part in text.split('+') {
        match part {
            "Ctrl" => modifiers |= egui::Modifiers::CTRL,
            "Alt" => modifiers |= egui::Modifiers::ALT,
            "Shift" => modifiers |= egui::Modifiers::SHIFT,
            name => key = Some(egui::Key::from_name(name)?),
        }
    }
    Some((modifiers, key?))
}

/// Combinação em vigor: a escolhida pelo usuário ou a padrão.
pub(super) fn effective(chosen: &BTreeMap<String, String>, binding: &Binding) -> Combo {
    chosen
        .get(binding.id)
        .and_then(|text| parse(text))
        .unwrap_or(binding.default)
}

/// Os do sistema valem em toda parte; nos demais, só no mesmo contexto.
fn overlaps(a: Scope, b: Scope) -> bool {
    use Scope::*;
    a == b
        || a == System
        || b == System
        || matches!(
            (a, b),
            (Workspace, Model | Assembly) | (Model | Assembly, Workspace)
        )
}

/// Outra ação que já usa `combo` onde `id` vale, se houver.
pub(super) fn conflict(
    chosen: &BTreeMap<String, String>,
    id: &str,
    combo: Combo,
) -> Option<Binding> {
    let all = bindings();
    let scope = all.iter().find(|b| b.id == id)?.scope;
    all.into_iter()
        .find(|b| b.id != id && overlaps(b.scope, scope) && same(effective(chosen, b), combo))
}

fn same(a: Combo, b: Combo) -> bool {
    a.1 == b.1
        && (a.0.command || a.0.ctrl) == (b.0.command || b.0.ctrl)
        && a.0.alt == b.0.alt
        && a.0.shift == b.0.shift
}

/// Primeira ação do contexto cuja combinação foi pressionada; consome a tecla.
pub(super) fn pressed(
    context: &egui::Context,
    chosen: &BTreeMap<String, String>,
    scope: Scope,
) -> Option<Target> {
    bindings()
        .iter()
        .filter(|b| b.scope == scope)
        .find(|b| {
            let (modifiers, key) = effective(chosen, b);
            context.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(modifiers, key)))
        })
        .map(|b| b.target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combos_round_trip_and_conflicts_respect_the_context() {
        for binding in bindings() {
            assert_eq!(parse(&format(binding.default)), Some(binding.default));
        }
        let mut chosen = BTreeMap::new();
        // E na Construção (Extrusão) não colide com E no Rascunho (Espelhar).
        assert!(
            conflict(
                &chosen,
                "model.extrude",
                (egui::Modifiers::NONE, egui::Key::E)
            )
            .is_none()
        );
        // R na Construção já é Revolução.
        let found = conflict(
            &chosen,
            "model.extrude",
            (egui::Modifiers::NONE, egui::Key::R),
        );
        assert_eq!(found.map(|b| b.id), Some("model.revolve"));
        // Ctrl+S vale em toda parte.
        assert!(
            conflict(
                &chosen,
                "sketch.line",
                (egui::Modifiers::CTRL, egui::Key::S)
            )
            .is_some()
        );
        chosen.insert("model.revolve".into(), "Shift+R".into());
        assert!(
            conflict(
                &chosen,
                "model.extrude",
                (egui::Modifiers::NONE, egui::Key::R)
            )
            .is_none()
        );
    }
}
