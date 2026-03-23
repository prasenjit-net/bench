use anyhow::Result;
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;
use chrono::Local;

use crate::report::ScenarioGroup;
use crate::stats::ScenarioResult;

// ── Page geometry ─────────────────────────────────────────────────────────────
const PW: f32 = 210.0;  // A4 width mm
const PH: f32 = 297.0;  // A4 height mm
const ML: f32 = 14.0;   // left margin
const MR: f32 = 14.0;   // right margin
const MB: f32 = 14.0;   // bottom margin (reserved for footer)
const CW: f32 = PW - ML - MR; // content width = 182mm

// ── Palette (RGB 0.0–1.0) ────────────────────────────────────────────────────
type Rgb3 = (f32, f32, f32);
const INDIGO:       Rgb3 = (0.310, 0.275, 0.898); // #4f46e5
const INDIGO_DARK:  Rgb3 = (0.231, 0.184, 0.792); // header bg
const INDIGO_LITE:  Rgb3 = (0.906, 0.910, 0.992); // #e7e8fd card bg
const SUCCESS:      Rgb3 = (0.086, 0.639, 0.290); // #16a34a green-600
const SUCCESS_LITE: Rgb3 = (0.863, 0.969, 0.902); // #dcfce7
const ERROR:        Rgb3 = (0.863, 0.149, 0.149); // #dc2626 red-600
const ERROR_LITE:   Rgb3 = (0.996, 0.878, 0.878); // #fee2e2
const WARN:         Rgb3 = (0.851, 0.467, 0.024); // #d97706 amber-600
const WARN_LITE:    Rgb3 = (0.996, 0.941, 0.843); // #fef3cd
const TEXT:         Rgb3 = (0.059, 0.090, 0.165); // #0f172a slate-900
const MUTED:        Rgb3 = (0.392, 0.455, 0.545); // #64748b slate-500
const BORDER:       Rgb3 = (0.796, 0.835, 0.882); // #cbd5e1 slate-300
const BG_LIGHT:     Rgb3 = (0.945, 0.961, 0.976); // #f1f5f9 slate-100
const WHITE:        Rgb3 = (1.0, 1.0, 1.0);

// ── Rendering context ────────────────────────────────────────────────────────
struct Ctx {
    doc:      PdfDocumentReference,
    layer:    PdfLayerReference,
    font:     IndirectFontRef,
    bold:     IndirectFontRef,
    y:        f32,   // current Y (PDF coords: Mm from bottom)
    page_num: usize,
}

impl Ctx {
    fn new(doc: PdfDocumentReference, layer: PdfLayerReference,
           font: IndirectFontRef, bold: IndirectFontRef) -> Self {
        Self { doc, layer, font, bold, y: PH - 5.0, page_num: 1 }
    }

    fn ensure(&mut self, needed: f32) {
        if self.y - needed < MB + 8.0 { self.new_page(); }
    }

    fn new_page(&mut self) {
        self.page_num += 1;
        let (pi, li) = self.doc.add_page(Mm(PW), Mm(PH), format!("Page {}", self.page_num));
        self.layer = self.doc.get_page(pi).get_layer(li);
        self.y = PH - 8.0;
    }

    // ── Primitives ──────────────────────────────────────────────────────────

    fn fill(&self, x: f32, yb: f32, w: f32, h: f32, (r, g, b): Rgb3) {
        self.layer.set_fill_color(Color::Rgb(Rgb::new(r, g, b, None)));
        self.layer.add_rect(Rect::new(Mm(x), Mm(yb), Mm(x + w), Mm(yb + h)));
    }

    fn stroke_rect(&self, x: f32, yb: f32, w: f32, h: f32, (r, g, b): Rgb3) {
        self.layer.set_outline_color(Color::Rgb(Rgb::new(r, g, b, None)));
        self.layer.set_outline_thickness(0.3);
        let pts = vec![
            (Point::new(Mm(x),     Mm(yb)),     false),
            (Point::new(Mm(x + w), Mm(yb)),     false),
            (Point::new(Mm(x + w), Mm(yb + h)), false),
            (Point::new(Mm(x),     Mm(yb + h)), false),
        ];
        self.layer.add_line(Line { points: pts, is_closed: true });
    }

    fn hline(&self, x1: f32, x2: f32, y: f32, (r, g, b): Rgb3) {
        self.layer.set_outline_color(Color::Rgb(Rgb::new(r, g, b, None)));
        self.layer.set_outline_thickness(0.3);
        let pts = vec![
            (Point::new(Mm(x1), Mm(y)), false),
            (Point::new(Mm(x2), Mm(y)), false),
        ];
        self.layer.add_line(Line { points: pts, is_closed: false });
    }

    fn text(&self, s: &str, x: f32, y: f32, pt: f32, bold: bool, (r, g, b): Rgb3) {
        let f = if bold { &self.bold } else { &self.font };
        self.layer.set_fill_color(Color::Rgb(Rgb::new(r, g, b, None)));
        self.layer.use_text(s, pt, Mm(x), Mm(y), f);
    }

    fn down(&mut self, mm: f32) { self.y -= mm; }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn success_color(rate: f64) -> Rgb3 {
    if rate >= 99.0 { SUCCESS } else if rate >= 90.0 { WARN } else { ERROR }
}

fn method_color(method: &str) -> Rgb3 {
    match method.to_uppercase().as_str() {
        "GET"    => (0.059, 0.533, 0.278), // green-700
        "POST"   => (0.086, 0.396, 0.690), // blue-700
        "PUT"    => (0.753, 0.400, 0.000), // amber-700
        "PATCH"  => (0.549, 0.082, 0.502), // purple-700
        "DELETE" => (0.741, 0.122, 0.122), // red-700
        _        => MUTED,
    }
}

fn bar_color_for_status(code: u16) -> Rgb3 {
    if code < 300 { SUCCESS } else if code < 400 { INDIGO } else if code < 500 { WARN } else { ERROR }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars { s.to_string() }
    else { format!("{}…", &s[..max_chars - 1]) }
}

fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { result.push(','); }
        result.push(c);
    }
    result.chars().rev().collect()
}

// ── Drawing helpers ──────────────────────────────────────────────────────────

fn draw_page_header(ctx: &mut Ctx, generated_at: &str, group_count: usize) {
    let h = 20.0;
    ctx.fill(0.0, PH - h, PW, h, INDIGO_DARK);
    ctx.fill(ML, PH - h + 3.5, 6.5, 8.0, WHITE);
    ctx.text("B", ML + 1.2, PH - h + 5.0, 10.0, true, INDIGO_DARK);
    ctx.text("bench", ML + 9.0, PH - h + 9.0, 13.0, true, WHITE);
    ctx.text("HTTP Benchmark Report", ML + 9.0, PH - h + 3.5, 8.0, false, (0.8, 0.82, 1.0));
    let meta = format!("Generated {}  ·  {} scenario(s)", generated_at, group_count);
    ctx.text(&meta, PW - MR - 82.0, PH - h + 6.5, 7.5, false, (0.75, 0.78, 1.0));
    ctx.y = PH - h - 6.0;
}

fn draw_summary(ctx: &mut Ctx, groups: &[ScenarioGroup]) {
    let total: u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.total_requests).sum();
    let ok:    u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.successful_requests).sum();
    let fail:  u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.failed_requests).sum();
    let err:   u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.error_requests).sum();
    let rate       = if total == 0 { 100.0f64 } else { ok as f64 / total as f64 * 100.0 };

    let h = 18.0;
    ctx.ensure(h + 6.0);
    let yb = ctx.y - h;
    ctx.fill(ML - 2.0, yb, CW + 4.0, h, BG_LIGHT);
    ctx.stroke_rect(ML - 2.0, yb, CW + 4.0, h, BORDER);

    let cols: &[(&str, String, Rgb3)] = &[
        ("SCENARIOS",  groups.len().to_string(),     INDIGO),
        ("TOTAL REQ",  fmt_num(total),               TEXT),
        ("SUCCESSFUL", fmt_num(ok),                  SUCCESS),
        ("FAILED",     fmt_num(fail),                if fail > 0 { ERROR } else { MUTED }),
        ("NET ERRORS", fmt_num(err),                 if err > 0 { ERROR } else { MUTED }),
        ("SUCCESS RATE", format!("{:.1}%", rate),    success_color(rate)),
    ];
    let col_w = CW / cols.len() as f32;
    for (i, (label, value, color)) in cols.iter().enumerate() {
        let cx = ML + i as f32 * col_w + col_w / 2.0 - 10.0;
        ctx.text(label, cx, yb + h - 5.5, 5.5, false, MUTED);
        ctx.text(value, cx, yb + 4.0, 9.0, true, *color);
    }
    ctx.y = yb - 6.0;
}

fn draw_group_header(ctx: &mut Ctx, name: &str, run_desc: &str, concurrency: usize, step_count: usize) {
    let h = 14.0;
    ctx.ensure(h + 4.0);
    let yb = ctx.y - h;
    ctx.fill(ML - 2.0, yb, 3.5, h, INDIGO);
    ctx.fill(ML + 1.5, yb, CW + 2.5, h, BG_LIGHT);
    ctx.text(name, ML + 6.0, yb + h - 5.5, 11.0, true, TEXT);
    let meta = format!("{}  ·  concurrency {}  ·  {} step(s)", run_desc, concurrency, step_count);
    ctx.text(&meta, ML + 6.0, yb + 2.5, 7.0, false, MUTED);
    ctx.y = yb - 4.0;
}

fn draw_step_header(ctx: &mut Ctx, name: &str, url: &str, method: &str) {
    ctx.ensure(18.0);
    let badge_w = 14.0;
    ctx.fill(ML, ctx.y - 7.5, badge_w, 8.0, method_color(method));
    ctx.text(method, ML + 1.2, ctx.y - 5.5, 7.0, true, WHITE);
    ctx.text(name, ML + badge_w + 3.0, ctx.y - 4.5, 9.5, true, TEXT);
    ctx.down(9.0);
    ctx.text(&truncate(url, 85), ML + badge_w + 3.0, ctx.y - 4.0, 7.0, false, MUTED);
    ctx.down(7.0);
}

fn draw_metric_cards(ctx: &mut Ctx, r: &ScenarioResult) {
    let h = 18.0;
    ctx.ensure(h + 3.0);
    let gap = 3.0;
    let card_w = (CW - gap * 3.0) / 4.0;
    let rate = r.success_rate();

    let cards: &[(&str, String, Rgb3, Rgb3)] = &[
        ("Throughput",   format!("{:.1} req/s", r.throughput_rps), INDIGO,            INDIGO_LITE),
        ("Total",        fmt_num(r.total_requests),                TEXT,              BG_LIGHT),
        ("Success Rate", format!("{:.1}%", rate),                  success_color(rate),
            if rate >= 99.0 { SUCCESS_LITE } else if rate >= 90.0 { WARN_LITE } else { ERROR_LITE }),
        ("Duration",     format!("{:.2}s", r.duration_secs),       TEXT,              BG_LIGHT),
    ];

    let yb = ctx.y - h;
    for (i, (label, value, fg, bg)) in cards.iter().enumerate() {
        let x = ML + i as f32 * (card_w + gap);
        ctx.fill(x, yb, card_w, h, *bg);
        ctx.stroke_rect(x, yb, card_w, h, BORDER);
        ctx.text(label, x + 2.5, yb + h - 5.0, 6.0, false, MUTED);
        ctx.text(value, x + 2.5, yb + 3.5, 8.5, true, *fg);
    }
    ctx.y = yb - 4.0;
}

fn draw_latency_table(ctx: &mut Ctx, r: &ScenarioResult) {
    let row_h = 6.5;
    let table_h = row_h * 2.0 + 2.0;
    ctx.ensure(table_h + 8.0);

    ctx.text("LATENCY PERCENTILES (ms)", ML, ctx.y - 4.5, 6.5, true, MUTED);
    ctx.down(6.0);

    let cols: &[(&str, f64)] = &[
        ("Min",   r.latency_min_ms),  ("Mean",  r.latency_mean_ms),
        ("p50",   r.latency_p50_ms),  ("p75",   r.latency_p75_ms),
        ("p90",   r.latency_p90_ms),  ("p95",   r.latency_p95_ms),
        ("p99",   r.latency_p99_ms),  ("p99.9", r.latency_p999_ms),
        ("Max",   r.latency_max_ms),
    ];
    let col_w = CW / cols.len() as f32;

    let yb_header = ctx.y - row_h;
    ctx.fill(ML, yb_header, CW, row_h, INDIGO_LITE);
    ctx.stroke_rect(ML, yb_header, CW, row_h, BORDER);
    for (i, (label, _)) in cols.iter().enumerate() {
        ctx.text(label, ML + i as f32 * col_w + 1.5, yb_header + 1.8, 6.5, true, INDIGO);
    }

    let yb_data = yb_header - row_h;
    ctx.fill(ML, yb_data, CW, row_h, WHITE);
    ctx.stroke_rect(ML, yb_data, CW, row_h, BORDER);
    let p99 = r.latency_p99_ms;
    for (i, (_, val)) in cols.iter().enumerate() {
        let color = if *val >= p99 * 1.5 { ERROR } else if *val >= p99 { WARN } else { TEXT };
        ctx.text(&format!("{:.2}", val), ML + i as f32 * col_w + 1.5, yb_data + 1.8, 6.5, false, color);
    }
    ctx.y = yb_data - 4.0;
}

fn draw_bar_section(ctx: &mut Ctx, title: &str, bars: &[(&str, u64, Rgb3)]) {
    if bars.is_empty() { return; }
    let bar_h = 5.5;
    let section_h = 7.0 + bars.len() as f32 * bar_h + 2.0;
    ctx.ensure(section_h);

    ctx.text(title, ML, ctx.y - 4.5, 6.5, true, MUTED);
    ctx.down(7.0);

    let max_val = bars.iter().map(|(_, c, _)| *c).max().unwrap_or(1);
    let label_w = 22.0;
    let count_w = 18.0;
    let bar_track_w = CW - label_w - count_w - 4.0;

    for (label, count, color) in bars {
        ctx.ensure(bar_h + 1.0);
        let yb = ctx.y - bar_h;
        let fill_w = (*count as f32 / max_val as f32) * bar_track_w;

        ctx.text(&truncate(label, 14), ML, yb + 1.2, 6.5, false, TEXT);
        ctx.fill(ML + label_w, yb + 1.0, bar_track_w, bar_h - 2.0, BG_LIGHT);
        if fill_w > 0.1 {
            ctx.fill(ML + label_w, yb + 1.0, fill_w, bar_h - 2.0, *color);
        }
        ctx.text(&fmt_num(*count), ML + label_w + bar_track_w + 2.0, yb + 1.2, 6.5, false, MUTED);
        ctx.y = yb;
    }
    ctx.down(3.0);
}

fn draw_footer(ctx: &Ctx) {
    let footer_text = format!("bench — HTTP Benchmark Report  ·  Page {}", ctx.page_num);
    ctx.hline(ML, PW - MR, MB + 4.0, BORDER);
    ctx.text(&footer_text, ML, MB + 1.0, 6.0, false, MUTED);
}

// ── Main entry point ──────────────────────────────────────────────────────────

pub fn generate(groups: &[ScenarioGroup], output_path: &str) -> Result<()> {
    let (doc, page1, layer1) = PdfDocument::new("bench Report", Mm(PW), Mm(PH), "Page 1");
    let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    let bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let layer = doc.get_page(page1).get_layer(layer1);
    let mut ctx = Ctx::new(doc, layer, font, bold);

    let generated_at = Local::now().format("%Y-%m-%d %H:%M").to_string();

    draw_page_header(&mut ctx, &generated_at, groups.len());
    draw_summary(&mut ctx, groups);

    for group in groups {
        ctx.ensure(40.0);
        draw_group_header(&mut ctx, &group.name, &group.run_desc, group.concurrency, group.results.len());

        for result in &group.results {
            ctx.ensure(80.0);
            draw_step_header(&mut ctx, &result.name, &result.url, &result.method);
            ctx.down(2.0);
            draw_metric_cards(&mut ctx, result);
            draw_latency_table(&mut ctx, result);

            // Latency histogram
            let max_lat = result.latency_histogram.iter().map(|(_, c)| *c).max().unwrap_or(1);
            let lat_bars: Vec<(&str, u64, Rgb3)> = result.latency_histogram.iter()
                .map(|(label, count)| {
                    let ratio = *count as f64 / max_lat as f64;
                    let color = if ratio >= 0.5 { INDIGO } else if ratio >= 0.2 { (0.557, 0.553, 0.957) } else { INDIGO_LITE };
                    (label.as_str(), *count, color)
                }).collect();
            draw_bar_section(&mut ctx, "LATENCY DISTRIBUTION", &lat_bars);

            // Status distribution
            let mut status_sorted: Vec<(u16, u64)> =
                result.status_distribution.iter().map(|(&k, &v)| (k, v)).collect();
            status_sorted.sort_by_key(|(k, _)| *k);
            if !status_sorted.is_empty() || result.error_requests > 0 {
                let mut status_bars: Vec<(String, u64, Rgb3)> = status_sorted.iter()
                    .map(|(code, count)| (format!("HTTP {}", code), *count, bar_color_for_status(*code)))
                    .collect();
                if result.error_requests > 0 {
                    status_bars.push(("Net Errors".to_string(), result.error_requests, ERROR));
                }
                let sb_refs: Vec<(&str, u64, Rgb3)> = status_bars.iter()
                    .map(|(l, c, color)| (l.as_str(), *c, *color)).collect();
                draw_bar_section(&mut ctx, "STATUS CODE DISTRIBUTION", &sb_refs);
            }

            // Error breakdown
            if !result.error_distribution.is_empty() {
                let mut err_sorted: Vec<(&str, u64)> =
                    result.error_distribution.iter().map(|(k, &v)| (k.as_str(), v)).collect();
                err_sorted.sort_by(|a, b| b.1.cmp(&a.1));
                let err_bars: Vec<(&str, u64, Rgb3)> = err_sorted.iter()
                    .map(|(l, c)| (*l, *c, ERROR)).collect();
                draw_bar_section(&mut ctx, "ERROR BREAKDOWN", &err_bars);
            }

            ctx.down(4.0);
        }

        ctx.down(4.0);
    }

    draw_footer(&ctx);

    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);
    ctx.doc.save(&mut writer)?;
    Ok(())
}
