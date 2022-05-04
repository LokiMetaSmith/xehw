use eframe::egui;
use eframe::egui::text::TextFormat;
use xeh::prelude::*;
use crate::style::*;

pub fn code_layouter(
    text: &str,
    err: Option<&Xsubstr>,
    dbg: Option<&Xsubstr>,
    font_id: &egui::FontId,
    wrap_width: f32,
    res: &mut Vec<(usize,usize,char)>,
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
        res.push((err_start,err_end,'E'));
    }
    if let Some(s) = dbg {
        let dbg_start = s.range().start.min(len);
        let dbg_end = s.range().end.min(len);
        res.push((dbg_start,dbg_end,'D'));
        let r = err_start..err_end;
        if !(r.contains(&dbg_start) || r.contains(&dbg_end)) {
            slst.push((dbg_start, 1));
            slst.push((dbg_end, 1));
        }
    }
    slst.sort_by(|a,b| {
        a.0.cmp(&b.0).then(a.1.cmp(&b.1))
    });
    let mut start = 0;
    let mut it = slst.into_iter();
    while let Some((p1, dbg)) = it.next() {
        j.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: start..p1,
            format: TextFormat::simple(font_id.clone(), CODE_FG),
        });
        res.push((start,p1,' '));
        let (p2, _) = it.next().unwrap();
        j.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: p1..p2,
            format: TextFormat {
                font_id: font_id.clone(),
                color: CODE_FG,
                background: if dbg==1 { CODE_DBG_BG } else { CODE_ERR_BG },
                ..Default::default()
            },
        });
        res.push((p1,p2,if dbg==1 { 'd' } else { 'e' }));
        start = p2;
    }
    j.sections.push(egui::text::LayoutSection {
        leading_space: 0.0,
        byte_range: start..len,
        format: TextFormat::simple(font_id.clone(), CODE_FG),
    });
    res.push((start,len,' '));
    j.wrap_width = wrap_width;
    j
}
