//! Ensaio reproduzível do ciclo de projeto, independente de GPU e OCCT.
use nator_commands::{Command, Effect, Session};
use nator_document::{Document, MeshAsset};
use nator_geometry::{Placement, reference_bracket};
use nator_project_io::{read_project, write_project};
use std::{error::Error, path::Path, sync::Arc, time::Instant};

pub fn run(directory: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = directory.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(directory)?;
    let start = Instant::now();
    let mut session = Session::new(Document::new("Suporte de validação")?)?;
    let source = directory.join("source.stl");
    nator_project_io::write_stl(&source, &reference_bracket()?)?;
    let asset = Arc::new(MeshAsset::new(nator_project_io::read_stl(&source, 1.)?));
    let Effect::Imported { part, body } = session.execute(Command::ImportMesh {
        destination: None,
        name: "Base".into(),
        geometry: Arc::clone(&asset),
    })?
    else {
        return Err("Importação não criou corpo.".into());
    };
    let Effect::Imported { body: second, .. } = session.execute(Command::ImportMesh {
        destination: Some(part),
        name: "Referência".into(),
        geometry: asset,
    })?
    else {
        return Err("O segundo corpo não foi criado.".into());
    };
    session.execute(Command::MoveBody {
        id: second,
        placement: Placement::new([0., 30., 0.], [0.; 3])?,
    })?;
    let instance = session.document().instances()[0].id;
    session.execute(Command::MoveInstance {
        id: instance,
        placement: Placement::new([100., 0., 0.], [0.; 3])?,
    })?;
    let path = directory.join("support.3dnator");
    write_project(&path, session.document())?;
    session.mark_saved(session.state());
    session.execute(Command::RenameBody {
        id: body,
        name: "Base modificada".into(),
    })?;
    if !session.is_dirty() || !session.undo() || session.is_dirty() || !session.redo() {
        return Err("Estado de desfazer/refazer inconsistente.".into());
    }
    let recovery = directory.join("recovery.3dnator");
    nator_project_io::write_recovery(&recovery, session.document())?;
    std::fs::remove_file(source)?;
    let loaded = read_project(&path)?;
    let recovered = read_project(&recovery)?;
    if loaded
        .body(second)
        .ok_or("Segundo corpo ausente")?
        .placement
        .translation_mm()
        != [0., 30., 0.]
    {
        return Err("A posição local do corpo não foi preservada.".into());
    }
    if loaded.parts()[0].bodies.len() != 2
        || loaded.body(body).ok_or("Corpo ausente")?.name != "Base"
        || recovered.body(body).ok_or("Corpo ausente")?.name != "Base modificada"
    {
        return Err("Projeto ou recuperação não preservou os corpos.".into());
    }
    let scene = loaded.scene_mesh()?.ok_or("Cena ausente")?;
    if scene.bounds().min != [100., 0., 0.]
        || loaded
            .body(body)
            .ok_or("Corpo ausente")?
            .geometry
            .mesh()
            .bounds()
            .min
            != [0.; 3]
    {
        return Err("Coordenadas locais foram alteradas pela montagem.".into());
    }
    nator_project_io::write_stl(
        &directory.join("body.stl"),
        loaded.body(body).ok_or("Corpo ausente")?.geometry.mesh(),
    )?;
    let report = serde_json::json!({"status":"passed","app_version":env!("CARGO_PKG_VERSION"),"format_version":nator_project_io::FORMAT_VERSION,"parts":loaded.parts().len(),"bodies":loaded.parts()[0].bodies.len(),"instances":loaded.instances().len(),"scene_bounds_mm":{"min":scene.bounds().min,"max":scene.bounds().max},"elapsed_ms":start.elapsed().as_secs_f64()*1000.,"checks":["STL import","multiple bodies in one part","local and assembly coordinates","atomic save","undo/redo and dirty state","independent recovery snapshot","reopen without source STL","STL export per body"]});
    std::fs::write(
        directory.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn inspect(path: &Path) -> Result<(), Box<dyn Error>> {
    let doc = read_project(path)?;
    let report = serde_json::json!({"format_version":nator_project_io::FORMAT_VERSION,"document_id":doc.id(),"name":doc.name(),"parts":doc.parts().iter().map(|p|serde_json::json!({"id":p.id.0,"name":p.name,"bodies":p.bodies.iter().map(|b|serde_json::json!({"id":b.id.0,"name":b.name,"body_type":"mesh","visible":b.visible,"triangles":b.geometry.mesh().triangles().len(),"translation_mm":b.placement.translation_mm(),"rotation_rad":b.placement.rotation_rad()})).collect::<Vec<_>>()})).collect::<Vec<_>>(),"instances":doc.instances().len()});
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
