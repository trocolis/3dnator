//! Único módulo Rust autorizado a chamar o ABI nativo.
//! Cada sólido tem propriedade exclusiva e permanece na thread que o criou.
//!
//! Além do ponteiro nativo, `Solid` carrega os nomes estáveis de suas faces (ver
//! `naming`). Toda operação produz os nomes do resultado a partir do histórico registrado
//! pela ponte, de modo que referências do histórico paramétrico sobrevivam a mudanças de cota.

use super::{EdgeMeasure, FaceKind, FaceMeasure, KernelError, MassProperties};
use crate::{
    hole::{self, HoleGeometry},
    naming::{self, Inheritance, Origin},
};
use nator_geometry::{Mesh, Placement, Point3};
use nator_sketch::{Sketch, profiles::Region};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    marker::PhantomData,
    path::Path,
    ptr::NonNull,
    rc::Rc,
};
mod step;
pub use step::{StepAssembly, StepBody, StepOccurrence, StepPart};

// O intercâmbio STEP usa configurações globais do OCCT (Interface_Static e XCAF).
// Serializar leitores/escritores evita que transferências de projetos em threads
// distintas troquem nomes ou configurações, sem bloquear a thread da interface.
static STEP_INTERCHANGE: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn step_guard() -> Result<std::sync::MutexGuard<'static, ()>, KernelError> {
    STEP_INTERCHANGE.lock().map_err(|_| {
        KernelError("O intercâmbio STEP foi interrompido por uma falha anterior.".into())
    })
}

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
    fn nator_shell(
        body: *const c_void,
        faces: *const usize,
        count: usize,
        thickness: f64,
        outward: bool,
    ) -> *mut c_void;
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
    fn nator_fillet(
        body: *const c_void,
        edges: *const usize,
        count: usize,
        radius: f64,
    ) -> *mut c_void;
    fn nator_chamfer(
        body: *const c_void,
        edges: *const usize,
        count: usize,
        distance: f64,
        angle: f64,
    ) -> *mut c_void;
    fn nator_volume(body: *const c_void, volume: *mut f64) -> bool;
    fn nator_read_step(path: *const c_char) -> *mut c_void;
    fn nator_write_step(body: *const c_void, path: *const c_char) -> bool;
    fn nator_tessellate(body: *mut c_void, linear: f64, angular: f64) -> bool;
    fn nator_mesh_size(body: *const c_void) -> usize;
    fn nator_mesh_copy(body: *const c_void, output: *mut f64, length: usize) -> bool;
    fn nator_mesh_faces_copy(body: *const c_void, output: *mut i32, length: usize) -> bool;
    fn nator_face_count(body: *const c_void) -> usize;
    fn nator_edge_count(body: *const c_void) -> usize;
    fn nator_edge_faces(body: *const c_void, output: *mut i32, length: usize) -> bool;
    fn nator_origins_size(body: *const c_void) -> usize;
    fn nator_origins_copy(body: *const c_void, output: *mut i32, length: usize) -> bool;
    fn nator_mirror(body: *const c_void, origin: *const f64, normal: *const f64) -> *mut c_void;
    fn nator_properties(body: *const c_void, output: *mut f64) -> bool;
    fn nator_face_measure(body: *const c_void, index: usize, output: *mut f64) -> bool;
    fn nator_edge_measure(body: *const c_void, index: usize, output: *mut f64) -> bool;
    fn nator_distance(a: *const c_void, b: *const c_void, output: *mut f64) -> bool;
    fn nator_half_space(body: *const c_void, origin: *const f64, normal: *const f64)
    -> *mut c_void;
}

/// Limite defensivo de faces/arestas copiadas da ponte (um modelo de protótipo tem centenas).
const MAX_TOPOLOGY: usize = 1_000_000;

pub struct Solid {
    pointer: NonNull<c_void>,
    /// Nome estável de cada face, na ordem de `TopExp::MapShapes`. Vazio = sem nome.
    faces: Vec<String>,
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

/// Vetor finito e não nulo, normalizado.
fn unit(vector: Point3, name: &str) -> Result<Point3, KernelError> {
    let length = nator_geometry::norm(vector);
    if vector.iter().all(|v| v.is_finite()) && length > 1e-9 {
        Ok(vector.map(|v| v / length))
    } else {
        Err(KernelError(format!(
            "{name} deve ser um vetor finito e não nulo."
        )))
    }
}

fn sketch_error(error: nator_sketch::SketchError) -> KernelError {
    KernelError(error.to_string())
}

fn owner_name(owner: &str) -> Result<(), KernelError> {
    if owner.is_empty() || owner.contains(':') || owner.len() > 64 {
        return Err(KernelError(
            "Identificador de dono das faces inválido.".into(),
        ));
    }
    Ok(())
}

pub fn version() -> String {
    // SAFETY: texto estático da versão do SDK vinculado.
    unsafe { CStr::from_ptr(nator_version()) }
        .to_string_lossy()
        .into_owned()
}

/// Perfil plano já convertido para a ponte, com a entidade de origem de cada segmento.
struct Profile {
    face: Solid,
    entities: Vec<u64>,
}

impl Solid {
    // ----- Construção a partir de dados persistidos ---------------------------------------

    /// Os dados B-rep imutáveis podem atravessar threads; objetos OCCT vivos não podem.
    /// `names` deve ter um nome por face; sem nomes, as faces ficam não referenciáveis.
    pub fn from_brep(bytes: &[u8], names: Option<&[String]>) -> Result<Self, KernelError> {
        if bytes.is_empty() || bytes.len() > 64 * 1024 * 1024 {
            return Err(KernelError("B-rep vazio ou excessivo.".into()));
        }
        // SAFETY: fatia vive durante a leitura síncrona; a ponte valida e copia o conteúdo.
        let mut solid =
            Self::from_pointer(unsafe { nator_brep_read(bytes.as_ptr().cast(), bytes.len()) })?;
        let count = solid.face_count()?;
        solid.faces = match names {
            Some(names) if names.len() == count => names.to_vec(),
            Some(_) => {
                return Err(KernelError(
                    "Os nomes de faces não correspondem ao B-rep armazenado.".into(),
                ));
            }
            None => vec![String::new(); count],
        };
        Ok(solid)
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

    /// Dá nomes posicionais às faces de geometria que não será recalculada.
    pub fn name_faces_by_index(&mut self, owner: &str) -> Result<(), KernelError> {
        owner_name(owner)?;
        self.faces = naming::indexed_names(owner, self.face_count()?);
        Ok(())
    }

    // ----- Topologia e nomes --------------------------------------------------------------

    pub fn face_names(&self) -> &[String] {
        &self.faces
    }

    pub fn face_count(&self) -> Result<usize, KernelError> {
        // SAFETY: objeto vivo nesta thread; a ponte apenas consulta a topologia.
        let count = unsafe { nator_face_count(self.pointer.as_ptr()) };
        if count > MAX_TOPOLOGY {
            return Err(KernelError("Sólido com faces demais.".into()));
        }
        Ok(count)
    }

    /// Para cada aresta, as faces adjacentes (base 0; -1 quando ausente).
    fn adjacency(&self) -> Result<Vec<[i32; 2]>, KernelError> {
        // SAFETY: objeto vivo nesta thread; apenas consulta.
        let count = unsafe { nator_edge_count(self.pointer.as_ptr()) };
        if count > MAX_TOPOLOGY {
            return Err(KernelError("Sólido com arestas demais.".into()));
        }
        let mut flat = vec![0_i32; count * 2];
        // SAFETY: buffer exclusivo com o comprimento exato verificado pela ponte.
        if !unsafe { nator_edge_faces(self.pointer.as_ptr(), flat.as_mut_ptr(), flat.len()) } {
            return Err(last_error());
        }
        Ok(flat.as_chunks::<2>().0.to_vec())
    }

    /// Origens das faces registradas pela última operação da ponte.
    fn origins(&self) -> Result<Vec<Origin>, KernelError> {
        // SAFETY: objeto vivo nesta thread; apenas consulta.
        let length = unsafe { nator_origins_size(self.pointer.as_ptr()) };
        if !length.is_multiple_of(3) || length / 3 != self.face_count()? {
            return Err(KernelError(
                "Histórico de faces da operação ausente ou incoerente.".into(),
            ));
        }
        let mut flat = vec![0_i32; length];
        // SAFETY: buffer exclusivo com o comprimento informado pela própria ponte.
        if !unsafe { nator_origins_copy(self.pointer.as_ptr(), flat.as_mut_ptr(), flat.len()) } {
            return Err(last_error());
        }
        Ok(flat.as_chunks::<3>().0.to_vec())
    }

    /// Índice da única aresta entre duas faces nomeadas.
    pub fn edge_between(&self, faces: &[String; 2]) -> Result<usize, KernelError> {
        naming::edge_between(&self.faces, &self.adjacency()?, faces)
    }

    /// Nomes das faces adjacentes a uma aresta, para criar uma referência estável.
    pub fn edge_face_names(&self, edge: usize) -> Result<[String; 2], KernelError> {
        naming::edge_face_names(&self.faces, &self.adjacency()?, edge)
    }

    /// Índice da única face com o nome dado.
    pub fn face_named(&self, face: &str) -> Result<usize, KernelError> {
        naming::face_named(&self.faces, face)
    }

    /// Nome estável da face `index`, quando houver.
    pub fn face_name(&self, index: usize) -> Result<&str, KernelError> {
        self.faces
            .get(index)
            .filter(|n| !n.is_empty())
            .map(String::as_str)
            .ok_or_else(|| KernelError("A face não possui nome estável.".into()))
    }

    /// Renomeia as faces nomeadas, por exemplo para distinguir cópias de um padrão.
    pub fn map_face_names(&mut self, rename: impl Fn(&str) -> String) {
        for name in self.faces.iter_mut().filter(|n| !n.is_empty()) {
            *name = rename(name);
        }
    }

    fn with_inherited(
        mut self,
        inputs: &[&[String]],
        rule: &Inheritance<'_>,
    ) -> Result<Self, KernelError> {
        self.faces = naming::inherited_names(&self.origins()?, inputs, rule);
        Ok(self)
    }

    // ----- Operações paramétricas ----------------------------------------------------------

    pub fn boolean(&self, other: &Self, mode: u32, owner: &str) -> Result<Self, KernelError> {
        owner_name(owner)?;
        if mode > 2 {
            return Err(KernelError("Operação booleana inválida.".into()));
        }
        // SAFETY: ambos os sólidos estão vivos; a operação é não destrutiva.
        Self::from_pointer(unsafe {
            nator_boolean(self.pointer.as_ptr(), other.pointer.as_ptr(), mode as i32)
        })?
        .with_inherited(
            &[&self.faces, &other.faces],
            &Inheritance {
                owner,
                generated: "section",
                modified_suffix: None,
            },
        )
    }

    /// Transformação rígida; as faces mantêm seus nomes.
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
        let moved =
            Self::from_pointer(unsafe { nator_transform(self.pointer.as_ptr(), data.as_ptr()) })?;
        // A cópia transformada tem a mesma estrutura; o histórico confirma a correspondência.
        moved.with_inherited(
            &[&self.faces],
            &Inheritance {
                owner: "transform",
                generated: "transform",
                modified_suffix: None,
            },
        )
    }

    /// Cópia espelhada pelo plano (origem, normal); as faces mantêm seus nomes.
    pub fn mirrored(&self, origin: Point3, normal: Point3) -> Result<Self, KernelError> {
        coordinates(origin)?;
        let normal = unit(normal, "A normal do plano de espelhamento")?;
        // SAFETY: buffers de três coordenadas vivos durante a chamada; resultado novo.
        Self::from_pointer(unsafe {
            nator_mirror(self.pointer.as_ptr(), origin.as_ptr(), normal.as_ptr())
        })?
        .with_inherited(
            &[&self.faces],
            &Inheritance {
                owner: "mirror",
                generated: "mirror",
                modified_suffix: None,
            },
        )
    }

    /// Parte do corpo do lado para onde `normal` aponta. A face criada no plano de corte
    /// recebe o nome `{dono}:split`.
    pub fn split(&self, origin: Point3, normal: Point3, owner: &str) -> Result<Self, KernelError> {
        owner_name(owner)?;
        coordinates(origin)?;
        let normal = unit(normal, "A normal do plano de divisão")?;
        // SAFETY: corpo vivo e buffers fixos durante a chamada; o bloco é um sólido novo.
        let mut half = Self::from_pointer(unsafe {
            nator_half_space(self.pointer.as_ptr(), origin.as_ptr(), normal.as_ptr())
        })?;
        half.faces = vec![format!("{owner}:split"); half.face_count()?];
        let part = self.boolean(&half, 2, owner)?;
        part.volume()
            .map_err(|_| KernelError("O plano não deixa material do lado escolhido.".into()))?;
        Ok(part)
    }

    /// Ferramenta de furo com eixo em `center` e furação na direção `direction`. As faces se
    /// chamam `{dono}:hole{índice}:{papel}` (parede, rebaixo, escareado, ponta...).
    pub fn hole_tool(
        center: Point3,
        direction: Point3,
        geometry: &HoleGeometry,
        owner: &str,
        index: usize,
    ) -> Result<Self, KernelError> {
        use nator_sketch::{Curve, Plane};
        owner_name(owner)?;
        coordinates(center)?;
        let axis = unit(direction, "A direção do furo")?;
        // Plano do meio-corte: U radial (qualquer perpendicular ao eixo) e V ao longo do eixo.
        let helper = if axis[0].abs() < 0.9 {
            [1., 0., 0.]
        } else {
            [0., 1., 0.]
        };
        let radial = unit(nator_geometry::cross(axis, helper), "O eixo radial do furo")?;
        let plane = Plane {
            origin: center,
            u: radial,
            v: axis,
        };
        let points = hole::profile(geometry)?;
        let mut sketch = Sketch::new("Furo", plane);
        let ids = points
            .iter()
            .map(|(p, _)| sketch.add_point(*p))
            .collect::<Result<Vec<_>, _>>()
            .map_err(sketch_error)?;
        let mut roles = std::collections::HashMap::new();
        for i in 0..ids.len() {
            let line = sketch
                .add_curve(
                    Curve::Line {
                        start: ids[i],
                        end: ids[(i + 1) % ids.len()],
                    },
                    false,
                )
                .map_err(sketch_error)?;
            roles.insert(format!("{owner}:e{line}"), points[i].1);
        }
        let region = nator_sketch::profiles::regions(&sketch)
            .map_err(sketch_error)?
            .into_iter()
            .next()
            .ok_or_else(|| KernelError("Perfil de furo sem região.".into()))?;
        let mut tool = Self::revolve(&sketch, &region, center, axis, std::f64::consts::TAU, owner)?;
        tool.map_face_names(|name| match roles.get(name) {
            Some(role) => format!("{owner}:hole{index}:{role}"),
            None => name.to_owned(),
        });
        Ok(tool)
    }

    /// União do corpo com cópias posicionadas por `placements`. A cópia k recebe nomes com
    /// sufixo `:p{k}` (o original, k = 0, mantém os seus), para que cada cópia possa ser
    /// referenciada sem ambiguidade.
    pub fn pattern(&self, placements: &[Placement], owner: &str) -> Result<Self, KernelError> {
        owner_name(owner)?;
        if placements.is_empty() || placements.len() > 1000 {
            return Err(KernelError(
                "O padrão deve ter entre 2 e 1000 ocorrências.".into(),
            ));
        }
        let mut result: Option<Self> = None;
        for (k, placement) in placements.iter().enumerate() {
            let mut copy = self.transformed(*placement)?;
            if k > 0 {
                copy.map_face_names(|name| format!("{name}:p{k}"));
            }
            result = Some(match result {
                None => copy,
                Some(previous) => previous.boolean(&copy, 0, owner)?,
            });
        }
        result.ok_or_else(|| KernelError("Padrão vazio.".into()))
    }

    /// Casca para dentro, removendo a face `face`.
    /// Casca com as faces `faces` removidas e paredes de `thickness` mm, para dentro ou para
    /// fora do sólido. As faces deslocadas recebem o sufixo `:inner` ou `:outer`.
    pub fn shell(
        &self,
        faces: &[usize],
        thickness: f64,
        outward: bool,
        owner: &str,
    ) -> Result<Self, KernelError> {
        owner_name(owner)?;
        positive(thickness, "Espessura")?;
        if faces.len() > 1000 {
            return Err(KernelError("Faces demais na casca.".into()));
        }
        // SAFETY: lista de índices viva durante a chamada; a ponte valida cada índice.
        Self::from_pointer(unsafe {
            nator_shell(
                self.pointer.as_ptr(),
                faces.as_ptr(),
                faces.len(),
                thickness,
                outward,
            )
        })?
        .with_inherited(
            &[&self.faces],
            &Inheritance {
                owner,
                generated: "rim",
                modified_suffix: Some(if outward { "outer" } else { "inner" }),
            },
        )
    }

    /// Arredondamento de raio constante nas arestas indicadas (com propagação tangente).
    pub fn fillet(&self, edges: &[usize], radius: f64, owner: &str) -> Result<Self, KernelError> {
        owner_name(owner)?;
        positive(radius, "Raio")?;
        // SAFETY: lista de índices viva durante a chamada; a ponte valida cada índice e a
        // quantidade, e falhas viram erro sem alterar o sólido original.
        Self::from_pointer(unsafe {
            nator_fillet(self.pointer.as_ptr(), edges.as_ptr(), edges.len(), radius)
        })?
        .with_inherited(
            &[&self.faces],
            &Inheritance {
                owner,
                generated: "round",
                modified_suffix: None,
            },
        )
    }

    /// Chanfro simétrico, ou por distância e ângulo (rad) quando `angle` é informado.
    pub fn chamfer(
        &self,
        edges: &[usize],
        distance: f64,
        angle: Option<f64>,
        owner: &str,
    ) -> Result<Self, KernelError> {
        owner_name(owner)?;
        positive(distance, "Distância")?;
        if let Some(angle) = angle
            && !(angle.is_finite() && angle > 0.01 && angle < std::f64::consts::FRAC_PI_2 - 0.01)
        {
            return Err(KernelError(
                "O ângulo do chanfro deve ficar entre 0° e 90°.".into(),
            ));
        }
        // SAFETY: mesmo contrato de fillet; não modifica o sólido original.
        Self::from_pointer(unsafe {
            nator_chamfer(
                self.pointer.as_ptr(),
                edges.as_ptr(),
                edges.len(),
                distance,
                angle.unwrap_or(0.),
            )
        })?
        .with_inherited(
            &[&self.faces],
            &Inheritance {
                owner,
                generated: "chamfer",
                modified_suffix: None,
            },
        )
    }

    /// Extrusão de uma região entre `from` e `to` mm ao longo da normal do plano do rascunho.
    pub fn extrude(
        sketch: &Sketch,
        region: &Region,
        from: f64,
        to: f64,
        owner: &str,
    ) -> Result<Self, KernelError> {
        owner_name(owner)?;
        if !from.is_finite() || !to.is_finite() || (to - from).abs() < 1e-7 {
            return Err(KernelError("Limites de extrusão inválidos.".into()));
        }
        // O perfil é construído diretamente no plano deslocado: transformar um perfil pronto
        // copiaria suas arestas e perderia a correspondência segmento → entidade.
        let profile = Self::profile(sketch, region, sketch.plane.normal().map(|v| v * from))?;
        let direction = sketch.plane.normal().map(|v| v * (to - from));
        // SAFETY: perfil vivo e vetor finito não nulo, sem retenção dos buffers.
        let mut solid = Self::from_pointer(unsafe {
            nator_extrude(profile.face.pointer.as_ptr(), direction.as_ptr())
        })?;
        solid.faces = naming::sweep_names(owner, &solid.origins()?, &profile.entities);
        solid.volume()?;
        Ok(solid)
    }

    pub fn revolve(
        sketch: &Sketch,
        region: &Region,
        origin: Point3,
        direction: Point3,
        angle: f64,
        owner: &str,
    ) -> Result<Self, KernelError> {
        owner_name(owner)?;
        coordinates(origin)?;
        coordinates(direction)?;
        if nator_geometry::norm(direction) < 1e-9
            || !angle.is_finite()
            || angle.abs() < 1e-7
            || angle.abs() > std::f64::consts::TAU + 1e-9
        {
            return Err(KernelError("Eixo ou ângulo de revolução inválido.".into()));
        }
        let profile = Self::profile(sketch, region, [0.; 3])?;
        // SAFETY: buffers de três coordenadas vivos e perfil válido durante a chamada.
        let mut solid = Self::from_pointer(unsafe {
            nator_revolve(
                profile.face.pointer.as_ptr(),
                origin.as_ptr(),
                direction.as_ptr(),
                angle,
            )
        })?;
        solid.faces = naming::sweep_names(owner, &solid.origins()?, &profile.entities);
        solid.volume()?;
        Ok(solid)
    }

    /// Converte uma região em face plana na ponte. `offset` desloca o plano em mm.
    fn profile(sketch: &Sketch, region: &Region, offset: Point3) -> Result<Profile, KernelError> {
        use nator_sketch::profiles::Segment;
        let plane = sketch.plane;
        let world = |p| nator_geometry::add(plane.world(p), offset);
        let mut records = Vec::new();
        let mut ends = Vec::new();
        let mut entities = Vec::new();
        // O contorno externo segue o sentido anti-horário e os furos o horário, convenção
        // que o OCCT usa para orientar a face.
        let wires =
            std::iter::once((&region.outer, true)).chain(region.holes.iter().map(|h| (h, false)));
        for (wire, outer) in wires {
            let reverse = (wire.area() > 0.) != outer;
            let mut segments = wire
                .segments
                .iter()
                .cloned()
                .zip(wire.entities.iter().copied())
                .collect::<Vec<_>>();
            if reverse {
                segments.reverse();
                for (segment, _) in &mut segments {
                    *segment = segment.reversed();
                }
            }
            for (segment, entity) in segments {
                let (kind, a, b, c, r) = match segment {
                    Segment::Line { a, b } => (0., world(a), world(b), [0.; 3], 0.),
                    Segment::Arc { .. } => {
                        let points = segment.sample(4);
                        (
                            1.,
                            world(points[0]),
                            world(points[points.len() / 2]),
                            world(points[points.len() - 1]),
                            0.,
                        )
                    }
                    Segment::Circle { center, radius } => (
                        2.,
                        world(center),
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
                entities.push(entity);
            }
            ends.push(records.len() / 11);
        }
        let data = [
            nator_geometry::add(plane.origin, offset),
            plane.normal(),
            plane.u,
        ]
        .concat();
        // SAFETY: registros fixos de 11 doubles e índices de término validados pela ponte.
        let face = Self::from_pointer(unsafe {
            nator_profile(
                records.as_ptr(),
                records.len() / 11,
                ends.as_ptr(),
                ends.len(),
                data.as_ptr(),
            )
        })?;
        Ok(Profile { face, entities })
    }

    fn from_pointer(pointer: *mut c_void) -> Result<Self, KernelError> {
        Ok(Self {
            pointer: NonNull::new(pointer).ok_or_else(last_error)?,
            faces: Vec::new(),
            _thread: PhantomData,
        })
    }

    // ----- Primitivas e intercâmbio --------------------------------------------------------

    pub fn cuboid(origin: Point3, size: Point3) -> Result<Self, KernelError> {
        coordinates(origin)?;
        for value in size {
            positive(value, "Dimensão")?;
        }
        // SAFETY: argumentos finitos/positivos; a ponte entrega propriedade exclusiva ou null.
        let mut solid = Self::from_pointer(unsafe {
            nator_box(origin[0], origin[1], origin[2], size[0], size[1], size[2])
        })?;
        solid.name_faces_by_index("box")?;
        Ok(solid)
    }

    pub fn cylinder(origin: Point3, radius: f64, height: f64) -> Result<Self, KernelError> {
        coordinates(origin)?;
        positive(radius, "Raio")?;
        positive(height, "Altura")?;
        // SAFETY: mesmos limites de propriedade de cuboid, com eixo Z fixo neste protótipo.
        let mut solid = Self::from_pointer(unsafe {
            nator_cylinder(origin[0], origin[1], origin[2], radius, height)
        })?;
        solid.name_faces_by_index("cylinder")?;
        Ok(solid)
    }

    pub fn cut(&self, tool: &Self) -> Result<Self, KernelError> {
        // SAFETY: ambos os ponteiros pertencem a sólidos vivos nesta thread. O resultado é novo.
        Self::from_pointer(unsafe { nator_cut(self.pointer.as_ptr(), tool.pointer.as_ptr()) })?
            .with_inherited(
                &[&self.faces, &tool.faces],
                &Inheritance {
                    owner: "cut",
                    generated: "section",
                    modified_suffix: None,
                },
            )
    }

    /// Volume, área e centro de massa (densidade uniforme).
    pub fn properties(&self) -> Result<MassProperties, KernelError> {
        let mut out = [0.; 5];
        // SAFETY: destino de cinco doubles válido durante a chamada.
        if !unsafe { nator_properties(self.pointer.as_ptr(), out.as_mut_ptr()) } {
            return Err(last_error());
        }
        Ok(MassProperties {
            volume: out[0],
            area: out[1],
            center: [out[2], out[3], out[4]],
        })
    }

    pub fn face_measure(&self, index: usize) -> Result<FaceMeasure, KernelError> {
        let mut out = [0.; 8];
        // SAFETY: destino de oito doubles; a ponte valida o índice.
        if !unsafe { nator_face_measure(self.pointer.as_ptr(), index, out.as_mut_ptr()) } {
            return Err(last_error());
        }
        let kind = match out[1] as i32 {
            0 => FaceKind::Plane {
                normal: [out[3], out[4], out[5]],
                offset: out[6],
            },
            1 => FaceKind::Cylinder { radius: out[2] },
            2 => FaceKind::Cone,
            3 => FaceKind::Sphere { radius: out[2] },
            4 => FaceKind::Torus {
                major_radius: out[2],
            },
            _ => FaceKind::Other,
        };
        Ok(FaceMeasure { area: out[0], kind })
    }

    pub fn edge_measure(&self, index: usize) -> Result<EdgeMeasure, KernelError> {
        let mut out = [0.; 3];
        // SAFETY: destino de três doubles; a ponte valida o índice.
        if !unsafe { nator_edge_measure(self.pointer.as_ptr(), index, out.as_mut_ptr()) } {
            return Err(last_error());
        }
        Ok(EdgeMeasure {
            length: out[0],
            straight: out[1] == 0.,
            radius: (out[1] == 1.).then_some(out[2]),
        })
    }

    /// Menor distância até outro sólido e os pontos mais próximos em cada um.
    pub fn distance(&self, other: &Self) -> Result<(f64, Point3, Point3), KernelError> {
        let mut out = [0.; 7];
        // SAFETY: dois sólidos vivos nesta thread e destino de sete doubles.
        if !unsafe {
            nator_distance(
                self.pointer.as_ptr(),
                other.pointer.as_ptr(),
                out.as_mut_ptr(),
            )
        } {
            return Err(last_error());
        }
        Ok((out[0], [out[1], out[2], out[3]], [out[4], out[5], out[6]]))
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

    /// Lê a geometria de um STEP. As faces ficam sem nome até `name_faces_by_index`.
    pub fn read_step(path: &Path) -> Result<Self, KernelError> {
        let _guard = step_guard()?;
        let metadata = std::fs::metadata(path)
            .map_err(|e| KernelError(format!("Não foi possível ler o STEP: {e}")))?;
        if !metadata.is_file() || metadata.len() > 256 * 1024 * 1024 {
            return Err(KernelError(
                "STEP deve ser um arquivo de até 256 MiB neste protótipo.".into(),
            ));
        }
        let path = path_string(path)?;
        // SAFETY: CString vive durante a chamada; o resultado possui propriedade exclusiva.
        let mut solid = Self::from_pointer(unsafe { nator_read_step(path.as_ptr()) })?;
        solid.faces = vec![String::new(); solid.face_count()?];
        Ok(solid)
    }

    pub fn write_step(&self, path: &Path) -> Result<(), KernelError> {
        let _guard = step_guard()?;
        let path = path_string(path)?;
        // SAFETY: sólido e caminho válidos durante a chamada; não há retenção do caminho.
        if unsafe { nator_write_step(self.pointer.as_ptr(), path.as_ptr()) } {
            Ok(())
        } else {
            Err(last_error())
        }
    }

    pub fn tessellate(&mut self, linear_mm: f64, angular_rad: f64) -> Result<Mesh, KernelError> {
        Ok(self.tessellate_with_faces(linear_mm, angular_rad)?.0)
    }

    /// Malha de exibição e a face de origem de cada triângulo (índice base 0), usada para
    /// destacar e selecionar faces e arestas na interface.
    pub fn tessellate_with_faces(
        &mut self,
        linear_mm: f64,
        angular_rad: f64,
    ) -> Result<(Mesh, Vec<u32>), KernelError> {
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
        let mut faces = vec![0_i32; length / 9];
        // SAFETY: um índice de face por triângulo, comprimento validado pela ponte.
        if !unsafe { nator_mesh_faces_copy(self.pointer.as_ptr(), faces.as_mut_ptr(), faces.len()) }
        {
            return Err(last_error());
        }
        let faces = faces
            .into_iter()
            .map(|f| {
                u32::try_from(f).map_err(|_| KernelError("Face de triângulo inválida.".into()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let vertices = positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2]])
            .collect();
        let triangles = (0..(length / 9) as u32)
            .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
            .collect();
        // Os nós das arestas são idênticos nas faces vizinhas; fundi-los fecha a malha
        // sem alterar coordenadas. A ordem dos triângulos (e `faces`) é preservada.
        let mesh = Mesh::new(vertices, triangles)
            .map_err(|e| KernelError(e.to_string()))?
            .welded();
        Ok((mesh, faces))
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
