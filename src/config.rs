use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single test scenario (from JSON file or synthesized from CLI args)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// Human-readable name for the scenario
    pub name: String,
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Run for this many seconds (mutually exclusive with `requests`)
    pub duration_secs: Option<u64>,
    /// Send exactly this many requests (mutually exclusive with `duration_secs`)
    pub requests: Option<u64>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_concurrency() -> usize {
    10
}

fn default_timeout_ms() -> u64 {
    5000
}

/// Top-level structure for the JSON scenario file
#[derive(Debug, Deserialize)]
pub struct ScenarioFile {
    #[serde(default)]
    pub global: GlobalConfig,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize, Default)]
pub struct GlobalConfig {
    pub output_format: Option<String>,
    pub output: Option<String>,
}

/// Resolved runtime configuration for the tool
#[derive(Debug)]
pub struct RunConfig {
    pub scenarios: Vec<Scenario>,
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
