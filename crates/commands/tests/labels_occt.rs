//! Histórico, persistência independente da fonte e cores STEP dos rótulos.
#![cfg(feature = "occt")]
use nator_commands::{
    Command, Effect, Session,
    export::{StepScope, step_scene},
    painting::Target,
};
use nator_document::{
    BodyId, Document, MeshAsset,
    features::{FaceRef, LabelMode, LabelSpec, Operation},
};
use nator_geometry::RgbColor;
use nator_kernel::{FaceKind, Solid, StepAssembly};
use std::sync::Arc;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const RED: RgbColor = RgbColor::new([220, 30, 50]);
const BLUE: RgbColor = RgbColor::new([20, 70, 210]);
fn solid(session: &Session, body: BodyId) -> Result<Solid> {
    let record = session.document().body(body).ok_or("Corpo")?;
    Ok(Solid::from_brep(
        record.brep.as_ref().ok_or("B-rep")?,
        Some(&record.face_names),
    )?)
}
#[test]
fn label_history_roundtrip_colors_and_step_survive_without_the_font() -> Result<()> {
    for curved in [false, true] {
        let mut source = if curved {
            Solid::cylinder([0.; 3], 10., 20.)?
        } else {
            Solid::cuboid([-20., -20., 0.], [40., 40., 5.])?
        };
        source.name_faces_by_index("base")?;
        let face = (0..source.face_count()?)
            .find(|i| {
                source.face_measure(*i).is_ok_and(|m| {
                    if curved {
                        matches!(m.kind, FaceKind::Cylinder { .. })
                    } else {
                        matches!(m.kind, FaceKind::Plane{normal,..} if normal[2]>0.9)
                    }
                })
            })
            .ok_or("Face")?;
        let (mesh, faces) = source.tessellate_with_faces(0.05, 0.3)?;
        let mut document = Document::new("Rótulos")?;
        let (part, base) = document.add_mesh(
            None,
            "Base".into(),
            Arc::new(MeshAsset::with_faces(mesh, faces)?),
        )?;
        let body = document.body_mut(base)?;
        body.brep = Some(Arc::new(source.to_brep()?));
        body.face_names = Arc::new(source.face_names().to_vec());
        let mut session = Session::new(document)?;
        session.execute(Command::PaintSurfaces {
            targets: vec![Target::Body(base)],
            color: BLUE,
        })?;
        let spec = LabelSpec {
            text: "O".into(),
            size: "3 mm".into(),
            position: if curved { [10., 0., 10.] } else { [0., 0., 5.] },
            color: RED,
            ..Default::default()
        };
        let Effect::Imported { body: label, .. } = session.execute(Command::AddFeature {
            part,
            name: "Rótulo".into(),
            operation: Operation::Label {
                base,
                face: FaceRef::Named(source.face_names()[face].clone()),
                spec,
            },
        })?
        else {
            return Err("Rótulo".into());
        };
        assert!((solid(&session, label)?.volume()? - source.volume()?).abs() < 1e-5);
        assert!(
            session
                .document()
                .body(label)
                .ok_or("Rótulo")?
                .intrinsic_colors
                .contains(&Some(RED))
        );
        assert!(session.undo());
        assert!(session.document().body(label).is_none());
        assert!(session.redo());
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("rotulo.3dnator");
        nator_project_io::write_project(&path, session.document())?;
        let mut reopened = Session::new(nator_project_io::read_project(&path)?)?;
        let mut operation = reopened
            .document()
            .part(part)
            .ok_or("Peça")?
            .features
            .last()
            .ok_or("Operação")?
            .operation
            .clone();
        let Operation::Label { spec, .. } = &mut operation else {
            return Err("Operação".into());
        };
        assert!(!spec.outline.is_empty());
        assert!(spec.uv.is_some());
        // Alterar posição, cor ou tamanho não consulta fontes. O contorno continua
        // sendo válido mesmo se esta família não existir na máquina de destino.
        spec.font_family = "Fonte inexistente nesta máquina".into();
        spec.outline_key = spec.shape_key();
        spec.size = "3.2 mm".into();
        reopened.execute(Command::EditFeature {
            part,
            output: label,
            operation: operation.clone(),
        })?;
        for mode in [LabelMode::Raised, LabelMode::Engraved, LabelMode::Surface] {
            if let Operation::Label { spec, .. } = &mut operation {
                spec.mode = mode;
            }
            reopened.execute(Command::EditFeature {
                part,
                output: label,
                operation: operation.clone(),
            })?;
            let delta = solid(&reopened, label)?.volume()? - source.volume()?;
            match mode {
                LabelMode::Raised => assert!(delta > 0.),
                LabelMode::Engraved => assert!(delta < 0.),
                LabelMode::Surface => assert!(delta.abs() < 1e-5),
            }
        }
        let step = directory.path().join("rotulo.step");
        step_scene(reopened.document(), StepScope::Body(label))?.write(&step)?;
        let read = StepAssembly::read(&step)?;
        let bodies = read
            .parts
            .iter()
            .flat_map(|p| &p.bodies)
            .collect::<Vec<_>>();
        assert_eq!(bodies.len(), 1);
        assert!(bodies[0].face_colors.contains(&Some(RED)));
        assert_eq!(bodies[0].color, Some(BLUE));
        assert!((bodies[0].solid.volume()? - source.volume()?).abs() < 1e-5);
        // Uma face pintada mantém as cores próprias das outras faces. O cache
        // guarda a associação ao B-rep original também após salvar/reabrir.
        let bottom = (0..source.face_count()?)
            .find(|i| {
                source.face_measure(*i).is_ok_and(
                    |m| matches!(m.kind, FaceKind::Plane {normal, ..} if normal[2] < -0.9),
                )
            })
            .ok_or("Base inferior")?;
        let green = RgbColor::new([30, 180, 60]);
        reopened.execute(Command::PaintSurfaces {
            targets: vec![Target::Face {
                body: label,
                face: FaceRef::Named(source.face_names()[bottom].clone()),
                boundaries: vec![],
            }],
            color: green,
        })?;
        let cached_path = directory.path().join("rotulo-pintado.3dnator");
        nator_project_io::write_project(&cached_path, reopened.document())?;
        reopened = Session::new(nator_project_io::read_project(&cached_path)?)?;
        assert!(
            reopened
                .document()
                .body_appearance(label)
                .and_then(|a| a.geometry.as_ref())
                .and_then(|g| g.triangle_faces())
                .is_some()
        );
        let scene = step_scene(reopened.document(), StepScope::Body(label))?;
        assert!(scene.parts[0].bodies[0].face_colors.contains(&Some(RED)));
        assert!(scene.parts[0].bodies[0].face_colors.contains(&Some(green)));
        // Pintura explícita posterior prevalece sobre a cor do rótulo.
        reopened.execute(Command::PaintSurfaces {
            targets: vec![Target::Body(label)],
            color: BLUE,
        })?;
        let painted = step_scene(reopened.document(), StepScope::Body(label))?;
        assert!(
            painted.parts[0].bodies[0]
                .face_colors
                .iter()
                .all(|c| *c == Some(BLUE))
        );
        let revision = reopened.revision();
        if let Operation::Label { spec, .. } = &mut operation {
            spec.position = [10000.; 3];
            spec.uv = None;
        }
        assert!(
            reopened
                .execute(Command::EditFeature {
                    part,
                    output: label,
                    operation
                })
                .is_err()
        );
        assert_eq!(reopened.revision(), revision);
    }
    Ok(())
}
