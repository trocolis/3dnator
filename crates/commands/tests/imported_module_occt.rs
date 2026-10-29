//! Ensaio opcional com o STEP fornecido pelo usuário, sem incorporar o CAD de terceiros.
#![cfg(feature = "occt")]
use nator_commands::{Command, Session};
use nator_document::Document;
use nator_geometry::Placement;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[test]
#[ignore = "Defina NATOR_STEP_REGRESSION com o STEP externo a testar."]
fn imported_module_moves_as_one_assembly_without_changing_local_children() -> Result<()> {
    let path = std::env::var_os("NATOR_STEP_REGRESSION").ok_or("Defina NATOR_STEP_REGRESSION")?;
    let mut session = Session::new(Document::new("Módulo importado")?)?;
    let effect = session.execute(Command::ImportStep {
        path: path.into(),
        destination: None,
        name: "Módulo".into(),
    })?;
    let document = session.document();
    let roots = document
        .instances()
        .iter()
        .filter(|i| i.parent.is_none())
        .map(|i| i.id)
        .collect::<Vec<_>>();
    println!(
        "Efeito: {effect:?}; peças={}, instâncias={}, raízes={}, corpos={}",
        document.parts().len(),
        document.instances().len(),
        roots.len(),
        document
            .parts()
            .iter()
            .map(|p| p.bodies.len())
            .sum::<usize>()
    );
    for root in &roots {
        let instance = document.instance(*root).ok_or("Raiz")?;
        println!("Raiz {root:?}: {}", instance.name);
    }
    assert_eq!(roots.len(), 1, "O módulo deve ser uma montagem única");
    let root = roots[0];
    if let nator_commands::Effect::Imported { part, body } = effect {
        let occurrence = document
            .occurrences()?
            .into_iter()
            .find(|o| {
                document
                    .instance(o.instance)
                    .is_some_and(|i| i.part == part)
            })
            .ok_or("Ocorrência importada")?;
        let record = document.body(body).ok_or("Corpo importado")?;
        let vertices = record
            .geometry
            .mesh()
            .triangle_points(
                *record
                    .geometry
                    .mesh()
                    .triangles()
                    .first()
                    .ok_or("Triângulo")?,
            )
            .map(|p| occurrence.placement.apply(record.placement.apply(p)));
        let point =
            std::array::from_fn(|i| (vertices[0][i] + vertices[1][i] + vertices[2][i]) / 3.);
        let pick = nator_document::assembly::Pick {
            instance: occurrence.instance,
            part,
            body,
            triangle: 0,
            point,
            normal: [0., 0., 1.],
            vertices,
            placement: occurrence.placement,
        };
        assert_eq!(document.occurrence_at(&pick)?.path.first(), Some(&root));
    }
    let before = document.occurrences()?;
    assert!(before.len() > 1, "O ensaio deve incluir subpeças");
    let locals = document
        .instances()
        .iter()
        .map(|i| (i.id, i.placement))
        .collect::<Vec<_>>();
    let initial = document.instance(root).ok_or("Raiz")?.placement;
    let delta = Placement::new([13., -7., 4.], [0.1, -0.2, 0.4])?;
    session.execute(Command::MoveInstance {
        id: root,
        placement: delta.compose(initial)?,
    })?;
    let after = session.document().occurrences()?;
    for old in before {
        let new = after
            .iter()
            .find(|o| o.path == old.path)
            .ok_or("Ocorrência")?;
        let expected = delta.compose(old.placement)?;
        assert!(
            nator_geometry::norm(nator_geometry::sub(
                new.placement.apply([2., 3., 5.]),
                expected.apply([2., 3., 5.])
            )) < 1e-7
        );
    }
    for (id, placement) in locals {
        if id != root {
            assert_eq!(
                session.document().instance(id).ok_or("Filho")?.placement,
                placement
            );
        }
    }
    assert!(session.undo());
    assert!(session.redo());
    Ok(())
}
