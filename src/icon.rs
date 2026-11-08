use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use png::{ColorType, Decoder, Transformations};

type Rgba = [u8; 4];

pub struct Image {
    width: usize,
    height: usize,
    pixels: Vec<Rgba>,
}

pub fn decode(favicon: &str) -> Option<Image> {
    let data = favicon.strip_prefix("data:image/png;base64,")?;
    let data: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = STANDARD.decode(data).ok()?;

    let mut decoder = Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let buf = &buf[..info.buffer_size()];

    let pixels = match info.color_type {
        ColorType::Rgba => buf.as_chunks::<4>().0.to_vec(),
        ColorType::Rgb => buf
            .as_chunks::<3>()
            .0
            .iter()
            .map(|&[r, g, b]| [r, g, b, 255])
            .collect(),
        ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&[v, a]| [v, v, v, a])
            .collect(),
        ColorType::Grayscale => buf.iter().map(|&v| [v, v, v, 255]).collect(),
        ColorType::Indexed => return None,
    };

    Some(Image {
        width: info.width as usize,
        height: info.height as usize,
        pixels,
    })
}

impl Image {
    fn resize(&self, size: usize) -> Vec<Rgba> {
        let span = |t: usize, len: usize| {
            let start = t * len / size;
            start..((t + 1) * len / size).max(start + 1).min(len)
        };
        let mut out = Vec::with_capacity(size * size);
        for ty in 0..size {
            for tx in 0..size {
                let mut sum = [0u32; 4];
                let mut count = 0u32;
                for y in span(ty, self.height) {
                    for x in span(tx, self.width) {
                        let [r, g, b, a] = self.pixels[y * self.width + x];
                        let a = u32::from(a);
                        sum[0] += u32::from(r) * a;
                        sum[1] += u32::from(g) * a;
                        sum[2] += u32::from(b) * a;
                        sum[3] += a;
                        count += 1;
                    }
                }
                out.push(match sum[3] {
                    0 => [0, 0, 0, 0],
                    alpha => [
                        (sum[0] / alpha) as u8,
                        (sum[1] / alpha) as u8,
                        (sum[2] / alpha) as u8,
                        (alpha / count) as u8,
                    ],
                });
            }
        }
        out
    }

    pub fn render(&self, size: usize) -> Vec<String> {
        let pixels = self.resize(size);
        let visible = |p: Rgba| (p[3] >= 128).then_some(p);
        (0..size)
            .step_by(2)
            .map(|y| {
                let mut line = String::new();
                for x in 0..size {
                    let top = visible(pixels[y * size + x]);
                    let bottom = pixels.get((y + 1) * size + x).copied().and_then(visible);
                    match (top, bottom) {
                        (Some(t), Some(b)) => line.push_str(&format!(
                            "\x1b[38;2;{};{};{};48;2;{};{};{}m▀",
                            t[0], t[1], t[2], b[0], b[1], b[2]
                        )),
                        (Some(t), None) => {
                            line.push_str(&format!("\x1b[0;38;2;{};{};{}m▀", t[0], t[1], t[2]))
                        }
                        (None, Some(b)) => {
                            line.push_str(&format!("\x1b[0;38;2;{};{};{}m▄", b[0], b[1], b[2]))
                        }
                        (None, None) => line.push_str("\x1b[0m "),
                    }
                }
                line.push_str("\x1b[0m");
                line
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn favicon(width: u32, height: u32, pixels: &[u8]) -> String {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(pixels)
            .unwrap();
        format!("data:image/png;base64,{}", STANDARD.encode(bytes))
    }

    #[test]
    fn decodes_and_averages_blocks() {
        let red = [255, 0, 0, 255];
        let blue = [0, 0, 255, 255];
        let clear = [0, 0, 0, 0];
        let pixels: Vec<u8> = [
            red, red, blue, blue, red, red, blue, blue, clear, clear, red, blue, clear, clear,
            blue, red,
        ]
        .concat();
        let image = decode(&favicon(4, 4, &pixels)).unwrap();
        assert_eq!(
            image.resize(2),
            [
                [255, 0, 0, 255],
                [0, 0, 255, 255],
                [0, 0, 0, 0],
                [127, 0, 127, 255]
            ]
        );
    }

    #[test]
    fn renders_half_blocks() {
        let pixels: Vec<u8> = [
            [10, 20, 30, 255],
            [0, 0, 0, 0],
            [40, 50, 60, 255],
            [70, 80, 90, 255],
        ]
        .concat();
        let image = decode(&favicon(2, 2, &pixels)).unwrap();
        let lines = image.render(2);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0],
            "\x1b[38;2;10;20;30;48;2;40;50;60m▀\x1b[0;38;2;70;80;90m▄\x1b[0m"
        );
    }

    #[test]
    fn rejects_non_png_data() {
        assert!(decode("data:image/png;base64,bm90IGEgcG5n").is_none());
        assert!(decode("https://example.com/icon.png").is_none());
    }
}
