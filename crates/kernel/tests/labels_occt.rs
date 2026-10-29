//! Contornos de fonte e os três modos de rótulo em superfícies planas/curvas.
#![cfg(feature = "occt")]
use nator_kernel::{FaceKind, Solid, TextMode, TextPlacement};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[test]
fn text_modes_preserve_surface_volume_and_emboss_or_engrave_planes_and_cylinders() -> Result<()> {
    assert!(!Solid::font_names()?.is_empty());
    let outline = Solid::text_outline("O", "sans-serif", false, false, false)?;
    let mut block = Solid::cuboid([-20., -20., 0.], [40., 40., 5.])?;
    block.name_faces_by_index("block")?;
    let face=(0..block.face_count()?).find(|i|matches!(block.face_measure(*i).map(|m|m.kind),Ok(FaceKind::Plane {normal,..}) if normal[2]>0.9)).ok_or("Tampa")?;
    let base = block.volume()?;
    let mut deltas = vec![];
    for mode in [TextMode::Surface, TextMode::Raised, TextMode::Engraved] {
        let (solid, colored) = block
            .label(
                face,
                &outline,
                TextPlacement {
                    point: [0., 0., 5.],
                    uv: None,
                    size_mm: 6.,
                    angle_rad: 0.,
                    depth_mm: 0.5,
                    mode,
                },
                "label",
            )
            .map_err(|e| format!("Plano {mode:?}: {e}"))?;
        assert!(!colored.is_empty());
        deltas.push(solid.volume()? - base);
    }
    assert!(deltas[0].abs() < 1e-4, "{deltas:?}");
    assert!(deltas[1] > 1.);
    assert!(deltas[2] < -1.);
    assert!((deltas[1] + deltas[2]).abs() < 1e-4, "{deltas:?}");
    let mut cylinder = Solid::cylinder([0.; 3], 10., 20.)?;
    cylinder.name_faces_by_index("cylinder")?;
    let face = (0..cylinder.face_count()?)
        .find(|i| {
            matches!(
                cylinder.face_measure(*i).map(|m| m.kind),
                Ok(FaceKind::Cylinder { .. })
            )
        })
        .ok_or("Cilindro")?;
    let base = cylinder.volume()?;
    for (i, mode) in [TextMode::Surface, TextMode::Raised, TextMode::Engraved]
        .into_iter()
        .enumerate()
    {
        let (solid, colored) = cylinder
            .label(
                face,
                &outline,
                TextPlacement {
                    point: [10., 0., 10.],
                    uv: None,
                    size_mm: 3.,
                    angle_rad: 0.,
                    depth_mm: 0.3,
                    mode,
                },
                "curved",
            )
            .map_err(|e| format!("Curva {mode:?}: {e}"))?;
        assert!(!colored.is_empty());
        let delta = solid.volume()? - base;
        match i {
            0 => assert!(delta.abs() < 1e-5),
            1 => assert!(delta > 0.),
            _ => assert!(delta < 0.),
        }
    }
    let styled = Solid::text_outline("Peça Nº1", "sans-serif", true, true, true)?;
    assert!(!styled.is_empty());
    Ok(())
}
