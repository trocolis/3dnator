//! Controles das referências e da posição das cotas. Capturar geometria só emite uma
//! solicitação para o worker; o arraste na folha manipula dados de apresentação já resolvidos.
use super::*;
use nator_document::drawing::{DrawingAnnotation, DrawingMeasure, DrawingTopologyRef};
use nator_drawing::{AnnotationId, View};
const KINDS: [T; 7] = [
    T::DimensionLength,
    T::DimensionRadiusFace,
    T::DimensionDiameterFace,
    T::DimensionRadiusEdge,
    T::DimensionDiameterEdge,
    T::DimensionFaceDistance,
    T::DimensionFaceAngle,
];
fn view_controls(ui: &mut egui::Ui, view: &mut View) {
    egui::ComboBox::from_label(text(T::DimensionView))
        .selected_text(view.label())
        .show_ui(ui, |ui| {
            for v in [View::Front, View::Top, View::Right, View::Isometric] {
                ui.selectable_value(view, v, v.label());
            }
        });
}
impl Draft {
    fn measurement(&self) -> Option<DrawingMeasure> {
        let a = self.first_reference.clone()?;
        match (self.annotation_kind, a) {
            (0, DrawingTopologyRef::Edge(edge)) => Some(DrawingMeasure::EdgeLength(edge)),
            (1 | 2, topology @ DrawingTopologyRef::Face(_))
            | (3 | 4, topology @ DrawingTopologyRef::Edge(_)) => Some(DrawingMeasure::Radius {
                topology,
                diameter: self.annotation_kind == 2 || self.annotation_kind == 4,
            }),
            (5, DrawingTopologyRef::Face(first)) => {
                let DrawingTopologyRef::Face(second) = self.second_reference.clone()? else {
                    return None;
                };
                Some(DrawingMeasure::FaceDistance { first, second })
            }
            (6, DrawingTopologyRef::Face(first)) => {
                let DrawingTopologyRef::Face(second) = self.second_reference.clone()? else {
                    return None;
                };
                Some(DrawingMeasure::FaceAngle {
                    first,
                    second,
                    supplementary: self.supplementary,
                })
            }
            _ => None,
        }
    }
    fn select_annotation(&mut self, id: AnnotationId) {
        let Some(a) = self.annotations.iter().find(|a| a.id == id) else {
            return;
        };
        self.selected_annotation = Some(id);
        self.annotation_view = a.view;
        match &a.measurement {
            DrawingMeasure::EdgeLength(edge) => {
                self.annotation_kind = 0;
                self.first_reference = Some(DrawingTopologyRef::Edge(edge.clone()));
                self.second_reference = None;
            }
            DrawingMeasure::Radius { topology, diameter } => {
                self.annotation_kind = match (topology, diameter) {
                    (DrawingTopologyRef::Face(_), false) => 1,
                    (DrawingTopologyRef::Face(_), true) => 2,
                    (DrawingTopologyRef::Edge(_), false) => 3,
                    _ => 4,
                };
                self.first_reference = Some(topology.clone());
                self.second_reference = None;
            }
            DrawingMeasure::FaceDistance { first, second } => {
                self.annotation_kind = 5;
                self.first_reference = Some(DrawingTopologyRef::Face(first.clone()));
                self.second_reference = Some(DrawingTopologyRef::Face(second.clone()));
            }
            DrawingMeasure::FaceAngle {
                first,
                second,
                supplementary,
            } => {
                self.annotation_kind = 6;
                self.supplementary = *supplementary;
                self.first_reference = Some(DrawingTopologyRef::Face(first.clone()));
                self.second_reference = Some(DrawingTopologyRef::Face(second.clone()));
            }
        }
    }
    pub(super) fn annotation_controls(
        &mut self,
        ui: &mut egui::Ui,
        document: &Document,
        pick_available: bool,
    ) -> Option<(bool, bool)> {
        let mut capture = None;
        ui.collapsing(text(T::AssociativeDimensions), |ui| {
            let previous = self.annotation_kind;
            egui::ComboBox::from_label(text(T::DimensionKind))
                .selected_text(text(KINDS[self.annotation_kind]))
                .show_ui(ui, |ui| {
                    for (index, kind) in KINDS.iter().enumerate() {
                        ui.selectable_value(&mut self.annotation_kind, index, text(*kind));
                    }
                });
            if self.annotation_kind != previous {
                self.first_reference = None;
                self.second_reference = None;
            }
            view_controls(ui, &mut self.annotation_view);
            if self.annotation_kind == 6 {
                ui.checkbox(&mut self.supplementary, text(T::SupplementaryAngle));
            }
            let edge = matches!(self.annotation_kind, 0 | 3 | 4);
            if ui
                .add_enabled(
                    pick_available,
                    egui::Button::new(text(T::CaptureFirstReference)),
                )
                .clicked()
            {
                capture = Some((true, edge));
            }
            if let Some(r) = &self.first_reference {
                reference_label(ui, document, r);
            }
            if self.annotation_kind >= 5 {
                if ui
                    .add_enabled(
                        pick_available,
                        egui::Button::new(text(T::CaptureSecondReference)),
                    )
                    .clicked()
                {
                    capture = Some((false, false));
                }
                if let Some(r) = &self.second_reference {
                    reference_label(ui, document, r);
                }
            }
            let measurement = self.measurement();
            ui.horizontal_wrapped(|ui| {
                let id = self
                    .annotations
                    .iter()
                    .map(|a| a.id.0)
                    .max()
                    .unwrap_or(0)
                    .checked_add(1);
                if ui
                    .add_enabled(
                        measurement.is_some() && self.annotations.len() < 256 && id.is_some(),
                        egui::Button::new(text(T::AddDimension)),
                    )
                    .clicked()
                    && let (Some(measurement), Some(id)) = (measurement.clone(), id)
                {
                    let id = AnnotationId(id);
                    self.annotations.push(DrawingAnnotation {
                        id,
                        view: self.annotation_view,
                        measurement,
                        offset_mm: [10., -10.],
                    });
                    self.selected_annotation = Some(id);
                }
                if ui
                    .add_enabled(
                        measurement.is_some() && self.selected_annotation.is_some(),
                        egui::Button::new(text(T::UpdateDimensionReferences)),
                    )
                    .clicked()
                    && let (Some(id), Some(measurement)) =
                        (self.selected_annotation, measurement.clone())
                    && let Some(a) = self.annotations.iter_mut().find(|a| a.id == id)
                {
                    a.measurement = measurement;
                    a.view = self.annotation_view;
                }
            });
            let mut selected = None;
            for a in &self.annotations {
                if ui
                    .selectable_label(
                        self.selected_annotation == Some(a.id),
                        format!("#{} · {}", a.id.0, a.view.label()),
                    )
                    .clicked()
                {
                    selected = Some(a.id);
                }
            }
            if let Some(id) = selected {
                self.select_annotation(id);
            }
            if let Some(id) = self.selected_annotation {
                if let Some(a) = self.annotations.iter_mut().find(|a| a.id == id) {
                    ui.label(text(T::DimensionTextPosition));
                    ui.horizontal(|ui| {
                        for v in &mut a.offset_mm {
                            ui.add(egui::DragValue::new(v).speed(0.1).range(-1000..=1000));
                        }
                    });
                }
                if ui.button(text(T::RemoveDimension)).clicked() {
                    self.annotations.retain(|a| a.id != id);
                    self.selected_annotation = None;
                }
            }
        });
        capture
    }
    pub(super) fn preview_interactions(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        scale: f32,
        revision: u64,
        busy: bool,
    ) -> Result<(), String> {
        let Some(sheet) = self.preview.as_mut() else {
            return Ok(());
        };
        let mut moved = None;
        for handle in &sheet.annotation_handles {
            let at = rect.min + egui::vec2(handle.at_mm[0] as f32, handle.at_mm[1] as f32) * scale;
            let size = egui::vec2(
                (handle.label.chars().count() as f32 * handle.size_mm as f32 * 0.7 + 2.) * scale,
                handle.size_mm as f32 * 2. * scale,
            );
            let bounds =
                egui::Rect::from_min_size(at - egui::vec2(0., size.y * 0.7), size).intersect(rect);
            let response = ui.interact(
                bounds,
                egui::Id::new((self.tab, "drawing_annotation", handle.id.0)),
                egui::Sense::drag(),
            );
            if self.selected_annotation == Some(handle.id) {
                ui.painter().rect_stroke(
                    bounds,
                    0.,
                    egui::Stroke::new(1., egui::Color32::LIGHT_BLUE),
                    egui::StrokeKind::Inside,
                );
            }
            if response.dragged() && !busy && self.preview_revision == Some(revision) {
                let delta = ui.input(|i| i.pointer.delta());
                moved = Some((
                    handle.id,
                    [
                        handle.offset_mm[0] + f64::from(delta.x / scale),
                        handle.offset_mm[1] + f64::from(delta.y / scale),
                    ],
                ));
            }
        }
        if let Some((id, offset)) = moved {
            sheet
                .reposition_annotation(id, offset)
                .map_err(|e| e.to_string())?;
            if let Some(a) = self.annotations.iter_mut().find(|a| a.id == id) {
                a.offset_mm = offset;
                self.selected_annotation = Some(id);
            }
            ui.ctx().request_repaint();
        }
        Ok(())
    }
}
fn reference_label(ui: &mut egui::Ui, document: &Document, r: &DrawingTopologyRef) {
    let (target, kind) = match r {
        DrawingTopologyRef::Face(r) => (&r.target, T::FaceLabel),
        DrawingTopologyRef::Edge(r) => (&r.target, T::EdgeLabel),
    };
    ui.small(format!(
        "{} · {}",
        document
            .body(target.body)
            .map(|b| b.name.as_str())
            .unwrap_or(text(T::DrawingReferenceMissing)),
        text(kind)
    ));
}
