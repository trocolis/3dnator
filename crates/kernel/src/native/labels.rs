//! Fontes e rótulos geométricos: apenas contornos B-rep e bases próprias
//! atravessam a fronteira; objetos de fonte/superfície ficam na thread nativa.
use super::*;
use crate::{SurfaceFrame, TextMode, TextPlacement};
unsafe extern "C" {
    fn nator_font_names_size() -> usize;
    fn nator_font_names_copy(output: *mut c_char, length: usize) -> bool;
    fn nator_text_outline(
        text: *const c_char,
        font: *const c_char,
        bold: bool,
        italic: bool,
        underline: bool,
    ) -> *mut c_void;
    fn nator_surface_frame(
        body: *const c_void,
        index: usize,
        point: *const f64,
        uv: *const f64,
        fixed: bool,
        output: *mut f64,
    ) -> bool;
    fn nator_label(
        body: *const c_void,
        index: usize,
        outline: *const c_void,
        point: *const f64,
        uv: *const f64,
        fixed: bool,
        size: f64,
        angle: f64,
        depth: f64,
        mode: i32,
    ) -> *mut c_void;
}
impl Solid {
    pub fn font_names() -> Result<Vec<String>, KernelError> {
        // SAFETY: cache por thread; tamanho e cópia ocorrem sequencialmente.
        let size = unsafe { nator_font_names_size() };
        if size == 0 || size > 1024 * 1024 {
            return Err(last_error());
        }
        let mut bytes = vec![0; size];
        // SAFETY: buffer exclusivo com o tamanho consultado.
        if !unsafe { nator_font_names_copy(bytes.as_mut_ptr().cast(), size) } {
            return Err(last_error());
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| KernelError("Nome de fonte com codificação inválida.".into()))?;
        let mut names = text
            .lines()
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        Ok(names)
    }
    pub fn text_outline(
        text: &str,
        font: &str,
        bold: bool,
        italic: bool,
        underline: bool,
    ) -> Result<Vec<u8>, KernelError> {
        if text.chars().count() > 128
            || text.trim().is_empty()
            || text.contains(['\n', '\r'])
            || font.len() > 256
        {
            return Err(KernelError(
                "Use um texto de uma linha com até 128 caracteres e uma fonte válida.".into(),
            ));
        }
        let text =
            CString::new(text).map_err(|_| KernelError("Texto contém caractere nulo.".into()))?;
        let font = CString::new(font).map_err(|_| KernelError("Fonte inválida.".into()))?;
        // SAFETY: UTF-8 terminado em zero, vivo durante a chamada; retorno exclusivo.
        let mut outline = Self::from_pointer(unsafe {
            nator_text_outline(text.as_ptr(), font.as_ptr(), bold, italic, underline)
        })?;
        outline.to_brep()
    }
    pub fn surface_frame(
        &self,
        index: usize,
        point: Point3,
        uv: Option<[f64; 2]>,
    ) -> Result<SurfaceFrame, KernelError> {
        coordinates(point)?;
        if uv.is_some_and(|uv| uv.iter().any(|v| !v.is_finite())) {
            return Err(KernelError("Parâmetros da superfície inválidos.".into()));
        }
        let fixed = uv.is_some();
        let uv = uv.unwrap_or([0.; 2]);
        let mut out = [0.; 11];
        // SAFETY: buffers de entrada e saída fixos; índice validado na ponte.
        if !unsafe {
            nator_surface_frame(
                self.pointer.as_ptr(),
                index,
                point.as_ptr(),
                uv.as_ptr(),
                fixed,
                out.as_mut_ptr(),
            )
        } {
            return Err(last_error());
        }
        Ok(SurfaceFrame {
            origin: [out[0], out[1], out[2]],
            u: [out[3], out[4], out[5]],
            v: [out[6], out[7], out[8]],
            uv: [out[9], out[10]],
        })
    }
    pub fn label(
        &self,
        index: usize,
        outline: &[u8],
        placement: TextPlacement,
        owner: &str,
    ) -> Result<(Self, Vec<usize>), KernelError> {
        let TextPlacement {
            point,
            uv,
            size_mm: size,
            angle_rad: angle,
            depth_mm: depth,
            mode,
        } = placement;
        owner_name(owner)?;
        coordinates(point)?;
        positive(size, "Tamanho da fonte")?;
        if !angle.is_finite()
            || !depth.is_finite()
            || (!matches!(mode, TextMode::Surface) && depth <= 0.)
        {
            return Err(KernelError(
                "Rotação ou profundidade do rótulo inválida.".into(),
            ));
        }
        let glyphs = Self::from_brep(outline, None)?;
        let fixed = uv.is_some();
        let uv = uv.unwrap_or([0.; 2]);
        // SAFETY: formas e parâmetros vivos nesta thread; ponte valida a face e devolve uma forma exclusiva.
        let mut result = Self::from_pointer(unsafe {
            nator_label(
                self.pointer.as_ptr(),
                index,
                glyphs.pointer.as_ptr(),
                point.as_ptr(),
                uv.as_ptr(),
                fixed,
                size,
                angle,
                depth,
                match mode {
                    TextMode::Surface => 0,
                    TextMode::Raised => 1,
                    TextMode::Engraved => 2,
                },
            )
        })?;
        let origins = result.origins()?;
        let mut colored = vec![];
        result.faces = origins
            .iter()
            .enumerate()
            .map(|(index, origin)| {
                if origin[2] == 1 {
                    colored.push(index);
                    format!("{owner}:label")
                } else {
                    usize::try_from(origin[1])
                        .ok()
                        .and_then(|i| self.faces.get(i))
                        .cloned()
                        .unwrap_or_default()
                }
            })
            .collect();
        result.volume()?;
        if colored.is_empty() {
            return Err(KernelError("O texto não alcança a face escolhida.".into()));
        }
        Ok((result, colored))
    }
}
