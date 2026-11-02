//! Contatos em linha de um sólido e seu reparo. Só a aresta e os nomes das faces em volta
//! dela identificam o contato; a geometria devolvida serve para apresentação.
use super::{KernelError, Solid, last_error, owner_name, positive};
use crate::{Contact, naming};
use std::ffi::c_void;
unsafe extern "C" {
    fn nator_contacts_size(body: *const c_void, size: *mut usize) -> bool;
    fn nator_contacts_copy(body: *const c_void, output: *mut f64, length: usize) -> bool;
    fn nator_contact_pin(body: *const c_void, index: usize, radius: f64) -> *mut c_void;
}

impl Solid {
    /// Arestas compartilhadas por mais de duas faces, em ordem topológica.
    pub fn contacts(&self) -> Result<Vec<Contact>, KernelError> {
        let mut size = 0;
        // SAFETY: sólido vivo e destino exclusivo de um usize.
        if !unsafe { nator_contacts_size(self.pointer.as_ptr(), &mut size) } {
            return Err(last_error());
        }
        let mut records = vec![0.; size];
        // SAFETY: buffer exclusivo do tamanho que a ponte acabou de informar.
        if !unsafe { nator_contacts_copy(self.pointer.as_ptr(), records.as_mut_ptr(), size) } {
            return Err(last_error());
        }
        let invalid = || KernelError("Registro de contatos inválido.".into());
        let mut contacts = Vec::new();
        let mut rest = records.as_slice();
        while let [edge, count, tail @ ..] = rest {
            let count = *count as usize;
            let indices = tail.get(..count).ok_or_else(invalid)?;
            let mut faces = indices
                .iter()
                .map(|i| self.faces.get(*i as usize).cloned().ok_or_else(invalid))
                .collect::<Result<Vec<_>, _>>()?;
            faces.sort();
            let (samples, tail) = tail[count..].split_first().ok_or_else(invalid)?;
            let samples = *samples as usize;
            let points = tail.get(..samples * 3).ok_or_else(invalid)?;
            contacts.push(Contact {
                edge: *edge as usize,
                faces,
                points: points.as_chunks().0.to_vec(),
                length: self.edge_measure(*edge as usize)?.length,
            });
            rest = &tail[samples * 3..];
        }
        Ok(contacts)
    }

    /// Une (`weld`) ou subtrai um pino de `radius` mm ao longo do contato. `sequence`
    /// distingue os nomes das faces quando a mesma operação repara vários contatos.
    pub fn repair_contact(
        &self,
        edge: usize,
        radius: f64,
        weld: bool,
        sequence: usize,
        owner: &str,
    ) -> Result<Self, KernelError> {
        owner_name(owner)?;
        positive(radius, "Raio")?;
        // SAFETY: sólido vivo; a ponte valida o índice e devolve propriedade exclusiva.
        let mut pin =
            Self::from_pointer(unsafe { nator_contact_pin(self.pointer.as_ptr(), edge, radius) })?;
        pin.faces = naming::contact_names(owner, sequence, pin.face_count()?);
        self.boolean(&pin, if weld { 0 } else { 1 }, owner)
    }
}
