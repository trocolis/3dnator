//! Tipos próprios de intercâmbio STEP. Objetos OCCT e o documento XCAF ficam nesta thread.
use super::{KernelError, Solid, last_error, path_string};
use nator_geometry::{Placement, RgbColor};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    path::Path,
    ptr::NonNull,
};

pub struct StepBody {
    pub name: String,
    pub solid: Solid,
    pub color: Option<RgbColor>,
}
pub struct StepPart {
    pub name: String,
    pub bodies: Vec<StepBody>,
}
pub struct StepOccurrence {
    pub part: usize,
    /// Índice da definição pai; ausência identifica uma raiz da montagem.
    pub parent: Option<usize>,
    pub name: String,
    pub placement: Placement,
    pub color: Option<RgbColor>,
}
pub struct StepAssembly {
    pub parts: Vec<StepPart>,
    pub instances: Vec<StepOccurrence>,
}
unsafe extern "C" {
    fn nator_step_scene_new() -> *mut c_void;
    fn nator_step_scene_release(scene: *mut c_void);
    fn nator_step_scene_read(path: *const c_char) -> *mut c_void;
    fn nator_step_scene_write(scene: *const c_void, path: *const c_char) -> bool;
    fn nator_step_part_count(scene: *const c_void) -> usize;
    fn nator_step_instance_count(scene: *const c_void) -> usize;
    fn nator_step_body_count(scene: *const c_void, part: usize) -> usize;
    fn nator_step_name(scene: *const c_void, part: usize, body: usize, kind: i32) -> *const c_char;
    fn nator_step_body(
        scene: *const c_void,
        part: usize,
        body: usize,
        color: *mut i32,
    ) -> *mut c_void;
    fn nator_step_instance(
        scene: *const c_void,
        index: usize,
        part: *mut usize,
        parent: *mut i64,
        matrix: *mut f64,
        color: *mut i32,
    ) -> bool;
    fn nator_step_add_part(scene: *mut c_void, name: *const c_char) -> bool;
    fn nator_step_add_body(
        scene: *mut c_void,
        part: usize,
        body: *const c_void,
        name: *const c_char,
        color: *const i32,
    ) -> bool;
    fn nator_step_add_instance(
        scene: *mut c_void,
        part: usize,
        parent: i64,
        name: *const c_char,
        matrix: *const f64,
        color: *const i32,
    ) -> bool;
}
struct Scene(NonNull<c_void>);
impl Scene {
    fn new(pointer: *mut c_void) -> Result<Self, KernelError> {
        NonNull::new(pointer).map(Self).ok_or_else(last_error)
    }
    fn pointer(&self) -> *mut c_void {
        self.0.as_ptr()
    }
    fn name(&self, part: usize, body: usize, kind: i32) -> Result<String, KernelError> {
        // SAFETY: cena viva; índices vieram das contagens da ponte. O texto permanece
        // válido até a próxima mutação da cena e é copiado antes dela.
        let pointer = unsafe { nator_step_name(self.pointer(), part, body, kind) };
        if pointer.is_null() {
            return Err(KernelError("Nome STEP ausente.".into()));
        }
        // SAFETY: a ponte fornece string terminada em NUL pertencente à cena viva.
        let text = unsafe { CStr::from_ptr(pointer) }
            .to_str()
            .map_err(|_| KernelError("Nome STEP não é UTF-8 válido.".into()))?;
        checked_name(text)?;
        Ok(text.into())
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        // SAFETY: propriedade exclusiva recebida da ponte; liberação ocorre uma só vez.
        unsafe {
            nator_step_scene_release(self.pointer());
        }
    }
}
fn checked_name(text: &str) -> Result<CString, KernelError> {
    if text.trim().is_empty() || text.len() > 1024 {
        return Err(KernelError("Nome STEP vazio ou excessivo.".into()));
    }
    CString::new(text).map_err(|_| KernelError("Nome STEP contém NUL.".into()))
}
fn color(channels: [i32; 3]) -> Result<Option<RgbColor>, KernelError> {
    if channels == [-1; 3] {
        return Ok(None);
    }
    if channels.iter().any(|c| !(0..=255).contains(c)) {
        return Err(KernelError("Cor STEP inválida.".into()));
    }
    Ok(Some(RgbColor::new(channels.map(|c| c as u8))))
}
fn channels(color: Option<RgbColor>) -> [i32; 3] {
    color
        .map(|c| c.channels().map(i32::from))
        .unwrap_or([-1; 3])
}
fn check(ok: bool) -> Result<(), KernelError> {
    if ok { Ok(()) } else { Err(last_error()) }
}
impl StepAssembly {
    /// Limita o grafo antes de entregá-lo à recursão nativa. Alturas memorizadas
    /// evitam trabalho exponencial em montagens com definições repetidas por vários
    /// caminhos e verificam o maior caminho mesmo quando um filho já foi visitado.
    fn validate(&self) -> Result<(), KernelError> {
        if self.parts.is_empty()
            || self.parts.len() > 10000
            || self.instances.len() > 10000
            || self.parts.iter().map(|p| p.bodies.len()).sum::<usize>() > 10000
        {
            return Err(KernelError("Contagem de elementos STEP inválida.".into()));
        }
        let mut children = vec![Vec::new(); self.parts.len()];
        let mut roots = Vec::new();
        for instance in &self.instances {
            if instance.part >= self.parts.len()
                || instance.parent.is_some_and(|p| p >= self.parts.len())
            {
                return Err(KernelError("Referência STEP inexistente.".into()));
            }
            if let Some(parent) = instance.parent {
                children[parent].push(instance.part);
            } else {
                roots.push(instance.part);
            }
        }
        fn height(
            part: usize,
            children: &[Vec<usize>],
            visiting: &mut [bool],
            heights: &mut [Option<usize>],
            depth: usize,
        ) -> Result<usize, KernelError> {
            if depth > 64 || visiting[part] {
                return Err(KernelError("Hierarquia STEP inválida ou cíclica.".into()));
            }
            if let Some(height) = heights[part] {
                return Ok(height);
            }
            visiting[part] = true;
            let mut result = 0;
            for child in &children[part] {
                result = result.max(height(*child, children, visiting, heights, depth + 1)? + 1);
            }
            visiting[part] = false;
            if result > 64 {
                return Err(KernelError("Hierarquia STEP excede 64 níveis.".into()));
            }
            heights[part] = Some(result);
            Ok(result)
        }
        let mut heights = vec![None; self.parts.len()];
        let mut visiting = vec![false; self.parts.len()];
        for root in roots {
            height(root, &children, &mut visiting, &mut heights, 0)?;
        }
        if heights.iter().any(Option::is_none) {
            return Err(KernelError(
                "STEP contém definições sem uma raiz válida.".into(),
            ));
        }
        Ok(())
    }
    pub fn read(path: &Path) -> Result<Self, KernelError> {
        let _guard = super::step_guard()?;
        let metadata = std::fs::metadata(path)
            .map_err(|e| KernelError(format!("Não foi possível ler o STEP: {e}")))?;
        if !metadata.is_file() || metadata.len() > 256 * 1024 * 1024 {
            return Err(KernelError(
                "STEP deve ser um arquivo de até 256 MiB.".into(),
            ));
        }
        let path = path_string(path)?;
        // SAFETY: caminho vive durante a leitura; a cena retornada possui propriedade exclusiva.
        let scene = Scene::new(unsafe { nator_step_scene_read(path.as_ptr()) })?;
        // SAFETY: consulta somente leitura em cena viva desta thread.
        let (part_count, instance_count) = unsafe {
            (
                nator_step_part_count(scene.pointer()),
                nator_step_instance_count(scene.pointer()),
            )
        };
        if part_count == 0 || part_count > 10000 || instance_count > 10000 {
            return Err(KernelError("Contagem de elementos STEP inválida.".into()));
        }
        let mut parts = Vec::with_capacity(part_count);
        let mut total_bodies = 0;
        for part in 0..part_count {
            // SAFETY: índice dentro da contagem validada; cena viva.
            let count = unsafe { nator_step_body_count(scene.pointer(), part) };
            total_bodies += count;
            if total_bodies > 10000 {
                return Err(KernelError("STEP excede dez mil corpos.".into()));
            }
            let mut bodies = Vec::with_capacity(count);
            for body in 0..count {
                let mut rgb = [-1; 3];
                // SAFETY: índices válidos e buffer de três canais exclusivo. A forma
                // resultante é nova e sua propriedade é transferida para Solid.
                let mut solid = Solid::from_pointer(unsafe {
                    nator_step_body(scene.pointer(), part, body, rgb.as_mut_ptr())
                })?;
                solid.faces = vec![String::new(); solid.face_count()?];
                solid.volume()?;
                bodies.push(StepBody {
                    name: scene.name(part, body, 1)?,
                    solid,
                    color: color(rgb)?,
                });
            }
            parts.push(StepPart {
                name: scene.name(part, 0, 0)?,
                bodies,
            });
        }
        let mut instances = Vec::with_capacity(instance_count);
        for index in 0..instance_count {
            let (mut part, mut parent, mut matrix, mut rgb) = (0, -1, [0.; 12], [-1; 3]);
            // SAFETY: índice validado e destinos exclusivos com os comprimentos do ABI.
            check(unsafe {
                nator_step_instance(
                    scene.pointer(),
                    index,
                    &mut part,
                    &mut parent,
                    matrix.as_mut_ptr(),
                    rgb.as_mut_ptr(),
                )
            })?;
            if part >= part_count || parent < -1 || parent >= part_count as i64 {
                return Err(KernelError("Referência STEP inexistente.".into()));
            }
            instances.push(StepOccurrence {
                part,
                parent: (parent >= 0).then_some(parent as usize),
                name: scene.name(index, 0, 2)?,
                placement: Placement::from_affine_rows(matrix)
                    .map_err(|e| KernelError(e.to_string()))?,
                color: color(rgb)?,
            });
        }
        let assembly = Self { parts, instances };
        assembly.validate()?;
        Ok(assembly)
    }
    pub fn write(&self, path: &Path) -> Result<(), KernelError> {
        let _guard = super::step_guard()?;
        if self.parts.is_empty() || self.parts.len() > 10000 || self.instances.len() > 10000 {
            return Err(KernelError("Contagem de elementos STEP inválida.".into()));
        }
        self.validate()?;
        // SAFETY: nova cena de propriedade exclusiva nesta thread.
        let scene = Scene::new(unsafe { nator_step_scene_new() })?;
        for (index, part) in self.parts.iter().enumerate() {
            let name = checked_name(&part.name)?;
            // SAFETY: texto vive durante a chamada e é copiado para a cena.
            check(unsafe { nator_step_add_part(scene.pointer(), name.as_ptr()) })?;
            for body in &part.bodies {
                let name = checked_name(&body.name)?;
                let rgb = channels(body.color);
                // SAFETY: cena e sólido vivos; nome e três canais são copiados.
                check(unsafe {
                    nator_step_add_body(
                        scene.pointer(),
                        index,
                        body.solid.pointer.as_ptr(),
                        name.as_ptr(),
                        rgb.as_ptr(),
                    )
                })?;
            }
        }
        for i in &self.instances {
            if i.part >= self.parts.len() || i.parent.is_some_and(|p| p >= self.parts.len()) {
                return Err(KernelError("Referência STEP inexistente.".into()));
            }
            let name = checked_name(&i.name)?;
            let matrix = i.placement.affine_rows();
            let rgb = channels(i.color);
            // SAFETY: referências validadas e buffers vivos, com os tamanhos do ABI.
            check(unsafe {
                nator_step_add_instance(
                    scene.pointer(),
                    i.part,
                    i.parent.map(|p| p as i64).unwrap_or(-1),
                    name.as_ptr(),
                    matrix.as_ptr(),
                    rgb.as_ptr(),
                )
            })?;
        }
        let path = path_string(path)?;
        // SAFETY: cena e caminho vivos até a escrita terminar.
        check(unsafe { nator_step_scene_write(scene.pointer(), path.as_ptr()) })
    }
}
