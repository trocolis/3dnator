//! Fusão de vértices pela precisão de desenho, pensada para a impressão 3D: pontas que
//! ficam mais perto que a precisão são o mesmo vértice para quem desenha, e uma fresta
//! de micrômetros entre elas abriria o contorno ou deixaria uma lasca no sólido.
//!
//! - Ao desenhar (`fuse_new`): cada ponta nova recebe a precisão vigente e, se estiver a
//!   menos dela de um ponto mais antigo, vai para a posição dele e ganha uma ligação de
//!   coincidência. Perto de uma linha da face ou projeção, é levada para a linha. Um
//!   traço novo menor que a precisão é recusado.
//! - Ao construir (`fused`): uma cópia do rascunho em que cada ponta a menos da sua
//!   própria precisão de outra mais antiga (ou fixa) passa para a posição dela. O
//!   rascunho guardado não muda, e as áreas da interface usam a mesma cópia.
//!
//! O ponto mais antigo mantém a posição, e cada ponto conserva a precisão com que foi
//! criado, mesmo que a configuração mude depois. Nenhuma fusão colapsa um traço.
use crate::{profiles::Segment, *};
use std::borrow::Cow;

/// Precisão atribuída a pontos de projetos anteriores ao registro por ponto (mm).
pub const LEGACY_PRECISION: f64 = 1e-3;
/// Abaixo disto, os pontos já estão no mesmo lugar.
const SAME: f64 = 1e-9;

/// Pontos fixos (projeções e linhas da face) nunca se movem e têm prioridade como destino.
fn locked_point(point: &Point) -> bool {
    point.projected || point.face
}

/// Prioridade de destino: fixos primeiro, depois os mais antigos.
fn rank(point: &Point) -> (bool, u64) {
    (!locked_point(point), point.id)
}

/// Outras pontas das entidades que usam `point`: mover `point` sobre uma delas
/// colapsaria o traço.
fn partners(sketch: &Sketch, point: u64) -> Vec<u64> {
    sketch
        .entities
        .iter()
        .filter(|e| e.point_ids().contains(&point))
        .flat_map(Entity::point_ids)
        .filter(|id| *id != point)
        .collect()
}

fn collapses(sketch: &Sketch, point: u64, target: Point2) -> bool {
    partners(sketch, point)
        .iter()
        .any(|id| sketch.point(*id).is_ok_and(|p| distance(p, target) < SAME))
}

/// Pontas de traços do perfil: extremos de linhas e arcos que não são de construção.
fn profile_ends(sketch: &Sketch) -> Vec<u64> {
    let mut ids = sketch
        .entities
        .iter()
        .filter(|e| !e.construction)
        .flat_map(|e| match e.curve {
            Curve::Line { start, end } | Curve::Arc { start, end, .. } => vec![start, end],
            Curve::Circle { .. } => Vec::new(),
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Cópia do rascunho com as pontas próximas fundidas, para áreas e sólidos. Sem nada a
/// fundir, devolve o próprio rascunho.
pub fn fused(sketch: &Sketch) -> Cow<'_, Sketch> {
    let ends = profile_ends(sketch);
    let mut result = Cow::Borrowed(sketch);
    let mut order = ends
        .iter()
        .filter_map(|id| sketch.points.iter().find(|p| p.id == *id))
        .collect::<Vec<_>>();
    order.sort_by_key(|p| rank(p));
    let order = order.into_iter().cloned().collect::<Vec<_>>();
    for (index, moving) in order.iter().enumerate() {
        if locked_point(moving) {
            continue;
        }
        let current = |s: &Sketch, id: u64| s.point(id).ok();
        let Some(here) = current(&result, moving.id) else {
            continue;
        };
        // Só destinos de prioridade maior: o mais antigo (ou fixo) fica onde está.
        let target = order[..index].iter().find_map(|candidate| {
            let there = current(&result, candidate.id)?;
            let gap = distance(here, there);
            (gap >= SAME && gap < moving.precision() && !collapses(&result, moving.id, there))
                .then_some(there)
        });
        if let Some(there) = target
            && let Some(point) = result
                .to_mut()
                .points
                .iter_mut()
                .find(|p| p.id == moving.id)
        {
            point.position = there;
        }
    }
    result
}

/// Ponta aberta: usada por uma só entidade e sem ligação de coincidência. Vértices de
/// cadeias, cópias e deslocamentos já estão ligados e não são fundidos.
fn open_end(sketch: &Sketch, point: u64) -> bool {
    let owners = sketch
        .entities
        .iter()
        .filter(|e| e.point_ids().contains(&point))
        .count();
    owners == 1 && !linked(sketch, point, None)
}

/// Há coincidência dirigente envolvendo `point` (com `other`, se dado).
fn linked(sketch: &Sketch, point: u64, other: Option<u64>) -> bool {
    sketch.constraints.iter().any(|c| {
        !c.reference
            && matches!(c.kind, ConstraintKind::Coincident { a, b }
                if (a == point && other.is_none_or(|o| b == o))
                    || (b == point && other.is_none_or(|o| a == o)))
    })
}

/// Tamanho característico de um traço novo, comparado com a precisão.
fn extent(sketch: &Sketch, entity: &Entity) -> Result<Option<f64>> {
    Ok(match entity.curve {
        Curve::Line { .. } if entity.infinite => None,
        Curve::Line { start, end } => Some(distance(sketch.point(start)?, sketch.point(end)?)),
        Curve::Circle { radius, .. } => Some(2. * radius),
        Curve::Arc { center, start, .. } => {
            Some(2. * distance(sketch.point(center)?, sketch.point(start)?))
        }
    })
}

/// Fusão ao desenhar. `previous` é o rascunho antes da edição; pontos cujo ID não existia
/// nele são novos e recebem `precision`. Devolve erro se um traço novo ficar menor que ela.
pub fn fuse_new(previous: &Sketch, sketch: &mut Sketch, precision: f64) -> Result<()> {
    if !(precision.is_finite() && precision > 0.) {
        return Ok(());
    }
    let old = previous.points.iter().map(|p| p.id).collect::<HashSet<_>>();
    let new = sketch
        .points
        .iter()
        .filter(|p| !old.contains(&p.id) && !locked_point(p))
        .map(|p| p.id)
        .collect::<Vec<_>>();
    if new.is_empty() {
        return Ok(());
    }
    for point in &mut sketch.points {
        if new.contains(&point.id) {
            point.precision = Some(precision);
        }
    }
    let known = previous
        .entities
        .iter()
        .map(|e| e.id)
        .collect::<HashSet<_>>();
    for entity in sketch.entities.iter().filter(|e| !known.contains(&e.id)) {
        if !entity.projected
            && let Some(size) = extent(sketch, entity)?
            && size < precision
        {
            return Err(SketchError(format!(
                "O traço tem {size:.3} mm, menos que a precisão de desenho ({precision} mm)."
            )));
        }
    }
    for id in new {
        if !open_end(sketch, id) {
            continue;
        }
        let here = sketch.point(id)?;
        let target = sketch
            .points
            .iter()
            .filter(|q| q.id != id && (q.id < id || locked_point(q)))
            .filter(|q| distance(q.position, here) < precision)
            .filter(|q| !partners(sketch, id).contains(&q.id) && !linked(sketch, id, Some(q.id)))
            .filter(|q| !collapses(sketch, id, q.position))
            .min_by_key(|q| rank(q))
            .map(|q| (q.id, q.position));
        if let Some((other, position)) = target {
            set(sketch, id, position);
            sketch.add_constraint(ConstraintKind::Coincident { a: id, b: other }, false)?;
            continue;
        }
        // Sem vértice por perto, uma linha fixa próxima recebe a ponta.
        let mut nearest: Option<(f64, Point2)> = None;
        for entity in sketch.entities.iter().filter(|e| e.projected || e.face) {
            if entity.point_ids().contains(&id) {
                continue;
            }
            let curve = profiles::segment(sketch, entity)?;
            let foot = closest(&curve, here);
            let gap = distance(foot, here);
            if gap >= SAME
                && gap < precision
                && nearest.is_none_or(|(d, _)| gap < d)
                && !collapses(sketch, id, foot)
            {
                nearest = Some((gap, foot));
            }
        }
        if let Some((_, foot)) = nearest {
            set(sketch, id, foot);
        }
    }
    Ok(())
}

fn closest(curve: &Segment, p: Point2) -> Point2 {
    crate::curve_geometry::point(curve, crate::curve_geometry::closest_parameter(curve, p))
}

fn set(sketch: &mut Sketch, id: u64, position: Point2) {
    if let Some(point) = sketch.points.iter_mut().find(|p| p.id == id) {
        point.position = position;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    /// Retângulo cujo último lado termina `gap` antes do início.
    fn almost_closed(gap: f64) -> Result<(Sketch, Sketch)> {
        let mut previous = Sketch::new("Fresta", Plane::xy());
        previous.add_line([0., 0.], [40., 0.], false)?;
        previous.add_line([40., 0.], [40., 20.], false)?;
        previous.add_line([40., 20.], [0., 20.], false)?;
        let mut next = previous.clone();
        next.add_line([0., 20.], [0., gap], false)?;
        Ok((previous, next))
    }

    #[test]
    fn a_new_end_within_the_precision_joins_the_older_vertex() -> TestResult {
        let (previous, mut sketch) = almost_closed(0.3)?;
        assert!(profiles::regions(&sketch).is_err());
        fuse_new(&previous, &mut sketch, 0.4)?;
        let solved = solver::solve(&sketch)?.sketch;
        assert_eq!(profiles::regions(&solved)?.len(), 1);
        // O vértice mais antigo não se moveu; a ponta nova guarda a precisão.
        assert_eq!(solved.point(1)?, [0., 0.]);
        let newest = solved.points.iter().max_by_key(|p| p.id).ok_or("ponto")?;
        assert_eq!(newest.precision, Some(0.4));
        assert!(
            solved
                .constraints
                .iter()
                .any(|c| matches!(c.kind, ConstraintKind::Coincident { b: 1, .. }))
        );
        // Fora da precisão, nada muda.
        let (previous, mut wide) = almost_closed(0.6)?;
        fuse_new(&previous, &mut wide, 0.4)?;
        assert!(profiles::regions(&wide).is_err());
        Ok(())
    }

    #[test]
    fn strokes_shorter_than_the_precision_are_refused() -> TestResult {
        let previous = Sketch::new("Curto", Plane::xy());
        let mut sketch = previous.clone();
        sketch.add_line([0., 0.], [0.3, 0.], false)?;
        assert!(fuse_new(&previous, &mut sketch, 0.4).is_err());
        let mut fine = previous.clone();
        fine.add_line([0., 0.], [0.5, 0.], false)?;
        fuse_new(&previous, &mut fine, 0.4)?;
        Ok(())
    }

    #[test]
    fn a_new_end_near_a_face_line_lands_on_it() -> TestResult {
        let mut previous = Sketch::new("Face", Plane::xy());
        projection::insert_face(
            &mut previous,
            &[Segment::Line {
                a: [0., 0.],
                b: [40., 0.],
            }],
        )?;
        let mut sketch = previous.clone();
        let line = sketch.add_line([10., 10.], [10., 0.2], false)?;
        fuse_new(&previous, &mut sketch, 0.4)?;
        let (_, end) = sketch.line(line)?;
        assert!(distance(end, [10., 0.]) < 1e-12, "{end:?}");
        Ok(())
    }

    #[test]
    fn building_closes_gaps_below_each_points_own_precision() -> TestResult {
        let (_, mut sketch) = almost_closed(0.0005)?;
        // Pontos antigos (sem precisão) fundem até 0,001 mm.
        assert_eq!(profiles::regions(&sketch)?.len(), 1);
        assert_eq!(areas::regions(&sketch)?.len(), 1);
        // A precisão gravada vale mesmo maior; o rascunho guardado não muda.
        let (_, wide) = almost_closed(0.3)?;
        sketch = wide;
        assert!(profiles::regions(&sketch).is_err());
        for point in &mut sketch.points {
            point.precision = Some(0.4);
        }
        assert_eq!(profiles::regions(&sketch)?.len(), 1);
        let last = sketch.points.iter().max_by_key(|p| p.id).ok_or("ponto")?;
        assert!((last.position[1] - 0.3).abs() < 1e-12);
        // Um traço inteiro menor que a precisão não colapsa.
        let mut tiny = Sketch::new("Curto", Plane::xy());
        let short = tiny.add_line([0., 0.], [0.2, 0.], false)?;
        for point in &mut tiny.points {
            point.precision = Some(0.4);
        }
        assert_eq!(fused(&tiny).line(short)?, tiny.line(short)?);
        Ok(())
    }
}
