use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::config::Scenario;
use crate::stats::{RequestOutcome, ScenarioResult};

pub async fn run_scenario(scenario: &Scenario) -> Result<ScenarioResult> {
    let client = Client::builder()
        .timeout(Duration::from_millis(scenario.timeout_ms))
        .danger_accept_invalid_certs(false)
        .use_rustls_tls()
        .build()?;

    let client = Arc::new(client);
    let outcomes: Arc<Mutex<Vec<RequestOutcome>>> = Arc::new(Mutex::new(Vec::new()));

    // Determine total request count (for progress bar)
    let total_requests = scenario.requests;
    let duration_limit = scenario.duration_secs.map(Duration::from_secs);

    // Setup progress bar
    let pb = if let Some(n) = total_requests {
        let pb = ProgressBar::new(n);
        pb.set_style(
            ProgressStyle::with_template(
                "{msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} req/s: {per_sec}",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        pb.set_message(format!("[{}]", scenario.name));
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{msg} [{elapsed_precise}] {spinner} {pos} requests sent")
                .unwrap(),
        );
        pb.set_message(format!("[{}]", scenario.name));
        pb
    };
    let pb = Arc::new(pb);

    let start_time = Instant::now();

    if let Some(total) = total_requests {
        // Count-based mode: distribute `total` requests across `concurrency` workers
        let chunk = (total + scenario.concurrency as u64 - 1) / scenario.concurrency as u64;

        let tasks: Vec<_> = (0..scenario.concurrency)
            .map(|worker_idx| {
                let client = Arc::clone(&client);
                let outcomes = Arc::clone(&outcomes);
                let pb = Arc::clone(&pb);
                let scenario = scenario.clone();
                let worker_start = worker_idx as u64 * chunk;
                let worker_end = (worker_start + chunk).min(total);
                let count = if worker_end > worker_start {
                    worker_end - worker_start
                } else {
                    0
                };

                tokio::spawn(async move {
                    for _ in 0..count {
                        let outcome =
                            execute_request(&client, &scenario, start_time).await;
                        pb.inc(1);
                        outcomes.lock().await.push(outcome);
                    }
                })
            })
            .collect();

        futures::future::join_all(tasks).await;
    } else {
        let deadline = start_time + duration_limit.unwrap();

        let tasks: Vec<_> = (0..scenario.concurrency)
            .map(|_| {
                let client = Arc::clone(&client);
                let outcomes = Arc::clone(&outcomes);
                let pb = Arc::clone(&pb);
                let scenario = scenario.clone();

                tokio::spawn(async move {
                    loop {
                        if Instant::now() >= deadline {
                            break;
                        }
                        let remaining = deadline.saturating_duration_since(Instant::now());
                        if remaining.is_zero() {
                            break;
                        }
                        let outcome =
                            execute_request(&client, &scenario, start_time).await;
                        pb.inc(1);
                        outcomes.lock().await.push(outcome);
                    }
                })
            })
            .collect();

        futures::future::join_all(tasks).await;
    }

    pb.finish_with_message(format!("[{}] done", scenario.name));

    let total_duration = start_time.elapsed();
    let all_outcomes = Arc::try_unwrap(outcomes)
        .expect("all tasks done")
        .into_inner();

    Ok(ScenarioResult::from_outcomes(
        scenario,
        all_outcomes,
        total_duration,
    ))
}

async fn execute_request(
    client: &Client,
    scenario: &Scenario,
    start: Instant,
) -> RequestOutcome {
    let req_start = Instant::now();
    let offset_ms = start.elapsed().as_millis() as u64;

    let mut request = client.request(
        scenario.method.parse().unwrap_or(reqwest::Method::GET),
        &scenario.url,
    );

    for (k, v) in &scenario.headers {
        request = request.header(k.as_str(), v.as_str());
    }

    if let Some(body) = &scenario.body {
        request = request.body(body.clone());
    }

    let result = timeout(
        Duration::from_millis(scenario.timeout_ms + 100), // slight grace
        request.send(),
    )
    .await;

    let latency_us = req_start.elapsed().as_micros() as u64;

    match result {
        Ok(Ok(response)) => {
            let status = response.status().as_u16();
            // consume body to get accurate timing
            let _ = response.bytes().await;
            let latency_us = req_start.elapsed().as_micros() as u64;
            RequestOutcome {
                latency_us,
                status_code: Some(status),
                error: None,
                offset_ms,
            }
        }
        Ok(Err(e)) => {
            let msg = if e.is_timeout() {
                "timeout".to_string()
            } else if e.is_connect() {
                "connection error".to_string()
            } else {
                "request error".to_string()
            };
            RequestOutcome {
                latency_us,
                status_code: None,
                error: Some(msg),
                offset_ms,
            }
        }
        Err(_) => RequestOutcome {
            latency_us,
            status_code: None,
            error: Some("timeout".to_string()),
            offset_ms,
        },
    }
}
