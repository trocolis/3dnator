//! Transição polígono → círculo em triângulos planos e cones, com casca de parede uniforme.
#![cfg(feature = "occt")]
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch, profiles};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type Vector = [f64; 3];

fn rectangle(width: f64, depth: f64) -> Result<Sketch> {
    let mut sketch = Sketch::new("Base", Plane::xy());
    sketch.add_rectangle([-width / 2., -depth / 2.], [width / 2., depth / 2.], false)?;
    Ok(sketch)
}
fn circle(radius: f64, height: f64) -> Result<Sketch> {
    let mut sketch = Sketch::new(
        "Topo",
        Plane {
            origin: [0., 0., height],
            ..Plane::xy()
        },
    );
    sketch.add_circle([0., 0.], radius, false)?;
    Ok(sketch)
}
fn developable(first: &Sketch, last: &Sketch) -> Result<Solid> {
    let (a, b) = (profiles::regions(first)?, profiles::regions(last)?);
    Ok(Solid::loft(
        (first, a.first().ok_or("Perfil")?),
        (last, b.first().ok_or("Perfil")?),
        true,
        true,
        "loft",
    )?)
}
fn caps(solid: &Solid) -> Vec<usize> {
    let names = solid.face_names().iter().enumerate();
    names
        .filter(|(_, name)| name.ends_with(":start") || name.ends_with(":end"))
        .map(|(index, _)| index)
        .collect()
}
fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: Vector, b: Vector) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
/// Volume do fecho convexo pela fórmula do prismatoide; a seção média é a soma de Minkowski
/// da metade do retângulo com a metade do círculo.
fn hull_volume(width: f64, depth: f64, radius: f64, height: f64) -> f64 {
    let middle = width * depth / 4.
        + (width + depth) * radius / 2.
        + std::f64::consts::PI * radius * radius / 4.;
    height / 6. * (width * depth + 4. * middle + std::f64::consts::PI * radius * radius)
}
/// Planos tangentes da superfície lateral exata, como (ponto, normal externa).
fn tangent_planes(width: f64, depth: f64, radius: f64, height: f64) -> Vec<(Vector, Vector)> {
    let corners = [
        [width / 2., depth / 2., 0.],
        [-width / 2., depth / 2., 0.],
        [-width / 2., -depth / 2., 0.],
        [width / 2., -depth / 2., 0.],
    ];
    let mut planes = vec![];
    for (quadrant, corner) in corners.into_iter().enumerate() {
        for step in 0..=200 {
            let angle = (quadrant as f64 + f64::from(step) / 200.) * std::f64::consts::FRAC_PI_2;
            let (sin, cos) = angle.sin_cos();
            let normal = cross(
                sub([radius * cos, radius * sin, height], corner),
                [-sin, cos, 0.],
            );
            let size = dot(normal, normal).sqrt();
            let sign = if dot(normal, [cos, sin, 0.]) < 0. {
                -1.
            } else {
                1.
            };
            planes.push((corner, normal.map(|v| sign * v / size)));
        }
    }
    planes
}
/// Distância até a superfície lateral: positiva dentro do corpo original.
fn depth_below(planes: &[(Vector, Vector)], point: Vector) -> f64 {
    planes
        .iter()
        .map(|(at, normal)| dot(sub(*at, point), *normal))
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn rectangle_to_circle_is_the_exact_convex_hull_in_any_profile_order() -> Result<()> {
    for (width, depth, radius, height) in [(40., 30., 10., 30.), (60., 20., 12., 8.)] {
        let (a, b) = (rectangle(width, depth)?, circle(radius, height)?);
        let expected = hull_volume(width, depth, radius, height);
        for mut solid in [developable(&a, &b)?, developable(&b, &a)?] {
            assert!(
                (solid.volume()? - expected).abs() < 1e-3,
                "{} ≠ {expected}",
                solid.volume()?
            );
            // Duas tampas, quatro triângulos e quatro cones, sem aberturas.
            assert_eq!(solid.face_names().len(), 10);
            assert_eq!(caps(&solid).len(), 2);
            let (mesh, _) = solid.tessellate_with_faces(0.05, 0.2)?;
            assert_eq!(mesh.topology().boundary_edges, 0);
        }
    }
    Ok(())
}

#[test]
fn cones_are_meshed_as_regular_fans_that_follow_the_exact_surface() -> Result<()> {
    let (width, depth, radius, height) = (80., 60., 20., 60.);
    let mut solid = developable(&rectangle(width, depth)?, &circle(radius, height)?)?;
    let (mesh, faces) = solid.tessellate_with_faces(0.05, 0.25)?;
    assert_eq!(mesh.topology().boundary_edges, 0);
    // Sem pontos internos, cada cone é um leque do canto até o arco.
    assert!(faces.len() < 400, "{} triângulos", faces.len());
    let planes = tangent_planes(width, depth, radius, height);
    for triangle in mesh.triangles() {
        let points = triangle.map(|index| mesh.vertices()[index as usize]);
        if points.iter().all(|p| p[2] == points[0][2]) {
            continue;
        }
        // Um triângulo que cortasse a superfície por dentro apareceria aqui, mesmo com
        // os três vértices sobre ela.
        let center = std::array::from_fn(|axis| points.iter().map(|p| p[axis]).sum::<f64>() / 3.);
        let error = depth_below(&planes, center);
        assert!(error.abs() < 0.1, "{points:?}: {error}");
    }
    Ok(())
}

#[test]
fn inclined_and_off_center_profiles_keep_a_closed_convex_transition() -> Result<()> {
    let base = rectangle(40., 30.)?;
    let plane = Plane::from_normal([6., -3., 28.], [0.2, 0.1, 1.])?;
    let mut top = Sketch::new("Topo", plane);
    top.add_circle([0., 0.], 9., false)?;
    let mut solid = developable(&base, &top)?;
    assert_eq!(solid.face_names().len(), 10);
    assert_eq!(
        solid
            .tessellate_with_faces(0.05, 0.2)?
            .0
            .topology()
            .boundary_edges,
        0
    );
    Ok(())
}

#[test]
fn other_profile_pairs_keep_the_common_transition() -> Result<()> {
    // Dois círculos: não há lados, e o tronco de cone continua exato.
    let (a, b) = (circle(5., 0.)?, circle(3., 20.)?);
    let expected = std::f64::consts::PI * 20. * (25. + 15. + 9.) / 3.;
    assert!((developable(&a, &b)?.volume()? - expected).abs() < 1e-5);
    // Perfil côncavo: sem plano de apoio em algum lado.
    let mut concave = Sketch::new("Base", Plane::xy());
    let corners = [[-10., -10.], [10., -10.], [10., 10.], [0., 0.], [-10., 10.]];
    for (index, start) in corners.iter().enumerate() {
        concave.add_line(*start, corners[(index + 1) % corners.len()], false)?;
    }
    let top = circle(3., 20.)?;
    let solid = developable(&concave, &top)?;
    // Cinco laterais e duas tampas; a construção desenvolvível teria doze faces.
    assert_eq!(solid.face_names().len(), 7);
    Ok(())
}

#[test]
fn shell_walls_are_uniform_with_flat_rims_inward_and_outward() -> Result<()> {
    let thickness = 2.;
    for (width, depth, radius, height) in [
        (40., 30., 10., 30.),
        (20., 20., 6., 25.),
        (60., 20., 12., 8.),
    ] {
        let solid = developable(&rectangle(width, depth)?, &circle(radius, height)?)?;
        let openings = caps(&solid);
        let planes = tangent_planes(width, depth, radius, height);
        for outward in [false, true] {
            let mut shell = solid.shell(&openings, thickness, outward, "casca")?;
            let (mesh, _) = shell.tessellate_with_faces(0.05, 0.2)?;
            assert_eq!(mesh.topology().boundary_edges, 0);
            let wall = if outward { -thickness } else { thickness };
            for vertex in mesh.vertices() {
                // Todo vértice está na superfície original ou na parede deslocada, e as
                // bordas ficam nos planos dos perfis.
                let depth_inside = depth_below(&planes, *vertex);
                assert!(
                    depth_inside.abs() < 0.03 || (depth_inside - wall).abs() < 0.03,
                    "{vertex:?}: {depth_inside}, parede {wall}, {width}×{depth}"
                );
                assert!(
                    vertex[2] > -1e-6 && vertex[2] < height + 1e-6,
                    "{vertex:?} fora dos perfis"
                );
            }
            assert!(shell.volume()? > 0.);
        }
    }
    Ok(())
}

#[test]
fn shell_keeps_a_floor_of_the_requested_thickness_on_the_closed_profile() -> Result<()> {
    let (width, depth, radius, height) = (40., 30., 10., 30.);
    let solid = developable(&rectangle(width, depth)?, &circle(radius, height)?)?;
    let openings = caps(&solid);
    let open = solid.shell(&openings, 2., false, "aberta")?.volume()?;
    // Só o círculo aberto: a base retangular fica como fundo.
    let mut names = solid.face_names().iter().enumerate();
    let top = names
        .find(|(_, name)| name.ends_with(":end"))
        .ok_or("Tampa")?
        .0;
    let closed = solid.shell(&[top], 2., false, "fundo")?;
    let floor = closed.volume()? - open;
    // O fundo é a seção interna a 2 mm, um pouco menor que a base menos as paredes.
    assert!(floor > 1500. && floor < 2400., "{floor}");
    Ok(())
}

#[test]
fn shell_refuses_a_thickness_that_consumes_the_body() -> Result<()> {
    let solid = developable(&rectangle(40., 30.)?, &circle(10., 30.)?)?;
    let openings = caps(&solid);
    assert!(solid.shell(&openings, 25., false, "casca").is_err());
    Ok(())
}
