//! Recorte dos furos do apoio: somente dados geométricos atravessam a ponte.
use super::*;
unsafe extern "C" {
    fn nator_support_holes(
        body: *const c_void,
        origin: *const f64,
        normal: *const f64,
        distance: f64,
    ) -> *mut c_void;
}
impl Solid {
    /// Contornos internos coplanares ao início, extrudidos na direção solicitada.
    /// Todos os valores estão em mm e no mesmo referencial do sólido de apoio.
    pub fn support_holes(
        &self,
        origin: Point3,
        normal: Point3,
        distance: f64,
        owner: &str,
    ) -> Result<Option<Self>, KernelError> {
        owner_name(owner)?;
        if !origin.iter().chain(normal.iter()).all(|v| v.is_finite())
            || nator_geometry::norm(normal) < 1e-12
            || !distance.is_finite()
            || distance.abs() < 1e-7
        {
            return Err(KernelError(
                "Plano inicial ou comprimento inválido para preservar furos.".into(),
            ));
        }
        // SAFETY: sólido vivo nesta thread; os vetores têm três doubles e a
        // ponte não guarda seus ponteiros. O novo Shape usa o Drop habitual.
        let mut holes = Self::from_pointer(unsafe {
            nator_support_holes(
                self.pointer.as_ptr(),
                origin.as_ptr(),
                normal.as_ptr(),
                distance,
            )
        })?;
        if holes.face_count()? == 0 {
            return Ok(None);
        }
        holes.name_faces_by_index(owner)?;
        holes.map_face_names(|name| format!("{name}:support"));
        Ok(Some(holes))
    }
}
