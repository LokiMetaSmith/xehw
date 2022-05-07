use crate::style::*;
use eframe::egui;
use eframe::egui::text::TextFormat;
use xeh::prelude::*;

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
    j.text = text.to_string();
    let len = text.len();
    let mut slst = Vec::new();
    let mut err_start = 0;
    let mut err_end = 0;
    if let Some(s) = err {
        err_start = s.range().start.min(len);
        err_end = s.range().end.min(len);
        slst.push((err_start, 0));
        slst.push((err_end, 0));
    }
    if let Some(s) = dbg {
        let dbg_start = s.range().start.min(len);
        let dbg_end = s.range().end.min(len);
        let r = err_start..err_end;
        if !(r.contains(&dbg_start) || r.contains(&dbg_end)) {
            slst.push((dbg_start, 1));
            slst.push((dbg_end, 1));
        }
    }
    slst.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut start = 0;
    let mut it = slst.into_iter();
    while let Some((p1, dbg)) = it.next() {
        j.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: start..p1,
            format: TextFormat::simple(font_id.clone(), theme.code),
        });
        let (p2, _) = it.next().unwrap();
        j.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: p1..p2,
            format: TextFormat {
                font_id: font_id.clone(),
                color: theme.code,
                background: if dbg == 1 {
                    theme.debug_marker
                } else {
                    theme.error
                },
                ..Default::default()
            },
        });
        start = p2;
    }
    j.sections.push(egui::text::LayoutSection {
        leading_space: 0.0,
        byte_range: start..len,
        format: TextFormat::simple(font_id.clone(), theme.code),
    });
    j.wrap_width = wrap_width;
    j
}
