use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::config::{ExecutionMode, RunParams, Scenario, ScenarioNode};
use crate::stats::{RequestOutcome, ScenarioResult};

// ── Public recursive entry point ─────────────────────────────────────────────

/// Recursively execute a scenario tree node using `run` as global parameters.
/// Sequential groups run their children one after the other;
/// parallel groups run all children concurrently and wait for all to finish.
/// Returns results in execution order (parallel children ordered by their position).
pub fn execute_node<'a>(
    node: &'a ScenarioNode,
    run: &'a RunParams,
    depth: usize,
) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<ScenarioResult>>> + Send + 'a>> {
    Box::pin(async move {
        match node {
            ScenarioNode::Request(req) => {
                let scenario = req.clone().into_scenario(run);
                let result = run_scenario(&scenario).await?;
                Ok(vec![result])
            }

            ScenarioNode::Group(group) => {
                let indent = "  ".repeat(depth);
                match group.mode {
                    ExecutionMode::Sequential => {
                        println!(
                            "{}▶ [sequential] {}  ({} step(s))",
                            indent,
                            group.name,
                            group.steps.len()
                        );
                        let mut results = Vec::new();
                        for step in &group.steps {
                            let mut r = execute_node(step, run, depth + 1).await?;
                            results.append(&mut r);
                        }
                        Ok(results)
                    }
                    ExecutionMode::Parallel => {
                        println!(
                            "{}▶ [parallel]   {}  ({} step(s) running concurrently)",
                            indent,
                            group.name,
                            group.steps.len()
                        );
                        let futures: Vec<_> = group
                            .steps
                            .iter()
                            .map(|step| execute_node(step, run, depth + 1))
                            .collect();
                        let all = futures::future::join_all(futures).await;
                        let mut results = Vec::new();
                        for r in all {
                            results.extend(r?);
                        }
                        Ok(results)
                    }
                }
            }
        }
    })
}

// ── Single scenario runner ────────────────────────────────────────────────────

pub async fn run_scenario(scenario: &Scenario) -> Result<ScenarioResult> {
    let client = Client::builder()
        .timeout(Duration::from_millis(scenario.timeout_ms))
        .use_rustls_tls()
        .build()?;

    let client = Arc::new(client);
    let outcomes: Arc<Mutex<Vec<RequestOutcome>>> = Arc::new(Mutex::new(Vec::new()));

    let total_requests = scenario.requests;
    let duration_limit = scenario.duration_secs.map(Duration::from_secs);

    let pb = if let Some(n) = total_requests {
        let pb = ProgressBar::new(n);
        pb.set_style(
            ProgressStyle::with_template(
                "    {msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec})",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        pb.set_message(format!("{} {}", scenario.method, scenario.url));
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template(
                "    {msg} [{elapsed_precise}] {spinner} {pos} sent ({per_sec})",
            )
            .unwrap(),
        );
        pb.set_message(format!("{} {}", scenario.method, scenario.url));
        pb
    };
    let pb = Arc::new(pb);

    let start_time = Instant::now();

    if let Some(total) = total_requests {
        let chunk = (total + scenario.concurrency as u64 - 1) / scenario.concurrency as u64;
        let tasks: Vec<_> = (0..scenario.concurrency)
            .map(|worker_idx| {
                let client = Arc::clone(&client);
                let outcomes = Arc::clone(&outcomes);
                let pb = Arc::clone(&pb);
                let scenario = scenario.clone();
                let worker_start = worker_idx as u64 * chunk;
                let worker_end = (worker_start + chunk).min(total);
                let count = worker_end.saturating_sub(worker_start);

                tokio::spawn(async move {
                    for _ in 0..count {
                        let outcome = execute_request(&client, &scenario, start_time).await;
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
                        let outcome = execute_request(&client, &scenario, start_time).await;
                        pb.inc(1);
                        outcomes.lock().await.push(outcome);
                    }
                })
            })
            .collect();
        futures::future::join_all(tasks).await;
    }

    pb.finish_and_clear();

    let total_duration = start_time.elapsed();
    let all_outcomes = Arc::try_unwrap(outcomes)
        .expect("all tasks done")
        .into_inner();

    let result = ScenarioResult::from_outcomes(scenario, all_outcomes, total_duration);
    println!(
        "    ✓ {} — {:.1} req/s  success: {}  failed: {}  errors: {}  p99: {:.2}ms",
        scenario.name,
        result.throughput_rps,
        result.successful_requests,
        result.failed_requests,
        result.error_requests,
        result.latency_p99_ms,
    );
    Ok(result)
}

// ── Per-request execution ─────────────────────────────────────────────────────

async fn execute_request(client: &Client, scenario: &Scenario, start: Instant) -> RequestOutcome {
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
        Duration::from_millis(scenario.timeout_ms + 100),
        request.send(),
    )
    .await;

    match result {
        Ok(Ok(response)) => {
            let status = response.status().as_u16();
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
                "timeout"
            } else if e.is_connect() {
                "connection error"
            } else {
                "request error"
            };
            RequestOutcome {
                latency_us: req_start.elapsed().as_micros() as u64,
                status_code: None,
                error: Some(msg.to_string()),
                offset_ms,
            }
        }
        Err(_) => RequestOutcome {
            latency_us: req_start.elapsed().as_micros() as u64,
            status_code: None,
            error: Some("timeout".to_string()),
            offset_ms,
        },
    }
}


