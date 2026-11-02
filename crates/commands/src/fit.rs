//! Encaixe de componentes: recorte pela silhueta na direção de inserção.
//!
//! A silhueta vem da malha exibida, projetada e unida em 2D, e a folga é aplicada nesse
//! contorno. O deslocamento exato de B-rep do núcleo falha em boa parte dos STEP reais.
use nator_document::{BodyId, Document, DocumentError, features::ComponentOutline};
use nator_geometry::{Placement, Point3, dot, silhouette};
use nator_sketch::Plane;

/// Margem além do corpo, para o corte não terminar sobre uma face existente.
#[cfg(feature = "occt")]
const EXIT_MARGIN_MM: f64 = 1.;
/// Remoção abaixo disto significa que a passagem não alcança o corpo.
#[cfg(feature = "occt")]
const EMPTY_CUT_MM3: f64 = 1e-6;
/// Resolução gravada no projeto, muito abaixo da tolerância do contorno.
const STORED_MM: f64 = 1e-6;

/// Silhueta do corpo `component`, posto em `pose`, vista na direção `insertion`, em
/// coordenadas de uma peça posta em `target`.
pub fn capture(
    doc: &Document,
    component: BodyId,
    pose: Placement,
    target: Placement,
    insertion: Point3,
) -> Result<ComponentOutline, DocumentError> {
    let record = doc
        .body(component)
        .ok_or_else(|| DocumentError("Componente inexistente.".into()))?;
    let geometry = doc
        .display_geometry(component)
        .ok_or_else(|| DocumentError("Componente inexistente.".into()))?;
    let pose = pose
        .compose(record.placement)
        .map_err(|e| DocumentError(e.to_string()))?;
    let (u, v) = silhouette::plane_basis(insertion.map(|value| -value))
        .ok_or_else(|| DocumentError("Direção de inserção inválida.".into()))?;
    let down = nator_geometry::cross(v, u);
    let mesh = geometry.mesh();
    let points = mesh
        .vertices()
        .iter()
        .map(|p| target.inverse_apply(pose.apply(*p)))
        .collect::<Vec<_>>();
    let deepest = points
        .iter()
        .map(|p| dot(*p, down))
        .fold(f64::NEG_INFINITY, f64::max);
    let plane = Plane {
        origin: down.map(|value| value * deepest),
        u,
        v,
    };
    let project = |index: u32| plane.project(points[index as usize]);
    let contours = silhouette::silhouette(mesh.triangles().iter().map(|t| t.map(project)))
        .into_iter()
        .map(|contour| contour.into_iter().map(rounded).collect())
        .collect::<Vec<Vec<_>>>();
    if contours.is_empty() {
        return Err(DocumentError(
            "O componente não tem área vista nessa direção.".into(),
        ));
    }
    Ok(ComponentOutline {
        source: record.name.clone(),
        plane,
        contours,
    })
}

fn rounded(point: [f64; 2]) -> [f64; 2] {
    point.map(|value| (value / STORED_MM).round() * STORED_MM)
}

/// Remove do corpo a passagem do componente, do fundo até a abertura.
#[cfg(feature = "occt")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn cut(
    doc: &Document,
    part: nator_document::PartId,
    base: BodyId,
    outline: &ComponentOutline,
    clearance: f64,
    floor: f64,
    depth: Option<f64>,
    owner: &str,
) -> Result<nator_kernel::Solid, DocumentError> {
    use nator_document::features::BooleanMode;
    let error = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let (solid, _) = crate::features::source(doc, part, base)?;
    let plane = outline.plane;
    let top = match depth {
        Some(depth) => depth,
        None => reach(doc, base, plane)? + EXIT_MARGIN_MM,
    };
    if top <= -floor {
        return Err(DocumentError(
            "A abertura do corpo fica abaixo do componente: confira a direção de inserção.".into(),
        ));
    }
    let contours = silhouette::grown(&outline.contours, clearance);
    let mut result = None;
    let mut entity = 1;
    for (index, contour) in contours.iter().enumerate() {
        let mut tool =
            nator_kernel::Solid::contour_prism(plane, contour, entity, -floor, top, owner)
                .map_err(error)?;
        entity += contour.len() as u64;
        // As tampas de cada contorno têm papéis iguais: o índice os mantém distintos.
        tool.map_face_names(|name| match name.rsplit_once(':') {
            Some((_, "start" | "end")) => format!("{name}{index}"),
            _ => name.to_owned(),
        });
        let current = result.as_ref().unwrap_or(&solid);
        result = Some(
            current
                .boolean(
                    &tool,
                    crate::features::boolean_mode(BooleanMode::Cut),
                    owner,
                )
                .map_err(error)?,
        );
    }
    let result = result.ok_or_else(|| DocumentError("Silhueta sem contorno.".into()))?;
    if solid.volume().map_err(error)? - result.volume().map_err(error)? < EMPTY_CUT_MM3 {
        return Err(DocumentError(
            "O recorte não alcança o corpo: confira o componente e a direção de inserção.".into(),
        ));
    }
    Ok(result)
}

/// Distância do plano até o ponto do corpo mais próximo da abertura.
#[cfg(feature = "occt")]
fn reach(doc: &Document, base: BodyId, plane: Plane) -> Result<f64, DocumentError> {
    let record = doc
        .body(base)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let normal = plane.normal();
    let origin = dot(plane.origin, normal);
    let geometry = doc
        .display_geometry(base)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    Ok(geometry
        .mesh()
        .vertices()
        .iter()
        .map(|p| dot(record.placement.apply(*p), normal) - origin)
        .fold(f64::NEG_INFINITY, f64::max))
}
