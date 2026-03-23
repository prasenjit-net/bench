pub mod html;
pub mod json;
pub mod pdf;

use anyhow::Result;

use crate::stats::ScenarioResult;

/// A group of step results belonging to one scenario.
pub struct ScenarioGroup {
    pub name: String,
    pub concurrency: usize,
    pub run_desc: String,
    pub results: Vec<ScenarioResult>,
}

/// Generate JSON report.
pub fn generate_json(groups: &[ScenarioGroup], output_path: &str) -> Result<()> {
    json::generate(groups, output_path)
}

/// Export an existing JSON report file to HTML or PDF (format inferred from extension).
pub fn export_report(json_path: &str, export_path: &str) -> Result<()> {
    let ext = std::path::Path::new(export_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "html" => html::generate_from_json_file(json_path, export_path),
        "pdf"  => {
            let report = json::read_report(json_path)?;
            let groups = json::groups_from_report(&report);
            pdf::generate(&groups, export_path)
        }
        other => anyhow::bail!(
            "Cannot infer export format from extension '.{other}'. Use .html or .pdf"
        ),
    }
}
