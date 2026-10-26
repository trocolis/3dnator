//! Regressões da reflexão de hierarquias: posições locais, definições compartilhadas,
//! visibilidade e transações, sem exigir um núcleo CAD para corpos de malha.
use nator_commands::{Command, Session};
use nator_document::{Document, InstanceId, MeshAsset};
use nator_geometry::{Placement, Point3, dot, norm, reference_bracket, sub};
use std::{error::Error, sync::Arc};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const ORIGIN: Point3 = [13., -7., 4.];
const NORMAL: Point3 = [2., -3., 4.];

fn reflect(point: Point3) -> Point3 {
    let n = NORMAL.map(|v| v / norm(NORMAL));
    let distance = dot(sub(point, ORIGIN), n);
    std::array::from_fn(|i| point[i] - 2. * distance * n[i])
}

/// A peça selecionada é filha de um pai rotacionado. Uma mesma folha aparece diretamente
/// na raiz selecionada e em um subconjunto com duas ocorrências: o grafo não é só uma árvore.
/// Inclui corpos e ocorrências ocultos que devem continuar disponíveis no clone.
fn hierarchy() -> Result<(Session, InstanceId, Placement)> {
    let mut doc = Document::new("Hierarquia")?;
    let outer = doc.create_part("Montagem".into())?;
    let outer_instance = doc.instances()[0].id;
    let outer_placement = Placement::new([200., -100., 60.], [-0.4, 0.3, 0.8])?;
    doc.instance_mut(outer_instance)?.placement = outer_placement;
    let geometry = Arc::new(MeshAsset::new(reference_bracket()?));
    let (root, body) = doc.add_mesh(None, "Conjunto".into(), geometry.clone())?;
    let selected = doc.instances().last().ok_or("Raiz ausente")?.id;
    doc.reparent(selected, Some(outer))?;
    doc.instance_mut(selected)?.placement = Placement::new([40., -20., 10.], [0.2, -0.3, 0.5])?;
    doc.body_mut(body)?.placement = Placement::new([-10., 5., 2.], [0.1, 0.6, -0.2])?;
    let branch = doc.create_part("Subconjunto".into())?;
    let branch_instance = doc.instances().last().ok_or("Subconjunto ausente")?.id;
    doc.reparent(branch_instance, Some(root))?;
    doc.instance_mut(branch_instance)?.placement =
        Placement::new([90., 30., -40.], [0.7, -0.2, 0.4])?;
    let second_branch = doc.add_instance(
        branch,
        Some(root),
        Placement::new([-50., 80., 15.], [-0.2, 0.5, -0.9])?,
    )?;
    doc.add_group(
        "Par de subconjuntos".into(),
        vec![branch_instance, second_branch],
    )?;
    let (leaf, leaf_body) = doc.add_mesh(None, "Componente".into(), geometry.clone())?;
    let leaf_instance = doc.instances().last().ok_or("Componente ausente")?.id;
    doc.reparent(leaf_instance, Some(branch))?;
    doc.instance_mut(leaf_instance)?.placement = Placement::new([25., 15., 5.], [-0.6, 0.9, 0.4])?;
    doc.body_mut(leaf_body)?.placement = Placement::new([3., -4., 5.], [0.1, -0.3, 0.2])?;
    doc.add_instance(
        leaf,
        Some(root),
        Placement::new([5., -65., 25.], [0.3, 0.8, -0.7])?,
    )?;
    let hidden_instance = doc.add_instance(
        leaf,
        Some(branch),
        Placement::new([75., 10., -20.], [0.4, -0.1, 0.2])?,
    )?;
    doc.instance_mut(hidden_instance)?.visible = false;
    doc.add_group(
        "Componentes do subconjunto".into(),
        vec![leaf_instance, hidden_instance],
    )?;
    doc.add_group("Grupo externo".into(), vec![selected])?;
    let (_, hidden_body) = doc.add_mesh(Some(leaf), "Corpo oculto".into(), geometry)?;
    doc.body_mut(hidden_body)?.visible = false;
    doc.body_mut(hidden_body)?.placement = Placement::new([-25., 20., 10.], [0.3, 0.4, -0.5])?;
    Ok((Session::new(doc)?, selected, outer_placement))
}

fn subtree_vertices(doc: &Document, root: InstanceId, reveal: bool) -> Result<Vec<Point3>> {
    let mut doc = doc.clone();
    if reveal {
        for id in doc.instances().iter().map(|i| i.id).collect::<Vec<_>>() {
            doc.instance_mut(id)?.visible = true;
        }
        for id in doc
            .parts()
            .iter()
            .flat_map(|p| p.bodies.iter().map(|b| b.id))
            .collect::<Vec<_>>()
        {
            doc.body_mut(id)?.visible = true;
        }
    }
    let mut points = Vec::new();
    for occurrence in doc.occurrences()?.iter().filter(|o| o.path.contains(&root)) {
        let instance = doc
            .instance(occurrence.instance)
            .ok_or("Instância ausente")?;
        let part = doc.part(instance.part).ok_or("Peça ausente")?;
        for body in part.bodies.iter().filter(|b| b.visible) {
            let placement = occurrence.placement.compose(body.placement)?;
            points.extend(
                body.geometry
                    .mesh()
                    .vertices()
                    .iter()
                    .map(|p| placement.apply(*p)),
            );
        }
    }
    Ok(points)
}

fn mirror(session: &mut Session, selected: InstanceId) -> Result<InstanceId> {
    session.execute(Command::MirrorInstance {
        id: selected,
        origin: ORIGIN,
        normal: NORMAL,
        name: "Conjunto espelhado".into(),
    })?;
    Ok(session
        .document()
        .instances()
        .last()
        .ok_or("Imagem ausente")?
        .id)
}

fn assert_reflected(before: &[Point3], after: &[Point3], parent: Placement) {
    assert!(!before.is_empty());
    assert_eq!(before.len(), after.len());
    for (source, actual) in before.iter().zip(after) {
        // O plano pertence ao pai da instância selecionada, não à montagem global.
        let expected = parent.apply(reflect(parent.inverse_apply(*source)));
        assert!(
            norm(sub(expected, *actual)) < 1e-8,
            "{expected:?} != {actual:?}"
        );
    }
}

#[test]
fn nested_mirror_preserves_sharing_and_reflects_every_occurrence() -> Result<()> {
    let (mut session, selected, parent) = hierarchy()?;
    let before = session.document().clone();
    let image = mirror(&mut session, selected)?;
    let doc = session.document();
    // Só a definição selecionada e suas duas definições descendentes são clonadas.
    assert_eq!(doc.parts().len(), before.parts().len() + 3);
    assert_eq!(doc.groups().len(), before.groups().len() + 2);
    let cloned_parts = &doc.parts()[before.parts().len()..];
    for group in &doc.groups()[before.groups().len()..] {
        for member in &group.members {
            let instance = doc.instance(*member).ok_or("Membro ausente")?;
            assert!(cloned_parts.iter().any(|p| Some(p.id) == instance.parent));
        }
    }
    let clone = cloned_parts
        .iter()
        .find(|p| p.name == "Componente")
        .ok_or("Componente clonado ausente")?;
    assert_eq!(
        doc.instances()
            .iter()
            .filter(|i| i.part == clone.id)
            .count(),
        3
    );
    for part in cloned_parts {
        assert!(
            part.bodies
                .iter()
                .all(|b| b.geometry.mesh().topology().has_closed_oriented_edges())
        );
        assert!(
            part.bodies
                .iter()
                .all(|b| b.geometry.mesh().signed_volume() > 0.)
        );
        for instance in doc.instances().iter().filter(|i| i.parent == Some(part.id)) {
            assert!(cloned_parts.iter().any(|p| p.id == instance.part));
        }
    }
    for reveal in [false, true] {
        assert_reflected(
            &subtree_vertices(&before, selected, reveal)?,
            &subtree_vertices(doc, image, reveal)?,
            parent,
        );
    }
    // A operação não reflete nem movimenta o original.
    assert_eq!(
        subtree_vertices(&before, selected, true)?,
        subtree_vertices(doc, selected, true)?
    );
    assert!(subtree_vertices(doc, image, false)?.len() < subtree_vertices(doc, image, true)?.len());
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("mirrored.3dnator");
    nator_project_io::write_project(&path, doc)?;
    let reopened = nator_project_io::read_project(&path)?;
    // A reconstrução das matrizes a partir dos ângulos serializados admite o mesmo
    // erro geométrico em mm usado na verificação da imagem antes do salvamento.
    assert_reflected(
        &subtree_vertices(&before, selected, true)?,
        &subtree_vertices(&reopened, image, true)?,
        parent,
    );
    let state = session.state();
    assert!(session.undo());
    assert_eq!(session.document().parts().len(), before.parts().len());
    assert_eq!(
        session.document().instances().len(),
        before.instances().len()
    );
    assert_eq!(session.document().groups().len(), before.groups().len());
    assert!(session.redo());
    assert_eq!(session.state(), state);
    assert_reflected(
        &subtree_vertices(&before, selected, true)?,
        &subtree_vertices(session.document(), image, true)?,
        parent,
    );
    Ok(())
}

#[test]
fn mirroring_a_hierarchy_twice_restores_its_world_geometry() -> Result<()> {
    let (mut session, selected, _) = hierarchy()?;
    let before = subtree_vertices(session.document(), selected, true)?;
    let first = mirror(&mut session, selected)?;
    let second = mirror(&mut session, first)?;
    let restored = subtree_vertices(session.document(), second, true)?;
    assert_eq!(before.len(), restored.len());
    for (a, b) in before.iter().zip(restored) {
        assert!(norm(sub(*a, b)) < 1e-8);
    }
    Ok(())
}

#[test]
fn invalid_mirror_does_not_change_the_document_or_redo() -> Result<()> {
    let (mut session, selected, _) = hierarchy()?;
    mirror(&mut session, selected)?;
    assert!(session.undo());
    let before = session.document().clone();
    let revision = session.revision();
    for normal in [[0.; 3], [f64::NAN, 0., 0.], [f64::MAX; 3]] {
        assert!(
            session
                .execute(Command::MirrorInstance {
                    id: selected,
                    origin: ORIGIN,
                    normal,
                    name: "Inválida".into()
                })
                .is_err()
        );
        assert_eq!(session.revision(), revision);
        assert_eq!(session.document().next_id(), before.next_id());
        assert_eq!(session.document().parts().len(), before.parts().len());
        assert_eq!(
            subtree_vertices(session.document(), selected, true)?,
            subtree_vertices(&before, selected, true)?
        );
    }
    assert!(session.redo());
    Ok(())
}

#[test]
fn a_child_kernel_failure_rolls_back_the_entire_mirror() -> Result<()> {
    let (mut session, selected, _) = hierarchy()?;
    let doc = session.document();
    let child = doc
        .parts()
        .iter()
        .find(|p| p.name == "Componente")
        .ok_or("Componente ausente")?
        .id;
    // B-rep inválido simula falha do núcleo depois de a raiz de malha já ter sido refletida.
    session.execute(Command::ImportSolid {
        destination: Some(child),
        name: "Importação inválida".into(),
        geometry: Arc::new(MeshAsset::new(reference_bracket()?)),
        brep: Arc::new(vec![0]),
        faces: 1,
    })?;
    let before = session.document().clone();
    let revision = session.revision();
    assert!(mirror(&mut session, selected).is_err());
    assert_eq!(session.revision(), revision);
    assert_eq!(session.document().next_id(), before.next_id());
    assert_eq!(session.document().parts().len(), before.parts().len());
    assert_eq!(
        subtree_vertices(session.document(), selected, true)?,
        subtree_vertices(&before, selected, true)?
    );
    Ok(())
}
