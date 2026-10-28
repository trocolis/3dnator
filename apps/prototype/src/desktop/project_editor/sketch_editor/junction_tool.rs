//! Indicadores e ação contextual das junções. O cadeado representa a ação:
//! fechado fortalece uma ligação fraca; aberto libera uma ligação forte.
use super::*;
use nator_sketch::junctions::{self, Junction};

pub(super) fn vertex(
    sketch: &Sketch,
    virtual_vertices: &[Point2],
    cursor: Point2,
    tolerance: f64,
) -> Option<Point2> {
    sketch
        .visible_points()
        .map(|p| p.position)
        .chain(virtual_vertices.iter().copied())
        .filter(|p| nator_sketch::distance(*p, cursor) <= tolerance)
        .min_by(|a, b| {
            nator_sketch::distance(*a, cursor).total_cmp(&nator_sketch::distance(*b, cursor))
        })
}

pub(super) fn paint(frame: &sketch_scene::Frame, painter: &egui::Painter, junctions: &[Junction]) {
    for junction in junctions {
        if let Some(at) = frame.screen(junction.position) {
            painter.circle_filled(
                at,
                3.5,
                if junction.strong {
                    theme::JUNCTION_STRONG
                } else {
                    theme::CANVAS
                },
            );
            painter.circle_stroke(
                at,
                4.5,
                egui::Stroke::new(
                    1.,
                    if junction.strong {
                        theme::JUNCTION_STRONG
                    } else {
                        theme::MUTED
                    },
                ),
            );
        }
    }
}

#[derive(Default)]
pub(super) struct Update {
    pub changed: bool,
    pub blocked: bool,
    pub warning: Option<String>,
}

pub(super) fn button(
    ui: &egui::Ui,
    frame: &sketch_scene::Frame,
    painter: &egui::Painter,
    sketch: &mut Sketch,
    selected: Option<Point2>,
    enabled: bool,
) -> Update {
    let Some(position) = selected else {
        return Update::default();
    };
    let junction = match junctions::at(sketch, position) {
        Ok(Some(junction)) => junction,
        Ok(None) => return Update::default(),
        Err(error) => {
            return Update {
                warning: Some(error.to_string()),
                ..Default::default()
            };
        }
    };
    let Some(at) = frame.screen(position) else {
        return Update::default();
    };
    let rect = egui::Rect::from_min_size(at + egui::vec2(12., -32.), egui::Vec2::splat(28.));
    let rect = rect.translate(egui::vec2(
        (frame.rect.left() - rect.left()).max(0.) - (rect.right() - frame.rect.right()).max(0.),
        (frame.rect.top() - rect.top()).max(0.) - (rect.bottom() - frame.rect.bottom()).max(0.),
    ));
    let action = if junction.strong {
        T::JunctionWeaken
    } else {
        T::JunctionStrengthen
    };
    let response = ui
        .interact(
            rect,
            egui::Id::new("sketch_junction_lock"),
            if enabled {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        )
        .on_hover_text(text(action));
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text(action)));
    painter.rect_filled(
        rect,
        4.,
        if response.hovered() {
            theme::BLUE_FILL
        } else {
            theme::PANEL
        },
    );
    icons::paint(
        painter,
        rect.shrink(3.),
        if junction.strong {
            icons::Icon::LockOpen
        } else {
            icons::Icon::LockClosed
        },
        enabled,
    );
    painter.circle_stroke(at, 7., egui::Stroke::new(1.5, theme::JUNCTION_STRONG));
    let mut update = Update {
        blocked: response.hovered(),
        ..Default::default()
    };
    if enabled && response.clicked() {
        match junctions::set_strong(sketch, position, !junction.strong) {
            Ok(changed) => update.changed = changed,
            Err(error) => update.warning = Some(error.to_string()),
        }
    }
    update
}
