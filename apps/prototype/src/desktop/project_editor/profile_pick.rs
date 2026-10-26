//! Escolha de perfis diretamente na área 3D: com o campo Perfil ativo, as regiões fechadas
//! dos rascunhos da peça são realçadas sob o cursor e escolhidas por clique, no plano do
//! rascunho e na posição da ocorrência. Só regiões válidas de `profiles::regions` aparecem;
//! nada é fechado ou corrigido implicitamente.
use super::{feature_editor::Field, sketch_scene::Frame, *};
use nator_sketch::{Point2, profiles};

/// Altura de cada faixa do preenchimento por varredura, em pontos.
const BAND: f32 = 2.;

/// Preenche um contorno com furos por varredura horizontal (regra par-ímpar), que serve
/// para regiões côncavas sem triangulação.
fn fill(painter: &egui::Painter, polygons: &[Vec<egui::Pos2>], color: egui::Color32) {
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
        let chosen = draft
            .sketch()
            .zip(draft.region_boundary().map(<[u64]>::to_vec));
        let current = draft.sketch();
        let document = self.current().session.document();
        let Some(part) = document.part(part_id) else {
            return false;
        };
        let Ok(placement) = sketch_scene::part_placement(self.current(), part_id) else {
            return false;
        };
        let mut candidates = Vec::new();
        for record in part
            .sketches
            .iter()
            .filter(|s| s.visible || Some(s.id) == current)
        {
            let Ok(regions) = profiles::regions(&record.sketch) else {
                continue;
            };
            for region in regions {
                candidates.push(Candidate {
                    sketch: record.id,
                    boundary: region.boundary(),
                    area: region.area(),
                    outer: region.outer,
                    holes: region.holes,
                    frame: Frame {
                        camera: self.viewport.camera.clone(),
                        bounds: self.display_bounds(),
                        rect: response.rect,
                        placement,
                        plane: record.sketch.plane,
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
            let selected = chosen
                .as_ref()
                .is_some_and(|(s, b)| *s == candidate.sketch && *b == candidate.boundary);
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
            let (sketch, boundary) = (candidate.sketch, candidate.boundary.clone());
            if let Some(draft) = self.feature_edit.as_mut() {
                draft.choose_region(sketch, boundary);
            }
            return true;
        }
        response.clicked()
    }
}
