//! Imagens de referência no plano de um rascunho. Pixels são compartilhados pelo
//! histórico; posição/tamanho usam milímetros e transparência varia de zero a um.
use super::*;
pub const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;
#[derive(Debug, Clone)]
pub struct RasterAsset {
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<Vec<u8>>,
}
impl RasterAsset {
    pub fn validate(&self) -> Result<(), DocumentError> {
        if self.width == 0 || self.height == 0 || self.width > 8192 || self.height > 8192 {
            return Err(DocumentError("Dimensões da imagem inválidas.".into()));
        }
        let bytes = u64::from(self.width) * u64::from(self.height) * 4;
        if self.width == 0
            || self.height == 0
            || self.width > 8192
            || self.height > 8192
            || bytes > MAX_IMAGE_BYTES as u64
            || bytes != self.rgba.len() as u64
        {
            return Err(DocumentError(
                "Imagem inválida: use até 8192 pixels por lado e 64 MiB de pixels RGBA.".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct ReferenceImage {
    pub id: ImageId,
    pub sketch: SketchId,
    pub name: String,
    pub asset: Arc<RasterAsset>,
    pub origin: nator_sketch::Point2,
    pub size: nator_sketch::Point2,
    pub transparency: f64,
    pub visible: bool,
}
impl ReferenceImage {
    pub fn validate(&self) -> Result<(), DocumentError> {
        validate_name(&self.name)?;
        self.asset.validate()?;
        if self
            .origin
            .iter()
            .chain(&self.size)
            .any(|v| !v.is_finite() || v.abs() > 1e9)
            || self.size.iter().any(|v| *v < 1e-6)
            || !self.transparency.is_finite()
            || !(0. ..=1.).contains(&self.transparency)
        {
            return Err(DocumentError(
                "Posição, tamanho ou transparência da imagem inválidos.".into(),
            ));
        }
        Ok(())
    }
}
impl Document {
    pub fn images(&self) -> &[ReferenceImage] {
        &self.images
    }
    pub fn image(&self, id: ImageId) -> Option<&ReferenceImage> {
        self.images.iter().find(|i| i.id == id)
    }
    pub fn add_image(&mut self, mut image: ReferenceImage) -> Result<ImageId, DocumentError> {
        image.validate()?;
        if self.sketch(image.sketch).is_none() {
            return Err(DocumentError("Rascunho da imagem inexistente.".into()));
        }
        if self.images.len() >= 256
            || self
                .images
                .iter()
                .map(|i| i.asset.rgba.len() as u64)
                .sum::<u64>()
                + image.asset.rgba.len() as u64
                > 128 * 1024 * 1024
        {
            return Err(DocumentError(
                "Limite de imagens de referência atingido.".into(),
            ));
        }
        let number = self.record_creation(ElementKind::Image)?;
        if image.name == "Imagem" {
            image.name = format!("Imagem {number}");
        }
        let id = ImageId(self.allocate_id()?);
        image.id = id;
        self.images.push(image);
        Ok(id)
    }
    pub fn update_image(&mut self, image: ReferenceImage) -> Result<(), DocumentError> {
        image.validate()?;
        let previous = self
            .images
            .iter_mut()
            .find(|i| i.id == image.id)
            .ok_or_else(|| DocumentError("Imagem inexistente.".into()))?;
        // Editar propriedades não pode trocar a origem, os pixels ou a identidade.
        if image.sketch != previous.sketch || !Arc::ptr_eq(&image.asset, &previous.asset) {
            return Err(DocumentError(
                "A edição deve preservar o rascunho e os pixels da imagem.".into(),
            ));
        }
        *previous = image;
        Ok(())
    }
    pub fn remove_image(&mut self, id: ImageId) -> Result<(), DocumentError> {
        if self.image(id).is_none() {
            return Err(DocumentError("Imagem inexistente.".into()));
        }
        self.images.retain(|i| i.id != id);
        Ok(())
    }
    pub fn restore_images(&mut self, images: Vec<ReferenceImage>) -> Result<(), DocumentError> {
        let previous = std::mem::replace(&mut self.images, images);
        if let Err(error) = self.validate_images() {
            self.images = previous;
            return Err(error);
        }
        Ok(())
    }
    pub(super) fn validate_images(&self) -> Result<(), DocumentError> {
        let mut ids = self
            .parts
            .iter()
            .flat_map(|p| {
                std::iter::once(p.id.0)
                    .chain(p.bodies.iter().map(|b| b.id.0))
                    .chain(p.sketches.iter().map(|s| s.id.0))
                    .chain(p.work_planes.iter().map(|w| w.id.0))
            })
            .chain(self.instances.iter().map(|i| i.id.0))
            .chain(self.groups.iter().map(|g| g.id.0))
            .chain(self.sheets.iter().map(|s| s.id.0))
            .chain(self.reference_sketches.iter().map(|s| s.id.0))
            .collect::<HashSet<_>>();
        if self.images.len() > 256
            || self
                .images
                .iter()
                .map(|i| i.asset.rgba.len() as u64)
                .sum::<u64>()
                > 128 * 1024 * 1024
        {
            return Err(DocumentError(
                "Limite de imagens de referência atingido.".into(),
            ));
        }
        for image in &self.images {
            image.validate()?;
            if image.id.0 == 0
                || image.id.0 >= self.next_id
                || !ids.insert(image.id.0)
                || self.sketch(image.sketch).is_none()
            {
                return Err(DocumentError(
                    "Identificador ou rascunho de imagem inválido.".into(),
                ));
            }
        }
        Ok(())
    }
}
