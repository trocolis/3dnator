//! Projeções do modelo são preparadas fora da interface, sem limites arbitrários de
//! ocorrências/vértices. A chave acompanha documento, visibilidade e referencial editado.
use super::*;
use nator_commands::{point_snap::PointKind, sketch_projection::SnapGeometry};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Key {
    pub scene: SceneKey,
    pub sketch: SketchId,
    pub plane: nator_sketch::Plane,
    pub placement: Placement,
}

pub(super) struct Cache {
    key: Option<Key>,
    pub points: Vec<Point2>,
    /// O que cada apoio representa, para o ícone junto ao cursor.
    pub marks: Vec<(Point2, PointKind)>,
    pub curves: Vec<nator_sketch::profiles::Segment>,
    pub chains: Vec<Vec<Point2>>,
    running: Option<Receiver<Result<SnapGeometry, String>>>,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            key: None,
            points: vec![[0., 0.]],
            marks: Vec::new(),
            curves: Vec::new(),
            chains: Vec::new(),
            running: None,
        }
    }
}
impl Cache {
    pub fn prepare(
        &mut self,
        key: Key,
        document: &Document,
        view: &ViewState,
        context: &egui::Context,
    ) -> Result<(), String> {
        if self.key != Some(key) {
            // Descartar a entrega anterior antes de consultar outra revisão ou plano.
            self.key = Some(key);
            self.points = vec![[0., 0.]];
            self.marks.clear();
            self.curves.clear();
            self.chains.clear();
            self.running = None;
            let occurrences = document
                .occurrences()
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|o| view.pickable(o))
                .collect::<Vec<_>>();
            let document = document.clone();
            let context = context.clone();
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("sketch-projection".into())
                .spawn(move || {
                    let result = nator_commands::sketch_projection::snap_geometry(
                        &document,
                        &occurrences,
                        key.sketch,
                        key.plane,
                        key.placement,
                    )
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
                    let geometry = result?;
                    self.points = geometry.points.iter().map(|(p, _)| *p).collect();
                    self.marks = geometry.points;
                    self.curves = geometry.curves;
                    self.chains = geometry.chains;
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
    /// Ícone do tipo de captura, mostrado só enquanto o cursor está sobre o apoio.
    pub fn snap_icon(&self, kind: Kind, point: Point2) -> Option<icons::Icon> {
        use icons::Icon;
        Some(match kind {
            Kind::Endpoint(_) | Kind::Intersection => Icon::ConstraintCoincident,
            Kind::Midpoint(_) | Kind::CurveMidpoint(_) | Kind::ProjectedMidpoint => {
                Icon::SnapMidpoint
            }
            Kind::Center(_) | Kind::RegionCenter => Icon::SnapCenter,
            Kind::Tangent => Icon::ConstraintTangent,
            Kind::OnCurve(_) | Kind::ProjectedEdge => Icon::ConstraintCollinear,
            Kind::Projected => match self
                .marks
                .iter()
                .find(|(p, _)| nator_sketch::distance(*p, point) < 1e-7)
                .map(|(_, mark)| *mark)
            {
                Some(PointKind::EdgeMidpoint) => Icon::SnapMidpoint,
                Some(PointKind::CircleCenter | PointKind::ArcCenter | PointKind::FaceCenter) => {
                    Icon::SnapCenter
                }
                _ => Icon::ConstraintCoincident,
            },
            Kind::Alignment | Kind::Step => return None,
        })
    }
    #[cfg(all(test, feature = "occt"))]
    pub fn ready(&self) -> bool {
        self.key.is_some() && self.running.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_icon_tells_what_was_captured() {
        use icons::Icon;
        let cache = Cache {
            marks: vec![
                ([1., 0.], PointKind::EdgeMidpoint),
                ([2., 0.], PointKind::CircleCenter),
                ([3., 0.], PointKind::Vertex),
            ],
            ..Default::default()
        };
        assert!(cache.snap_icon(Kind::Midpoint(1), [0., 0.]) == Some(Icon::SnapMidpoint));
        assert!(cache.snap_icon(Kind::Center(1), [0., 0.]) == Some(Icon::SnapCenter));
        assert!(cache.snap_icon(Kind::Tangent, [0., 0.]) == Some(Icon::ConstraintTangent));
        assert!(cache.snap_icon(Kind::Projected, [1., 0.]) == Some(Icon::SnapMidpoint));
        assert!(cache.snap_icon(Kind::Projected, [2., 0.]) == Some(Icon::SnapCenter));
        assert!(cache.snap_icon(Kind::Projected, [3., 0.]) == Some(Icon::ConstraintCoincident));
        assert!(cache.snap_icon(Kind::Step, [0., 0.]).is_none());
    }

    #[test]
    fn plane_and_view_changes_discard_queued_old_projections()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut document = Document::new("Projeções em cache")?;
        let (part, _) = document.add_mesh(
            None,
            "Retângulo".into(),
            Arc::new(MeshAsset::new(Mesh::new(
                vec![[0., 0., 7.], [20., 0., 7.], [20., 10., 7.], [0., 10., 7.]],
                vec![[0, 1, 2], [0, 2, 3]],
            )?)),
        )?;
        let (_, id) =
            document.add_sketch(Some(part), Sketch::new("Plano", nator_sketch::Plane::xy()))?;
        let old = Key {
            scene: SceneKey {
                tab: 1,
                revision: 0,
                view: 0,
                active: Some(part),
                preview: 0,
            },
            sketch: id,
            plane: nator_sketch::Plane::xy(),
            placement: Placement::default(),
        };
        let (sender, receiver) = mpsc::channel();
        sender.send(Ok(SnapGeometry {
            points: vec![([999., 999.], PointKind::Vertex)],
            ..Default::default()
        }))?;
        let mut cache = Cache {
            key: Some(old),
            points: vec![[10., 5.]],
            running: Some(receiver),
            ..Default::default()
        };
        let mut key = old;
        key.plane.origin = [100., 0., 40.];
        let context = egui::Context::default();
        let mut view = ViewState::default();
        cache.prepare(key, &document, &view, &context)?;
        assert!(!cache.points.contains(&[10., 5.]));
        let finish =
            |cache: &mut Cache, key, view: &ViewState| -> Result<(), Box<dyn std::error::Error>> {
                let deadline = Instant::now() + Duration::from_secs(5);
                while cache.running.is_some() {
                    if Instant::now() > deadline {
                        return Err("Consulta não terminou".into());
                    }
                    std::thread::sleep(Duration::from_millis(1));
                    cache.prepare(key, &document, view, &context)?;
                }
                Ok(())
            };
        finish(&mut cache, key, &view)?;
        assert!(cache.points.contains(&[-90., 5.]));
        assert!(!cache.points.contains(&[999., 999.]));
        view.locked.insert(document.instances()[0].id);
        view.touched();
        key.scene.view = view.version;
        cache.prepare(key, &document, &view, &context)?;
        finish(&mut cache, key, &view)?;
        assert_eq!(cache.points, vec![[0., 0.]]);
        Ok(())
    }
}
