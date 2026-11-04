//! Seção circular constante por uma guia exata; perfis auxiliares não entram no documento.
use nator_document::DocumentError;
use nator_kernel::Solid;
use nator_sketch::{Plane, Sketch, profiles};

fn error(message: impl ToString) -> DocumentError {
    DocumentError(message.to_string())
}

pub(super) fn build(
    sketch: &Sketch,
    entities: &[u64],
    diameter: f64,
    thickness: Option<f64>,
    owner: &str,
) -> Result<Solid, DocumentError> {
    if !diameter.is_finite()
        || diameter <= 0.
        || thickness.is_some_and(|t| !t.is_finite() || t <= 0. || 2. * t >= diameter)
    {
        return Err(error("Diâmetro ou espessura do tubo inválidos."));
    }
    for id in entities {
        let radius = match sketch.entity(*id).map_err(error)?.curve {
            nator_sketch::Curve::Arc { center, start, .. } => Some(nator_sketch::distance(
                sketch.point(center).map_err(error)?,
                sketch.point(start).map_err(error)?,
            )),
            nator_sketch::Curve::Circle { radius, .. } => Some(radius),
            nator_sketch::Curve::Line { .. } => None,
        };
        if radius.is_some_and(|radius| diameter / 2. >= radius) {
            return Err(error(
                "O raio da curva do tubo deve ser maior que metade do diâmetro externo.",
            ));
        }
    }
    let path = crate::sweep::path(sketch, entities, None)?;
    if !path.smooth {
        return Err(error(
            "A trajetória do tubo tem uma quina. Ligue os trechos com um arco tangente.",
        ));
    }
    let section = |plane: Plane| -> Result<_, DocumentError> {
        let mut s = Sketch::new("Seção do tubo", plane);
        s.add_circle([0.; 2], diameter / 2., false).map_err(error)?;
        if let Some(t) = thickness {
            s.add_circle([0.; 2], diameter / 2. - t, false)
                .map_err(error)?;
        }
        let region = profiles::regions(&s)
            .map_err(error)?
            .into_iter()
            .find(|r| r.holes.len() == usize::from(thickness.is_some()))
            .ok_or_else(|| error("A seção do tubo não formou uma área válida."))?;
        Ok((s, region))
    };
    let a = section(path.start)?;
    if path.closed {
        return Solid::closed_pipe((&a.0, &a.1), &path.curves, owner).map_err(error);
    }
    let b = section(path.end)?;
    Solid::guided_transition((&a.0, &a.1), (&b.0, &b.1), &path.curves, owner).map_err(error)
}
