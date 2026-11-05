//! Estado de restrição no canvas: marcação de vértices e arestas, alerta de pontas sem
//! ligação e o menu Restringir. As regras ficam em `nator_sketch::{constraint_state, restrict}`.
use super::*;
use nator_sketch::constraint_state::Level;
use nator_sketch::repair::{self, Fix};
use nator_sketch::restrict::{self, Restriction, Selection};

const VERTEX_RADIUS: f32 = 3.5;
const RING_RADIUS: f32 = 6.5;
const SELECTED: egui::Color32 = egui::Color32::from_rgb(183, 106, 17);
/// As linhas do menu ocupam toda a largura disponível; sem limite, ele tomaria a tela.
const MENU_WIDTH: f32 = 240.;

fn in_conflict(work: &Workspace) -> bool {
    work.status == Status::Conflict
}

/// Aresta verde só quando determinada: o detalhe do que ainda se move fica nos vértices.
pub(super) fn entity_color(work: &Workspace, entity: u64) -> egui::Color32 {
    let state = &work.references.state;
    if in_conflict(work) && state.entity_in_conflict(entity) {
        theme::danger()
    } else if state.entity_restricted(entity) {
        theme::green()
    } else {
        theme::accent()
    }
}

/// Quanto mais cheio o vértice, mais preso ele está: vazado, cheio, cheio com anel. O quadrado
/// marca o que foi fixado no espaço. A forma dispensa depender só da cor.
pub(super) fn paint_vertices(
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    sketch: &Sketch,
    work: &Workspace,
) {
    let state = &work.references.state;
    let conflict = in_conflict(work);
    // A origem é selecionável mesmo com a do projeto oculta na árvore.
    if let Some(at) = frame.screen(restrict::ORIGIN) {
        painter.circle_filled(at, 2.5, theme::ink());
        painter.circle_stroke(at, 5.5, egui::Stroke::new(1., theme::ink()));
    }
    // Encontros são vértices virtuais enquanto as curvas estão inteiras: ponto neutro, sem estado.
    for vertex in &work.references.points.vertices {
        let real = sketch
            .points
            .iter()
            .any(|p| nator_sketch::distance(p.position, *vertex) < 1e-6);
        if !real && let Some(at) = frame.screen(*vertex) {
            painter.circle_filled(at, 2., theme::muted());
        }
    }
    let selected = sketch
        .entities
        .iter()
        .filter(|e| work.selected.contains(&e.id))
        .flat_map(|e| e.point_ids())
        .collect::<Vec<_>>();
    for point in sketch.visible_points() {
        let Some(at) = frame.screen(point.position) else {
            continue;
        };
        let level = state.level(point.id);
        let wrong = conflict && state.point_in_conflict(point.id);
        let color = if wrong {
            theme::danger()
        } else if selected.contains(&point.id) {
            SELECTED
        } else if point.face {
            theme::face_line()
        } else if point.projected {
            theme::projected()
        } else if level == Level::Restricted {
            theme::green()
        } else {
            theme::accent()
        };
        if wrong {
            painter.circle_filled(at, VERTEX_RADIUS, color);
        } else if state.is_fixed(point.id) {
            painter.rect_filled(
                egui::Rect::from_center_size(at, egui::Vec2::splat(VERTEX_RADIUS * 2.4)),
                1.,
                color,
            );
        } else {
            match level {
                Level::Free => {
                    painter.circle_filled(at, VERTEX_RADIUS, theme::canvas());
                    painter.circle_stroke(at, VERTEX_RADIUS, egui::Stroke::new(1.4, color));
                }
                Level::Partial => {
                    painter.circle_filled(at, VERTEX_RADIUS, color);
                }
                Level::Restricted => {
                    painter.circle_filled(at, VERTEX_RADIUS - 0.5, color);
                    painter.circle_stroke(at, RING_RADIUS, egui::Stroke::new(1.3, color));
                }
            }
        }
    }
    for vertex in work.selected_vertices.iter().chain(&work.operation.corners) {
        if let Some(at) = frame.screen(*vertex) {
            painter.circle_stroke(at, RING_RADIUS + 2.5, egui::Stroke::new(1.6, SELECTED));
        }
    }
}

/// Apoios que o clique pode escolher, incluindo a origem do rascunho.
pub(super) fn supports(sketch: &Sketch, work: &Workspace) -> Vec<Point2> {
    sketch
        .visible_points()
        .map(|p| p.position)
        .chain(work.references.points.vertices.iter().copied())
        .chain(work.references.indicators.iter().map(|m| m.point))
        .chain([restrict::ORIGIN])
        .collect()
}

pub(super) fn toggle_vertex(work: &mut Workspace, position: Point2, additive: bool) {
    let found = work
        .selected_vertices
        .iter()
        .position(|p| nator_sketch::distance(*p, position) < 1e-6);
    if !additive {
        work.selected.clear();
        work.selected_vertices = vec![position];
    } else if let Some(index) = found {
        work.selected_vertices.remove(index);
    } else {
        work.selected_vertices.push(position);
    }
}

fn selection(work: &Workspace) -> Selection<'_> {
    Selection {
        vertices: &work.selected_vertices,
        entities: &work.selected,
    }
}

#[derive(Default)]
pub(super) struct AlertUpdate {
    pub changed: bool,
    pub blocked: bool,
    pub warning: Option<String>,
}

/// Único ícone do canvas: pontas no mesmo lugar sem ligação não se distinguem das ligadas.
pub(super) fn alerts(
    ui: &egui::Ui,
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    sketch: &mut Sketch,
    work: &Workspace,
    enabled: bool,
) -> AlertUpdate {
    let mut update = AlertUpdate::default();
    for fix in &work.references.unlinked {
        let Fix::LinkEnds { a, b, .. } = *fix else {
            continue;
        };
        let Some(at) = sketch.point(a).ok().and_then(|p| frame.screen(p)) else {
            continue;
        };
        let rect = egui::Rect::from_min_size(at + egui::vec2(9., -24.), egui::Vec2::splat(18.));
        let response = ui
            .interact(
                rect,
                egui::Id::new("sketch_unlinked_ends").with((a, b)),
                if enabled {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                },
            )
            .on_hover_text(text(T::UnlinkedEnds));
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text(T::UnlinkedEnds))
        });
        if response.hovered() {
            painter.rect_filled(rect.expand(1.), 4., theme::blue_fill());
        }
        icons::paint(painter, rect, icons::Icon::Warning, true);
        update.blocked |= response.hovered();
        if enabled && response.clicked() {
            match repair::apply(sketch, std::slice::from_ref(fix)) {
                Ok(next) => {
                    *sketch = next;
                    update.changed = true;
                }
                Err(error) => update.warning = Some(error.to_string()),
            }
        }
    }
    update
}

/// F fixa a seleção; se tudo já estiver fixado, libera.
pub(super) fn fix_key(ui: &egui::Ui, sketch: &mut Sketch, work: &Workspace, enabled: bool) -> bool {
    if !enabled
        || !work.is_selecting()
        || (work.selected.is_empty() && work.selected_vertices.is_empty())
        || ui.ctx().egui_wants_keyboard_input()
        || !ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F))
    {
        return false;
    }
    ui.input_mut(|i| {
        i.events
            .retain(|e| !matches!(e, egui::Event::Text(s) if s.eq_ignore_ascii_case("f")))
    });
    let options = restrict::available(sketch, selection(work)).unwrap_or_default();
    let choice = if options.contains(&Restriction::Fix) {
        Restriction::Fix
    } else if options.contains(&Restriction::Release) {
        Restriction::Release
    } else {
        return false;
    };
    match restrict::apply(sketch, selection(work), choice) {
        Ok(next) => {
            *sketch = next;
            true
        }
        Err(_) => false,
    }
}

pub(in crate::desktop::project_editor) enum Choice {
    Apply(Restriction),
    Remove(u64),
    Dimension,
    Stroke,
}

/// Ferramentas sempre à vista na seção Restringir, em duas fileiras de três.
pub(in crate::desktop::project_editor) const GALLERY: [Restriction; 16] = [
    Restriction::HorizontalVertical,
    Restriction::Coincident,
    Restriction::Tangent,
    Restriction::Equal,
    Restriction::Parallel,
    Restriction::Perpendicular,
    Restriction::Fix,
    Restriction::Release,
    Restriction::Midpoint,
    Restriction::Concentric,
    Restriction::Collinear,
    Restriction::Symmetry,
    Restriction::Polygon,
    Restriction::Link,
    Restriction::Detach,
    Restriction::Dissolve,
];

pub(in crate::desktop::project_editor) fn presentation(
    restriction: Restriction,
) -> (icons::Icon, T) {
    use icons::Icon;
    match restriction {
        Restriction::HorizontalVertical => {
            (Icon::ConstraintHorizontalVertical, T::HorizontalVertical)
        }
        Restriction::Parallel => (Icon::ConstraintParallel, T::Parallel),
        Restriction::Perpendicular => (Icon::ConstraintPerpendicular, T::Perpendicular),
        Restriction::Equal => (Icon::ConstraintEqual, T::EqualRestriction),
        Restriction::Tangent => (Icon::ConstraintTangent, T::Tangent),
        Restriction::Concentric => (Icon::ConstraintConcentric, T::Concentric),
        Restriction::Coincident => (Icon::ConstraintCoincident, T::Coincident),
        Restriction::Midpoint => (Icon::SnapMidpoint, T::Midpoint),
        Restriction::Collinear => (Icon::ConstraintCollinear, T::Collinear),
        Restriction::Symmetry => (Icon::ConstraintSymmetry, T::Symmetry),
        Restriction::Polygon => (Icon::ConstraintPolygon, T::RegularPolygon),
        Restriction::Link => (Icon::ConstraintCoincident, T::LinkVertex),
        Restriction::Detach => (Icon::Trim, T::DetachVertex),
        Restriction::Dissolve => (Icon::Delete, T::RemoveVertex),
        Restriction::Fix => (Icon::ConstraintFix, T::FixElement),
        Restriction::Release => (Icon::LockOpen, T::ReleaseElement),
    }
}

/// Aplica a restrição à espera assim que a seleção a completa. Ela continua ativa para a
/// próxima, até Esc ou outra ferramenta.
pub(super) fn armed_step(sketch: &mut Sketch, work: &mut Workspace) -> bool {
    let Some(armed) = work.armed else {
        return false;
    };
    let Ok(resolved) = restrict::resolve(sketch, selection(work), armed) else {
        return false;
    };
    let ready = restrict::available(sketch, selection(work))
        .is_ok_and(|options| options.contains(&resolved));
    if !ready {
        return false;
    }
    let Ok(next) = restrict::apply(sketch, selection(work), resolved) else {
        return false;
    };
    *sketch = next;
    work.selected.clear();
    work.selected_vertices.clear();
    true
}

/// O que já afeta a seleção, com × para remover.
fn applied_list(
    ui: &mut egui::Ui,
    sketch: &Sketch,
    work: &Workspace,
    enabled: bool,
) -> Option<u64> {
    let applied = restrict::related(sketch, selection(work));
    if applied.is_empty() {
        return None;
    }
    let mut removed = None;
    ui.separator();
    ui.label(text(T::Constraints));
    for id in applied {
        let Some(constraint) = sketch.constraints.iter().find(|c| c.id == id) else {
            continue;
        };
        ui.horizontal(|ui| {
            ui.label(constraint_label(&constraint.kind, work.settings));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(enabled, egui::Button::new("×").small())
                    .clicked()
                {
                    removed = Some(id);
                }
            });
        });
    }
    removed
}

/// Lista completa da seção Restringir, na ordem da faixa. Todas ficam disponíveis: escolher
/// uma ferramenta que a seleção ainda não completa deixa-a à espera.
fn tool_menu(
    ui: &mut egui::Ui,
    sketch: &Sketch,
    work: &Workspace,
    enabled: bool,
) -> Option<Choice> {
    use icons::Icon;
    let tools = [
        (Icon::Dimension, T::SketchDimension, Choice::Dimension),
        (
            Icon::ConstraintHorizontalVertical,
            T::HorizontalVertical,
            Choice::Apply(Restriction::HorizontalVertical),
        ),
        (
            Icon::ConstraintCoincident,
            T::Coincident,
            Choice::Apply(Restriction::Coincident),
        ),
        (
            Icon::ConstraintTangent,
            T::Tangent,
            Choice::Apply(Restriction::Tangent),
        ),
        (
            Icon::ConstraintEqual,
            T::EqualRestriction,
            Choice::Apply(Restriction::Equal),
        ),
        (
            Icon::ConstraintParallel,
            T::Parallel,
            Choice::Apply(Restriction::Parallel),
        ),
        (
            Icon::ConstraintPerpendicular,
            T::Perpendicular,
            Choice::Apply(Restriction::Perpendicular),
        ),
        (Icon::LineConstruction, T::NormalReference, Choice::Stroke),
        (
            Icon::ConstraintFix,
            T::FixUnfix,
            Choice::Apply(Restriction::Fix),
        ),
        (
            Icon::SnapMidpoint,
            T::Midpoint,
            Choice::Apply(Restriction::Midpoint),
        ),
        (
            Icon::ConstraintConcentric,
            T::Concentric,
            Choice::Apply(Restriction::Concentric),
        ),
        (
            Icon::ConstraintCollinear,
            T::Collinear,
            Choice::Apply(Restriction::Collinear),
        ),
        (
            Icon::ConstraintSymmetry,
            T::Symmetry,
            Choice::Apply(Restriction::Symmetry),
        ),
        (
            Icon::ConstraintPolygon,
            T::RegularPolygon,
            Choice::Apply(Restriction::Polygon),
        ),
    ];
    let mut chosen = None;
    ui.set_width(MENU_WIDTH);
    for (icon, label, choice) in tools {
        let clicked = ui
            .add_enabled_ui(enabled, |ui| {
                icons::entry(ui, icon, text(label), false).clicked()
            })
            .inner;
        if clicked {
            chosen = Some(choice);
            ui.close();
        }
    }
    if let Some(id) = applied_list(ui, sketch, work, enabled) {
        chosen = Some(Choice::Remove(id));
    }
    chosen
}

/// Botão direito: só o que vale para o elemento clicado e o que já se aplica a ele.
fn menu(ui: &mut egui::Ui, sketch: &Sketch, work: &Workspace, enabled: bool) -> Option<Choice> {
    let mut chosen = None;
    ui.set_width(MENU_WIDTH);
    let chosen_selection = selection(work);
    if chosen_selection.vertices.is_empty() && chosen_selection.entities.is_empty() {
        ui.label(text(T::RestrictPickHint));
        return None;
    }
    match restrict::available(sketch, chosen_selection) {
        Ok(options) => {
            for option in options {
                let (icon, label) = presentation(option);
                ui.add_enabled_ui(enabled, |ui| {
                    if icons::entry(ui, icon, text(label), false).clicked() {
                        chosen = Some(Choice::Apply(option));
                        ui.close();
                    }
                });
            }
        }
        Err(error) => {
            ui.colored_label(theme::warning(), error.to_string());
        }
    }
    if let Some(id) = applied_list(ui, sketch, work, enabled) {
        chosen = Some(Choice::Remove(id));
    }
    chosen
}

fn apply_choice(
    sketch: &mut Sketch,
    work: &Workspace,
    choice: Choice,
) -> Result<(), nator_sketch::SketchError> {
    match choice {
        Choice::Apply(restriction) => {
            *sketch = restrict::apply(sketch, selection(work), restriction)?;
        }
        Choice::Remove(id) => restrict::remove(sketch, id),
        Choice::Dimension | Choice::Stroke => {}
    }
    Ok(())
}

/// Botão direito sobre um vértice ou aresta. O elemento clicado passa a ser a seleção, salvo
/// se já fizer parte dela.
pub(super) fn context_menu(
    response: &egui::Response,
    frame: &sketch_scene::Frame,
    sketch: &mut Sketch,
    work: &mut Workspace,
    supports: &[Point2],
    tolerance: f64,
    enabled: bool,
) -> bool {
    if enabled
        && work.is_selecting()
        && response.secondary_clicked()
        && let Some(cursor) = response
            .interact_pointer_pos()
            .and_then(|p| frame.cursor(p))
    {
        let vertex = junction_tool::vertex(sketch, supports, cursor, tolerance);
        let entity = nator_sketch::tools::pick_entity(sketch, cursor, tolerance)
            .ok()
            .flatten();
        if let Some(position) = vertex {
            let chosen = work
                .selected_vertices
                .iter()
                .any(|p| nator_sketch::distance(*p, position) < 1e-6);
            if !chosen {
                toggle_vertex(work, position, false);
            }
        } else if let Some(id) = entity
            && !work.selected.contains(&id)
        {
            work.selected = vec![id];
            work.selected_vertices.clear();
        }
        work.restriction_menu = vertex.is_some() || entity.is_some();
    }
    if !work.restriction_menu {
        return false;
    }
    let mut chosen = None;
    let current: &Sketch = sketch;
    let view: &Workspace = work;
    let open = response
        .context_menu(|ui| chosen = menu(ui, current, view, enabled))
        .is_some();
    let applied = chosen.is_some_and(|choice| apply_choice(sketch, work, choice).is_ok());
    if !open {
        work.restriction_menu = false;
    }
    applied
}

impl Editor {
    pub(in crate::desktop::project_editor) fn restriction_menu(
        &self,
        ui: &mut egui::Ui,
    ) -> Option<Choice> {
        let sketch = self.working_sketch()?;
        let work = self.current().sketch_edit.as_ref()?;
        tool_menu(ui, &sketch, work, !self.busy())
    }

    pub(in crate::desktop::project_editor) fn restriction_is_armed(
        &self,
        restriction: Restriction,
    ) -> bool {
        self.current()
            .sketch_edit
            .as_ref()
            .is_some_and(|work| work.armed == Some(restriction))
    }

    pub(in crate::desktop::project_editor) fn apply_restriction(
        &mut self,
        context: &egui::Context,
        choice: Choice,
    ) {
        match choice {
            Choice::Dimension => return self.select_sketch_tool(Tool::Dimension),
            Choice::Stroke => return self.toggle_sketch_stroke(context),
            Choice::Apply(_) | Choice::Remove(_) => {}
        }
        self.sanitize_sketch_selection();
        let Some(mut sketch) = self.working_sketch() else {
            return;
        };
        let Some(work) = self.current_mut().sketch_edit.as_mut() else {
            return;
        };
        work.constraint = None;
        // Uma ferramenta de desenho aberta dá lugar à seleção dos elementos.
        if !work.is_selecting() {
            work.select_tool(Tool::Select);
            work.tool_panel_open = false;
        }
        let choice = match choice {
            Choice::Apply(restriction) => {
                let resolved =
                    restrict::resolve(&sketch, selection(work), restriction).unwrap_or(restriction);
                let ready = restrict::available(&sketch, selection(work))
                    .is_ok_and(|options| options.contains(&resolved));
                if !ready {
                    work.armed = Some(restriction);
                    self.status.set(text(T::RestrictArmedHint));
                    return;
                }
                Choice::Apply(resolved)
            }
            other => other,
        };
        let result = apply_choice(&mut sketch, work, choice);
        match result {
            Ok(()) => self.solve_sketch(context, sketch),
            Err(error) => self.status.warning(error.to_string()),
        }
    }
}
