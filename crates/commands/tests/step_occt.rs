//! Intercâmbio STEP com o núcleo real: nomes UTF-8, cores, hierarquia compartilhada,
//! posições e preservação autocontida após salvar, reabrir e exportar novamente.
#![cfg(feature = "occt")]
use nator_commands::{
    Command, PreparedCommand, Session,
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
        .map(|b| (!b.surface).then(|| b.solid.volume()).transpose())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(raw);
    let mut session = Session::new(Document::new("STEP")?)?;
    let prepared = PreparedCommand::new(
        session.document().clone(),
        session.revision(),
        Command::ImportStep {
            path: path.clone(),
            destination: None,
            name: "Arquivo de ensaio".into(),
        },
    )?;
    // Sólidos reprovados pelo BRepCheck entram como lidos e são informados à interface.
    eprintln!("Corpos não verificados: {:?}", prepared.unverified_bodies());
    session.commit(prepared)?;
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
        // Superfícies abertas entram como malha, sem volume nem faces de origem.
        let Some(expected_volume) = expected_volume else {
            assert!(body.brep.is_none());
            continue;
        };
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
                    surface: false,
                    verified: true,
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
/// Quadrado plano de 10 mm como superfície aberta, na estrutura exportada pelo KiCad.
/// Devolve as entidades e o identificador do modelo, a partir de `first`.
fn open_surface(first: usize) -> (String, usize) {
    let mut lines = vec![];
    let mut next = first;
    let mut add = |text: String| {
        lines.push((next, text));
        next += 1;
        next - 1
    };
    let corners = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
    let vertices = corners.map(|[x, y]| {
        let point = add(format!("CARTESIAN_POINT('',({x:.1},{y:.1},0.))"));
        (add(format!("VERTEX_POINT('',#{point})")), point)
    });
    let mut oriented = vec![];
    for side in 0..4 {
        let (start, point) = vertices[side];
        let end = vertices[(side + 1) % 4].0;
        let [a, b] = [corners[side], corners[(side + 1) % 4]];
        let direction = add(format!(
            "DIRECTION('',({:.1},{:.1},0.))",
            (b[0] - a[0]) / 10.,
            (b[1] - a[1]) / 10.
        ));
        let vector = add(format!("VECTOR('',#{direction},1.)"));
        let line = add(format!("LINE('',#{point},#{vector})"));
        let edge = add(format!("EDGE_CURVE('',#{start},#{end},#{line},.T.)"));
        oriented.push(format!(
            "#{}",
            add(format!("ORIENTED_EDGE('',*,*,#{edge},.T.)"))
        ));
    }
    let list = oriented.join(",");
    let edge_loop = add(format!("EDGE_LOOP('',({list}))"));
    let bound = add(format!("FACE_BOUND('',#{edge_loop},.T.)"));
    let origin = add("CARTESIAN_POINT('',(0.,0.,0.))".into());
    let z = add("DIRECTION('',(0.,0.,1.))".into());
    let x = add("DIRECTION('',(1.,0.,0.))".into());
    let axis = add(format!("AXIS2_PLACEMENT_3D('',#{origin},#{z},#{x})"));
    let plane = add(format!("PLANE('',#{axis})"));
    let face = add(format!("ADVANCED_FACE('',(#{bound}),#{plane},.T.)"));
    let shell = add(format!("OPEN_SHELL('',(#{face}))"));
    let model = add(format!("SHELL_BASED_SURFACE_MODEL('',(#{shell}))"));
    let text = lines
        .into_iter()
        .map(|(id, line)| format!("#{id} = {line};\n"))
        .collect();
    (text, model)
}

/// Grava duas peças sólidas e troca a representação da primeira por superfícies, sem
/// sólido, e acrescenta superfícies à segunda: os dois formatos que o KiCad gera.
fn surface_step(path: &Path) -> Result<()> {
    let part = |name: &str, solid| StepPart {
        name: name.into(),
        bodies: vec![StepBody {
            name: name.into(),
            solid,
            color: None,
            face_colors: Vec::new(),
            surface: false,
            verified: true,
        }],
    };
    let root = |index, name: &str| -> Result<StepOccurrence> {
        Ok(StepOccurrence {
            part: index,
            parent: None,
            name: name.into(),
            placement: Placement::default(),
            color: None,
        })
    };
    StepAssembly {
        parts: vec![
            part("Serigrafia", Solid::cuboid([0.; 3], [1., 1., 1.])?),
            part("Placa", Solid::cuboid([0.; 3], [20., 20., 2.])?),
        ],
        instances: vec![root(0, "Serigrafia")?, root(1, "Placa")?],
    }
    .write(path)?;
    let mut text = std::fs::read_to_string(path)?;
    let mut items = vec![];
    let mut from = 0;
    const LIST: &str = "ADVANCED_BREP_SHAPE_REPRESENTATION('',(";
    while let Some(found) = text[from..].find(LIST) {
        items.push(from + found + LIST.len());
        from += found + LIST.len();
    }
    assert_eq!(items.len(), 2, "Representações sólidas da montagem");
    let (first, model) = open_surface(900_000);
    let (second, mixed) = open_surface(900_100);
    // De trás para frente: a primeira inserção não desloca a posição da outra.
    text.insert_str(items[1], &format!("#{mixed},"));
    let list = items[0]..text[items[0]..].find(')').ok_or("Lista")? + items[0];
    let kept = text[list.clone()]
        .split(',')
        .next()
        .ok_or("Item")?
        .to_owned();
    text.replace_range(list, &format!("{kept},#{model}"));
    let end = text.rfind("ENDSEC;").ok_or("Fim dos dados")?;
    text.insert_str(end, &format!("{first}{second}"));
    std::fs::write(path, text)?;
    Ok(())
}

#[test]
fn open_surfaces_enter_as_meshes_without_blocking_the_solids() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("kicad.step");
    surface_step(&path)?;
    let mut session = import(&path)?;
    let bodies = |doc: &Document| -> Vec<(String, bool, f64)> {
        doc.parts()
            .iter()
            .flat_map(|part| part.bodies.iter())
            .map(|body| {
                let size = body.geometry.mesh().bounds().size();
                (body.name.clone(), body.brep.is_some(), size[0])
            })
            .collect()
    };
    let found = bodies(session.document());
    // Peça só de superfícies, sólido da placa e superfície da peça mista.
    assert_eq!(found.len(), 3, "{found:?}");
    assert_eq!(found.iter().filter(|(_, solid, _)| *solid).count(), 1);
    assert!(found.iter().any(|(name, solid, size)| {
        name == "Serigrafia" && !solid && (size - 10.).abs() < 1e-6
    }));
    assert!(
        found
            .iter()
            .any(|(name, solid, size)| { name == "Placa" && !solid && (size - 10.).abs() < 1e-6 })
    );
    assert!(
        found
            .iter()
            .any(|(name, solid, size)| { name == "Placa" && *solid && (size - 20.).abs() < 1e-6 })
    );

    // O arquivo de origem não é mais necessário depois de salvar.
    let project = temp.path().join("modulo.3dnator");
    nator_project_io::write_project(&project, session.document())?;
    std::fs::remove_file(&path)?;
    let reopened = nator_project_io::read_project(&project)?;
    assert_eq!(bodies(&reopened), found);

    // A exportação STEP explica a malha visível e aceita o conjunto com ela oculta.
    assert!(step_scene(session.document(), StepScope::Assembly).is_err());
    let meshes = session
        .document()
        .parts()
        .iter()
        .flat_map(|part| part.bodies.iter())
        .filter(|body| body.brep.is_none())
        .map(|body| body.id)
        .collect::<Vec<_>>();
    for id in meshes {
        session.execute(Command::SetBodyVisibility { id, visible: false })?;
    }
    assert!(step_scene(session.document(), StepScope::Assembly).is_ok());
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
/// Importar para a peça em edição achata a hierarquia: cada ocorrência vira um corpo da
/// peça, na posição do mundo, sem subpeças; a cor da ocorrência vale sobre a do sólido.
#[test]
fn step_bodies_enter_the_edited_part_in_world_position_without_subparts() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = temp.path().join("montagem.step");
    let fixture = fixture()?;
    fixture.write(&source)?;
    let mut doc = Document::new("Peça")?;
    let part = doc.create_part("Destino".into())?;
    let mut session = Session::new(doc)?;
    session.execute(Command::ImportStepBodies {
        path: source.clone(),
        part,
    })?;
    let doc = session.document();
    assert_eq!(doc.parts().len(), 1);
    assert_eq!(doc.instances().len(), 1);
    let bodies = &doc.part(part).ok_or("Peça")?.bodies;
    assert_eq!(bodies.len(), 3);
    let mut actual = Vec::new();
    for body in bodies {
        assert!(body.brep.is_some());
        let p = measure::body_properties(doc, body.id)?;
        assert!((p.volume_mm3 - 6000.).abs() < 1e-6);
        actual.push((p.center_mm, body.color, body.name.clone()));
    }
    actual.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]));
    let p = &fixture.instances;
    let parent = p[0].placement.compose(p[1].placement)?;
    let mut expected = [
        parent.compose(p[2].placement)?.apply([5., 10., 15.]),
        parent.compose(p[3].placement)?.apply([5., 10., 15.]),
        p[4].placement.apply([5., 10., 15.]),
    ];
    expected.sort_by(|a, b| a[0].total_cmp(&b[0]));
    for (p, (q, ..)) in expected.iter().zip(&actual) {
        assert!(norm(sub(*p, *q)) < 1e-7, "{p:?} != {q:?}");
    }
    let red = actual
        .iter()
        .filter(|(_, c, _)| *c == Some(RED))
        .collect::<Vec<_>>();
    assert_eq!(red.len(), 1);
    assert_eq!(red[0].2, "Ocorrência vermelha");
    // Operações usam a posição do corpo: dividir no centro deixa metade do volume.
    let (center, _, _) = actual[0];
    let base = bodies
        .iter()
        .find(|b| {
            measure::body_properties(doc, b.id).is_ok_and(|p| norm(sub(p.center_mm, center)) < 1e-7)
        })
        .ok_or("Corpo")?
        .id;
    session.execute(Command::SplitBody {
        part,
        name: "Metade".into(),
        operation: nator_document::features::Operation::Split {
            base,
            origin: center,
            normal: [0., 0., 1.],
            cutting_plane: None,
            reverse: false,
        },
    })?;
    let half = session
        .document()
        .part(part)
        .ok_or("Peça")?
        .features
        .first()
        .ok_or("Divisão")?
        .output;
    let volume = measure::body_properties(session.document(), half)?.volume_mm3;
    assert!((volume - 3000.).abs() < 1e-6, "{volume}");
    assert!(session.undo());
    assert!(session.undo());
    assert!(
        session
            .document()
            .part(part)
            .ok_or("Peça")?
            .bodies
            .is_empty()
    );
    // Uma peça de referência não recebe corpos.
    session.execute(Command::SetPartReference {
        part,
        reference: true,
    })?;
    assert!(
        session
            .execute(Command::ImportStepBodies { path: source, part })
            .is_err()
    );
    Ok(())
}
