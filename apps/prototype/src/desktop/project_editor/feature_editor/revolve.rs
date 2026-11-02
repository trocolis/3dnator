//! Guias escolhidas nas curvas do viewport, no referencial da ocorrência da
//! peça. A amostragem é só para apresentação/captura; a operação usa curvas exatas.
use super::*;
struct GuideHit {
    distance: f32,
    sketch: SketchId,
    entity: u64,
    points: Vec<egui::Pos2>,
}
impl Editor {
    pub(in crate::desktop::project_editor) fn revolve_axis_overlay(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
    ) -> bool {
        let Some(draft) = self.feature_edit.as_ref().filter(|d| {
            matches!(
                d.field,
                Some(Field::RevolveAxis | Field::SweepAxis | Field::SweepPath)
            )
        }) else {
            return false;
        };
        let Some(part) = self.current().session.document().part(draft.part) else {
            return false;
        };
        let Ok(placement) = sketch_scene::part_placement(self.current(), draft.part) else {
            return false;
        };
        let plane = draft.profile_plane(part);
        let path = draft.field == Some(Field::SweepPath);
        let selected = if draft.tool == Tool::Sweep {
            match &draft.sweep_route {
                SweepRoute::Axis { sketch, .. } | SweepRoute::Sketch { sketch, .. } => {
                    Some(*sketch)
                }
                _ => None,
            }
        } else {
            draft.revolve_axis.map(|(s, _)| s)
        };
        let profiles = draft.selected_profiles();
        let pointer = response.hover_pos();
        let mut hovered: Option<GuideHit> = None;
        for record in &part.sketches {
            if !part.shows_sketch(record)
                && selected != Some(record.id)
                && !(draft.output.is_some() && profiles.iter().any(|p| p.sketch == record.id))
            {
                continue;
            }
            if !path
                && plane.is_some_and(|p| {
                    nator_document::features::validate_coplanar(p, record.sketch.plane).is_err()
                })
            {
                continue;
            }
            let frame = sketch_scene::Frame {
                camera: self.viewport.camera.clone(),
                bounds: self.display_bounds(),
                rect: response.rect,
                placement,
                plane: record.sketch.plane,
            };
            let Some(cursor) = pointer.and_then(|p| frame.cursor(p)) else {
                continue;
            };
            for entity in &record.sketch.entities {
                let points = match entity.curve {
                    Curve::Line { .. } => {
                        let Ok((a, b)) = record.sketch.line(entity.id) else {
                            continue;
                        };
                        if entity.infinite {
                            if path {
                                continue;
                            }
                            let delta = nator_sketch::sub(b, a);
                            let length = nator_sketch::dot(delta, delta);
                            if length < 1e-18 {
                                continue;
                            }
                            let t = nator_sketch::dot(nator_sketch::sub(cursor, a), delta) / length;
                            let q = nator_sketch::add(a, nator_sketch::scale(delta, t));
                            vec![
                                nator_sketch::add(q, nator_sketch::scale(delta, -1000.)),
                                nator_sketch::add(q, nator_sketch::scale(delta, 1000.)),
                            ]
                        } else {
                            vec![a, b]
                        }
                    }
                    Curve::Arc { .. } if path => match profiles::segment(&record.sketch, entity) {
                        Ok(segment) => segment.sample(64),
                        Err(_) => continue,
                    },
                    _ => continue,
                };
                let Some(points) = points
                    .into_iter()
                    .map(|p| frame.screen(p))
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                let distance = points
                    .windows(2)
                    .map(|pair| {
                        let delta = pair[1] - pair[0];
                        let length = delta.length_sq();
                        let t = if length > 0. {
                            (pointer.unwrap_or(pair[0]) - pair[0]).dot(delta) / length
                        } else {
                            0.
                        };
                        (pair[0] + delta * t.clamp(0., 1.)).distance(pointer.unwrap_or(pair[0]))
                    })
                    .fold(f32::INFINITY, f32::min);
                if distance <= 10. && hovered.as_ref().is_none_or(|h| distance < h.distance) {
                    hovered = Some(GuideHit {
                        distance,
                        sketch: record.id,
                        entity: entity.id,
                        points,
                    });
                }
            }
        }
        if let Some(hit) = hovered {
            ui.painter()
                .with_clip_rect(response.rect)
                .add(egui::Shape::line(
                    hit.points,
                    egui::Stroke::new(3., theme::accent()),
                ));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if response.clicked()
                && !self.busy()
                && let Some(draft) = self.feature_edit.as_mut()
            {
                if path {
                    draft.choose_sweep_path(hit.sketch, hit.entity);
                } else {
                    draft.choose_revolve_axis(hit.sketch, hit.entity);
                }
            }
        }
        response.clicked()
    }
}
