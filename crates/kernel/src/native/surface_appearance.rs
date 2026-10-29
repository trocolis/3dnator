//! Partições de aparência para exibição e para uma cópia sólida de intercâmbio.
//! O sólido original nunca é alterado; os índices identificam a última camada.
use super::*;
unsafe extern "C" {
    fn nator_surface_appearance(
        body: *const c_void,
        faces: *const usize,
        tools: *const *const c_void,
        count: usize,
    ) -> *mut c_void;
    fn nator_painted_solid(
        body: *const c_void,
        faces: *const usize,
        tools: *const *const c_void,
        count: usize,
    ) -> *mut c_void;
}
impl Solid {
    /// Subdivide e costura as faces mantendo volume e cascas fechadas. Materiais
    /// são por face (0 = base, 1..=255 = camada), na ordem de `face_count`.
    pub fn painted_solid(
        &self,
        layers: &[(usize, Option<&Solid>)],
    ) -> Result<(Solid, Vec<u32>), KernelError> {
        if layers.is_empty()
            || layers.len() > 255
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
        // SAFETY: buffers e sólidos vivos nesta thread até o retorno. A ponte
        // devolve uma cópia exclusiva; exceções geométricas viram KernelError.
        let mut partition = Self::from_pointer(unsafe {
            nator_painted_solid(
                self.pointer.as_ptr(),
                faces.as_ptr(),
                tools.as_ptr(),
                layers.len(),
            )
        })?;
        let materials = partition
            .origins()?
            .into_iter()
            .map(|origin| {
                if origin[2] < 0 || origin[2] as usize > layers.len() {
                    return Err(KernelError("Índice de material inválido.".into()));
                }
                Ok(origin[2] as u32)
            })
            .collect::<Result<Vec<_>, KernelError>>()?;
        partition.faces = vec![String::new(); materials.len()];
        Ok((partition, materials))
    }
    pub fn surface_appearance(
        &self,
        layers: &[(usize, Option<&Solid>)],
    ) -> Result<(Mesh, Vec<u32>), KernelError> {
        if layers.is_empty()
            || layers.len() > 255
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

unsafe extern "C" {
    fn nator_face_common_area(
        first: *const c_void,
        a: usize,
        second: *const c_void,
        b: usize,
        output: *mut f64,
    ) -> bool;
}
impl Solid {
    pub fn face_common_area(&self, a: usize, other: &Self, b: usize) -> Result<f64, KernelError> {
        let mut area = 0.;
        // SAFETY: sólidos vivos na mesma thread, saída exclusiva com um double;
        // a ponte confere ambos os índices antes de consultar as faces.
        if !unsafe {
            nator_face_common_area(
                self.pointer.as_ptr(),
                a,
                other.pointer.as_ptr(),
                b,
                &mut area,
            )
        } {
            return Err(last_error());
        }
        Ok(area)
    }
}
