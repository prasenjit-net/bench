use serde::Deserialize;
use std::collections::HashMap;

// ── Scenario tree node types ──────────────────────────────────────────────────

/// A node in the scenario tree: either a group (parallel/sequential) or a leaf request.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ScenarioNode {
    /// Branch node — children run in `mode` order.
    Group(GroupNode),
    /// Leaf node — one HTTP endpoint to benchmark.
    Request(RequestNode),
}

/// Branch node: runs its `steps` either sequentially or in parallel.
#[derive(Debug, Clone, Deserialize)]
pub struct GroupNode {
    pub name: String,
    #[serde(default)]
    pub mode: ExecutionMode,
    /// Must be present to disambiguate from a RequestNode during deserialization.
    pub steps: Vec<ScenarioNode>,
}

/// Leaf node: HTTP request specification.
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

/// How child steps within a GroupNode are executed in one iteration.
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

/// Global execution parameters — drive how many times the full scenario tree repeats.
///
/// One "iteration" = traverse the entire tree once, firing each leaf request exactly once.
/// `concurrency` workers each run their share of iterations in parallel with each other.
/// Within one iteration, sequential/parallel ordering is determined by the tree structure.
#[derive(Debug, Clone, Deserialize)]
pub struct RunParams {
    /// Number of parallel workers; each runs its share of total iterations.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Repeat the full tree for this many seconds total (mutually exclusive with `requests`).
    pub duration_secs: Option<u64>,
    /// Repeat the full tree exactly this many times total (mutually exclusive with `duration_secs`).
    pub requests: Option<u64>,
    /// Per-request timeout in milliseconds.
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

/// Top-level JSON scenario file structure.
#[derive(Debug, Deserialize)]
pub struct ScenarioFile {
    pub run: RunParams,
    /// Single root node: a Group (parallel|sequential) or a lone Request.
    pub scenario: ScenarioNode,
}

// ── Resolved runtime configuration ───────────────────────────────────────────

#[derive(Debug)]
pub struct RunConfig {
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
