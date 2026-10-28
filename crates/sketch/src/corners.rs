//! Cantos de uma seleção de segmentos. Cruzamentos interiores não são cantos;
//! ramificações não têm um par inequívoco para aparar e são recusadas.
use crate::*;

#[derive(Debug, Clone)]
pub struct Corner {
    pub first: u64,
    pub second: u64,
    pub point: Point2,
    /// Bissetriz unitária para o interior do canto, usada pelo manipulador.
    pub direction: Point2,
    pub half_angle: f64,
}
pub fn selected(sketch: &Sketch, entities: &[u64]) -> Result<Vec<Corner>> {
    let mut ends: Vec<(Point2, Vec<(u64, Point2)>)> = Vec::new();
    let mut unique = HashSet::new();
    for id in entities {
        let entity = sketch.entity(*id)?;
        if !unique.insert(*id) {
            return Err(SketchError("Selecione entidades diferentes.".into()));
        }
        // Arcos/círculos podem fazer parte da seleção de um contorno misto,
        // mas o refinamento atual atua nos encontros entre segmentos retos.
        if entity.infinite || !matches!(entity.curve, Curve::Line { .. }) {
            continue;
        }
        let (a, b) = sketch.line(*id)?;
        for (at, far) in [(a, b), (b, a)] {
            if let Some((_, owners)) = ends.iter_mut().find(|(p, _)| distance(*p, at) < 1e-7) {
                owners.push((*id, far));
            } else {
                ends.push((at, vec![(*id, far)]));
            }
        }
    }
    let mut result = Vec::new();
    for (point, owners) in ends {
        if owners.len() > 2 {
            return Err(SketchError("Escolha cantos sem ramificações.".into()));
        }
        let [(first, a), (second, b)] = owners.as_slice() else {
            continue;
        };
        let u = scale(sub(*a, point), 1. / distance(*a, point));
        let v = scale(sub(*b, point), 1. / distance(*b, point));
        let half_angle = dot(u, v).clamp(-1., 1.).acos() / 2.;
        if half_angle.sin().abs() < 1e-6 || half_angle.cos().abs() < 1e-6 {
            continue;
        }
        let direction = add(u, v);
        result.push(Corner {
            first: *first,
            second: *second,
            point,
            direction: scale(direction, 1. / distance(direction, [0., 0.])),
            half_angle,
        });
    }
    if result.is_empty() {
        return Err(SketchError(
            "Selecione pelo menos duas linhas que formem um canto.".into(),
        ));
    }
    Ok(result)
}

/// Aplica todos os cantos na mesma candidata. Se a soma dos cortes consumir um
/// segmento, a edição inteira falha e o documento original permanece intacto.
pub fn refine(sketch: &mut Sketch, entities: &[u64], size: f64, rounded: bool) -> Result<Vec<u64>> {
    let corners = selected(sketch, entities)?;
    let mut next = sketch.clone();
    let mut created = Vec::new();
    for corner in corners {
        created.push(tools::refine_corner(
            &mut next,
            corner.first,
            corner.second,
            size,
            rounded,
        )?);
    }
    next.validate()?;
    *sketch = next;
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_rectangle_corners_are_refined_and_overlapping_cuts_are_atomic() -> Result<()> {
        for rounded in [true, false] {
            let mut sketch = Sketch::new("Cantos", Plane::xy());
            let lines = sketch.add_rectangle([0., 0.], [20., 10.], false)?;
            let original = sketch.clone();
            let created = refine(&mut sketch, &lines, 2., rounded)?;
            assert_eq!(created.len(), 4);
            assert_eq!(profiles::regions(&sketch)?.len(), 1);
            assert!(solver::max_error(&sketch)? < 1e-6);
            let expected = if rounded {
                200. - 4. * (4. - std::f64::consts::PI)
            } else {
                192.
            };
            assert!((profiles::regions(&sketch)?[0].outer.area().abs() - expected).abs() < 1e-6);
            let mut invalid = original.clone();
            assert!(refine(&mut invalid, &lines, 6., rounded).is_err());
            assert_eq!(invalid.next_id, original.next_id);
            assert_eq!(invalid.entities.len(), original.entities.len());
            assert_eq!(invalid.line(lines[0])?, original.line(lines[0])?);
        }
        Ok(())
    }
    #[test]
    fn disconnected_corners_are_included_and_crossings_are_ignored() -> Result<()> {
        let mut sketch = Sketch::new("Separados", Plane::xy());
        let mut lines = Vec::new();
        for x in [0., 30.] {
            lines.push(sketch.add_line([x, 10.], [x, 0.], false)?);
            lines.push(sketch.add_line([x, 0.], [x + 10., 0.], false)?);
        }
        lines.push(sketch.add_line([-5., 5.], [40., 5.], false)?);
        assert_eq!(selected(&sketch, &lines)?.len(), 2);
        assert_eq!(refine(&mut sketch, &lines, 1., false)?.len(), 2);
        Ok(())
    }
}
