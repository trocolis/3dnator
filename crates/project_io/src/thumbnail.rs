//! Miniatura do projeto: vista isométrica sombreada da cena visível, desenhada na CPU com
//! z-buffer, sem depender de GPU nem de janela. Serve para reconhecer o arquivo (por exemplo,
//! nas cópias de recuperação); não participa da geometria nem é lida de volta como modelo.
use super::ImportError;
use nator_document::Document;
use nator_geometry::{Point3, cross, dot, norm, sub};

/// Entrada da miniatura no pacote.
pub const PATH: &str = "thumbnail.png";
/// Limite de tamanho aceito na leitura.
pub const MAX_BYTES: u64 = 4 * 1024 * 1024;
const WIDTH: usize = 256;
const HEIGHT: usize = 192;
/// Amostras por pixel em cada eixo (suavização das bordas).
const SAMPLES: usize = 2;
const BACKGROUND: [f32; 3] = [233., 238., 245.];

fn normalize(v: Point3) -> Point3 {
    let length = norm(v).max(1e-12);
    v.map(|x| x / length)
}

/// PNG da cena visível, ou `None` quando não há corpos visíveis.
pub fn render(document: &Document) -> Result<Option<Vec<u8>>, ImportError> {
    let Some(scene) = document.scene().map_err(|e| ImportError(e.to_string()))? else {
        return Ok(None);
    };
    let mesh = &scene.mesh;
    // Vista isométrica de frente, pela direita e de cima, como a câmera padrão do editor.
    let toward = normalize([0.55, -1., 0.75]);
    let right = normalize(cross(toward, [0., 0., 1.]).map(|v| -v));
    let up = cross(right, toward).map(|v| -v);
    let up = normalize(up);
    let center = mesh.bounds().center();
    let view = |p: Point3| {
        let d = sub(p, center);
        [dot(d, right), dot(d, up), dot(d, toward)]
    };
    let projected = mesh.vertices().iter().map(|p| view(*p)).collect::<Vec<_>>();
    let (mut min, mut max) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for p in &projected {
        for k in 0..2 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    let (w, h) = (WIDTH * SAMPLES, HEIGHT * SAMPLES);
    let extent = [(max[0] - min[0]).max(1e-9), (max[1] - min[1]).max(1e-9)];
    let scale = (0.88 * w as f64 / extent[0]).min(0.88 * h as f64 / extent[1]);
    let middle = [(min[0] + max[0]) / 2., (min[1] + max[1]) / 2.];
    let screen = |p: [f64; 3]| {
        [
            w as f64 / 2. + (p[0] - middle[0]) * scale,
            h as f64 / 2. - (p[1] - middle[1]) * scale,
            p[2],
        ]
    };
    let light = normalize([0.4, -0.6, 1.]);
    let mut depth = vec![f64::NEG_INFINITY; w * h];
    let mut color = vec![BACKGROUND; w * h];
    for (index, triangle) in mesh.triangles().iter().enumerate() {
        let world = mesh.triangle_points(*triangle);
        let normal = normalize(cross(sub(world[1], world[0]), sub(world[2], world[0])));
        let intensity = (0.38 + 0.62 * dot(normal, light).abs()) as f32;
        let base = scene
            .colors
            .get(index)
            .map(|c| c.channels())
            .unwrap_or([180, 186, 194]);
        let shade = base.map(|c| f32::from(c) * intensity);
        let p = triangle.map(|i| screen(projected[i as usize]));
        let area =
            (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
        if area.abs() < 1e-12 {
            continue;
        }
        let x0 = p
            .iter()
            .map(|q| q[0])
            .fold(f64::INFINITY, f64::min)
            .floor()
            .max(0.) as usize;
        let x1 = p
            .iter()
            .map(|q| q[0])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            .min(w as f64 - 1.) as usize;
        let y0 = p
            .iter()
            .map(|q| q[1])
            .fold(f64::INFINITY, f64::min)
            .floor()
            .max(0.) as usize;
        let y1 = p
            .iter()
            .map(|q| q[1])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            .min(h as f64 - 1.) as usize;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let edge = |a: [f64; 3], b: [f64; 3]| {
                    (b[0] - a[0]) * (py - a[1]) - (b[1] - a[1]) * (px - a[0])
                };
                let w0 = edge(p[1], p[2]) / area;
                let w1 = edge(p[2], p[0]) / area;
                let w2 = edge(p[0], p[1]) / area;
                if w0 < 0. || w1 < 0. || w2 < 0. {
                    continue;
                }
                let z = w0 * p[0][2] + w1 * p[1][2] + w2 * p[2][2];
                let pixel = y * w + x;
                if z > depth[pixel] {
                    depth[pixel] = z;
                    color[pixel] = shade;
                }
            }
        }
    }
    // Contorno: pixels com salto de profundidade ou na borda com o fundo escurecem.
    let jump = 0.02 * extent[0].max(extent[1]);
    let mut outlined = color.clone();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let pixel = y * w + x;
            let here = depth[pixel];
            if here == f64::NEG_INFINITY {
                continue;
            }
            let edge = [pixel - 1, pixel + 1, pixel - w, pixel + w]
                .iter()
                .any(|n| depth[*n] == f64::NEG_INFINITY || (depth[*n] - here).abs() > jump);
            if edge {
                outlined[pixel] = color[pixel].map(|c| c * 0.35);
            }
        }
    }
    // Média das amostras de cada pixel final.
    let mut rgb = Vec::with_capacity(WIDTH * HEIGHT * 3);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let mut sum = [0_f32; 3];
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let c = outlined[(y * SAMPLES + sy) * w + x * SAMPLES + sx];
                    for k in 0..3 {
                        sum[k] += c[k];
                    }
                }
            }
            let n = (SAMPLES * SAMPLES) as f32;
            rgb.extend(sum.map(|v| (v / n).round().clamp(0., 255.) as u8));
        }
    }
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, WIDTH as u32, HEIGHT as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| ImportError(e.to_string()))?;
        writer
            .write_image_data(&rgb)
            .map_err(|e| ImportError(e.to_string()))?;
    }
    Ok(Some(output))
}

/// Confere a assinatura PNG de uma miniatura lida do pacote.
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
}
