//! Sugestões de reparo de rascunho. Nada é corrigido implicitamente: as sugestões são
//! calculadas, apresentadas com prévia e aplicadas só por escolha explícita do usuário.
//!
//! - Entidade duplicada: mesma curva que outra (extremos ou centro e raio coincidentes).
//! - Entidade contida: linha inteira sobre outra linha colinear, que já a cobre.
//! - Folga pequena: duas pontas abertas mais próximas que a tolerância; o reparo acrescenta
//!   uma restrição de coincidência, e o solver fecha a folga respeitando as demais.
//!
//! Cruzamentos não têm reparo automático (exigem decidir onde aparar) e são só relatados.
use crate::*;
use std::collections::{HashMap, HashSet};

/// Coincidência geométrica usada para reconhecer extremos iguais.
const SAME: f64 = 1e-6;

#[derive(Debug, Clone, PartialEq)]
pub enum Fix {
    /// Remove `entity`, igual a `keeps`.
    RemoveDuplicate { entity: u64, keeps: u64 },
    /// Remove a linha `entity`, coberta pela linha `within`.
    RemoveContained { entity: u64, within: u64 },
    /// Une as pontas abertas `a` e `b`, afastadas de `gap` mm.
    CloseGap { a: u64, b: u64, gap: f64 },
}
impl Fix {
    /// Entidades e pontos envolvidos, para realce na prévia.
    pub fn entities(&self) -> Vec<u64> {
        match *self {
            Self::RemoveDuplicate { entity, keeps } => vec![entity, keeps],
            Self::RemoveContained { entity, within } => vec![entity, within],
            Self::CloseGap { .. } => Vec::new(),
        }
    }
    pub fn points(&self) -> Vec<u64> {
        match *self {
            Self::CloseGap { a, b, .. } => vec![a, b],
            _ => Vec::new(),
        }
    }
}

/// Diagnóstico completo: reparos possíveis e cruzamentos sem reparo automático.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub fixes: Vec<Fix>,
    pub crossings: Vec<Point2>,
}

fn same_line(a: (Point2, Point2), b: (Point2, Point2)) -> bool {
    (distance(a.0, b.0) < SAME && distance(a.1, b.1) < SAME)
        || (distance(a.0, b.1) < SAME && distance(a.1, b.0) < SAME)
}

/// Parâmetro de `p` sobre o segmento `a`–`b`, se `p` estiver na reta (dentro de `SAME`).
fn along(p: Point2, a: Point2, b: Point2) -> Option<f64> {
    let d = sub(b, a);
    let length = dot(d, d);
    if length < SAME * SAME {
        return None;
    }
    let t = dot(sub(p, a), d) / length;
    let foot = add(a, scale(d, t));
    (distance(foot, p) < SAME).then_some(t)
}

/// Calcula as sugestões para a tolerância de folga dada (mm).
pub fn suggest(sketch: &Sketch, tolerance: f64) -> Result<Report> {
    sketch.validate()?;
    let mut report = Report::default();
    let profile = sketch
        .entities
        .iter()
        .filter(|e| !e.construction)
        .collect::<Vec<_>>();
    let mut removed = HashSet::new();
    // Duplicadas e contidas, comparando cada par uma vez; a mais recente é a removida.
    for (i, first) in profile.iter().enumerate() {
        for second in &profile[i + 1..] {
            if removed.contains(&first.id) || removed.contains(&second.id) {
                continue;
            }
            let duplicate = match (&first.curve, &second.curve) {
                (Curve::Line { .. }, Curve::Line { .. }) => {
                    let (a, b) = (sketch.line(first.id)?, sketch.line(second.id)?);
                    if same_line(a, b) {
                        Some(None)
                    } else {
                        // Linha inteira sobre outra colinear: a contida é removida.
                        let inside = |inner: (Point2, Point2), outer: (Point2, Point2)| {
                            [inner.0, inner.1].iter().all(|p| {
                                along(*p, outer.0, outer.1)
                                    .is_some_and(|t| (-1e-9..=1. + 1e-9).contains(&t))
                            })
                        };
                        if inside(b, a) {
                            Some(Some((second.id, first.id)))
                        } else if inside(a, b) {
                            Some(Some((first.id, second.id)))
                        } else {
                            None
                        }
                    }
                }
                (Curve::Circle { .. }, Curve::Circle { .. }) => {
                    let (c1, r1) = sketch.circle(first.id)?;
                    let (c2, r2) = sketch.circle(second.id)?;
                    (distance(c1, c2) < SAME && (r1 - r2).abs() < SAME).then_some(None)
                }
                (Curve::Arc { .. }, Curve::Arc { .. }) => {
                    let ends = |e: &Entity| -> Result<(Point2, Point2, Point2, bool)> {
                        match e.curve {
                            Curve::Arc {
                                center,
                                start,
                                end,
                                clockwise,
                            } => Ok((
                                sketch.point(center)?,
                                sketch.point(start)?,
                                sketch.point(end)?,
                                clockwise,
                            )),
                            _ => Err(SketchError("Arco esperado.".into())),
                        }
                    };
                    let (c1, s1, e1, w1) = ends(first)?;
                    let (c2, s2, e2, w2) = ends(second)?;
                    let same = distance(c1, c2) < SAME
                        && ((w1 == w2 && distance(s1, s2) < SAME && distance(e1, e2) < SAME)
                            || (w1 != w2 && distance(s1, e2) < SAME && distance(e1, s2) < SAME));
                    same.then_some(None)
                }
                _ => None,
            };
            match duplicate {
                Some(None) => {
                    removed.insert(second.id);
                    report.fixes.push(Fix::RemoveDuplicate {
                        entity: second.id,
                        keeps: first.id,
                    });
                }
                Some(Some((entity, within))) => {
                    removed.insert(entity);
                    report.fixes.push(Fix::RemoveContained { entity, within });
                }
                None => {}
            }
        }
    }
    // Pontas abertas: extremos de linhas e arcos referenciados uma única vez (por posição),
    // sem contar as entidades que já serão removidas.
    let mut ends: Vec<(u64, Point2)> = Vec::new();
    for entity in profile.iter().filter(|e| !removed.contains(&e.id)) {
        let ids = match entity.curve {
            Curve::Line { start, end } | Curve::Arc { start, end, .. } => [start, end],
            Curve::Circle { .. } => continue,
        };
        for id in ids {
            ends.push((id, sketch.point(id)?));
        }
    }
    let degree = |p: Point2| ends.iter().filter(|(_, q)| distance(*q, p) < SAME).count();
    let open = ends
        .iter()
        .filter(|(_, p)| degree(*p) == 1)
        .copied()
        .collect::<Vec<_>>();
    // Pares mais próximos primeiro; cada ponta participa de um único reparo.
    let mut pairs = Vec::new();
    for (i, (a, p)) in open.iter().enumerate() {
        for (b, q) in &open[i + 1..] {
            let gap = distance(*p, *q);
            if gap >= SAME && gap <= tolerance {
                pairs.push((gap, *a, *b));
            }
        }
    }
    pairs.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut used = HashSet::new();
    for (gap, a, b) in pairs {
        if used.insert(a) && used.insert(b) {
            report.fixes.push(Fix::CloseGap { a, b, gap });
        }
    }
    let (_, diagnostics) = profiles::analyze(sketch)?;
    report.crossings = diagnostics.crossings;
    Ok(report)
}

/// Remove uma entidade, as restrições que dependiam dela e os pontos que só ela usava.
/// Pontos isolados que já existiam (centros de furos) são preservados.
fn remove_entity(sketch: &mut Sketch, id: u64) -> Result<()> {
    let entity = sketch.entity(id)?.clone();
    sketch.entities.retain(|e| e.id != id);
    let still_used = sketch
        .entities
        .iter()
        .flat_map(Entity::point_ids)
        .collect::<HashSet<_>>();
    let orphans = entity
        .point_ids()
        .into_iter()
        .filter(|p| !still_used.contains(p))
        .collect::<HashSet<_>>();
    sketch.points.retain(|p| !orphans.contains(&p.id));
    let candidate = sketch.clone();
    sketch
        .constraints
        .retain(|c| solver::constraint_residuals(&candidate, &c.kind).is_ok());
    let constraints = sketch
        .constraints
        .iter()
        .map(|c| c.id)
        .collect::<HashSet<_>>();
    sketch
        .dimension_expressions
        .retain(|id, _| constraints.contains(id));
    sketch
        .dimension_labels
        .retain(|id, _| constraints.contains(id));
    Ok(())
}

/// Aplica os reparos escolhidos a uma cópia. A coincidência acrescentada ainda precisa ser
/// resolvida pelo solver; o resultado só é publicado se a solução for consistente.
pub fn apply(sketch: &Sketch, fixes: &[Fix]) -> Result<Sketch> {
    let mut next = sketch.clone();
    let mut done = HashMap::new();
    for fix in fixes {
        match *fix {
            Fix::RemoveDuplicate { entity, .. } | Fix::RemoveContained { entity, .. } => {
                if done.insert(entity, ()).is_none() {
                    remove_entity(&mut next, entity)?;
                }
            }
            Fix::CloseGap { a, b, .. } => {
                next.point(a)?;
                next.point(b)?;
                next.add_constraint(ConstraintKind::Coincident { a, b }, false)?;
            }
        }
    }
    next.validate()?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn duplicates_contained_lines_and_small_gaps_are_suggested_and_repaired() -> TestResult {
        let mut sketch = Sketch::new("Reparo", Plane::xy());
        // Retângulo com o último lado terminando 0,2 mm antes do início.
        let a = sketch.add_line([0., 0.], [40., 0.], false)?;
        sketch.add_line([40., 0.], [40., 20.], false)?;
        sketch.add_line([40., 20.], [0., 20.], false)?;
        sketch.add_line([0., 20.], [0., 0.2], false)?;
        // Cópia exata de um lado e um trecho colinear dentro de outro.
        let duplicate = sketch.add_line([40., 0.], [0., 0.], false)?;
        let contained = sketch.add_line([40., 5.], [40., 15.], false)?;
        // Centro de furo isolado: nunca removido pelo reparo.
        let hole = sketch.add_point([20., 10.])?;
        assert!(profiles::regions(&sketch).is_err());
        let report = suggest(&sketch, 0.5)?;
        assert!(report.fixes.contains(&Fix::RemoveDuplicate {
            entity: duplicate,
            keeps: a,
        }));
        assert!(
            report
                .fixes
                .iter()
                .any(|f| matches!(f, Fix::RemoveContained { entity, .. } if *entity == contained))
        );
        let gaps = report
            .fixes
            .iter()
            .filter(|f| matches!(f, Fix::CloseGap { .. }))
            .collect::<Vec<_>>();
        assert_eq!(gaps.len(), 1);
        assert!(matches!(gaps[0], Fix::CloseGap { gap, .. } if (gap - 0.2).abs() < 1e-9));
        // Fora da tolerância, a folga não é sugerida.
        assert!(
            !suggest(&sketch, 0.1)?
                .fixes
                .iter()
                .any(|f| matches!(f, Fix::CloseGap { .. }))
        );
        let repaired = apply(&sketch, &report.fixes)?;
        let solved = solver::solve(&repaired)?.sketch;
        assert_eq!(profiles::regions(&solved)?.len(), 1);
        assert!(solved.point(hole).is_ok());
        assert!(solved.entity(duplicate).is_err() && solved.entity(contained).is_err());
        // O rascunho original não muda.
        assert_eq!(sketch.entities.len(), 6);
        Ok(())
    }

    #[test]
    fn crossings_are_reported_without_automatic_repair() -> TestResult {
        let mut sketch = Sketch::new("Cruzamento", Plane::xy());
        sketch.add_line([0., 0.], [10., 10.], false)?;
        sketch.add_line([0., 10.], [10., 0.], false)?;
        let report = suggest(&sketch, 0.5)?;
        assert_eq!(report.crossings.len(), 1);
        assert!(distance(report.crossings[0], [5., 5.]) < 1e-9);
        assert!(report.fixes.is_empty());
        Ok(())
    }
}
