//! A pintura STEP usa superfícies exatas: comparamos áreas analíticas, volume,
//! quantidade de sólidos e herança de cores após gravar e reler o arquivo XCAF.
#![cfg(feature = "occt")]
use nator_commands::{
    Command, Session,
    export::{StepScope, step_scene},
    painting::Target,
};
use nator_document::{BodyId, Document, MeshAsset, PartId};
use nator_geometry::{Placement, RgbColor};
use nator_kernel::{Solid, StepAssembly};
use nator_sketch::{Plane, Sketch};
use std::{collections::BTreeMap, error::Error, sync::Arc};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const BLUE: RgbColor = RgbColor::new([20, 80, 230]);
const RED: RgbColor = RgbColor::new([230, 20, 40]);
const GREEN: RgbColor = RgbColor::new([30, 220, 60]);
fn fixture(mut solid: Solid, placement: Placement) -> Result<(Session, PartId, BodyId)> {
    let (mesh, faces) = solid.tessellate_with_faces(0.02, 0.2)?;
    let mut doc = Document::new("Intercâmbio pintado")?;
    let (part, body) = doc.add_mesh(
        None,
        "Corpo pintado".into(),
        Arc::new(MeshAsset::with_faces(mesh, faces)?),
    )?;
    let record = doc.body_mut(body)?;
    record.brep = Some(Arc::new(solid.to_brep()?));
    // Importações podem apresentar nomes repetidos: a pintura guarda índice e
    // assinatura e não depende de resolver um nome ambíguo na exportação.
    record.face_names = Arc::new(vec!["shared:face".into(); solid.face_count()?]);
    record.placement = placement;
    record.color = Some(BLUE);
    Ok((Session::new(doc)?, part, body))
}
fn paint(s: &mut Session, placement: Placement, point: [f64; 3], color: RgbColor) -> Result<()> {
    let pick = s
        .document()
        .pick(nator_geometry::query::Ray {
            origin: placement.apply(point),
            direction: placement.apply_vector([0., 0., -1.]),
        })?
        .ok_or("Superfície ausente")?;
    let target = nator_commands::painting::pick(s.document(), &pick, false)?.target;
    s.execute(Command::PaintSurfaces {
        targets: vec![target],
        color,
    })?;
    Ok(())
}
fn colored_areas(scene: &StepAssembly) -> Result<BTreeMap<[u8; 3], f64>> {
    let mut areas = BTreeMap::new();
    for body in scene.parts.iter().flat_map(|p| &p.bodies) {
        assert_eq!(body.face_colors.len(), body.solid.face_count()?);
        for (i, color) in body.face_colors.iter().enumerate() {
            if let Some(color) = color {
                *areas.entry(color.channels()).or_default() += body.solid.face_measure(i)?.area;
            }
        }
    }
    Ok(areas)
}
fn roundtrip(s: &Session, scope: StepScope) -> Result<StepAssembly> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("cores.step");
    let scene = step_scene(s.document(), scope)?;
    scene.write(&path)?;
    Ok(StepAssembly::read(&path)?)
}
#[test]
fn step_exports_exact_sketch_cells_holes_overlaps_and_placements_as_one_solid() -> Result<()> {
    let placement = Placement::new([30., -12., 7.], [0.1, 0.2, 0.3])?;
    let (mut s, part, body) = fixture(Solid::cuboid([0.; 3], [20., 20., 5.])?, placement)?;
    s.execute(Command::PaintSurfaces {
        targets: vec![Target::Body(body)],
        color: BLUE,
    })?;
    let mut sketch = Sketch::new(
        "Regiões com furo",
        Plane {
            origin: placement.apply([0., 0., 5.]),
            u: placement.apply_vector([1., 0., 0.]),
            v: placement.apply_vector([0., 1., 0.]),
        },
    );
    sketch.add_rectangle([2., 2.], [10., 10.], false)?;
    sketch.add_circle([6., 6.], 2., false)?;
    s.execute(Command::AddSketch {
        destination: Some(part),
        sketch,
    })?;
    paint(&mut s, placement, [3., 3., 50.], RED)?;
    paint(&mut s, placement, [6., 6., 50.], GREEN)?;
    let original = s.document().body(body).ok_or("Corpo ausente")?.brep.clone();
    let revision = s.revision();
    // Salvar/reabrir não depende da malha colorida nem do STEP original.
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("pintura.3dnator");
    nator_project_io::write_project(&path, s.document())?;
    let reopened = Session::new(nator_project_io::read_project(&path)?)?;
    for scope in [
        StepScope::Body(body),
        StepScope::Part(part),
        StepScope::Assembly,
    ] {
        let scene = roundtrip(&reopened, scope)?;
        let bodies = scene
            .parts
            .iter()
            .flat_map(|p| &p.bodies)
            .collect::<Vec<_>>();
        assert_eq!(
            bodies.len(),
            1,
            "Cada região deve permanecer no mesmo sólido"
        );
        assert!((bodies[0].solid.volume()? - 2000.).abs() < 1e-6);
        let expected_center = placement.apply([10., 10., 2.5]);
        let center = bodies[0].solid.properties()?.center;
        assert!(nator_geometry::norm(nator_geometry::sub(center, expected_center)) < 1e-6);
        let areas = colored_areas(&scene)?;
        for (color, area) in [
            (RED, 64. - 4. * std::f64::consts::PI),
            (GREEN, 4. * std::f64::consts::PI),
            (BLUE, 1136.),
        ] {
            assert!(
                (areas.get(&color.channels()).copied().unwrap_or(0.) - area).abs() < 1e-6,
                "Cor {color:?}: {areas:?}"
            );
        }
    }
    assert_eq!(s.revision(), revision);
    assert_eq!(
        s.document().body(body).ok_or("Corpo ausente")?.brep,
        original
    );
    Ok(())
}

#[test]
fn step_preserves_through_holes_shared_edges_and_overlapping_masks() -> Result<()> {
    let solid =
        Solid::cuboid([0.; 3], [20., 20., 5.])?.cut(&Solid::cylinder([10., 10., -1.], 2., 7.)?)?;
    let (mut s, part, body) = fixture(solid, Placement::default())?;
    for x in [-2., 7.] {
        let mut sketch = Sketch::new(
            "Corte atravessa borda",
            Plane {
                origin: [0., 0., 5.],
                ..Plane::xy()
            },
        );
        sketch.add_rectangle([x, -2.], [x + 10., 22.], false)?;
        s.execute(Command::AddSketch {
            destination: Some(part),
            sketch,
        })?;
    }
    // Interseção e complemento de duas máscaras, alcançando bordas do corpo.
    paint(&mut s, Placement::default(), [7.5, 3., 50.], RED)?;
    paint(&mut s, Placement::default(), [18., 3., 50.], GREEN)?;
    let scene = roundtrip(&s, StepScope::Body(body))?;
    let bodies = scene
        .parts
        .iter()
        .flat_map(|p| &p.bodies)
        .collect::<Vec<_>>();
    assert_eq!(bodies.len(), 1);
    assert!((bodies[0].solid.volume()? - (2000. - 20. * std::f64::consts::PI)).abs() < 1e-6);
    let areas = colored_areas(&scene)?;
    assert!((areas[&RED.channels()] - 20.).abs() < 1e-6);
    assert!((areas[&GREEN.channels()] - 60.).abs() < 1e-6);
    Ok(())
}

#[test]
fn step_partitions_curved_faces_and_preserves_internal_cavity_shells() -> Result<()> {
    for (solid, curved) in [
        (Solid::cylinder([0.; 3], 5., 10.)?, true),
        (
            Solid::cuboid([0.; 3], [20.; 3])?.cut(&Solid::cuboid([5.; 3], [10.; 3])?)?,
            false,
        ),
    ] {
        let volume = solid.volume()?;
        let index = (0..solid.face_count()?).find(|i| solid.face_measure(*i).is_ok_and(|m|
            if curved {matches!(m.kind, nator_kernel::FaceKind::Cylinder {..})}
            else {matches!(m.kind, nator_kernel::FaceKind::Plane {normal, offset} if (normal[2].abs()-1.).abs()<1e-7 && (offset.abs()-20.).abs()<1e-7)}
        )).ok_or("Face de teste ausente")?;
        let mask = Solid::cuboid([0., -10., -1.], [10., 40., 30.])?;
        let (painted, materials) = solid.painted_solid(&[(index, Some(&mask))])?;
        assert!((painted.volume()? - volume).abs() < 1e-6);
        let colors = materials
            .into_iter()
            .map(|m| (m == 1).then_some(RED))
            .collect();
        let scene = nator_kernel::StepAssembly {
            parts: vec![nator_kernel::StepPart {
                name: "Peça".into(),
                bodies: vec![nator_kernel::StepBody {
                    name: "Corpo".into(),
                    solid: painted,
                    color: Some(BLUE),
                    face_colors: colors,
                }],
            }],
            instances: vec![nator_kernel::StepOccurrence {
                part: 0,
                parent: None,
                name: "Peça".into(),
                placement: Placement::default(),
                color: None,
            }],
        };
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("curva.step");
        scene.write(&path)?;
        let reopened = StepAssembly::read(&path)?;
        let bodies = reopened
            .parts
            .iter()
            .flat_map(|p| &p.bodies)
            .collect::<Vec<_>>();
        assert_eq!(bodies.len(), 1);
        assert!((bodies[0].solid.volume()? - volume).abs() < 1e-6);
        let red_area = colored_areas(&reopened)?[&RED.channels()];
        let expected = if curved {
            50. * std::f64::consts::PI
        } else {
            200.
        };
        assert!((red_area - expected).abs() < 1e-6);
    }
    Ok(())
}

#[test]
fn step_face_colors_preserve_occurrence_inheritance_and_explicit_body_paint() -> Result<()> {
    for explicit_base in [false, true] {
        let (mut s, part, body) = fixture(
            Solid::cuboid([0.; 3], [20., 20., 5.])?,
            Placement::default(),
        )?;
        if explicit_base {
            s.execute(Command::PaintSurfaces {
                targets: vec![Target::Body(body)],
                color: BLUE,
            })?;
            let scene = roundtrip(&s, StepScope::Body(body))?;
            assert!((colored_areas(&scene)?[&BLUE.channels()] - 1200.).abs() < 1e-6);
        }
        // Uma face inteira colorida ainda pertence ao mesmo sólido, sem precisar
        // costurá-lo. As demais faces herdam a ocorrência ou a pintura de base.
        paint(&mut s, Placement::default(), [3., 3., 50.], RED)?;
        let mut doc = s.document().clone();
        let first = doc.instances()[0].id;
        doc.instance_mut(first)?.color = Some(GREEN);
        doc.add_instance(part, None, Placement::new([50., 0., 0.], [0.; 3])?)?;
        let s = Session::new(doc)?;
        let scene = roundtrip(&s, StepScope::Assembly)?;
        let bodies = scene
            .parts
            .iter()
            .flat_map(|p| &p.bodies)
            .collect::<Vec<_>>();
        assert_eq!(bodies.len(), 1, "A definição continua compartilhada");
        assert_eq!(bodies[0].solid.face_count()?, 6);
        assert!(scene.instances.iter().any(|i| i.color == Some(GREEN)));
        let areas = colored_areas(&scene)?;
        assert!((areas[&RED.channels()] - 400.).abs() < 1e-6);
        if explicit_base {
            assert!((areas[&BLUE.channels()] - 800.).abs() < 1e-6);
            assert!(bodies[0].face_colors.iter().all(Option::is_some));
        } else {
            assert_eq!(
                bodies[0].face_colors.iter().filter(|c| c.is_none()).count(),
                5
            );
        }
    }
    Ok(())
}

#[test]
fn step_preserves_periodic_single_face_shells_with_a_covering_mask() -> Result<()> {
    // A revolução de um círculo deslocado produz um toro: uma única face
    // periódica fecha toda a casca. Sewing pode devolver Face em vez de Shell.
    let mut profile = Sketch::new("Toro", Plane::xy());
    profile.add_circle([10., 0.], 2., false)?;
    let regions = nator_sketch::areas::regions(&profile)?;
    let region = regions.first().ok_or("Perfil ausente")?;
    let solid = Solid::revolve(
        &profile,
        region,
        [0.; 3],
        [0., 1., 0.],
        std::f64::consts::TAU,
        "torus",
    )?;
    assert_eq!(solid.face_count()?, 1);
    let envelope = Solid::cuboid([-20.; 3], [40.; 3])?;
    let (painted, materials) = solid.painted_solid(&[(0, Some(&envelope))])?;
    assert!((painted.volume()? - 80. * std::f64::consts::PI.powi(2)).abs() < 1e-6);
    assert_eq!(materials, vec![1]);
    let (s, _, body) = fixture(painted, Placement::default())?;
    let scene = roundtrip(&s, StepScope::Body(body))?;
    assert_eq!(scene.parts.iter().map(|p| p.bodies.len()).sum::<usize>(), 1);
    Ok(())
}
