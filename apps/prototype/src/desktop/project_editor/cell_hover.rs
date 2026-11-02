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
    /// Escolha de área da extrusão: nada além de células e faces planas da peça.
    area: bool,
    section: Option<section::Section>,
    /// Dois raios fixos da vista identificam orientação, projeção, zoom e pan,
    /// sem confundir essas mudanças com o deslocamento do ponteiro.
    view_rays: [(Point3, Point3); 2],
    rect: egui::Rect,
    placement: Placement,
}
impl Key {
    fn same_context(&self, other: &Self) -> bool {
        self.scene == other.scene
            && self.paint == other.paint
            && self.area == other.area
            && self.section == other.section
            && self.view_rays == other.view_rays
            && self.rect == other.rect
            && self.placement == other.placement
    }
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

/// Realce de uma célula em coordenadas locais, colocado no mundo pela pose do corpo.
pub(super) fn area_overlay(mesh: &Mesh, pose: Placement) -> Option<highlight::Overlay> {
    let vertices = mesh.vertices().iter().map(|p| pose.apply(*p)).collect();
    Mesh::new(vertices, mesh.triangles().to_vec())
        .ok()
        .map(|mesh| mesh_overlay(&mesh))
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
    // Na extrusão, `profile_overlay` já cuida das regiões de rascunho.
    let profile = if key.paint || key.area {
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
    // Curvas, malhas e outras peças não são áreas de extrusão.
    if key.area
        && (pick.part != part
            || document.body(pick.body).is_none_or(|b| b.brep.is_none())
            || nator_commands::measure::sketch_plane(document, &pick).is_err())
    {
        return Computed {
            key,
            answer: Answer::Empty,
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
    fn poll(&mut self, key: &Key) {
        // Mover o cursor permite manter a última resposta enquanto o trabalhador
        // verifica a superfície nova. Mudanças na vista/documento nunca permitem
        // esse reaproveitamento: poderiam mostrar uma área oculta ou de outra peça.
        if self
            .ready
            .as_ref()
            .is_some_and(|r| !r.key.same_context(key))
        {
            self.ready = None;
            self.cached = None;
        }
        if let Some(receiver) = &self.running {
            match receiver.try_recv() {
                Ok(result) => {
                    // Uma consulta de um contexto abandonado pode terminar tarde;
                    // ela não deve substituir o realce nem o cache atuais.
                    // Se o ponteiro voltou a uma posição já pronta, uma entrega
                    // de outra posição também não pode substituir essa resposta.
                    if result.key.same_context(key)
                        && (result.key == *key || self.ready.as_ref().is_none_or(|r| r.key != *key))
                    {
                        self.cached = result.cached.clone();
                        self.ready = Some(result);
                    }
                    self.running = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => self.running = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }
    fn query(&mut self, query: Query, context: &egui::Context) -> Option<&Answer> {
        self.poll(&query.key);
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
        // Exigir igualdade do raio aqui causava quadros vazios a cada movimento
        // e impedia entregar resultados durante movimento contínuo do ponteiro.
        self.ready.as_ref().map(|r| &r.answer)
    }
}
impl Editor {
    pub(super) fn cell_hover_active(&self) -> bool {
        let area = self
            .feature_edit
            .as_ref()
            .is_some_and(|d| d.tab() == self.current().id && d.choosing_area());
        self.current().active_part.is_some()
            && (area
                || (self.pick_mode == navigation::PickMode::Face
                    && (self.selection_active() || self.paint_edit.is_some())))
            && self.current().sketch_edit.is_none()
    }
    pub(super) fn update_cell_hover(&mut self, ui: &egui::Ui, response: &egui::Response) {
        self.hover = None;
        // A região do rascunho sob o cursor já está realçada e tem prioridade no clique.
        if self.profile_hovered {
            return;
        }
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
            area: self.feature_edit.is_some(),
            section: self.current().section,
            view_rays: [[0., 0.], [1., 1.]].map(|point| {
                let ray =
                    self.viewport
                        .camera
                        .world_ray(point, bounds, f64::from(rect.aspect_ratio()));
                (ray.origin, ray.direction)
            }),
            rect,
            placement,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    fn key() -> Key {
        Key {
            scene: SceneKey {
                tab: 1,
                revision: 1,
                view: 0,
                active: Some(PartId(1)),
                preview: 0,
            },
            origin: [0., 0., 10.],
            direction: [0., 0., -1.],
            paint: false,
            area: false,
            section: None,
            view_rays: [
                ([0., 0., 10.], [0., 0., -1.]),
                ([1., 1., 10.], [0., 0., -1.]),
            ],
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.)),
            placement: Placement::default(),
        }
    }
    fn computed(key: Key, label: &str) -> Computed {
        Computed {
            key,
            answer: Answer::Cell(
                highlight::Overlay {
                    triangles: vec![[[0.; 3], [1., 0., 0.], [0., 1., 0.]]],
                    ..Default::default()
                },
                label.into(),
            ),
            cached: None,
        }
    }
    fn query(key: Key) -> Result<Query, Box<dyn Error>> {
        Ok(Query {
            part: PartId(1),
            placement: key.placement,
            section: key.section,
            key,
            document: Document::new("Realce assíncrono")?,
            view: ViewState::default(),
        })
    }
    fn label(answer: Option<&Answer>) -> Option<&str> {
        match answer {
            Some(Answer::Cell(_, label)) => Some(label.as_str()),
            _ => None,
        }
    }

    #[test]
    fn moving_pointer_keeps_highlight_while_worker_is_pending_and_accepts_finished_results()
    -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let first = key();
        let mut cache = Cache {
            ready: Some(computed(first.clone(), "Área")),
            ..Default::default()
        };
        // A entrega é controlada por canais, sem depender da velocidade do OCCT
        // ou de sleeps: cada quadro tem um raio novo e um trabalhador pendente.
        for frame in 1..=20 {
            let mut current = first.clone();
            current.origin[0] = f64::from(frame);
            let (sender, receiver) = mpsc::channel();
            cache.running = Some(receiver);
            assert_eq!(
                label(cache.query(query(current.clone())?, &context)),
                Some("Área")
            );
            sender.send(computed(current.clone(), "Área"))?;
            let mut next = current;
            next.origin[0] += 0.5;
            cache.poll(&next);
            assert!(
                cache
                    .ready
                    .as_ref()
                    .is_some_and(|r| r.key.origin[0] == f64::from(frame))
            );
            let (_pending, receiver) = mpsc::channel();
            cache.running = Some(receiver);
            assert_eq!(label(cache.query(query(next)?, &context)), Some("Área"));
        }
        // A resposta vazia também precisa substituir o destaque: sair da peça
        // não pode deixar uma superfície presa na tela durante movimento contínuo.
        let mut next = first;
        next.origin[0] = 50.;
        let (sender, receiver) = mpsc::channel();
        cache.running = Some(receiver);
        sender.send(Computed {
            key: next.clone(),
            answer: Answer::Empty,
            cached: None,
        })?;
        assert!(matches!(
            cache.query(query(next)?, &context),
            Some(Answer::Empty)
        ));
        Ok(())
    }

    #[test]
    fn view_and_document_changes_discard_highlights_and_late_results() -> Result<(), Box<dyn Error>>
    {
        let original = key();
        let mut changes = Vec::new();
        let mut changed = original.clone();
        changed.scene.tab += 1;
        changes.push(changed);
        let mut changed = original.clone();
        changed.scene.revision += 1;
        changes.push(changed);
        let mut changed = original.clone();
        changed.scene.view += 1;
        changes.push(changed);
        let mut changed = original.clone();
        changed.scene.active = Some(PartId(2));
        changes.push(changed);
        let mut changed = original.clone();
        changed.scene.preview += 1;
        changes.push(changed);
        let mut changed = original.clone();
        changed.paint = true;
        changes.push(changed);
        let mut changed = original.clone();
        changed.area = true;
        changes.push(changed);
        let mut changed = original.clone();
        changed.section = Some(section::Section {
            base: section::Base::Principal(0),
            offset: 2.,
            flip: false,
            visible: true,
        });
        changes.push(changed);
        let mut changed = original.clone();
        changed.view_rays[0].0[0] += 1.;
        changes.push(changed);
        let mut changed = original.clone();
        changed.view_rays[1].1[0] += 1.;
        changes.push(changed);
        let mut changed = original.clone();
        changed.rect.max.x += 1.;
        changes.push(changed);
        let mut changed = original.clone();
        changed.placement = Placement::new([1., 0., 0.], [0.; 3])?;
        changes.push(changed);
        for changed in changes {
            let mut cache = Cache {
                ready: Some(computed(original.clone(), "Antigo")),
                ..Default::default()
            };
            let (sender, receiver) = mpsc::channel();
            cache.running = Some(receiver);
            cache.poll(&changed);
            assert!(cache.ready.is_none());
            sender.send(computed(original.clone(), "Entrega antiga"))?;
            cache.poll(&changed);
            assert!(cache.ready.is_none());
        }
        Ok(())
    }

    #[test]
    fn finished_current_highlight_is_not_replaced_by_an_older_pointer_position()
    -> Result<(), Box<dyn Error>> {
        let current = key();
        let mut previous = current.clone();
        previous.origin[0] = 10.;
        let mut cache = Cache {
            ready: Some(computed(current.clone(), "Atual")),
            ..Default::default()
        };
        let (sender, receiver) = mpsc::channel();
        cache.running = Some(receiver);
        sender.send(computed(previous, "Anterior"))?;
        assert_eq!(
            label(cache.query(query(current)?, &egui::Context::default())),
            Some("Atual")
        );
        Ok(())
    }
}
