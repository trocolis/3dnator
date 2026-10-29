//! Transições mantêm curvas analíticas/B-splines do núcleo. Furos correspondem
//! pela ordem de criação de seus contornos e são subtraídos como transições próprias.
use super::*;
unsafe extern "C" {
    fn nator_loft_wire(
        first: *const c_void,
        last: *const c_void,
        wire: usize,
        ruled: bool,
    ) -> *mut c_void;
}
impl Solid {
    pub fn loft(
        first: (&Sketch, &Region),
        last: (&Sketch, &Region),
        ruled: bool,
        owner: &str,
    ) -> Result<Self, KernelError> {
        owner_name(owner)?;
        if first.1.holes.len() != last.1.holes.len() || first.1.holes.len() > 64 {
            return Err(KernelError("Os perfis precisam ter a mesma quantidade de furos (até 64), na ordem de criação correspondente.".into()));
        }
        let a = Self::profile(first.0, first.1, [0.; 3])?;
        let b = Self::profile(last.0, last.1, [0.; 3])?;
        let build = |wire| -> Result<Self, KernelError> {
            // SAFETY: perfis vivos, índice limitado aos contornos acima. A
            // ponte devolve forma exclusiva e converte exceções em KernelError.
            let mut solid = Self::from_pointer(unsafe {
                nator_loft_wire(
                    a.face.pointer.as_ptr(),
                    b.face.pointer.as_ptr(),
                    wire,
                    ruled,
                )
            })?;
            solid.faces = naming::sweep_names(owner, &solid.origins()?, &[]);
            solid.volume()?;
            Ok(solid)
        };
        let outer = build(0)?;
        let mut result = None;
        let mut holes: Vec<Solid> = vec![];
        for wire in 1..=first.1.holes.len() {
            let mut hole = build(wire)?;
            hole.map_face_names(|_| String::new());
            let volume = hole.volume()?;
            let tolerance = (volume * 1e-6).max(1e-7);
            let mut crossing = false;
            for previous in &holes {
                crossing |= previous.common_volume(&hole)? > tolerance;
            }
            if (outer.common_volume(&hole)? - volume).abs() > tolerance || crossing {
                return Err(KernelError("Os furos cruzam entre si ou saem do contorno da transição; revise a correspondência.".into()));
            }
            let previous = result.as_ref().unwrap_or(&outer);
            result = Some(previous.boolean(&hole, 1, owner)?);
            holes.push(hole);
        }
        Ok(result.unwrap_or(outer))
    }
}
