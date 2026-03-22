mod cli;
mod config;
mod report;
mod runner;
mod stats;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use config::ScenarioNode;
use report::generate_report;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let run_config = args.into_run_config()?;

    // Print the scenario tree so the user knows what will run
    println!("\n🚀 bench — scenario tree:");
    print_tree(&run_config.root, 0);

    let run = &run_config.run_params;
    let iter_desc = match (run.requests, run.duration_secs) {
        (Some(n), _) => format!("{n} iterations"),
        (_, Some(s)) => format!("{s}s duration"),
        _ => "?".to_string(),
    };
    println!(
        "\n  concurrency: {}  ·  {}  ·  timeout: {}ms\n",
        run.concurrency, iter_desc, run.timeout_ms
    );

    let results = runner::run_scenario_tree(&run_config.root, &run_config.run_params).await?;

    // Print per-leaf summary
    println!();
    for r in &results {
        println!(
            "  ✓ {}  [{} {}]",
            r.name, r.method, r.url
        );
        println!(
            "    {:.1} req/s  |  {} total  |  {} success  |  {} failed  |  {} errors  |  p99: {:.2}ms",
            r.throughput_rps,
            r.total_requests,
            r.successful_requests,
            r.failed_requests,
            r.error_requests,
            r.latency_p99_ms,
        );
    }

    println!(
        "\n📄 Generating {} report → {}",
        match run_config.output_format {
            config::OutputFormat::Html => "HTML",
            config::OutputFormat::Pdf => "PDF",
        },
        run_config.output_path
    );

    generate_report(&results, &run_config.output_format, &run_config.output_path)?;

    let total_reqs: u64 = results.iter().map(|r| r.total_requests).sum();
    let total_success: u64 = results.iter().map(|r| r.successful_requests).sum();
    let total_errors: u64 = results.iter().map(|r| r.error_requests).sum();
    println!(
        "✅ Done!  {} leaf endpoint(s)  ·  {} requests total  ·  {} success  ·  {} errors",
        results.len(),
        total_reqs,
        total_success,
        total_errors,
    );

    Ok(())
}

/// Print the scenario tree in a readable indented format before execution.
fn print_tree(node: &ScenarioNode, depth: usize) {
    let indent = "  ".repeat(depth);
    match node {
        ScenarioNode::Group(g) => {
            println!("{}[{}] {}", indent, g.mode, g.name);
            for step in &g.steps {
                print_tree(step, depth + 1);
            }
        }
        ScenarioNode::Request(r) => {
            println!("{}→ {} {}  ({})", indent, r.method.to_uppercase(), r.url, r.name);
        }
    }
}



