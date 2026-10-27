//! Vértices virtuais e centros das células, sem modificar entidades ou suas cotas.
//! O aparador e a captura compartilham os mesmos encontros analíticos das curvas.
use crate::{curve_geometry::intersections, profiles, *};

#[derive(Debug, Clone, Default)]
pub struct References {
    pub vertices: Vec<Point2>,
    pub centers: Vec<Point2>,
}

pub fn references(sketch: &Sketch) -> Result<References> {
    let curves = sketch
        .entities
        .iter()
        .map(|e| profiles::segment(sketch, e))
        .collect::<Result<Vec<_>>>()?;
    let mut result = References::default();
    for (i, a) in curves.iter().enumerate() {
        for b in &curves[i + 1..] {
            for p in intersections(a, b) {
                if !result.vertices.iter().any(|q| distance(*q, p) < 1e-6) {
                    result.vertices.push(p);
                }
            }
        }
    }
    // Geometria de construção também serve de apoio: não cria perfil de extrusão,
    // mas um triângulo/retângulo auxiliar ainda possui centro para captura.
    let mut contours = sketch.clone();
    for entity in &mut contours.entities {
        entity.construction = false;
    }
    for region in areas::regions(&contours)? {
        // O centro pertence ao contorno externo. Um furo interno não desloca
        // o encontro das diagonais nem o baricentro do triângulo exterior.
        let mut vertices = Vec::new();
        for segment in &region.outer.segments {
            let profiles::Segment::Line { a, .. } = segment else {
                vertices.clear();
                break;
            };
            vertices.push([a[0], a[1], 0.]);
        }
        if let Some(center) = nator_geometry::polygon_center(&vertices) {
            let p = [center[0], center[1]];
            if !result.centers.iter().any(|q| distance(*q, p) < 1e-6) {
                result.centers.push(p);
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossings_are_exact_vertices_without_splitting_or_changing_source_curves() -> Result<()> {
        let mut sketch = Sketch::new("Encontros", Plane::xy());
        sketch.add_line([-20., 0.], [20., 0.], false)?;
        sketch.add_circle([0., 0.], 10., false)?;
        let before = (sketch.points.len(), sketch.entities.len(), sketch.next_id);
        let refs = references(&sketch)?;
        assert_eq!(refs.vertices.len(), 2);
        for p in [[-10., 0.], [10., 0.]] {
            assert!(refs.vertices.iter().any(|q| distance(*q, p) < 1e-9));
        }
        assert_eq!(
            before,
            (sketch.points.len(), sketch.entities.len(), sketch.next_id)
        );
        let snap = snapping::find(
            &sketch,
            [10.1, 0.1],
            0.5,
            &snapping::Tracker::default(),
            &[],
        )?
        .ok_or_else(|| SketchError("Vértice de interseção ausente".into()))?;
        assert_eq!(snap.kind, snapping::Kind::Intersection);
        assert!(distance(snap.point, [10., 0.]) < 1e-9);
        Ok(())
    }

    #[test]
    fn triangle_and_rectangle_centers_are_capturable_while_open_shapes_are_not() -> Result<()> {
        let mut sketch = Sketch::new("Centros", Plane::xy());
        for (a, b) in [
            ([0., 0.], [12., 0.]),
            ([12., 0.], [0., 6.]),
            ([0., 6.], [0., 0.]),
        ] {
            sketch.add_line(a, b, false)?;
        }
        sketch.add_rectangle([20., 0.], [40., 10.], true)?;
        sketch.add_line([50., 0.], [60., 0.], false)?;
        let refs = references(&sketch)?;
        assert_eq!(refs.centers.len(), 2);
        for p in [[4., 2.], [30., 5.]] {
            let snap = snapping::find(
                &sketch,
                add(p, [0.1, 0.1]),
                0.5,
                &snapping::Tracker::default(),
                &[],
            )?
            .ok_or_else(|| SketchError("Centro ausente".into()))?;
            assert_eq!(snap.kind, snapping::Kind::RegionCenter);
            assert!(distance(snap.point, p) < 1e-9);
        }
        Ok(())
    }

    #[test]
    fn finite_crossings_and_t_junctions_offer_vertices_without_extending_curves() -> Result<()> {
        let mut sketch = Sketch::new("Cruzamentos", Plane::xy());
        for (a, b) in [
            ([0., 0.], [20., 0.]),
            ([7., -5.], [7., 5.]),
            ([10., 0.], [10., 5.]),
            ([30., -5.], [30., 5.]),
        ] {
            sketch.add_line(a, b, false)?;
        }
        let refs = references(&sketch)?;
        assert_eq!(refs.vertices.len(), 2);
        assert!(refs.vertices.contains(&[7., 0.]));
        assert!(refs.vertices.contains(&[10., 0.]));
        assert!(refs.centers.is_empty());
        Ok(())
    }

    #[test]
    fn coincident_circles_do_not_create_an_arbitrary_seam_vertex() -> Result<()> {
        let mut sketch = Sketch::new("Círculos coincidentes", Plane::xy());
        sketch.add_circle([0., 0.], 10., false)?;
        sketch.add_circle([0., 0.], 10., false)?;
        assert!(references(&sketch)?.vertices.is_empty());
        Ok(())
    }
}
