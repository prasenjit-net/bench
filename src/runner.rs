use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio::sync::Mutex;

use crate::config::{RunParams, Scenario, Step};
use crate::stats::{RequestOutcome, ScenarioResult};

/// Run the scenario according to `run` parameters.
///
/// `concurrency` workers each execute the full scenario (all steps in order)
/// for their share of total runs. Stats are collected per step name.
pub async fn run(scenario: &Scenario, run: &RunParams) -> Result<Vec<ScenarioResult>> {
    let client = Arc::new(
        Client::builder()
            .timeout(Duration::from_millis(run.timeout_ms))
            .use_rustls_tls()
            .build()?,
    );

    // outcomes: step name → collected outcomes across all runs × workers
    let outcomes: Arc<Mutex<HashMap<String, Vec<RequestOutcome>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let total_runs = run.requests;
    let duration_limit = run.duration_secs.map(Duration::from_secs);

    let pb = match total_runs {
        Some(n) => {
            let pb = ProgressBar::new(n);
            pb.set_style(
                ProgressStyle::with_template(
                    "  [{elapsed_precise}] [{bar:50.cyan/blue}] {pos}/{len} runs  ({per_sec})",
                )
                .unwrap()
                .progress_chars("=>-"),
            );
            pb
        }
        None => {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::with_template(
                    "  [{elapsed_precise}] {spinner}  {pos} runs  ({per_sec})",
                )
                .unwrap(),
            );
            pb
        }
    };
    let pb = Arc::new(pb);
    let start = Instant::now();

    if let Some(total) = total_runs {
        // Distribute total runs evenly across workers
        let chunk = (total + run.concurrency as u64 - 1) / run.concurrency as u64;
        let tasks: Vec<_> = (0..run.concurrency)
            .map(|i| {
                let client = Arc::clone(&client);
                let outcomes = Arc::clone(&outcomes);
                let pb = Arc::clone(&pb);
                let steps = scenario.steps.clone();
                let worker_start = i as u64 * chunk;
                let worker_end = (worker_start + chunk).min(total);
                let count = worker_end.saturating_sub(worker_start);
                let timeout_ms = run.timeout_ms;

                tokio::spawn(async move {
                    for _ in 0..count {
                        execute_steps_once(&steps, &client, &outcomes, start, timeout_ms).await;
                        pb.inc(1);
                    }
                })
            })
            .collect();
        futures::future::join_all(tasks).await;
    } else {
        // Run until deadline
        let deadline = start + duration_limit.unwrap();
        let tasks: Vec<_> = (0..run.concurrency)
            .map(|_| {
                let client = Arc::clone(&client);
                let outcomes = Arc::clone(&outcomes);
                let pb = Arc::clone(&pb);
                let steps = scenario.steps.clone();
                let timeout_ms = run.timeout_ms;

                tokio::spawn(async move {
                    loop {
                        if Instant::now() >= deadline {
                            break;
                        }
                        execute_steps_once(&steps, &client, &outcomes, start, timeout_ms).await;
                        pb.inc(1);
                    }
                })
            })
            .collect();
        futures::future::join_all(tasks).await;
    }

    pb.finish_and_clear();
    let elapsed = start.elapsed();

    // Build one ScenarioResult per step, in definition order
    let mut map = Arc::try_unwrap(outcomes)
        .expect("all tasks completed")
        .into_inner();

    let results = scenario
        .steps
        .iter()
        .map(|step| {
            let step_outcomes = map.remove(&step.name).unwrap_or_default();
            ScenarioResult::from_outcomes(
                &step.name,
                &step.url,
                &step.method.to_uppercase(),
                run.concurrency,
                step_outcomes,
                elapsed,
            )
        })
        .collect();

    Ok(results)
}

/// Execute all steps of the scenario once, in order.
async fn execute_steps_once(
    steps: &[Step],
    client: &Client,
    outcomes: &Arc<Mutex<HashMap<String, Vec<RequestOutcome>>>>,
    start: Instant,
    _timeout_ms: u64,
) {
    for step in steps {
        let outcome = fire_request(client, step, start).await;
        outcomes.lock().await.entry(step.name.clone()).or_default().push(outcome);
    }
}

async fn fire_request(client: &Client, step: &Step, start: Instant) -> RequestOutcome {
    let req_start = Instant::now();
    let offset_ms = start.elapsed().as_millis() as u64;

    let mut builder = client.request(
        step.method.to_uppercase().parse().unwrap_or(reqwest::Method::GET),
        &step.url,
    );
    for (k, v) in &step.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }
    if let Some(body) = &step.body {
        builder = builder.body(body.clone());
    }

    match builder.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let _ = resp.bytes().await;
            RequestOutcome {
                latency_us: req_start.elapsed().as_micros() as u64,
                status_code: Some(status),
                error: None,
                offset_ms,
            }
        }
        Err(e) => {
            let msg = if e.is_timeout() { "timeout" }
                      else if e.is_connect() { "connection error" }
                      else { "request error" };
            RequestOutcome {
                latency_us: req_start.elapsed().as_micros() as u64,
                status_code: None,
                error: Some(msg.to_string()),
                offset_ms,
            }
        }
    }
}
