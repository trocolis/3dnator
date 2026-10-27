//! Apoios de cotas e captura obtidos da geometria precisa, sem depender da tesselação.
use super::{Solid, last_error};
use crate::{CircleAnchor, EdgeAnchors, FaceAnchors, KernelError};
use std::ffi::c_void;
unsafe extern "C" {
    fn nator_face_polygon(body: *const c_void, index: usize, output: *mut f64) -> bool;
    fn nator_face_anchors(body: *const c_void, index: usize, output: *mut f64) -> bool;
    fn nator_edge_anchors(body: *const c_void, index: usize, output: *mut f64) -> bool;
}
impl Solid {
    /// Centro do contorno externo triangular/retangular, no plano da face. Não é
    /// centro de massa: um furo assimétrico não desloca este ponto de referência.
    pub fn face_polygon_center(&self, index: usize) -> Result<Option<[f64; 3]>, KernelError> {
        let mut values = [0.; 13];
        // SAFETY: sólido vivo e buffer exclusivo de treze doubles conforme a ponte.
        if !unsafe { nator_face_polygon(self.pointer.as_ptr(), index, values.as_mut_ptr()) } {
            return Err(last_error());
        }
        let count = match values[0] {
            0. => return Ok(None),
            3. => 3,
            4. => 4,
            _ => return Err(KernelError("Contorno de face inválido.".into())),
        };
        let points = values[1..1 + count * 3].as_chunks::<3>().0;
        Ok(nator_geometry::polygon_center(points))
    }
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
        let mut values = [0.; 24];
        // SAFETY: sólido vivo e buffer de vinte e quatro doubles, conforme o ABI da ponte.
        if !unsafe { nator_edge_anchors(self.pointer.as_ptr(), index, values.as_mut_ptr()) } {
            return Err(last_error());
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err(KernelError("Apoio de aresta inválido.".into()));
        }
        Ok(EdgeAnchors {
            start: [values[0], values[1], values[2]],
            end: [values[3], values[4], values[5]],
            midpoint: [values[16], values[17], values[18]],
            circle: (values[9] > 0.5).then_some(CircleAnchor {
                center: [values[6], values[7], values[8]],
                normal: [values[10], values[11], values[12]],
                x_direction: [values[19], values[20], values[21]],
                radius_mm: values[13],
                start_angle_rad: values[22],
                sweep_rad: values[23],
                closed: values[14] > 0.5,
            }),
            seam: values[15] > 0.5,
        })
    }
}
