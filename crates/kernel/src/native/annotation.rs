//! Apoios de cotas obtidos da geometria precisa, nunca da tesselação de exibição.
use super::{Solid, last_error};
use crate::{EdgeAnchors, FaceAnchors, KernelError};
use std::ffi::c_void;
unsafe extern "C" {
    fn nator_face_anchors(body: *const c_void, index: usize, output: *mut f64) -> bool;
    fn nator_edge_anchors(body: *const c_void, index: usize, output: *mut f64) -> bool;
}
impl Solid {
    pub fn face_anchors(&self, index: usize) -> Result<FaceAnchors, KernelError> {
        let mut values = [0.; 10];
        // SAFETY: sólido vivo nesta thread e buffer exclusivo com dez doubles.
        if !unsafe { nator_face_anchors(self.pointer.as_ptr(), index, values.as_mut_ptr()) } {
            return Err(last_error());
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err(KernelError("Apoio de face inválido.".into()));
        }
        Ok(FaceAnchors {
            centroid: [values[0], values[1], values[2]],
            radial: (values[9] > 0.5).then_some([
                [values[3], values[4], values[5]],
                [values[6], values[7], values[8]],
            ]),
        })
    }
    pub fn edge_anchors(&self, index: usize) -> Result<EdgeAnchors, KernelError> {
        let mut values = [0.; 10];
        // SAFETY: sólido vivo e buffer de dez doubles, cujo tamanho faz parte do ABI.
        if !unsafe { nator_edge_anchors(self.pointer.as_ptr(), index, values.as_mut_ptr()) } {
            return Err(last_error());
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err(KernelError("Apoio de aresta inválido.".into()));
        }
        Ok(EdgeAnchors {
            start: [values[0], values[1], values[2]],
            end: [values[3], values[4], values[5]],
            circle_center: (values[9] > 0.5).then_some([values[6], values[7], values[8]]),
        })
    }
}
