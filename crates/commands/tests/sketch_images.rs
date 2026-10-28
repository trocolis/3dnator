//! Imagens, nomes e histórico passam pelos mesmos comandos do restante do documento.
use nator_commands::{Command, Session};
use nator_document::{Document, ElementKind, ImageId, RasterAsset, ReferenceImage};
use nator_sketch::{Plane, Sketch};
use std::sync::Arc;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn images_and_creation_counters_survive_history_renaming_deletion_and_roundtrip() -> Result<()> {
    let mut doc = Document::new("Referências")?;
    let (part, sketch) = doc.add_sketch(None, Sketch::new("Rascunho", Plane::xy()))?;
    assert_eq!(doc.part(part).ok_or("Peça ausente")?.name, "Peça 1");
    assert_eq!(
        doc.sketch(sketch).ok_or("Rascunho ausente")?.sketch.name,
        "Rascunho 1"
    );
    let mut session = Session::new(doc)?;
    session.execute(Command::RenameSketch {
        id: sketch,
        name: "Principal".into(),
    })?;
    let image = ReferenceImage {
        id: ImageId(0),
        sketch,
        name: "Imagem".into(),
        asset: Arc::new(RasterAsset {
            width: 2,
            height: 1,
            rgba: Arc::new(vec![255, 0, 0, 255, 0, 255, 0, 128]),
        }),
        origin: [3., 4.],
        size: [20., 10.],
        transparency: 0.5,
        visible: true,
    };
    session.execute(Command::AddReferenceImage { image })?;
    let id = session.document().images()[0].id;
    assert_eq!(
        session.document().image(id).ok_or("Imagem ausente")?.name,
        "Imagem 1"
    );
    let mut next = session
        .document()
        .image(id)
        .ok_or("Imagem ausente")?
        .clone();
    next.origin = [8., 9.];
    next.transparency = 1.;
    session.execute(Command::UpdateReferenceImage {
        image: next.clone(),
    })?;
    assert!(session.undo());
    assert_eq!(
        session
            .document()
            .image(id)
            .ok_or("Imagem ausente")?
            .transparency,
        0.5
    );
    assert!(session.redo());
    next.transparency = 1.1;
    let revision = session.revision();
    assert!(
        session
            .execute(Command::UpdateReferenceImage { image: next })
            .is_err()
    );
    assert_eq!(session.revision(), revision);
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("imagens.3dnator");
    nator_project_io::write_project(&path, session.document())?;
    let reopened = nator_project_io::read_project(&path)?;
    assert_eq!(
        reopened
            .image(id)
            .ok_or("Imagem ausente")?
            .asset
            .rgba
            .as_slice(),
        &[255, 0, 0, 255, 0, 255, 0, 128]
    );
    assert_eq!(reopened.image(id).ok_or("Imagem ausente")?.origin, [8., 9.]);
    assert_eq!(reopened.next_name_number(ElementKind::Sketch), 2);
    let mut session = Session::new(reopened)?;
    session.execute(Command::RemoveSketch { id: sketch })?;
    assert!(session.document().images().is_empty());
    session.execute(Command::AddSketch {
        destination: Some(part),
        sketch: Sketch::new("Rascunho", Plane::xy()),
    })?;
    assert_eq!(
        session
            .document()
            .part(part)
            .ok_or("Peça ausente")?
            .sketches[0]
            .sketch
            .name,
        "Rascunho 2"
    );
    Ok(())
}

#[test]
fn cloned_parts_remap_reference_images_and_bad_pixels_are_rejected() -> Result<()> {
    let mut doc = Document::new("Cópia")?;
    let (part, sketch) = doc.add_sketch(None, Sketch::new("Rascunho", Plane::xy()))?;
    let asset = Arc::new(RasterAsset {
        width: 1,
        height: 1,
        rgba: Arc::new(vec![255; 4]),
    });
    doc.add_image(ReferenceImage {
        id: ImageId(0),
        sketch,
        name: "Imagem".into(),
        asset: asset.clone(),
        origin: [0., 0.],
        size: [10., 10.],
        transparency: 0.5,
        visible: true,
    })?;
    let copy = doc.clone_part(part, "Cópia".into())?;
    doc.validate()?;
    assert_eq!(doc.images().len(), 2);
    assert_eq!(doc.sketch_part(doc.images()[1].sketch), Some(copy));
    assert_ne!(doc.images()[0].id, doc.images()[1].id);
    assert!(Arc::ptr_eq(&doc.images()[1].asset, &asset));
    assert!(
        RasterAsset {
            width: 8192,
            height: 8192,
            rgba: Arc::new(vec![])
        }
        .validate()
        .is_err()
    );
    assert!(
        RasterAsset {
            width: u32::MAX,
            height: u32::MAX,
            rgba: Arc::new(vec![])
        }
        .validate()
        .is_err()
    );
    Ok(())
}
