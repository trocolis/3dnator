//! Operações da etapa 3 com o OCCT real: furos, padrões, espelhamento e divisão.
#![cfg(feature = "occt")]

use nator_geometry::Placement;
use nator_kernel::{
    Solid,
    hole::{HoleGeometry, HoleHead, removed_volume},
};
use nator_sketch::{Plane, Sketch, profiles};
use std::{error::Error, f64::consts::PI};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Bloco 40 × 30 × 20 mm com a face superior em z = 20.
fn block() -> Result<Solid> {
    let mut sketch = Sketch::new("Bloco", Plane::xy());
    sketch.add_rectangle([0., 0.], [40., 30.], false)?;
    let region = profiles::regions(&sketch)?.remove(0);
    Ok(Solid::extrude(&sketch, &region, 0., 20., "b1")?)
}

const BLOCK: f64 = 40. * 30. * 20.;

fn drill(body: &Solid, geometry: &HoleGeometry, at: [f64; 2]) -> Result<Solid> {
    let tool = Solid::hole_tool([at[0], at[1], 20.], [0., 0., -1.], geometry, "b2", 0)?;
    Ok(body.boolean(&tool, 1, "b2")?)
}

#[test]
fn blind_hole_with_drill_point_removes_the_analytical_volume() -> Result<()> {
    let geometry = HoleGeometry {
        diameter: 6.,
        depth: 10.,
        head: HoleHead::Simple,
        tip_angle: Some(118_f64.to_radians()),
    };
    let drilled = drill(&block()?, &geometry, [10., 10.])?;
    assert!((BLOCK - drilled.volume()? - removed_volume(&geometry)).abs() < 1e-4);
    let names = drilled.face_names();
    for role in ["wall", "tip"] {
        assert!(names.contains(&format!("b2:hole0:{role}")), "{names:?}");
    }
    assert!(names.iter().all(|n| !n.is_empty()), "{names:?}");
    Ok(())
}

#[test]
fn through_holes_with_counterbore_and_countersink() -> Result<()> {
    // Passante: a parede vai além dos 20 mm do bloco, sem ponta.
    let counterbore = HoleGeometry {
        diameter: 6.,
        depth: 30.,
        head: HoleHead::Counterbore {
            diameter: 11.,
            depth: 6.,
        },
        tip_angle: None,
    };
    // Faces planas da revolução (topo, piso do rebaixo, fundo) também recebem nome.
    let tool = Solid::hole_tool([10., 15., 20.], [0., 0., -1.], &counterbore, "b2", 0)?;
    assert_eq!(tool.face_names().len(), 5);
    assert!(tool.face_names().iter().all(|n| n.starts_with("b2:hole0:")));
    let drilled = drill(&block()?, &counterbore, [10., 15.])?;
    let expected = PI * 5.5 * 5.5 * 6. + PI * 9. * 14.;
    assert!((BLOCK - drilled.volume()? - expected).abs() < 1e-4);
    assert!(
        drilled
            .face_names()
            .contains(&"b2:hole0:cbore_floor".to_owned())
    );
    let countersink = HoleGeometry {
        head: HoleHead::Countersink {
            diameter: 12.,
            angle: 90_f64.to_radians(),
        },
        ..counterbore
    };
    let drilled = drill(&block()?, &countersink, [30., 15.])?;
    // Cone de 90° do Ø12 ao Ø6 tem 3 mm de altura; o restante é cilindro de Ø6.
    let cone = PI * 3. * (36. + 18. + 9.) / 3.;
    let expected = cone + PI * 9. * 17.;
    assert!((BLOCK - drilled.volume()? - expected).abs() < 1e-4);
    Ok(())
}

#[test]
fn linear_pattern_of_hole_tools_keeps_each_copy_referenceable() -> Result<()> {
    let geometry = HoleGeometry {
        diameter: 4.,
        depth: 30.,
        head: HoleHead::Simple,
        tip_angle: None,
    };
    let tool = Solid::hole_tool([8., 15., 20.], [0., 0., -1.], &geometry, "b2", 0)?;
    let placements = (0..3)
        .map(|k| Placement::new([k as f64 * 12., 0., 0.], [0.; 3]))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let tools = tool.pattern(&placements, "b3")?;
    let drilled = block()?.boolean(&tools, 1, "b4")?;
    assert!((BLOCK - drilled.volume()? - 3. * PI * 4. * 20.).abs() < 1e-4);
    let names = drilled.face_names();
    for name in ["b2:hole0:wall", "b2:hole0:wall:p1", "b2:hole0:wall:p2"] {
        assert!(names.contains(&name.to_owned()), "{names:?}");
    }
    Ok(())
}

#[test]
fn circular_pattern_mirror_and_split() -> Result<()> {
    let body = block()?;
    // Quatro cópias girando 90° em torno de um eixo vertical fora do bloco: sem sobreposição.
    let placements = (0..4)
        .map(|k| {
            Placement::default().rotated_about([-10., -10., 0.], [0., 0., 1.], k as f64 * PI / 2.)
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let ring = body.pattern(&placements, "b5")?;
    assert!((ring.volume()? - 4. * BLOCK).abs() < 1e-3);
    // Espelhar pelo plano x = 0 preserva volume e nomes.
    let mirrored = body.mirrored([0.; 3], [1., 0., 0.])?;
    assert!((mirrored.volume()? - BLOCK).abs() < 1e-6);
    assert_eq!(mirrored.face_names(), body.face_names());
    let mut mesh = mirrored.clone_mesh()?;
    assert!((mesh.bounds().min[0] + 40.).abs() < 1e-9);
    mesh = mesh.welded();
    assert!(mesh.topology().has_closed_oriented_edges());
    // Dividir pelo plano z = 5 mantém a parte de cima.
    let upper = body.split([0., 0., 5.], [0., 0., 1.], "b6")?;
    assert!((upper.volume()? - 40. * 30. * 15.).abs() < 1e-6);
    assert!(upper.face_names().contains(&"b6:split".to_owned()));
    assert!(upper.face_names().contains(&"b1:end".to_owned()));
    assert!(!upper.face_names().contains(&"b1:start".to_owned()));
    // Um plano que não corta o corpo do lado escolhido é um erro explícito.
    assert!(body.split([0., 0., 50.], [0., 0., 1.], "b7").is_err());
    Ok(())
}

/// Ajuda de teste: tesselação de uma cópia sem alterar o sólido original.
trait CloneMesh {
    fn clone_mesh(&self) -> Result<nator_geometry::Mesh>;
}
impl CloneMesh for Solid {
    fn clone_mesh(&self) -> Result<nator_geometry::Mesh> {
        let mut copy = self.transformed(Placement::default())?;
        Ok(copy.tessellate(0.1, 0.3)?)
    }
}
