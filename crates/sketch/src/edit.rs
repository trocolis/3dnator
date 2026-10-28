//! Intenções de edição do rascunho, independentes da interface. A seleção contém
//! identificadores de entidades, na ordem escolhida: nos padrões/espelho, o primeiro
//! elemento é copiado e o segundo define direção, centro ou eixo. Valores estão em mm.
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Move,
    Offset,
    Fillet,
    Chamfer,
    Extend,
    Mirror,
    CircularPattern,
    RectangularPattern,
    Dimension,
    HorizontalVertical,
    Coincident,
    Tangent,
    Equal,
    Parallel,
    Perpendicular,
    Construction,
    FixPoint,
    Delete,
}
impl Action {
    pub fn needs_value(self) -> bool {
        matches!(
            self,
            Self::Offset
                | Self::Fillet
                | Self::Chamfer
                | Self::RectangularPattern
                | Self::Dimension
        )
    }
    pub fn is_pattern(self) -> bool {
        matches!(self, Self::CircularPattern | Self::RectangularPattern)
    }
    pub fn selection_count(self) -> usize {
        match self {
            Self::Offset
            | Self::Move
            | Self::Dimension
            | Self::HorizontalVertical
            | Self::Construction
            | Self::FixPoint => 1,
            Self::Delete => 1,
            _ => 2,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub value_mm: f64,
    /// Número total de ocorrências, incluindo a original.
    pub count: usize,
    pub reference: bool,
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub created: Vec<u64>,
    /// Cota criada/alterada, para o documento vincular sua expressão paramétrica.
    pub dimension: Option<u64>,
}

/// Uma intenção inválida nunca deixa meia edição: ferramentas como arredondar podem
/// precisar modificar duas linhas e criar arco/restrições antes da validação final.
pub fn apply(
    sketch: &mut Sketch,
    action: Action,
    selected: &[u64],
    options: Options,
) -> Result<Outcome> {
    let mut candidate = sketch.clone();
    let result = apply_inner(&mut candidate, action, selected, options)?;
    candidate.validate()?;
    *sketch = candidate;
    Ok(result)
}

fn apply_inner(
    sketch: &mut Sketch,
    action: Action,
    selected: &[u64],
    options: Options,
) -> Result<Outcome> {
    if selected.len() != action.selection_count()
        && !(matches!(action, Action::Construction | Action::Delete | Action::Move)
            && !selected.is_empty())
        && !(matches!(action, Action::Fillet | Action::Chamfer) && selected.len() >= 2)
    {
        return Err(SketchError(format!(
            "Selecione {} entidade(s) do rascunho.",
            action.selection_count()
        )));
    }
    let mut unique = HashSet::new();
    for id in selected {
        sketch.entity(*id)?;
        if !unique.insert(*id) {
            return Err(SketchError("Selecione entidades diferentes.".into()));
        }
    }
    if action.needs_value() && !options.value_mm.is_finite() {
        return Err(SketchError("O valor deve ser finito.".into()));
    }
    let first = selected[0];
    let mut outcome = Outcome::default();
    match action {
        Action::Move => {
            *sketch = movement::translated(sketch, selected, [options.value_mm, 0.])?;
        }
        Action::Offset => {
            outcome
                .created
                .push(tools::offset_entity(sketch, first, options.value_mm)?)
        }
        Action::Fillet | Action::Chamfer => {
            outcome.created =
                corners::refine(sketch, selected, options.value_mm, action == Action::Fillet)?
        }
        Action::Extend => {
            let (a, b) = sketch.line(first)?;
            let (c, d) = sketch.line(selected[1])?;
            let (_, t, _) = profiles::line_intersection(a, b, c, d).ok_or_else(|| {
                SketchError("Linhas paralelas não podem ser estendidas até se encontrar.".into())
            })?;
            // A extremidade fora do segmento é determinada pela interseção, não pela
            // ordem interna dos pontos da linha.
            tools::extend_line(sketch, first, selected[1], if t < 0. { a } else { b })?;
        }
        Action::Mirror => {
            let (a, b) = sketch.line(selected[1])?;
            outcome.created = tools::mirror(sketch, &[first], a, b)?;
        }
        Action::RectangularPattern => {
            let (a, b) = sketch.line(selected[1])?;
            outcome.created = tools::pattern_linear(
                sketch,
                &[first],
                sub(b, a),
                options.count,
                options.value_mm,
            )?;
        }
        Action::CircularPattern => {
            let center = sketch.radial(selected[1])?.0;
            outcome.created = tools::pattern_circular(sketch, &[first], center, options.count)?;
        }
        Action::Construction => {
            for entity in &mut sketch.entities {
                if unique.contains(&entity.id) {
                    if entity.infinite {
                        return Err(SketchError(
                            "Uma reta infinita deve permanecer como referência de construção."
                                .into(),
                        ));
                    }
                    entity.construction = !entity.construction;
                }
            }
        }
        Action::Delete => {
            sketch.entities.retain(|e| !unique.contains(&e.id));
            sketch.prune_unused_points();
        }
        Action::Dimension => {
            if sketch.entity(first)?.infinite {
                return Err(SketchError(
                    "Uma reta infinita não tem comprimento para cotar.".into(),
                ));
            }
            let kind = match sketch.entity(first)?.curve {
                Curve::Line { start, end } => ConstraintKind::Distance {
                    a: start,
                    b: end,
                    value: options.value_mm,
                },
                _ => ConstraintKind::Radius {
                    circle: first,
                    value: options.value_mm,
                },
            };
            let old = sketch
                .constraints
                .iter_mut()
                .find(|c| match (&c.kind, &kind) {
                    (
                        ConstraintKind::Distance { a, b, .. },
                        ConstraintKind::Distance { a: x, b: y, .. },
                    ) => a == x && b == y,
                    (
                        ConstraintKind::Radius { circle, .. },
                        ConstraintKind::Radius { circle: other, .. },
                    ) => circle == other,
                    _ => false,
                });
            let id = if let Some(old) = old {
                old.kind = kind;
                old.reference = options.reference;
                old.id
            } else {
                sketch.add_constraint(kind, options.reference)?
            };
            if options.reference {
                sketch.dimension_expressions.remove(&id);
            }
            outcome.dimension = Some(id);
        }
        _ => {
            let kind = match action {
                Action::HorizontalVertical => {
                    let (a, b) = sketch.line(first)?;
                    if (b[0] - a[0]).abs() >= (b[1] - a[1]).abs() {
                        ConstraintKind::Horizontal { line: first }
                    } else {
                        ConstraintKind::Vertical { line: first }
                    }
                }
                Action::FixPoint => {
                    let point = sketch
                        .entity(first)?
                        .point_ids()
                        .first()
                        .copied()
                        .ok_or_else(|| {
                            SketchError("A entidade não possui ponto de apoio.".into())
                        })?;
                    ConstraintKind::Fixed {
                        point,
                        position: sketch.point(point)?,
                    }
                }
                Action::Coincident => {
                    // Para linhas/arcos, usa as pontas mais próximas; para círculos,
                    // o apoio é o centro. A escolha é determinística e não funde ids.
                    let left = anchors(sketch.entity(first)?);
                    let right = anchors(sketch.entity(selected[1])?);
                    if left.iter().any(|a| right.contains(a)) {
                        return Err(SketchError(
                            "As entidades já compartilham uma ponta.".into(),
                        ));
                    }
                    let mut best = None;
                    for a in left {
                        for b in &right {
                            let b = *b;
                            let distance = distance(sketch.point(a)?, sketch.point(b)?);
                            if best.is_none_or(|(d, _, _)| distance < d) {
                                best = Some((distance, a, b));
                            }
                        }
                    }
                    let (_, a, b) = best.ok_or_else(|| {
                        SketchError("As entidades já compartilham suas pontas.".into())
                    })?;
                    ConstraintKind::Coincident { a, b }
                }
                Action::Tangent => {
                    let (line, arc) = if sketch.line(first).is_ok() {
                        (first, selected[1])
                    } else {
                        (selected[1], first)
                    };
                    sketch.line(line)?;
                    if !matches!(sketch.entity(arc)?.curve, Curve::Arc { .. }) {
                        return Err(SketchError(
                            "Selecione uma linha e um arco com extremidade compartilhada.".into(),
                        ));
                    }
                    let ends = anchors(sketch.entity(arc)?);
                    let point = anchors(sketch.entity(line)?)
                        .into_iter()
                        .find(|p| ends.contains(p))
                        .ok_or_else(|| {
                            SketchError(
                                "A linha e o arco precisam compartilhar uma extremidade.".into(),
                            )
                        })?;
                    ConstraintKind::Tangent { line, arc, point }
                }
                Action::Equal | Action::Parallel | Action::Perpendicular => {
                    sketch.line(first)?;
                    sketch.line(selected[1])?;
                    match action {
                        Action::Equal => ConstraintKind::EqualLength {
                            a: first,
                            b: selected[1],
                        },
                        Action::Parallel => ConstraintKind::Parallel {
                            a: first,
                            b: selected[1],
                        },
                        _ => ConstraintKind::Perpendicular {
                            a: first,
                            b: selected[1],
                        },
                    }
                }
                _ => return Err(SketchError("Intenção de restrição inválida.".into())),
            };
            if sketch
                .constraints
                .iter()
                .any(|c| c.kind == kind && !c.reference)
            {
                return Err(SketchError("Essa restrição já está aplicada.".into()));
            }
            sketch.add_constraint(kind, false)?;
        }
    }
    Ok(outcome)
}

fn anchors(entity: &Entity) -> Vec<u64> {
    match entity.curve {
        Curve::Line { start, end } | Curve::Arc { start, end, .. } => vec![start, end],
        Curve::Circle { center, .. } => vec![center],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(value_mm: f64) -> Options {
        Options {
            value_mm,
            count: 4,
            reference: false,
        }
    }

    #[test]
    fn rejected_edits_preserve_geometry_constraints_and_identifiers() -> Result<()> {
        let mut sketch = Sketch::new("Canto", Plane::xy());
        let lines = sketch.add_rectangle([0., 0.], [40., 20.], false)?;
        let before = format!("{sketch:?}");
        for (action, selection, value) in [
            (Action::Fillet, vec![lines[0], lines[1]], 100.),
            (Action::Offset, vec![lines[0]], f64::NAN),
            (Action::Parallel, vec![lines[0], lines[0]], 0.),
            (Action::Coincident, vec![lines[0], lines[1]], 0.),
            (Action::Dimension, vec![], 10.),
        ] {
            assert!(apply(&mut sketch, action, &selection, options(value)).is_err());
            assert_eq!(format!("{sketch:?}"), before);
        }
        // A falha por limite pode ocorrer depois de a ferramenta começar a criar
        // pontos. A transação também precisa desfazer essa alteração parcial.
        let saved = sketch.next_id;
        sketch.next_id = u64::MAX - 1;
        let before = format!("{sketch:?}");
        assert!(apply(&mut sketch, Action::Offset, &[lines[0]], options(5.)).is_err());
        assert_eq!(format!("{sketch:?}"), before);
        sketch.next_id = saved;
        Ok(())
    }

    #[test]
    fn patterns_mirror_and_offset_keep_the_source_and_use_the_selected_reference() -> Result<()> {
        let mut sketch = Sketch::new("Cópias", Plane::xy());
        let source = sketch.add_circle([10., 5.], 2., true)?;
        let axis = sketch.add_line([0., -10.], [0., 10.], true)?;
        let center = sketch.add_circle([0., 0.], 1., true)?;
        let mirrored = apply(&mut sketch, Action::Mirror, &[source, axis], options(0.))?.created[0];
        assert_eq!(sketch.circle(mirrored)?, ([-10., 5.], 2.));
        let linear = apply(
            &mut sketch,
            Action::RectangularPattern,
            &[source, axis],
            options(10.),
        )?
        .created;
        assert_eq!(sketch.circle(linear[2])?, ([10., 35.], 2.));
        let ring = apply(
            &mut sketch,
            Action::CircularPattern,
            &[source, center],
            options(0.),
        )?
        .created;
        assert!(distance(sketch.circle(ring[0])?.0, [-5., 10.]) < 1e-10);
        let offset = apply(&mut sketch, Action::Offset, &[source], options(-1.))?.created[0];
        assert_eq!(sketch.circle(offset)?, ([10., 5.], 1.));
        assert_eq!(sketch.circle(source)?, ([10., 5.], 2.));
        assert!(sketch.entity(mirrored)?.construction);
        sketch.validate()
    }

    #[test]
    fn dimensions_reuse_the_constraint_and_reference_dimensions_stop_driving() -> Result<()> {
        let mut sketch = Sketch::new("Cota", Plane::xy());
        let line = sketch.add_line([0., 0.], [10., 0.], false)?;
        let id = apply(&mut sketch, Action::Dimension, &[line], options(20.))?
            .dimension
            .ok_or_else(|| SketchError("Cota ausente".into()))?;
        sketch.dimension_expressions.insert(id, "20 mm".into());
        let report = solver::solve(&sketch)?;
        assert_ne!(report.status, solver::Status::Conflict);
        sketch = report.sketch;
        let (a, b) = sketch.line(line)?;
        assert!((distance(a, b) - 20.).abs() < 1e-6);
        let outcome = apply(
            &mut sketch,
            Action::Dimension,
            &[line],
            Options {
                reference: true,
                ..options(999.)
            },
        )?;
        assert_eq!(outcome.dimension, Some(id));
        assert_eq!(sketch.constraints.len(), 1);
        assert!(!sketch.dimension_expressions.contains_key(&id));
        let report = solver::solve(&sketch)?;
        let (a, b) = report.sketch.line(line)?;
        assert!((distance(a, b) - 20.).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn constraints_solve_geometry_and_do_not_duplicate_existing_rules() -> Result<()> {
        let mut sketch = Sketch::new("Restrições", Plane::xy());
        let first = sketch.add_line([0., 0.], [10., 1.], false)?;
        let second = sketch.add_line([12., 2.], [25., 4.], false)?;
        for (action, ids) in [
            (Action::HorizontalVertical, vec![first]),
            (Action::Parallel, vec![first, second]),
            (Action::Equal, vec![first, second]),
            (Action::Coincident, vec![first, second]),
        ] {
            apply(&mut sketch, action, &ids, options(0.))?;
        }
        let report = solver::solve(&sketch)?;
        assert_ne!(report.status, solver::Status::Conflict);
        assert!(report.max_error < 1e-6);
        assert!(apply(&mut sketch, Action::Parallel, &[first, second], options(0.)).is_err());
        let (a, b) = report.sketch.line(first)?;
        let (c, d) = report.sketch.line(second)?;
        assert!((a[1] - b[1]).abs() < 1e-6);
        assert!(distance(b, c) < 1e-6);
        assert!((distance(a, b) - distance(c, d)).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn tangent_requires_a_shared_arc_endpoint_and_preserves_valid_geometry() -> Result<()> {
        let mut sketch = Sketch::new("Tangência", Plane::xy());
        let center = sketch.add_point([0., 0.])?;
        let start = sketch.add_point([5., 0.])?;
        let end = sketch.add_point([0., 5.])?;
        let far = sketch.add_point([5., 10.])?;
        let arc = sketch.add_curve(
            Curve::Arc {
                center,
                start,
                end,
                clockwise: false,
            },
            false,
        )?;
        let line = sketch.add_curve(Curve::Line { start, end: far }, false)?;
        apply(&mut sketch, Action::Tangent, &[line, arc], options(0.))?;
        assert!(solver::max_error(&sketch)? < 1e-9);
        let other = sketch.add_line([5., 0.], [5., 10.], false)?;
        let before = format!("{sketch:?}");
        assert!(apply(&mut sketch, Action::Tangent, &[other, arc], options(0.)).is_err());
        assert_eq!(format!("{sketch:?}"), before);
        Ok(())
    }

    #[test]
    fn extend_chooses_the_outside_endpoint_and_corner_edits_remain_closed() -> Result<()> {
        let mut sketch = Sketch::new("Estender", Plane::xy());
        let line = sketch.add_line([0., 0.], [5., 0.], false)?;
        let target = sketch.add_line([10., -5.], [10., 5.], true)?;
        apply(&mut sketch, Action::Extend, &[line, target], options(0.))?;
        assert_eq!(sketch.line(line)?, ([0., 0.], [10., 0.]));
        for action in [Action::Fillet, Action::Chamfer] {
            let mut corner = Sketch::new("Canto", Plane::xy());
            let lines = corner.add_rectangle([0., 0.], [40., 20.], false)?;
            apply(&mut corner, action, &lines[..2], options(3.))?;
            assert_eq!(profiles::regions(&corner)?.len(), 1);
            assert!(solver::max_error(&corner)? < 1e-6);
        }
        Ok(())
    }
}
