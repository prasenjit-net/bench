mod cli;
mod config;
mod editor;
mod report;
mod runner;
mod stats;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use report::{generate_report, ScenarioGroup};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // ── edit subcommand ────────────────────────────────────────────────────────
    if let Command::Edit { file } = args.command {
        let path = std::path::PathBuf::from(file);
        editor::run_editor(path).await?;
        return Ok(());
    }

    // ── report subcommand ──────────────────────────────────────────────────────
    if let Command::Report { file } = args.command {
        let path = std::path::PathBuf::from(file);
        editor::run_report_viewer(path).await?;
        return Ok(());
    }

    // ── run subcommand (default) ───────────────────────────────────────────────
    let cfg = args.into_run_config()?;

    println!("\n🚀 bench — {} scenario(s)\n", cfg.scenarios.len());

    let mut groups: Vec<ScenarioGroup> = Vec::new();

    for scenario in &cfg.scenarios {
        let run = cfg.effective_run(scenario);
        let concurrency = run.effective_concurrency();
        let run_desc = match (run.requests, run.duration_secs) {
            (Some(n), _) => format!("{n} runs"),
            (_, Some(s)) => format!("{s}s duration"),
            _            => "?".to_string(),
        };

        println!("▶ Scenario: {}  [{}  ·  concurrency {}]", scenario.name, run_desc, concurrency);
        for (i, step) in scenario.steps.iter().enumerate() {
            println!("    {}. {} {} ({})", i + 1, step.method.to_uppercase(), step.url, step.name);
        }
        println!();

        let results = runner::run(scenario, &run).await?;

        for r in &results {
            println!("  ✓ {}  [{} {}]", r.name, r.method, r.url);
            println!("    {:.1} req/s  ·  {} total  ·  {} ok  ·  {} fail  ·  {} err  ·  p99 {:.2}ms",
                r.throughput_rps, r.total_requests, r.successful_requests,
                r.failed_requests, r.error_requests, r.latency_p99_ms);
        }
        println!();

        groups.push(ScenarioGroup { name: &scenario.name, concurrency, run_desc, results });
    }

    let fmt_name = match cfg.output_format {
        config::OutputFormat::Json => "JSON",
        config::OutputFormat::Html => "HTML",
        config::OutputFormat::Pdf  => "PDF",
    };
    println!("📄 Generating {fmt_name} report → {}", cfg.output_path);
    generate_report(&groups, &cfg.output_format, &cfg.output_path)?;

    let total: u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.total_requests).sum();
    let ok: u64    = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.successful_requests).sum();
    let err: u64   = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.error_requests).sum();
    println!("✅ Done!  {} scenario(s)  ·  {} total requests  ·  {} ok  ·  {} errors",
        groups.len(), total, ok, err);

    Ok(())
}
