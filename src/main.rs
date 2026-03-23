mod cli;
mod config;
mod editor;
mod report;
mod runner;
mod stats;
mod ui_assets;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use report::ScenarioGroup;

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
    if let Command::Report { file, export } = args.command {
        if let Some(export_path) = export {
            println!("📄 Exporting report → {export_path}");
            report::export_report(&file, &export_path)?;
            println!("✅ Export complete");
        } else {
            let path = std::path::PathBuf::from(file);
            editor::run_report_viewer(path).await?;
        }
        return Ok(());
    }

    // ── run subcommand ─────────────────────────────────────────────────────────
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

        groups.push(ScenarioGroup { name: scenario.name.clone(), concurrency, run_desc, results });
    }

    let total: u64 = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.total_requests).sum();
    let ok: u64    = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.successful_requests).sum();
    let err: u64   = groups.iter().flat_map(|g| g.results.iter()).map(|r| r.error_requests).sum();

    if cfg.no_report {
        println!("✅ Done!  {} scenario(s)  ·  {} total requests  ·  {} ok  ·  {} errors",
            groups.len(), total, ok, err);
    } else {
        println!("📄 Generating JSON report → {}", cfg.json_output);
        report::generate_json(&groups, &cfg.json_output)?;

        if let Some(ref export_path) = cfg.export_path {
            println!("📄 Exporting report → {export_path}");
            report::export_report(&cfg.json_output, export_path)?;
        }

        println!("✅ Done!  {} scenario(s)  ·  {} total requests  ·  {} ok  ·  {} errors",
            groups.len(), total, ok, err);

        if cfg.open_report {
            println!("\n🌐 Opening report in browser…");
            let path = std::path::PathBuf::from(&cfg.json_output);
            editor::run_report_viewer(path).await?;
        }
    }

    Ok(())
}
