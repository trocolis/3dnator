//! Capturas reais do renderizador, com leitura de pixels pelo wgpu.
use nator_geometry::reference_bracket;
use nator_renderer::{Viewport, wgpu};
use std::{
    error::Error,
    fs::OpenOptions,
    io::BufWriter,
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};

pub(crate) struct Capture {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
impl Capture {
    pub(crate) fn surface_pixels(&self) -> usize {
        self.pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 60 && p[1] > 60 && p[2] > 60)
            .count()
    }
    pub(crate) fn write_png(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&self.pixels)?;
        writer.finish()?;
        Ok(())
    }
}

pub(crate) fn read_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
) -> Result<Capture, Box<dyn Error>> {
    let width = texture.width();
    let height = texture.height();
    let row_bytes = (width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Captura do protótipo"),
        size: u64::from(row_bytes) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let (sender, receiver) = mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    })?;
    receiver.recv_timeout(Duration::from_secs(10))??;
    let mapped = buffer.slice(..).get_mapped_range()?;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in mapped.chunks_exact(row_bytes as usize) {
        pixels.extend_from_slice(&row[..(width * 4) as usize]);
    }
    drop(mapped);
    buffer.unmap();
    Ok(Capture {
        pixels,
        width,
        height,
    })
}

pub fn render(path: &Path) -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let info = adapter.get_info();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut viewport = Viewport::new(&device, &reference_bracket()?)?;
    viewport.resize(&device, [960, 640]);
    viewport.show_triangles = true;
    viewport.render(&device, &queue);
    viewport.show_triangles = false;
    viewport.render(&device, &queue);
    let capture = read_texture(&device, &queue, viewport.color_texture())?;
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(format!("Falha de validação wgpu: {error}").into());
    }
    let surface_pixels = capture.surface_pixels();
    if surface_pixels < 1000 {
        return Err("A captura não contém uma superfície visível suficiente.".into());
    }
    capture.write_png(path)?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"preview":path,"adapter":info.name,"backend":format!("{:?}",info.backend),"surface_pixels":surface_pixels,"elapsed_ms":start.elapsed().as_secs_f64()*1000.})
        )?
    );
    Ok(())
}
