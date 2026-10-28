//! Espelhamento com seleção explícita de eixo e deslocamentos por lado.
//! Operações transacionais: falhar em um dos lados ou alvos descarta tudo.
use crate::*;
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum OffsetMode {
    #[default]
    Normal,
    Double,
    Symmetric,
}

fn stroke(sketch: &mut Sketch, ids: &[u64], line_type: LineType) -> Result<()> {
    for id in ids {
        let entity = sketch
            .entities
            .iter_mut()
            .find(|e| e.id == *id)
            .ok_or_else(|| SketchError("Entidade copiada inexistente.".into()))?;
        if line_type == LineType::Infinite && !matches!(entity.curve, Curve::Line { .. }) {
            return Err(SketchError("Traço infinito exige somente linhas.".into()));
        }
        entity.construction = line_type != LineType::Normal;
        entity.infinite = line_type == LineType::Infinite;
    }
    Ok(())
}

/// O eixo é independente dos elementos. Apoios compartilhados e coincidências
/// internas continuam ligados na cópia, sem vínculo com os originais ou o eixo.
pub fn mirror(
    sketch: &mut Sketch,
    selected: &[u64],
    axis: u64,
    line_type: LineType,
) -> Result<Vec<u64>> {
    if selected.is_empty() || selected.contains(&axis) {
        return Err(SketchError(
            "Selecione os elementos e uma linha separada como eixo.".into(),
        ));
    }
    let (a, b) = sketch.line(axis)?;
    let direction = sub(b, a);
    let length = distance(a, b);
    if length < 1e-9 {
        return Err(SketchError("Eixo de espelhamento inválido.".into()));
    }
    let direction = scale(direction, 1. / length);
    let reflect = |p| {
        let d = sub(p, a);
        add(a, sub(scale(direction, 2. * dot(d, direction)), d))
    };
    let mut next = sketch.clone();
    let mut points = HashMap::new();
    let mut created = Vec::new();
    let mut unique = HashSet::new();
    for id in selected {
        if !unique.insert(*id) {
            return Err(SketchError("Selecione entidades diferentes.".into()));
        }
        let entity = sketch.entity(*id)?;
        for point in entity.point_ids() {
            if let std::collections::hash_map::Entry::Vacant(entry) = points.entry(point) {
                entry.insert(next.add_point(reflect(sketch.point(point)?))?);
            }
        }
        let mapped = |id| {
            points
                .get(&id)
                .copied()
                .ok_or_else(|| SketchError("Apoio da cópia inexistente.".into()))
        };
        let curve = match entity.curve {
            Curve::Line { start, end } => Curve::Line {
                start: mapped(start)?,
                end: mapped(end)?,
            },
            Curve::Circle { center, radius } => Curve::Circle {
                center: mapped(center)?,
                radius,
            },
            Curve::Arc {
                center,
                start,
                end,
                clockwise,
            } => Curve::Arc {
                center: mapped(center)?,
                start: mapped(start)?,
                end: mapped(end)?,
                clockwise: !clockwise,
            },
        };
        created.push(next.add_curve(curve, line_type != LineType::Normal)?);
    }
    for constraint in &sketch.constraints {
        if let ConstraintKind::Coincident { a, b } = constraint.kind
            && let (Some(a), Some(b)) = (points.get(&a), points.get(&b))
        {
            next.add_constraint(
                ConstraintKind::Coincident { a: *a, b: *b },
                constraint.reference,
            )?;
        }
    }
    stroke(&mut next, &created, line_type)?;
    next.validate()?;
    *sketch = next;
    Ok(created)
}

/// Normal usa distância assinada. Dupla usa uma distância positiva para cada
/// lado; Simétrica usa a mesma distância por lado, sem dividir o valor por dois.
pub fn offset(
    sketch: &mut Sketch,
    selected: &[u64],
    mode: OffsetMode,
    first: f64,
    second: f64,
    line_type: LineType,
) -> Result<Vec<u64>> {
    if selected.is_empty()
        || !first.is_finite()
        || first.abs() < 1e-9
        || (mode != OffsetMode::Normal && first <= 0.)
        || (mode == OffsetMode::Double && (!second.is_finite() || second <= 1e-9))
    {
        return Err(SketchError(
            "Informe elementos e distâncias válidas; nos dois lados, use valores positivos.".into(),
        ));
    }
    let distances = match mode {
        OffsetMode::Normal => vec![first],
        OffsetMode::Double => vec![first, -second],
        OffsetMode::Symmetric => vec![first, -first],
    };
    let mut next = sketch.clone();
    let mut created = Vec::new();
    let mut unique = HashSet::new();
    for id in selected {
        if !unique.insert(*id) {
            return Err(SketchError("Selecione entidades diferentes.".into()));
        }
    }
    for distance in distances {
        created.extend(crate::path_offset::append(&mut next, selected, distance)?);
    }
    stroke(&mut next, &created, line_type)?;
    next.validate()?;
    *sketch = next;
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_offsets_close_corners_and_reject_collapsed_contours_atomically() -> Result<()> {
        let mut sketch = Sketch::new("Contorno", Plane::xy());
        let lines = sketch.add_rectangle([0., 0.], [10., 10.], false)?;
        let created = offset(
            &mut sketch,
            &lines,
            OffsetMode::Normal,
            1.,
            0.,
            LineType::Normal,
        )?;
        assert_eq!(created.len(), 4);
        let regions = profiles::regions(&sketch)?;
        assert!(
            regions
                .iter()
                .flat_map(|r| &r.holes)
                .any(|w| (w.area().abs() - 64.).abs() < 1e-6)
        );
        for i in 0..4 {
            assert_eq!(
                sketch.entity(created[i])?.point_ids()[1],
                sketch.entity(created[(i + 1) % 4])?.point_ids()[0]
            );
        }
        let before = (sketch.next_id, sketch.points.len(), sketch.entities.len());
        assert!(
            offset(
                &mut sketch,
                &lines,
                OffsetMode::Normal,
                6.,
                0.,
                LineType::Normal
            )
            .is_err()
        );
        assert_eq!(
            before,
            (sketch.next_id, sketch.points.len(), sketch.entities.len())
        );
        Ok(())
    }
    #[test]
    fn a_line_and_arc_offset_meet_on_their_exact_supports() -> Result<()> {
        let mut sketch = Sketch::new("Contorno misto", Plane::xy());
        let center = sketch.add_point([0., 0.])?;
        let a = sketch.add_point([10., 0.])?;
        let b = sketch.add_point([-10., 0.])?;
        let arc = sketch.add_curve(
            Curve::Arc {
                center,
                start: a,
                end: b,
                clockwise: false,
            },
            false,
        )?;
        let line = sketch.add_curve(Curve::Line { start: b, end: a }, false)?;
        let created = offset(
            &mut sketch,
            &[arc, line],
            OffsetMode::Normal,
            1.,
            0.,
            LineType::Normal,
        )?;
        assert_eq!(created.len(), 2);
        assert!((sketch.radial(created[0])?.1 - 9.).abs() < 1e-6);
        let ends = sketch.entity(created[0])?.point_ids();
        let (a, b) = sketch.line(created[1])?;
        assert!(
            distance(a, sketch.point(ends[2])?) < 1e-6
                && distance(b, sketch.point(ends[1])?) < 1e-6
        );
        assert!((a[1] - 1.).abs() < 1e-6 && (b[1] - 1.).abs() < 1e-6);
        sketch.validate()
    }
    #[test]
    fn mirrored_selection_preserves_shared_corners_and_uses_the_chosen_stroke() -> Result<()> {
        let mut sketch = Sketch::new("Espelho", Plane::xy());
        let lines = sketch.add_rectangle([2., 0.], [12., 10.], false)?;
        let axis = sketch.add_line([0., -10.], [0., 20.], true)?;
        let created = mirror(&mut sketch, &lines, axis, LineType::Construction)?;
        assert_eq!(created.len(), 4);
        assert!(
            created
                .iter()
                .all(|id| sketch.entity(*id).is_ok_and(|e| e.construction))
        );
        assert_eq!(sketch.line(created[0])?, ([-2., 0.], [-12., 0.]));
        assert_eq!(
            sketch.entity(created[0])?.point_ids()[1],
            sketch.entity(created[1])?.point_ids()[0]
        );
        assert!(junctions::at(&sketch, [-12., 0.])?.is_some_and(|j| j.strong));
        Ok(())
    }

    #[test]
    fn offset_modes_use_independent_sides_and_failed_second_sides_leave_no_changes() -> Result<()> {
        let mut sketch = Sketch::new("Dois lados", Plane::xy());
        let line = sketch.add_line([0., 0.], [10., 0.], false)?;
        let created = offset(
            &mut sketch,
            &[line],
            OffsetMode::Double,
            2.,
            5.,
            LineType::Infinite,
        )?;
        assert_eq!(sketch.line(created[0])?, ([0., 2.], [10., 2.]));
        assert_eq!(sketch.line(created[1])?, ([0., -5.], [10., -5.]));
        assert!(
            created
                .iter()
                .all(|id| sketch.entity(*id).is_ok_and(|e| e.infinite))
        );
        let created = offset(
            &mut sketch,
            &[line],
            OffsetMode::Symmetric,
            3.,
            0.,
            LineType::Normal,
        )?;
        assert_eq!(sketch.line(created[0])?.0[1], 3.);
        assert_eq!(sketch.line(created[1])?.0[1], -3.);
        let circle = sketch.add_circle([30., 0.], 2., false)?;
        let before = (sketch.next_id, sketch.points.len(), sketch.entities.len());
        assert!(
            offset(
                &mut sketch,
                &[circle],
                OffsetMode::Double,
                1.,
                3.,
                LineType::Normal
            )
            .is_err()
        );
        assert_eq!(
            before,
            (sketch.next_id, sketch.points.len(), sketch.entities.len())
        );
        assert!(mirror(&mut sketch, &[circle], line, LineType::Infinite).is_err());
        assert_eq!(
            before,
            (sketch.next_id, sketch.points.len(), sketch.entities.len())
        );
        Ok(())
    }

    #[test]
    fn arc_offsets_keep_angles_and_both_radii() -> Result<()> {
        let mut sketch = Sketch::new("Arcos paralelos", Plane::xy());
        let c = sketch.add_point([0., 0.])?;
        let a = sketch.add_point([10., 0.])?;
        let b = sketch.add_point([0., 10.])?;
        let arc = sketch.add_curve(
            Curve::Arc {
                center: c,
                start: a,
                end: b,
                clockwise: false,
            },
            false,
        )?;
        let created = offset(
            &mut sketch,
            &[arc],
            OffsetMode::Symmetric,
            2.,
            0.,
            LineType::Construction,
        )?;
        assert_eq!(sketch.radial(created[0])?.1, 12.);
        assert_eq!(sketch.radial(created[1])?.1, 8.);
        sketch.validate()
    }
}
