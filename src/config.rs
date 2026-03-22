use serde::Deserialize;
use std::collections::HashMap;

// ── Step (leaf HTTP request) ──────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Step {
    pub name: String,
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

// ── Scenario ──────────────────────────────────────────────────────────────────

/// A scenario is a named, ordered list of HTTP steps executed sequentially.
/// One execution = all steps run once in order.
#[derive(Debug, Clone, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub steps: Vec<Step>,
}

// ── Run parameters ────────────────────────────────────────────────────────────

/// Controls how many times the scenario is executed and with what parallelism.
///
/// `concurrency` workers each run their share of total executions in parallel.
/// Each worker executes the scenario sequentially (all steps, one by one).
#[derive(Debug, Clone, Deserialize)]
pub struct RunParams {
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Run the scenario for this many seconds (mutually exclusive with `requests`).
    pub duration_secs: Option<u64>,
    /// Run the scenario exactly this many times total (mutually exclusive with `duration_secs`).
    pub requests: Option<u64>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub output_format: Option<String>,
    pub output: Option<String>,
}

fn default_concurrency() -> usize { 10 }
fn default_timeout_ms() -> u64 { 5000 }

// ── Top-level JSON file ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ScenarioFile {
    pub run: RunParams,
    pub scenario: Scenario,
}

// ── Runtime config ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct RunConfig {
    pub scenario: Scenario,
    pub run: RunParams,
    pub output_format: OutputFormat,
    pub output_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Html,
    Pdf,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pdf" => OutputFormat::Pdf,
            _ => OutputFormat::Html,
        }
    }
    pub fn default_extension(&self) -> &str {
        match self {
            OutputFormat::Html => "html",
            OutputFormat::Pdf => "pdf",
        }
    }
}
