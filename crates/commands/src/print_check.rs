//! Verificar para impressão: consulta que nunca altera o documento. Analisa a malha exibida,
//! já com a face escolhida na mesa, e devolve os problemas por triângulo dessa mesma malha,
//! para que a cor apareça na orientação da modelagem.
use crate::export::{on_bed, transformed};
use nator_document::{BodyId, Document, DocumentError, Inspection, print_profile::PrintProfile};
use nator_geometry::{
    Mesh, Point3, RgbColor,
    printability::{self, Issue, PrintLimits, Printability},
};

/// Apoio abaixo desta fração da base ocupada pode soltar da mesa.
const SMALL_BASE: f64 = 0.05;

pub fn color(issue: Issue) -> RgbColor {
    RgbColor::new(match issue {
        Issue::Overhang => [236, 138, 28],
        Issue::ThinWall => [214, 46, 54],
        Issue::Gap => [146, 62, 204],
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub issue: Issue,
    pub area: f64,
    /// Ângulo em graus nos balanços; menor medida em mm nas paredes e frestas.
    pub worst: f64,
    /// Centro da região, em coordenadas da peça.
    pub center: Point3,
}

/// Estimativa antes de fatiar: paredes pela área da superfície e o resto pelo preenchimento.
/// O fatiador é quem dá o número final, com suportes, cabeçalho e purga.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    pub volume_mm3: f64,
    pub mass_g: f64,
    /// `None` enquanto o perfil não tiver preço.
    pub cost: Option<f64>,
}
impl Estimate {
    pub fn of(mesh: &Mesh, profile: &PrintProfile) -> Self {
        let volume_mm3 = mesh.signed_volume().abs();
        let skin = (mesh.surface_area() * profile.wall_lines * profile.nozzle_mm).min(volume_mm3);
        let printed = skin + (volume_mm3 - skin) * profile.infill_pct / 100.;
        let mass_g = printed / 1000. * profile.density_g_cm3;
        Self {
            volume_mm3,
            mass_g,
            cost: (profile.price_per_kg > 0.).then_some(mass_g / 1000. * profile.price_per_kg),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Report {
    pub body: BodyId,
    pub estimate: Estimate,
    pub regions: Vec<Region>,
    pub size: Point3,
    pub contact_area: f64,
    /// O corpo tem uma face escolhida; sem ela, vale a orientação da modelagem.
    pub oriented: bool,
    pub oversized: bool,
    pub small_base: bool,
    /// Triângulos menores que a precisão do STL e o primeiro ponto deles, na peça.
    pub slivers: Option<(usize, Point3)>,
    analysis: Printability,
    triangles: usize,
}
impl Report {
    /// Cor por triângulo para os problemas mostrados. Quando um triângulo tem mais de um,
    /// vale o mais grave, na ordem de `Issue::ALL`.
    pub fn inspection(&self, shown: impl Fn(Issue) -> bool) -> Inspection {
        Inspection {
            body: self.body,
            colors: (0..self.triangles)
                .map(|triangle| {
                    Issue::ALL
                        .into_iter()
                        .find(|issue| shown(*issue) && self.analysis.has(triangle, *issue))
                        .map(color)
                })
                .collect(),
        }
    }
}

fn limits(profile: &PrintProfile) -> PrintLimits {
    PrintLimits {
        max_overhang_deg: profile.max_overhang_deg,
        min_wall_mm: 2. * profile.nozzle_mm,
        min_floor_mm: 2. * profile.layer_mm,
        min_gap_mm: profile.nozzle_mm,
        min_region_mm2: profile.nozzle_mm * profile.nozzle_mm,
    }
}

pub fn inspect(doc: &Document, body: BodyId) -> Result<Report, DocumentError> {
    let error = |e: nator_geometry::GeometryError| DocumentError(e.to_string());
    let record = doc
        .body(body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let geometry = doc
        .display_geometry(body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let local = transformed(geometry.mesh(), record.placement)?;
    let down = record.print_down.map(|d| record.placement.apply_vector(d));
    let placed = on_bed(&local, down)?;
    let profile = doc.print_profile();
    let analysis = printability::analyze(&placed, limits(profile));
    let size = placed.bounds().size();
    let base = size[0] * size[1];
    let regions = analysis
        .regions
        .iter()
        .map(|region| {
            let center = printability::weighted_center(&local, &region.triangles)
                .ok_or_else(|| DocumentError("Região sem área.".into()))?;
            Ok(Region {
                issue: region.issue,
                area: region.area,
                worst: region.worst,
                center,
            })
        })
        .collect::<Result<Vec<_>, DocumentError>>()?;
    placed.single_precision().map_err(error)?;
    let collapsed = local.collapsed_in_single_precision();
    let slivers = collapsed.first().map(|first| {
        let [point, ..] = local.triangle_points(local.triangles()[*first]);
        (collapsed.len(), point)
    });
    Ok(Report {
        slivers,
        body,
        estimate: Estimate::of(&local, profile),
        regions,
        size,
        contact_area: analysis.contact_area,
        oriented: record.print_down.is_some(),
        oversized: profile
            .bed_mm
            .is_some_and(|bed| (0..3).any(|i| size[i] > bed[i])),
        small_base: analysis.contact_area < SMALL_BASE * base,
        triangles: geometry.mesh().triangles().len(),
        analysis,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nator_document::MeshAsset;
    use std::{error::Error, sync::Arc};

    /// Perfil em T deitado, de casca única: base de 5 mm e aba larga no ar, de x = 5 a 30.
    fn tee() -> Result<Mesh, Box<dyn Error>> {
        let outline = [
            [0., 0.],
            [5., 0.],
            [5., 10.],
            [30., 10.],
            [30., 15.],
            [0., 15.],
        ];
        let mut vertices = Vec::new();
        for y in [0., 5.] {
            vertices.extend(outline.map(|[x, z]| [x, y, z]));
        }
        let mut triangles = Vec::new();
        for [a, b, c] in [[0, 1, 2], [0, 2, 5], [2, 4, 5], [2, 3, 4]] {
            triangles.push([a, b, c]);
            triangles.push([a + 6, c + 6, b + 6]);
        }
        for i in 0..6_u32 {
            let j = (i + 1) % 6;
            triangles.push([i, i + 6, j]);
            triangles.push([j, i + 6, j + 6]);
        }
        Ok(Mesh::new(vertices, triangles)?)
    }
    fn document() -> Result<(Document, BodyId), Box<dyn Error>> {
        let mut doc = Document::new("Verificação")?;
        let (_, body) = doc.add_mesh(None, "Tê".into(), Arc::new(MeshAsset::new(tee()?)))?;
        Ok((doc, body))
    }

    #[test]
    fn issues_become_colors_on_the_triangles_of_the_displayed_mesh() -> Result<(), Box<dyn Error>> {
        let (doc, body) = document()?;
        let report = inspect(&doc, body)?;
        assert!(!report.oriented);
        let region = report
            .regions
            .iter()
            .find(|r| r.issue == Issue::Overhang)
            .ok_or("Balanço")?;
        // O centro fica na aba, em coordenadas da peça, para a câmera ir até lá.
        assert!((region.center[2] - 10.).abs() < 1e-9);
        let painted = report.inspection(|_| true);
        assert_eq!(painted.colors.len(), 20);
        assert!(painted.colors.contains(&Some(color(Issue::Overhang))));
        assert!(
            report
                .inspection(|issue| issue != Issue::Overhang)
                .colors
                .iter()
                .all(Option::is_none)
        );
        Ok(())
    }

    #[test]
    fn the_estimate_counts_the_walls_whole_and_the_core_by_the_infill() -> Result<(), Box<dyn Error>>
    {
        let (doc, body) = document()?;
        let report = inspect(&doc, body)?;
        // Seção do tê (5 × 15 + 25 × 5) em 5 mm de largura.
        let volume = 200. * 5.;
        assert!((report.estimate.volume_mm3 - volume).abs() < 1e-9);
        let solid = PrintProfile {
            infill_pct: 100.,
            ..Default::default()
        };
        let mesh = doc.display_geometry(body).ok_or("Malha")?.mesh().clone();
        let whole = Estimate::of(&mesh, &solid);
        assert!((whole.mass_g - volume / 1000. * 1.24).abs() < 1e-9);
        assert_eq!(whole.cost, None);
        // Peça fina: as paredes tomam todo o volume, e o preenchimento não reduz nada.
        let thick_walls = PrintProfile {
            wall_lines: 20.,
            infill_pct: 0.,
            ..Default::default()
        };
        assert!((Estimate::of(&mesh, &thick_walls).mass_g - whole.mass_g).abs() < 1e-9);
        let priced = PrintProfile {
            price_per_kg: 100.,
            ..solid
        };
        let cost = Estimate::of(&mesh, &priced).cost.ok_or("Custo")?;
        assert!((cost - whole.mass_g / 10.).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn the_chosen_orientation_and_the_bed_change_the_verdict() -> Result<(), Box<dyn Error>> {
        let (mut doc, body) = document()?;
        // De cabeça para baixo a aba larga vira a base, e o balanço some.
        doc.body_mut(body)?.print_down = Some([0., 0., 1.]);
        let report = inspect(&doc, body)?;
        assert!(report.oriented && !report.small_base);
        assert!(report.regions.iter().all(|r| r.issue != Issue::Overhang));
        assert!((report.contact_area - 150.).abs() < 1e-9);
        doc.set_print_profile(PrintProfile {
            bed_mm: Some([20., 20., 100.]),
            ..Default::default()
        })?;
        assert!(inspect(&doc, body)?.oversized);
        // Inclinada, nenhuma face fica apoiada.
        let tilt = std::f64::consts::FRAC_1_SQRT_2;
        doc.body_mut(body)?.print_down = Some([tilt, 0., tilt]);
        assert!(inspect(&doc, body)?.small_base);
        Ok(())
    }
}
