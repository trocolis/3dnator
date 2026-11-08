//! Pré-seleção de suportes planos. A tesselação decide qual superfície está sob o
//! cursor; o núcleo confirma seu plano exato antes de desenhar o realce. A consulta
//! inteira roda fora da interface e não altera seleção, documento nem histórico.
use super::*;
use nator_geometry::query::Ray;

#[derive(Clone, Copy, PartialEq)]
struct Key {
    scene: SceneKey,
    section: Option<section::Section>,
    origin: Point3,
    direction: Point3,
}

struct Candidate {
    overlay: highlight::Overlay,
    label: String,
}

/// Camadas sob o cursor, da frente para trás: cada face plana visível vira um suporte; uma
/// superfície curva ou malha na frente ocupa a primeira camada sem suporte (`None`), para
/// não ser atravessada sem querer. As setas ↑/↓ percorrem as camadas.
type Answer = Arc<Vec<Option<Arc<Candidate>>>>;

/// Faces planas sob o raio, na ordem das camadas oferecidas (ver `navigation::face_layers`).
pub(super) fn plane_layers(
    document: &Document,
    view: &ViewState,
    section: Option<section::Section>,
    ray: Ray,
) -> Vec<Option<Pick>> {
    navigation::face_layers(document, view, section, ray, |pick| {
        nator_commands::measure::sketch_plane(document, pick).is_ok()
    })
}

#[derive(Default)]
pub(super) struct Cache {
    ready: Option<(Key, Answer)>,
    running: Option<(Key, Receiver<Answer>)>,
}

/// Não atravessar uma superfície inválida para realçar outra atrás dela. Curvas e
/// malhas obstruem o raio, mas não são suportes exatos para um novo rascunho.
fn candidates(document: &Document, view: &ViewState, key: Key) -> Answer {
    let ray = Ray {
        origin: key.origin,
        direction: key.direction,
    };
    Arc::new(
        plane_layers(document, view, key.section, ray)
            .into_iter()
            .map(|pick| pick.and_then(|pick| layer(document, &pick)))
            .collect(),
    )
}

/// Primeira camada: o suporte que o clique simples escolhe.
#[cfg(all(test, feature = "occt"))]
fn candidate(document: &Document, view: &ViewState, key: Key) -> Option<Arc<Candidate>> {
    candidates(document, view, key).first().cloned().flatten()
}

fn layer(document: &Document, pick: &Pick) -> Option<Arc<Candidate>> {
    let body = document.body(pick.body)?;
    let pose = pick.placement.compose(body.placement).ok()?;
    let face = u32::try_from(document.face_index_at(pick).ok()?).ok()?;
    Some(Arc::new(Candidate {
        overlay: highlight::face(body, pose, face),
        label: body.name.clone(),
    }))
}

impl Cache {
    fn query(
        &mut self,
        key: Key,
        document: &Document,
        view: &ViewState,
        context: &egui::Context,
    ) -> Result<Option<Answer>, String> {
        if let Some((running_key, receiver)) = &self.running {
            let answer = match receiver.try_recv() {
                Ok(answer) => Some(answer),
                Err(mpsc::TryRecvError::Disconnected) => Some(Arc::new(Vec::new())),
                Err(mpsc::TryRecvError::Empty) => None,
            };
            if let Some(answer) = answer {
                self.ready = Some((*running_key, answer));
                self.running = None;
            }
        }
        if let Some((ready_key, answer)) = &self.ready
            && *ready_key == key
        {
            return Ok(Some(answer.clone()));
        }
        // Uma consulta por vez: movimento contínuo não cria uma fila de workers.
        // Uma entrega antiga nunca é desenhada sobre outra aba/revisão/posição.
        if self.running.is_none() {
            let document = document.clone();
            let view = view.clone();
            let context = context.clone();
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("sketch-plane-hover".into())
                .spawn(move || {
                    let _ = sender.send(candidates(&document, &view, key));
                    context.request_repaint();
                })
                .map_err(|e| e.to_string())?;
            self.running = Some((key, receiver));
        }
        Ok(None)
    }
}

/// Muda a camada escolhida com ↑/↓ entre `total` camadas; devolve se mudou.
pub(in crate::desktop::project_editor) fn step_layer(
    context: &egui::Context,
    layer: &mut usize,
    total: usize,
) -> bool {
    if total < 2 {
        return false;
    }
    let (up, down) = context.input_mut(|i| {
        (
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
        )
    });
    let before = *layer;
    if down {
        *layer = (*layer + 1) % total;
    }
    if up {
        *layer = (*layer + total - 1) % total;
    }
    *layer != before
}

impl Editor {
    pub(in crate::desktop::project_editor) fn update_plane_hover(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
    ) {
        self.hover = None;
        let Some(picker) = self.current().plane_pick.as_ref() else {
            return;
        };
        let Some(position) = response.hover_pos().filter(|p| {
            !picker.origin_hovered
                && !self.busy()
                && !ui.input(|i| i.pointer.any_down())
                && !view_cube::rect(response.rect).contains(*p)
        }) else {
            return;
        };
        let rect = response.rect;
        let ray = self.viewport.camera.world_ray(
            [
                f64::from((position.x - rect.center().x) / rect.width() * 2.),
                f64::from((rect.center().y - position.y) / rect.height() * 2.),
            ],
            self.display_bounds(),
            f64::from(rect.aspect_ratio()),
        );
        let key = Key {
            scene: self.scene_key(),
            section: self.current().section,
            origin: ray.origin,
            direction: ray.direction,
        };
        let tab = self.current_mut();
        let Some(picker) = tab.plane_pick.as_mut() else {
            return;
        };
        match picker
            .hover
            .query(key, tab.session.document(), &tab.view, ui.ctx())
        {
            Ok(Some(layers)) => {
                picker.faces_under = layers.len();
                // As camadas dos planos da origem e de trabalho vêm antes das faces.
                let index = picker.layer.checked_sub(picker.planes_under);
                let total = picker.planes_under + layers.len();
                if let Some(Some(candidate)) = index.and_then(|i| layers.get(i)) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    let label = if total > 1 {
                        format!("{} ({}/{total}, ↑/↓)", candidate.label, picker.layer + 1)
                    } else {
                        candidate.label.clone()
                    };
                    let mut overlay = candidate.overlay.clone();
                    // Faces atrás da primeira continuam visíveis através do corpo.
                    overlay.through = index.is_some_and(|i| i > 0);
                    self.hover = Some(highlight::Hover::fixed(
                        key.scene.tab,
                        key.scene.revision,
                        position,
                        overlay,
                        label,
                    ));
                }
            }
            Ok(None) => {}
            Err(error) => self.status.error(error),
        }
    }
}

#[cfg(all(test, feature = "occt"))]
mod tests {
    use super::*;
    use nator_kernel::Solid;

    fn import(
        document: &mut Document,
        mut solid: Solid,
    ) -> Result<BodyId, Box<dyn std::error::Error>> {
        let (mesh, faces) = solid.tessellate_with_faces(0.1, 0.2)?;
        let (_, body) = document.add_mesh(
            None,
            "Suporte".into(),
            Arc::new(MeshAsset::with_faces(mesh, faces)?),
        )?;
        // O realce não depende de nomes topológicos: aceita as faces de chanfro
        // sem nome e os nomes repetidos de projetos antigos, como a criação faz.
        document.body_mut(body)?.brep = Some(Arc::new(solid.to_brep()?));
        Ok(body)
    }

    fn key(origin: Point3, direction: Point3) -> Key {
        Key {
            scene: SceneKey {
                tab: 1,
                revision: 0,
                view: 0,
                active: None,
                preview: 0,
            },
            section: None,
            origin,
            direction,
        }
    }

    #[test]
    fn only_exact_planar_faces_are_highlighted_in_the_occurrence_frame()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut document = Document::new("Suportes de rascunho")?;
        let body = import(&mut document, Solid::cuboid([0.; 3], [20.; 3])?)?;
        document.body_mut(body)?.placement = Placement::new([7., -3., 15.], [0.2, -0.4, 0.1])?;
        let instance = document.instances()[0].id;
        document.instance_mut(instance)?.placement =
            Placement::new([70., 30., -10.], [-0.1, 0.2, 0.3])?;
        let pose = document
            .instance(instance)
            .ok_or("Instância ausente")?
            .placement
            .compose(document.body(body).ok_or("Corpo ausente")?.placement)?;
        let ray = key(pose.apply([5., 5., 100.]), pose.apply_vector([0., 0., -1.]));
        let mut view = ViewState::default();
        let hovered = candidate(&document, &view, ray).ok_or("Face plana sem realce")?;
        assert_eq!(hovered.overlay.triangles.len(), 2);
        for p in hovered.overlay.triangles.iter().flatten() {
            assert!((pose.inverse_apply(*p)[2] - 20.).abs() < 1e-8);
        }
        view.locked.insert(instance);
        assert!(candidate(&document, &view, ray).is_none());

        let mut cylinder = Document::new("Superfície curva")?;
        import(&mut cylinder, Solid::cylinder([0.; 3], 10., 20.)?)?;
        let view = ViewState::default();
        assert!(candidate(&cylinder, &view, key([30., 0., 10.], [-1., 0., 0.])).is_none());
        assert!(candidate(&cylinder, &view, key([0., 0., 100.], [0., 0., -1.])).is_some());
        // Uma malha na frente do suporte não permite realçar o sólido atrás dela.
        cylinder.add_mesh(
            None,
            "Malha sobre a tampa".into(),
            Arc::new(MeshAsset::new(Mesh::new(
                vec![[-30., -30., 40.], [30., -30., 40.], [0., 30., 40.]],
                vec![[0, 1, 2]],
            )?)),
        )?;
        assert!(candidate(&cylinder, &view, key([0., 0., 100.], [0., 0., -1.])).is_none());
        Ok(())
    }

    /// Dois blocos empilhados sob o cursor: topo e base de cada um, da frente para trás
    /// (a base é a face oposta, vista por trás); uma malha na frente ocupa a primeira
    /// camada, sem suporte, e as faces atrás continuam alcançáveis pelas setas.
    #[test]
    fn overlapping_planes_are_layered_front_to_back() -> Result<(), Box<dyn std::error::Error>> {
        let mut document = Document::new("Planos sobrepostos")?;
        import(&mut document, Solid::cuboid([0.; 3], [20., 20., 5.])?)?;
        import(&mut document, Solid::cuboid([0., 0., 10.], [20., 20., 5.])?)?;
        let view = ViewState::default();
        let ray = key([5., 5., 100.], [0., 0., -1.]);
        let heights = |layers: &Answer| -> Vec<Option<f64>> {
            layers
                .iter()
                .map(|l| {
                    l.as_ref()
                        .and_then(|c| c.overlay.triangles.first().map(|t| t[0][2]))
                })
                .collect()
        };
        let layers = candidates(&document, &view, ray);
        assert_eq!(heights(&layers), [Some(15.), Some(10.), Some(5.), Some(0.)]);
        document.add_mesh(
            None,
            "Malha na frente".into(),
            Arc::new(MeshAsset::new(Mesh::new(
                vec![[-30., -30., 40.], [30., -30., 40.], [0., 30., 40.]],
                vec![[0, 1, 2]],
            )?)),
        )?;
        let layers = candidates(&document, &view, ray);
        assert_eq!(
            heights(&layers),
            [None, Some(15.), Some(10.), Some(5.), Some(0.)]
        );
        // As setas percorrem as camadas em volta.
        let context = egui::Context::default();
        let mut layer = 0;
        let press = |key: egui::Key| egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let step = |layer: &mut usize, key| {
            let mut changed = false;
            let mut output = context.run_ui(press(key), |ui| {
                changed = step_layer(ui.ctx(), layer, 5);
            });
            output.textures_delta.clear();
            changed
        };
        assert!(step(&mut layer, egui::Key::ArrowDown));
        assert!(step(&mut layer, egui::Key::ArrowDown));
        assert_eq!(layer, 2);
        for _ in 0..3 {
            assert!(step(&mut layer, egui::Key::ArrowUp));
        }
        assert_eq!(layer, 4);
        Ok(())
    }

    #[test]
    fn queued_hover_from_an_old_revision_or_cursor_is_never_displayed()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut document = Document::new("Entregas de realce")?;
        import(&mut document, Solid::cuboid([0.; 3], [20.; 3])?)?;
        let view = ViewState::default();
        let old = key([5., 5., 100.], [0., 0., -1.]);
        let stale = candidates(&document, &view, old);
        assert!(
            stale.first().is_some_and(Option::is_some),
            "Suporte ausente"
        );
        let (sender, receiver) = mpsc::channel();
        sender.send(stale)?;
        let mut cache = Cache {
            ready: None,
            running: Some((old, receiver)),
        };
        let mut current = old;
        current.scene.revision += 1;
        current.origin[0] = 200.;
        let context = egui::Context::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            // Nunca a resposta antiga: enquanto calcula, nada; pronta, a do cursor atual
            // (vazia, porque o raio passa ao lado do bloco).
            let answer = cache.query(current, &document, &view, &context)?;
            assert!(answer.is_none_or(|layers| layers.is_empty()));
            if cache.ready.as_ref().is_some_and(|(k, _)| *k == current) {
                break;
            }
            if Instant::now() > deadline {
                return Err("Pré-seleção não concluiu".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }
}
