//! Escolha de perfis diretamente na área 3D: com o campo Perfil ativo, as regiões fechadas
//! dos rascunhos da peça são realçadas sob o cursor e escolhidas por clique, no plano do
//! rascunho e na posição da ocorrência. Só regiões válidas de `profiles::regions` aparecem;
//! nada é fechado ou corrigido implicitamente.
use super::{feature_editor::Field, sketch_scene::Frame, *};
use nator_sketch::{Point2, profiles};
type AreaResult = Result<Arc<Areas>, String>;

/// Uma consulta por vez e cache por aba/revisão. Arranjos densos não bloqueiam
/// navegação nem são recalculados a cada quadro; entregas de revisões antigas são descartadas.
#[derive(Default)]
pub(super) struct Cache {
    revision: Option<(u64, u64)>,
    entries: std::collections::HashMap<SketchId, AreaResult>,
    running: Option<(SketchId, Receiver<AreaResult>)>,
}
pub(super) struct Areas {
    plane: nator_sketch::Plane,
    pub regions: Arc<Vec<profiles::Region>>,
    references: Vec<nator_document::features::RegionRef>,
}
impl Cache {
    fn query(
        &mut self,
        revision: (u64, u64),
        id: SketchId,
        sketch: &nator_sketch::Sketch,
        context: &egui::Context,
    ) -> Result<Option<Arc<Areas>>, String> {
        if self.revision != Some(revision) {
            self.entries.clear();
            self.running = None;
            self.revision = Some(revision);
        }
        if let Some((id, receiver)) = &self.running {
            let ready = match receiver.try_recv() {
                Ok(result) => Some((*id, result)),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some((*id, Err(text(T::ProfileQueryFailed).into())))
                }
                Err(mpsc::TryRecvError::Empty) => None,
            };
            if let Some((id, result)) = ready {
                self.entries.insert(id, result);
                self.running = None;
            }
        }
        if let Some(entry) = self.entries.get(&id) {
            return entry.clone().map(Some);
        }
        if self.running.is_none() {
            let sketch = sketch.clone();
            let (sender, receiver) = mpsc::channel();
            let context = context.clone();
            std::thread::Builder::new()
                .name("profile-areas".into())
                .spawn(move || {
                    let result = nator_sketch::areas::regions(&sketch)
                        .and_then(|regions| {
                            let references = regions
                                .iter()
                                .map(|r| {
                                    nator_sketch::areas::signature(&sketch, r)
                                        .map(nator_document::features::RegionRef::Area)
                                })
                                .collect::<nator_sketch::Result<Vec<_>>>()?;
                            Ok(Arc::new(Areas {
                                plane: sketch.plane,
                                regions: Arc::new(regions),
                                references,
                            }))
                        })
                        .map_err(|e| e.to_string());
                    let _ = sender.send(result);
                    context.request_repaint();
                })
                .map_err(|e| e.to_string())?;
            self.running = Some((id, receiver));
        }
        Ok(None)
    }
}

/// Altura de cada faixa do preenchimento por varredura, em pontos.
const BAND: f32 = 2.;

/// Preenche um contorno com furos por varredura horizontal (regra par-ímpar), que serve
/// para regiões côncavas sem triangulação.
pub(super) fn fill(painter: &egui::Painter, polygons: &[Vec<egui::Pos2>], color: egui::Color32) {
    let (top, bottom) = polygons
        .iter()
        .flatten()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), p| {
            (a.min(p.y), b.max(p.y))
        });
    if !top.is_finite() || bottom - top > 4000. {
        return;
    }
    let mut mesh = egui::Mesh::default();
    let mut y = top + BAND / 2.;
    let mut crossings = Vec::new();
    while y < bottom {
        crossings.clear();
        for polygon in polygons {
            for (a, b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
                if (a.y > y) != (b.y > y) {
                    crossings.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
        }
        crossings.sort_by(f32::total_cmp);
        for pair in crossings.as_chunks::<2>().0 {
            mesh.add_colored_rect(
                egui::Rect::from_min_max(
                    egui::pos2(pair[0], y - BAND / 2.),
                    egui::pos2(pair[1], y + BAND / 2.),
                ),
                color,
            );
        }
        y += BAND;
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Região de um rascunho candidata a perfil, já projetada na tela.
struct Candidate {
    sketch: SketchId,
    boundary: Vec<u64>,
    reference: nator_document::features::RegionRef,
    area: f64,
    outer: profiles::Wire,
    holes: Vec<profiles::Wire>,
    frame: Frame,
}
impl Candidate {
    fn contains(&self, point: Point2) -> bool {
        self.outer.contains(point) && !self.holes.iter().any(|h| h.contains(point))
    }
    fn polygons(&self) -> Option<Vec<Vec<egui::Pos2>>> {
        std::iter::once(&self.outer)
            .chain(&self.holes)
            .map(|wire| {
                wire.sample(48)
                    .into_iter()
                    .map(|p| self.frame.screen(p))
                    .collect::<Option<Vec<_>>>()
            })
            .collect()
    }
}

impl Editor {
    pub(super) fn profile_areas(
        &mut self,
        id: SketchId,
        context: &egui::Context,
    ) -> Option<Result<Option<Arc<Areas>>, String>> {
        let tab = &self.tabs[self.active];
        let sketch = &tab.session.document().sketch(id)?.sketch;
        Some(
            self.profile_cache
                .query((tab.id, tab.session.revision()), id, sketch, context),
        )
    }
    /// Desenha e trata a escolha de perfil; devolve verdadeiro se consumiu o clique.
    pub(super) fn profile_overlay(&mut self, ui: &egui::Ui, response: &egui::Response) -> bool {
        let Some(draft) = self
            .feature_edit
            .as_ref()
            .filter(|d| d.tab() == self.current().id && d.field() == Some(Field::Profile))
        else {
            return false;
        };
        let part_id = draft.part();
        let chosen = draft.sketch().zip(draft.region_reference().cloned());
        let current = draft.sketch();
        let editing = draft.output().is_some();
        let document = self.current().session.document();
        let Some(part) = document.part(part_id) else {
            return false;
        };
        let Ok(placement) = sketch_scene::part_placement(self.current(), part_id) else {
            return false;
        };
        let ids = part
            .sketches
            .iter()
            .filter(|s| part.shows_sketch(s) || (editing && Some(s.id) == current))
            .map(|s| s.id)
            .collect::<Vec<_>>();
        let mut candidates = Vec::new();
        let mut pending = false;
        for id in ids {
            let Some(result) = self.profile_areas(id, ui.ctx()) else {
                continue;
            };
            let areas = match result {
                Ok(Some(areas)) => areas,
                Ok(None) => {
                    pending = true;
                    continue;
                }
                Err(_) => continue,
            };
            for (region, reference) in areas.regions.iter().zip(&areas.references) {
                candidates.push(Candidate {
                    sketch: id,
                    boundary: region.boundary(),
                    reference: reference.clone(),
                    area: region.area(),
                    outer: region.outer.clone(),
                    holes: region.holes.clone(),
                    frame: Frame {
                        camera: self.viewport.camera.clone(),
                        bounds: self.display_bounds(),
                        rect: response.rect,
                        placement,
                        plane: areas.plane,
                    },
                });
            }
        }
        // A menor região sob o cursor: regiões dentro de furos ficam acessíveis.
        let hovered = response.hover_pos().and_then(|cursor| {
            candidates
                .iter()
                .enumerate()
                .filter(|(_, c)| c.frame.cursor(cursor).is_some_and(|p| c.contains(p)))
                .min_by(|a, b| a.1.area.total_cmp(&b.1.area))
                .map(|(i, _)| i)
        });
        let painter = ui
            .painter()
            .with_clip_rect(response.rect.intersect(ui.clip_rect()));
        for (index, candidate) in candidates.iter().enumerate() {
            let selected = chosen.as_ref().is_some_and(|(s, b)| {
                *s == candidate.sketch
                    && match b {
                        nator_document::features::RegionRef::Boundary(ids) => {
                            *ids == candidate.boundary
                        }
                        _ => *b == candidate.reference,
                    }
            });
            let Some(polygons) = candidate.polygons() else {
                continue;
            };
            let color = if selected {
                egui::Color32::from_rgba_unmultiplied(35, 112, 180, 95)
            } else if hovered == Some(index) {
                egui::Color32::from_rgba_unmultiplied(35, 112, 180, 55)
            } else {
                egui::Color32::from_rgba_unmultiplied(35, 112, 180, 14)
            };
            fill(&painter, &polygons, color);
            for polygon in &polygons {
                let mut closed = polygon.clone();
                closed.extend(polygon.first().copied());
                painter.add(egui::Shape::line(
                    closed,
                    egui::Stroke::new(
                        if selected || hovered == Some(index) {
                            2.
                        } else {
                            1.
                        },
                        theme::ACCENT,
                    ),
                ));
            }
        }
        if response.clicked()
            && let Some(candidate) = hovered.and_then(|i| candidates.get(i))
        {
            let (sketch, reference) = (candidate.sketch, candidate.reference.clone());
            if let Some(draft) = self.feature_edit.as_mut() {
                draft.choose_region(sketch, reference);
            }
            return true;
        }
        response.clicked()
            && (pending
                || !self
                    .feature_edit
                    .as_ref()
                    .is_some_and(|d| d.accepts_face_profile()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn area_cache_discards_old_revisions_and_reuses_the_current_result()
    -> Result<(), Box<dyn std::error::Error>> {
        let context = egui::Context::default();
        let mut cache = Cache::default();
        let mut old = nator_sketch::Sketch::new("Antigo", nator_sketch::Plane::xy());
        old.add_circle([0., 0.], 4., false)?;
        let mut current = old.clone();
        current.plane.origin = [0., 0., 15.];
        current.add_circle([20., 0.], 2., false)?;
        let id = SketchId(1);
        let _ = cache.query((1, 0), id, &old, &context)?;
        let deadline = Instant::now() + Duration::from_secs(3);
        let result = loop {
            if let Some(result) = cache.query((1, 1), id, &current, &context)? {
                break result;
            }
            if Instant::now() > deadline {
                return Err("Consulta de áreas não concluiu".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(result.regions.len(), 2);
        assert_eq!(result.plane, current.plane);
        let cached = cache
            .query((1, 1), id, &current, &context)?
            .ok_or("Cache ausente")?;
        assert!(Arc::ptr_eq(&result, &cached));
        Ok(())
    }
}
