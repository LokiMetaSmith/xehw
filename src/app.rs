use eframe::{egui, epi};
use eframe::egui::*;

use xeh::prelude::*;
use crate::{style::*, hotkeys};
use crate::hotkeys::*;
use crate::canvas::*;

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
    start_row: isize,
    num_rows: isize,
    num_cols: isize,
    live_code: String,
    trial_code: Option<String>,
    frozen_code: Vec<FrozenStr>,
    last_dt: Option<f64>,
    canvas: Canvas,
    debug_token: Option<TokenLocation>,
    rdebug_enabled: bool,
    snapshot: Option<(Xstate, Vec<FrozenStr>)>,
    bin_future: Option<Pin<BoxFuture>>,
    setup_focus: bool,
    bytecode_open: bool,
    help_open: bool,
}

#[derive(Clone)]
enum FrozenStr {
    Code(Xsubstr),
    Log(String),
}

impl Default for TemplateApp {
    fn default() -> Self {
        let mut xs = Xstate::boot().unwrap();
        xs.intercept_stdout(true);
        xeh::d2_plugin::load(&mut xs).unwrap();
        Self {
            xs,
            start_row: 0,
            num_rows: 10,
            num_cols: 8,
            live_code: String::new(),
            frozen_code: Vec::new(),
            trial_code: None,
            last_dt: None,
            debug_token: None,
            canvas: Canvas::new(),
            snapshot: None,
            bin_future: None,
            setup_focus: true,
            rdebug_enabled: false,
            bytecode_open: true,
            help_open: false,
        }
    }
}

impl TemplateApp {
    fn move_view(&mut self, nrows: isize) {
        let n = (self.start_row + nrows)
            .max(0)
            .min(self.current_bstr().end() as isize / (self.num_cols * 8));
        self.start_row = n.max(0);
    }

    fn current_bstr(&self) -> &Xbitstr {
        self.xs.get_var_value("current-bitstr").unwrap().bitstr().unwrap()
    }

    fn current_offset(&self) -> usize {
        self.xs.get_var_value("offset").unwrap().to_usize().unwrap()
    }

    fn binary_dropped(&mut self, s: Xbitstr) {
        self.xs.set_binary_input(s).unwrap();
        if self.snapshot.is_none() {
            // initial snapshot
            self.snapshot = Some((self.xs.clone(), self.frozen_code.to_owned()));
        }
    }

    fn editor(&mut self, ctx: &egui::Context) {
        let mut snapshot_clicked = false;
        let mut rollback_clicked = false;
        let mut run_clicked = false;
        let mut next_clicked = false;
        let mut rnext_clicked = false;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open...").clicked() {
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
                    run_clicked = ui.button("Evaluate").clicked();
                    let has_log = self.xs.reverse_log.as_ref().map(|l| l.len() > 0).unwrap_or(false);
                    let rnext_btn = ui.add_enabled(has_log, Button::new("Rnext"));
                    if has_log {
                        rnext_clicked = rnext_btn.clicked() || rnext_pressed(ui);
                    }
                    let next_btn = ui.add_enabled(self.xs.is_running(), Button::new("Next"));
                    if self.xs.is_running() {
                        next_clicked = next_btn.clicked() || next_pressed(ui);
                    }
                });
    
                snapshot_clicked = ui.button("Snapshot").clicked() || snapshot_pressed(ui);
                if snapshot_clicked {
                    let t = Instant::now();
                    self.snapshot = Some((self.xs.clone(), self.frozen_code.to_owned()));
                    self.last_dt = Some(t.elapsed().as_secs_f64());
                }
                rollback_clicked = ui.add_enabled(self.snapshot.is_some(), Button::new("Rollback")).clicked()
                    || rollback_pressed(ui);
                if rollback_clicked {
                    if let Some((xs_old, frozen)) = self.snapshot.clone() {
                        self.xs = xs_old;
                        self.frozen_code = frozen;
                    }
                }
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.rdebug_enabled, "Reverse Debugging").changed() {
                        if self.rdebug_enabled {
                            self.xs.start_recording();
                        } else {
                            self.xs.stop_recording();
                        }
                    }
                });
                if ui.button("Help (Ctrl+G)").clicked() || help_pressed(ui) {
                    self.help_open = !self.help_open;
                }
            });
        });

        // egui::Window::new("Bytecode")
        // .open(&mut self.bytecode_open)
        // .default_pos(pos2(200.0, 400.0))
        // .vscroll(true)
        // .show(ctx, |ui| {
        //     //ctx.style_ui(ui);
        //     ui.label(format!("ip={}", self.xs.ip()));
        //     ui.vertical(|ui| {
        //         for (ip, op) in self.xs.bytecode().iter().enumerate() {
        //             let optext = self.xs.fmt_opcode(ip, op);    
        //             let mut rich = RichText::new(format!("{:05x}: {}", ip, optext)).monospace().color(TEXT_FG);
        //             if ip == self.xs.ip() {
        //                 rich = rich.background_color(TEXT_HIGLIGHT);
        //             }
        //             ui.label(rich);
        //         }
        //     });
        // });

        egui::Window::new("Help")
        .open(&mut self.help_open)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .vscroll(true)
        .show(ctx, |ui| {
            let add = |ui: &mut Ui, text, combo| {
                ui.horizontal(|ui| {
                    ui.label(text);
                    ui.label(RichText::new(combo).color(GREEN));
                });
            };
            ui.heading("Hotkeys");
            ui.label("Drag and drop binary file to start a new program.");
            add(ui, "Open binary file...", "(Ctrl + O)");
            add(ui, "Program - Evaluate", "(Ctrl + Enter)");
            add(ui, "Program - Snapshot", "(Ctrl + Shift + S)");
            add(ui, "Program - Rollback", "(Ctrl + Shift + R)");
            add(ui, "Debugger - Next", "(Alt + Right)");
            add(ui, "Debugger - Reverse Next", "(Alt + Left)");
            add(ui, "Canvas - Show", "(Ctrl + Shift + M)");
            add(ui, "Switch to Hex Panel", "(Ctrl + 1)");
            add(ui, "Switch to Code Panel", "(Ctrl + 2)");
            add(ui, "Help - Show", "(Ctrl + G)");
        });

       let hex_panel = egui::SidePanel::left("left_panel").show(ctx, |ui| {
            let ncols = self.num_cols * 4 + 10;
            let total_rows = self.num_rows;
            let text_style = TextStyle::Monospace;
            let font = text_style.resolve(ui.style());
            let glyph_width = ui.fonts().glyph_width(&font, '0');
            let row_height = ui.fonts().row_height(&font);
            let size1 = Vec2::new(ncols as f32 * glyph_width,
                total_rows as f32 * row_height);
            ui.set_min_width(size1.x);

            let xgrid = ui.vertical(|ui|{
                let offset = self.current_offset();
                let mut from = (self.start_row * self.num_cols * 8) as usize;
                let bs = self.current_bstr().seek(from).unwrap_or_default();
                let mut it = bs.iter8();
                let visible_bits = self.num_rows * self.num_cols * 8;
                let to = bs.end().min(from + visible_bits as usize);

                ui.set_min_height(size1.y);
                if bs.len() > 0 {
                    let header = hex_addr_rich(
                        format!("{:06x},{}", offset / 8, offset % 8));
                    ui.add(Label::new(header));
                }

                while from < to {
                    let spacing = ui.spacing_mut().clone();
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.horizontal(|ui| {
                        let addr_text = hex_addr_rich(format!("{:06x} ", from / 8));
                        ui.add(Label::new(addr_text));
                        let mut ascii = String::new();
                        ascii.push_str("  ");
                        for i in 0..self.num_cols {
                            if let Some((val, n)) = it.next() { 
                                let hex_data = hex_data_rich(
                                    format!(" {:02x}", val), from < offset);
                                ui.label(hex_data);
                                let c= xeh::bitstr_ext::byte_to_dump_char(val);
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
                    let footer = crate::style::hex_addr_rich(
                        format!("{:06x},{}", bs.end() / 8, bs.end() % 8));
                    ui.add(Label::new(footer));
                }
            });

            let resp = xgrid.response.interact(egui::Sense::drag());
            let v = resp.drag_delta();
            self.move_view(v.y as isize);

            ui.separator();
            ui.label(RichText::new("Data Stack:").color(COMMENT_FG));

            egui::containers::ScrollArea::vertical().show(ui, |ui| {
                ui.set_min_width(size1.x);
                ui.set_max_width(size1.x);
                for i in 0.. {
                    if let Some(x) = self.xs.get_data(i) {
                        let mut s = self.xs.format_cell(x).unwrap();
                        if s.chars().count() > ncols as usize {
                            s.truncate(ncols as usize - 3);
                            s.push_str("...");
                        }
                        let mut val = egui::RichText::new(s).monospace();
                        if i >= self.xs.data_depth() {
                            val = val.background_color(Color32::DARK_GRAY);
                        }
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{:4}:", i)).monospace().color(COMMENT_FG));
                            ui.label(val);
                        });
                    } else {
                        break;
                    }
                }
            });
        });
        if hotkeys::switch_to_grid_pressed(&ctx.input()) {
            hex_panel.response.request_focus();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut live_has_focus = false;

            egui::containers::ScrollArea::vertical()
                     .stick_to_bottom().show(ui, |ui| {
                for x in self.frozen_code.iter() {
                    match x {
                        FrozenStr::Log(s) => {
                            ui.label(RichText::new(s.to_string()).monospace().color(COMMENT_FG));
                        }
                        FrozenStr::Code(s) => {
                            if let Some((err, loc)) = self.xs.last_error() {
                                if Xsubstr::shallow_eq(&loc.whole_line, s) {
                                    let (a, b, c) = split_highlight(&loc);
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    ui.horizontal_top(|ui| {
                                        ui.label(RichText::new(a).monospace().color(CODE_FG));
                                        ui.label(RichText::new(b).monospace().background_color(CODE_ERR_BG));
                                        ui.label(RichText::new(c).monospace().color(CODE_FG));
                                    });
                                    let n: usize = loc.whole_line
                                        .chars()
                                        .take(loc.col)
                                        .map(|c| if c == '\t' { egui::text::TAB_SIZE } else { 1 })
                                        .sum();
                                    let pos = format!("{:->1$}", '^', n + 1);
                                    ui.label(RichText::new(pos).monospace().color(CODE_ERR_BG));
                                    ui.label(RichText::new(format!("error: {}", err))
                                        .monospace().color(CODE_ERR_BG));
                                    continue;
                                }
                            }
                            if let Some(dbg) = self.debug_token.as_ref() {
                                if Xsubstr::shallow_eq(&dbg.whole_line, s) {
                                    let (a, b, c) = split_highlight(dbg);
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    ui.horizontal_top(|ui| {
                                        ui.label(RichText::new(a).monospace().color(CODE_FG));
                                        ui.label(RichText::new(b).monospace().background_color(CODE_DBG_BG));
                                        ui.label(RichText::new(c).monospace().color(CODE_FG));
                                    });
                                    continue;
                                }
                            }
                            ui.label(RichText::new(s.to_string()).monospace().color(CODE_FG));
                        }
                    }
                }
                if let Some(secs) = self.last_dt {
                    ui.colored_label(COMMENT_FG, format!("{:.4}s", secs));
                }
                let code = egui::TextEdit::multiline(&mut self.live_code)
                    .desired_rows(1)
                    .desired_width(f32::INFINITY)
                    .code_editor()
                    .margin(vec2(0.0, 2.0))
                    .id(Id::new("live"));
                let res = ui.add(code);
                if hotkeys::switch_to_code_pressed(&ctx.input()) {
                    res.request_focus();
                    self.setup_focus = false;
                }
                if self.setup_focus {
                    res.request_focus();
                    self.setup_focus = false;
                }
                live_has_focus = res.has_focus();
                if live_has_focus && run_pressed(ui)  {
                    run_clicked = true;
                }
            });
            
            if !live_has_focus {
                let n = scroll_view_pressed(ctx, self.num_cols);
                if n != 0 {
                    self.move_view(n);
                }
            }

            if let Some(s) = self.xs.stdout() {
                if !s.is_empty() {
                    self.frozen_code.push(FrozenStr::Log(s.take()));
                }
            }

            if next_clicked || rnext_clicked {
                let _res = if next_clicked { self.xs.next() } else { self.xs.rnext() };
                self.debug_token = self.xs.token_from_current_ip();
            } else if run_clicked && !self.live_code.trim().is_empty() {
                let t = Instant::now();
                let xsrc = Xstr::from(self.live_code.trim_end());
                let res = self.xs.evalxstr(xsrc.clone());
                self.debug_token = self.xs.token_from_current_ip();
                for s in xeh::lex::XstrLines::new(xsrc) {
                    self.frozen_code.push(FrozenStr::Code(s))
                }
                self.last_dt = Some(t.elapsed().as_secs_f64());
                self.live_code.clear();
            }
            if next_clicked || rnext_clicked || run_clicked || rollback_clicked  {
                if let Ok((w, h, buf)) = crate::canvas::copy_rgba(&mut self.xs) {
                    self.canvas.update(ctx, w, h, buf);
                }
            }
        });

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

fn split_highlight(loc: &TokenLocation) -> (String, String, String) {
    let line = &loc.whole_line;
    let start = loc.col;
    let end = (start + loc.token.len()).min(line.len());
    let a = line.substr(0..start).to_string();
    let b = line.substr(start..end).to_string();
    let c = line.substr(end..).to_string();
    (a, b, c)
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
        crate::style::tune(ctx);
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
        self.canvas.ui(ctx);
        if !self.canvas.interactive() {
            self.editor(ctx);
        }
    }
}
