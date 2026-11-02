//! Silhuetas em 2D: a sombra de um corpo em uma direção e o seu contorno afastado.
//!
//! Serve para recortar a passagem de um componente. O deslocamento exato de B-rep falha em
//! componentes STEP reais; projetar a malha e unir os triângulos em 2D não depende da
//! qualidade do modelo. O resultado é um contorno de retas, com erro limitado por
//! `TOLERANCE_MM`, muito abaixo do que um bico de impressão reproduz.
use crate::{Point3, cross, norm};
use i_overlay::{
    core::fill_rule::FillRule,
    float::simplify::SimplifyShape,
    mesh::float::{
        outline::offset::OutlineOffset,
        style::{LineJoin, OutlineStyle},
    },
};

pub type Contour = Vec<[f64; 2]>;

/// Desvio máximo admitido ao retirar pontos de um contorno.
pub const TOLERANCE_MM: f64 = 0.01;
/// Triângulo vazio na projeção: uma face vista de lado.
const EMPTY_TRIANGLE_MM2: f64 = 1e-9;
/// Contorno menor que isto é resto numérico, não um volume do componente.
const EMPTY_CONTOUR_MM2: f64 = 1e-4;
/// Cantos mais agudos que isto seriam pontas longas na folga e viram chanfro.
const SHARPEST_CORNER_RAD: f64 = 0.35;

/// Área com sinal: positiva no sentido anti-horário.
pub fn area(contour: &[[f64; 2]]) -> f64 {
    let doubled: f64 = contour
        .iter()
        .zip(contour.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum();
    doubled / 2.
}

/// Base `(u, v)` do plano perpendicular a `normal`, com `u × v = normal`. Usa o eixo principal
/// menos alinhado, para que direções retas deem coordenadas retas.
pub fn plane_basis(normal: Point3) -> Option<(Point3, Point3)> {
    let length = norm(normal);
    if !length.is_finite() || length < 1e-12 {
        return None;
    }
    let n = normal.map(|value| value / length);
    let smallest = (0..3).min_by(|a, b| n[*a].abs().total_cmp(&n[*b].abs()))?;
    let mut axis = [0.; 3];
    axis[smallest] = 1.;
    let v = cross(n, axis);
    let v = v.map(|value| value / norm(v));
    Some((cross(v, n), v))
}

fn outer_contours(shapes: Vec<Vec<Contour>>) -> Vec<Contour> {
    shapes
        .into_iter()
        .filter_map(|shape| shape.into_iter().next())
        .filter(|contour| contour.len() >= 3 && area(contour).abs() >= EMPTY_CONTOUR_MM2)
        .collect()
}

/// Distância de `p` à reta que passa por `a` e `b`.
fn deviation(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length = dx.hypot(dy);
    if length < 1e-12 {
        return (p[0] - a[0]).hypot(p[1] - a[1]);
    }
    ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs() / length
}

/// Douglas-Peucker em contorno fechado, aberto no ponto mais distante do primeiro.
fn simplified(contour: &[[f64; 2]], tolerance: f64) -> Contour {
    let count = contour.len();
    if count < 4 {
        return contour.to_vec();
    }
    let reach = |i: usize| (contour[i][0] - contour[0][0]).hypot(contour[i][1] - contour[0][1]);
    let far = (0..count)
        .max_by(|a, b| reach(*a).total_cmp(&reach(*b)))
        .unwrap_or(0);
    let mut keep = vec![false; count];
    keep[0] = true;
    keep[far] = true;
    let mut pending = vec![(0, far), (far, count)];
    while let Some((start, end)) = pending.pop() {
        let (a, b) = (contour[start], contour[end % count]);
        let worst = (start + 1..end)
            .map(|i| (i, deviation(contour[i], a, b)))
            .max_by(|x, y| x.1.total_cmp(&y.1));
        if let Some((index, distance)) = worst
            && distance > tolerance
        {
            keep[index] = true;
            pending.push((start, index));
            pending.push((index, end));
        }
    }
    contour
        .iter()
        .zip(keep)
        .filter_map(|(point, kept)| kept.then_some(*point))
        .collect()
}

/// Simplifica sem deixar o contorno cruzar a si mesmo: a união refaz o que a retirada de
/// pontos tiver dobrado em gargalos estreitos.
fn clean(contours: Vec<Contour>) -> Vec<Contour> {
    let reduced = contours
        .iter()
        .map(|contour| simplified(contour, TOLERANCE_MM))
        .collect::<Vec<_>>();
    outer_contours(reduced.simplify_shape(FillRule::NonZero))
}

/// Contornos externos da união de triângulos já projetados. Vãos internos ficam de fora: um
/// pino no meio do recorte não imprime e não ajuda a guiar o componente.
pub fn silhouette(triangles: impl IntoIterator<Item = [[f64; 2]; 3]>) -> Vec<Contour> {
    let contours = triangles
        .into_iter()
        .filter_map(|triangle| {
            let signed = area(&triangle);
            let mut contour = triangle.to_vec();
            (signed.abs() >= EMPTY_TRIANGLE_MM2).then(|| {
                if signed < 0. {
                    contour.reverse();
                }
                contour
            })
        })
        .collect::<Vec<_>>();
    clean(outer_contours(contours.simplify_shape(FillRule::NonZero)))
}

/// Contornos afastados de `distance` mm para fora. Contornos que se encostam viram um só, e
/// cantos vivos continuam vivos: um componente reto dá um recorte reto.
pub fn grown(contours: &[Contour], distance: f64) -> Vec<Contour> {
    if distance < 1e-9 {
        return contours.to_vec();
    }
    // No sentido horário, o contorno seria um vão: o afastamento o faria encolher.
    let oriented = |contour: &Contour| {
        let mut contour = contour.clone();
        if area(&contour) < 0. {
            contour.reverse();
        }
        vec![contour]
    };
    let shapes = contours.iter().map(oriented).collect::<Vec<_>>();
    let style = OutlineStyle::new(distance).line_join(LineJoin::Miter(SHARPEST_CORNER_RAD));
    clean(outer_contours(shapes.outline(&style)))
}

/// Contorno deslocado: para fora com `distance` positiva, para dentro com negativa. `None`
/// quando ele some ou se divide, porque aí não há como acompanhar o original.
pub fn shifted(contour: &[[f64; 2]], distance: f64) -> Option<Contour> {
    let mut result = if distance >= 0. {
        grown(&[contour.to_vec()], distance)
    } else {
        // Dentro é fora do complemento: crescer a moldura em volta fecha o vão.
        let (mut low, mut high) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for point in contour {
            for axis in 0..2 {
                low[axis] = low[axis].min(point[axis]);
                high[axis] = high[axis].max(point[axis]);
            }
        }
        let frame = 1. - distance;
        let (a, b) = (low.map(|v| v - frame), high.map(|v| v + frame));
        let mut gap = contour.to_vec();
        if area(&gap) > 0. {
            gap.reverse();
        }
        let shape = vec![
            vec![[a[0], a[1]], [b[0], a[1]], [b[0], b[1]], [a[0], b[1]]],
            gap,
        ];
        let style = OutlineStyle::new(-distance).line_join(LineJoin::Miter(SHARPEST_CORNER_RAD));
        let gaps = shape
            .outline(&style)
            .into_iter()
            .flat_map(|mut shape| shape.split_off(1.min(shape.len())))
            .filter(|contour| area(contour).abs() >= EMPTY_CONTOUR_MM2)
            .collect();
        clean(gaps)
    };
    if result.len() == 1 {
        result.pop()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    fn rectangle(x: f64, y: f64, w: f64, h: f64) -> [[[f64; 2]; 3]; 2] {
        [
            [[x, y], [x + w, y], [x + w, y + h]],
            [[x, y], [x + w, y + h], [x, y + h]],
        ]
    }

    #[test]
    fn triangles_in_any_orientation_merge_into_the_outer_contour() {
        let [a, mut b] = rectangle(0., 0., 10., 5.);
        b.reverse();
        // O retângulo coberto não acrescenta nada à sombra.
        let inner = rectangle(2., 1., 3., 2.);
        let result = silhouette([a, b, inner[0], inner[1]]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 4);
        assert!((area(&result[0]).abs() - 50.).abs() < 1e-6);
    }

    #[test]
    fn faces_seen_edge_on_cast_no_shadow() {
        assert!(silhouette([[[0., 0.], [5., 0.], [10., 0.]]]).is_empty());
    }

    #[test]
    fn inner_gaps_stay_filled_and_separate_parts_stay_apart() {
        // Moldura: quatro barras em volta de um vão.
        let mut frame = Vec::new();
        for (x, y, w, h) in [
            (0., 0., 10., 2.),
            (0., 8., 10., 2.),
            (0., 0., 2., 10.),
            (8., 0., 2., 10.),
        ] {
            frame.extend(rectangle(x, y, w, h));
        }
        let framed = silhouette(frame.clone());
        assert_eq!(framed.len(), 1);
        assert!((area(&framed[0]).abs() - 100.).abs() < 1e-6);
        frame.extend(rectangle(20., 0., 4., 4.));
        assert_eq!(silhouette(frame).len(), 2);
    }

    #[test]
    fn growth_keeps_straight_corners_and_the_exact_distance() {
        let [a, b] = rectangle(0., 0., 10., 5.);
        let grown = grown(&silhouette([a, b]), 0.2);
        assert_eq!(grown.len(), 1);
        assert_eq!(grown[0].len(), 4);
        assert!((area(&grown[0]).abs() - 10.4 * 5.4).abs() < 1e-6);
        assert!(grown[0].iter().flatten().fold(0., |m: f64, v| m.min(*v)) < -0.19);
    }

    #[test]
    fn contours_closer_than_twice_the_distance_join() {
        let [a, b] = rectangle(0., 0., 4., 4.);
        let [c, d] = rectangle(4.3, 0., 4., 4.);
        let apart = silhouette([a, b, c, d]);
        assert_eq!(apart.len(), 2);
        assert_eq!(grown(&apart, 0.1).len(), 2);
        assert_eq!(grown(&apart, 0.2).len(), 1);
    }

    #[test]
    fn a_contour_shifts_both_ways_or_refuses_to_split() -> Result<(), Box<dyn Error>> {
        let square = vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        let outside = shifted(&square, 1.).ok_or("Fora")?;
        assert!((area(&outside).abs() - 144.).abs() < 1e-6);
        let hole = square.iter().rev().copied().collect::<Vec<_>>();
        assert!((area(&shifted(&hole, 1.).ok_or("Fora")?).abs() - 144.).abs() < 1e-6);
        let inside = shifted(&square, -1.).ok_or("Dentro")?;
        assert!((area(&inside).abs() - 64.).abs() < 1e-6);
        // O sentido do contorno não muda o lado.
        let reversed = square.iter().rev().copied().collect::<Vec<_>>();
        assert!((area(&shifted(&reversed, -1.).ok_or("Dentro")?).abs() - 64.).abs() < 1e-6);
        assert!(shifted(&square, -6.).is_none());
        // Haltere: o gargalo some e o contorno vira dois.
        let dumbbell = vec![
            [0., 0.],
            [10., 0.],
            [10., 4.],
            [20., 4.],
            [20., 0.],
            [30., 0.],
            [30., 10.],
            [20., 10.],
            [20., 5.],
            [10., 5.],
            [10., 10.],
            [0., 10.],
        ];
        assert!(shifted(&dumbbell, -1.).is_none());
        Ok(())
    }

    #[test]
    fn a_fine_circle_loses_points_but_not_shape() {
        let radius = 10.;
        let step = std::f64::consts::TAU / 5000.;
        let ring = (0..5000).map(|i| {
            let (a, b) = (f64::from(i) * step, f64::from(i + 1) * step);
            [
                [0., 0.],
                [radius * a.cos(), radius * a.sin()],
                [radius * b.cos(), radius * b.sin()],
            ]
        });
        let result = silhouette(ring);
        assert_eq!(result.len(), 1);
        assert!(result[0].len() < 200);
        let circle = std::f64::consts::PI * radius * radius;
        assert!((area(&result[0]).abs() - circle).abs() / circle < 0.002);
    }

    #[test]
    fn the_basis_is_right_handed_and_straight_on_main_axes() -> Result<(), Box<dyn Error>> {
        for normal in [[0., 0., 1.], [0., -1., 0.], [1., 2., 2.]] {
            let (u, v) = plane_basis(normal).ok_or("Base")?;
            let n = cross(u, v);
            let unit = normal.map(|value| value / norm(normal));
            assert!((0..3).all(|i| (n[i] - unit[i]).abs() < 1e-12));
        }
        let (u, v) = plane_basis([0., 0., -1.]).ok_or("Base")?;
        assert!(
            u.iter()
                .chain(&v)
                .all(|c| c.abs() < 1e-12 || (c.abs() - 1.).abs() < 1e-12)
        );
        assert!(plane_basis([0.; 3]).is_none());
        Ok(())
    }
}
