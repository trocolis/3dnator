//! Nomes estáveis com o OCCT real. Executados apenas com `--features occt`.
#![cfg(feature = "occt")]

use nator_kernel::Solid;
use nator_sketch::{
    Plane, Sketch,
    profiles::{self, Region},
};
use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Perfil do suporte em L no plano XZ: base de 60 mm, altura de 30 mm e espessura `t`.
/// Retorna o rascunho e as entidades das duas linhas que formam o canto interno.
fn bracket(thickness: f64) -> Result<(Sketch, u64, u64)> {
    let mut sketch = Sketch::new("Suporte", Plane::xz());
    let points = [
        [0., 0.],
        [60., 0.],
        [60., thickness],
        [thickness, thickness],
        [thickness, 30.],
        [0., 30.],
    ];
    let mut lines = Vec::new();
    for i in 0..points.len() {
        lines.push(sketch.add_line(points[i], points[(i + 1) % points.len()], false)?);
    }
    // Pontos coincidentes viram o mesmo nó do perfil; não é preciso fundi-los no rascunho.
    Ok((sketch, lines[2], lines[3]))
}

fn single_region(sketch: &Sketch) -> Result<Region> {
    let mut regions = profiles::regions(sketch)?;
    if regions.len() != 1 {
        return Err("O perfil deveria ter uma única região".into());
    }
    Ok(regions.remove(0))
}

fn unique(names: &[String]) -> bool {
    let mut seen = std::collections::HashSet::new();
    names.iter().all(|n| !n.is_empty() && seen.insert(n))
}

#[test]
fn extrusion_faces_are_named_by_entities_and_caps() -> Result<()> {
    let (sketch, inner_horizontal, _) = bracket(6.)?;
    let solid = Solid::extrude(&sketch, &single_region(&sketch)?, 0., 20., "b1")?;
    let names = solid.face_names();
    assert_eq!(names.len(), 8);
    assert!(unique(names), "nomes: {names:?}");
    assert!(names.contains(&"b1:start".to_owned()));
    assert!(names.contains(&"b1:end".to_owned()));
    assert!(names.contains(&format!("b1:e{inner_horizontal}")));
    assert!((solid.volume()? - 10_080. * 20. / 20.).abs() < 1e-6);
    Ok(())
}

/// O caso clássico de nomeação topológica: arredondar o canto interno, mudar a espessura e
/// recalcular. O arredondamento deve encontrar a mesma aresta pelos nomes das faces.
#[test]
fn fillet_reference_survives_a_thickness_change() -> Result<()> {
    let mut volumes = Vec::new();
    let mut reference = None;
    for thickness in [6., 9.] {
        let (sketch, horizontal, vertical) = bracket(thickness)?;
        let mut body = Solid::extrude(&sketch, &single_region(&sketch)?, 0., 20., "b1")?;
        // A referência é criada uma única vez, na primeira espessura, como faria a interface.
        let pair = match &reference {
            Some(pair) => pair,
            None => reference.insert([format!("b1:e{horizontal}"), format!("b1:e{vertical}")]),
        };
        let edge = body.edge_between(pair)?;
        let rounded = body.fillet(&[edge], 3., "b2")?;
        let without = body.volume()?;
        let with = rounded.volume()?;
        // Arredondamento interno (côncavo) acrescenta material: (1 - π/4)·r²·L.
        let expected = (1. - std::f64::consts::FRAC_PI_4) * 9. * 20.;
        assert!(
            ((with - without) - expected).abs() < 1e-3,
            "{with} - {without}"
        );
        assert!(rounded.face_names().contains(&"b2:round".to_owned()));
        volumes.push(with);
        body.tessellate(0.1, 0.3)?;
    }
    assert!(volumes[1] > volumes[0]);
    Ok(())
}

#[test]
fn brep_round_trip_keeps_names_aligned_with_faces() -> Result<()> {
    let (sketch, horizontal, vertical) = bracket(6.)?;
    let mut body = Solid::extrude(&sketch, &single_region(&sketch)?, 0., 20., "b1")?;
    let pair = [format!("b1:e{horizontal}"), format!("b1:e{vertical}")];
    let before = body
        .fillet(&[body.edge_between(&pair)?], 2., "b2")?
        .volume()?;
    let names = body.face_names().to_vec();
    let restored = Solid::from_brep(&body.to_brep()?, Some(&names))?;
    let after = restored
        .fillet(&[restored.edge_between(&pair)?], 2., "b2")?
        .volume()?;
    assert!((before - after).abs() < 1e-9);
    // Uma lista de nomes de outro sólido é rejeitada em vez de desalinhada.
    assert!(Solid::from_brep(&body.to_brep()?, Some(&names[..3])).is_err());
    // Sem nomes, as faces existem, mas não podem ser referenciadas.
    let unnamed = Solid::from_brep(&body.to_brep()?, None)?;
    assert!(unnamed.edge_between(&pair).is_err());
    Ok(())
}

#[test]
fn boolean_cut_inherits_names_from_both_bodies() -> Result<()> {
    let (sketch, horizontal, _) = bracket(6.)?;
    let body = Solid::extrude(&sketch, &single_region(&sketch)?, 0., 20., "b1")?;
    // Furo vertical de Ø6 mm atravessando a base, desenhado no plano XY.
    let mut hole = Sketch::new("Furo", Plane::xy());
    let circle = hole.add_circle([40., -10.], 3., false)?;
    let tool = Solid::extrude(&hole, &single_region(&hole)?, -1., 10., "b2")?;
    let drilled = body.boolean(&tool, 1, "b3")?;
    let names = drilled.face_names();
    assert!(names.contains(&format!("b2:e{circle}")), "{names:?}");
    assert!(names.contains(&format!("b1:e{horizontal}")));
    assert!(names.iter().all(|n| !n.is_empty()), "{names:?}");
    let expected = 10_080. - std::f64::consts::PI * 9. * 6.;
    assert!((drilled.volume()? - expected).abs() < 1e-5);
    Ok(())
}

#[test]
fn shell_and_transform_keep_referenceable_faces() -> Result<()> {
    let mut sketch = Sketch::new("Caixa", Plane::xy());
    sketch.add_rectangle([0., 0.], [40., 30.], false)?;
    let block = Solid::extrude(&sketch, &single_region(&sketch)?, 0., 20., "b1")?;
    let open = block.shell(&[block.face_named("b1:end")?], 2., false, "b2")?;
    let expected = 40. * 30. * 20. - 36. * 26. * 18.;
    assert!((open.volume()? - expected).abs() < 1e-4);
    let names = open.face_names();
    assert!(!names.contains(&"b1:end".to_owned()));
    assert!(names.contains(&"b1:start".to_owned()));
    assert!(names.iter().any(|n| n.ends_with(":inner")), "{names:?}");
    let moved = open.transformed(nator_geometry::Placement::new(
        [10., 0., 0.],
        [0., 0., 0.5],
    )?)?;
    assert_eq!(moved.face_names(), open.face_names());
    assert!((moved.volume()? - expected).abs() < 1e-4);
    Ok(())
}
