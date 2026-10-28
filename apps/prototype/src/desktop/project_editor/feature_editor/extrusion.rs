//! Manipulador de distância. Só altera o rascunho do painel; o prisma e a sugestão
//! geométrica continuam no trabalhador de prévia, com revisão e chave verificadas.
use super::*;
use nator_commands::extrusion::Guide;

#[derive(Default)]
pub(super) struct Interaction {
    guide: Option<Guide>,
    drag: Option<Drag>,
}
struct Drag {
    motion: distance_arrow::Drag,
    start: bool,
}
impl ExtrudeDraft {
    fn set_mouse_endpoint(&mut self, value: f64, start: bool, current_from: f64) {
        self.to_face = false;
        if let Some((from, _)) = self.custom.take() {
            self.offset = from;
            self.direction = Direction::OneSide;
        }
        if self.direction == Direction::TwoSides {
            let expression = format!("{} mm", value.abs());
            if start != self.flip {
                self.distance2 = expression;
            } else {
                self.distance = expression;
            }
        } else {
            // O manipulador mede a posição desde o plano, enquanto Distância mede
            // desde o início deslocado. Conservar essa diferença evita somar o offset duas vezes.
            let length = if self.direction == Direction::OneSide {
                value - current_from
            } else {
                value
            };
            self.flip = length < 0.;
            self.distance = format!("{} mm", length.abs());
        }
    }
}
impl Draft {
    #[cfg(all(test, feature = "occt"))]
    pub(in crate::desktop::project_editor) fn extrusion_state(&self) -> Option<(Guide, usize)> {
        Some((self.interaction.guide?, self.combine))
    }
    pub(in crate::desktop::project_editor) fn accepts_face_profile(&self) -> bool {
        self.tool == Tool::Extrude
    }
    pub(in crate::desktop::project_editor) fn receive_extrusion(
        &mut self,
        guide: Option<Guide>,
        choice: Option<Combine>,
    ) {
        if self.tool != Tool::Extrude {
            return;
        }
        self.interaction.guide = guide;
        if self.automatic {
            self.combine = match choice.map(|c| c.mode) {
                None => 0,
                Some(BooleanMode::Join) => 1,
                Some(BooleanMode::Cut) => 2,
                Some(BooleanMode::Intersect) => 3,
            };
            self.combine_target = choice.map(|c| c.target);
        }
    }
}

impl Editor {
    pub(in crate::desktop::project_editor) fn extrusion_gizmo(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
    ) -> bool {
        let Some(mut draft) = self.feature_edit.take() else {
            return false;
        };
        let consumed = self.draw_extrusion_gizmo(ui, response, &mut draft);
        self.feature_edit = Some(draft);
        consumed
    }
    fn draw_extrusion_gizmo(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        d: &mut Draft,
    ) -> bool {
        if d.tool != Tool::Extrude || d.field.is_some() || self.busy() {
            return false;
        }
        let Some(g) = d.interaction.guide else {
            return false;
        };
        let Some(part) = self.current().session.document().part(d.part) else {
            return false;
        };
        let Ok(placement) = sketch_scene::part_placement(self.current(), d.part) else {
            return false;
        };
        let (from, to) = if d.extrude.to_face {
            (g.from, g.to)
        } else {
            let (from, to) = d.extrude.limits();
            let length = |s: &str| {
                part.parameters
                    .evaluate(s, nator_document::parameters::Dimension::Length)
                    .ok()
            };
            (length(&from).unwrap_or(g.from), length(&to).unwrap_or(g.to))
        };
        let Ok(plane) = nator_sketch::Plane::from_normal(g.origin, g.normal) else {
            return false;
        };
        let frame = sketch_scene::Frame {
            camera: self.viewport.camera.clone(),
            bounds: self.display_bounds(),
            rect: response.rect,
            placement,
            plane,
        };
        let project = |distance: f64| {
            sketch_scene::project(
                &frame.camera,
                frame.bounds,
                frame.rect,
                placement.apply(std::array::from_fn(|i| {
                    g.origin[i] + g.normal[i] * distance
                })),
            )
        };
        let Some(origin) = project(0.) else {
            return false;
        };
        let vector = project(1.).map(|p| p - origin).unwrap_or_default();
        let (axis, scale) = if vector.length() > 0.5 {
            (vector.normalized(), vector.length())
        } else {
            // Vista de frente para o perfil: a normal não se projeta numa reta.
            // O arraste vertical conserva a escala do plano e permite trabalhar sem orbitar.
            (egui::vec2(0., -1.), frame.pixels_per_mm([0., 0.]) as f32)
        };
        let mut hovered = false;
        let pointer = response
            .interact_pointer_pos()
            .or_else(|| ui.input(|i| i.pointer.hover_pos()));
        let painter = ui.painter().with_clip_rect(response.rect);
        let mut active_handle = None;
        for (distance, start) in [(to, false), (from, true)] {
            if start && d.extrude.direction != Direction::TwoSides {
                continue;
            }
            let position = if vector.length() > 0.5 {
                project(distance).unwrap_or(origin)
            } else {
                origin + axis * (distance as f32 * scale)
            };
            let position = egui::pos2(
                position
                    .x
                    .clamp(response.rect.left() + 18., response.rect.right() - 18.),
                position
                    .y
                    .clamp(response.rect.top() + 18., response.rect.bottom() - 18.),
            );
            let over = pointer.is_some_and(|p| p.distance(position) < 16.);
            hovered |= over;
            let sign = if distance < 0. { -1. } else { 1. };
            let arrow = distance_arrow::Arrow {
                origin,
                tip: position,
                direction: axis * sign,
            };
            arrow.paint(&painter, over || d.interaction.drag.is_some());
            painter.text(
                position + egui::vec2(16., -12.),
                egui::Align2::LEFT_BOTTOM,
                format!("{} mm", nator_drawing::format_mm(distance)),
                egui::FontId::proportional(13.),
                theme::INK,
            );
            if arrow.started(ui, response) {
                active_handle = Some((distance, start));
            }
        }
        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if let Some((initial, start)) = active_handle {
            d.interaction.drag = ui.input(|i| i.pointer.press_origin()).map(|pointer| Drag {
                motion: distance_arrow::Drag {
                    pointer,
                    axis,
                    pixels_per_unit: scale,
                    initial,
                },
                start,
            });
        }
        let dragging = d.interaction.drag.is_some();
        if let Some(drag) = &d.interaction.drag {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            if let Some(pointer) = pointer {
                let value = drag.motion.value(pointer);
                if value.is_finite() && value.abs() <= 1e7 {
                    d.extrude.set_mouse_endpoint(value, drag.start, from);
                    ui.ctx().request_repaint();
                }
            }
        }
        if response.drag_stopped_by(egui::PointerButton::Primary)
            || !ui.input(|i| i.pointer.primary_down())
        {
            d.interaction.drag = None;
        }
        hovered || dragging
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mouse_distance_preserves_offsets_custom_starts_and_two_sided_limits()
    -> Result<(), Box<dyn std::error::Error>> {
        let parameters = nator_document::parameters::Parameters::default();
        let limits = |d: &ExtrudeDraft| {
            let (a, b) = d.limits();
            Ok::<_, nator_document::DocumentError>((
                parameters.evaluate(&a, nator_document::parameters::Dimension::Length)?,
                parameters.evaluate(&b, nator_document::parameters::Dimension::Length)?,
            ))
        };
        let mut d = ExtrudeDraft {
            offset: "5 mm".into(),
            ..Default::default()
        };
        d.set_mouse_endpoint(20., false, 5.);
        assert_eq!(limits(&d)?, (5., 20.));
        d.set_mouse_endpoint(3., false, 5.);
        assert_eq!(limits(&d)?, (5., 3.));
        d.custom = Some(("2 mm + 3 mm".into(), "16 mm".into()));
        d.set_mouse_endpoint(15., false, 5.);
        assert_eq!(limits(&d)?, (5., 15.));
        d.direction = Direction::TwoSides;
        d.flip = false;
        d.distance = "10 mm".into();
        d.set_mouse_endpoint(-4., true, -10.);
        assert_eq!(limits(&d)?, (-4., 10.));
        d.flip = true;
        d.set_mouse_endpoint(15., false, -10.);
        assert_eq!(limits(&d)?, (-10., 15.));
        d.direction = Direction::Symmetric;
        d.set_mouse_endpoint(-7., false, -10.);
        assert_eq!(limits(&d)?, (-7., 7.));
        let mut draft = Draft::new(1, PartId(1), Tool::Extrude);
        draft.field = Some(Field::Target);
        draft.apply_pick(BodyId(2), Kept::Body(BodyId(2)));
        assert!(!draft.automatic);
        assert_eq!(draft.combine_target, Some(BodyId(2)));
        draft.extrude.custom = Some(("5 mm".into(), "20 mm".into()));
        draft.field = None;
        draft.apply_pick(BodyId(3), Kept::Face(FaceRef::Named("b3:end".into())));
        assert_eq!(draft.extrude.offset, "5 mm");
        Ok(())
    }
}
