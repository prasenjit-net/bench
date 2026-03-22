mod cli;
mod config;
mod report;
mod runner;
mod stats;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use report::generate_report;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let cfg = args.into_run_config()?;

    let run = &cfg.run;
    let iter_desc = match (run.requests, run.duration_secs) {
        (Some(n), _) => format!("{n} runs"),
        (_, Some(s)) => format!("{s}s duration"),
        _ => "?".to_string(),
    };

    println!("\n🚀 bench — \"{}\"", cfg.scenario.name);
    println!("  {} step(s) · {} · concurrency {} · timeout {}ms\n",
        cfg.scenario.steps.len(), iter_desc, run.concurrency, run.timeout_ms);

    for (i, step) in cfg.scenario.steps.iter().enumerate() {
        println!("  {}. {} {} ({})", i + 1, step.method.to_uppercase(), step.url, step.name);
    }
    println!();

    let results = runner::run(&cfg.scenario, &cfg.run).await?;

    println!();
    for r in &results {
        println!("  ✓ {}  [{} {}]", r.name, r.method, r.url);
        println!("    {:.1} req/s  ·  {} total  ·  {} ok  ·  {} fail  ·  {} err  ·  p99 {:.2}ms",
            r.throughput_rps, r.total_requests, r.successful_requests,
            r.failed_requests, r.error_requests, r.latency_p99_ms);
    }

    let fmt_name = match cfg.output_format {
        config::OutputFormat::Html => "HTML",
        config::OutputFormat::Pdf  => "PDF",
    };
    println!("\n📄 Generating {fmt_name} report → {}", cfg.output_path);
    generate_report(&results, &cfg.output_format, &cfg.output_path)?;

    let total: u64 = results.iter().map(|r| r.total_requests).sum();
    let ok: u64    = results.iter().map(|r| r.successful_requests).sum();
    let err: u64   = results.iter().map(|r| r.error_requests).sum();
    println!("✅ Done!  {} step(s) · {} requests · {} ok · {} errors", results.len(), total, ok, err);

    Ok(())
}
