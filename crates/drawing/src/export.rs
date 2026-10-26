//! Exportadores autocontidos: PDF mantém traços e textos vetoriais; PNG usa a mesma folha.
use super::*;
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use std::fmt::Write as _;
fn colors(style: Style) -> ([u8; 3], [u8; 3]) {
    match style {
        Style::Conventional => ([255; 3], [20; 3]),
        Style::Blueprint => ([17, 49, 85], [225, 239, 250]),
    }
}
fn pdf_text(value: &str) -> Vec<u8> {
    value
        .chars()
        .flat_map(|c| {
            let c = if (32..=255).contains(&(c as u32)) {
                c as u8
            } else {
                b'?'
            };
            if matches!(c, b'(' | b')' | b'\\') {
                vec![b'\\', c]
            } else {
                vec![c]
            }
        })
        .collect()
}
pub fn pdf(sheet: &Sheet) -> Result<Vec<u8>> {
    let point = 72. / 25.4;
    let [w, h] = sheet.size.map(|v| v * point);
    let (bg, fg) = colors(sheet.style);
    let rgb = |c: [u8; 3]| {
        format!(
            "{:.5} {:.5} {:.5}",
            c[0] as f64 / 255.,
            c[1] as f64 / 255.,
            c[2] as f64 / 255.
        )
    };
    let mut content = format!(
        "{} rg 0 0 {w:.4} {h:.4} re f\n{} RG {} rg\n1 J 1 j\n",
        rgb(bg),
        rgb(fg),
        rgb(fg)
    )
    .into_bytes();
    for p in &sheet.primitives {
        match p {
            Primitive::Line {
                a,
                b,
                width,
                dashed,
            } => content.extend_from_slice(
                format!(
                    "{:.4} w {} 0 d {:.4} {:.4} m {:.4} {:.4} l S\n",
                    width * point,
                    if *dashed { "[4 3]" } else { "[]" },
                    a[0] * point,
                    h - a[1] * point,
                    b[0] * point,
                    h - b[1] * point
                )
                .as_bytes(),
            ),
            Primitive::Text { at, size, value } => {
                content.extend_from_slice(
                    format!(
                        "BT /F1 {:.4} Tf {:.4} {:.4} Td (",
                        size * point,
                        at[0] * point,
                        h - at[1] * point
                    )
                    .as_bytes(),
                );
                content.extend(pdf_text(value));
                content.extend_from_slice(b") Tj ET\n");
            }
        }
    }
    let objects=[b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w:.4} {h:.4}] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>").into_bytes(),b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),[format!("<< /Length {} >>\nstream\n",content.len()).into_bytes(),content,b"endstream".to_vec()].concat()];
    let mut output = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(output.len());
        output.extend(format!("{} 0 obj\n", i + 1).as_bytes());
        output.extend(object);
        output.extend(b"\nendobj\n");
    }
    let xref = output.len();
    let mut trailer = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        writeln!(&mut trailer, "{offset:010} 00000 n ").map_err(|e| DrawingError(e.to_string()))?;
    }
    write!(
        &mut trailer,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .map_err(|e| DrawingError(e.to_string()))?;
    output.extend(trailer.as_bytes());
    Ok(output)
}
fn blend(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    color: [u8; 3],
    coverage: f32,
) {
    if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
        return;
    }
    let i = (y as usize * width + x as usize) * 3;
    let a = coverage.clamp(0., 1.);
    for k in 0..3 {
        pixels[i + k] =
            (f32::from(pixels[i + k]) * (1. - a) + f32::from(color[k]) * a).round() as u8;
    }
}
pub fn png(sheet: &Sheet, dpi: u32) -> Result<Vec<u8>> {
    if !(72..=300).contains(&dpi) {
        return Err(DrawingError(
            "A resolução deve estar entre 72 e 300 DPI.".into(),
        ));
    }
    let scale = dpi as f64 / 25.4;
    let [w, h] = sheet.size.map(|v| (v * scale).round() as usize);
    if w * h > 20_000_000 {
        return Err(DrawingError(
            "Imagem excede vinte milhões de pixels.".into(),
        ));
    }
    let (bg, fg) = colors(sheet.style);
    let mut pixels = bg.repeat(w * h);
    let font = FontRef::try_from_slice(epaint_default_fonts::HACK_REGULAR)
        .map_err(|e| DrawingError(e.to_string()))?;
    for p in &sheet.primitives {
        match p {
            Primitive::Line {
                a,
                b,
                width,
                dashed,
            } => {
                let a = a.map(|v| v * scale);
                let b = b.map(|v| v * scale);
                let radius = (width * scale / 2.).max(0.3);
                let d = [b[0] - a[0], b[1] - a[1]];
                let len = d[0].hypot(d[1]);
                if len < 1e-9 {
                    continue;
                }
                let xmin = (a[0].min(b[0]) - radius - 1.).floor().max(0.) as usize;
                let xmax = (a[0].max(b[0]) + radius + 1.)
                    .ceil()
                    .min(w as f64 - 1.)
                    .max(0.) as usize;
                let ymin = (a[1].min(b[1]) - radius - 1.).floor().max(0.) as usize;
                let ymax = (a[1].max(b[1]) + radius + 1.)
                    .ceil()
                    .min(h as f64 - 1.)
                    .max(0.) as usize;
                for y in ymin..=ymax {
                    for x in xmin..=xmax {
                        let v = [x as f64 + 0.5 - a[0], y as f64 + 0.5 - a[1]];
                        let t = ((v[0] * d[0] + v[1] * d[1]) / (len * len)).clamp(0., 1.);
                        if *dashed && (t * len / scale).rem_euclid(3.) > 1.8 {
                            continue;
                        }
                        let distance = (v[0] - t * d[0]).hypot(v[1] - t * d[1]);
                        blend(
                            &mut pixels,
                            w,
                            h,
                            x as i32,
                            y as i32,
                            fg,
                            (radius + 0.5 - distance) as f32,
                        );
                    }
                }
            }
            Primitive::Text { at, size, value } => {
                let scaled = font.as_scaled(PxScale::from((size * scale) as f32));
                let mut x = (at[0] * scale) as f32;
                let y = (at[1] * scale) as f32;
                for c in value.chars() {
                    let id = scaled.glyph_id(c);
                    let glyph = id.with_scale_and_position(scaled.scale(), ab_glyph::point(x, y));
                    if let Some(outline) = font.outline_glyph(glyph) {
                        let bounds = outline.px_bounds();
                        outline.draw(|gx, gy, a| {
                            blend(
                                &mut pixels,
                                w,
                                h,
                                gx as i32 + bounds.min.x as i32,
                                gy as i32 + bounds.min.y as i32,
                                fg,
                                a,
                            )
                        });
                    }
                    x += scaled.h_advance(id);
                }
            }
        }
    }
    let mut output = Vec::new();
    {
        let mut encoder = ::png::Encoder::new(&mut output, w as u32, h as u32);
        encoder.set_color(::png::ColorType::Rgb);
        encoder.set_depth(::png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| DrawingError(e.to_string()))?;
        writer
            .write_image_data(&pixels)
            .map_err(|e| DrawingError(e.to_string()))?;
    }
    Ok(output)
}
