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

    println!("\n🚀 bench — scenario tree:");
    print_tree(&run_config.root, 0);
    println!();

    let results = runner::execute_node(&run_config.root, &run_config.run_params, 0).await?;

    println!(
        "\n📄 Generating {} report → {}",
        match run_config.output_format {
            config::OutputFormat::Html => "HTML",
            config::OutputFormat::Pdf => "PDF",
        },
        run_config.output_path
    );

    generate_report(&results, &run_config.output_format, &run_config.output_path)?;

    let total: u64 = results.iter().map(|r| r.total_requests).sum();
    let success: u64 = results.iter().map(|r| r.successful_requests).sum();
    let errors: u64 = results.iter().map(|r| r.error_requests).sum();
    println!(
        "✅ Done! {} scenarios  |  {} total requests  |  {} success  |  {} errors",
        results.len(),
        total,
        success,
        errors
    );

    Ok(())
}

/// Recursively print the scenario tree before execution so the user knows what will run.
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
            println!("{}→ {} {}  ({})", indent, r.method, r.url, r.name);
        }
    }
}


