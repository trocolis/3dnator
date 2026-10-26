//! Fronteira do núcleo CAD: detalhes e tipos nativos não saem deste crate.

use std::{error::Error, fmt};

#[derive(Debug)]
pub struct KernelError(pub String);
impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for KernelError {}

pub fn is_available() -> bool {
    cfg!(feature = "occt")
}

#[cfg(feature = "occt")]
#[allow(unsafe_code)]
mod native;
#[cfg(feature = "occt")]
pub use native::{Solid, version};

/// Não existe núcleo alternativo silencioso: sem SDK, a operação é indisponível.
pub fn require_available() -> Result<(), KernelError> {
    if is_available() {
        Ok(())
    } else {
        Err(KernelError("OCCT não habilitado. Instale o SDK e compile com --features occt. Consulte docs/PROTOTYPE.md.".into()))
    }
}
