use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::collections::HashMap;

use crate::config::{RunConfig, RunParams, Scenario, ScenarioFile, Step};

// ── Top-level CLI ─────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "bench",
    about = "HTTP/REST API benchmarking tool",
    long_about = "Benchmark HTTP/REST APIs or edit scenario files.\n\n\
                  bench run  [flags]          run a benchmark\n\
                  bench edit [--file FILE]    open the scenario editor UI\n\
                  bench report [--file FILE]  view a JSON report in the browser"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Open the scenario file editor in your browser
    Edit {
        /// Scenario file to edit (created if it doesn't exist)
        #[arg(long, short = 'f', default_value = "scenarios.json")]
        file: String,
    },

    /// View a JSON benchmark report in your browser, or export to HTML/PDF
    Report {
        /// Path to the JSON report file
        #[arg(long, short = 'f', default_value = "report.json")]
        file: String,

        /// Export to a file instead of opening the browser (format inferred from extension: .html or .pdf)
        #[arg(long, value_name = "FILE")]
        export: Option<String>,
    },

    /// Run a benchmark
    Run(RunArgs),
}

// ── Run subcommand args ───────────────────────────────────────────────────────

#[derive(clap::Args, Debug)]
#[command(
    about = "Run a benchmark",
    long_about = "Run an HTTP benchmark.\n\n\
                  File mode:         bench run --file scenarios.json\n\
                  Single-step mode:  bench run --url <URL> --requests 1000\n\n\
                  By default a JSON report is written to report.json.\n\
                  Use --export to additionally generate an HTML or PDF file.\n\
                  Use --no-report to skip all file output (console only).\n\n\
                  CLI flags override the global 'run' block in the JSON file."
)]
pub struct RunArgs {
    // ── Input ─────────────────────────────────────────────────────────────────
    /// Path to a JSON scenario file
    #[arg(long, short = 'f', value_name = "FILE", conflicts_with = "url")]
    pub file: Option<String>,

    /// Target URL (single-step mode)
    #[arg(long, value_name = "URL", required_unless_present = "file")]
    pub url: Option<String>,

    /// HTTP method
    #[arg(long, short = 'X', default_value = "GET")]
    pub method: String,

    /// Request header "Key:Value" (repeatable)
    #[arg(long = "header", short = 'H', value_name = "KEY:VALUE",
          action = clap::ArgAction::Append)]
    pub headers: Vec<String>,

    /// Raw request body
    #[arg(long, short = 'd')]
    pub body: Option<String>,

    /// Shorthand for Content-Type header
    #[arg(long)]
    pub content_type: Option<String>,

    // ── Run parameters ────────────────────────────────────────────────────────
    /// Concurrent workers
    #[arg(long, short = 'c')]
    pub concurrency: Option<usize>,

    /// Run for this many seconds (mutually exclusive with --requests)
    #[arg(long, conflicts_with = "requests")]
    pub duration: Option<u64>,

    /// Total number of scenario executions (mutually exclusive with --duration)
    #[arg(long, short = 'n')]
    pub requests: Option<u64>,

    /// Per-request timeout in milliseconds
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Scenario name (single-step mode)
    #[arg(long, default_value = "Benchmark")]
    pub name: String,

    // ── Output ────────────────────────────────────────────────────────────────
    /// JSON report output path [default: report.json]
    #[arg(long, short = 'o')]
    pub output: Option<String>,

    /// Also export to HTML or PDF after the run (format inferred from .html or .pdf extension)
    #[arg(long, value_name = "FILE")]
    pub export: Option<String>,

    /// Skip writing any report file — print results to console only
    #[arg(long)]
    pub no_report: bool,

    /// Open the JSON report in the browser after the benchmark completes
    #[arg(long, conflicts_with = "no_report")]
    pub open: bool,
}

// ── Config resolution ─────────────────────────────────────────────────────────

impl Cli {
    pub fn into_run_config(self) -> Result<RunConfig> {
        match self.command {
            Command::Run(args)     => args.into_run_config(),
            Command::Edit { .. }   => unreachable!("edit handled in main"),
            Command::Report { .. } => unreachable!("report handled in main"),
        }
    }
}

impl RunArgs {
    pub fn into_run_config(self) -> Result<RunConfig> {
        let cli_run = RunParams {
            concurrency:   self.concurrency,
            duration_secs: self.duration,
            requests:      self.requests,
            timeout_ms:    self.timeout,
            output_format: None,
            output:        None,
        };

        let json_output = self.output.unwrap_or_else(|| "report.json".to_string());

        if let Some(file_path) = self.file {
            let content = std::fs::read_to_string(&file_path)
                .with_context(|| format!("Cannot read file: {file_path}"))?;
            let sf: ScenarioFile = serde_json::from_str(&content)
                .with_context(|| format!("Cannot parse file: {file_path}"))?;

            if sf.scenarios.is_empty() {
                bail!("No scenarios defined in file");
            }

            let json_global = sf.run.unwrap_or(RunParams {
                concurrency: None, duration_secs: None, requests: None,
                timeout_ms: None, output_format: None, output: None,
            });
            let global_run = cli_run.merge_over(&json_global);

            let mut scenarios: Vec<Scenario> = Vec::with_capacity(sf.scenarios.len());
            for (i, sref) in sf.scenarios.iter().enumerate() {
                let ctx = format!("scenario #{i} \"{}\"", sref.name);
                if sref.steps.is_empty() {
                    bail!("{ctx}: has no steps");
                }
                let steps = sref.steps.iter().map(|step_name| {
                    sf.requests.get(step_name)
                        .ok_or_else(|| anyhow::anyhow!(
                            "{ctx}: step \"{step_name}\" not found in the top-level \"requests\" map"
                        ))
                        .map(|def| def.clone().into_step(step_name.clone()))
                }).collect::<Result<Vec<Step>>>()?;

                let eff = match &sref.run {
                    Some(s) => s.merge_over(&global_run),
                    None    => global_run.clone(),
                };
                validate_run(&eff, &ctx)?;

                for step in &steps {
                    if step.url.is_empty() {
                        bail!("{ctx}, step \"{}\": empty URL", step.name);
                    }
                }

                scenarios.push(Scenario { name: sref.name.clone(), run: sref.run.clone(), steps });
            }

            Ok(RunConfig { scenarios, global_run, json_output, export_path: self.export,
                           no_report: self.no_report, open_report: self.open })
        } else {
            if self.duration.is_none() && self.requests.is_none() {
                bail!("Either --duration or --requests must be specified");
            }
            let mut headers: HashMap<String, String> =
                self.headers.iter().map(|h| parse_header(h)).collect::<Result<_>>()?;
            if let Some(ct) = self.content_type {
                headers.insert("content-type".to_string(), ct);
            }

            let global_run = RunParams {
                concurrency:   Some(self.concurrency.unwrap_or(10)),
                duration_secs: self.duration,
                requests:      self.requests,
                timeout_ms:    Some(self.timeout.unwrap_or(5000)),
                output_format: None,
                output:        None,
            };
            let scenario = Scenario {
                name: self.name.clone(),
                run: None,
                steps: vec![Step {
                    name: self.name,
                    url: self.url.expect("--url required"),
                    method: self.method.to_uppercase(),
                    headers,
                    body: self.body,
                }],
            };

            Ok(RunConfig { scenarios: vec![scenario], global_run, json_output, export_path: self.export,
                           no_report: self.no_report, open_report: self.open })
        }
    }
}

fn parse_header(h: &str) -> Result<(String, String)> {
    let colon = h.find(':')
        .with_context(|| format!("Invalid header format (expected Key:Value): {h}"))?;
    Ok((h[..colon].trim().to_lowercase(), h[colon + 1..].trim().to_string()))
}

fn validate_run(r: &RunParams, ctx: &str) -> Result<()> {
    if r.duration_secs.is_none() && r.requests.is_none() {
        bail!("{ctx}: no run config — specify \"requests\" or \"duration_secs\" \
               in the global \"run\" block, a scenario 'run' block, or via CLI flags");
    }
    if r.effective_concurrency() == 0 {
        bail!("{ctx}: concurrency must be >= 1");
    }
    Ok(())
}
