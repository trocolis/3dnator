//! Pré-seleção das células visíveis na peça. A consulta não modifica seleção ou
//! documento e não executa arranjos/booleanas na thread da interface.
use super::*;
use nator_document::assembly::Pick;
use nator_geometry::{cross, dot, norm, query::Ray, sub};

#[derive(Clone, PartialEq)]
struct Key {
    scene: SceneKey,
    origin: Point3,
    direction: Point3,
    paint: bool,
    section: Option<section::Section>,
}
struct Query {
    key: Key,
    document: Document,
    view: ViewState,
    section: Option<section::Section>,
    part: PartId,
    placement: Placement,
}
#[derive(Clone)]
struct Cell {
    scene: SceneKey,
    body: BodyId,
    mesh: Arc<Mesh>,
}
enum Answer {
    Cell(highlight::Overlay, String),
    Face(Pick),
    Empty,
}
struct Computed {
    key: Key,
    answer: Answer,
    cached: Option<Cell>,
}
#[derive(Default)]
pub(super) struct Cache {
    ready: Option<Computed>,
    running: Option<Receiver<Computed>>,
    cached: Option<Cell>,
}

fn mesh_overlay(mesh: &Mesh) -> highlight::Overlay {
    // Fronteiras de incidência única, sem desenhar diagonais da tesselação.
    let mut edges = std::collections::HashMap::<(u32, u32), (usize, Point3)>::new();
    for triangle in mesh.triangles() {
        let [a, b, c] = mesh.triangle_points(*triangle);
        let normal = cross(sub(b, a), sub(c, a));
        for i in 0..3 {
            let (a, b) = (triangle[i], triangle[(i + 1) % 3]);
            let item = edges.entry((a.min(b), a.max(b))).or_insert((0, normal));
            item.0 += 1;
        }
    }
    highlight::Overlay {
        triangles: mesh
            .triangles()
            .iter()
            .map(|t| mesh.triangle_points(*t))
            .collect(),
        segments: edges
            .into_iter()
            .filter(|(_, (n, _))| *n == 1)
            .map(|((a, b), (_, normal))| {
                (
                    [mesh.vertices()[a as usize], mesh.vertices()[b as usize]],
                    Some(normal),
                )
            })
            .collect(),
        ..Default::default()
    }
}

/// Rascunhos sem apoio sólido continuam oferecendo a mesma área de perfil usada
/// na Extrusão. Faces à frente impedem destacar um desenho escondido atrás delas.
fn free_profile(
    document: &Document,
    part: PartId,
    placement: Placement,
    ray: Ray,
    occluder: Option<&Pick>,
) -> Option<(highlight::Overlay, String, f64)> {
    let piece = document.part(part)?;
    let blocked = occluder
        .map(|p| dot(sub(p.point, ray.origin), ray.direction))
        .unwrap_or(f64::INFINITY);
    let mut best: Option<(highlight::Overlay, String, f64, f64)> = None;
    for record in piece.sketches.iter().filter(|s| piece.shows_sketch(s)) {
        let sketch = &record.sketch;
        let origin = placement.apply(sketch.plane.origin);
        let normal = placement.apply_vector(sketch.plane.normal());
        let denominator = dot(normal, ray.direction);
        if denominator.abs() < 1e-9 {
            continue;
        }
        let distance = dot(normal, sub(origin, ray.origin)) / denominator;
        if distance < 0. || distance > blocked + 1e-6 {
            continue;
        }
        let point = std::array::from_fn(|i| ray.origin[i] + distance * ray.direction[i]);
        let point = sketch.plane.project(placement.inverse_apply(point));
        let Ok(regions) = nator_sketch::areas::regions(sketch) else {
            continue;
        };
        for region in regions
            .iter()
            .filter(|r| r.outer.contains(point) && !r.holes.iter().any(|h| h.contains(point)))
        {
            let area = region.area();
            if best.as_ref().is_some_and(|(_, _, d, a)| {
                *d < distance - 1e-6 || ((*d - distance).abs() < 1e-6 && *a <= area)
            }) {
                continue;
            }
            let contours = std::iter::once(&region.outer)
                .chain(&region.holes)
                .map(|w| {
                    w.sample(128)
                        .into_iter()
                        .map(|p| placement.apply(sketch.plane.world(p)))
                        .collect()
                })
                .collect();
            best = Some((
                highlight::Overlay {
                    contours,
                    ..Default::default()
                },
                sketch.name.clone(),
                distance,
                area,
            ));
        }
    }
    best.map(|(overlay, label, d, _)| (overlay, label, d))
}
fn compute(
    document: &Document,
    view: &ViewState,
    section: Option<section::Section>,
    part: PartId,
    placement: Placement,
    key: Key,
    cached: Option<Cell>,
) -> Computed {
    let ray = Ray {
        origin: key.origin,
        direction: key.direction,
    };
    let pick = document
        .pick_all_with(ray, |o| view.pickable(o))
        .ok()
        .and_then(|p| section::first_visible(p, section));
    let profile = if key.paint {
        None
    } else {
        free_profile(document, part, placement, ray, pick.as_ref())
    };
    if let Some((overlay, label, distance)) = &profile
        && pick
            .as_ref()
            .is_none_or(|p| dot(sub(p.point, ray.origin), ray.direction) > distance + 1e-6)
    {
        return Computed {
            key,
            answer: Answer::Cell(overlay.clone(), label.clone()),
            cached,
        };
    }
    let Some(pick) = pick else {
        return Computed {
            key,
            answer: profile
                .map(|(o, l, _)| Answer::Cell(o, l))
                .unwrap_or(Answer::Empty),
            cached,
        };
    };
    if pick.part == part
        && document
            .body(pick.body)
            .is_some_and(|b| b.brep.is_some() && b.geometry.triangle_faces().is_some())
    {
        let label = document
            .body(pick.body)
            .map(|b| b.name.clone())
            .unwrap_or_default();
        // Dentro da mesma célula, basta consultar sua malha já recortada. Ainda
        // verificamos o corpo mais próximo para respeitar oclusões e isolamento.
        if let Some(cell) = &cached
            && cell.scene == key.scene
            && cell.body == pick.body
            && cell
                .mesh
                .ray_hit(ray)
                .is_some_and(|hit| norm(sub(hit.point, pick.point)) < 1e-5)
        {
            return Computed {
                key,
                answer: Answer::Cell(mesh_overlay(&cell.mesh), label),
                cached,
            };
        }
        match nator_commands::painting::highlight(document, &pick) {
            Ok(Some(mesh)) => {
                let mesh = Arc::new(mesh);
                let overlay = mesh_overlay(&mesh);
                let cached = Some(Cell {
                    scene: key.scene,
                    body: pick.body,
                    mesh,
                });
                return Computed {
                    key,
                    answer: Answer::Cell(overlay, label),
                    cached,
                };
            }
            // Não mostrar a face inteira como substituta de uma célula inválida.
            Err(_) => {
                return Computed {
                    key,
                    answer: Answer::Empty,
                    cached: None,
                };
            }
            Ok(None) => {
                if !nator_kernel::is_available()
                    && let Some((overlay, label, _)) = profile
                {
                    return Computed {
                        key,
                        answer: Answer::Cell(overlay, label),
                        cached,
                    };
                }
            }
        }
    } else if pick.part == part
        && let Some((overlay, label, _)) = profile
    {
        return Computed {
            key,
            answer: Answer::Cell(overlay, label),
            cached,
        };
    }
    Computed {
        key,
        answer: Answer::Face(pick),
        cached,
    }
}
impl Cache {
    fn query(&mut self, query: Query, context: &egui::Context) -> Option<&Answer> {
        if let Some(receiver) = &self.running {
            match receiver.try_recv() {
                Ok(result) => {
                    self.cached = result.cached.clone();
                    self.ready = Some(result);
                    self.running = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => self.running = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.ready.as_ref().is_some_and(|r| r.key == query.key) {
            return self.ready.as_ref().map(|r| &r.answer);
        }
        if self.running.is_none() {
            let Query {
                key,
                document,
                view,
                section,
                part,
                placement,
            } = query;
            let cached = self.cached.clone();
            let context = context.clone();
            let (sender, receiver) = mpsc::channel();
            if std::thread::Builder::new()
                .name("cell-hover".into())
                .spawn(move || {
                    let result = compute(&document, &view, section, part, placement, key, cached);
                    let _ = sender.send(result);
                    context.request_repaint();
                })
                .is_ok()
            {
                self.running = Some(receiver);
            }
        }
        context.request_repaint_after(Duration::from_millis(16));
        None
    }
}
impl Editor {
    pub(super) fn cell_hover_active(&self) -> bool {
        self.current().active_part.is_some()
            && self.pick_mode == navigation::PickMode::Face
            && (self.selection_active() || self.paint_edit.is_some())
            && self.current().sketch_edit.is_none()
    }
    pub(super) fn update_cell_hover(&mut self, ui: &egui::Ui, response: &egui::Response) {
        self.hover = None;
        if self
            .scene
            .as_ref()
            .is_some_and(|s| s.triangles().len() > highlight::HOVER_TRIANGLES)
        {
            return;
        }
        let Some(position) = response.hover_pos().filter(|p| {
            !view_cube::rect(response.rect).contains(*p) && !ui.input(|i| i.pointer.any_down())
        }) else {
            return;
        };
        let Some(part) = self.current().active_part else {
            return;
        };
        let Ok(placement) = sketch_scene::part_placement(self.current(), part) else {
            return;
        };
        let bounds = self.display_bounds();
        let rect = response.rect;
        let ray = self.viewport.camera.world_ray(
            [
                f64::from((position.x - rect.center().x) / rect.width() * 2.),
                f64::from((rect.center().y - position.y) / rect.height() * 2.),
            ],
            bounds,
            f64::from(rect.aspect_ratio()),
        );
        let key = Key {
            scene: self.scene_key(),
            origin: ray.origin,
            direction: ray.direction,
            paint: self.paint_edit.is_some(),
            section: self.current().section,
        };
        let query = Query {
            key: key.clone(),
            document: self.picking_document(),
            view: self.current().view.clone(),
            section: self.current().section,
            part,
            placement,
        };
        let answer = self.cell_hover.query(query, ui.ctx());
        let (overlay, label) = match answer {
            Some(Answer::Cell(o, l)) => (o.clone(), l.clone()),
            Some(Answer::Face(pick)) => {
                let pick = pick.clone();
                let document = self.picking_document();
                self.pick_overlay(&document, &pick)
            }
            Some(Answer::Empty) => (Default::default(), String::new()),
            None => return,
        };
        self.hover = Some(highlight::Hover::fixed(
            key.scene.tab,
            key.scene.revision,
            position,
            overlay,
            label,
        ));
    }
}
