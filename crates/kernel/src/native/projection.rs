//! Projeção exata de um conjunto de sólidos. A ponte classifica oclusões entre todos
//! os corpos de uma vez; apenas segmentos próprios em mm saem da integração nativa.
use super::{KernelError, Solid, last_error, unit};
use nator_geometry::{Point3, ProjectedSegment, dot};
use std::{ffi::c_void, ptr::NonNull};
unsafe extern "C" {
    fn nator_project(
        bodies: *const *const c_void,
        count: usize,
        right: *const f64,
        direction: *const f64,
        tolerance: f64,
    ) -> *mut c_void;
    fn nator_projection_size(projection: *const c_void) -> usize;
    fn nator_projection_copy(projection: *const c_void, output: *mut f64, count: usize) -> bool;
    fn nator_projection_release(projection: *mut c_void);
}
struct Projection(NonNull<c_void>);
impl Drop for Projection {
    fn drop(&mut self) {
        // SAFETY: resultado de propriedade exclusiva, liberado uma única vez nesta thread.
        unsafe {
            nator_projection_release(self.0.as_ptr());
        }
    }
}
pub fn projected_edges(
    bodies: &[Solid],
    right: Point3,
    direction: Point3,
    tolerance_mm: f64,
) -> Result<Vec<ProjectedSegment>, KernelError> {
    let right = unit(right, "Eixo horizontal da vista")?;
    let direction = unit(direction, "Direção da vista")?;
    if bodies.is_empty()
        || bodies.len() > 10000
        || dot(right, direction).abs() > 1e-8
        || !tolerance_mm.is_finite()
        || !(1e-6..=1.).contains(&tolerance_mm)
    {
        return Err(KernelError("Parâmetros de projeção inválidos.".into()));
    }
    let pointers = bodies
        .iter()
        .map(|s| s.pointer.as_ptr().cast_const())
        .collect::<Vec<_>>();
    // SAFETY: sólidos e buffers vivem durante o cálculo síncrono, na mesma thread.
    let result = NonNull::new(unsafe {
        nator_project(
            pointers.as_ptr(),
            pointers.len(),
            right.as_ptr(),
            direction.as_ptr(),
            tolerance_mm,
        )
    })
    .map(Projection)
    .ok_or_else(last_error)?;
    // SAFETY: consulta somente leitura ao resultado vivo.
    let count = unsafe { nator_projection_size(result.0.as_ptr()) };
    if count > 500000 * 5 || count % 5 != 0 {
        return Err(KernelError(
            "Contagem de segmentos projetados inválida.".into(),
        ));
    }
    let mut values = vec![0.; count];
    // SAFETY: buffer exclusivo com o tamanho informado pela própria ponte.
    if !unsafe { nator_projection_copy(result.0.as_ptr(), values.as_mut_ptr(), count) } {
        return Err(last_error());
    }
    if values.iter().any(|v| !v.is_finite()) {
        return Err(KernelError("Projeção contém coordenadas inválidas.".into()));
    }
    Ok(values
        .as_chunks::<5>()
        .0
        .iter()
        .map(|p| ProjectedSegment {
            a: [p[0], p[1]],
            b: [p[2], p[3]],
            hidden: p[4] > 0.5,
        })
        .collect())
}
