use eframe::egui::*;
use xeh::prelude::*;

pub struct Canvas {
    tex: Option<TextureHandle>,
    zoom: usize,
}

impl Canvas {
    pub fn new() -> Self {
        Self { tex: None, zoom: 1 }
    }

    pub fn is_empty(&self) -> bool {
        self.tex.is_none()
    }

    pub fn size(&self) -> Vec2 {
        if let Some(texture) = self.tex.as_ref() {
            texture.size_vec2()
        } else {
            vec2(0.0, 0.0)
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, theme: &crate::style::Theme) {
        let size = self.size();
        ui.horizontal(|ui| {
            ui.colored_label(theme.comment, format!("{}x{}", size.x, size.y));
            ui.add(
                Slider::new(&mut self.zoom, 1..=32)
                    .text("zoom")
                    .text_color(theme.comment),
            );
        });
        if let Some(texture) = self.tex.as_ref() {
            let size = texture.size_vec2();
            ui.image(texture, size);
        }
    }

    pub fn update(&mut self, ctx: &Context, w: usize, h: usize, buf: Vec<u8>) {
        let image = zoom_image(self.zoom, w, h, &buf);
        if let Some(tex) = self.tex.as_mut() {
            tex.set(image);
        } else {
            let tex = ctx.load_texture("canvas-texture", image);
            self.tex = Some(tex);
        }
    }
}

pub fn copy_rgba(xs: &mut Xstate) -> Xresult1<(usize, usize, Vec<u8>)> {
    let (w, h) = xeh::d2_plugin::size(xs)?;
    if w > 0 && h > 0 {
        let mut buf = Vec::new();
        xeh::d2_plugin::copy_rgba_data(xs, &mut buf)?;
        Ok((w, h, buf))
    } else {
        Err(Xerr::OutOfBounds { index: 0, range: 0 })
    }
}

fn zoom_image(zoom: usize, w: usize, h: usize, data: &[u8]) -> ColorImage {
    if zoom == 1 {
        return ColorImage::from_rgba_unmultiplied([w, h], data);
    }
    let wx = w * zoom;
    let hx = h * zoom;
    let mut buf: Vec<u8> = Vec::new();
    for y in 0..h {
        for _ in 0..zoom {
            for x in 0..w {
                for _ in 0..zoom {
                    let idx = (y * w + x) * 4;
                    buf.push(data[idx]);
                    buf.push(data[idx + 1]);
                    buf.push(data[idx + 2]);
                    buf.push(data[idx + 3]);
                }
            }
        }
    }
    assert_eq!(hx * wx * 4, buf.len());
    ColorImage::from_rgba_unmultiplied([wx, hx], &buf)
}
