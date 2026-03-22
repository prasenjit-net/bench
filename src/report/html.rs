use anyhow::Result;
use chrono::Local;
use tera::{Context, Tera};

use crate::stats::ScenarioResult;

const TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1.0"/>
<title>bench — HTTP Benchmark Report</title>
<style>
  :root {
    --bg: #0f1117; --surface: #1a1d27; --border: #2d3148;
    --accent: #5c6bc0; --green: #4caf50; --red: #ef5350;
    --yellow: #ffa726; --text: #e8eaf6; --muted: #9fa8da;
    --font: 'Segoe UI', system-ui, sans-serif;
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { background: var(--bg); color: var(--text); font-family: var(--font); padding: 2rem; }
  h1 { font-size: 1.8rem; color: var(--accent); margin-bottom: .25rem; }
  .meta { color: var(--muted); font-size: .85rem; margin-bottom: 2.5rem; }
  .scenario { background: var(--surface); border: 1px solid var(--border); border-radius: 10px;
              padding: 1.5rem; margin-bottom: 2.5rem; }
  .scenario-header { display: flex; align-items: baseline; gap: 1rem; margin-bottom: 1.25rem; }
  .scenario-header h2 { font-size: 1.2rem; color: var(--text); }
  .tag { background: var(--accent); color: #fff; border-radius: 4px; padding: .2rem .6rem;
         font-size: .75rem; font-weight: 600; }
  .tag.get { background: #2e7d32; } .tag.post { background: #1565c0; }
  .tag.put { background: #e65100; } .tag.delete { background: #b71c1c; }
  .tag.patch { background: #6a1b9a; }
  .url { color: var(--muted); font-size: .85rem; word-break: break-all; margin-bottom: 1rem; }
  .kv-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: .75rem;
             margin-bottom: 1.5rem; }
  .kv { background: var(--bg); border-radius: 6px; padding: .75rem; }
  .kv .label { font-size: .7rem; text-transform: uppercase; color: var(--muted); margin-bottom: .25rem; }
  .kv .value { font-size: 1.1rem; font-weight: 600; }
  .value.green { color: var(--green); } .value.red { color: var(--red); }
  .value.yellow { color: var(--yellow); }
  .charts { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; margin-bottom: 1.5rem; }
  @media (max-width: 900px) { .charts { grid-template-columns: 1fr; } }
  .chart-box { background: var(--bg); border-radius: 8px; padding: 1rem; }
  .chart-box h3 { font-size: .85rem; color: var(--muted); margin-bottom: .75rem;
                  text-transform: uppercase; letter-spacing: .05em; }
  .bar-row { display: flex; align-items: center; margin-bottom: .45rem; font-size: .8rem; }
  .bar-label { width: 90px; flex-shrink: 0; text-align: right; padding-right: .5rem; color: var(--muted); }
  .bar-track { flex: 1; background: var(--border); border-radius: 3px; height: 16px; overflow: hidden; }
  .bar-fill { height: 100%; border-radius: 3px; transition: width .3s; }
  .bar-count { width: 60px; text-align: right; padding-left: .5rem; color: var(--muted); }
  .latency-table { width: 100%; border-collapse: collapse; font-size: .82rem; margin-top: .5rem; }
  .latency-table th, .latency-table td { padding: .4rem .7rem; text-align: right; border-bottom: 1px solid var(--border); }
  .latency-table th { color: var(--muted); font-weight: 500; text-align: left; }
  .latency-table td:first-child { text-align: left; color: var(--muted); }
  .error-table { width: 100%; border-collapse: collapse; font-size: .82rem; }
  .error-table th, .error-table td { padding: .4rem .7rem; border-bottom: 1px solid var(--border); }
  .error-table th { color: var(--muted); }
  .section-title { font-size: .8rem; text-transform: uppercase; color: var(--muted);
                   letter-spacing: .06em; margin: 1.25rem 0 .5rem; }
  footer { text-align: center; color: var(--muted); font-size: .75rem; margin-top: 3rem; }
</style>
</head>
<body>
<h1>🚀 HTTP Benchmark Report</h1>
<p class="meta">Generated {{ generated_at }} &nbsp;·&nbsp; {{ scenario_count }} scenario(s)</p>

{% for s in scenarios %}
<div class="scenario">
  <div class="scenario-header">
    <h2>{{ s.name }}</h2>
    <span class="tag {{ s.method | lower }}">{{ s.method }}</span>
  </div>
  <div class="url">{{ s.url }}</div>

  <div class="kv-grid">
    <div class="kv"><div class="label">Total Requests</div>
      <div class="value">{{ s.total_requests }}</div></div>
    <div class="kv"><div class="label">Throughput</div>
      <div class="value">{{ s.throughput_rps | round(precision=1) }} req/s</div></div>
    <div class="kv"><div class="label">Duration</div>
      <div class="value">{{ s.duration_secs | round(precision=2) }}s</div></div>
    <div class="kv"><div class="label">Concurrency</div>
      <div class="value">{{ s.concurrency }}</div></div>
    <div class="kv"><div class="label">Successful</div>
      <div class="value green">{{ s.successful_requests }}</div></div>
    <div class="kv"><div class="label">Failed (4xx/5xx)</div>
      <div class="value {% if s.failed_requests > 0 %}red{% else %}green{% endif %}">{{ s.failed_requests }}</div></div>
    <div class="kv"><div class="label">Errors (network)</div>
      <div class="value {% if s.error_requests > 0 %}red{% else %}green{% endif %}">{{ s.error_requests }}</div></div>
    <div class="kv"><div class="label">Success Rate</div>
      <div class="value {% if s.success_rate >= 99.0 %}green{% elif s.success_rate >= 90.0 %}yellow{% else %}red{% endif %}">
        {{ s.success_rate | round(precision=1) }}%</div></div>
  </div>

  <div class="charts">
    <!-- Latency histogram -->
    <div class="chart-box">
      <h3>Latency Distribution</h3>
      {% for bucket in s.latency_histogram %}
      <div class="bar-row">
        <div class="bar-label">{{ bucket.0 }}</div>
        <div class="bar-track">
          <div class="bar-fill" style="width:{{ bucket.1 * 100 / s.latency_chart_max }}%;background:#5c6bc0;"></div>
        </div>
        <div class="bar-count">{{ bucket.1 }}</div>
      </div>
      {% endfor %}
    </div>

    <!-- Status code distribution -->
    <div class="chart-box">
      <h3>Status Code Distribution</h3>
      {% for pair in s.status_distribution_sorted %}
      <div class="bar-row">
        <div class="bar-label">HTTP {{ pair.0 }}</div>
        <div class="bar-track">
          <div class="bar-fill" style="width:{{ pair.1 * 100 / s.status_chart_max }}%;background:{% if pair.0 < 300 %}#4caf50{% elif pair.0 < 400 %}#ffa726{% elif pair.0 < 500 %}#ef5350{% else %}#b71c1c{% endif %};"></div>
        </div>
        <div class="bar-count">{{ pair.1 }}</div>
      </div>
      {% endfor %}
      {% if s.error_requests > 0 %}
      <div class="bar-row">
        <div class="bar-label">Network Err</div>
        <div class="bar-track">
          <div class="bar-fill" style="width:{{ s.error_requests * 100 / s.status_chart_max }}%;background:#78909c;"></div>
        </div>
        <div class="bar-count">{{ s.error_requests }}</div>
      </div>
      {% endif %}
    </div>
  </div>

  <!-- Latency percentiles table -->
  <div class="section-title">Latency Percentiles</div>
  <table class="latency-table">
    <thead><tr><th>Metric</th><th>Min</th><th>Mean</th><th>Std Dev</th>
      <th>p50</th><th>p75</th><th>p90</th><th>p95</th><th>p99</th><th>p99.9</th><th>Max</th></tr></thead>
    <tbody><tr>
      <td>ms</td>
      <td>{{ s.latency_min_ms | round(precision=2) }}</td>
      <td>{{ s.latency_mean_ms | round(precision=2) }}</td>
      <td>{{ s.latency_stddev_ms | round(precision=2) }}</td>
      <td>{{ s.latency_p50_ms | round(precision=2) }}</td>
      <td>{{ s.latency_p75_ms | round(precision=2) }}</td>
      <td>{{ s.latency_p90_ms | round(precision=2) }}</td>
      <td>{{ s.latency_p95_ms | round(precision=2) }}</td>
      <td>{{ s.latency_p99_ms | round(precision=2) }}</td>
      <td>{{ s.latency_p999_ms | round(precision=2) }}</td>
      <td>{{ s.latency_max_ms | round(precision=2) }}</td>
    </tr></tbody>
  </table>

  <!-- Throughput timeline -->
  {% if s.timeline | length > 1 %}
  <div class="section-title">Throughput Timeline (req/s per second)</div>
  <div class="chart-box" style="margin-top:.25rem;">
    {% for point in s.timeline %}
    <div class="bar-row">
      <div class="bar-label">{{ point.0 }}s</div>
      <div class="bar-track">
        <div class="bar-fill" style="width:{{ point.1 * 100 / s.timeline_chart_max }}%;background:#26a69a;"></div>
      </div>
      <div class="bar-count">{{ point.1 }}</div>
    </div>
    {% endfor %}
  </div>
  {% endif %}

  <!-- Error breakdown -->
  {% if s.error_distribution_sorted | length > 0 %}
  <div class="section-title">Error Breakdown</div>
  <table class="error-table">
    <thead><tr><th>Error Type</th><th>Count</th></tr></thead>
    <tbody>
    {% for pair in s.error_distribution_sorted %}
    <tr><td>{{ pair.0 }}</td><td>{{ pair.1 }}</td></tr>
    {% endfor %}
    </tbody>
  </table>
  {% endif %}
</div>
{% endfor %}

<footer>Generated by <strong>bench</strong> — HTTP Benchmarking Tool</footer>
</body>
</html>
"#;

/// A view-model for a scenario result enriched with sorted lists for the template
#[derive(serde::Serialize)]
struct ScenarioView<'a> {
    name: &'a str,
    url: &'a str,
    method: &'a str,
    concurrency: usize,
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    error_requests: u64,
    duration_secs: f64,
    throughput_rps: f64,
    success_rate: f64,
    latency_min_ms: f64,
    latency_max_ms: f64,
    latency_mean_ms: f64,
    latency_stddev_ms: f64,
    latency_p50_ms: f64,
    latency_p75_ms: f64,
    latency_p90_ms: f64,
    latency_p95_ms: f64,
    latency_p99_ms: f64,
    latency_p999_ms: f64,
    status_distribution_sorted: Vec<(u16, u64)>,
    error_distribution_sorted: Vec<(String, u64)>,
    timeline: &'a Vec<(u64, u64)>,
    latency_histogram: &'a Vec<(String, u64)>,
    /// Pre-computed chart max values (safe for Tera integer math)
    status_chart_max: u64,
    latency_chart_max: u64,
    timeline_chart_max: u64,
}

pub fn generate(results: &[ScenarioResult], output_path: &str) -> Result<()> {
    let mut tera = Tera::default();
    tera.add_raw_template("report", TEMPLATE)?;

    let mut ctx = Context::new();
    ctx.insert("generated_at", &Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
    ctx.insert("scenario_count", &results.len());

    let views: Vec<ScenarioView> = results
        .iter()
        .map(|r| {
            let mut status_sorted: Vec<(u16, u64)> =
                r.status_distribution.iter().map(|(&k, &v)| (k, v)).collect();
            status_sorted.sort_by_key(|(k, _)| *k);

            let mut error_sorted: Vec<(String, u64)> =
                r.error_distribution.iter().map(|(k, &v)| (k.clone(), v)).collect();
            error_sorted.sort_by(|a, b| b.1.cmp(&a.1));

            // Pre-compute chart max values in Rust to avoid Tera math issues on empty arrays
            let status_chart_max = status_sorted
                .iter()
                .map(|(_, c)| *c)
                .max()
                .unwrap_or(0)
                .max(r.error_requests)
                .max(1);

            let latency_chart_max = r
                .latency_histogram
                .iter()
                .map(|(_, c)| *c)
                .max()
                .unwrap_or(1)
                .max(1);

            let timeline_chart_max = r
                .timeline
                .iter()
                .map(|(_, c)| *c)
                .max()
                .unwrap_or(1)
                .max(1);

            ScenarioView {
                name: &r.name,
                url: &r.url,
                method: &r.method,
                concurrency: r.concurrency,
                total_requests: r.total_requests,
                successful_requests: r.successful_requests,
                failed_requests: r.failed_requests,
                error_requests: r.error_requests,
                duration_secs: r.duration_secs,
                throughput_rps: r.throughput_rps,
                success_rate: r.success_rate(),
                latency_min_ms: r.latency_min_ms,
                latency_max_ms: r.latency_max_ms,
                latency_mean_ms: r.latency_mean_ms,
                latency_stddev_ms: r.latency_stddev_ms,
                latency_p50_ms: r.latency_p50_ms,
                latency_p75_ms: r.latency_p75_ms,
                latency_p90_ms: r.latency_p90_ms,
                latency_p95_ms: r.latency_p95_ms,
                latency_p99_ms: r.latency_p99_ms,
                latency_p999_ms: r.latency_p999_ms,
                status_distribution_sorted: status_sorted,
                error_distribution_sorted: error_sorted,
                timeline: &r.timeline,
                latency_histogram: &r.latency_histogram,
                status_chart_max,
                latency_chart_max,
                timeline_chart_max,
            }
        })
        .collect();

    ctx.insert("scenarios", &views);

    let html = tera.render("report", &ctx)?;
    std::fs::write(output_path, html)?;

    Ok(())
}
