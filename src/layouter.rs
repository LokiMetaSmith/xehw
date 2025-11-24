use crate::style::*;
use eframe::egui;
use eframe::egui::text::TextFormat;
use xeh::prelude::*;
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use std::sync::OnceLock;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines())
}

fn get_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(|| ThemeSet::load_defaults())
}

pub fn word_under_cursor(s: &String, char_index: Option<usize>) -> Option<String> {
    let char_index = char_index?;
    let mut it = s.char_indices();
    let mut start = 0;
    let mut end = s.len();
    let mut cur_index = 0;
    while let Some((ci, c)) = it.next() {
        if cur_index < char_index {
            if c.is_whitespace() {
                start = ci + c.len_utf8();
            }
        } else {
            if c.is_whitespace() {
                end = ci;
            } else {
                while let Some((ci, c)) = it.next() {
                    if c.is_whitespace() {
                        end = ci;
                        break;
                    }
                }
            }
            break;
        }
        cur_index += 1;
    }
    if end - start < 1000 {
        Some(s[start..end].to_string())
    } else {
        None
    }
}

pub fn code_layouter(
    text: &str,
    err: Option<&Xsubstr>,
    dbg: Option<&Xsubstr>,
    font_id: &egui::FontId,
    wrap_width: f32,
    theme: &Theme,
) -> egui::text::LayoutJob {
    let mut j: egui::text::LayoutJob = Default::default();
    j.text = text.to_string(); // This effectively sets the text for all sections to reference
    j.wrap.max_width = wrap_width;

    let ps = get_syntax_set();
    let ts = get_theme_set();
    // Use Rust syntax highlighting as a fallback/default for xeh if no specific one exists
    let syntax = ps.find_syntax_by_extension("rs").unwrap_or_else(|| ps.find_syntax_plain_text());
    let syn_theme = &ts.themes["base16-ocean.dark"];
    let mut h = HighlightLines::new(syntax, syn_theme);

    let mut err_range = None;
    if let Some(s) = err {
        let start = s.range().start.min(text.len());
        let end = s.range().end.min(text.len());
        err_range = Some(start..end);
    }

    let mut dbg_range = None;
    if let Some(s) = dbg {
        let start = s.range().start.min(text.len());
        let end = s.range().end.min(text.len());
        dbg_range = Some(start..end);
    }

    // Iterate over lines to apply syntax highlighting
    // Note: egui LayoutJob expects byte ranges into the original text.
    // syntect works line by line. We need to track global byte offset.
    let mut global_offset = 0;

    for line in LinesWithEndings::from(text) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, ps).unwrap_or_default();
        for (style, range_text) in ranges {
            let start = global_offset;
            let end = global_offset + range_text.len();

            // Determine background/underline based on err/dbg ranges
            let mut background = theme.code_background;
            let mut underline = egui::Stroke::NONE;

            if let Some(r) = &err_range {
                if r.start < end && r.end > start {
                    // Overlaps with error
                    underline = egui::Stroke::new(1.0, theme.error);
                }
            }
            if let Some(r) = &dbg_range {
                if r.start < end && r.end > start {
                    // Overlaps with debug
                    background = theme.debug_marker;
                    underline = egui::Stroke::new(1.0, theme.debug_marker);
                }
            }

            let color = egui::Color32::from_rgb(style.foreground.r, style.foreground.g, style.foreground.b);

            j.sections.push(egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: start..end,
                format: TextFormat {
                    font_id: font_id.clone(),
                    color,
                    background,
                    underline,
                    ..Default::default()
                },
            });

            global_offset = end;
        }
    }

    j
}
