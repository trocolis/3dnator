//! Sugestões de reparo de rascunho. Nada é corrigido implicitamente: as sugestões são
//! calculadas, apresentadas com prévia e aplicadas só por escolha explícita do usuário.
//!
//! - Entidade duplicada: mesma curva que outra (extremos ou centro e raio coincidentes).
//! - Entidade contida: linha inteira sobre outra linha colinear, que já a cobre.
//! - Folga pequena: duas pontas abertas mais próximas que a tolerância; o reparo acrescenta
//!   uma restrição de coincidência, e o solver fecha a folga respeitando as demais.
//! - Pontas sem ligação: dois extremos no mesmo lugar, sem relação entre eles. O contorno
//!   fecha agora, mas o solver pode afastá-los ou deixar uma folga residual.
//! - Quase alinhada: linha a menos de `ALIGNED_DEGREES` de horizontal ou vertical, sem
//!   restrição de direção; costuma ser um vértice torto, e vira fresta depois de impresso.
//!
//! Cruzamentos não têm reparo automático (exigem decidir onde aparar) e são só relatados.
use crate::*;
use std::collections::{HashMap, HashSet};

/// Coincidência geométrica usada para reconhecer extremos iguais.
const SAME: f64 = 1e-6;
/// Extremos a esta distância estão no mesmo lugar para o usuário, mesmo com folga real.
const NEAR: f64 = 1e-4;
/// Abaixo disto, a inclinação de uma linha dificilmente é intencional.
const ALIGNED_DEGREES: f64 = 1.;

#[derive(Debug, Clone, PartialEq)]
pub enum Fix {
    /// Remove `entity`, igual a `keeps`.
    RemoveDuplicate { entity: u64, keeps: u64 },
    /// Remove a linha `entity`, coberta pela linha `within`.
    RemoveContained { entity: u64, within: u64 },
    /// Une as pontas abertas `a` e `b`, afastadas de `gap` mm.
    CloseGap { a: u64, b: u64, gap: f64 },
    /// Liga os extremos `a` e `b`, afastados de `gap` mm e sem relação entre si.
    LinkEnds { a: u64, b: u64, gap: f64 },
    /// Alinha `line`, inclinada `degrees` em relação ao eixo.
    AlmostAligned {
        line: u64,
        horizontal: bool,
        degrees: f64,
    },
}
impl Fix {
    /// Entidades e pontos envolvidos, para realce na prévia.
    pub fn entities(&self) -> Vec<u64> {
        match *self {
            Self::RemoveDuplicate { entity, keeps } => vec![entity, keeps],
            Self::RemoveContained { entity, within } => vec![entity, within],
            Self::CloseGap { .. } | Self::LinkEnds { .. } => Vec::new(),
            Self::AlmostAligned { line, .. } => vec![line],
        }
    }
    pub fn points(&self) -> Vec<u64> {
        match *self {
            Self::CloseGap { a, b, .. } | Self::LinkEnds { a, b, .. } => vec![a, b],
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
    // Pontas de referências fixas (texto, projeção, linhas da face) não se movem nem se
    // ligam entre si: entre duas delas não há o que reparar.
    let fixed = |id: u64| {
        sketch
            .points
            .iter()
            .any(|p| p.id == id && (p.projected || p.face))
    };
    // Pares mais próximos primeiro; cada ponta participa de um único reparo.
    let mut pairs = Vec::new();
    for (i, (a, p)) in open.iter().enumerate() {
        for (b, q) in &open[i + 1..] {
            let gap = distance(*p, *q);
            if gap >= SAME && gap <= tolerance && !(fixed(*a) && fixed(*b)) {
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
    // Só a ligação dirigente mantém as pontas juntas. A fraca de projetos antigos não prende.
    let linked = |a: u64, b: u64| {
        sketch.constraints.iter().any(|c| {
            !c.reference
                && matches!(c.kind, ConstraintKind::Coincident { a: x, b: y }
                if (x == a && y == b) || (x == b && y == a))
        })
    };
    for (i, (a, p)) in ends.iter().enumerate() {
        for (b, q) in &ends[i + 1..] {
            let gap = distance(*p, *q);
            if a != b
                && gap < NEAR
                && !(fixed(*a) && fixed(*b))
                && !used.contains(a)
                && !used.contains(b)
                && !linked(*a, *b)
                && !sketch.entities.iter().any(|e| {
                    let ids = e.point_ids();
                    ids.contains(a) && ids.contains(b)
                })
            {
                used.insert(*a);
                used.insert(*b);
                report.fixes.push(Fix::LinkEnds { a: *a, b: *b, gap });
            }
        }
    }
    // Direção já definida por restrição é intenção explícita, por menor que seja o ângulo.
    let directed = |line: u64| {
        sketch.constraints.iter().any(|c| match c.kind {
            ConstraintKind::Horizontal { line: other }
            | ConstraintKind::Vertical { line: other } => other == line,
            ConstraintKind::LineAngle {
                line: x,
                reference: y,
                ..
            }
            | ConstraintKind::Angle { a: x, b: y, .. }
            | ConstraintKind::Parallel { a: x, b: y }
            | ConstraintKind::Collinear { a: x, b: y }
            | ConstraintKind::Perpendicular { a: x, b: y } => x == line || y == line,
            _ => false,
        })
    };
    for entity in profile.iter().filter(|e| !removed.contains(&e.id)) {
        if !matches!(entity.curve, Curve::Line { .. }) || directed(entity.id) {
            continue;
        }
        let (a, b) = sketch.line(entity.id)?;
        let [dx, dy] = sub(b, a).map(f64::abs);
        let degrees = dx.min(dy).atan2(dx.max(dy)).to_degrees();
        // Abaixo disto a inclinação é ruído numérico, sem efeito na peça.
        if degrees > 1e-6 && degrees < ALIGNED_DEGREES {
            report.fixes.push(Fix::AlmostAligned {
                line: entity.id,
                horizontal: dx > dy,
                degrees,
            });
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
    sketch.dimension_targets.retain(|id, pair| {
        constraints.contains(id) && pair.iter().all(|t| t.points(&candidate).is_ok())
    });
    Ok(())
}

/// `point` é extremo de uma das linhas quase alinhadas de `fixes`, fora `except`.
fn touches_crooked(
    sketch: &Sketch,
    fixes: &[Fix],
    point: u64,
    except: Option<u64>,
) -> Result<bool> {
    let position = sketch.point(point)?;
    for fix in fixes {
        if let Fix::AlmostAligned { line, .. } = fix
            && Some(*line) != except
        {
            let (a, b) = sketch.line(*line)?;
            if distance(a, position) < SAME || distance(b, position) < SAME {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Reúne duas pontas antes da coincidência. Sozinho, o solver dividiria a folga entre elas,
/// e meio mícron basta para desalinhar o rascunho de outro que dependa da posição antiga.
/// Fica parada a ponta que definirá a direção de uma linha corrigida.
fn join(next: &mut Sketch, original: &Sketch, fixes: &[Fix], a: u64, b: u64) -> Result<()> {
    let keep_b =
        touches_crooked(original, fixes, b, None)? && !touches_crooked(original, fixes, a, None)?;
    let (kept, moved) = if keep_b { (b, a) } else { (a, b) };
    let (kept, moved) = (next.point(kept)?, next.point(moved)?);
    for point in &mut next.points {
        if distance(point.position, moved) < SAME {
            point.position = kept;
        }
    }
    Ok(())
}

/// Endireita a linha movendo só a ponta torta. Sozinho, o solver dividiria o erro entre as
/// duas pontas e tiraria do lugar a que estava certa. A ponta torta é a que encosta em outra
/// linha quase alinhada; sem essa pista, fica a de origem da linha, que costuma estar ancorada.
fn align(
    next: &mut Sketch,
    original: &Sketch,
    fixes: &[Fix],
    line: u64,
    horizontal: bool,
) -> Result<()> {
    let Curve::Line { start, end } = next.entity(line)?.curve else {
        return Err(SketchError("Linha esperada.".into()));
    };
    let crooked = |point| touches_crooked(original, fixes, point, Some(line));
    let (kept, moved) = if crooked(start)? && !crooked(end)? {
        (end, start)
    } else {
        (start, end)
    };
    let (kept, moved) = (next.point(kept)?, next.point(moved)?);
    // Pontos no mesmo lugar acompanham a ponta: são o mesmo vértice para o usuário.
    let axis = usize::from(horizontal);
    for point in &mut next.points {
        if distance(point.position, moved) < NEAR {
            point.position[axis] = kept[axis];
        }
    }
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
            Fix::CloseGap { a, b, .. } | Fix::LinkEnds { a, b, .. } => {
                join(&mut next, sketch, fixes, a, b)?;
                next.add_constraint(ConstraintKind::Coincident { a, b }, false)?;
            }
            Fix::AlmostAligned {
                line, horizontal, ..
            } => {
                align(&mut next, sketch, fixes, line, horizontal)?;
                next.add_constraint(
                    if horizontal {
                        ConstraintKind::Horizontal { line }
                    } else {
                        ConstraintKind::Vertical { line }
                    },
                    false,
                )?;
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
    fn crooked_lines_are_aligned_unless_their_direction_is_constrained() -> TestResult {
        let mut sketch = Sketch::new("Torto", Plane::xy());
        // Mesma inclinação do defeito real: 0,0094 mm em 6,95 mm.
        let crooked = sketch.add_line([0., 0.], [6.95, 0.0094], false)?;
        let straight = sketch.add_line([6.95, 0.0094], [6.95, 10.], false)?;
        let inclined = sketch.add_line([6.95, 10.], [0., 12.], false)?;
        sketch.add_constraint(ConstraintKind::Vertical { line: straight }, false)?;
        // As pontas soltas também geram `LinkEnds`: aqui só interessa a direção.
        let aligned = |s: &Sketch| -> Result<Vec<Fix>> {
            Ok(suggest(s, 0.)?
                .fixes
                .into_iter()
                .filter(|f| matches!(f, Fix::AlmostAligned { .. }))
                .collect())
        };
        let fixes = aligned(&sketch)?;
        let [
            Fix::AlmostAligned {
                line,
                horizontal: true,
                degrees,
            },
        ] = fixes[..]
        else {
            return Err(format!("{fixes:?}").into());
        };
        assert_eq!(line, crooked);
        assert!(degrees > 0.05 && degrees < 0.1);
        assert_ne!(line, inclined);
        let solved = solver::solve(&apply(&sketch, &fixes)?)?.sketch;
        let (a, b) = solved.line(crooked)?;
        assert!((a[1] - b[1]).abs() < 1e-6);
        // A origem ancorada não se move: o erro não é dividido entre as pontas.
        assert!(a[1].abs() < 1e-9);
        // A intenção explícita silencia o aviso.
        let mut intended = sketch.clone();
        intended.add_constraint(
            ConstraintKind::Perpendicular {
                a: crooked,
                b: straight,
            },
            false,
        )?;
        assert!(aligned(&intended)?.is_empty());
        Ok(())
    }

    /// Duas linhas tortas no mesmo vértice, como no defeito real: as outras pontas estão
    /// certas, então o canto corrigido é o cruzamento das direções que elas definem.
    #[test]
    fn crooked_corner_moves_alone_to_where_its_good_ends_point() -> TestResult {
        let mut sketch = Sketch::new("Canto", Plane::xy());
        let vertical = sketch.add_line([61.32017, -27.98186], [61.31773, -26.19134], false)?;
        let horizontal = sketch.add_line([68.27, -26.18187], [61.31773, -26.19134], false)?;
        let fixes = suggest(&sketch, 0.)?.fixes;
        let solved = solver::solve(&apply(&sketch, &fixes)?)?.sketch;
        let (bottom, top) = solved.line(vertical)?;
        let (right, left) = solved.line(horizontal)?;
        assert!(distance(top, [61.32017, -26.18187]) < 1e-6, "{top:?}");
        assert!(distance(left, top) < 1e-6);
        assert!(distance(bottom, [61.32017, -27.98186]) < 1e-6);
        assert!(distance(right, [68.27, -26.18187]) < 1e-6);
        Ok(())
    }

    /// Caso real: juntar as pontas não pode tirar do lugar a que uma linha torta usa como
    /// referência, ou o rascunho deixa de alinhar com outro por poucos mícrons.
    #[test]
    fn closing_a_tiny_gap_keeps_the_end_that_defines_a_corrected_line() -> TestResult {
        let mut sketch = Sketch::new("Folga", Plane::xy());
        sketch.add_line([68.27, -11.68], [68.27, -26.181879], false)?;
        sketch.add_line([68.27, -26.181871], [61.31773, -26.19134], false)?;
        let fixes = suggest(&sketch, 0.5)?.fixes;
        assert!(fixes.iter().any(|f| matches!(f, Fix::CloseGap { .. })));
        let solved = solver::solve(&apply(&sketch, &fixes)?)?.sketch;
        for point in &solved.points {
            if point.position[1] < -20. {
                assert!((point.position[1] + 26.181871).abs() < 1e-8, "{point:?}");
            }
        }
        Ok(())
    }

    #[test]
    fn coincident_ends_without_a_link_are_reported_once() -> TestResult {
        let mut sketch = Sketch::new("Sem ligação", Plane::xy());
        // Folga residual do solver: fecha a área, mas nada mantém as pontas juntas.
        sketch.add_line([0., 0.], [10., 0.], false)?;
        sketch.add_line([10. + 8e-8, 0.], [10., 10.], false)?;
        sketch.add_line([10., 10.], [0., 0.], false)?;
        assert_eq!(profiles::regions(&sketch)?.len(), 1);
        let links = |s: &Sketch| -> Result<usize> {
            Ok(suggest(s, 0.)?
                .fixes
                .iter()
                .filter(|f| matches!(f, Fix::LinkEnds { .. }))
                .count())
        };
        assert_eq!(links(&sketch)?, 3);
        let report = suggest(&sketch, 0.)?;
        let solved = solver::solve(&apply(&sketch, &report.fixes)?)?.sketch;
        assert_eq!(links(&solved)?, 0);
        assert_eq!(profiles::regions(&solved)?.len(), 1);
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
