use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::report::ScenarioGroup;
use crate::stats::ScenarioResult;

#[derive(Serialize, Deserialize)]
pub struct JsonReport {
    pub generated_at: String,
    pub group_count: usize,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub error_requests: u64,
    pub groups: Vec<JsonGroup>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonGroup {
    pub name: String,
    pub concurrency: usize,
    pub run_desc: String,
    pub step_count: usize,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub error_requests: u64,
    pub steps: Vec<ScenarioResult>,
}

fn build_report(groups: &[ScenarioGroup]) -> JsonReport {
    let total_requests: u64         = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.total_requests).sum();
    let successful_requests: u64    = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.successful_requests).sum();
    let failed_requests: u64        = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.failed_requests).sum();
    let error_requests: u64         = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.error_requests).sum();

    let json_groups: Vec<JsonGroup> = groups.iter().map(|g| JsonGroup {
        name: g.name.clone(),
        concurrency: g.concurrency,
        run_desc: g.run_desc.clone(),
        step_count: g.results.len(),
        total_requests: g.results.iter().map(|r| r.total_requests).sum(),
        successful_requests: g.results.iter().map(|r| r.successful_requests).sum(),
        failed_requests: g.results.iter().map(|r| r.failed_requests).sum(),
        error_requests: g.results.iter().map(|r| r.error_requests).sum(),
        steps: g.results.clone(),
    }).collect();

    JsonReport {
        generated_at: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        group_count: groups.len(),
        total_requests,
        successful_requests,
        failed_requests,
        error_requests,
        groups: json_groups,
    }
}

/// Write JSON report to file.
pub fn generate(groups: &[ScenarioGroup], output_path: &str) -> Result<()> {
    let report = build_report(groups);
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(output_path, json)?;
    Ok(())
}

/// Build JSON string (reused by html.rs for inline embedding).
#[allow(dead_code)]
pub fn build_json_string(groups: &[ScenarioGroup]) -> Result<String> {
    Ok(serde_json::to_string(&build_report(groups))?)
}

/// Read and deserialize a JSON report file.
pub fn read_report(path: &str) -> Result<JsonReport> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Cannot read report file '{}': {}", path, e))?;
    let report: JsonReport = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Cannot parse report file '{}': {}", path, e))?;
    Ok(report)
}

/// Convert a deserialized JsonReport back into ScenarioGroups (for PDF generation).
pub fn groups_from_report(report: &JsonReport) -> Vec<ScenarioGroup> {
    report.groups.iter().map(|g| ScenarioGroup {
        name: g.name.clone(),
        concurrency: g.concurrency,
        run_desc: g.run_desc.clone(),
        results: g.steps.clone(),
    }).collect()
}
