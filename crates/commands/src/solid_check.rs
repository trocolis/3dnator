//! Verificação de sólidos: consulta que nunca altera o documento e o reparo de contatos em
//! linha usado pelo recálculo. Coordenadas são as da peça, como nas medições.
use nator_document::{BodyId, Document, DocumentError, PartId};
use nator_geometry::{Point3, Topology};

/// Operação do histórico em que o contato apareceu.
#[derive(Debug, Clone, PartialEq)]
pub struct Origin {
    /// Posição no histórico, a partir de 1.
    pub step: usize,
    pub name: String,
    pub sketch: Option<String>,
}

/// Contato em linha. Arestas entre as mesmas faces formam um único contato: o nome das
/// faces é a referência estável, e escolher só uma delas seria ambíguo.
#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    pub faces: Vec<String>,
    pub segments: Vec<Vec<Point3>>,
    pub length: f64,
    pub origin: Option<Origin>,
}

/// Face estreita demais para o `f32` do STL: uma lasca de poucos nanômetros, em geral vinda
/// de um rascunho quase alinhado à parede que já existia. Não muda a peça impressa.
#[derive(Debug, Clone, PartialEq)]
pub struct Sliver {
    pub face: String,
    /// Centro da lasca, em coordenadas da peça.
    pub center: Point3,
    pub width: f64,
    pub origin: Option<Origin>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub body: BodyId,
    pub contacts: Vec<Contact>,
    pub slivers: Vec<Sliver>,
    /// Topologia da malha na precisão do STL: acusa defeitos que não vêm de contatos,
    /// como frestas que colapsam ao exportar.
    pub mesh: Topology,
}
impl Report {
    /// Defeitos na malha sem contato reparável no B-rep.
    pub fn unexplained_mesh_defects(&self) -> bool {
        self.contacts.is_empty() && !self.mesh.has_closed_oriented_edges()
    }
}

#[cfg(feature = "occt")]
pub fn inspect(doc: &Document, part: PartId, body: BodyId) -> Result<Report, DocumentError> {
    use crate::features::source;
    let error = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let piece = doc
        .part(part)
        .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
    let record = doc
        .body(body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let mesh = record
        .geometry
        .mesh()
        .single_precision()
        .map_err(|e| DocumentError(e.to_string()))?
        .topology();
    let origin_of = |step: usize, feature: &nator_document::features::Feature| Origin {
        step: step + 1,
        name: feature.name.clone(),
        sketch: feature
            .operation
            .sketch()
            .and_then(|id| piece.sketches.iter().find(|s| s.id == id))
            .map(|s| s.sketch.name.clone()),
    };
    let slivers = slivers(record, |face| {
        // A face nasceu na operação que a nomeou (`b{corpo}:...`), não na última do corpo.
        let owner = face.strip_prefix('b')?.split(':').next()?.parse().ok()?;
        piece
            .features
            .iter()
            .enumerate()
            .find(|(_, feature)| feature.output == BodyId(owner))
            .map(|(step, feature)| origin_of(step, feature))
    });
    let (solid, _) = source(doc, part, body)?;
    let found = solid.contacts().map_err(error)?;
    // Só os corpos de que este resultado descende, na ordem do histórico.
    let mut chain = vec![body];
    let mut next = 0;
    while let Some(id) = chain.get(next).copied() {
        next += 1;
        if let Some(feature) = piece.features.iter().find(|f| f.output == id) {
            for input in feature.operation.inputs() {
                if !chain.contains(&input) {
                    chain.push(input);
                }
            }
        }
    }
    let history = piece
        .features
        .iter()
        .enumerate()
        .filter(|(_, feature)| !found.is_empty() && chain.contains(&feature.output))
        .filter_map(|(step, feature)| {
            let contacts = source(doc, part, feature.output).ok()?.0.contacts().ok()?;
            Some((step, feature, contacts))
        })
        .collect::<Vec<_>>();
    let same_place = |a: &[Point3], b: &[Point3]| {
        let close = |p: Option<&Point3>, q: Option<&Point3>| {
            p.zip(q)
                .is_some_and(|(p, q)| nator_geometry::norm(nator_geometry::sub(*p, *q)) < 1e-6)
        };
        (close(a.first(), b.first()) && close(a.last(), b.last()))
            || (close(a.first(), b.last()) && close(a.last(), b.first()))
    };
    let origin = |points: &[Point3]| {
        history
            .iter()
            .find(|(.., contacts)| contacts.iter().any(|c| same_place(&c.points, points)))
            .map(|(step, feature, _)| origin_of(*step, feature))
    };
    let mut contacts: Vec<Contact> = Vec::new();
    for contact in found {
        if let Some(group) = contacts.iter_mut().find(|c| c.faces == contact.faces) {
            group.length += contact.length;
            group.segments.push(contact.points);
            continue;
        }
        contacts.push(Contact {
            origin: origin(&contact.points),
            length: contact.length,
            segments: vec![contact.points],
            faces: contact.faces,
        });
    }
    Ok(Report {
        body,
        contacts,
        slivers,
        mesh,
    })
}

/// Faces com triângulos que somem no `f32`, com a posição e a largura de cada uma.
#[cfg(feature = "occt")]
fn slivers(record: &nator_document::Body, origin: impl Fn(&str) -> Option<Origin>) -> Vec<Sliver> {
    let mesh = record.geometry.mesh();
    let Some(owners) = record.geometry.triangle_faces() else {
        return Vec::new();
    };
    let mut faces = mesh
        .collapsed_in_single_precision()
        .into_iter()
        .filter_map(|triangle| owners.get(triangle).copied())
        .collect::<Vec<_>>();
    faces.sort_unstable();
    faces.dedup();
    faces
        .into_iter()
        .map(|face| {
            let mut min = [f64::INFINITY; 3];
            let mut max = [f64::NEG_INFINITY; 3];
            for point in mesh
                .triangles()
                .iter()
                .zip(owners)
                .filter(|(_, owner)| **owner == face)
                .flat_map(|(triangle, _)| mesh.triangle_points(*triangle))
            {
                let point = record.placement.apply(point);
                min = std::array::from_fn(|axis| min[axis].min(point[axis]));
                max = std::array::from_fn(|axis| max[axis].max(point[axis]));
            }
            // A fita tem uma direção nula e duas reais: a largura é a menor das reais.
            let mut size: [f64; 3] = std::array::from_fn(|axis| max[axis] - min[axis]);
            size.sort_by(f64::total_cmp);
            let name = record
                .face_names
                .get(face as usize)
                .cloned()
                .unwrap_or_default();
            Sliver {
                origin: origin(&name),
                face: name,
                center: std::array::from_fn(|axis| (min[axis] + max[axis]) / 2.),
                width: size[1],
            }
        })
        .collect()
}

#[cfg(not(feature = "occt"))]
pub fn inspect(_: &Document, _: PartId, _: BodyId) -> Result<Report, DocumentError> {
    nator_kernel::require_available().map_err(|e| DocumentError(e.to_string()))?;
    Err(DocumentError("Verificação de sólidos indisponível.".into()))
}

/// Repara cada contato indicado. Contato inexistente é erro, nunca outra geometria, e um
/// resultado que mantenha ou crie contatos é recusado.
#[cfg(feature = "occt")]
pub(super) fn repair(
    mut solid: nator_kernel::Solid,
    contacts: &[Vec<String>],
    mode: nator_document::features::ContactRepair,
    radius: f64,
    owner: &str,
) -> Result<nator_kernel::Solid, DocumentError> {
    use nator_document::features::ContactRepair;
    let error = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let before = solid.contacts().map_err(error)?;
    let mut repaired = 0;
    for (index, faces) in contacts.iter().enumerate() {
        let count = before.iter().filter(|c| c.faces == *faces).count();
        if count == 0 {
            return Err(DocumentError(format!(
                "O contato {} não existe mais neste corpo; edite ou remova o reparo.",
                index + 1
            )));
        }
        for _ in 0..count {
            let current = solid.contacts().map_err(error)?;
            // Um pino pode alcançar o contato vizinho entre as mesmas faces.
            let Some(contact) = current.iter().find(|c| c.faces == *faces) else {
                break;
            };
            solid = solid
                .repair_contact(
                    contact.edge,
                    radius,
                    mode == ContactRepair::Weld,
                    repaired,
                    owner,
                )
                .map_err(error)?;
            repaired += 1;
        }
    }
    let after = solid.contacts().map_err(error)?;
    if after.len() + repaired > before.len()
        || contacts
            .iter()
            .any(|faces| after.iter().any(|c| c.faces == *faces))
    {
        return Err(DocumentError(
            "O reparo não eliminou o contato; ajuste o raio.".into(),
        ));
    }
    Ok(solid)
}
