//! Captura em tela compartilhada pelo realce e pelo clique do movimento ponto a ponto.
//! Centros são descobertos também no vazio; o cursor de captura aparece no apoio quando
//! entra no raio em pontos de tela, mantendo a mesma sensação em qualquer zoom.
use super::*;
use nator_commands::point_snap::{PointAnchor, PointKind, body_anchors};
use nator_document::assembly::Pick;
use nator_geometry::{Bounds, dot, norm, query::Ray, sub};
use std::collections::HashMap;

const CAPTURE_RADIUS: f32 = 14.;
type Anchors = HashMap<BodyId, Vec<PointAnchor>>;
pub(super) type ReferencePoints = HashMap<SketchId, Vec<nator_sketch::Point2>>;
type QueryResult = Result<(Anchors, ReferencePoints), String>;

#[derive(Default)]
pub(super) struct Cache {
    revision: Option<(u64, u64)>,
    entries: Anchors,
    references: ReferencePoints,
    running: Option<Receiver<QueryResult>>,
}
impl Cache {
    pub(super) fn references(&self, revision: (u64, u64)) -> Option<&ReferencePoints> {
        (self.revision == Some(revision)).then_some(&self.references)
    }
    fn prepare(
        &mut self,
        revision: (u64, u64),
        document: &Document,
        context: &egui::Context,
    ) -> Result<(), String> {
        if self.revision != Some(revision) {
            self.entries.clear();
            self.references.clear();
            self.running = None;
            self.revision = Some(revision);
            let bodies = document
                .parts()
                .iter()
                .flat_map(|p| p.shown_bodies().cloned())
                .collect::<Vec<_>>();
            let (sender, receiver) = mpsc::channel();
            let references = document
                .reference_sketches()
                .iter()
                .filter(|s| document.shows_reference_sketch(s))
                .cloned()
                .collect::<Vec<_>>();
            let context = context.clone();
            std::thread::Builder::new()
                .name("point-anchors".into())
                .spawn(move || {
                    let result = (|| {
                        let bodies = bodies
                            .iter()
                            .map(|body| {
                                body_anchors(body)
                                    .map(|anchors| (body.id, anchors))
                                    .map_err(|e| e.to_string())
                            })
                            .collect::<Result<Anchors, String>>()?;
                        let references = references
                            .iter()
                            .map(|record| {
                                nator_commands::point_snap::reference_points(&record.sketch)
                                    .map(|points| (record.id, points))
                                    .map_err(|e| e.to_string())
                            })
                            .collect::<Result<ReferencePoints, String>>()?;
                        Ok((bodies, references))
                    })();
                    let _ = sender.send(result);
                    context.request_repaint();
                })
                .map_err(|e| e.to_string())?;
            self.running = Some(receiver);
        }
        if let Some(receiver) = &self.running {
            match receiver.try_recv() {
                Ok(result) => {
                    self.running = None;
                    (self.entries, self.references) = result?;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.running = None;
                    return Err(text(T::PointQueryFailed).into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        Ok(())
    }
}

pub(super) struct Target {
    pub pick: Pick,
    pub captured: bool,
    pub kind: PointKind,
}

/// Raio que passa por uma posição de tela; usado também no centro virtual para
/// recusar apoios atrás de material, mesmo quando o cursor está ao lado do centro.
fn screen_ray(camera: &Camera, bounds: Bounds, rect: egui::Rect, position: egui::Pos2) -> Ray {
    camera.world_ray(
        [
            f64::from((position.x - rect.center().x) / rect.width() * 2.),
            f64::from((rect.center().y - position.y) / rect.height() * 2.),
        ],
        bounds,
        f64::from(rect.aspect_ratio()),
    )
}

/// O disco (círculo) ou setor (arco) revela o centro antes da captura, inclusive
/// no vazio. A interseção no plano acompanha a projeção em vistas oblíquas.
fn inside_circular_region(ray: Ray, circle: nator_kernel::CircleAnchor, pose: Placement) -> bool {
    let center = pose.apply(circle.center);
    let normal = pose.apply_vector(circle.normal);
    let denominator = dot(normal, ray.direction);
    if denominator.abs() < 1e-9 {
        return false;
    }
    let t = dot(normal, sub(center, ray.origin)) / denominator;
    let point = std::array::from_fn(|i| ray.origin[i] + t * ray.direction[i]);
    let delta = sub(point, center);
    let distance = norm(delta);
    if t < 0. || distance > circle.radius_mm {
        return false;
    }
    if circle.closed || distance < 1e-9 {
        return true;
    }
    // Para arcos, a descoberta usa somente o setor correspondente da curva.
    // O centro permanece no plano, mesmo que esteja fora da área preenchida.
    let x = pose.apply_vector(circle.x_direction);
    let y = nator_geometry::cross(normal, x);
    let angle = dot(delta, y).atan2(dot(delta, x));
    (angle - circle.start_angle_rad).rem_euclid(std::f64::consts::TAU) <= circle.sweep_rad + 1e-9
}

struct Candidate {
    target: Target,
    distance: f32,
    depth: f64,
    screen: egui::Pos2,
}

struct Query<'a> {
    document: &'a Document,
    camera: &'a Camera,
    bounds: Bounds,
    rect: egui::Rect,
    position: egui::Pos2,
    view: &'a ViewState,
    section: Option<section::Section>,
    source_part: Option<PartId>,
}

impl Query<'_> {
    fn target(self, entries: &Anchors) -> Option<Target> {
        let Self {
            document,
            camera,
            bounds,
            rect,
            position,
            view,
            section,
            source_part,
        } = self;
        let ray = screen_ray(camera, bounds, rect, position);
        let hovered = document
            .pick_all_with(ray, |o| view.pickable(o))
            .ok()
            .and_then(|hits| section::first_visible(hits, section));
        let mut candidates = Vec::new();
        for occurrence in document
            .occurrences()
            .ok()?
            .into_iter()
            .filter(|o| view.pickable(o))
        {
            let instance = document.instance(occurrence.instance)?;
            let part = document.part(instance.part)?;
            if source_part.is_some_and(|id| id != part.id) {
                continue;
            }
            for body in part.shown_bodies() {
                let Some(anchors) = entries.get(&body.id) else {
                    continue;
                };
                let Ok(pose) = occurrence.placement.compose(body.placement) else {
                    continue;
                };
                for anchor in anchors {
                    let point = pose.apply(anchor.point);
                    if section.is_some_and(|s| !s.keeps(point)) {
                        continue;
                    }
                    let Some(screen) = sketch_scene::project(camera, bounds, rect, point)
                        .filter(|p| rect.contains(*p))
                    else {
                        continue;
                    };
                    let distance = screen.distance(position);
                    let captured = distance <= CAPTURE_RADIUS;
                    let normal = anchor
                        .circle
                        .map(|c| pose.apply_vector(c.normal))
                        .unwrap_or(camera.direction());
                    let discovered = anchor
                        .circle
                        .is_some_and(|c| inside_circular_region(ray, c, pose))
                        || (anchor.circle.is_none()
                            && hovered.as_ref().is_some_and(|p| {
                                p.body == body.id
                                    && p.instance == instance.id
                                    && norm(sub(
                                        p.placement.apply([0.; 3]),
                                        occurrence.placement.apply([0.; 3]),
                                    )) < 1e-9
                            }));
                    if !captured && !discovered {
                        continue;
                    }
                    let target_ray = screen_ray(camera, bounds, rect, screen);
                    candidates.push(Candidate {
                        target: Target {
                            pick: Pick {
                                instance: instance.id,
                                part: part.id,
                                body: body.id,
                                // Captura transitória de ponto: não é referência de face.
                                triangle: 0,
                                point,
                                normal,
                                vertices: [point; 3],
                                placement: occurrence.placement,
                            },
                            captured,
                            kind: anchor.kind,
                        },
                        distance,
                        depth: dot(sub(point, target_ray.origin), target_ray.direction),
                        screen,
                    });
                }
            }
        }
        // Captura ganha da descoberta. Distâncias arredondadas ao pixel evitam
        // que ruído de projeção escolha o verso de centros sobrepostos: nesse caso
        // ganha a borda da frente, no plano da superfície, sem média da espessura.
        candidates.sort_by(|a, b| {
            b.target
                .captured
                .cmp(&a.target.captured)
                .then(a.distance.round().total_cmp(&b.distance.round()))
                .then(a.depth.total_cmp(&b.depth))
                .then(a.distance.total_cmp(&b.distance))
        });
        for candidate in candidates {
            let target_ray = screen_ray(camera, bounds, rect, candidate.screen);
            let hit = document
                .pick_all_with(target_ray, |o| view.pickable(o))
                .ok()
                .and_then(|hits| section::first_visible(hits, section));
            let tolerance = bounds.size().into_iter().fold(1_f64, f64::max) * 1e-6;
            if hit.is_some_and(|hit| {
                dot(sub(hit.point, target_ray.origin), target_ray.direction)
                    < candidate.depth - tolerance
            }) {
                continue;
            }
            return Some(candidate.target);
        }
        None
    }
}

impl Editor {
    pub(super) fn point_target(
        &mut self,
        document: &Document,
        rect: egui::Rect,
        position: egui::Pos2,
        context: &egui::Context,
    ) -> Option<Target> {
        let bounds = self.current().scene_bounds?;
        if let Err(error) = self.point_cache.prepare(
            (self.current().id, self.current().session.revision()),
            document,
            context,
        ) {
            self.status.error(error);
        }
        Query {
            document,
            camera: &self.viewport.camera,
            bounds,
            rect,
            position,
            view: &self.current().view,
            section: self.current().section.filter(|s| s.visible),
            source_part: self.alignment_source_part(),
        }
        .target(&self.point_cache.entries)
    }
}

#[cfg(test)]
mod mesh_tests {
    use super::*;
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn mesh_vertices_capture_the_nearest_point_without_a_triangle_hit() -> Result<()> {
        let mut document = Document::new("Captura de vértices")?;
        let (part, body) = document.add_mesh(
            None,
            "Triângulo".into(),
            Arc::new(MeshAsset::new(Mesh::new(
                vec![[0., 0., 0.], [10., 0., 0.], [0., 10., 0.]],
                vec![[0, 1, 2]],
            )?)),
        )?;
        let entries = HashMap::from([(
            body,
            body_anchors(document.body(body).ok_or("Corpo ausente")?)?,
        )]);
        let mut camera = Camera::default();
        camera.set_direction([0., 0., 1.]);
        let bounds = document.scene_mesh()?.ok_or("Cena ausente")?.bounds();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600., 600.));
        for (point, kind) in [
            ([0., 0., 0.], PointKind::Vertex),
            ([10., 0., 0.], PointKind::Vertex),
            ([5., 0., 0.], PointKind::EdgeMidpoint),
        ] {
            let position = sketch_scene::project(&camera, bounds, rect, point)
                .ok_or("Projeção ausente")?
                + egui::vec2(4., 6.);
            let query = Query {
                document: &document,
                camera: &camera,
                bounds,
                rect,
                position,
                view: &ViewState::default(),
                section: None,
                source_part: Some(part),
            };
            let target = query.target(&entries).ok_or("Vértice ausente")?;
            assert_eq!(target.pick.point, point);
            assert!(target.captured && target.kind == kind);
        }
        let position =
            sketch_scene::project(&camera, bounds, rect, [3., 2., 0.]).ok_or("Projeção ausente")?;
        let target = Query {
            document: &document,
            camera: &camera,
            bounds,
            rect,
            position,
            view: &ViewState::default(),
            section: None,
            source_part: Some(part),
        }
        .target(&entries)
        .ok_or("Pré-seleção distante ausente")?;
        assert_eq!(target.pick.point, [10. / 3., 10. / 3., 0.]);
        assert_eq!(target.kind, PointKind::FaceCenter);
        assert!(
            !target.captured,
            "No interior da face o apoio aparece antes da captura"
        );
        Ok(())
    }

    #[test]
    fn cache_discards_pending_results_from_an_older_document_revision() -> Result<()> {
        let (sender, receiver) = mpsc::channel();
        sender.send(Ok((
            HashMap::from([(
                BodyId(99),
                vec![PointAnchor {
                    point: [99.; 3],
                    kind: PointKind::Vertex,
                    circle: None,
                }],
            )]),
            HashMap::new(),
        )))?;
        let mut cache = Cache {
            revision: Some((1, 1)),
            entries: HashMap::new(),
            references: HashMap::new(),
            running: Some(receiver),
        };
        let document = Document::new("Nova revisão")?;
        let context = egui::Context::default();
        cache.prepare((1, 2), &document, &context)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while cache.running.is_some() {
            if Instant::now() > deadline {
                return Err("Consulta de pontos não terminou".into());
            }
            std::thread::sleep(Duration::from_millis(1));
            cache.prepare((1, 2), &document, &context)?;
        }
        assert!(cache.entries.is_empty());
        Ok(())
    }
}

#[cfg(all(test, feature = "occt"))]
mod tests {
    use super::*;
    use nator_kernel::Solid;
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    struct Fixture {
        document: Document,
        entries: Anchors,
        camera: Camera,
        bounds: Bounds,
        rect: egui::Rect,
        view: ViewState,
        part: PartId,
        body: BodyId,
    }
    impl Fixture {
        fn new(hole: bool) -> Result<Self> {
            let outer = Solid::cylinder([0., 0., 2.], 20., 4.)?;
            let solid = if hole {
                outer.cut(&Solid::cylinder([0.; 3], 10., 10.)?)?
            } else {
                outer
            };
            Self::from_solid(solid)
        }
        fn from_solid(mut solid: Solid) -> Result<Self> {
            let (mesh, faces) = solid.tessellate_with_faces(0.1, 0.2)?;
            let mut session = Session::new(Document::new("Centros circulares")?)?;
            let Effect::Imported { part, .. } = session.execute(Command::ImportSolid {
                destination: None,
                name: "Arruela".into(),
                geometry: Arc::new(MeshAsset::with_faces(mesh, faces)?),
                brep: Arc::new(solid.to_brep()?),
                faces: solid.face_count()?,
            })?
            else {
                return Err("Importação ausente".into());
            };
            let document = session.document().clone();
            let record = &document.part(part).ok_or("Peça ausente")?.bodies[0];
            let body = record.id;
            let entries = HashMap::from([(body, body_anchors(record)?)]);
            let mut camera = Camera::default();
            camera.set_direction([0., 0., 1.]);
            let bounds = document.scene_mesh()?.ok_or("Cena ausente")?.bounds();
            Ok(Self {
                document,
                entries,
                camera,
                bounds,
                rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.)),
                view: ViewState::default(),
                part,
                body,
            })
        }
        fn at(
            &self,
            point: Point3,
            offset: egui::Vec2,
            section: Option<section::Section>,
            source_part: Option<PartId>,
        ) -> Option<Target> {
            let position =
                sketch_scene::project(&self.camera, self.bounds, self.rect, point)? + offset;
            Query {
                document: &self.document,
                camera: &self.camera,
                bounds: self.bounds,
                rect: self.rect,
                position,
                view: &self.view,
                section,
                source_part,
            }
            .target(&self.entries)
        }
    }

    #[test]
    fn polygon_centers_capture_on_the_front_surface_instead_of_a_diagonal() -> Result<()> {
        let f = Fixture::from_solid(Solid::cuboid([0.; 3], [20., 10., 4.])?)?;
        let target = f
            .at([10., 5., 4.], egui::vec2(4., 6.), None, None)
            .ok_or("Centro ausente")?;
        assert_eq!(target.kind, PointKind::FaceCenter);
        assert!(target.captured);
        assert!(norm(sub(target.pick.point, [10., 5., 4.])) < 1e-9);
        Ok(())
    }

    #[test]
    fn washer_hole_discovers_front_center_before_capturing_it() -> Result<()> {
        let f = Fixture::new(true)?;
        let center = [0., 0., 6.];
        let screen =
            sketch_scene::project(&f.camera, f.bounds, f.rect, center).ok_or("Projeção ausente")?;
        let ray = screen_ray(&f.camera, f.bounds, f.rect, screen + egui::vec2(35., 0.));
        assert!(
            f.document.pick(ray)?.is_none(),
            "O cursor está no vazio do furo"
        );
        let discovered = f
            .at(center, egui::vec2(35., 0.), None, None)
            .ok_or("Centro não descoberto")?;
        assert!(discovered.kind == PointKind::CircleCenter && !discovered.captured);
        assert!(norm(sub(discovered.pick.point, center)) < 1e-9);
        let captured = f
            .at(center, egui::vec2(8., 0.), None, None)
            .ok_or("Centro não capturado")?;
        assert!(captured.captured);
        assert!(norm(sub(captured.pick.point, center)) < 1e-9);
        // Olhar por baixo deve oferecer o outro plano da arruela, jamais z=4.
        let mut f = f;
        f.camera.set_direction([0., 0., -1.]);
        let bottom = f
            .at([0., 0., 2.], egui::Vec2::ZERO, None, None)
            .ok_or("Centro inferior ausente")?;
        assert!(norm(sub(bottom.pick.point, [0., 0., 2.])) < 1e-9);
        Ok(())
    }

    #[test]
    fn disk_center_is_on_surface_and_capture_radius_is_in_screen_points() -> Result<()> {
        let mut f = Fixture::new(false)?;
        let center = [0., 0., 6.];
        for span in [0.7, 1.5, 4.] {
            f.camera.span = span;
            assert!(
                f.at(center, egui::vec2(13., 0.), None, None)
                    .ok_or("Captura ausente")?
                    .captured
            );
            let far = f
                .at(center, egui::vec2(16., 0.), None, None)
                .ok_or("Descoberta ausente")?;
            assert!(!far.captured);
            assert!(norm(sub(far.pick.point, center)) < 1e-9);
        }
        Ok(())
    }

    #[test]
    fn washer_inner_and_outer_quadrants_capture_on_the_front_surface() -> Result<()> {
        let f = Fixture::new(true)?;
        for radius in [10., 20.] {
            for point in [
                [radius, 0., 6.],
                [0., radius, 6.],
                [-radius, 0., 6.],
                [0., -radius, 6.],
            ] {
                let target = f
                    .at(point, egui::vec2(3., 2.), None, None)
                    .ok_or("Quadrante ausente")?;
                assert_eq!(target.kind, PointKind::CircleBoundary);
                assert!(target.captured);
                assert!(norm(sub(target.pick.point, point)) < 1e-9);
            }
        }
        Ok(())
    }

    #[test]
    fn arc_center_is_discovered_in_its_empty_sector_and_midpoint_stays_on_curve() -> Result<()> {
        use nator_sketch::{Curve, Plane, Sketch, areas};
        let mut sketch = Sketch::new("Arco", Plane::xy());
        let center = sketch.add_point([0., 0.])?;
        let start = sketch.add_point([10., 0.])?;
        let end = sketch.add_point([0., 10.])?;
        sketch.add_curve(
            Curve::Arc {
                center,
                start,
                end,
                clockwise: false,
            },
            false,
        )?;
        sketch.add_curve(
            Curve::Line {
                start: end,
                end: start,
            },
            false,
        )?;
        let region = areas::regions(&sketch)?
            .into_iter()
            .next()
            .ok_or("Região ausente")?;
        let f = Fixture::from_solid(Solid::extrude(&sketch, &region, 0., 2., "arc")?)?;
        let target = f
            .at([2., 2., 2.], egui::Vec2::ZERO, None, None)
            .ok_or("Centro do arco ausente")?;
        assert_eq!(target.kind, PointKind::ArcCenter);
        assert!(!target.captured);
        assert!(norm(sub(target.pick.point, [0., 0., 2.])) < 1e-9);
        assert!(f.at([-2., -2., 2.], egui::Vec2::ZERO, None, None).is_none());
        let captured = f
            .at([0., 0., 2.], egui::vec2(3., 2.), None, None)
            .ok_or("Captura do centro ausente")?;
        assert!(captured.captured && captured.kind == PointKind::ArcCenter);
        let radial = 10. / 2_f64.sqrt();
        let midpoint = [radial, radial, 2.];
        let captured = f
            .at(midpoint, egui::vec2(2., 3.), None, None)
            .ok_or("Captura do ponto médio ausente")?;
        assert!(captured.captured && captured.kind == PointKind::EdgeMidpoint);
        assert!(norm(sub(captured.pick.point, midpoint)) < 1e-8);
        Ok(())
    }

    #[test]
    fn circular_discovery_follows_the_projected_ellipse_in_oblique_views() -> Result<()> {
        let mut f = Fixture::new(true)?;
        f.camera.set_direction([0.3, -0.25, 1.]);
        let center = [0., 0., 6.];
        let target = f
            .at(center, egui::vec2(22., 0.), None, None)
            .ok_or("Centro oblíquo ausente")?;
        assert!(target.kind == PointKind::CircleCenter && !target.captured);
        assert!(norm(sub(target.pick.point, center)) < 1e-9);
        assert!(f.at([25., 0., 6.], egui::Vec2::ZERO, None, None).is_none());
        Ok(())
    }

    #[test]
    fn centers_follow_body_and_occurrence_placements_and_respect_visibility() -> Result<()> {
        let mut f = Fixture::new(true)?;
        let body_pose = Placement::new([3., -5., 8.], [0.3, 0.1, -0.4])?;
        let instance_pose = Placement::new([30., 40., 50.], [-0.2, 0.5, 0.6])?;
        let instance = f.document.instances()[0].id;
        f.document.body_mut(f.body)?.placement = body_pose;
        f.document.instance_mut(instance)?.placement = instance_pose;
        let pose = instance_pose.compose(body_pose)?;
        f.camera.set_direction(pose.apply_vector([0., 0., 1.]));
        f.bounds = f.document.scene_mesh()?.ok_or("Cena ausente")?.bounds();
        let center = pose.apply([0., 0., 6.]);
        let target = f
            .at(center, egui::Vec2::ZERO, None, Some(f.part))
            .ok_or("Centro transformado ausente")?;
        assert!(norm(sub(target.pick.point, center)) < 1e-8);
        assert!(
            norm(sub(
                target.pick.placement.apply([1., 2., 3.]),
                instance_pose.apply([1., 2., 3.])
            )) < 1e-9
        );
        assert!(
            f.at(center, egui::Vec2::ZERO, None, Some(PartId(u64::MAX)))
                .is_none()
        );
        f.view.locked.insert(instance);
        assert!(f.at(center, egui::Vec2::ZERO, None, None).is_none());
        f.view.locked.clear();
        f.document.body_mut(f.body)?.visible = false;
        assert!(f.at(center, egui::Vec2::ZERO, None, None).is_none());
        Ok(())
    }

    #[test]
    fn centers_behind_material_or_removed_by_section_are_not_offered() -> Result<()> {
        let mut f = Fixture::new(true)?;
        let center = [0., 0., 6.];
        let cut = section::Section {
            base: section::Base::Face {
                point: [0., 0., 1.],
                normal: [0., 0., 1.],
            },
            offset: 0.,
            flip: false,
            visible: true,
        };
        assert!(f.at(center, egui::Vec2::ZERO, Some(cut), None).is_none());
        // Tampa opaca sobre o furo: ela não tem apoios no cache, mas oculta os
        // centros da arruela. A seleção não deve atravessar material visível.
        let mut lid = Solid::cuboid([-25., -25., 10.], [50., 50., 2.])?;
        let (mesh, faces) = lid.tessellate_with_faces(0.1, 0.2)?;
        let mut session = Session::new(f.document)?;
        session.execute(Command::ImportSolid {
            destination: None,
            name: "Tampa".into(),
            geometry: Arc::new(MeshAsset::with_faces(mesh, faces)?),
            brep: Arc::new(lid.to_brep()?),
            faces: lid.face_count()?,
        })?;
        f.document = session.document().clone();
        let lid_instance = f.document.instances().last().ok_or("Tampa ausente")?.id;
        // A importação posiciona peças novas ao lado; aqui queremos sobreposição.
        f.document.instance_mut(lid_instance)?.placement = Placement::default();
        f.bounds = f.document.scene_mesh()?.ok_or("Cena ausente")?.bounds();
        assert!(f.at(center, egui::Vec2::ZERO, None, None).is_none());
        Ok(())
    }
}
