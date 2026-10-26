//! Interseção precisa do corpo com um plano. O resultado contém segmentos em coordenadas
//! de mundo (mm), amostrados das curvas do B-rep sem alterar a geometria de entrada.
use super::{KernelError, Solid, coordinates, last_error, unit};
use nator_geometry::Point3;
use std::{ffi::c_void, ptr::NonNull};
unsafe extern "C" {
    fn nator_section(
        body: *const c_void,
        origin: *const f64,
        normal: *const f64,
        tolerance: f64,
    ) -> *mut c_void;
    fn nator_section_size(section: *const c_void) -> usize;
    fn nator_section_copy(section: *const c_void, output: *mut f64, size: usize) -> bool;
    fn nator_section_release(section: *mut c_void);
}
struct Section(NonNull<c_void>);
impl Drop for Section {
    fn drop(&mut self) {
        // SAFETY: propriedade exclusiva da interseção recebida da ponte.
        unsafe {
            nator_section_release(self.0.as_ptr());
        }
    }
}
impl Solid {
    pub fn section_segments(
        &self,
        origin: Point3,
        normal: Point3,
        tolerance_mm: f64,
    ) -> Result<Vec<[Point3; 2]>, KernelError> {
        coordinates(origin)?;
        let normal = unit(normal, "Normal do corte")?;
        if !tolerance_mm.is_finite() || !(1e-6..=1.).contains(&tolerance_mm) {
            return Err(KernelError("Tolerância do corte inválida.".into()));
        }
        // SAFETY: sólido e buffers vivem durante a chamada nesta thread; a ponte
        // retorna uma interseção nova de propriedade exclusiva, sem modificar o sólido.
        let result = NonNull::new(unsafe {
            nator_section(
                self.pointer.as_ptr(),
                origin.as_ptr(),
                normal.as_ptr(),
                tolerance_mm,
            )
        })
        .map(Section)
        .ok_or_else(last_error)?;
        // SAFETY: consulta somente leitura em resultado vivo.
        let count = unsafe { nator_section_size(result.0.as_ptr()) };
        if count > 500000 * 6 || count % 6 != 0 {
            return Err(KernelError("Contagem do corte inválida.".into()));
        }
        let mut values = vec![0.; count];
        // SAFETY: buffer exclusivo com o tamanho fornecido pela própria ponte.
        if !unsafe { nator_section_copy(result.0.as_ptr(), values.as_mut_ptr(), count) } {
            return Err(last_error());
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err(KernelError("O corte contém coordenadas inválidas.".into()));
        }
        Ok(values
            .as_chunks::<6>()
            .0
            .iter()
            .map(|p| [[p[0], p[1], p[2]], [p[3], p[4], p[5]]])
            .collect())
    }
}
