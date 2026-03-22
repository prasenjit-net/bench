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
    let run_config = args.into_run_config()?;

    println!(
        "\n🚀 bench — running {} scenario(s)\n",
        run_config.scenarios.len()
    );

    let mut results = Vec::with_capacity(run_config.scenarios.len());

    for scenario in &run_config.scenarios {
        println!(
            "▶ Running: {} [{} {}]  concurrency={}",
            scenario.name, scenario.method, scenario.url, scenario.concurrency
        );
        let result = runner::run_scenario(scenario).await?;
        println!(
            "  ✓ {:.1} req/s  |  success: {}  failed: {}  errors: {}  p99: {:.2}ms\n",
            result.throughput_rps,
            result.successful_requests,
            result.failed_requests,
            result.error_requests,
            result.latency_p99_ms,
        );
        results.push(result);
    }

    println!(
        "📄 Generating {} report → {}",
        match run_config.output_format {
            config::OutputFormat::Html => "HTML",
            config::OutputFormat::Pdf => "PDF",
        },
        run_config.output_path
    );

    generate_report(&results, &run_config.output_format, &run_config.output_path)?;

    println!("✅ Done!");
    Ok(())
}

