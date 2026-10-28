//! Arranjos de curvas são calculados no worker e reaproveitados enquanto a geometria
//! for a mesma, inclusive em prévias conflitantes que não avançam a revisão do documento.
use super::*;
use nator_sketch::topology::{self, References};
type Answer = Result<(References, Vec<profiles::Region>), String>;

#[derive(Default)]
pub(super) struct Cache {
    key: Vec<u64>,
    pub points: References,
    pub projected_regions: Vec<profiles::Region>,
    running: Option<Receiver<Answer>>,
}

fn key(sketch: &Sketch) -> Vec<u64> {
    let mut result = vec![sketch.points.len() as u64, sketch.entities.len() as u64];
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
    pub fn prepare(&mut self, sketch: &Sketch, context: &egui::Context) -> Result<(), String> {
        let key = key(sketch);
        if self.key != key {
            self.key = key;
            self.points = References::default();
            self.projected_regions.clear();
            self.running = None;
            let sketch = sketch.clone();
            let context = context.clone();
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("sketch-references".into())
                .spawn(move || {
                    let result = topology::references(&sketch)
                        .map(|points| {
                            // Somente contornos projetados recebem lilás. O arranjo e
                            // os furos são resolvidos fora da thread da interface.
                            let mut projected = sketch.clone();
                            projected.entities.retain(|e| e.projected);
                            projected.constraints.clear();
                            projected.dimension_expressions.clear();
                            projected.dimension_labels.clear();
                            let regions = profiles::regions(&projected).unwrap_or_else(|_| {
                                nator_sketch::areas::regions(&projected).unwrap_or_default()
                            });
                            (points, regions)
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
                    (self.points, self.projected_regions) = result?;
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
