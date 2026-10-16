use arbitrary::Arbitrary;
use clap::{Parser, Subcommand};
use rand::SeedableRng;
use scout_performance_bugs::benchmark_comparison;
use scout_performance_bugs::fuzz::ModuleTestCase;

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
        Command::Fuzz => fuzz()?,
    }

    Ok(())
}

// TODO: The code below could be parallelised
fn fuzz() -> anyhow::Result<()> {
    let mut counter = 0;
    let seed: [u8; 32] = *b"Well well, what do we have here!";
    let mut rng = rand::rngs::Xoshiro256PlusPlus::from_seed(seed);

    loop {
        let input: &mut [u8] = &mut [0; 8096];

        rand::Fill::fill_slice(input, &mut rng);
        println!("[{counter}] - [{:?}]", &input[0..30]);
        counter += 1;
        let mut u = arbitrary::Unstructured::new(input);
        let test_case = ModuleTestCase::arbitrary(&mut u)?;
        let engine = wasmtime::Engine::default();
        test_case.assert_wasmtime_forward_equality_only_success(&engine)?;
    }
}
