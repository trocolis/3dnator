//! Medições: consultas ao documento e ao núcleo que nunca alteram a geometria.
//!
//! Sólidos CAD são medidos sobre o B-rep exato; corpos de malha, sobre a malha, com o
//! resultado marcado como aproximado. Coordenadas são as da peça (origem local), como na
//! construção; quem mede em contexto de montagem compõe a posição da instância.
//! Sem materiais definidos, o centro de massa supõe densidade uniforme.
use nator_document::{BodyId, Document, DocumentError, assembly::Pick};
use nator_geometry::{Point3, cross, dot, norm, sub};

/// Volume, área e centro de massa de um corpo em coordenadas da peça.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyProperties {
    pub volume_mm3: f64,
    pub area_mm2: f64,
    pub center_mm: Point3,
    /// Medido no B-rep exato (sólido CAD) em vez da malha de exibição.
    pub exact: bool,
}

/// Medida de uma face ou aresta selecionada em um sólido.
#[derive(Debug, Clone, PartialEq)]
pub enum Topology {
    Face {
        name: String,
        area_mm2: f64,
        /// Raio de faces cilíndricas, esféricas ou toroidais (raio maior).
        radius_mm: Option<f64>,
        planar: bool,
    },
    Edge {
        faces: [String; 2],
        length_mm: f64,
        radius_mm: Option<f64>,
    },
}

/// Plano exato da face clicada, em coordenadas de montagem. A captura é
/// independente do corpo de origem: alterações futuras não criam vínculo externo.
#[cfg(feature = "occt")]
pub fn sketch_plane(doc: &Document, pick: &Pick) -> Result<nator_sketch::Plane, DocumentError> {
    let body = doc
        .body(pick.body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let bytes = body.brep.as_ref().ok_or_else(|| {
        DocumentError(
            "Selecione uma face plana de um sólido CAD; malhas não definem planos exatos.".into(),
        )
    })?;
    // O suporte guarda somente o plano, sem referência associativa. Faces de transição
    // do chanfro podem não ter nome (ou compartilhar o mesmo papel); isso não impede
    // consultar sua superfície exata. O índice vem da malha desta mesma revisão e nunca
    // é persistido. Consultamos antes de transformar para preservar a correspondência
    // entre os índices da tesselação e os do B-rep gravado, inclusive em projetos antigos.
    let index = doc.face_index_at(pick)?;
    let solid = nator_kernel::Solid::from_brep(bytes, None).map_err(error)?;
    let measured = solid.face_measure(index).map_err(error)?;
    let nator_kernel::FaceKind::Plane { normal, offset } = measured.kind else {
        return Err(DocumentError(
            "A face escolhida é curva. Selecione uma face plana ou um plano da origem.".into(),
        ));
    };
    let placement = pick.placement.compose(body.placement).map_err(error)?;
    let local = placement.inverse_apply(pick.point);
    let distance = dot(normal, local) - offset;
    let origin = std::array::from_fn(|i| local[i] - normal[i] * distance);
    let mut world_normal = placement.apply_vector(normal);
    if dot(world_normal, pick.normal) < 0. {
        world_normal = world_normal.map(|v| -v);
    }
    nator_sketch::Plane::from_normal(placement.apply(origin), world_normal).map_err(error)
}
#[cfg(not(feature = "occt"))]
pub fn sketch_plane(_doc: &Document, _pick: &Pick) -> Result<nator_sketch::Plane, DocumentError> {
    Err(DocumentError(
        "Seleção de face plana requer o núcleo OCCT.".into(),
    ))
}

/// Ângulo entre duas faces clicadas e, se forem paralelas, a distância entre elas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacePair {
    pub angle_rad: f64,
    pub parallel_distance_mm: Option<f64>,
}

fn error(e: impl std::fmt::Display) -> DocumentError {
    DocumentError(e.to_string())
}

/// Propriedades do corpo. Malhas precisam ser fechadas para ter volume significativo.
pub fn body_properties(doc: &Document, body: BodyId) -> Result<BodyProperties, DocumentError> {
    let record = doc
        .body(body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    #[cfg(feature = "occt")]
    if let Some(bytes) = &record.brep {
        let properties = nator_kernel::Solid::from_brep(bytes, None)
            .and_then(|s| s.transformed(record.placement))
            .and_then(|s| s.properties())
            .map_err(error)?;
        return Ok(BodyProperties {
            volume_mm3: properties.volume,
            area_mm2: properties.area,
            center_mm: properties.center,
            exact: true,
        });
    }
    let mesh = record.geometry.mesh();
    if !record.geometry.topology().has_closed_oriented_edges() {
        return Err(DocumentError(
            "A malha está aberta ou com orientação inconsistente; volume e centro de massa não têm significado."
                .into(),
        ));
    }
    let (center, volume) = mesh.mass_center().map_err(error)?;
    let area = mesh
        .triangles()
        .iter()
        .map(|t| {
            let [a, b, c] = mesh.triangle_points(*t);
            norm(cross(sub(b, a), sub(c, a))) / 2.
        })
        .sum();
    Ok(BodyProperties {
        volume_mm3: volume,
        area_mm2: area,
        center_mm: record.placement.apply(center),
        exact: false,
    })
}

/// Face ou aresta sob o clique, medida no B-rep exato.
#[cfg(feature = "occt")]
pub fn topology(doc: &Document, pick: &Pick, edge: bool) -> Result<Topology, DocumentError> {
    use nator_document::features::{EdgeRef, FaceRef};
    use nator_kernel::FaceKind;
    let record = doc
        .body(pick.body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let bytes = record
        .brep
        .as_ref()
        .ok_or_else(|| DocumentError("Medidas de faces e arestas exigem sólido CAD.".into()))?;
    let solid = nator_kernel::Solid::from_brep(bytes, Some(&record.face_names))
        .and_then(|s| s.transformed(record.placement))
        .map_err(error)?;
    if edge {
        let EdgeRef::Between(faces) = doc.edge_at(pick)? else {
            return Err(DocumentError("Aresta sem faces nomeadas.".into()));
        };
        let measure = solid
            .edge_measure(solid.edge_between(&faces).map_err(error)?)
            .map_err(error)?;
        Ok(Topology::Edge {
            faces,
            length_mm: measure.length,
            radius_mm: measure.radius,
        })
    } else {
        let FaceRef::Named(name) = doc.face_at(pick)? else {
            return Err(DocumentError("Face sem nome estável.".into()));
        };
        let measure = solid
            .face_measure(solid.face_named(&name).map_err(error)?)
            .map_err(error)?;
        let (radius_mm, planar) = match measure.kind {
            FaceKind::Plane { .. } => (None, true),
            FaceKind::Cylinder { radius } | FaceKind::Sphere { radius } => (Some(radius), false),
            FaceKind::Torus { major_radius } => (Some(major_radius), false),
            FaceKind::Cone | FaceKind::Other => (None, false),
        };
        Ok(Topology::Face {
            name,
            area_mm2: measure.area,
            radius_mm,
            planar,
        })
    }
}

#[cfg(not(feature = "occt"))]
pub fn topology(_doc: &Document, _pick: &Pick, _edge: bool) -> Result<Topology, DocumentError> {
    nator_kernel::require_available().map_err(error)?;
    Err(DocumentError(
        "Medição de faces e arestas indisponível.".into(),
    ))
}

/// Ângulo entre as normais de duas faces clicadas (coordenadas de mundo). Faces paralelas,
/// dentro de 0,01°, informam também a distância entre os planos que passam pelos cliques.
pub fn face_pair(first: &Pick, second: &Pick) -> Result<FacePair, DocumentError> {
    let (a, b) = (norm(first.normal), norm(second.normal));
    if a < 1e-12 || b < 1e-12 {
        return Err(DocumentError("Face sem normal definida.".into()));
    }
    let n = first.normal.map(|v| v / a);
    let cosine = (dot(n, second.normal) / b).clamp(-1., 1.);
    let angle_rad = cosine.acos();
    let parallel = angle_rad < 0.01_f64.to_radians() || angle_rad > (180. - 0.01_f64).to_radians();
    Ok(FacePair {
        angle_rad,
        parallel_distance_mm: parallel.then(|| dot(sub(second.point, first.point), n).abs()),
    })
}

/// Menor distância entre os sólidos de dois cliques, em coordenadas de mundo.
#[cfg(feature = "occt")]
pub fn body_distance(doc: &Document, first: &Pick, second: &Pick) -> Result<f64, DocumentError> {
    let solid = |pick: &Pick| -> Result<nator_kernel::Solid, DocumentError> {
        let record = doc
            .body(pick.body)
            .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
        let bytes = record
            .brep
            .as_ref()
            .ok_or_else(|| DocumentError("Distância entre corpos exige sólidos CAD.".into()))?;
        let pose = pick.placement.compose(record.placement).map_err(error)?;
        nator_kernel::Solid::from_brep(bytes, None)
            .and_then(|s| s.transformed(pose))
            .map_err(error)
    };
    Ok(solid(first)?.distance(&solid(second)?).map_err(error)?.0)
}

#[cfg(not(feature = "occt"))]
pub fn body_distance(_doc: &Document, _first: &Pick, _second: &Pick) -> Result<f64, DocumentError> {
    nator_kernel::require_available().map_err(error)?;
    Err(DocumentError("Distância entre corpos indisponível.".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nator_document::{MeshAsset, assembly::Pick};
    use nator_geometry::Placement;
    use std::{error::Error, sync::Arc};

    fn pick(point: Point3, normal: Point3) -> Pick {
        Pick {
            instance: nator_document::InstanceId(1),
            part: nator_document::PartId(1),
            body: BodyId(1),
            triangle: 0,
            point,
            normal,
            vertices: [[0.; 3]; 3],
            placement: Placement::default(),
        }
    }

    #[test]
    fn mesh_properties_are_approximate_and_follow_the_body_position() -> Result<(), Box<dyn Error>>
    {
        let mut doc = Document::new("Medição")?;
        let (_, body) = doc.add_mesh(
            None,
            "Suporte".into(),
            Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
        )?;
        doc.body_mut(body)?.placement = Placement::new([100., 0., 0.], [0.; 3])?;
        let properties = body_properties(&doc, body)?;
        assert!(!properties.exact);
        assert!((properties.volume_mm3 - 10_080.).abs() < 1e-6);
        // L de 60 × 20 × 30 com 6 de espessura: área total analítica.
        let area = 2. * 504. + 20. * (60. + 6. + 54. + 24. + 6. + 30.);
        assert!((properties.area_mm2 - area).abs() < 1e-6);
        assert!(properties.center_mm[0] > 100.);
        Ok(())
    }

    #[test]
    fn parallel_faces_report_their_distance() -> Result<(), Box<dyn Error>> {
        let pair = face_pair(
            &pick([0., 0., 0.], [0., 0., -1.]),
            &pick([5., 3., 12.], [0., 0., 1.]),
        )?;
        assert!((pair.angle_rad - std::f64::consts::PI).abs() < 1e-12);
        assert_eq!(pair.parallel_distance_mm, Some(12.));
        let square = face_pair(&pick([0.; 3], [1., 0., 0.]), &pick([0.; 3], [0., 1., 0.]))?;
        assert!((square.angle_rad - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert_eq!(square.parallel_distance_mm, None);
        Ok(())
    }
}
