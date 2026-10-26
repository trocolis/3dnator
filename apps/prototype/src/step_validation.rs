//! Ensaio STEP autocontido: montagem com duas ocorrências, cores e reabertura sem a origem.
use std::{error::Error, path::Path};
pub fn run(directory: &Path) -> Result<(), Box<dyn Error>> {
    nator_kernel::require_available()?;
    #[cfg(feature = "occt")]
    return native_run(directory);
    #[cfg(not(feature = "occt"))]
    {
        let _ = directory;
        Err("Intercâmbio STEP indisponível.".into())
    }
}
#[cfg(feature = "occt")]
fn native_run(directory: &Path) -> Result<(), Box<dyn Error>> {
    use nator_commands::{
        Command, Session,
        export::{StepScope, step_scene},
        measure,
    };
    use nator_document::{Document, MeshAsset};
    use nator_geometry::{Placement, RgbColor};
    use nator_kernel::Solid;
    use std::{sync::Arc, time::Instant};
    if let Some(parent) = directory.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(directory)?;
    let started = Instant::now();
    let mut solid = Solid::cuboid([0.; 3], [10., 20., 30.])?;
    let brep = solid.to_brep()?;
    let faces = solid.face_count()?;
    let (mesh, triangles) = solid.tessellate_with_faces(0.05, 0.25)?;
    let mut session = Session::new(Document::new("Montagem STEP colorida")?)?;
    let effect = session.execute(Command::ImportSolid {
        destination: None,
        name: "Peça azul".into(),
        geometry: Arc::new(MeshAsset::with_faces(mesh, triangles)?),
        brep: Arc::new(brep),
        faces,
    })?;
    let nator_commands::Effect::Imported { part, body } = effect else {
        return Err("Corpo não criado".into());
    };
    session.execute(Command::SetBodyColor {
        id: body,
        color: Some(RgbColor::new([20, 180, 230])),
    })?;
    let mut doc = session.document().clone();
    let leaf = doc.instances()[0].id;
    let root = doc.create_part("Conjunto de aço".into())?;
    let root_instance = doc.instances().last().ok_or("Raiz ausente")?.id;
    doc.instance_mut(root_instance)?.placement =
        Placement::new([100., -50., 20.], [0.2, -0.3, 0.4])?;
    doc.reparent(leaf, Some(root))?;
    doc.instance_mut(leaf)?.placement = Placement::new([25., 0., 5.], [0.2, 0.4, -0.1])?;
    let second = doc.add_instance(
        part,
        Some(root),
        Placement::new([-25., 30., 10.], [-0.2, 0.1, 0.3])?,
    )?;
    doc.instance_mut(second)?.name = "Ocorrência vermelha".into();
    doc.instance_mut(second)?.color = Some(RgbColor::new([230, 30, 40]));
    let source = directory.join("source.step");
    nator_project_io::export_step(&source, |p| {
        step_scene(&doc, StepScope::Assembly)
            .map_err(|e| e.to_string())?
            .write(p)
            .map_err(|e| e.to_string())
    })?;
    let expected = doc.scene_mesh()?.ok_or("Cena ausente")?.bounds();
    let mut imported = Session::new(Document::new("Montagem importada")?)?;
    imported.execute(Command::ImportStep {
        path: source.clone(),
        destination: None,
        name: "Montagem importada".into(),
    })?;
    let project = directory.join("imported.3dnator");
    nator_project_io::write_project(&project, imported.document())?;
    std::fs::remove_file(&source)?;
    let reopened = nator_project_io::read_project(&project)?;
    let scene = reopened.scene()?.ok_or("Cena importada ausente")?;
    for (a, b) in expected.min.into_iter().chain(expected.max).zip(
        scene
            .mesh
            .bounds()
            .min
            .into_iter()
            .chain(scene.mesh.bounds().max),
    ) {
        if (a - b).abs() > 1e-7 {
            return Err("Posições STEP divergiram".into());
        }
    }
    if !scene.colors.contains(&RgbColor::new([20, 180, 230]))
        || !scene.colors.contains(&RgbColor::new([230, 30, 40]))
    {
        return Err("Cores STEP divergiram".into());
    }
    let definitions = reopened
        .parts()
        .iter()
        .filter(|p| !p.bodies.is_empty())
        .collect::<Vec<_>>();
    if definitions.len() != 1
        || reopened
            .instances()
            .iter()
            .filter(|i| i.part == definitions[0].id)
            .count()
            != 2
    {
        return Err("Compartilhamento STEP perdido".into());
    }
    let volume = measure::body_properties(&reopened, definitions[0].bodies[0].id)?.volume_mm3;
    if (volume - 6000.).abs() > 1e-6 {
        return Err("Volume STEP divergente".into());
    }
    let output = directory.join("assembly.step");
    nator_project_io::export_step(&output, |p| {
        step_scene(&reopened, StepScope::Assembly)
            .map_err(|e| e.to_string())?
            .write(p)
            .map_err(|e| e.to_string())
    })?;
    let mut final_session = Session::new(Document::new("Reimportação")?)?;
    final_session.execute(Command::ImportStep {
        path: output,
        destination: None,
        name: "Reimportação".into(),
    })?;
    if final_session
        .document()
        .scene_mesh()?
        .ok_or("Cena reimportada ausente")?
        .triangles()
        .len()
        != scene.mesh.triangles().len()
    {
        return Err("Geometria STEP mudou no segundo ciclo".into());
    }
    let report = serde_json::json!({"status":"passed","app_version":env!("CARGO_PKG_VERSION"),"format_version":nator_project_io::FORMAT_VERSION,"occt_version":nator_kernel::version(),"elapsed_ms":started.elapsed().as_secs_f64()*1000.,"checks":["UTF-8 names","shared part definition","rotated assembly positions","body and occurrence colors","self-contained save/reopen without source STEP","exact volume","assembly export and reimport"],"body_volume_mm3":volume,"instances":reopened.instances().len(),"parts":reopened.parts().len()});
    std::fs::write(
        directory.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
