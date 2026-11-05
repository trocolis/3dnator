//! Consulta compartilhada de hover e clique: sólidos e rascunhos visíveis.
use super::*;
use nator_document::assembly::Pick;
use nator_geometry::{Placement, dot, query::Ray};

pub struct Query {
    pub camera: Camera,
    pub bounds: nator_geometry::Bounds,
    pub rect: egui::Rect,
    pub position: egui::Pos2,
    pub reach: f64,
    pub section: Option<section::Section>,
}
pub struct Candidate {
    pub target: Target,
    pub overlay: highlight::Overlay,
}
struct ScreenCandidate {
    value: Candidate,
    point: Point3,
    priority: u8,
    distance: f32,
    depth: f64,
}
fn body_overlay(document: &Document, pick: &Pick, target: &Target) -> highlight::Overlay {
    match target {
        Target::Vertex { point } => highlight::Overlay {
            points: vec![*point],
            ..Default::default()
        },
        Target::Edge { points, .. } => highlight::Overlay {
            segments: points.windows(2).map(|s| ([s[0], s[1]], None)).collect(),
            ..Default::default()
        },
        Target::Face { .. } => {
            if let Some(body) = document.body(pick.body)
                && let Some(face) = body
                    .geometry
                    .triangle_faces()
                    .and_then(|f| f.get(pick.triangle))
                && let Ok(pose) = pick.placement.compose(body.placement)
            {
                return highlight::face(body, pose, *face);
            }
            highlight::Overlay {
                triangles: vec![pick.vertices],
                ..Default::default()
            }
        }
    }
}
impl Query {
    fn ray(&self, at: egui::Pos2) -> Ray {
        self.camera.world_ray(
            [
                f64::from((at.x - self.rect.center().x) / self.rect.width() * 2.),
                f64::from((self.rect.center().y - at.y) / self.rect.height() * 2.),
            ],
            self.bounds,
            f64::from(self.rect.aspect_ratio()),
        )
    }
    fn project(&self, p: Point3) -> Option<egui::Pos2> {
        sketch_scene::project(&self.camera, self.bounds, self.rect, p)
    }
    fn candidate(&self, target: Target, overlay: highlight::Overlay) -> Option<ScreenCandidate> {
        let (point, priority, distance) = match &target {
            Target::Vertex { point } => (*point, 0, self.project(*point)?.distance(self.position)),
            Target::Edge { points, .. } => {
                let (point, distance) = points
                    .windows(2)
                    .filter_map(|s| {
                        let a = self.project(s[0])?;
                        let b = self.project(s[1])?;
                        let delta = b - a;
                        let t = if delta.length_sq() > 1e-12 {
                            ((self.position - a).dot(delta) / delta.length_sq()).clamp(0., 1.)
                        } else {
                            0.
                        };
                        Some((
                            std::array::from_fn(|i| s[0][i] + f64::from(t) * (s[1][i] - s[0][i])),
                            (a + t * delta).distance(self.position),
                        ))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))?;
                (point, 1, distance)
            }
            Target::Face { point, .. } => (*point, 2, 0.),
        };
        if distance > 8.
            || self
                .section
                .filter(|s| s.visible)
                .is_some_and(|s| !s.keeps(point))
        {
            return None;
        }
        let ray = self.ray(self.project(point)?);
        Some(ScreenCandidate {
            value: Candidate { target, overlay },
            point,
            priority,
            distance,
            depth: dot(sub(point, ray.origin), ray.direction),
        })
    }
    fn sketch(
        &self,
        sketch: &nator_sketch::Sketch,
        pose: Placement,
        result: &mut Vec<ScreenCandidate>,
    ) {
        for point in sketch.visible_points() {
            let point = pose.apply(sketch.plane.world(point.position));
            let target = Target::Vertex { point };
            let overlay = highlight::Overlay {
                points: vec![point],
                ..Default::default()
            };
            if let Some(candidate) = self.candidate(target, overlay) {
                result.push(candidate);
            }
        }
        for entity in &sketch.entities {
            if entity.infinite {
                continue;
            }
            if let Ok(target) = nator_commands::measure::sketch_entity(sketch, entity.id, pose) {
                let overlay = match &target {
                    Target::Edge { points, .. } => highlight::Overlay {
                        segments: points.windows(2).map(|s| ([s[0], s[1]], None)).collect(),
                        ..Default::default()
                    },
                    _ => continue,
                };
                if let Some(candidate) = self.candidate(target, overlay) {
                    result.push(candidate);
                }
            }
        }
    }
    pub fn run(&self, document: &Document, view: &ViewState) -> Result<Option<Candidate>, String> {
        let mut candidates = Vec::new();
        let ray = self.ray(self.position);
        let picks = document
            .pick_all_clipped_with(ray, |o| view.pickable(o), section::clip(self.section))
            .map_err(|e| e.to_string())?;
        if let Some(pick) = section::first_visible(picks, self.section) {
            let target = nator_commands::measure::target_at(document, &pick, self.reach)
                .map_err(|e| e.to_string())?;
            let overlay = body_overlay(document, &pick, &target);
            if let Some(candidate) = self.candidate(target, overlay) {
                candidates.push(candidate);
            }
        }
        for occurrence in document
            .occurrences()
            .map_err(|e| e.to_string())?
            .iter()
            .filter(|o| view.pickable(o))
        {
            if let Some(part) = document
                .instance(occurrence.instance)
                .and_then(|i| document.part(i.part))
            {
                for record in part.sketches.iter().filter(|s| part.shows_sketch(s)) {
                    self.sketch(&record.sketch, occurrence.placement, &mut candidates);
                }
            }
        }
        for record in document
            .reference_sketches()
            .iter()
            .filter(|s| document.shows_reference_sketch(s))
        {
            self.sketch(&record.sketch, Placement::default(), &mut candidates);
        }
        candidates.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then(a.distance.total_cmp(&b.distance))
                .then(a.depth.total_cmp(&b.depth))
        });
        for candidate in candidates {
            let Some(at) = self.project(candidate.point) else {
                continue;
            };
            let ray = self.ray(at);
            let query = document
                .pick_clipped_with(ray, |o| view.pickable(o), section::clip(self.section))
                .map_err(|e| e.to_string())?;
            let blocked = query.cap_distance.unwrap_or(f64::INFINITY).min(
                query
                    .picks
                    .first()
                    .map(|p| dot(sub(p.point, ray.origin), ray.direction))
                    .unwrap_or(f64::INFINITY),
            );
            let tolerance = nator_geometry::norm(self.bounds.size()).max(1.) * 1e-6;
            if blocked >= candidate.depth - tolerance {
                return Ok(Some(candidate.value));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_sketches_and_material_in_front_are_not_candidates()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = Document::new("Oclusão")?;
        let mut sketch = nator_sketch::Sketch::new(
            "Linha",
            nator_sketch::Plane {
                origin: [0., 0., 2.],
                ..Default::default()
            },
        );
        sketch.add_line([10., 5.], [30., 5.], false)?;
        let (part, id) = doc.add_sketch(None, sketch)?;
        let bounds = nator_geometry::Bounds {
            min: [0., 0., 0.],
            max: [60., 30., 30.],
        };
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.));
        let mut camera = Camera::default();
        camera.set_direction([0., 0., 1.]);
        let position =
            sketch_scene::project(&camera, bounds, rect, [20., 5., 2.]).ok_or("Ponto")?;
        let query = Query {
            camera,
            bounds,
            rect,
            position,
            reach: 1.,
            section: None,
        };
        let view = ViewState::default();
        assert!(matches!(
            query.run(&doc, &view)?.map(|c| c.target),
            Some(Target::Edge { .. })
        ));
        doc.sketch_mut(id)?.visible = false;
        assert!(query.run(&doc, &view)?.is_none());
        doc.sketch_mut(id)?.visible = true;
        doc.add_mesh(
            Some(part),
            "Material".into(),
            Arc::new(nator_document::MeshAsset::new(
                nator_geometry::reference_bracket()?,
            )),
        )?;
        let result = query.run(&doc, &view)?.ok_or("Corpo sem candidato")?;
        assert!(
            result.target.anchor()[2] >= 6. - 1e-7,
            "Linha oculta foi medida"
        );
        Ok(())
    }
}
