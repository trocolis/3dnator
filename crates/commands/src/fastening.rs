//! Fixação entre peças impressas: ressalto de tampa, torres de parafuso e presilhas.
use nator_document::{
    DocumentError,
    features::{BossSpec, LipKind, LipReference, LipSpec, SnapKind, SnapSpec},
    parameters::{Dimension, Parameters},
};
use nator_geometry::{add, sub};
use nator_kernel::{KernelError, SnapGeometry, Solid, hole};
use nator_sketch::{Plane, Sketch};

const JOIN: u32 = 0;
const CUT: u32 = 1;
/// Margem do rebaixo acima da face, para o corte não terminar sobre ela.
const GROOVE_MARGIN_MM: f64 = 0.5;
/// Afastamento acima disto deixa a peça solta, sem encostar no corpo.
const CONTACT_MM: f64 = 1e-4;
/// Remoção abaixo disto significa que a janela não alcança o corpo.
const EMPTY_CUT_MM3: f64 = 1e-6;

fn error(e: KernelError) -> DocumentError {
    DocumentError(e.to_string())
}

fn point_of(sketch: &Sketch, id: u64) -> Result<[f64; 3], DocumentError> {
    let point = sketch.point(id).map_err(|_| {
        DocumentError(
            "Um ponto da operação não existe mais no rascunho; selecione os pontos novamente."
                .into(),
        )
    })?;
    Ok(sketch.plane.world(point))
}

fn joined(solid: &Solid, tool: &Solid, owner: &str, part: &str) -> Result<Solid, DocumentError> {
    if solid.distance(tool).map_err(error)?.0 > CONTACT_MM {
        return Err(DocumentError(format!(
            "{part} não encosta no corpo: confira o rascunho e o sentido."
        )));
    }
    solid.boolean(tool, JOIN, owner).map_err(error)
}

pub(crate) fn lip(
    solid: &Solid,
    face: usize,
    spec: &LipSpec,
    parameters: &Parameters,
    owner: &str,
) -> Result<Solid, DocumentError> {
    let length = |e: &str| parameters.evaluate(e, Dimension::Length);
    let inner = spec.reference == LipReference::Inner;
    let (inset, width, height) = (
        length(&spec.inset)?,
        length(&spec.width)?,
        length(&spec.height)?,
    );
    match spec.kind {
        LipKind::Lip => {
            let tool = solid
                .rim_prism(face, inner, inset, inset + width, 0., height, owner)
                .map_err(error)?;
            solid.boolean(&tool, JOIN, owner).map_err(error)
        }
        LipKind::Groove => {
            let gap = length(&spec.clearance)?;
            let from = (inset - gap).max(0.);
            let to = inset + width + gap;
            let tool = solid
                .rim_prism(
                    face,
                    inner,
                    from,
                    to,
                    -(height + gap),
                    GROOVE_MARGIN_MM,
                    owner,
                )
                .map_err(error)?;
            solid.boolean(&tool, CUT, owner).map_err(error)
        }
    }
}

pub(crate) fn bosses(
    mut solid: Solid,
    sketch: &Sketch,
    points: &[u64],
    spec: &BossSpec,
    parameters: &Parameters,
    owner: &str,
) -> Result<Solid, DocumentError> {
    let length = |e: &str| parameters.evaluate(e, Dimension::Length);
    let height = length(&spec.height)?;
    let normal = sketch.plane.normal();
    let axis = normal.map(|v| if spec.reverse { -v } else { v });
    let bore = hole::HoleGeometry {
        diameter: length(&spec.hole_diameter)?,
        depth: spec
            .hole_depth
            .as_deref()
            .map(length)
            .transpose()?
            .unwrap_or(height),
        head: hole::HoleHead::Simple,
        tip_angle: None,
        teardrop: None,
    };
    for (index, point) in points.iter().enumerate() {
        let position = point_of(sketch, *point)?;
        let tool = Solid::boss_tool(
            position,
            axis,
            length(&spec.outer_diameter)?,
            height,
            length(&spec.fillet)?,
            owner,
            index,
        )
        .map_err(error)?;
        solid = joined(&solid, &tool, owner, "A torre")?;
        let top = add(position, axis.map(|v| v * height));
        let down = sub([0.; 3], axis);
        let tool = Solid::hole_tool(top, down, &bore, owner, index).map_err(error)?;
        solid = solid.boolean(&tool, CUT, owner).map_err(error)?;
    }
    Ok(solid)
}

pub(crate) fn snaps(
    mut solid: Solid,
    sketch: &Sketch,
    points: &[u64],
    spec: &SnapSpec,
    parameters: &Parameters,
    owner: &str,
) -> Result<Solid, DocumentError> {
    let length = |e: &str| parameters.evaluate(e, Dimension::Length);
    let size = SnapGeometry {
        length: length(&spec.length)?,
        thickness: length(&spec.thickness)?,
        width: length(&spec.width)?,
        overhang: length(&spec.overhang)?,
        head: length(&spec.head)?,
        fillet: length(&spec.fillet)?,
    };
    let (sin, cos) = parameters
        .evaluate(&spec.angle, Dimension::Angle)?
        .sin_cos();
    let Plane { u, v, .. } = sketch.plane;
    let outward = std::array::from_fn(|i| u[i] * cos + v[i] * sin);
    let up = sketch
        .plane
        .normal()
        .map(|v| if spec.reverse { -v } else { v });
    for (index, point) in points.iter().enumerate() {
        let plane = Plane {
            origin: point_of(sketch, *point)?,
            u: outward,
            v: up,
        };
        solid = match spec.kind {
            SnapKind::Hook => {
                let tool = Solid::snap_hook(plane, size, owner, index).map_err(error)?;
                joined(&solid, &tool, owner, "A presilha")?
            }
            SnapKind::Window => {
                let (gap, window) = (length(&spec.clearance)?, length(&spec.window)?);
                let tool =
                    Solid::snap_window(plane, size, gap, window, owner, index).map_err(error)?;
                let cut = solid.boolean(&tool, CUT, owner).map_err(error)?;
                if solid.volume().map_err(error)? - cut.volume().map_err(error)? < EMPTY_CUT_MM3 {
                    return Err(DocumentError(
                        "A janela não alcança o corpo: confira o rascunho e o sentido.".into(),
                    ));
                }
                cut
            }
        };
    }
    Ok(solid)
}
