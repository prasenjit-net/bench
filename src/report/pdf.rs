use anyhow::Result;
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;
use chrono::Local;

use crate::report::ScenarioGroup;
use crate::stats::ScenarioResult;

const PAGE_W: f32 = 210.0; // A4 mm
const PAGE_H: f32 = 297.0;
const MARGIN: f32 = 20.0;
const LINE_H: f32 = 7.0;

struct PdfPage {
    layer: PdfLayerReference,
    font_bold: IndirectFontRef,
    font: IndirectFontRef,
    y: f32,
    doc: PdfDocumentReference,
    page_num: usize,
}

impl PdfPage {
    fn check_new_page(&mut self, needed: f32) {
        if self.y - needed < MARGIN {
            let (page_idx, layer_idx) = self.doc.add_page(
                Mm(PAGE_W),
                Mm(PAGE_H),
                format!("Page {}", self.page_num + 1),
            );
            self.layer = self.doc.get_page(page_idx).get_layer(layer_idx);
            self.y = PAGE_H - MARGIN;
            self.page_num += 1;
        }
    }

    fn text(&mut self, text: &str, x: f32, size: f32, bold: bool) {
        self.check_new_page(LINE_H);
        let font = if bold { &self.font_bold } else { &self.font };
        self.layer.use_text(text, size, Mm(x), Mm(self.y), font);
    }

    fn newline(&mut self) {
        self.y -= LINE_H;
    }

    fn newline_n(&mut self, n: usize) {
        self.y -= LINE_H * n as f32;
    }

    fn hline(&mut self, x1: f32, x2: f32) {
        self.check_new_page(3.0);
        let line = Line {
            points: vec![
                (Point::new(Mm(x1), Mm(self.y)), false),
                (Point::new(Mm(x2), Mm(self.y)), false),
            ],
            is_closed: false,
        };
        self.layer.add_line(line);
        self.y -= 2.0;
    }

    fn bar(&mut self, x: f32, value: u64, max_value: u64, width: f32, label: &str, count_str: &str) {
        self.check_new_page(LINE_H);
        let fill_w = if max_value > 0 {
            (value as f32 / max_value as f32) * width
        } else {
            0.0
        };

        // label
        self.layer
            .use_text(label, 7.0, Mm(x), Mm(self.y), &self.font);

        let bar_x = x + 30.0;
        if fill_w > 0.0 {
            let rect = Rect::new(
                Mm(bar_x),
                Mm(self.y - 0.5),
                Mm(bar_x + fill_w),
                Mm(self.y + 4.5),
            );
            self.layer.set_fill_color(Color::Rgb(Rgb::new(0.36, 0.42, 0.75, None)));
            self.layer.add_rect(rect);
            self.layer.set_fill_color(Color::Rgb(Rgb::new(0.9, 0.9, 0.9, None)));
        }

        self.layer
            .use_text(count_str, 7.0, Mm(bar_x + width + 2.0), Mm(self.y), &self.font);

        self.y -= LINE_H;
    }
}

pub fn generate(groups: &[ScenarioGroup<'_>], output_path: &str) -> Result<()> {
    let (doc, page1, layer1) = PdfDocument::new(
        "HTTP Benchmark Report",
        Mm(PAGE_W),
        Mm(PAGE_H),
        "Page 1",
    );

    let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let layer = doc.get_page(page1).get_layer(layer1);

    let mut page = PdfPage {
        layer,
        font: font.clone(),
        font_bold: font_bold.clone(),
        y: PAGE_H - MARGIN,
        doc,
        page_num: 1,
    };

    // Title
    page.text("HTTP Benchmark Report", MARGIN, 18.0, true);
    page.newline_n(2);

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let total_steps: usize = groups.iter().map(|g| g.results.len()).sum();
    page.text(
        &format!("Generated: {}   Scenarios: {}   Steps: {}", now, groups.len(), total_steps),
        MARGIN, 9.0, false,
    );
    page.newline_n(2);
    page.hline(MARGIN, PAGE_W - MARGIN);
    page.newline();

    for group in groups {
        // Scenario group heading
        page.text(&format!("◆ {}", group.name), MARGIN, 14.0, true);
        page.newline();
        page.text(
            &format!("  {}  ·  concurrency {}", group.run_desc, group.concurrency),
            MARGIN, 8.0, false,
        );
        page.newline_n(2);

        for result in &group.results {
            render_step(&mut page, result)?;
            page.newline();
        }

        page.hline(MARGIN, PAGE_W - MARGIN);
        page.newline();
    }

    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);
    page.doc.save(&mut writer)?;

    Ok(())
}

fn render_step(page: &mut PdfPage, r: &ScenarioResult) -> Result<()> {
    // Scenario heading
    page.text(&format!("▶  {}  [{}]", r.name, r.method), MARGIN, 13.0, true);
    page.newline_n(2);
    page.text(&r.url, MARGIN, 8.0, false);
    page.newline_n(2);

    // Key metrics row 1
    page.text(
        &format!(
            "Total: {}   Throughput: {:.1} req/s   Duration: {:.2}s   Concurrency: {}",
            r.total_requests, r.throughput_rps, r.duration_secs, r.concurrency
        ),
        MARGIN,
        9.0,
        false,
    );
    page.newline();

    page.text(
        &format!(
            "Successful: {}   Failed (4xx/5xx): {}   Network Errors: {}   Success Rate: {:.1}%",
            r.successful_requests,
            r.failed_requests,
            r.error_requests,
            r.success_rate()
        ),
        MARGIN,
        9.0,
        false,
    );
    page.newline_n(2);

    // Latency table header
    page.text("Latency (ms)", MARGIN, 10.0, true);
    page.newline();
    page.text(
        "Min       Mean      StdDev    p50       p75       p90       p95       p99       p99.9     Max",
        MARGIN,
        8.0,
        false,
    );
    page.newline();
    page.text(
        &format!(
            "{:<10.2}{:<10.2}{:<10.2}{:<10.2}{:<10.2}{:<10.2}{:<10.2}{:<10.2}{:<10.2}{:<10.2}",
            r.latency_min_ms,
            r.latency_mean_ms,
            r.latency_stddev_ms,
            r.latency_p50_ms,
            r.latency_p75_ms,
            r.latency_p90_ms,
            r.latency_p95_ms,
            r.latency_p99_ms,
            r.latency_p999_ms,
            r.latency_max_ms,
        ),
        MARGIN,
        8.0,
        false,
    );
    page.newline_n(2);

    // Latency histogram bars
    page.text("Latency Distribution", MARGIN, 10.0, true);
    page.newline();
    let max_lat = r.latency_histogram.iter().map(|(_, c)| *c).max().unwrap_or(1);
    for (label, count) in &r.latency_histogram {
        page.bar(MARGIN, *count, max_lat, 80.0, label, &count.to_string());
    }
    page.newline();

    // Status distribution
    page.text("Status Code Distribution", MARGIN, 10.0, true);
    page.newline();
    let mut status_sorted: Vec<(u16, u64)> = r
        .status_distribution
        .iter()
        .map(|(&k, &v)| (k, v))
        .collect();
    status_sorted.sort_by_key(|(k, _)| *k);
    let max_sc = status_sorted.iter().map(|(_, c)| *c).max().unwrap_or(1);
    for (code, count) in &status_sorted {
        page.bar(
            MARGIN,
            *count,
            max_sc,
            80.0,
            &format!("HTTP {}", code),
            &count.to_string(),
        );
    }
    page.newline();

    // Error breakdown
    if !r.error_distribution.is_empty() {
        page.text("Network Error Breakdown", MARGIN, 10.0, true);
        page.newline();
        for (err, count) in &r.error_distribution {
            page.text(
                &format!("  {}: {}", err, count),
                MARGIN,
                8.0,
                false,
            );
            page.newline();
        }
    }

    Ok(())
}
