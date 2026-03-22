use anyhow::{bail, Context, Result};
use clap::Parser;
use std::collections::HashMap;

use crate::config::{
    ExecutionMode, GroupNode, OutputFormat, RequestNode, RunConfig, RunParams, ScenarioFile,
    ScenarioNode,
};

#[derive(Parser, Debug)]
#[command(
    name = "bench",
    about = "HTTP/REST API benchmarking tool",
    long_about = "Benchmark HTTP/REST APIs from the command line or a JSON scenario file.\n\n\
                  Single-API mode:  bench --url <URL> [flags]\n\
                  File mode:        bench --file scenarios.json\n\n\
                  In file mode the JSON describes a scenario tree where groups can be\n\
                  'parallel' (all children run simultaneously) or 'sequential' (one by one).\n\
                  Groups can be nested arbitrarily deep. Global run parameters (concurrency,\n\
                  duration, timeout) are declared once under \"run\" and apply to every leaf."
)]
pub struct Cli {
    // ── File mode ──────────────────────────────────────────────────────────────
    /// Path to a JSON scenario file (enables tree scenario mode)
    #[arg(long, short = 'f', value_name = "FILE", conflicts_with = "url")]
    pub file: Option<String>,

    // ── Single-API mode ────────────────────────────────────────────────────────
    /// Target URL (single-API mode)
    #[arg(long, value_name = "URL", required_unless_present = "file")]
    pub url: Option<String>,

    /// HTTP method [default: GET]
    #[arg(long, short = 'X', default_value = "GET")]
    pub method: String,

    /// Request header in "Key:Value" format (repeatable)
    #[arg(long = "header", short = 'H', value_name = "KEY:VALUE", action = clap::ArgAction::Append)]
    pub headers: Vec<String>,

    /// Raw request body
    #[arg(long, short = 'd')]
    pub body: Option<String>,

    /// Shorthand for Content-Type header
    #[arg(long)]
    pub content_type: Option<String>,

    /// Number of concurrent workers [default: 10]
    #[arg(long, short = 'c', default_value_t = 10)]
    pub concurrency: usize,

    /// Run for this many seconds (mutually exclusive with --requests)
    #[arg(long, conflicts_with = "requests")]
    pub duration: Option<u64>,

    /// Send exactly this many requests (mutually exclusive with --duration)
    #[arg(long, short = 'n')]
    pub requests: Option<u64>,

    /// Per-request timeout in milliseconds [default: 5000]
    #[arg(long, default_value_t = 5000)]
    pub timeout: u64,

    /// Scenario name (single-API mode)
    #[arg(long, default_value = "Benchmark")]
    pub name: String,

    // ── Output ─────────────────────────────────────────────────────────────────
    /// Output format: html or pdf [default: html]
    #[arg(long, default_value = "html", value_name = "FORMAT")]
    pub output_format: String,

    /// Output file path [default: report.html or report.pdf]
    #[arg(long, short = 'o')]
    pub output: Option<String>,
}

impl Cli {
    pub fn into_run_config(self) -> Result<RunConfig> {
        let format = OutputFormat::from_str(&self.output_format);

        if let Some(file_path) = self.file {
            // ── File mode ──
            let content = std::fs::read_to_string(&file_path)
                .with_context(|| format!("Failed to read scenario file: {file_path}"))?;
            let sf: ScenarioFile = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse scenario file: {file_path}"))?;

            validate_run_params(&sf.run)?;
            validate_node(&sf.scenario)?;

            let fmt = sf
                .run
                .output_format
                .as_deref()
                .map(OutputFormat::from_str)
                .unwrap_or(format);

            let default_out = format!("report.{}", fmt.default_extension());
            let output_path = self.output.or(sf.run.output.clone()).unwrap_or(default_out);

            Ok(RunConfig {
                root: sf.scenario,
                run_params: sf.run,
                output_format: fmt,
                output_path,
            })
        } else {
            // ── Single-API mode ──
            let url = self.url.expect("--url required (enforced by clap)");

            if self.duration.is_none() && self.requests.is_none() {
                bail!("Either --duration or --requests must be specified");
            }

            let mut headers: HashMap<String, String> = self
                .headers
                .iter()
                .map(|h| parse_header(h))
                .collect::<Result<_>>()?;

            if let Some(ct) = self.content_type {
                headers.insert("content-type".to_string(), ct);
            }

            let run_params = RunParams {
                concurrency: self.concurrency,
                duration_secs: self.duration,
                requests: self.requests,
                timeout_ms: self.timeout,
                output_format: Some(self.output_format.clone()),
                output: None,
            };

            let root = ScenarioNode::Group(GroupNode {
                name: self.name.clone(),
                mode: ExecutionMode::Sequential,
                steps: vec![ScenarioNode::Request(RequestNode {
                    name: self.name,
                    url,
                    method: self.method.to_uppercase(),
                    headers,
                    body: self.body,
                })],
            });

            let default_out = format!("report.{}", format.default_extension());
            let output_path = self.output.unwrap_or(default_out);

            Ok(RunConfig {
                root,
                run_params,
                output_format: format,
                output_path,
            })
        }
    }
}

fn parse_header(h: &str) -> Result<(String, String)> {
    let colon = h
        .find(':')
        .with_context(|| format!("Invalid header format (expected Key:Value): {h}"))?;
    let key = h[..colon].trim().to_lowercase().to_string();
    let value = h[colon + 1..].trim().to_string();
    Ok((key, value))
}

fn validate_run_params(r: &RunParams) -> Result<()> {
    if r.duration_secs.is_none() && r.requests.is_none() {
        bail!("\"run\" must specify either \"duration_secs\" or \"requests\"");
    }
    if r.concurrency == 0 {
        bail!("\"run.concurrency\" must be >= 1");
    }
    Ok(())
}

fn validate_node(node: &ScenarioNode) -> Result<()> {
    match node {
        ScenarioNode::Request(r) => {
            if r.url.is_empty() {
                bail!("Request '{}' has an empty URL", r.name);
            }
        }
        ScenarioNode::Group(g) => {
            if g.steps.is_empty() {
                bail!("Group '{}' has no steps", g.name);
            }
            for step in &g.steps {
                validate_node(step)?;
            }
        }
    }
    Ok(())
}


