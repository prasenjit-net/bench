use std::collections::HashMap;
use std::time::Duration;

use hdrhistogram::Histogram;
use serde::Serialize;

/// Per-request outcome from the runner
#[derive(Debug)]
pub struct RequestOutcome {
    /// Latency in microseconds
    pub latency_us: u64,
    pub status_code: Option<u16>,
    /// Error kind string if the request failed at transport level
    pub error: Option<String>,
    /// Milliseconds since test-run start (for timeline chart)
    pub offset_ms: u64,
}

/// Aggregated statistics for one leaf request node across all iterations
#[derive(Debug, Serialize)]
pub struct ScenarioResult {
    pub name: String,
    pub url: String,
    pub method: String,
    pub concurrency: usize,

    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub error_requests: u64,

    pub duration_secs: f64,
    pub throughput_rps: f64,

    // Latency in milliseconds
    pub latency_min_ms: f64,
    pub latency_max_ms: f64,
    pub latency_mean_ms: f64,
    pub latency_stddev_ms: f64,
    pub latency_p50_ms: f64,
    pub latency_p75_ms: f64,
    pub latency_p90_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_p99_ms: f64,
    pub latency_p999_ms: f64,

    /// HTTP status code → count
    pub status_distribution: HashMap<u16, u64>,
    /// Error kind → count
    pub error_distribution: HashMap<String, u64>,

    /// Timeline: (second_bucket, req_count) for throughput chart
    pub timeline: Vec<(u64, u64)>,

    /// Histogram buckets: (bucket_label_ms, count) for latency chart
    pub latency_histogram: Vec<(String, u64)>,
}

impl ScenarioResult {
    /// Build stats from a flat list of per-request outcomes.
    /// `name`, `url`, `method` come from the leaf RequestNode;
    /// `elapsed` is the total wall-clock duration of the whole test run.
    pub fn from_outcomes(
        name: &str,
        url: &str,
        method: &str,
        concurrency: usize,
        outcomes: Vec<RequestOutcome>,
        elapsed: Duration,
    ) -> Self {
        let total = outcomes.len() as u64;
        let duration_secs = elapsed.as_secs_f64();

        let mut histogram: Histogram<u64> =
            Histogram::new_with_max(60_000_000, 3).unwrap_or_else(|_| Histogram::new(3).unwrap());

        let mut status_distribution: HashMap<u16, u64> = HashMap::new();
        let mut error_distribution: HashMap<String, u64> = HashMap::new();
        let mut timeline_map: HashMap<u64, u64> = HashMap::new();

        let mut successful = 0u64;
        let mut failed = 0u64;
        let mut errors = 0u64;

        let mut latencies_us: Vec<u64> = Vec::with_capacity(outcomes.len());

        for outcome in &outcomes {
            let bucket = outcome.offset_ms / 1000;
            *timeline_map.entry(bucket).or_insert(0) += 1;

            if let Some(err) = &outcome.error {
                errors += 1;
                *error_distribution.entry(err.clone()).or_insert(0) += 1;
            } else if let Some(code) = outcome.status_code {
                *status_distribution.entry(code).or_insert(0) += 1;
                if code < 400 {
                    successful += 1;
                } else {
                    failed += 1;
                }
            }

            // Record latency (cap at histogram max)
            let lat = outcome.latency_us.min(60_000_000);
            let _ = histogram.record(lat);
            latencies_us.push(outcome.latency_us);
        }

        // Latency stats
        let (min_us, max_us, mean_us, stddev_us) = if latencies_us.is_empty() {
            (0, 0, 0.0, 0.0)
        } else {
            let min = *latencies_us.iter().min().unwrap();
            let max = *latencies_us.iter().max().unwrap();
            let sum: u64 = latencies_us.iter().sum();
            let mean = sum as f64 / latencies_us.len() as f64;
            let variance = latencies_us
                .iter()
                .map(|&x| {
                    let diff = x as f64 - mean;
                    diff * diff
                })
                .sum::<f64>()
                / latencies_us.len() as f64;
            (min, max, mean, variance.sqrt())
        };

        let us_to_ms = |us: u64| us as f64 / 1000.0;
        let hdr_ms = |p: f64| histogram.value_at_quantile(p) as f64 / 1000.0;

        // Timeline: fill gaps with zero
        let max_bucket = timeline_map.keys().copied().max().unwrap_or(0);
        let timeline: Vec<(u64, u64)> = (0..=max_bucket)
            .map(|b| (b, *timeline_map.get(&b).unwrap_or(&0)))
            .collect();

        // Latency histogram buckets (log-ish)
        let latency_histogram = build_latency_histogram(&histogram);

        ScenarioResult {
            name: name.to_string(),
            url: url.to_string(),
            method: method.to_string(),
            concurrency,
            total_requests: total,
            successful_requests: successful,
            failed_requests: failed,
            error_requests: errors,
            duration_secs,
            throughput_rps: if duration_secs > 0.0 {
                total as f64 / duration_secs
            } else {
                0.0
            },
            latency_min_ms: us_to_ms(min_us),
            latency_max_ms: us_to_ms(max_us),
            latency_mean_ms: mean_us / 1000.0,
            latency_stddev_ms: stddev_us / 1000.0,
            latency_p50_ms: hdr_ms(0.50),
            latency_p75_ms: hdr_ms(0.75),
            latency_p90_ms: hdr_ms(0.90),
            latency_p95_ms: hdr_ms(0.95),
            latency_p99_ms: hdr_ms(0.99),
            latency_p999_ms: hdr_ms(0.999),
            status_distribution,
            error_distribution,
            timeline,
            latency_histogram,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.successful_requests as f64 / self.total_requests as f64 * 100.0
    }
}

fn build_latency_histogram(hist: &Histogram<u64>) -> Vec<(String, u64)> {
    // Define bucket boundaries in microseconds
    let boundaries: &[u64] = &[
        500, 1_000, 2_000, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000,
        1_000_000, 2_500_000, 5_000_000, 10_000_000,
    ];

    let mut buckets: Vec<(String, u64)> = Vec::new();
    let mut prev = 0u64;

    for &boundary in boundaries {
        let count = hist.count_between(prev, boundary);
        if count > 0 || !buckets.is_empty() {
            let label = if boundary < 1_000 {
                format!("≤{}µs", boundary)
            } else if boundary < 1_000_000 {
                format!("≤{}ms", boundary / 1_000)
            } else {
                format!("≤{}s", boundary / 1_000_000)
            };
            buckets.push((label, count));
        }
        prev = boundary + 1;
    }

    // overflow bucket
    let overflow = hist.count_between(prev, u64::MAX);
    if overflow > 0 {
        buckets.push((">10s".to_string(), overflow));
    }

    // Remove leading zero buckets
    while buckets.first().map(|(_, c)| *c == 0).unwrap_or(false) {
        buckets.remove(0);
    }

    buckets
}
