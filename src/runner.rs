use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio::sync::Mutex;

use crate::config::{ExecutionMode, RequestNode, RunParams, ScenarioNode};
use crate::stats::{RequestOutcome, ScenarioResult};

// ── Shared outcome collector: leaf name → all outcomes across iterations ──────
type OutcomeMap = Arc<Mutex<HashMap<String, Vec<RequestOutcome>>>>;

// ── Public entry point ────────────────────────────────────────────────────────

/// Run the full scenario tree according to `run` parameters.
///
/// Execution model:
///   - `run.concurrency` workers are spawned; each runs its share of iterations.
///   - One "iteration" = traverse the tree once, firing every leaf request exactly once.
///   - Within one iteration, sequential groups fire their children one-by-one;
///     parallel groups fire all children concurrently (tokio join_all).
///   - Outcomes are collected per leaf-node name across all iterations and workers.
///   - Returns one `ScenarioResult` per unique leaf node, in tree-definition order.
pub async fn run_scenario_tree(
    root: &ScenarioNode,
    run: &RunParams,
) -> Result<Vec<ScenarioResult>> {
    let client = Arc::new(
        Client::builder()
            .timeout(Duration::from_millis(run.timeout_ms))
            .use_rustls_tls()
            .build()?,
    );

    let outcome_map: OutcomeMap = Arc::new(Mutex::new(HashMap::new()));
    let total_iters = run.requests;
    let duration_limit = run.duration_secs.map(Duration::from_secs);

    // Progress bar tracks full-tree iterations, not individual requests
    let pb = if let Some(n) = total_iters {
        let pb = ProgressBar::new(n);
        pb.set_style(
            ProgressStyle::with_template(
                "  [{elapsed_precise}] [{bar:50.cyan/blue}] {pos}/{len} iterations  ({per_sec})",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template(
                "  [{elapsed_precise}] {spinner}  {pos} iterations  ({per_sec})",
            )
            .unwrap(),
        );
        pb
    };
    let pb = Arc::new(pb);
    let start = Instant::now();

    if let Some(total) = total_iters {
        // Count-based: distribute total iterations evenly across workers
        let chunk = (total + run.concurrency as u64 - 1) / run.concurrency as u64;
        let tasks: Vec<_> = (0..run.concurrency)
            .map(|i| {
                let client = Arc::clone(&client);
                let outcome_map = Arc::clone(&outcome_map);
                let pb = Arc::clone(&pb);
                let root = root.clone();
                let worker_start = i as u64 * chunk;
                let worker_end = (worker_start + chunk).min(total);
                let count = worker_end.saturating_sub(worker_start);

                tokio::spawn(async move {
                    for _ in 0..count {
                        execute_tree_once(&root, &client, &outcome_map, start).await;
                        pb.inc(1);
                    }
                })
            })
            .collect();

        futures::future::join_all(tasks).await;
    } else {
        // Duration-based: each worker keeps iterating until deadline
        let deadline = start + duration_limit.unwrap();
        let tasks: Vec<_> = (0..run.concurrency)
            .map(|_| {
                let client = Arc::clone(&client);
                let outcome_map = Arc::clone(&outcome_map);
                let pb = Arc::clone(&pb);
                let root = root.clone();

                tokio::spawn(async move {
                    loop {
                        if Instant::now() >= deadline {
                            break;
                        }
                        execute_tree_once(&root, &client, &outcome_map, start).await;
                        pb.inc(1);
                    }
                })
            })
            .collect();

        futures::future::join_all(tasks).await;
    }

    pb.finish_and_clear();
    let elapsed = start.elapsed();

    // Collect leaf nodes in definition order and build results
    let mut leaves: Vec<(String, String, String)> = Vec::new(); // (name, url, method)
    collect_leaves(root, &mut leaves);

    let mut map = Arc::try_unwrap(outcome_map)
        .expect("all tasks completed")
        .into_inner();

    let mut results = Vec::new();
    for (name, url, method) in &leaves {
        let outcomes = map.remove(name.as_str()).unwrap_or_default();
        let result = ScenarioResult::from_outcomes(
            name,
            url,
            method,
            run.concurrency,
            outcomes,
            elapsed,
        );
        results.push(result);
    }

    Ok(results)
}

// ── Tree traversal helpers ────────────────────────────────────────────────────

/// Collect leaf request nodes in depth-first definition order.
fn collect_leaves(node: &ScenarioNode, out: &mut Vec<(String, String, String)>) {
    match node {
        ScenarioNode::Request(r) => out.push((r.name.clone(), r.url.clone(), r.method.to_uppercase())),
        ScenarioNode::Group(g) => {
            for step in &g.steps {
                collect_leaves(step, out);
            }
        }
    }
}

/// Execute the scenario tree exactly once:
///   - Sequential group → children run one after the other.
///   - Parallel group   → all children start at the same time (join_all).
///   - Leaf request     → one HTTP call, outcome appended under the leaf's name.
fn execute_tree_once<'a>(
    node: &'a ScenarioNode,
    client: &'a Client,
    outcome_map: &'a OutcomeMap,
    start: Instant,
) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        match node {
            ScenarioNode::Request(req) => {
                let outcome = fire_request(client, req, start).await;
                outcome_map
                    .lock()
                    .await
                    .entry(req.name.clone())
                    .or_default()
                    .push(outcome);
            }

            ScenarioNode::Group(group) => match group.mode {
                ExecutionMode::Sequential => {
                    for step in &group.steps {
                        execute_tree_once(step, client, outcome_map, start).await;
                    }
                }
                ExecutionMode::Parallel => {
                    let futs: Vec<_> = group
                        .steps
                        .iter()
                        .map(|step| execute_tree_once(step, client, outcome_map, start))
                        .collect();
                    futures::future::join_all(futs).await;
                }
            },
        }
    })
}

// ── HTTP request execution ────────────────────────────────────────────────────

async fn fire_request(client: &Client, req: &RequestNode, start: Instant) -> RequestOutcome {
    let req_start = Instant::now();
    let offset_ms = start.elapsed().as_millis() as u64;

    let mut builder = client.request(
        req.method.to_uppercase().parse().unwrap_or(reqwest::Method::GET),
        &req.url,
    );

    for (k, v) in &req.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }

    if let Some(body) = &req.body {
        builder = builder.body(body.clone());
    }

    match builder.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let _ = response.bytes().await; // drain body for accurate timing
            RequestOutcome {
                latency_us: req_start.elapsed().as_micros() as u64,
                status_code: Some(status),
                error: None,
                offset_ms,
            }
        }
        Err(e) => {
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
    }
}
