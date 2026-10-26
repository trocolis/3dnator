//! Hospedagem desktop; o estado do documento vive nos módulos internos.
mod project_editor;
pub(crate) use project_editor::capture::{
    render_project, render_project_at_size, render_ui_scenario,
};
pub use project_editor::run;
