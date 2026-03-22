use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Public tree node types ────────────────────────────────────────────────────

/// A node in the scenario tree: either a group (parallel/sequential) or a leaf request.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ScenarioNode {
    /// Branch node — contains child steps run in `mode` order.
    Group(GroupNode),
    /// Leaf node — a single HTTP endpoint to benchmark.
    Request(RequestNode),
}

/// Branch node: runs its `steps` either in parallel or sequentially.
#[derive(Debug, Clone, Deserialize)]
pub struct GroupNode {
    pub name: String,
    #[serde(default)]
    pub mode: ExecutionMode,
    /// Must be present and non-empty to distinguish from a RequestNode.
    pub steps: Vec<ScenarioNode>,
}

/// Leaf node: the HTTP request specification.
#[derive(Debug, Clone, Deserialize)]
pub struct RequestNode {
    pub name: String,
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl RequestNode {
    /// Merge this request spec with global run parameters to produce a runnable Scenario.
    pub fn into_scenario(self, run: &RunParams) -> Scenario {
        Scenario {
            name: self.name,
            url: self.url,
            method: self.method.to_uppercase(),
            headers: self.headers,
            body: self.body,
            concurrency: run.concurrency,
            duration_secs: run.duration_secs,
            requests: run.requests,
            timeout_ms: run.timeout_ms,
        }
    }
}

/// How child steps in a GroupNode are executed.
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    #[default]
    Sequential,
    Parallel,
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionMode::Sequential => write!(f, "sequential"),
            ExecutionMode::Parallel => write!(f, "parallel"),
        }
    }
}

// ── Global run parameters ─────────────────────────────────────────────────────

/// Global execution parameters — specified once and applied to every leaf request.
#[derive(Debug, Clone, Deserialize)]
pub struct RunParams {
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Run each scenario for this many seconds (mutually exclusive with `requests`).
    pub duration_secs: Option<u64>,
    /// Send exactly this many requests per scenario (mutually exclusive with `duration_secs`).
    pub requests: Option<u64>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Output format: "html" (default) or "pdf".
    #[serde(default)]
    pub output_format: Option<String>,
    /// Output file path.
    pub output: Option<String>,
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

// ── Top-level JSON file ───────────────────────────────────────────────────────

/// The top-level JSON scenario file format.
#[derive(Debug, Deserialize)]
pub struct ScenarioFile {
    pub run: RunParams,
    /// Single root node — may be a Group (parallel/sequential) or a lone Request.
    pub scenario: ScenarioNode,
}

// ── Internal runner representation ───────────────────────────────────────────

/// Fully resolved, runnable scenario — a RequestNode merged with RunParams.
/// Used internally by the runner; never deserialized directly from user JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub concurrency: usize,
    pub duration_secs: Option<u64>,
    pub requests: Option<u64>,
    pub timeout_ms: u64,
}

// ── Resolved runtime configuration ───────────────────────────────────────────

#[derive(Debug)]
pub struct RunConfig {
    /// The root scenario node to execute.
    pub root: ScenarioNode,
    pub run_params: RunParams,
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

