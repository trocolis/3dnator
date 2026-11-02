//! Verificar encaixe: consulta que compara os corpos da montagem como estão posicionados e
//! acusa interferências e folgas menores que a justa do perfil. Nunca altera o documento.
use nator_document::{Document, DocumentError, PartId};
use nator_geometry::Point3;

/// Volume comum abaixo disto é resto numérico de faces encostadas.
pub const INTERFERENCE_MM3: f64 = 1e-3;
/// Distância abaixo disto conta como encostado.
pub const TOUCHING_MM: f64 = 1e-4;
/// Tempo de comparação, para uma montagem enorme não prender o trabalhador.
#[cfg(feature = "occt")]
const TIME_BUDGET: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Interference,
    Touching,
    Tight,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    pub severity: Severity,
    pub names: [String; 2],
    pub volume_mm3: f64,
    pub distance_mm: f64,
    /// Onde olhar, em coordenadas da montagem.
    pub point: Point3,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub contacts: Vec<Contact>,
    /// Pares próximos o bastante para serem medidos.
    pub compared: usize,
    /// Pares que ficaram sem comparar ao acabar o tempo.
    pub beyond_limit: usize,
    /// Malhas e superfícies abertas, sem volume para comparar.
    pub skipped: Vec<String>,
    /// Pares que o núcleo não conseguiu comparar.
    pub failed: Vec<([String; 2], String)>,
}

#[cfg(not(feature = "occt"))]
pub fn check(_doc: &Document, _part: Option<PartId>) -> Result<Report, DocumentError> {
    nator_kernel::require_available().map_err(|e| DocumentError(e.to_string()))?;
    Ok(Report::default())
}

#[cfg(feature = "occt")]
struct Item<'a> {
    name: String,
    part: PartId,
    /// Componente de primeiro nível: as peças internas de um mesmo módulo importado se
    /// tocam e se cruzam de propósito, e encheriam o relatório.
    component: nator_document::InstanceId,
    instance: nator_document::InstanceId,
    body: &'a nator_document::Body,
    pose: nator_geometry::Placement,
    bounds: (Point3, Point3),
}

/// Caixa alinhada aos eixos da montagem, pelos oito cantos da caixa local.
#[cfg(feature = "occt")]
fn placed_bounds(item: &nator_document::Body, pose: nator_geometry::Placement) -> (Point3, Point3) {
    let bounds = item.geometry.mesh().bounds();
    let (mut low, mut high) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for corner in 0..8_u8 {
        let local = std::array::from_fn(|axis| {
            if corner >> axis & 1 == 0 {
                bounds.min[axis]
            } else {
                bounds.max[axis]
            }
        });
        let point = pose.apply(local);
        for axis in 0..3 {
            low[axis] = low[axis].min(point[axis]);
            high[axis] = high[axis].max(point[axis]);
        }
    }
    (low, high)
}

/// Com `part`, compara os corpos dela entre si e com todo o resto. Sem peça, compara os
/// componentes da montagem entre si.
#[cfg(feature = "occt")]
pub fn check(doc: &Document, part: Option<PartId>) -> Result<Report, DocumentError> {
    use nator_kernel::{KernelError, Solid};
    use std::collections::{HashMap, hash_map::Entry};
    let margin = doc.print_profile().fit_tight_mm;
    let mut report = Report::default();
    let mut items = Vec::new();
    for occurrence in doc.occurrences()? {
        let definition = doc
            .instance(occurrence.instance)
            .and_then(|i| doc.part(i.part))
            .ok_or_else(|| DocumentError("Ocorrência inexistente.".into()))?;
        for body in definition.shown_bodies() {
            let name = format!("{} / {}", definition.name, body.name);
            let mesh = body.geometry.mesh();
            if body.brep.is_none()
                || mesh.topology().boundary_edges > 0
                || mesh.signed_volume() <= 0.
            {
                report.skipped.push(name);
                continue;
            }
            let pose = occurrence
                .placement
                .compose(body.placement)
                .map_err(|e| DocumentError(e.to_string()))?;
            items.push(Item {
                name,
                part: definition.id,
                component: occurrence
                    .path
                    .first()
                    .copied()
                    .unwrap_or(occurrence.instance),
                instance: occurrence.instance,
                body,
                bounds: placed_bounds(body, pose),
                pose,
            });
        }
    }
    let near = |a: &Item, b: &Item| {
        (0..3).all(|axis| {
            a.bounds.0[axis] <= b.bounds.1[axis] + margin
                && b.bounds.0[axis] <= a.bounds.1[axis] + margin
        })
    };
    let mut pairs = Vec::new();
    for (i, a) in items.iter().enumerate() {
        for b in &items[i + 1..] {
            let chosen = match part {
                Some(part) => a.part == part || b.part == part,
                None => a.component != b.component,
            };
            if chosen && near(a, b) {
                pairs.push((a, b));
            }
        }
    }
    let started = std::time::Instant::now();
    let mut solids = HashMap::new();
    let total = pairs.len();
    for (a, b) in pairs {
        if started.elapsed() > TIME_BUDGET {
            report.beyond_limit = total - report.compared;
            break;
        }
        report.compared += 1;
        let compared = (|| -> Result<Option<Contact>, KernelError> {
            let load = |item: &Item| -> Result<Solid, KernelError> {
                let bytes = item
                    .body
                    .brep
                    .as_ref()
                    .ok_or_else(|| KernelError("Corpo sem sólido.".into()))?;
                Solid::from_brep(bytes, None)?.transformed(item.pose)
            };
            for item in [a, b] {
                let key = (item.instance, item.body.id);
                if let Entry::Vacant(slot) = solids.entry(key) {
                    slot.insert(load(item)?);
                }
            }
            let first = solids
                .get(&(a.instance, a.body.id))
                .ok_or_else(|| KernelError("Sólido ausente.".into()))?;
            let second = solids
                .get(&(b.instance, b.body.id))
                .ok_or_else(|| KernelError("Sólido ausente.".into()))?;
            let volume_mm3 = first.common_volume(second)?;
            let (distance_mm, from, to) = first.distance(second)?;
            let severity = if volume_mm3 > INTERFERENCE_MM3 {
                Severity::Interference
            } else if distance_mm < TOUCHING_MM {
                Severity::Touching
            } else if distance_mm < margin {
                Severity::Tight
            } else {
                return Ok(None);
            };
            // Peças aninhadas têm distância sem sentido: vale o meio da caixa em comum.
            let point = if severity == Severity::Interference {
                std::array::from_fn(|axis| {
                    (a.bounds.0[axis].max(b.bounds.0[axis])
                        + a.bounds.1[axis].min(b.bounds.1[axis]))
                        / 2.
                })
            } else {
                std::array::from_fn(|axis| (from[axis] + to[axis]) / 2.)
            };
            Ok(Some(Contact {
                severity,
                names: [a.name.clone(), b.name.clone()],
                volume_mm3,
                distance_mm,
                point,
            }))
        })();
        match compared {
            Ok(Some(contact)) => report.contacts.push(contact),
            Ok(None) => {}
            Err(e) => report
                .failed
                .push(([a.name.clone(), b.name.clone()], e.to_string())),
        }
    }
    report.contacts.sort_by(|a, b| {
        (a.severity, b.volume_mm3, a.distance_mm)
            .partial_cmp(&(b.severity, a.volume_mm3, b.distance_mm))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(report)
}
