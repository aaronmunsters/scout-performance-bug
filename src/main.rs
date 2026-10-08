mod benchmark_comparison;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone)]
enum Command {
    Benchmark {
        #[arg(short, long)]
        runs: usize,
    },
    Fuzz,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Benchmark { runs } => benchmark_comparison::benchmark(runs)?,
        Command::Fuzz => todo!(),
    }

    Ok(())
}
