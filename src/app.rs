use eframe::{egui, epi};
use eframe::egui::*;

use xeh::prelude::*;

#[cfg(target_arch = "wasm32")]
type Instant = instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
type Instant = std::time::Instant;

type BoxFuture = Box<dyn Future<Output = Vec<u8>>>;
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    xs: Xstate,
    view_start: isize,
    num_rows: isize,
    num_cols: isize,
    live_code: String,
    win_size: Vec2,
    frozen_code: Vec<FrozenStr>,
    error_highlight: Xsubstr,
    backup: Option<(Xstate, Vec<FrozenStr>)>,
    bin_future: Option<Pin<BoxFuture>>,
    canvas: Option<egui::TextureHandle>,
    canvas_open: bool,
    canvas_zoom: usize,
    setup_focus: bool,
    help_open: bool,
}

#[derive(Clone)]
enum FrozenStr {
    Code(),

    text: Xsubstr,
    fg: Color32,
    bg: Option<Color32>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let mut xs = Xstate::boot().unwrap();
        xs.capture_stdout();
        xeh::d2_plugin::load(&mut xs).unwrap();
        Self {
            xs,
            view_start: 0,
            num_rows: 10,
            num_cols: 16,
            win_size: Vec2::new(640.0, 480.0),
            live_code: String::new(),
            frozen_code: Vec::new(),
            error_highlight: Xstr::from("").into(),
            canvas: None,
            canvas_open: true,
            canvas_zoom: 1,
            backup: None,
            bin_future: None,
            setup_focus: true,
            help_open: false,
        }
    }
}

impl TemplateApp {
    fn move_view(&mut self, nrows: isize) {
        let limit = (self.current_bstr().end() / 8) as isize;
        let n = self.view_start + self.num_rows * nrows;
        self.view_start = n.max(0).min((limit - 1).max(0));
    }

    fn current_bstr(&self) -> Xbitstr {
        self.xs.get_var_value("bitstr/input").unwrap().clone().to_bitstring().unwrap()
    }
}

use std::future::{Future};
use std::task::{Poll, Context, Wake};
use std::pin::Pin;
use std::sync::Arc;

struct MyWaker();

impl Wake for MyWaker {
    fn wake(self: Arc<Self>) {
    }
}

fn get_canvas_data(xs: &mut Xstate) -> Xresult1<(usize, usize, Xbitstr)> {
    xs.eval_word("d2-width")?;
    let w = xs.pop_data()?.to_usize()?;
    xs.eval_word("d2-height")?;
    let h = xs.pop_data()?.to_usize()?;
    if w > 0 && h > 0 {
        xs.eval_word("d2-capture-rgba")?;
        let bs = xs.pop_data()?.to_bitstring()?;
        Ok((w, h, bs))
    } else {
        Err(Xerr::NotFound)
    }
}

fn zoom_image(zoom: usize, w: usize, h: usize, data: &[u8]) -> ColorImage {
    if zoom == 1 {
        return egui::ColorImage::from_rgba_unmultiplied([w, h], data);
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
    assert_eq!(wx * wx * 4, buf.len());
    return egui::ColorImage::from_rgba_unmultiplied([wx, hx], &buf);
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "eframe template"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        let mut snapshot_clicked = false;
        let mut rollback_clicked = false;
        let mut run_clicked = false;
        let mut debug_clicked = false;
        let mut zoom_changed = false;
        self.win_size = ctx.used_size();

        let font = FontId::monospace(14.0);

        // update style
        let mut vis = Visuals::default();
        vis.override_text_color = Some(Color32::from_rgb(0xE6, 0x9F, 0x00));
        ctx.set_visuals(vis);

        let mut style = (*ctx.style()).clone();
        style.override_font_id = Some(font.clone());
        ctx.set_style(style);        

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open Binary...").clicked() {
                    self.bin_future = Some(Box::pin(async {
                        let res = rfd::AsyncFileDialog::new()
                            .pick_file()
                            .await;
                        if let Some(file) = res {
                            file.read().await
                        } else {
                            Vec::new()
                        }
                    }));
                }
                if let Some(future) = self.bin_future.as_mut() {
                    let waker = Arc::new(MyWaker()).into();
                    let context = &mut Context::from_waker(&waker);
                    match Pin::new(future).poll(context) {
                        Poll::Pending => (),
                        Poll::Ready(data) => {
                            let s = Xbitstr::from(data);
                            self.xs.set_binary_input(s).unwrap();
                            self.bin_future.take();
                        }
                    }
                }
                if let Some(d) = ctx.input().raw.dropped_files.first() {
                    if let Some(data) = &d.bytes {
                        let s = Xbitstr::from(data.as_ref().to_owned());
                        self.xs.set_binary_input(s).unwrap();
                    }
                }

                if ui.input().modifiers.ctrl && ui.input().key_down(egui::Key::Enter)  {
                    run_clicked = true;
                }
                snapshot_clicked = ui.button("Snapshot").clicked();
                if ui.input().modifiers.ctrl && ui.input().key_down(egui::Key::G) {
                    snapshot_clicked = true;
                }
                if snapshot_clicked {
                    let t = Instant::now();
                    self.backup = Some((self.xs.clone(), self.frozen_code.to_owned()));
                    self.xs.print(&format!("Snapshot {:0.3}s", t.elapsed().as_secs_f64()));
                }
                if self.backup.is_some() {
                    rollback_clicked = ui.button("Rollback").clicked();
                    if ui.input().modifiers.ctrl && ui.input().key_down(egui::Key::K) {
                        rollback_clicked  = true;
                    }
                    if rollback_clicked {
                        if let Some((xs_old, frozen)) = self.backup.clone() {
                            self.xs = xs_old;
                            self.frozen_code = frozen;
                        }
                    }
                }
                if ui.button("Help").clicked() {
                    self.help_open = true;
                }
            });
        });

        egui::Window::new("Canvas")
        .id(Id::new("canvas-window"))
        .default_pos([0.0, 500.0])
        .open(&mut self.canvas_open)
        .show(ctx, |ui| {
            let old_zoom = self.canvas_zoom;
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.canvas_zoom, 1, "x1");
                ui.radio_value(&mut self.canvas_zoom, 2, "x2");
                ui.radio_value(&mut self.canvas_zoom, 4, "x4");
            });
            zoom_changed = old_zoom != self.canvas_zoom;
            if let Some(texture) = self.canvas.as_ref() {
                ui.label(format!("Canvas {}x{}", texture.size_vec2().x, texture.size_vec2().y));
                ui.add(egui::Image::new(texture, texture.size_vec2()));
            } else {
                ui.label("Canvas is empty");
            }
        });

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            let total_cols = self.num_cols * 3 + 2;
            let total_rows = self.num_rows;
            let glyph_width = ui.fonts().glyph_width(&font, '0');
            let row_height = ui.fonts().row_height(&font);
            let size1 = Vec2::new(total_cols as f32 * glyph_width,
                total_rows as f32 * row_height);
            ui.set_min_width(size1.x);
            
            let xgrid = ui.vertical(|ui|{
                let bs = self.current_bstr();
                let mut from = (self.view_start as usize) * 8;
                let mut it = bs.iter8_unleashed(from);
                let visible_bits = self.num_rows * self.num_cols * 8;
                let to = bs.end().min(from + visible_bits as usize);
                let start = bs.start();
                let header = format!("consumed {}.{} of {}.{} bytes", start / 8, start % 8,
                    bs.end() / 8, bs.end() % 8);
                ui.monospace(header);
                ui.set_min_height(size1.y * 1.5);

                while from < to {
                    ui.horizontal(|ui| {
                        let addr_text = RichText::new(format!("{:06x}", from / 8)).monospace();
                        ui.add(egui::Label::new(addr_text));
                        ui.separator();
                        let xspacing = ui.spacing_mut().item_spacing;
                        ui.spacing_mut().item_spacing *= 0.5;
                        for _ in 0..self.num_cols {
                            if let Some((val, n)) = it.next() {
                                let mut text = RichText::new(&format!("{:02x}", val)).monospace();
                                if from < start {
                                    text = text.color(Color32::DARK_GRAY);
                                }
                                ui.add(Label::new(text));
                                from += n as usize;
                            } else {
                                break;
                            }
                        }
                        ui.spacing_mut().item_spacing = xspacing;
                    });
                }
            });

            let resp = xgrid.response.interact(egui::Sense::drag());
            let v = resp.drag_delta();
            self.move_view(v.y as isize);

            ui.separator();
            ui.label(format!("Data Stack: {} items", self.xs.data_depth()));

            egui::containers::ScrollArea::vertical().show(ui, |ui| {
                ui.set_min_width(size1.x);
                ui.set_max_width(size1.x);
                for i in 0.. {
                    if let Some(x) = self.xs.get_data(i) {
                        let mut s = self.xs.format_cell(x).unwrap();
                        if s.chars().count() > total_cols as usize {
                            s.truncate(total_cols as usize - 3);
                            s.push_str("...");
                        }
                        let mut val = egui::RichText::new(s).monospace();
                        if i >= self.xs.data_depth() {
                            val = val.background_color(Color32::DARK_GRAY);
                        }
                        ui.horizontal(|ui| {
                            ui.monospace(format!("{:4}:", i));
                            ui.label(val);
                        });
                    } else {
                        break;
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let mut code_has_focus = false;
            
            egui::containers::ScrollArea::vertical()
                     .stick_to_bottom().show(ui, |ui| {
                for x in self.frozen_code.iter() {
                    let mut richtext = RichText::new(x.text.to_string())
                        .monospace()
                        .color(x.fg);
                    if let Some(bg) = x.bg {
                        richtext = richtext.background_color(bg);
                    }
                    ui.add(Label::new(richtext));
                }
                let code = egui::TextEdit::multiline(&mut self.live_code)
                    .desired_width(f32::INFINITY)
                    .code_editor()
                    .id(Id::new("code"));
                let res = ui.add(code);
                if self.setup_focus {
                    res.request_focus();
                    self.setup_focus = false;
                }
                code_has_focus = res.has_focus();
            });
            
            ui.horizontal(|ui| {
                if ui.button("Run").clicked() {
                    run_clicked = true;
                }
                if ui.button("Debug").clicked() {
                    debug_clicked = true;
                }
            });

            if !code_has_focus {
                if ctx.input().key_pressed(egui::Key::ArrowUp) {
                    self.move_view(-1);
                }
                if ctx.input().key_pressed(egui::Key::PageUp) {
                    self.move_view(-self.num_rows);
                }
                if ctx.input().key_pressed(egui::Key::ArrowDown) {
                    self.move_view(1);
                }
                if ctx.input().key_pressed(egui::Key::PageDown) {
                    self.move_view(self.num_rows);
                }
            }

            if let Some(log) = self.xs.console() {
                if !log.is_empty() {
                    let text = log.take().into();
                    self.frozen_code.push(FrozenStr { text, fg: Color32::GRAY, bg: None});
                }
            }
            if (run_clicked || debug_clicked) && !self.live_code.trim_end().is_empty() {
                let t = Instant::now();
                let xsrc = Xstr::from(self.live_code.trim_end());
                let res = if run_clicked {
                    self.xs.evalxstr(xsrc.clone())
                } else {
                    self.xs.loadxstr(xsrc.clone())
                };
                let mut err_line = None;
                if res.is_err() {
                    err_line = self.xs.last_error().map(|e| e.1.whole_line);
                }
                for (line, text) in xeh::lex::XstrLines::new(xsrc).enumerate() {
                    let bg = if Some(&text) == err_line.as_ref() {
                        Some(Color32::RED)
                    } else {
                        None
                    };
                    self.frozen_code.push(FrozenStr{text, fg: Color32::WHITE, bg});
                }
                if res.is_ok() {
                    let text = format!("OK {:0.3}s", t.elapsed().as_secs_f64()).into();
                    self.frozen_code.push(FrozenStr{text,fg:Color32::GRAY,bg:None});
                }
                self.live_code.clear();
            }
            if run_clicked || rollback_clicked || zoom_changed {
                if let Ok((w, h, bs)) = get_canvas_data(&mut self.xs) {
                    let image = zoom_image(self.canvas_zoom, w, h, bs.slice());
                    if let Some(tex) = self.canvas.as_mut() {
                        tex.set(image);
                    } else {
                        let tex = ui.ctx().load_texture("canvas-texture", image);
                        self.canvas = Some(tex);
                    }
                }
            }
        });

        if self.help_open {
            Window::new("Help")
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut self.help_open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label("Drag and Drop file or click \"Open Binary...\" to start exploring");
                ui.label("Click \"Run\" or Ctrl+Return to evaluate expression in the code window");
            });
        }
    }
}
