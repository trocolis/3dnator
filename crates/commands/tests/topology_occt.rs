//! Faces/arestas sem nome usam a assinatura exata, inclusive em ocorrências movidas.
#![cfg(feature = "occt")]
use nator_commands::{Command, Session};
use nator_document::{
    Document, MeshAsset,
    features::{EdgeRef, FaceRef, Operation, signature},
};
use nator_geometry::{Placement, query::Ray};
use nator_kernel::Solid;
use std::sync::Arc;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[test]
fn unnamed_faces_and_edges_are_selectable_but_do_not_follow_changed_geometry() -> Result<()> {
    let mut solid = Solid::cuboid([0.; 3], [20.; 3])?;
    let (mesh, faces) = solid.tessellate_with_faces(0.05, 0.25)?;
    let bytes = Arc::new(solid.to_brep()?);
    let mut doc = Document::new("Referência conservadora")?;
    let (part, body) = doc.add_mesh(
        None,
        "Cubo".into(),
        Arc::new(MeshAsset::with_faces(mesh, faces)?),
    )?;
    let target = doc.body_mut(body)?;
    target.brep = Some(bytes.clone());
    target.face_names = Arc::new(vec![String::new(); solid.face_count()?]);
    target.placement = Placement::new([30., -4., 7.], [0.1, 0.2, 0.3])?;
    let instance = doc.instances()[0].id;
    let occurrence = Placement::new([100., 20., 0.], [-0.2, 0.3, 0.1])?;
    doc.instance_mut(instance)?.placement = occurrence;
    let pose = occurrence.compose(doc.body(body).ok_or("Corpo")?.placement)?;
    let point = pose.apply([10., 0.1, 20.]);
    let normal = pose.apply_vector([0., 0., 1.]);
    let pick = doc
        .pick(Ray {
            origin: std::array::from_fn(|i| point[i] + normal[i] * 5.),
            direction: normal.map(|x| -x),
        })?
        .ok_or("Clique")?;
    let face = doc.face_at(&pick)?;
    assert!(matches!(face,FaceRef::Index {signature:hash,..} if hash==signature(&bytes)));
    let edge = nator_commands::topology::edge_at(&doc, &pick)?;
    assert!(matches!(edge,EdgeRef::Index {signature:hash,..} if hash==signature(&bytes)));
    nator_commands::measure::topology(&doc, &pick, false)?;
    nator_commands::measure::topology(&doc, &pick, true)?;
    let mut session = Session::new(doc)?;
    session.execute(Command::AddFeature {
        part,
        name: "Arredondamento".into(),
        operation: Operation::Fillet {
            base: body,
            edge,
            radius: "1 mm".into(),
            additional_edges: vec![],
        },
    })?;
    // Uma assinatura inventada não é reinterpretada como a face no mesmo índice.
    let invalid = match face {
        FaceRef::Index { index, .. } => FaceRef::Index {
            index,
            signature: 0,
        },
        other => other,
    };
    assert!(
        session
            .execute(Command::AddFeature {
                part,
                name: "Casca inválida".into(),
                operation: Operation::Shell {
                    base: body,
                    face: invalid,
                    thickness: "1 mm".into(),
                    additional_faces: vec![],
                    outward: false,
                    through: false,
                    exit_face: None
                }
            })
            .is_err()
    );
    Ok(())
}
