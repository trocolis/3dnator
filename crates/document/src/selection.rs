//! Seleção topológica pela malha com origem de faces. Nomes únicos conservam
//! identidade no recálculo; faces sem nome único usam índice + assinatura do
//! B-rep, exigindo nova seleção se a geometria mudar. Arestas exatas sem nome
//! são resolvidas na camada de comandos; a malha fornece seu par de faces.
use super::*;
use assembly::Pick;
use features::{EdgeRef, FaceRef};
use nator_geometry::Point3;
use std::collections::HashMap;

fn solid<'a>(doc: &'a Document, pick: &Pick) -> Result<(&'a Body, &'a [u32]), DocumentError> {
    let body = doc
        .body(pick.body)
        .ok_or_else(|| DocumentError("Corpo selecionado inexistente.".into()))?;
    if body.brep.is_none() {
        return Err(DocumentError(
            "Corpos de malha não têm faces e arestas CAD para selecionar.".into(),
        ));
    }
    let faces = body.geometry.triangle_faces().ok_or_else(|| {
        DocumentError(
            "A malha deste sólido não registra as faces de origem; recalcule a operação.".into(),
        )
    })?;
    if pick.triangle >= faces.len() {
        return Err(DocumentError("Triângulo selecionado inexistente.".into()));
    }
    Ok((body, faces))
}

fn face_name(body: &Body, face: u32) -> Result<String, DocumentError> {
    body.face_names
        .get(face as usize)
        .filter(|n| !n.is_empty())
        .cloned()
        .ok_or_else(|| {
            DocumentError(
                "A face selecionada não tem nome estável e não pode ser referenciada.".into(),
            )
        })
}

/// Distância de `p` ao segmento `a`–`b`.
fn segment_distance(p: Point3, a: Point3, b: Point3) -> f64 {
    use nator_geometry::{dot, norm, sub};
    let d = sub(b, a);
    let length = dot(d, d);
    let t = if length > 0. {
        (dot(sub(p, a), d) / length).clamp(0., 1.)
    } else {
        0.
    };
    norm(sub(p, std::array::from_fn(|i| a[i] + t * d[i])))
}

impl Document {
    /// Índice transitório da face do B-rep sob o clique, válido só para esta geometria.
    /// Serve para consultas que capturam valores (como um plano fixo de rascunho).
    /// Referências persistentes usam `face_at`, com nome ou assinatura do B-rep.
    pub fn face_index_at(&self, pick: &Pick) -> Result<usize, DocumentError> {
        let (_, faces) = solid(self, pick)?;
        Ok(faces[pick.triangle] as usize)
    }

    /// Face do sólido sob o clique.
    pub fn face_at(&self, pick: &Pick) -> Result<FaceRef, DocumentError> {
        let (body, faces) = solid(self, pick)?;
        let index = faces[pick.triangle] as usize;
        Ok(match face_name(body, index as u32) {
            Ok(name) if body.face_names.iter().filter(|n| **n == name).count() == 1 => {
                FaceRef::Named(name)
            }
            _ => FaceRef::Index {
                index,
                signature: features::signature(
                    body.brep.as_deref().map(Vec::as_slice).unwrap_or_default(),
                ),
            },
        })
    }

    /// Aresta do B-rep mais próxima do clique entre a face clicada e uma face vizinha.
    pub fn edge_at(&self, pick: &Pick) -> Result<EdgeRef, DocumentError> {
        let body = self
            .body(pick.body)
            .ok_or_else(|| DocumentError("Corpo selecionado inexistente.".into()))?;
        let [a, b] = self.edge_faces_at(pick)?;
        Ok(EdgeRef::Between([
            face_name(body, a as u32)?,
            face_name(body, b as u32)?,
        ]))
    }
    /// Par transitório de faces junto à aresta mais próxima. Não exige nome;
    /// serve ao realce e à consulta exata de aresta no núcleo, fora da interface.
    pub fn edge_faces_at(&self, pick: &Pick) -> Result<[usize; 2], DocumentError> {
        let (body, faces) = solid(self, pick)?;
        let mesh = body.geometry.mesh();
        let pose = pick
            .placement
            .compose(body.placement)
            .map_err(|e| DocumentError(e.to_string()))?;
        let point = pose.inverse_apply(pick.point);
        let clicked = faces[pick.triangle];
        // Triângulos por aresta da malha (os vértices da tesselação são fundidos).
        let mut edges: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for (index, t) in mesh.triangles().iter().enumerate() {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                edges.entry((a.min(b), a.max(b))).or_default().push(index);
            }
        }
        let mut best: Option<(f64, u32)> = None;
        for ((a, b), triangles) in &edges {
            let [first, second] = triangles.as_slice() else {
                continue;
            };
            let (f, g) = (faces[*first], faces[*second]);
            if f == g || (f != clicked && g != clicked) {
                continue;
            }
            let other = if f == clicked { g } else { f };
            let distance = segment_distance(
                point,
                mesh.vertices()[*a as usize],
                mesh.vertices()[*b as usize],
            );
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, other));
            }
        }
        let (_, other) = best.ok_or_else(|| {
            DocumentError("A face selecionada não faz fronteira com outra face.".into())
        })?;
        Ok([clicked as usize, other as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cubo unitário em que cada par de triângulos é uma face nomeada `b1:f{n}`.
    fn cube() -> Result<(Document, BodyId), Box<dyn Error>> {
        let vertices = vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
        ];
        let triangles = vec![
            [0, 2, 1],
            [0, 3, 2], // z = 0
            [4, 5, 6],
            [4, 6, 7], // z = 1
            [0, 1, 5],
            [0, 5, 4], // y = 0
            [2, 3, 7],
            [2, 7, 6], // y = 1
            [1, 2, 6],
            [1, 6, 5], // x = 1
            [3, 0, 4],
            [3, 4, 7], // x = 0
        ];
        let faces = (0..12).map(|i| i / 2).collect();
        let asset = MeshAsset::with_faces(Mesh::new(vertices, triangles)?, faces)?;
        let mut doc = Document::new("Cubo")?;
        let (_, body) = doc.add_mesh(None, "Cubo".into(), Arc::new(asset))?;
        let target = doc.body_mut(body)?;
        target.brep = Some(Arc::new(b"brep".to_vec()));
        target.face_names = Arc::new((0..6).map(|i| format!("b1:f{i}")).collect());
        doc.validate()?;
        Ok((doc, body))
    }

    fn pick_at(doc: &Document, body: BodyId, triangle: usize, point: Point3) -> Pick {
        Pick {
            instance: doc.instances()[0].id,
            part: doc.parts()[0].id,
            body,
            triangle,
            point,
            normal: [0., 0., 1.],
            vertices: [[0.; 3]; 3],
            placement: Placement::default(),
        }
    }

    #[test]
    fn clicks_become_named_faces_and_edges() -> Result<(), Box<dyn Error>> {
        let (doc, body) = cube()?;
        // Clique na tampa superior (z = 1), perto da aresta com a face y = 0.
        let pick = pick_at(&doc, body, 2, [0.5, 0.05, 1.]);
        assert_eq!(doc.face_at(&pick)?, FaceRef::Named("b1:f1".into()));
        assert_eq!(
            doc.edge_at(&pick)?,
            EdgeRef::Between(["b1:f1".into(), "b1:f2".into()])
        );
        // Perto da aresta com x = 1, a outra fronteira da mesma face é a escolhida.
        let pick = pick_at(&doc, body, 3, [0.97, 0.5, 1.]);
        assert_eq!(
            doc.edge_at(&pick)?,
            EdgeRef::Between(["b1:f1".into(), "b1:f4".into()])
        );
        Ok(())
    }

    #[test]
    fn unnamed_faces_use_geometry_signature_and_allow_transient_edge_pairs()
    -> Result<(), Box<dyn Error>> {
        let (mut doc, body) = cube()?;
        let mut names = (*doc.body(body).ok_or("Corpo ausente")?.face_names).clone();
        names[1].clear();
        doc.body_mut(body)?.face_names = Arc::new(names);
        let pick = pick_at(&doc, body, 2, [0.5, 0.05, 1.]);
        // Referências por assinatura não escolhem outra face após recálculo.
        assert_eq!(doc.face_index_at(&pick)?, 1);
        assert!(matches!(
            doc.face_at(&pick)?,
            FaceRef::Index { index: 1, .. }
        ));
        assert_eq!(doc.edge_faces_at(&pick)?, [1, 2]);
        assert!(doc.edge_at(&pick).is_err());
        let mut invalid = pick.clone();
        invalid.triangle = usize::MAX;
        assert!(doc.face_index_at(&invalid).is_err());
        doc.body_mut(body)?.brep = None;
        doc.body_mut(body)?.face_names = Arc::default();
        assert!(doc.face_index_at(&pick).is_err());
        assert!(doc.face_at(&pick).is_err());
        Ok(())
    }
}
