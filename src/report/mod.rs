pub mod html;
pub mod pdf;

use anyhow::Result;

use crate::config::OutputFormat;
use crate::stats::ScenarioResult;

pub fn generate_report(
    results: &[ScenarioResult],
    format: &OutputFormat,
    output_path: &str,
) -> Result<()> {
    match format {
        OutputFormat::Html => html::generate(results, output_path),
        OutputFormat::Pdf => pdf::generate(results, output_path),
    }
}
