//! Arranjos de curvas são calculados no worker e reaproveitados enquanto a geometria
//! for a mesma, inclusive em prévias conflitantes que não avançam a revisão do documento.
use super::*;
use nator_sketch::constraint_state::{self, State};
use nator_sketch::repair::{self, Fix};
use nator_sketch::topology::{self, References};
struct Computed {
    points: References,
    regions: Vec<profiles::Region>,
    indicators: Vec<nator_sketch::indicators::Indicator>,
    state: State,
    unlinked: Vec<Fix>,
    status: Option<nator_sketch::solver::Status>,
}
type Answer = Result<Computed, String>;

#[derive(Default)]
pub(super) struct Cache {
    key: Vec<u64>,
    pub points: References,
    pub projected_regions: Vec<profiles::Region>,
    pub indicators: Vec<nator_sketch::indicators::Indicator>,
    /// Estado de restrição do rascunho. Fica até o novo chegar, para os elementos não
    /// piscarem como livres a cada edição.
    pub state: State,
    /// Pontas no mesmo lugar sem ligação, com o reparo que as liga.
    pub unlinked: Vec<Fix>,
    /// Graus de liberdade do rascunho como está, já ao abrir, antes de qualquer edição.
    pub status: Option<nator_sketch::solver::Status>,
    running: Option<Receiver<Answer>>,
}

fn key(sketch: &Sketch) -> Vec<u64> {
    let mut result = vec![
        sketch.points.len() as u64,
        sketch.entities.len() as u64,
        sketch.constraints.len() as u64,
    ];
    // Abrir/fechar não precisa mover pontos para alterar o indicador.
    for constraint in &sketch.constraints {
        result.extend([constraint.id, u64::from(constraint.reference)]);
    }
    for point in &sketch.points {
        result.extend([
            point.id,
            point.position[0].to_bits(),
            point.position[1].to_bits(),
        ]);
    }
    for entity in &sketch.entities {
        result.extend([
            entity.id,
            u64::from(entity.construction),
            u64::from(entity.infinite),
            u64::from(entity.projected),
        ]);
        match entity.curve {
            Curve::Line { start, end } => result.extend([0, start, end]),
            Curve::Circle { center, radius } => result.extend([1, center, radius.to_bits()]),
            Curve::Arc {
                center,
                start,
                end,
                clockwise,
            } => result.extend([2, center, start, end, u64::from(clockwise)]),
        }
    }
    result
}

impl Cache {
    #[cfg(all(test, feature = "occt"))]
    pub fn pending(&self) -> bool {
        self.running.is_some()
    }

    pub fn prepare(&mut self, sketch: &Sketch, context: &egui::Context) -> Result<(), String> {
        let key = key(sketch);
        if self.key != key {
            self.key = key;
            self.points = References::default();
            self.projected_regions.clear();
            self.indicators.clear();
            self.running = None;
            let sketch = sketch.clone();
            let context = context.clone();
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("sketch-references".into())
                .spawn(move || {
                    let result = topology::references(&sketch)
                        .and_then(|points| {
                            // Geometria degenerada não impede o desenho nem os apoios.
                            let state = constraint_state::analyze(&sketch).unwrap_or_default();
                            let unlinked = repair::suggest(&sketch, 0.)
                                .map(|report| {
                                    report
                                        .fixes
                                        .into_iter()
                                        .filter(|fix| matches!(fix, Fix::LinkEnds { .. }))
                                        .collect()
                                })
                                .unwrap_or_default();
                            let mut indicators = nator_sketch::indicators::markers(&sketch)?;
                            for center in &points.centers {
                                let hidden_circle = sketch.entities.iter().any(|e| {
                                    matches!(e.curve, Curve::Circle { .. })
                                        && sketch.radial(e.id).is_ok_and(|(c, _)| {
                                            nator_sketch::distance(c, *center) < 1e-6
                                        })
                                        && nator_sketch::connected::center_is_linked(&sketch, e.id)
                                            .is_ok_and(|linked| !linked)
                                });
                                if hidden_circle {
                                    continue;
                                }
                                if !indicators.iter().any(|i| {
                                    i.kind == nator_sketch::indicators::Kind::Center
                                        && nator_sketch::distance(i.point, *center) < 1e-6
                                }) {
                                    indicators.push(nator_sketch::indicators::Indicator {
                                        point: *center,
                                        kind: nator_sketch::indicators::Kind::Center,
                                    });
                                }
                            }
                            // Somente contornos projetados recebem lilás. O arranjo e
                            // os furos são resolvidos fora da thread da interface.
                            let mut projected = sketch.clone();
                            projected.entities.retain(|e| e.projected);
                            projected.constraints.clear();
                            projected.dimension_expressions.clear();
                            projected.dimension_labels.clear();
                            projected.dimension_targets.clear();
                            let regions = profiles::regions(&projected).unwrap_or_else(|_| {
                                nator_sketch::areas::regions(&projected).unwrap_or_default()
                            });
                            let status = nator_sketch::solver::solve(&sketch)
                                .ok()
                                .map(|report| report.status);
                            Ok(Computed {
                                points,
                                regions,
                                indicators,
                                state,
                                unlinked,
                                status,
                            })
                        })
                        .map_err(|e| e.to_string());
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
                    let computed = result?;
                    self.points = computed.points;
                    self.projected_regions = computed.regions;
                    self.indicators = computed.indicators;
                    self.state = computed.state;
                    self.unlinked = computed.unlinked;
                    self.status = computed.status;
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

#[cfg(test)]
mod tests {
    use super::*;
    use nator_sketch::constraint_state::Level;

    #[test]
    fn changing_only_constraints_updates_the_marks() -> Result<(), Box<dyn std::error::Error>> {
        let mut sketch = Sketch::new("Indicadores", nator_sketch::Plane::xy());
        sketch.add_line([-10., 0.], [0., 0.], false)?;
        sketch.add_line([0., 0.], [10., 0.], false)?;
        let context = egui::Context::default();
        let mut cache = Cache::default();
        let finish =
            |cache: &mut Cache, sketch: &Sketch| -> Result<(), Box<dyn std::error::Error>> {
                cache.prepare(sketch, &context)?;
                let deadline = Instant::now() + Duration::from_secs(5);
                while cache.running.is_some() {
                    if Instant::now() > deadline {
                        return Err("Indicadores não terminaram".into());
                    }
                    std::thread::sleep(Duration::from_millis(1));
                    cache.prepare(sketch, &context)?;
                }
                Ok(())
            };
        finish(&mut cache, &sketch)?;
        assert_eq!(cache.unlinked.len(), 1);
        let [a, _] = sketch.entities[0].point_ids()[..] else {
            return Err("Linha sem pontas".into());
        };
        assert_eq!(cache.state.level(a), Level::Free);
        // Mesmos pontos e entidades; só as restrições mudam.
        nator_sketch::junctions::link(&mut sketch, [0., 0.])?;
        let line = sketch.entities[0].id;
        sketch.add_constraint(nator_sketch::ConstraintKind::Horizontal { line }, false)?;
        cache.prepare(&sketch, &context)?;
        // Enquanto o trabalhador calcula, o estado anterior continua à mostra.
        assert_eq!(cache.state.level(a), Level::Free);
        finish(&mut cache, &sketch)?;
        assert_eq!(cache.state.level(a), Level::Partial);
        assert!(cache.unlinked.is_empty());
        Ok(())
    }

    #[test]
    fn geometry_changes_discard_old_references_without_needing_a_new_document_revision()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut sketch = Sketch::new("Prévia", nator_sketch::Plane::xy());
        sketch.add_rectangle([0., 0.], [20., 10.], false)?;
        let context = egui::Context::default();
        let mut cache = Cache::default();
        cache.prepare(&sketch, &context)?;
        // Mesmo conjunto de IDs, mas outra posição: uma prévia conflitante não
        // avança a revisão e ainda assim precisa invalidar os apoios anteriores.
        for point in &mut sketch.points {
            point.position = nator_sketch::add(point.position, [1000., 1000.]);
        }
        cache.prepare(&sketch, &context)?;
        assert!(!cache.points.centers.contains(&[10., 5.]));
        let deadline = Instant::now() + Duration::from_secs(5);
        while cache.running.is_some() {
            if Instant::now() > deadline {
                return Err("Consulta não terminou".into());
            }
            std::thread::sleep(Duration::from_millis(1));
            cache.prepare(&sketch, &context)?;
        }
        assert_eq!(cache.points.centers, vec![[1010., 1005.]]);
        Ok(())
    }
}
