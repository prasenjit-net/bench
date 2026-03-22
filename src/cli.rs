use anyhow::{bail, Context, Result};
use clap::Parser;
use std::collections::HashMap;

use crate::config::{OutputFormat, RunConfig, RunParams, Scenario, ScenarioFile, Step};

#[derive(Parser, Debug)]
#[command(
    name = "bench",
    about = "HTTP/REST API benchmarking tool",
    long_about = "Benchmark HTTP/REST APIs.\n\n\
                  Single-step mode:  bench --url <URL> [flags]\n\
                  File mode:         bench --file scenarios.json\n\n\
                  Run parameters (concurrency, duration/requests, timeout) control\n\
                  how many times the scenario executes. Each execution runs all steps\n\
                  in order, exactly once."
)]
pub struct Cli {
    // ── File mode ─────────────────────────────────────────────────────────────
    /// Path to a JSON scenario file
    #[arg(long, short = 'f', value_name = "FILE", conflicts_with = "url")]
    pub file: Option<String>,

    // ── Single-step mode ──────────────────────────────────────────────────────
    /// Target URL
    #[arg(long, value_name = "URL", required_unless_present = "file")]
    pub url: Option<String>,

    /// HTTP method [default: GET]
    #[arg(long, short = 'X', default_value = "GET")]
    pub method: String,

    /// Request header "Key:Value" (repeatable)
    #[arg(long = "header", short = 'H', value_name = "KEY:VALUE", action = clap::ArgAction::Append)]
    pub headers: Vec<String>,

    /// Raw request body
    #[arg(long, short = 'd')]
    pub body: Option<String>,

    /// Shorthand for Content-Type header
    #[arg(long)]
    pub content_type: Option<String>,

    // ── Run parameters ────────────────────────────────────────────────────────
    /// Concurrent workers [default: 10]
    #[arg(long, short = 'c', default_value_t = 10)]
    pub concurrency: usize,

    /// Run for this many seconds (mutually exclusive with --requests)
    #[arg(long, conflicts_with = "requests")]
    pub duration: Option<u64>,

    /// Execute the scenario this many times total (mutually exclusive with --duration)
    #[arg(long, short = 'n')]
    pub requests: Option<u64>,

    /// Per-request timeout in milliseconds [default: 5000]
    #[arg(long, default_value_t = 5000)]
    pub timeout: u64,

    /// Scenario name [default: Benchmark]
    #[arg(long, default_value = "Benchmark")]
    pub name: String,

    // ── Output ────────────────────────────────────────────────────────────────
    /// Output format: html or pdf [default: html]
    #[arg(long, default_value = "html")]
    pub output_format: String,

    /// Output file path
    #[arg(long, short = 'o')]
    pub output: Option<String>,
}

impl Cli {
    pub fn into_run_config(self) -> Result<RunConfig> {
        let fmt = OutputFormat::from_str(&self.output_format);

        if let Some(file_path) = self.file {
            let content = std::fs::read_to_string(&file_path)
                .with_context(|| format!("Cannot read file: {file_path}"))?;
            let sf: ScenarioFile = serde_json::from_str(&content)
                .with_context(|| format!("Cannot parse file: {file_path}"))?;
            validate_run(&sf.run)?;
            validate_scenario(&sf.scenario)?;

            let fmt = sf.run.output_format.as_deref().map(OutputFormat::from_str).unwrap_or(fmt);
            let default_out = format!("report.{}", fmt.default_extension());
            let output_path = self.output.or(sf.run.output.clone()).unwrap_or(default_out);

            Ok(RunConfig { scenario: sf.scenario, run: sf.run, output_format: fmt, output_path })
        } else {
            if self.duration.is_none() && self.requests.is_none() {
                bail!("Either --duration or --requests must be specified");
            }
            let mut headers: HashMap<String, String> =
                self.headers.iter().map(|h| parse_header(h)).collect::<Result<_>>()?;
            if let Some(ct) = self.content_type {
                headers.insert("content-type".to_string(), ct);
            }

            let run = RunParams {
                concurrency: self.concurrency,
                duration_secs: self.duration,
                requests: self.requests,
                timeout_ms: self.timeout,
                output_format: Some(self.output_format.clone()),
                output: None,
            };
            let scenario = Scenario {
                name: self.name.clone(),
                steps: vec![Step {
                    name: self.name,
                    url: self.url.expect("--url required"),
                    method: self.method.to_uppercase(),
                    headers,
                    body: self.body,
                }],
            };

            let default_out = format!("report.{}", fmt.default_extension());
            let output_path = self.output.unwrap_or(default_out);
            Ok(RunConfig { scenario, run, output_format: fmt, output_path })
        }
    }
}

fn parse_header(h: &str) -> Result<(String, String)> {
    let colon = h.find(':')
        .with_context(|| format!("Invalid header format (expected Key:Value): {h}"))?;
    Ok((h[..colon].trim().to_lowercase(), h[colon + 1..].trim().to_string()))
}

fn validate_run(r: &RunParams) -> Result<()> {
    if r.duration_secs.is_none() && r.requests.is_none() {
        bail!("\"run\" must specify either \"duration_secs\" or \"requests\"");
    }
    if r.concurrency == 0 {
        bail!("\"run.concurrency\" must be >= 1");
    }
    Ok(())
}

fn validate_scenario(s: &Scenario) -> Result<()> {
    if s.steps.is_empty() {
        bail!("Scenario \"{}\" has no steps", s.name);
    }
    for (i, step) in s.steps.iter().enumerate() {
        if step.url.is_empty() {
            bail!("Step #{i} \"{}\" has an empty URL", step.name);
        }
    }
    Ok(())
}
