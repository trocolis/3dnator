//! A partição de aparência não retorna um sólido de modelagem: somente uma
//! malha de exibição e o índice de material por triângulo atravessam a fronteira.
use super::*;
unsafe extern "C" {
    fn nator_surface_appearance(
        body: *const c_void,
        faces: *const usize,
        tools: *const *const c_void,
        count: usize,
    ) -> *mut c_void;
}
impl Solid {
    pub fn surface_appearance(
        &self,
        layers: &[(usize, Option<&Solid>)],
    ) -> Result<(Mesh, Vec<u32>), KernelError> {
        if layers.is_empty()
            || layers.len() > 64
            || layers.iter().any(|(face, _)| *face >= self.faces.len())
        {
            return Err(KernelError("Seleção de pintura inválida.".into()));
        }
        let faces = layers.iter().map(|(face, _)| *face).collect::<Vec<_>>();
        let tools = layers
            .iter()
            .map(|(_, tool)| {
                tool.map(|s| s.pointer.as_ptr().cast_const())
                    .unwrap_or(std::ptr::null())
            })
            .collect::<Vec<_>>();
        // SAFETY: todos os objetos pertencem a esta thread e vivem até a chamada
        // terminar. O resultado é exclusivo e usa o mesmo Drop dos demais Shape.
        let mut partition = Self::from_pointer(unsafe {
            nator_surface_appearance(
                self.pointer.as_ptr(),
                faces.as_ptr(),
                tools.as_ptr(),
                layers.len(),
            )
        })?;
        let origins = partition.origins()?;
        let (mesh, faces) = partition.tessellate_with_faces(0.02, 0.2)?;
        let materials = faces
            .iter()
            .map(|face| {
                let material = origins
                    .get(*face as usize)
                    .map(|o| o[2])
                    .ok_or_else(|| KernelError("Material de face ausente.".into()))?;
                if material < 0 || material as usize > layers.len() {
                    return Err(KernelError("Índice de material inválido.".into()));
                }
                Ok(material as u32)
            })
            .collect::<Result<Vec<_>, KernelError>>()?;
        Ok((mesh, materials))
    }
}
