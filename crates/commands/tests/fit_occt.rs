//! Recorte de componente com o núcleo real: volumes exatos da passagem, folga ligada aos
//! parâmetros e erros claros quando a passagem não alcança o corpo.
#![cfg(feature = "occt")]
use nator_commands::{Command, Effect, Session, fit};
use nator_document::{
    BodyId, Document, PartId, SketchId,
    features::{ComponentOutline, Operation, RegionRef},
};
use nator_geometry::{Placement, silhouette};
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A união 2D trabalha em grade inteira: a seção erra na casa do nanômetro.
const VOLUME_MM3: f64 = 1e-4;

fn sketch(s: &mut Session, part: Option<PartId>, sketch: Sketch) -> Result<(PartId, SketchId)> {
    let Effect::SketchCreated { part, sketch } = s.execute(Command::AddSketch {
        destination: part,
        sketch,
    })?
    else {
        return Err("Rascunho ausente".into());
    };
    Ok((part, sketch))
}
fn created(effect: Effect) -> Result<BodyId> {
    let Effect::Imported { body, .. } = effect else {
        return Err("Corpo ausente".into());
    };
    Ok(body)
}
fn prism(
    s: &mut Session,
    part: Option<PartId>,
    origin: [f64; 3],
    size: [f64; 3],
) -> Result<(PartId, BodyId)> {
    let mut profile = Sketch::new("Perfil", Plane::xy());
    profile.plane.origin = [0., 0., origin[2]];
    let corner = [origin[0], origin[1]];
    profile.add_rectangle(corner, [corner[0] + size[0], corner[1] + size[1]], false)?;
    let (part, id) = sketch(s, part, profile)?;
    let body = created(s.execute(Command::AddFeature {
        part,
        name: "Bloco".into(),
        operation: Operation::Extrude {
            sketch: id,
            region: RegionRef::Index(0),
            from: "0 mm".into(),
            to: format!("{} mm", size[2]),
            combine: None,
            until: None,
        },
    })?)?;
    Ok((part, body))
}
fn volume(s: &Session, body: BodyId) -> Result<f64> {
    let body = s.document().body(body).ok_or("Corpo")?;
    Ok(Solid::from_brep(body.brep.as_ref().ok_or("B-rep")?, Some(&body.face_names))?.volume()?)
}
/// Bloco de 40 × 40 × 20 mm, base em z = 0, e componente de 10 × 6 × 4 mm em outra peça, com o
/// fundo em z = 15.
fn scene() -> Result<(Session, PartId, BodyId, BodyId)> {
    let mut s = Session::new(Document::new("Encaixe")?)?;
    let (part, block) = prism(&mut s, None, [0., 0., 0.], [40., 40., 20.])?;
    let (_, component) = prism(&mut s, None, [10., 12., 15.], [10., 6., 4.])?;
    Ok((s, part, block, component))
}
fn outline(s: &Session, component: BodyId, down: [f64; 3]) -> Result<ComponentOutline> {
    let origin = Placement::default();
    Ok(fit::capture(s.document(), component, origin, origin, down)?)
}
fn pocket(
    s: &mut Session,
    part: PartId,
    base: BodyId,
    outline: ComponentOutline,
) -> Result<BodyId> {
    created(s.execute(Command::AddFeature {
        part,
        name: "Recorte".into(),
        operation: Operation::ComponentCut {
            base,
            outline,
            clearance: "0.2 mm".into(),
            floor: "0.5 mm".into(),
            depth: None,
        },
    })?)
}

#[test]
fn the_passage_opens_from_the_bottom_of_the_component_with_exact_clearance() -> Result<()> {
    let (mut s, part, block, component) = scene()?;
    let captured = outline(&s, component, [0., 0., -1.])?;
    assert_eq!(captured.contours.len(), 1);
    assert_eq!(captured.source, "Corpo 2");
    let before = volume(&s, block)?;
    let cut = pocket(&mut s, part, block, captured)?;
    // Do fundo do componente, 0,5 mm abaixo dele, até o topo do bloco.
    let expected = 10.4 * 6.4 * (20. - 15. + 0.5);
    assert!(((before - volume(&s, cut)?) - expected).abs() < VOLUME_MM3);
    Ok(())
}

#[test]
fn changing_the_clearance_recomputes_without_a_new_capture() -> Result<()> {
    let (mut s, part, block, component) = scene()?;
    let captured = outline(&s, component, [0., 0., -1.])?;
    let cut = pocket(&mut s, part, block, captured.clone())?;
    s.execute(Command::EditFeature {
        part,
        output: cut,
        operation: Operation::ComponentCut {
            base: block,
            outline: captured,
            clearance: "folga_livre".into(),
            floor: "0 mm".into(),
            depth: Some("3 mm".into()),
        },
    })?;
    let gap = s.document().print_profile().fit_free_mm;
    let expected = (10. + 2. * gap) * (6. + 2. * gap) * 3.;
    assert!(((volume(&s, block)? - volume(&s, cut)?) - expected).abs() < VOLUME_MM3);
    Ok(())
}

#[test]
fn a_stepped_component_needs_the_union_of_its_levels() -> Result<()> {
    let (mut s, part, block, component) = scene()?;
    // Conector mais largo em cima: a passagem precisa deixar a parte larga descer.
    let (_, wide) = prism(&mut s, None, [8., 14., 19.], [14., 2., 1.])?;
    let lower = outline(&s, component, [0., 0., -1.])?;
    let upper = outline(&s, wide, [0., 0., -1.])?;
    let triangles = [&lower, &upper].into_iter().flat_map(|o| {
        let c = &o.contours[0];
        [[c[0], c[1], c[2]], [c[0], c[2], c[3]]]
    });
    let contours = silhouette::silhouette(triangles);
    assert_eq!(contours.len(), 1);
    let captured = ComponentOutline { contours, ..lower };
    let before = volume(&s, block)?;
    let cut = pocket(&mut s, part, block, captured)?;
    // Silhueta em cruz: 10,4 × 6,4 mais duas abas de 2 × 2,4 mm.
    let section = 10.4 * 6.4 + 2. * 2. * 2.4;
    let removed = before - volume(&s, cut)?;
    assert!((removed - section * 5.5).abs() < VOLUME_MM3);
    Ok(())
}

#[test]
fn a_side_insertion_cuts_through_the_nearest_wall_only() -> Result<()> {
    let (mut s, part, block, component) = scene()?;
    let captured = outline(&s, component, [1., 0., 0.])?;
    let before = volume(&s, block)?;
    let cut = pocket(&mut s, part, block, captured)?;
    // Inserção ao longo de +X: a passagem vai de x = 20,5 até a face x = 0.
    let section = 6.4 * 4.4;
    assert!(((before - volume(&s, cut)?) - section * 20.5).abs() < VOLUME_MM3);
    Ok(())
}

#[test]
fn a_passage_that_misses_the_body_is_refused() -> Result<()> {
    let (mut s, part, block, component) = scene()?;
    let moved = Placement::new([100., 0., 0.], [0.; 3])?;
    let far = fit::capture(
        s.document(),
        component,
        moved,
        Placement::default(),
        [0., 0., -1.],
    )?;
    let error = pocket(&mut s, part, block, far).err().ok_or("Erro")?;
    assert!(error.to_string().contains("não alcança"));
    Ok(())
}

#[test]
fn the_capture_follows_the_placement_of_both_parts() -> Result<()> {
    let (s, _, _, component) = scene()?;
    let pose = Placement::new([0., 0., 30.], [0.; 3])?;
    let target = Placement::new([5., 0., 0.], [0.; 3])?;
    let captured = fit::capture(s.document(), component, pose, target, [0., 0., -1.])?;
    let contour = &captured.contours[0];
    let world = contour
        .iter()
        .map(|p| captured.plane.world(*p))
        .collect::<Vec<_>>();
    // Componente em x de 10 a 20, 5 mm mais perto da origem da peça.
    let x = world.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    assert!((x - 5.).abs() < 1e-6);
    assert!(world.iter().all(|p| (p[2] - 45.).abs() < 1e-9));
    Ok(())
}

#[test]
fn several_contours_become_separate_passages() -> Result<()> {
    let (mut s, part, block, component) = scene()?;
    let mut captured = outline(&s, component, [0., 0., -1.])?;
    let copy = captured.contours[0]
        .iter()
        .map(|p| [p[0] + 15., p[1]])
        .collect::<Vec<_>>();
    captured.contours.push(copy);
    let before = volume(&s, block)?;
    let cut = pocket(&mut s, part, block, captured)?;
    let one = 10.4 * 6.4 * 5.5;
    let removed = before - volume(&s, cut)?;
    assert!((removed - 2. * one).abs() < VOLUME_MM3);
    Ok(())
}

mod checking {
    use super::*;
    use nator_commands::fit_check::{self, Severity};
    use nator_document::InstanceId;

    fn instance_of(s: &Session, body: BodyId) -> Result<(PartId, InstanceId)> {
        let part = s
            .document()
            .parts()
            .iter()
            .find(|p| p.bodies.iter().any(|b| b.id == body))
            .ok_or("Peça")?
            .id;
        let occurrences = s.document().occurrences()?;
        let instance = occurrences
            .iter()
            .map(|o| o.instance)
            .find(|i| s.document().instance(*i).is_some_and(|i| i.part == part))
            .ok_or("Instância")?;
        Ok((part, instance))
    }
    fn moved(s: &mut Session, body: BodyId, x: f64) -> Result<()> {
        let (_, id) = instance_of(s, body)?;
        s.execute(Command::MoveInstance {
            id,
            placement: Placement::new([x, 0., 0.], [0.; 3])?,
        })?;
        Ok(())
    }

    #[test]
    fn a_component_inside_the_block_is_an_interference_with_its_exact_volume() -> Result<()> {
        let (s, part, _, _) = scene()?;
        let report = fit_check::check(s.document(), Some(part))?;
        assert_eq!(report.compared, 1);
        assert_eq!(report.contacts.len(), 1);
        let clash = &report.contacts[0];
        assert_eq!(clash.severity, Severity::Interference);
        assert!((clash.volume_mm3 - 10. * 6. * 4.).abs() < 1e-6);
        assert!((clash.point[2] - 17.).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn the_verdict_follows_the_placement_of_the_component() -> Result<()> {
        let (mut s, part, _, component) = scene()?;
        // Componente em x de 10 a 20: mover para fora do bloco (x até 40).
        moved(&mut s, component, 30.)?;
        let touching = fit_check::check(s.document(), Some(part))?;
        assert_eq!(touching.contacts[0].severity, Severity::Touching);
        moved(&mut s, component, 30.05)?;
        let tight = fit_check::check(s.document(), Some(part))?;
        assert_eq!(tight.contacts[0].severity, Severity::Tight);
        assert!((tight.contacts[0].distance_mm - 0.05).abs() < 1e-9);
        moved(&mut s, component, 31.)?;
        let free = fit_check::check(s.document(), Some(part))?;
        assert!(free.contacts.is_empty());
        assert_eq!(free.compared, 0);
        Ok(())
    }

    #[test]
    fn the_recess_of_the_component_clears_the_interference() -> Result<()> {
        let (mut s, part, block, component) = scene()?;
        let captured = outline(&s, component, [0., 0., -1.])?;
        pocket(&mut s, part, block, captured)?;
        let report = fit_check::check(s.document(), Some(part))?;
        assert!(report.contacts.is_empty(), "{:?}", report.contacts);
        assert_eq!(report.compared, 1);
        Ok(())
    }
}

mod excerpts {
    use super::*;
    use nator_commands::export::{self, Excerpt, Quality, Scope};

    fn excerpt(min: [f64; 3], max: [f64; 3]) -> Option<Excerpt> {
        Some(Excerpt { min, max })
    }

    #[test]
    fn only_the_box_is_exported_and_the_model_stays_the_same() -> Result<()> {
        let (s, _, block, _) = scene()?;
        let before = volume(&s, block)?;
        let plan = export::print_plan_of(
            s.document(),
            Scope::Body(block),
            Quality::Standard,
            true,
            None,
            excerpt([5., 5., -1.], [15., 25., 8.]),
        )?;
        assert_eq!(plan.files.len(), 1);
        assert!(plan.files[0].name.ends_with("_trecho"));
        let mesh = &plan.files[0].mesh;
        assert!((mesh.signed_volume() - 10. * 20. * 8.).abs() < 1e-6);
        // Com a preparação, a base do trecho vai para a mesa.
        let placed = &plan.layout.files[0];
        assert_eq!(placed.size, [10., 20., 8.]);
        nator_project_io::require_printable(&plan.files[0].name, mesh)?;
        assert!((volume(&s, block)? - before).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn a_box_away_from_the_body_is_a_clear_error() -> Result<()> {
        let (s, _, block, _) = scene()?;
        let error = export::print_plan_of(
            s.document(),
            Scope::Body(block),
            Quality::Standard,
            false,
            None,
            excerpt([100., 0., 0.], [110., 10., 10.]),
        )
        .err()
        .ok_or("Erro")?;
        assert!(error.to_string().contains("não alcança"));
        Ok(())
    }

    #[test]
    fn the_box_starts_around_the_body_and_only_fits_one_part() -> Result<()> {
        let (s, _, block, _) = scene()?;
        let bounds = export::scope_bounds(s.document(), Scope::Body(block)).ok_or("Caixa")?;
        assert_eq!((bounds.min, bounds.max), ([0.; 3], [40., 40., 20.]));
        assert!(export::scope_bounds(s.document(), Scope::Definitions).is_none());
        let refused = export::print_plan_of(
            s.document(),
            Scope::Definitions,
            Quality::Standard,
            false,
            None,
            excerpt([0.; 3], [1.; 3]),
        );
        assert!(refused.is_err());
        Ok(())
    }
}
