use anyhow::{bail, Context, Result};
use clap::Parser;
use std::collections::HashMap;

use crate::config::{OutputFormat, RunConfig, Scenario, ScenarioFile};

#[derive(Parser, Debug)]
#[command(
    name = "bench",
    about = "HTTP/REST API benchmarking tool",
    long_about = "Benchmark HTTP/REST APIs from the command line or a JSON scenario file.\n\
                  Single-API mode: provide --url and related flags.\n\
                  File mode: provide --file pointing to a JSON scenario file."
)]
pub struct Cli {
    // ── File mode ──────────────────────────────────────────────────────────────
    /// Path to a JSON scenario file (enables multi-scenario mode)
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
            let scenario_file: ScenarioFile = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse scenario file: {file_path}"))?;

            if scenario_file.scenarios.is_empty() {
                bail!("Scenario file contains no scenarios");
            }

            for (i, s) in scenario_file.scenarios.iter().enumerate() {
                validate_scenario(s, i)?;
            }

            let fmt = scenario_file
                .global
                .output_format
                .as_deref()
                .map(OutputFormat::from_str)
                .unwrap_or(format);

            let default_out = format!("report.{}", fmt.default_extension());
            let output_path = self
                .output
                .or(scenario_file.global.output)
                .unwrap_or(default_out);

            Ok(RunConfig {
                scenarios: scenario_file.scenarios,
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

            let scenario = Scenario {
                name: self.name,
                url,
                method: self.method.to_uppercase(),
                headers,
                body: self.body,
                concurrency: self.concurrency,
                duration_secs: self.duration,
                requests: self.requests,
                timeout_ms: self.timeout,
            };

            let default_out = format!("report.{}", format.default_extension());
            let output_path = self.output.unwrap_or(default_out);

            Ok(RunConfig {
                scenarios: vec![scenario],
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

fn validate_scenario(s: &Scenario, idx: usize) -> Result<()> {
    if s.url.is_empty() {
        bail!("Scenario #{idx} has an empty URL");
    }
    if s.duration_secs.is_none() && s.requests.is_none() {
        bail!(
            "Scenario #{idx} ('{}') must specify either duration_secs or requests",
            s.name
        );
    }
    if s.concurrency == 0 {
        bail!("Scenario #{idx} ('{}') concurrency must be >= 1", s.name);
    }
    Ok(())
}
