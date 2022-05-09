use eframe::egui::*;
use xeh::prelude::*;

pub struct Canvas {
    tex: Option<TextureHandle>,
    zoom: f32,
    offs: Vec2,
    pub interactive: bool,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            tex: None,
            interactive: false,
            zoom: 1.0,
            offs: vec2(0.0, 0.0),
        }
    }

    pub fn interactive(&mut self) -> bool {
        self.interactive
    }

    pub fn ui(&mut self, ctx: &Context) {
        if self.interactive {
            let zd = ctx.input().zoom_delta();
            if (zd - 1.0).abs() > 0.01 {
                let z = self.zoom + (zd - 1.0);
                self.zoom = z.min(8.0).max(0.1);
            }
            if ctx.input().pointer.any_down() {
                self.offs += ctx.input().pointer.delta();
            }
        }
        CentralPanel::default().show(ctx, |ui| {
            //ui.with_layer_id(LayerId::background(), |ui| {
            if let Some(texture) = self.tex.as_ref() {
                let sx = self.offs.x;
                let sy = self.offs.y;
                let zoom = self.zoom;
                let size = texture.size_vec2();
                let img = Image::new(texture, size);
                let rect = Rect {
                    min: pos2(sx, sy),
                    max: pos2(sx + size.x * zoom, sy + size.y * zoom),
                };
                img.paint_at(ui, rect);
            }
            //});
        });
    }

    pub fn update(&mut self, ctx: &Context, w: usize, h: usize, buf: Vec<u8>) {
        let image = ColorImage::from_rgba_unmultiplied([w, h], &buf);
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
        Err(Xerr::OutOfBounds(0))
    }
}
