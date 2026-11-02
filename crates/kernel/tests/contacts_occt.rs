//! Contatos em linha com o OCCT real: detecção, nomes e reparo por pino.
#![cfg(feature = "occt")]
use nator_kernel::Solid;
use std::{error::Error, f64::consts::PI};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Dois entalhes rasos que se tocam em (20; 20): o bloco do canto continua preso pela base,
/// mas encosta no restante da peça só ao longo de uma linha vertical de 5 mm.
fn notched() -> Result<Solid> {
    let mut plate = Solid::cuboid([0.; 3], [30., 30., 10.])?;
    plate.name_faces_by_index("b1")?;
    let mut first = Solid::cuboid([10., 20., 5.], [10., 10., 5.])?;
    first.name_faces_by_index("b2")?;
    let mut second = Solid::cuboid([20., 10., 5.], [10., 10., 5.])?;
    second.name_faces_by_index("b3")?;
    Ok(plate.boolean(&first, 1, "b4")?.boolean(&second, 1, "b4")?)
}

#[test]
fn line_contact_is_found_with_its_faces_and_location() -> Result<()> {
    let solid = notched()?;
    let contacts = solid.contacts()?;
    assert_eq!(contacts.len(), 1);
    let contact = &contacts[0];
    assert_eq!(contact.faces.len(), 4);
    assert!(contact.faces.is_sorted());
    assert!((contact.length - 5.).abs() < 1e-9);
    assert_eq!(contact.points.len(), 2);
    assert!(contact.points.iter().all(|p| p[..2] == [20., 20.]));
    let mut solid = solid;
    assert!(solid.tessellate(0.02, 0.25)?.topology().non_manifold_edges > 0);
    Ok(())
}

#[test]
fn manifold_solids_have_no_contacts() -> Result<()> {
    assert!(Solid::cuboid([0.; 3], [1., 2., 3.])?.contacts()?.is_empty());
    assert!(Solid::cylinder([0.; 3], 2., 3.)?.contacts()?.is_empty());
    Ok(())
}

#[test]
fn welding_and_separating_remove_the_contact_and_change_little_volume() -> Result<()> {
    const RADIUS: f64 = 0.05;
    // O pino ocupa dois dos quatro quadrantes em volta da aresta: dois cheios, dois vazios.
    let half_pin = PI * RADIUS * RADIUS * 5. / 2.;
    let solid = notched()?;
    let edge = solid.contacts()?[0].edge;
    let volume = solid.volume()?;
    for (weld, sign) in [(true, 1.), (false, -1.)] {
        let mut repaired = solid.repair_contact(edge, RADIUS, weld, 0, "b5")?;
        assert!(repaired.contacts()?.is_empty());
        assert!((repaired.volume()? - volume - sign * half_pin).abs() < 1e-6);
        let mesh = repaired.tessellate(0.02, 0.25)?;
        assert!(mesh.topology().has_closed_oriented_edges());
        assert!(
            repaired
                .face_names()
                .iter()
                .any(|name| name.starts_with("b5:contact0:") || name.starts_with("b1:"))
        );
    }
    Ok(())
}

/// Vazios por dentro e por fora de um círculo que se tocam em r = 5, z = 5.
#[test]
fn curved_contacts_are_repaired_along_the_curve() -> Result<()> {
    let mut plate = Solid::cuboid([-10., -10., 0.], [20., 20., 10.])?;
    plate.name_faces_by_index("b1")?;
    let mut disk = Solid::cylinder([0.; 3], 5., 5.)?;
    disk.name_faces_by_index("b2")?;
    let mut outer = Solid::cylinder([0., 0., 5.], 8., 5.)?;
    outer.name_faces_by_index("b3")?;
    let mut inner = Solid::cylinder([0., 0., 5.], 5., 5.)?;
    inner.name_faces_by_index("b6")?;
    let ring = outer.boolean(&inner, 1, "b3")?;
    let solid = plate.boolean(&disk, 1, "b4")?.boolean(&ring, 1, "b4")?;
    let contacts = solid.contacts()?;
    assert_eq!(contacts.len(), 1);
    assert!((contacts[0].length - 2. * PI * 5.).abs() < 1e-6);
    assert_eq!(contacts[0].points.len(), 33);
    let mut repaired = solid.repair_contact(contacts[0].edge, 0.05, true, 0, "b5")?;
    assert!(repaired.contacts()?.is_empty());
    assert!(repaired.volume()? > solid.volume()?);
    assert!(
        repaired
            .tessellate(0.02, 0.25)?
            .topology()
            .has_closed_oriented_edges()
    );
    Ok(())
}
