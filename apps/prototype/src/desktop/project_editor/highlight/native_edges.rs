//! Polilinhas de arestas sem nome, calculadas fora da interface. Uma consulta
//! agrupa todas as referências da seleção; respostas de outra geometria são descartadas.
use super::*;
type Curves = HashMap<usize, Vec<Point3>>;
type Key = (usize, Vec<(usize, u64)>);
#[derive(Default)]
pub(super) struct NativeEdges {
    key: Option<Key>,
    curves: Curves,
    running: Option<Receiver<Curves>>,
}
impl NativeEdges {
    pub fn prepare(
        &mut self,
        body: &Body,
        refs: impl Iterator<Item = (usize, u64)>,
        context: &egui::Context,
    ) {
        let Some(bytes) = body.brep.clone() else {
            return;
        };
        let mut indices = refs.collect::<Vec<_>>();
        indices.sort_unstable();
        indices.dedup();
        let key = (Arc::as_ptr(&bytes) as usize, indices.clone());
        if self.key.as_ref() != Some(&key) {
            self.key = Some(key);
            self.curves.clear();
            self.running = None;
            if !indices.is_empty() {
                let (sender, receiver) = mpsc::channel();
                let context = context.clone();
                let worker = std::thread::Builder::new()
                    .name("selected-edges".into())
                    .spawn(move || {
                        let mut curves = HashMap::new();
                        #[cfg(feature = "occt")]
                        if let Ok(solid) = nator_kernel::Solid::from_brep(&bytes, None) {
                            let signature = nator_document::features::signature(&bytes);
                            for (index, expected) in indices {
                                if expected == signature
                                    && let Ok(points) = solid.edge_polyline(index, 0.05)
                                {
                                    curves.insert(index, points);
                                }
                            }
                        }
                        #[cfg(not(feature = "occt"))]
                        let _ = (&bytes, &indices, &mut curves);
                        let _ = sender.send(curves);
                        context.request_repaint();
                    });
                if worker.is_ok() {
                    self.running = Some(receiver);
                }
            }
        }
        if let Some(receiver) = &self.running {
            match receiver.try_recv() {
                Ok(curves) => {
                    self.curves = curves;
                    self.running = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => self.running = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }
    pub fn overlay(&self, body: &Body, pose: Placement, index: usize, signature: u64) -> Overlay {
        let Some(bytes) = &body.brep else {
            return Overlay::default();
        };
        if !self.key.as_ref().is_some_and(|(key, indices)| {
            *key == Arc::as_ptr(bytes) as usize && indices.contains(&(index, signature))
        }) {
            return Overlay::default();
        }
        Overlay {
            segments: self
                .curves
                .get(&index)
                .map(Vec::as_slice)
                .unwrap_or_default()
                .windows(2)
                .map(|p| ([pose.apply(p[0]), pose.apply(p[1])], None))
                .collect(),
            ..Default::default()
        }
    }
}
