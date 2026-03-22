use serde::Deserialize;
use std::collections::HashMap;

// ── Step ──────────────────────────────────────────────────────────────────────

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

fn default_method() -> String { "GET".to_string() }

/// A request definition as declared in the `requests` library of a scenario file.
/// The map key becomes the step's `name` at resolution time.
#[derive(Debug, Clone, Deserialize)]
pub struct RequestDef {
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl RequestDef {
    pub fn into_step(self, name: String) -> Step {
        Step { name, url: self.url, method: self.method, headers: self.headers, body: self.body }
    }
}

// ── Run parameters ────────────────────────────────────────────────────────────

/// Controls how many times a scenario executes and with what parallelism.
/// Specified globally and/or per-scenario (per-scenario overrides global).
#[derive(Debug, Clone, Deserialize)]
pub struct RunParams {
    pub concurrency: Option<usize>,
    /// Run for this many seconds (mutually exclusive with `requests`).
    pub duration_secs: Option<u64>,
    /// Execute the scenario this many times total (mutually exclusive with `duration_secs`).
    pub requests: Option<u64>,
    pub timeout_ms: Option<u64>,
    pub output_format: Option<String>,
    pub output: Option<String>,
}

impl RunParams {
    /// Merge: self fields take priority over `base` (global) fields.
    pub fn merge_over(&self, base: &RunParams) -> RunParams {
        RunParams {
            concurrency:   self.concurrency.or(base.concurrency),
            duration_secs: self.duration_secs.or(base.duration_secs),
            requests:      self.requests.or(base.requests),
            timeout_ms:    self.timeout_ms.or(base.timeout_ms),
            output_format: self.output_format.clone().or(base.output_format.clone()),
            output:        self.output.clone().or(base.output.clone()),
        }
    }

    pub fn effective_concurrency(&self) -> usize { self.concurrency.unwrap_or(10) }
    pub fn effective_timeout_ms(&self) -> u64    { self.timeout_ms.unwrap_or(5000) }
}

// ── Scenario ──────────────────────────────────────────────────────────────────

/// A named, ordered list of HTTP steps (fully resolved — used at runtime).
/// One execution = all steps run sequentially, exactly once.
/// `run` is optional — if absent the global run config applies.
#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: String,
    /// Per-scenario run override (any field present overrides the global value).
    pub run: Option<RunParams>,
    pub steps: Vec<Step>,
}

/// A scenario as declared in the JSON file — steps are referenced by name.
#[derive(Debug, Deserialize)]
pub struct ScenarioRef {
    pub name: String,
    pub run: Option<RunParams>,
    /// Ordered list of step names referencing entries in the top-level `steps` map.
    pub steps: Vec<String>,
}

// ── Top-level JSON file ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ScenarioFile {
    /// Global run defaults — inherited by any scenario that omits its own `run`.
    pub run: Option<RunParams>,
    /// Named request library — define once, reference by name from any scenario's steps.
    #[serde(default)]
    pub requests: HashMap<String, RequestDef>,
    pub scenarios: Vec<ScenarioRef>,
}

// ── Runtime config ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct RunConfig {
    pub scenarios: Vec<Scenario>,
    /// Resolved global run defaults (used when a scenario has no `run` block).
    pub global_run: RunParams,
    pub output_format: OutputFormat,
    pub output_path: String,
}

impl RunConfig {
    /// Return the effective RunParams for a given scenario (scenario overrides global).
    pub fn effective_run(&self, scenario: &Scenario) -> RunParams {
        match &scenario.run {
            Some(s_run) => s_run.merge_over(&self.global_run),
            None        => self.global_run.clone(),
        }
    }
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
        match self { OutputFormat::Html => "html", OutputFormat::Pdf => "pdf" }
    }
}
