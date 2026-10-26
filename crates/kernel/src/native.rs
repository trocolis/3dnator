//! Único módulo Rust autorizado a chamar o ABI nativo.
//! Cada sólido tem propriedade exclusiva e permanece na thread que o criou.

use super::KernelError;
use nator_geometry::{Mesh, Point3};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    marker::PhantomData,
    path::Path,
    ptr::NonNull,
    rc::Rc,
};

unsafe extern "C" {
    fn nator_profile(
        records: *const f64,
        count: usize,
        ends: *const usize,
        wires: usize,
        plane: *const f64,
    ) -> *mut c_void;
    fn nator_extrude(face: *const c_void, direction: *const f64) -> *mut c_void;
    fn nator_revolve(
        face: *const c_void,
        origin: *const f64,
        direction: *const f64,
        angle: f64,
    ) -> *mut c_void;
    fn nator_boolean(a: *const c_void, b: *const c_void, mode: i32) -> *mut c_void;
    fn nator_transform(body: *const c_void, matrix: *const f64) -> *mut c_void;
    fn nator_shell(body: *const c_void, face: usize, thickness: f64) -> *mut c_void;
    fn nator_brep_prepare(body: *mut c_void) -> bool;
    fn nator_brep_size(body: *const c_void) -> usize;
    fn nator_brep_copy(body: *const c_void, target: *mut c_char, size: usize) -> bool;
    fn nator_brep_read(bytes: *const c_char, size: usize) -> *mut c_void;
    fn nator_last_error() -> *const c_char;
    fn nator_version() -> *const c_char;
    fn nator_release(shape: *mut c_void);
    fn nator_box(x: f64, y: f64, z: f64, dx: f64, dy: f64, dz: f64) -> *mut c_void;
    fn nator_cylinder(x: f64, y: f64, z: f64, radius: f64, height: f64) -> *mut c_void;
    fn nator_cut(body: *const c_void, tool: *const c_void) -> *mut c_void;
    fn nator_fillet(body: *const c_void, edge: usize, radius: f64) -> *mut c_void;
    fn nator_chamfer(body: *const c_void, edge: usize, distance: f64) -> *mut c_void;
    fn nator_volume(body: *const c_void, volume: *mut f64) -> bool;
    fn nator_read_step(path: *const c_char) -> *mut c_void;
    fn nator_write_step(body: *const c_void, path: *const c_char) -> bool;
    fn nator_tessellate(body: *mut c_void, linear: f64, angular: f64) -> bool;
    fn nator_mesh_size(body: *const c_void) -> usize;
    fn nator_mesh_copy(body: *const c_void, output: *mut f64, length: usize) -> bool;
}

pub struct Solid {
    pointer: NonNull<c_void>,
    // Rc impede Send/Sync: objetos nativos não atravessam threads por acidente.
    _thread: PhantomData<Rc<()>>,
}

fn last_error() -> KernelError {
    // SAFETY: a ponte devolve uma string terminada em NUL válida até a próxima chamada nesta thread.
    KernelError(
        unsafe { CStr::from_ptr(nator_last_error()) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn positive(value: f64, name: &str) -> Result<(), KernelError> {
    if value.is_finite() && value > 0. {
        Ok(())
    } else {
        Err(KernelError(format!("{name} deve ser positivo e finito.")))
    }
}

fn coordinates(origin: Point3) -> Result<(), KernelError> {
    if origin.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(KernelError(
            "A origem contém coordenadas não finitas.".into(),
        ))
    }
}

fn path_string(path: &Path) -> Result<CString, KernelError> {
    let text = path
        .to_str()
        .ok_or_else(|| KernelError("O caminho deve ser UTF-8 válido.".into()))?;
    CString::new(text).map_err(|_| KernelError("O caminho contém um caractere NUL.".into()))
}

pub fn version() -> String {
    // SAFETY: texto estático da versão do SDK vinculado.
    unsafe { CStr::from_ptr(nator_version()) }
        .to_string_lossy()
        .into_owned()
}

impl Solid {
    /// Os dados B-rep imutáveis podem atravessar threads; objetos OCCT vivos não podem.
    pub fn from_brep(bytes: &[u8]) -> Result<Self, KernelError> {
        if bytes.is_empty() || bytes.len() > 64 * 1024 * 1024 {
            return Err(KernelError("B-rep vazio ou excessivo.".into()));
        }
        // SAFETY: fatia vive durante a leitura síncrona; a ponte valida e copia o conteúdo.
        Self::from_pointer(unsafe { nator_brep_read(bytes.as_ptr().cast(), bytes.len()) })
    }
    pub fn to_brep(&mut self) -> Result<Vec<u8>, KernelError> {
        // SAFETY: acesso exclusivo ao objeto vivo nesta thread.
        if !unsafe { nator_brep_prepare(self.pointer.as_ptr()) } {
            return Err(last_error());
        }
        // SAFETY: tamanho pertence ao mesmo objeto, que permanece vivo.
        let size = unsafe { nator_brep_size(self.pointer.as_ptr()) };
        if size == 0 || size > 64 * 1024 * 1024 {
            return Err(KernelError("B-rep vazio ou excessivo.".into()));
        }
        let mut bytes = vec![0; size];
        // SAFETY: destino exclusivo com comprimento exato, também validado na ponte.
        if !unsafe { nator_brep_copy(self.pointer.as_ptr(), bytes.as_mut_ptr().cast(), size) } {
            return Err(last_error());
        }
        Ok(bytes)
    }
    pub fn boolean(&self, other: &Self, mode: u32) -> Result<Self, KernelError> {
        if mode > 2 {
            return Err(KernelError("Operação booleana inválida.".into()));
        }
        // SAFETY: ambos os sólidos estão vivos; a operação é não destrutiva.
        Self::from_pointer(unsafe {
            nator_boolean(self.pointer.as_ptr(), other.pointer.as_ptr(), mode as i32)
        })
    }
    pub fn transformed(&self, placement: nator_geometry::Placement) -> Result<Self, KernelError> {
        let origin = placement.apply([0.; 3]);
        let columns = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
            .map(|p| nator_geometry::sub(placement.apply(p), origin));
        let data = std::array::from_fn::<_, 12, _>(|i| {
            if i % 4 == 3 {
                origin[i / 4]
            } else {
                columns[i % 4][i / 4]
            }
        });
        // SAFETY: matriz rígida finita em buffer fixo durante a chamada.
        Self::from_pointer(unsafe { nator_transform(self.pointer.as_ptr(), data.as_ptr()) })
    }
    pub fn shell(&self, face: usize, thickness: f64) -> Result<Self, KernelError> {
        positive(thickness, "Espessura")?;
        // SAFETY: ponte valida índice e constrói geometria nova.
        Self::from_pointer(unsafe { nator_shell(self.pointer.as_ptr(), face, thickness) })
    }
    pub fn extrude(
        sketch: &nator_sketch::Sketch,
        region: usize,
        from: f64,
        to: f64,
    ) -> Result<Self, KernelError> {
        if !from.is_finite() || !to.is_finite() || (to - from).abs() < 1e-7 {
            return Err(KernelError("Limites de extrusão inválidos.".into()));
        }
        let mut profile = Self::profile(sketch, region)?;
        if from != 0. {
            profile = profile.transformed(
                nator_geometry::Placement::new(sketch.plane.normal().map(|v| v * from), [0.; 3])
                    .map_err(|e| KernelError(e.to_string()))?,
            )?;
        }
        let direction = sketch.plane.normal().map(|v| v * (to - from));
        // SAFETY: perfil vivo e vetor finito não nulo, sem retenção dos buffers.
        let solid = Self::from_pointer(unsafe {
            nator_extrude(profile.pointer.as_ptr(), direction.as_ptr())
        })?;
        solid.volume()?;
        Ok(solid)
    }
    pub fn revolve(
        sketch: &nator_sketch::Sketch,
        region: usize,
        origin: Point3,
        direction: Point3,
        angle: f64,
    ) -> Result<Self, KernelError> {
        coordinates(origin)?;
        coordinates(direction)?;
        if nator_geometry::norm(direction) < 1e-9
            || !angle.is_finite()
            || angle.abs() < 1e-7
            || angle.abs() > std::f64::consts::TAU + 1e-9
        {
            return Err(KernelError("Eixo ou ângulo de revolução inválido.".into()));
        }
        let profile = Self::profile(sketch, region)?;
        // SAFETY: buffers de três coordenadas vivos e perfil válido durante a chamada.
        let solid = Self::from_pointer(unsafe {
            nator_revolve(
                profile.pointer.as_ptr(),
                origin.as_ptr(),
                direction.as_ptr(),
                angle,
            )
        })?;
        solid.volume()?;
        Ok(solid)
    }
    fn profile(sketch: &nator_sketch::Sketch, region: usize) -> Result<Self, KernelError> {
        use nator_sketch::profiles::{self, Segment};
        let (mut wires, diagnostics) =
            profiles::analyze(sketch).map_err(|e| KernelError(e.to_string()))?;
        if !diagnostics.open_points.is_empty()
            || !diagnostics.crossings.is_empty()
            || !diagnostics.duplicate_entities.is_empty()
        {
            return Err(KernelError(
                "Feche os contornos e remova cruzamentos/duplicações antes de criar o sólido."
                    .into(),
            ));
        }
        wires.sort_by(|a, b| b.area().abs().total_cmp(&a.area().abs()));
        let parents = wires
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let first = w.sample(64).first().copied().unwrap_or([0.; 2]);
                (0..i).rev().find(|p| wires[*p].contains(first))
            })
            .collect::<Vec<_>>();
        let depth = |mut i: usize| {
            let mut d = 0;
            while let Some(p) = parents[i] {
                d += 1;
                i = p;
            }
            d
        };
        let outer = (0..wires.len())
            .filter(|i| depth(*i) % 2 == 0)
            .nth(region)
            .ok_or_else(|| KernelError("Região do rascunho inexistente.".into()))?;
        let mut records = Vec::new();
        let mut ends = Vec::new();
        let plane = sketch.plane;
        for i in
            std::iter::once(outer).chain((0..wires.len()).filter(|i| parents[*i] == Some(outer)))
        {
            let reverse = (wires[i].area() > 0.) != (i == outer);
            let segments = if reverse {
                wires[i]
                    .segments
                    .iter()
                    .rev()
                    .map(Segment::reversed)
                    .collect::<Vec<_>>()
            } else {
                wires[i].segments.clone()
            };
            for segment in segments {
                let (kind, a, b, c, r) = match segment {
                    Segment::Line { a, b } => (0., plane.world(a), plane.world(b), [0.; 3], 0.),
                    Segment::Arc { .. } => {
                        let points = segment.sample(4);
                        (
                            1.,
                            plane.world(points[0]),
                            plane.world(points[points.len() / 2]),
                            plane.world(points[points.len() - 1]),
                            0.,
                        )
                    }
                    Segment::Circle { center, radius } => (
                        2.,
                        plane.world(center),
                        plane.normal().map(|v| if reverse { -v } else { v }),
                        plane.u,
                        radius,
                    ),
                };
                records.push(kind);
                records.extend(a);
                records.extend(b);
                records.extend(c);
                records.push(r);
            }
            ends.push(records.len() / 11);
        }
        let data = [plane.origin, plane.normal(), plane.u].concat();
        // SAFETY: registros fixos de 11 doubles e índices de término validados pela ponte.
        Self::from_pointer(unsafe {
            nator_profile(
                records.as_ptr(),
                records.len() / 11,
                ends.as_ptr(),
                ends.len(),
                data.as_ptr(),
            )
        })
    }
    fn from_pointer(pointer: *mut c_void) -> Result<Self, KernelError> {
        Ok(Self {
            pointer: NonNull::new(pointer).ok_or_else(last_error)?,
            _thread: PhantomData,
        })
    }

    pub fn cuboid(origin: Point3, size: Point3) -> Result<Self, KernelError> {
        coordinates(origin)?;
        for value in size {
            positive(value, "Dimensão")?;
        }
        // SAFETY: argumentos finitos/positivos; a ponte entrega propriedade exclusiva ou null.
        Self::from_pointer(unsafe {
            nator_box(origin[0], origin[1], origin[2], size[0], size[1], size[2])
        })
    }

    pub fn cylinder(origin: Point3, radius: f64, height: f64) -> Result<Self, KernelError> {
        coordinates(origin)?;
        positive(radius, "Raio")?;
        positive(height, "Altura")?;
        // SAFETY: mesmos limites de propriedade de cuboid, com eixo Z fixo neste protótipo.
        Self::from_pointer(unsafe {
            nator_cylinder(origin[0], origin[1], origin[2], radius, height)
        })
    }

    pub fn cut(&self, tool: &Self) -> Result<Self, KernelError> {
        // SAFETY: ambos os ponteiros pertencem a sólidos vivos nesta thread. O resultado é novo.
        Self::from_pointer(unsafe { nator_cut(self.pointer.as_ptr(), tool.pointer.as_ptr()) })
    }

    pub fn fillet(&self, edge: usize, radius: f64) -> Result<Self, KernelError> {
        positive(radius, "Raio")?;
        // SAFETY: o índice da aresta é validado na ponte; falhas são convertidas em erro.
        Self::from_pointer(unsafe { nator_fillet(self.pointer.as_ptr(), edge, radius) })
    }

    pub fn chamfer(&self, edge: usize, distance: f64) -> Result<Self, KernelError> {
        positive(distance, "Distância")?;
        // SAFETY: mesmo contrato de fillet; não modifica o sólido original.
        Self::from_pointer(unsafe { nator_chamfer(self.pointer.as_ptr(), edge, distance) })
    }

    pub fn volume(&self) -> Result<f64, KernelError> {
        let mut volume = 0.;
        // SAFETY: destino exclusivo e válido durante toda a chamada síncrona.
        if !unsafe { nator_volume(self.pointer.as_ptr(), &mut volume) } {
            return Err(last_error());
        }
        if !volume.is_finite() || volume <= 0. {
            return Err(KernelError("O resultado não tem volume positivo.".into()));
        }
        Ok(volume)
    }

    pub fn read_step(path: &Path) -> Result<Self, KernelError> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| KernelError(format!("Não foi possível ler o STEP: {e}")))?;
        if !metadata.is_file() || metadata.len() > 256 * 1024 * 1024 {
            return Err(KernelError(
                "STEP deve ser um arquivo de até 256 MiB neste protótipo.".into(),
            ));
        }
        let path = path_string(path)?;
        // SAFETY: CString vive durante a chamada; o resultado possui propriedade exclusiva.
        Self::from_pointer(unsafe { nator_read_step(path.as_ptr()) })
    }

    pub fn write_step(&self, path: &Path) -> Result<(), KernelError> {
        let path = path_string(path)?;
        // SAFETY: sólido e caminho válidos durante a chamada; não há retenção do caminho.
        if unsafe { nator_write_step(self.pointer.as_ptr(), path.as_ptr()) } {
            Ok(())
        } else {
            Err(last_error())
        }
    }

    pub fn tessellate(&mut self, linear_mm: f64, angular_rad: f64) -> Result<Mesh, KernelError> {
        positive(linear_mm, "Tolerância linear")?;
        positive(angular_rad, "Tolerância angular")?;
        if angular_rad > std::f64::consts::PI {
            return Err(KernelError(
                "Tolerância angular deve ser no máximo π radianos.".into(),
            ));
        }
        // SAFETY: acesso exclusivo impede modificar a malha durante a cópia subsequente.
        if !unsafe { nator_tessellate(self.pointer.as_ptr(), linear_mm, angular_rad) } {
            return Err(last_error());
        }
        // SAFETY: o objeto continua vivo e sem outros acessos nesta thread.
        let length = unsafe { nator_mesh_size(self.pointer.as_ptr()) };
        if length == 0 || !length.is_multiple_of(9) || length > 18_000_000 {
            return Err(KernelError(
                "Tamanho da malha nativa inválido ou excessivo.".into(),
            ));
        }
        let mut positions = vec![0.; length];
        // SAFETY: buffer alocado com o comprimento informado pela ponte, que também o valida.
        if !unsafe {
            nator_mesh_copy(
                self.pointer.as_ptr(),
                positions.as_mut_ptr(),
                positions.len(),
            )
        } {
            return Err(last_error());
        }
        let vertices = positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2]])
            .collect();
        let triangles = (0..(length / 9) as u32)
            .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
            .collect();
        Mesh::new(vertices, triangles).map_err(|e| KernelError(e.to_string()))
    }
}

impl Drop for Solid {
    fn drop(&mut self) {
        // SAFETY: é o único proprietário; release é chamado uma única vez e não lança exceções.
        unsafe {
            nator_release(self.pointer.as_ptr());
        }
    }
}
