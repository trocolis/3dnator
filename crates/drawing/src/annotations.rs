//! Apresentação de cotas já resolvidas. A associação ao documento e a medição precisa
//! ficam nas camadas superiores; mover texto na folha só refaz os traços e as setas.
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnnotationId(pub u32);
#[derive(Debug, Clone)]
pub enum AnnotationGeometry {
    Linear {
        a: Point3,
        b: Point3,
        value_mm: f64,
    },
    Radius {
        center: Point3,
        point: Point3,
        radius_mm: f64,
        diameter: bool,
    },
    Angle {
        center: Point3,
        first: Point3,
        second: Point3,
        angle_rad: f64,
    },
    Leader {
        at: Point3,
        label: String,
    },
    Missing {
        message: String,
    },
}
#[derive(Debug, Clone)]
pub struct ResolvedAnnotation {
    pub id: AnnotationId,
    pub view: View,
    pub offset_mm: [f64; 2],
    pub geometry: AnnotationGeometry,
}
#[derive(Debug, Clone)]
pub struct AnnotationHandle {
    pub id: AnnotationId,
    pub at_mm: [f64; 2],
    pub offset_mm: [f64; 2],
    pub label: String,
    pub size_mm: f64,
}
impl AnnotationGeometry {
    fn valid(&self) -> bool {
        // Limita também magnitudes finitas: uma anotação não deve produzir infinito
        // ao projetar ou somar apoios durante a exportação vetorial.
        let point = |p: &Point3| p.iter().all(|v| v.is_finite() && v.abs() <= 1e12);
        let text = |s: &str| s.len() <= 1024 && !s.chars().any(char::is_control);
        match self {
            Self::Linear { a, b, value_mm } => {
                point(a) && point(b) && value_mm.is_finite() && *value_mm >= 0. && *value_mm <= 1e12
            }
            Self::Radius {
                center,
                point: radial,
                radius_mm,
                ..
            } => {
                point(center)
                    && point(radial)
                    && radius_mm.is_finite()
                    && *radius_mm > 0.
                    && *radius_mm <= 1e12
            }
            Self::Angle {
                center,
                first,
                second,
                angle_rad,
            } => {
                point(center)
                    && point(first)
                    && point(second)
                    && nator_geometry::norm(*first) > 1e-12
                    && nator_geometry::norm(*second) > 1e-12
                    && angle_rad.is_finite()
                    && (0. ..=std::f64::consts::PI).contains(angle_rad)
            }
            Self::Leader { at, label } => point(at) && text(label),
            Self::Missing { message } => text(message),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub(super) struct ViewPlacement {
    pub view: View,
    pub origin: [f64; 2],
    pub center: [f64; 2],
    pub scale: f64,
}
impl ViewPlacement {
    fn point(self, p: Point3) -> [f64; 2] {
        let (u, v, _) = self.view.basis();
        [
            self.origin[0] + (dot(p, u) - self.center[0]) * self.scale,
            self.origin[1] - (dot(p, v) - self.center[1]) * self.scale,
        ]
    }
}
fn arrow(sheet: &mut Sheet, tip: [f64; 2], toward: [f64; 2]) {
    let d = [toward[0] - tip[0], toward[1] - tip[1]];
    let length = d[0].hypot(d[1]);
    if length < 1e-9 {
        return;
    }
    let u = d.map(|v| v / length);
    for sign in [-1., 1.] {
        sheet.line(
            tip,
            [
                tip[0] + u[0] * 2. - sign * u[1] * 0.6,
                tip[1] + u[1] * 2. + sign * u[0] * 0.6,
            ],
            0.18,
            false,
        );
    }
}
impl Sheet {
    pub fn set_annotations(&mut self, annotations: Vec<ResolvedAnnotation>) -> Result<()> {
        if annotations.len() > 256 {
            return Err(DrawingError(
                "A folha aceita até 256 cotas associativas.".into(),
            ));
        }
        for (index, a) in annotations.iter().enumerate() {
            if a.id.0 == 0
                || annotations[..index].iter().any(|b| b.id == a.id)
                || a.offset_mm
                    .iter()
                    .any(|v| !v.is_finite() || v.abs() > 1000.)
                || !a.geometry.valid()
            {
                return Err(DrawingError(
                    "Identificador, posição ou geometria de cota inválidos.".into(),
                ));
            }
        }
        self.annotations = annotations;
        self.rebuild_annotations();
        Ok(())
    }
    /// Deslocamento em mm de papel. Reutiliza a geometria resolvida na revisão da prévia:
    /// o arraste não chama o núcleo nem altera o modelo ou o histórico do documento.
    pub fn reposition_annotation(&mut self, id: AnnotationId, offset_mm: [f64; 2]) -> Result<()> {
        if offset_mm.iter().any(|v| !v.is_finite() || v.abs() > 1000.) {
            return Err(DrawingError("Posição de cota inválida.".into()));
        }
        let a = self
            .annotations
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| DrawingError("Cota inexistente na prévia.".into()))?;
        a.offset_mm = offset_mm;
        self.rebuild_annotations();
        Ok(())
    }
    fn rebuild_annotations(&mut self) {
        self.primitives.truncate(self.annotation_base);
        self.annotation_handles.clear();
        self.annotation_warnings.clear();
        for annotation in self.annotations.clone() {
            let placement = self
                .placements
                .iter()
                .find(|p| p.view == annotation.view)
                .copied();
            let offset = annotation.offset_mm;
            let (at, label) = if let Some(p) = placement {
                match &annotation.geometry {
                    AnnotationGeometry::Linear { a, b, value_mm } => {
                        let (a, b) = (p.point(*a), p.point(*b));
                        let label = format_mm(*value_mm);
                        if (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-6 {
                            // Uma medida perpendicular à vista não tem comprimento
                            // projetado. Exibimos seu valor real com uma chamada.
                            let at = self.annotation_leader(a, offset, &label);
                            self.annotation_handles.push(AnnotationHandle {
                                id: annotation.id,
                                at_mm: at,
                                offset_mm: offset,
                                label,
                                size_mm: 2.5,
                            });
                            continue;
                        }
                        self.dimension(a, b, offset, label.clone());
                        (
                            [
                                (a[0] + b[0]) * 0.5 + offset[0] + 1.,
                                (a[1] + b[1]) * 0.5 + offset[1] - 1.,
                            ],
                            label,
                        )
                    }
                    AnnotationGeometry::Radius {
                        center,
                        point,
                        radius_mm,
                        diameter,
                    } => {
                        let (center, point) = (p.point(*center), p.point(*point));
                        let start = if *diameter {
                            [2. * center[0] - point[0], 2. * center[1] - point[1]]
                        } else {
                            center
                        };
                        let at = [point[0] + offset[0], point[1] + offset[1]];
                        self.line(start, point, 0.18, false);
                        arrow(self, point, start);
                        if *diameter {
                            arrow(self, start, point);
                        }
                        self.line(point, at, 0.15, false);
                        let label = format!(
                            "{} {}",
                            if *diameter { "Ø" } else { "R" },
                            format_mm(radius_mm * if *diameter { 2. } else { 1. })
                        );
                        self.text(at, 2.5, label.clone());
                        (at, label)
                    }
                    AnnotationGeometry::Angle {
                        center,
                        first,
                        second,
                        angle_rad,
                    } => {
                        let c = p.point(*center);
                        let a = p.point(std::array::from_fn(|i| center[i] + first[i]));
                        let b = p.point(std::array::from_fn(|i| center[i] + second[i]));
                        let label = format!("{}°", format_mm(angle_rad.to_degrees()));
                        let (_, _, direction) = p.view.basis();
                        let normal = nator_geometry::cross(*first, *second);
                        // O arco circular só representa o ângulo verdadeiro quando
                        // o plano da cota é paralelo ao papel. Nas outras vistas a
                        // chamada conserva o valor exato sem desenhar um arco falso.
                        let parallel = dot(normal, direction).abs()
                            >= nator_geometry::norm(normal) * (1. - 1e-8);
                        if !parallel
                            || (a[0] - c[0]).hypot(a[1] - c[1]) < 1e-6
                            || (b[0] - c[0]).hypot(b[1] - c[1]) < 1e-6
                        {
                            let at = self.annotation_leader(c, offset, &label);
                            self.annotation_handles.push(AnnotationHandle {
                                id: annotation.id,
                                at_mm: at,
                                offset_mm: offset,
                                label,
                                size_mm: 2.5,
                            });
                            continue;
                        }
                        let start = (a[1] - c[1]).atan2(a[0] - c[0]);
                        let end = (b[1] - c[1]).atan2(b[0] - c[0]);
                        let sweep = (end - start + std::f64::consts::PI)
                            .rem_euclid(std::f64::consts::TAU)
                            - std::f64::consts::PI;
                        let radius = (offset[0].hypot(offset[1]) * 0.4).clamp(4., 20.);
                        let point = |t: f64| {
                            [
                                c[0] + radius * (start + sweep * t).cos(),
                                c[1] + radius * (start + sweep * t).sin(),
                            ]
                        };
                        self.line(c, point(0.), 0.15, false);
                        self.line(c, point(1.), 0.15, false);
                        for i in 0..32 {
                            self.line(
                                point(i as f64 / 32.),
                                point((i + 1) as f64 / 32.),
                                0.18,
                                false,
                            );
                        }
                        arrow(self, point(0.), point(0.1));
                        arrow(self, point(1.), point(0.9));
                        let at = [c[0] + offset[0], c[1] + offset[1]];
                        self.text(at, 2.5, label.clone());
                        (at, label)
                    }
                    AnnotationGeometry::Leader { at, label } => {
                        let anchor = p.point(*at);
                        let at = self.annotation_leader(anchor, offset, label);
                        (at, label.clone())
                    }
                    AnnotationGeometry::Missing { message } => {
                        self.annotation_warnings
                            .push((annotation.id, message.clone()));
                        let at = [p.origin[0] + offset[0], p.origin[1] + offset[1]];
                        let label = format!("REVISAR COTA {}", annotation.id.0);
                        self.text(at, 2.5, label.clone());
                        (at, label)
                    }
                }
            } else {
                self.annotation_warnings.push((
                    annotation.id,
                    "A vista da cota não está presente na folha.".into(),
                ));
                let at = [12. + offset[0], 16. + offset[1]];
                let label = format!("REVISAR COTA {}", annotation.id.0);
                self.text(at, 2.5, label.clone());
                (at, label)
            };
            self.annotation_handles.push(AnnotationHandle {
                id: annotation.id,
                at_mm: at,
                offset_mm: offset,
                label,
                size_mm: 2.5,
            });
        }
    }
    fn annotation_leader(&mut self, anchor: [f64; 2], offset: [f64; 2], label: &str) -> [f64; 2] {
        let at = [anchor[0] + offset[0], anchor[1] + offset[1]];
        self.line(anchor, at, 0.15, false);
        arrow(self, anchor, at);
        self.text(at, 2.5, label.to_string());
        at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotations_present_handles_labels_and_repositioning() -> Result<()> {
        let mesh = nator_geometry::reference_bracket().map_err(|e| DrawingError(e.to_string()))?;
        let spec = Specification::default();
        let mut sheet = generate(&mesh, &spec)?;

        let annotations = vec![
            ResolvedAnnotation {
                id: AnnotationId(1),
                view: View::Front,
                offset_mm: [10., -10.],
                geometry: AnnotationGeometry::Linear {
                    a: [0., 0., 0.],
                    b: [60., 0., 0.],
                    value_mm: 60.,
                },
            },
            ResolvedAnnotation {
                id: AnnotationId(2),
                view: View::Front,
                offset_mm: [5., 5.],
                geometry: AnnotationGeometry::Radius {
                    center: [0., 0., 0.],
                    point: [5., 0., 0.],
                    radius_mm: 5.,
                    diameter: false,
                },
            },
            ResolvedAnnotation {
                id: AnnotationId(3),
                view: View::Front,
                offset_mm: [8., 8.],
                geometry: AnnotationGeometry::Radius {
                    center: [0., 0., 0.],
                    point: [5., 0., 0.],
                    radius_mm: 5.,
                    diameter: true,
                },
            },
            ResolvedAnnotation {
                id: AnnotationId(4),
                view: View::Front,
                offset_mm: [12., -12.],
                geometry: AnnotationGeometry::Leader {
                    at: [10., 10., 0.],
                    label: "Nota técnica".into(),
                },
            },
            ResolvedAnnotation {
                id: AnnotationId(5),
                view: View::Front,
                offset_mm: [0., 0.],
                geometry: AnnotationGeometry::Missing {
                    message: "Aresta removida".into(),
                },
            },
        ];

        sheet.set_annotations(annotations)?;
        assert_eq!(sheet.annotation_handles.len(), 5);
        assert_eq!(sheet.annotation_warnings.len(), 1);
        assert_eq!(sheet.annotation_warnings[0].0, AnnotationId(5));

        let h1 = sheet
            .annotation_handles
            .iter()
            .find(|h| h.id == AnnotationId(1))
            .ok_or_else(|| DrawingError("Alça 1 ausente".into()))?;
        assert_eq!(h1.label, "60");
        let h2 = sheet
            .annotation_handles
            .iter()
            .find(|h| h.id == AnnotationId(2))
            .ok_or_else(|| DrawingError("Alça 2 ausente".into()))?;
        assert_eq!(h2.label, "R 5");
        let h3 = sheet
            .annotation_handles
            .iter()
            .find(|h| h.id == AnnotationId(3))
            .ok_or_else(|| DrawingError("Alça 3 ausente".into()))?;
        assert_eq!(h3.label, "Ø 10");

        // Reposicionamento
        sheet.reposition_annotation(AnnotationId(1), [20., -30.])?;
        let h1_moved = sheet
            .annotation_handles
            .iter()
            .find(|h| h.id == AnnotationId(1))
            .ok_or_else(|| DrawingError("Alça 1 ausente".into()))?;
        assert_eq!(h1_moved.offset_mm, [20., -30.]);

        // Reposicionamento inválido
        assert!(
            sheet
                .reposition_annotation(AnnotationId(1), [f64::NAN, 0.])
                .is_err()
        );
        assert!(
            sheet
                .reposition_annotation(AnnotationId(1), [1500., 0.])
                .is_err()
        );
        assert!(
            sheet
                .reposition_annotation(AnnotationId(99), [0., 0.])
                .is_err()
        );

        // Exportação PNG e PDF funcionam com anotações
        let pdf_data = pdf(&sheet)?;
        assert!(pdf_data.starts_with(b"%PDF-1.4"));
        let png_data = png(&sheet, 96)?;
        assert_eq!(&png_data[..8], b"\x89PNG\r\n\x1a\n");

        Ok(())
    }

    #[test]
    fn invalid_annotation_geometry_is_rejected() {
        let mut sheet = Sheet {
            size: [297., 210.],
            style: Style::Conventional,
            primitives: Vec::new(),
            exact_projection: false,
            exact_sections: false,
            annotation_handles: Vec::new(),
            annotation_warnings: Vec::new(),
            annotations: Vec::new(),
            placements: Vec::new(),
            annotation_base: 0,
        };

        let invalid = vec![ResolvedAnnotation {
            id: AnnotationId(1),
            view: View::Front,
            offset_mm: [0., 0.],
            geometry: AnnotationGeometry::Linear {
                a: [f64::NAN, 0., 0.],
                b: [10., 0., 0.],
                value_mm: 10.,
            },
        }];
        assert!(sheet.set_annotations(invalid).is_err());
    }
}
