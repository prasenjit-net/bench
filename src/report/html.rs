use anyhow::Result;

use crate::report::ScenarioGroup;
use crate::ui_assets::UiAssets;

fn bundle_html(report_json: &str, output_path: &str) -> Result<()> {
    let mut js_content = String::new();
    let mut css_content = String::new();
    for file in UiAssets::iter() {
        let path = file.as_ref();
        if path.starts_with("assets/") && path.ends_with(".js") && !path.ends_with(".map") {
            if let Some(f) = UiAssets::get(path) {
                js_content = String::from_utf8_lossy(&f.data).to_string();
            }
        }
        if path.starts_with("assets/") && path.ends_with(".css") {
            if let Some(f) = UiAssets::get(path) {
                css_content = String::from_utf8_lossy(&f.data).to_string();
            }
        }
    }

    if js_content.is_empty() {
        anyhow::bail!("UI bundle not found in embedded assets — rebuild with `npm run build` in ui/");
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>bench — Report Viewer</title>
  <style>{css_content}</style>
</head>
<body>
  <div id="root"></div>
  <script>
    window.__BENCH_MODE__ = "report";
    window.__BENCH_REPORT__ = {report_json};
  </script>
  <script type="module">{js_content}</script>
</body>
</html>"#
    );

    std::fs::write(output_path, html)?;
    Ok(())
}

/// Generate HTML from in-memory benchmark results (used by --export during a run).
#[allow(dead_code)]
pub fn generate(groups: &[ScenarioGroup], output_path: &str) -> Result<()> {
    let report_json = super::json::build_json_string(groups)?;
    bundle_html(&report_json, output_path)
}

/// Generate HTML from an existing JSON report file.
pub fn generate_from_json_file(json_path: &str, output_path: &str) -> Result<()> {
    let report_json = std::fs::read_to_string(json_path)
        .map_err(|e| anyhow::anyhow!("Cannot read report file '{}': {}", json_path, e))?;
    // Validate it's valid JSON
    let _: serde_json::Value = serde_json::from_str(&report_json)
        .map_err(|e| anyhow::anyhow!("Invalid JSON in '{}': {}", json_path, e))?;
    bundle_html(&report_json, output_path)
}
