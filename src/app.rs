use eframe::{egui, epi};
use eframe::egui::*;

use xeh::prelude::*;
use xeh::state::{ErrorContext, TokenLocation};

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
    font: FontId,
    repl_code: String,
    frozen_code: Vec<FrozenStr>,
    highlight_line: Option<ErrorContext>,
    debug_token: Option<TokenLocation>,
    rdebug_enabled: bool,
    backup: Option<(Xstate, Vec<FrozenStr>)>,
    bin_future: Option<Pin<BoxFuture>>,
    canvas: Option<egui::TextureHandle>,
    canvas_zoom: f32,
    canvas_offs: Vec2,
    canvas_interaction: bool,
    setup_focus: bool,
    bytecode_open: bool,
}

#[derive(Clone)]
enum FrozenStr {
    Code(Xsubstr),
    Log(String),
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
            num_cols: 8,
            repl_code: String::new(),
            frozen_code: Vec::new(),
            highlight_line: None,
            debug_token: None,
            canvas: None,
            canvas_interaction: false,
            canvas_zoom: 1.0,
            canvas_offs: vec2(0.0, 0.0),
            backup: None,
            bin_future: None,
            setup_focus: true,
            rdebug_enabled: false,
            bytecode_open: true,
            font: FontId::monospace(14.0),
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

    fn binary_dropped(&mut self, s: Xbitstr) {
        self.xs.set_binary_input(s).unwrap();
        if self.backup.is_none() {
            // initial snapshot
            self.backup = Some((self.xs.clone(), self.frozen_code.to_owned()));
        }
    }

    fn canvas(&mut self, ctx: &egui::Context, interactive: bool) {
        if interactive {
            let zd = ctx.input().zoom_delta();
            if (zd - 1.0).abs() > 0.01 {
                let z = self.canvas_zoom + (zd - 1.0);
                self.canvas_zoom = z.min(8.0).max(0.1);
            }
            if ctx.input().pointer.any_down() {
                self.canvas_offs += ctx.input().pointer.delta();
            }
        }
        if self.canvas.is_none() {
            let t = ctx.load_texture("1", ColorImage::example());
            self.canvas = Some(t);
        }
        CentralPanel::default().show(ctx, |ui| {
            ui.with_layer_id(LayerId::background(), |ui| {
                if let Some(texture) = self.canvas.as_ref() {
                    let sx = self.canvas_offs.x;
                    let sy = self.canvas_offs.y;
                    let zoom = self.canvas_zoom;
                    let size = texture.size_vec2();
                    let img = egui::Image::new(texture, size);
                    let rect = Rect {
                        min: pos2(sx, sy),
                        max: pos2(sx + size.x * zoom, sy + size.y * zoom)
                    };
                    img.paint_at(ui, rect);
                }
            });
        });
    }

    fn editor(&mut self, ctx: &egui::Context) {
        let mut snapshot_clicked = false;
        let mut rollback_clicked = false;
        let mut eval_clicked = false;
        let mut debug_clicked = false;
        let mut next_clicked = false;
        let mut rnext_clicked = false;

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
                            self.bin_future.take();
                            self.binary_dropped(s);
                        }
                    }
                }
                if let Some(d) = ctx.input().raw.dropped_files.first() {
                    if let Some(data) = &d.bytes {
                        let s = Xbitstr::from(data.as_ref().to_owned());
                        self.binary_dropped(s);
                    }
                }
                
                ui.horizontal(|ui| {
                    if ui.button("Run").clicked() {
                        eval_clicked = true;
                    }
                    if ui.button("Debug").clicked() {
                        debug_clicked = true;
                    }
                    if ui.button("Next").clicked() {
                        next_clicked = true;
                    }
                    if self.rdebug_enabled && ui.button("Back").clicked() {
                        rnext_clicked = true;
                    }
                });
    
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
                if ui.checkbox(&mut self.rdebug_enabled, "Reverse Debugging").changed() {
                    if self.rdebug_enabled {
                        self.xs.start_recording();
                    } else {
                        self.xs.stop_recording();
                    }
                }
            });
        });

        // egui::Window::new("Bytecode")
        // .open(&mut self.bytecode_open)
        // .vscroll(true)
        // .show(ctx, |ui| {
        //     //ctx.style_ui(ui);
        //     ui.label(format!("ip={}", self.xs.ip()));
        //     ui.vertical(|ui| {
        //         for (ip, op) in self.xs.bytecode().iter().enumerate() {
        //             let optext = self.xs.fmt_opcode(ip, op);    
        //             let mut rich = RichText::new(format!("{:05x}: {}", ip, optext));
        //             if ip == self.xs.ip() {
        //                 rich = rich.background_color(Color32::LIGHT_GRAY);
        //             }
        //             ui.add(Label::new(rich));
        //         }
        //     });
        // });

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            let total_cols = self.num_cols * 3 + 2;
            let total_rows = self.num_rows;
            let glyph_width = ui.fonts().glyph_width(&self.font, '0');
            let row_height = ui.fonts().row_height(&self.font);
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
                if bs.len() > 0 {
                    let header = crate::style::hex_addr_rich(format!("{:06x},{}", start / 8, start % 8));
                    ui.add(Label::new(header));
                }
                        
                ui.set_min_height(size1.y * 1.5);

                while from < to {
                    let spacing = ui.spacing_mut().clone();
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.horizontal(|ui| {
                        let addr_text = crate::style::hex_addr_rich(format!("{:06x} ", from / 8));
                        ui.add(Label::new(addr_text));
                        let mut ascii = String::new();
                        ascii.push_str("  ");
                        for i in 0..self.num_cols {
                            if let Some((val, n)) = it.next() { 
                                let hex_data = crate::style::hex_data_rich(format!(" {:02x}", val), from < start);
                                ui.add(Label::new(hex_data));
                                let c= xeh::bitstring_mod::byte_to_dump_char(val);
                                ascii.push(c);
                                from += n as usize;
                            } else {
                                let n = (self.num_cols - i) as usize;
                                let mut s = String::with_capacity(n * 3);
                                for _ in 0..n {
                                    s.push_str("   ");
                                    ascii.push(' ');
                                }
                                let spaces = crate::style::hex_data_rich(s, false);
                                ui.add(Label::new(spaces));
                                break;
                            }
                        }
                        ui.add(Label::new(crate::style::hex_addr_rich(ascii)));
                    });
                    *ui.spacing_mut() = spacing;
                }
                if bs.len() > 0 {
                    let footer = crate::style::hex_addr_rich(format!("{:06x},{}", bs.end() / 8, bs.end() % 8));
                    ui.add(Label::new(footer));
                }
            });

            let resp = xgrid.response.interact(egui::Sense::drag());
            let v = resp.drag_delta();
            self.move_view(v.y as isize);

            ui.separator();
            ui.label("Data Stack:");

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
            let mut repl_has_focus = false;

            egui::containers::ScrollArea::vertical()
                     .stick_to_bottom().show(ui, |ui| {
                for x in self.frozen_code.iter() {
                    match x {
                        FrozenStr::Log(s) => {
                            let rich = crate::style::log(s.to_string());
                            ui.add(Label::new(rich));
                        }
                        FrozenStr::Code(s) => {
                            let mut is_error = false;
                            if let Some(errctx) = self.highlight_line.as_ref() {
                                if Xsubstr::shallow_eq(&errctx.location.whole_line, s) {
                                    is_error = true;
                                }
                            }
                            ui.add(Label::new(crate::style::code(s.to_string(), is_error)));
                            if let Some(dbg) = self.debug_token.as_ref() {
                                if Xsubstr::shallow_eq(&dbg.whole_line, s) {
                                    let text = format!("{:->1$}", '^', dbg.col + 1);
                                    let rich= crate::style::code(text, false);
                                    ui.add(Label::new(rich));
                                }
                            }
                        }
                    }
                }

                let code = egui::TextEdit::multiline(&mut self.repl_code)
                    .desired_rows(5)
                    .desired_width(f32::INFINITY)
                    .code_editor()
                    .id(Id::new("repl"));
                let res = ui.add(code);
                if self.setup_focus {
                    res.request_focus();
                    self.setup_focus = false;
                }
                repl_has_focus = res.has_focus();
                if repl_has_focus && ui.input().modifiers.ctrl 
                && ui.input().key_down(egui::Key::Enter)  {
                    eval_clicked = true;
                }
            });
            
            if !repl_has_focus {
                let n = hotkeys::scroll_view(ctx, self.num_cols);
                if n != 0 {
                    self.move_view(n);
                }
            }

            if let Some(log) = self.xs.console() {
                if !log.is_empty() {
                    let text = log.take().into();
                    self.frozen_code.push(FrozenStr::Log(text));
                }
            }

            if next_clicked || rnext_clicked {
                let res = if next_clicked { self.xs.next() } else { self.xs.rnext() };
                if let Err(err) = res {
                    let loc = self.xs.current_location();
                    self.debug_token = loc.clone();
                    self.highlight_line = loc.map(|location| ErrorContext {
                        err,
                        location,
                    });
                } else {
                    self.debug_token = self.xs.current_location();
                    self.highlight_line = None;
                }
            } else if eval_clicked && !self.repl_code.trim_end().is_empty() {
                let t = Instant::now();
                let xsrc = Xstr::from(self.repl_code.trim_end());
                let res = if eval_clicked {
                    self.xs.evalxstr(xsrc.clone())
                } else {
                    self.xs.compile_xstr(xsrc.clone())
                };
                if res.is_err() {
                    self.highlight_line = self.xs.last_error().clone();
                    self.debug_token = self.highlight_line.as_ref().map(|x| x.location.clone());
                } else if debug_clicked {
                    self.debug_token = self.xs.current_location();
                }
                for s in xeh::lex::XstrLines::new(xsrc) {
                    self.frozen_code.push(FrozenStr::Code(s))
                }
                if res.is_ok() {
                    let text = format!("OK {:0.3}s", t.elapsed().as_secs_f64()).into();
                    self.frozen_code.push(FrozenStr::Log(text));
                }
                self.repl_code.clear();
            }

            if next_clicked || rnext_clicked || eval_clicked || rollback_clicked  {
                if let Ok((w, h, buf)) = get_canvas_data(&mut self.xs) {
                    let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &buf);
                    if let Some(tex) = self.canvas.as_mut() {
                        tex.set(image);
                    } else {
                        let tex = ui.ctx().load_texture("canvas-texture", image);
                        self.canvas = Some(tex);
                    }
                }
            }
        });

    }
}

use std::future::{Future};
use std::task::{Poll, Context, Wake};
use std::pin::Pin;
use std::sync::Arc;

use crate::hotkeys;

struct MyWaker();

impl Wake for MyWaker {
    fn wake(self: Arc<Self>) {
    }
}

fn get_canvas_data(xs: &mut Xstate) -> Xresult1<(usize, usize, Vec<u8>)> {
    let (w, h) = xeh::d2_plugin::size(xs)?;
    if w > 0 && h  > 0 {
        let mut buf = Vec::new();
        xeh::d2_plugin::copy_rgba_data(xs, &mut buf)?;
        Ok((w, h, buf))
    } else {
        Err(Xerr::NotFound)
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "eframe template"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
        crate::style::tune(ctx, &self.font);
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
        if hotkeys::interactive_canvas_pressed(ctx) {
            self.canvas_interaction = !self.canvas_interaction;
        }
        self.canvas(ctx, self.canvas_interaction);
        if !self.canvas_interaction {
            self.editor(ctx);
        }
    }
}
