use anyhow::Result;

use crate::report::ScenarioGroup;
use crate::ui_assets::UiAssets;

pub fn generate(groups: &[ScenarioGroup<'_>], output_path: &str) -> Result<()> {
    // Build the report JSON to embed
    let report_json = super::json::build_json_string(groups)?;

    // Find the compiled JS and CSS bundles from the embedded assets
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
