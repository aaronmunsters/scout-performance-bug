// 1. run & ⏱️ an input program
// 2. run & ⏱️ an input program instrumented with wastrumentation
// 3. run & ⏱️ an input program instrumented with charlestrumentation

use std::ffi::OsString;
use std::fs::{File, read_dir};
use std::io::Write;
use std::path::{Path, absolute};
use std::time::Duration;

use anyhow::Result;
const PROGRAM_ORDER: &[&str] = &[
    "rtexpacker.wasm",
    "jqkungfu.wasm",
    "rtexviewer.wasm",
    "game-of-life.wasm",
    "factorial.wasm",
    "figma-startpage.wasm",
    "hydro.wasm",
    "ffmpeg.wasm",
    "parquet.wasm",
    "pacalc.wasm",
    "sqlgui.wasm",
    "riconpacker.wasm",
    "jsc.wasm",
    "boa.wasm",
    "rguistyler.wasm",
    "guiicons.wasm",
    "bullet.wasm",
    "rfxgen.wasm",
    "rguilayout.wasm",
    "funky-kart.wasm",
    "sandspiel.wasm",
    "pathfinding.wasm",
    "commanderkeen.wasm",
    "fib.wasm",
    "mandelbrot.wasm",
    "multiplyInt.wasm",
    "multiplyDouble.wasm",
];

fn main() -> Result<()> {
    let mut measures = File::create_new("measures.csv")?;

    let runs = 1;

    let mut input_programs = fetch_input_programs()?;

    let analyses: &[(&str, std::path::PathBuf)] = &[
        ("forward", absolute("./analyses/forward/Cargo.toml")?),
        (
            "generic-apply",
            absolute("./analyses/generic-apply/Cargo.toml")?,
        ),
    ];

    // [input_program, analysis] -> Vec<(variant, name)>
    let named_variants_for = |input_program: &[u8], analysis: &Path| {
        let res: anyhow::Result<_> = (|| {
            let charlestrumented_start_dis =
                charlestrument(input_program, analysis, Start::Disabled)?;
            let charlestrumented_start_en =
                charlestrument(input_program, analysis, Start::Enabled)?;
            let wastrumented = wastrument(input_program, analysis)?;

            Ok(vec![
                (charlestrumented_start_dis, "charlestrumented_start_dis"),
                (charlestrumented_start_en, "charlestrumented_start_en"),
                (wastrumented, "wastrumented"),
            ])
        })();
        res
    };

    // [input_program_variant] -> Vec<(engine_name, engine_duration)>
    let benchmark_for_variant = |input_program: &[u8]| {
        let res: anyhow::Result<_> = (|| {
            Ok(vec![
                ("wasmer", time_wasmer(input_program)?),
                ("wasmtime", time_wasmtime(input_program)?),
            ])
        })();
        res
    };

    input_programs.sort_by_key(|(name, _)| {
        PROGRAM_ORDER
            .iter()
            .position(|wanted| wanted == &name.to_string_lossy())
            .unwrap_or(usize::MAX)
    });

    for (input_program_name, input_program) in input_programs {
        for _run in 0..runs {
            // Baseline
            for (engine_name, engine_duration) in benchmark_for_variant(&input_program)? {
                let duration_number = engine_duration.as_nanos();
                let duration_format = "nanos";
                let input_program_name = input_program_name.display(); // format name
                writeln!(
                    measures,
                    "{input_program_name},no_analysis,no_instrumentation_platform,{engine_name},{duration_number},{duration_format}"
                )?;
            }

            // Instrumented
            for (analysis_name, analysis) in analyses {
                for (variant, inst_variant_name) in named_variants_for(&input_program, analysis)? {
                    for (engine_name, engine_duration) in benchmark_for_variant(&variant)? {
                        let duration_number = engine_duration.as_nanos();
                        let duration_format = "nanos";
                        let input_program_name = input_program_name.display(); // format name
                        writeln!(
                            measures,
                            "{input_program_name},{analysis_name},{inst_variant_name},{engine_name},{duration_number},{duration_format}"
                        )?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn fetch_input_programs() -> anyhow::Result<Vec<(OsString, Vec<u8>)>> {
    let dir = read_dir("./wasm-r3-bench")?;
    let programs = dir
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| std::fs::read(entry.path()).map(|content| (entry.file_name(), content)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(programs)
}

fn time_wasmer(input_program: &[u8]) -> anyhow::Result<Duration> {
    // WASMER
    let binary = input_program;
    let engine = wasmer::Engine::default();
    let module = wasmer::Module::from_binary(&engine, binary)?;
    let mut store = wasmer::Store::new(engine);
    let imports = wasmer::Imports::new();
    let instance = wasmer::Instance::new(&mut store, &module, &imports)?;
    let start = instance.exports.get_function("_start")?;
    /////////////////
    ///// BENCH /////
    /////////////////
    let before_bench = std::time::Instant::now();
    start.call(&mut store, &[])?;
    Ok(before_bench.elapsed())
}

fn time_wasmtime(input_program: &[u8]) -> anyhow::Result<Duration> {
    // WASMTIME
    let binary = input_program;
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::from_binary(&engine, binary)?;
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[])?;
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    /////////////////
    ///// BENCH /////
    /////////////////
    let before_bench = std::time::Instant::now();
    start.call(&mut store, ())?;
    Ok(before_bench.elapsed())
}

fn wastrument(input_program: &[u8], analysis: &Path) -> anyhow::Result<Vec<u8>> {
    use wastrumentation::{Wastrumenter, compiler::Compiles};
    use wastrumentation_rust::compile::compiler::Compiler;
    use wastrumentation_rust::compile::options::RustSource::Manifest;
    use wastrumentation_rust::generate::analysis::Hook;

    let wastrumenter = {
        let an_compiler = Compiler::setup_compiler()?;
        let in_compiler = Compiler::setup_compiler()?;
        Wastrumenter::new(Box::new(an_compiler), Box::new(in_compiler))
    };

    let source = Manifest(
        wastrumentation_rust::compile::options::WasiSupport::Disabled,
        absolute(analysis)?,
    );
    let hooks = Hook::all_hooks();
    let analysis =
        wastrumentation_rust::generate::analysis::RustAnalysisSpec { source, hooks }.into();

    let configuration = &wastrumentation::Configuration {
        target_indices: None,
        primary_selection: None,
    };

    let instrumented = wastrumenter
        .wastrument(input_program, analysis, configuration)
        .map_err(|e| anyhow::format_err!("{e:?}"))?;

    Ok(instrumented)
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
enum Start {
    Disabled,
    Enabled,
}

fn charlestrument(input_program: &[u8], analysis: &Path, start: Start) -> anyhow::Result<Vec<u8>> {
    use charlestrumentation::{Wastrumenter, compiler::Compiles};
    use charlestrumentation_rust::compile::compiler::Compiler;
    use charlestrumentation_rust::compile::options::RustSource::Manifest;
    use charlestrumentation_rust::generate::analysis::Hook;

    let wastrumenter = {
        let an_compiler = Compiler::setup_compiler()?;
        let in_compiler = Compiler::setup_compiler()?;
        Wastrumenter::new(Box::new(an_compiler), Box::new(in_compiler))
    };

    let source = Manifest(
        charlestrumentation_rust::compile::options::WasiSupport::Disabled,
        absolute(analysis)?,
    );
    let hooks = Hook::all_hooks();
    let analysis =
        charlestrumentation_rust::generate::analysis::RustAnalysisSpec { source, hooks }.into();

    let configuration = &charlestrumentation::Configuration {
        target_indices: None,
        primary_selection: None,
        start_disabled: start == Start::Disabled,
    };

    let instrumented = wastrumenter
        .wastrument(input_program, analysis, configuration)
        .map_err(|e| anyhow::format_err!("{e:?}"))?;

    Ok(instrumented)
}
