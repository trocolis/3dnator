//! Correspondência de costuras sem distorção em círculos e retângulos.
#![cfg(feature = "occt")]
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch, profiles};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[test]
fn loft_aligns_circle_seams_and_preserves_rectangle_circle_symmetry() -> Result<()> {
    for rotated in [false, true] {
        let mut first = Sketch::new("Círculo", Plane::xy());
        first.add_circle([0., 0.], 5., false)?;
        let plane = if rotated {
            Plane {
                origin: [0., 0., 20.],
                u: [0., 1., 0.],
                v: [-1., 0., 0.],
            }
        } else {
            Plane {
                origin: [0., 0., 20.],
                ..Plane::xy()
            }
        };
        let mut last = Sketch::new("Topo", plane);
        last.add_circle([0., 0.], 3., false)?;
        let a = profiles::regions(&first)?;
        let b = profiles::regions(&last)?;
        for ruled in [false, true] {
            let solid = Solid::loft(
                (&first, a.first().ok_or("Perfil")?),
                (&last, b.first().ok_or("Perfil")?),
                ruled,
                false,
                "loft",
            )?;
            let expected = std::f64::consts::PI * 20. * (25. + 15. + 9.) / 3.;
            assert!(
                (solid.volume()? - expected).abs() < 1e-5,
                "Volume circular: {}",
                solid.volume()?
            );
        }
    }
    let mut first = Sketch::new("Retângulo", Plane::xy());
    first.add_rectangle([-8., -5.], [8., 5.], false)?;
    let mut last = Sketch::new(
        "Círculo",
        Plane {
            origin: [0., 0., 20.],
            ..Plane::xy()
        },
    );
    last.add_circle([0., 0.], 3., false)?;
    let a = profiles::regions(&first)?;
    let b = profiles::regions(&last)?;
    for ruled in [false, true] {
        let solid = Solid::loft(
            (&first, a.first().ok_or("Perfil")?),
            (&last, b.first().ok_or("Perfil")?),
            ruled,
            false,
            "loft",
        )?;
        let volume = solid.volume()?;
        for normal in [[1., 0., 0.], [0., 1., 0.]] {
            let half = solid.split([0.; 3], normal, "half")?.volume()?;
            assert!(
                (half - volume / 2.).abs() < 1e-5,
                "Simetria: {half}, total {volume}, ruled {ruled}"
            );
        }
        assert!(
            nator_geometry::norm(nator_geometry::sub(
                solid.properties()?.center,
                [0., 0., solid.properties()?.center[2]]
            )) < 1e-6
        );
    }
    Ok(())
}

#[test]
fn circular_loft_on_an_inclined_plane_keeps_its_mirror_plane() -> Result<()> {
    let mut a = Sketch::new("Base", Plane::xy());
    a.add_rectangle([-8., -5.], [8., 5.], false)?;
    let angle = std::f64::consts::PI / 6.;
    let mut b = Sketch::new(
        "Topo inclinado",
        Plane {
            origin: [0., 0., 20.],
            u: [angle.cos(), 0., -angle.sin()],
            v: [0., 1., 0.],
        },
    );
    b.add_circle([0., 0.], 3., false)?;
    let ra = profiles::regions(&a)?;
    let rb = profiles::regions(&b)?;
    let solid = Solid::loft(
        (&a, ra.first().ok_or("Área")?),
        (&b, rb.first().ok_or("Área")?),
        true,
        false,
        "inclined",
    )?;
    let volume = solid.volume()?;
    assert!((solid.split([0.; 3], [0., 1., 0.], "half")?.volume()? - volume / 2.).abs() < 1e-5);
    Ok(())
}
