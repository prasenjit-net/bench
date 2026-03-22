use anyhow::Result;
use chrono::Local;
use serde::Serialize;

use crate::report::ScenarioGroup;
use crate::stats::ScenarioResult;

#[derive(Serialize)]
struct JsonReport<'a> {
    generated_at: String,
    group_count: usize,
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    error_requests: u64,
    groups: Vec<JsonGroup<'a>>,
}

#[derive(Serialize)]
struct JsonGroup<'a> {
    name: &'a str,
    concurrency: usize,
    run_desc: &'a str,
    step_count: usize,
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    error_requests: u64,
    steps: &'a Vec<ScenarioResult>,
}

pub fn generate(groups: &[ScenarioGroup<'_>], output_path: &str) -> Result<()> {
    let total_requests: u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.total_requests).sum();
    let successful_requests: u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.successful_requests).sum();
    let failed_requests: u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.failed_requests).sum();
    let error_requests: u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.error_requests).sum();

    let json_groups: Vec<JsonGroup> = groups.iter().map(|g| JsonGroup {
        name: g.name,
        concurrency: g.concurrency,
        run_desc: &g.run_desc,
        step_count: g.results.len(),
        total_requests: g.results.iter().map(|r| r.total_requests).sum(),
        successful_requests: g.results.iter().map(|r| r.successful_requests).sum(),
        failed_requests: g.results.iter().map(|r| r.failed_requests).sum(),
        error_requests: g.results.iter().map(|r| r.error_requests).sum(),
        steps: &g.results,
    }).collect();

    let report = JsonReport {
        generated_at: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        group_count: groups.len(),
        total_requests,
        successful_requests,
        failed_requests,
        error_requests,
        groups: json_groups,
    };

    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(output_path, json)?;
    Ok(())
}

/// Build the JSON report string — reused by html.rs for embedding.
pub fn build_json_string(groups: &[crate::report::ScenarioGroup<'_>]) -> Result<String> {
    let total_requests: u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.total_requests).sum();
    let successful_requests: u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.successful_requests).sum();
    let failed_requests: u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.failed_requests).sum();
    let error_requests: u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.error_requests).sum();

    let json_groups: Vec<JsonGroup> = groups.iter().map(|g| JsonGroup {
        name: g.name,
        concurrency: g.concurrency,
        run_desc: &g.run_desc,
        step_count: g.results.len(),
        total_requests: g.results.iter().map(|r| r.total_requests).sum(),
        successful_requests: g.results.iter().map(|r| r.successful_requests).sum(),
        failed_requests: g.results.iter().map(|r| r.failed_requests).sum(),
        error_requests: g.results.iter().map(|r| r.error_requests).sum(),
        steps: &g.results,
    }).collect();

    let report = JsonReport {
        generated_at: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        group_count: groups.len(),
        total_requests,
        successful_requests,
        failed_requests,
        error_requests,
        groups: json_groups,
    };

    Ok(serde_json::to_string(&report)?)
}
