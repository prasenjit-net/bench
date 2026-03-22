pub mod html;
pub mod json;
pub mod pdf;

use anyhow::Result;

use crate::config::OutputFormat;
use crate::stats::ScenarioResult;

/// A group of step results belonging to one scenario.
pub struct ScenarioGroup<'a> {
    pub name: &'a str,
    pub concurrency: usize,
    pub run_desc: String, // e.g. "20 runs" or "30s"
    pub results: Vec<ScenarioResult>,
}

pub fn generate_report(
    groups: &[ScenarioGroup<'_>],
    format: &OutputFormat,
    output_path: &str,
) -> Result<()> {
    match format {
        OutputFormat::Json => json::generate(groups, output_path),
        OutputFormat::Html => html::generate(groups, output_path),
        OutputFormat::Pdf  => pdf::generate(groups, output_path),
    }
}
