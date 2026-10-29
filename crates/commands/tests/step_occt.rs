//! Intercâmbio STEP com o núcleo real: nomes UTF-8, cores, hierarquia compartilhada,
//! posições e preservação autocontida após salvar, reabrir e exportar novamente.
#![cfg(feature = "occt")]
use nator_commands::{
    Command, Session,
    export::{StepScope, step_scene},
    measure,
};
use nator_document::{Document, MeshAsset};
use nator_geometry::{Placement, Point3, RgbColor, norm, sub};
use nator_kernel::{Solid, StepAssembly, StepBody, StepOccurrence, StepPart};
use std::{error::Error, path::Path, sync::Arc};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const BLUE: RgbColor = RgbColor::new([20, 180, 230]);
const RED: RgbColor = RgbColor::new([230, 30, 40]);

/// Arquivos de terceiros ficam fora do repositório. O ensaio optativo permite
/// repetir uma falha real, incluindo persistência autocontida e histórico.
#[test]
#[ignore = "Requer um STEP externo indicado por NATOR_STEP_PATH."]
fn external_step_preserves_all_solids_after_import_save_and_reopen() -> Result<()> {
    let path = std::path::PathBuf::from(
        std::env::var_os("NATOR_STEP_PATH")
            .ok_or("Informe NATOR_STEP_PATH para o ensaio externo.")?,
    );
    let start = std::time::Instant::now();
    let raw = StepAssembly::read(&path)?;
    let expected_parts = raw.parts.len();
    let expected_instances = raw.instances.len();
    let expected_bodies = raw.parts.iter().map(|p| p.bodies.len()).sum::<usize>();
    let expected_volumes = raw
        .parts
        .iter()
        .flat_map(|p| &p.bodies)
        .map(|b| b.solid.volume())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(raw);
    let mut session = import(&path)?;
    let document = session.document();
    assert_eq!(document.parts().len(), expected_parts);
    assert_eq!(document.instances().len(), expected_instances);
    let bodies = document
        .parts()
        .iter()
        .flat_map(|p| &p.bodies)
        .collect::<Vec<_>>();
    assert_eq!(bodies.len(), expected_bodies);
    for (body, expected_volume) in bodies.iter().zip(expected_volumes) {
        assert!(body.brep.is_some());
        let measured = measure::body_properties(document, body.id)?;
        assert!(measured.exact);
        assert!((measured.volume_mm3 - expected_volume).abs() <= 1e-8 * expected_volume.max(1.));
        let faces = body
            .geometry
            .triangle_faces()
            .ok_or("Mapeamento de faces ausente")?;
        assert_eq!(faces.len(), body.geometry.mesh().triangles().len());
        assert!(faces.iter().all(|f| (*f as usize) < body.face_names.len()));
    }
    let mesh = document.scene_mesh()?.ok_or("Cena importada ausente")?;
    let before = properties(document)?;
    let temp = tempfile::tempdir()?;
    // Saída opcional permite conservar o projeto de diagnóstico sem incorporar
    // o modelo externo aos testes versionados. Um arquivo existente é preservado.
    let project = std::env::var_os("NATOR_STEP_PROJECT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temp.path().join("importado.3dnator"));
    if project.exists() {
        return Err("O projeto de diagnóstico já existe; escolha outra saída.".into());
    }
    nator_project_io::write_project(&project, document)?;
    let reopened = nator_project_io::read_project(&project)?;
    reopened.validate()?;
    let after = properties(&reopened)?;
    assert_eq!(before.len(), after.len());
    for ((point, volume, color), (restored, restored_volume, restored_color)) in
        before.iter().zip(after)
    {
        assert!(norm(sub(*point, restored)) < 1e-8);
        assert!((volume - restored_volume).abs() <= 1e-8 * volume.max(1.));
        assert_eq!(*color, restored_color);
    }
    assert!(session.undo());
    assert!(session.document().parts().is_empty());
    assert!(session.redo());
    assert_eq!(
        session
            .document()
            .parts()
            .iter()
            .map(|p| p.bodies.len())
            .sum::<usize>(),
        expected_bodies
    );
    eprintln!(
        "STEP externo aprovado: {expected_parts} definições, {expected_instances} instâncias, {expected_bodies} corpos, {} triângulos, limites {:?}, {:?}.",
        mesh.triangles().len(),
        mesh.bounds(),
        start.elapsed()
    );
    Ok(())
}

fn fixture() -> Result<StepAssembly> {
    Ok(StepAssembly {
        parts: vec![
            StepPart {
                name: "Montagem de aço".into(),
                bodies: vec![],
            },
            StepPart {
                name: "Subconjunto".into(),
                bodies: vec![],
            },
            StepPart {
                name: "Peça compartilhada".into(),
                bodies: vec![StepBody {
                    name: "Bloco azul".into(),
                    solid: Solid::cuboid([0.; 3], [10., 20., 30.])?,
                    color: Some(BLUE),
                    face_colors: Vec::new(),
                }],
            },
        ],
        instances: vec![
            StepOccurrence {
                part: 0,
                parent: None,
                name: "Raiz rotacionada".into(),
                placement: Placement::new([100., -50., 20.], [0.2, -0.3, 0.4])?,
                color: None,
            },
            StepOccurrence {
                part: 1,
                parent: Some(0),
                name: "Subconjunto posicionado".into(),
                placement: Placement::new([40., 5., -10.], [0.3, 0.1, -0.2])?,
                color: None,
            },
            StepOccurrence {
                part: 2,
                parent: Some(1),
                name: "Ocorrência vermelha".into(),
                placement: Placement::new([25., 0., 10.], [-0.2, 0.4, 0.1])?,
                color: Some(RED),
            },
            StepOccurrence {
                part: 2,
                parent: Some(1),
                name: "Ocorrência azul".into(),
                placement: Placement::new([-25., 15., 10.], [0.1, -0.5, 0.3])?,
                color: None,
            },
            StepOccurrence {
                part: 2,
                parent: None,
                name: "Segunda raiz".into(),
                placement: Placement::new([-80., 70., 5.], [0.4, 0.2, -0.3])?,
                color: None,
            },
        ],
    })
}
fn import(path: &Path) -> Result<Session> {
    let mut session = Session::new(Document::new("STEP")?)?;
    session.execute(Command::ImportStep {
        path: path.into(),
        destination: None,
        name: "Arquivo de ensaio".into(),
    })?;
    Ok(session)
}
fn properties(doc: &Document) -> Result<Vec<(Point3, f64, Option<RgbColor>)>> {
    let mut out = Vec::new();
    for occurrence in doc.occurrences()? {
        let instance = doc
            .instance(occurrence.instance)
            .ok_or("Instância ausente")?;
        let part = doc.part(instance.part).ok_or("Peça ausente")?;
        let color = occurrence
            .path
            .iter()
            .filter_map(|id| doc.instance(*id).and_then(|i| i.color))
            .next_back();
        for body in part.bodies.iter().filter(|b| b.visible) {
            let p = measure::body_properties(doc, body.id)?;
            out.push((
                occurrence.placement.apply(p.center_mm),
                p.volume_mm3,
                color.or(body.color),
            ));
        }
    }
    out.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]));
    Ok(out)
}
#[test]
fn step_round_trip_preserves_shared_parts_colors_and_world_positions() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = temp.path().join("montagem.step");
    let fixture = fixture()?;
    fixture.write(&source)?;
    let raw = StepAssembly::read(&source)?;
    let leaf = raw
        .parts
        .iter()
        .position(|p| p.name == "Peça compartilhada")
        .ok_or_else(|| {
            format!(
                "Nome UTF-8 perdido: {:?}",
                raw.parts.iter().map(|p| &p.name).collect::<Vec<_>>()
            )
        })?;
    assert_eq!(raw.parts[leaf].bodies[0].color, Some(BLUE));
    assert_eq!(raw.instances.iter().filter(|i| i.part == leaf).count(), 3);
    assert!(raw.instances.iter().any(|i| i.color == Some(RED)));
    let mut imported = import(&source)?;
    let doc = imported.document();
    let actual = properties(doc)?;
    assert_eq!(actual.len(), 3);
    let p = &fixture.instances;
    let parent = p[0].placement.compose(p[1].placement)?;
    let mut expected = [
        parent.compose(p[2].placement)?.apply([5., 10., 15.]),
        parent.compose(p[3].placement)?.apply([5., 10., 15.]),
        p[4].placement.apply([5., 10., 15.]),
    ];
    expected.sort_by(|a, b| a[0].total_cmp(&b[0]));
    for (p, (q, v, _)) in expected.iter().zip(&actual) {
        assert!(norm(sub(*p, *q)) < 1e-7, "{p:?} != {q:?}");
        assert!((v - 6000.).abs() < 1e-6);
    }
    assert_eq!(actual.iter().filter(|(_, _, c)| *c == Some(RED)).count(), 1);
    let scene = doc.scene()?.ok_or("Cena ausente")?;
    assert!(scene.colors.contains(&RED) && scene.colors.contains(&BLUE));
    let project = temp.path().join("montagem.3dnator");
    nator_project_io::write_project(&project, doc)?;
    std::fs::remove_file(&source)?;
    let reopened = nator_project_io::read_project(&project)?;
    let output = temp.path().join("exportado.step");
    nator_project_io::export_step(&output, |path| {
        step_scene(&reopened, StepScope::Assembly)
            .map_err(|e| e.to_string())?
            .write(path)
            .map_err(|e| e.to_string())
    })?;
    let again = import(&output)?;
    let restored = properties(again.document())?;
    assert_eq!(actual.len(), restored.len());
    for ((p, v, c), (q, w, d)) in actual.iter().zip(restored) {
        assert!(norm(sub(*p, q)) < 1e-7);
        assert!((v - w).abs() < 1e-6);
        assert_eq!(*c, d);
    }
    assert!(imported.undo());
    assert!(imported.document().parts().is_empty());
    assert!(imported.redo());
    assert_eq!(properties(imported.document())?.len(), 3);
    Ok(())
}
#[test]
fn step_import_is_atomic_and_can_insert_an_independent_subassembly() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = temp.path().join("montagem.step");
    fixture()?.write(&source)?;
    let mut doc = Document::new("Construção")?;
    let destination = doc.create_part("Peça ativa".into())?;
    let mut session = Session::new(doc)?;
    session.execute(Command::ImportStep {
        path: source,
        destination: Some(destination),
        name: "Subconjunto importado".into(),
    })?;
    assert!(
        session
            .document()
            .instances()
            .iter()
            .filter(|i| i.parent.is_none())
            .all(|i| i.part == destination)
    );
    assert_eq!(properties(session.document())?.len(), 3);
    assert!(session.undo());
    let revision = session.revision();
    let broken = temp.path().join("invalido.step");
    std::fs::write(&broken, b"ISO-10303-21; quebrado")?;
    assert!(
        session
            .execute(Command::ImportStep {
                path: broken,
                destination: Some(destination),
                name: "Inválido".into()
            })
            .is_err()
    );
    assert_eq!(session.revision(), revision);
    assert_eq!(session.document().parts().len(), 1);
    assert!(session.redo());
    Ok(())
}
#[test]
fn step_export_rejects_visible_meshes_and_cycles_before_writing() -> Result<()> {
    let mut doc = Document::new("Malha")?;
    doc.add_mesh(
        None,
        "Referência".into(),
        Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
    )?;
    assert!(step_scene(&doc, StepScope::Assembly).is_err());
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("ciclo.step");
    let mut scene = fixture()?;
    scene.instances.push(StepOccurrence {
        part: 0,
        parent: Some(1),
        name: "Ciclo".into(),
        placement: Placement::default(),
        color: None,
    });
    assert!(scene.write(&path).is_err());
    assert!(!path.exists());
    let mut scene = fixture()?;
    scene.parts[2].bodies[0].face_colors = vec![Some(RED)];
    assert!(scene.write(&path).is_err());
    assert!(
        !path.exists(),
        "Cores incompatíveis devem falhar antes de gravar"
    );
    Ok(())
}
