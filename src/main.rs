use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{File, read_dir};
use std::io::Write;
use std::path::{PathBuf, absolute};
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

    let runs = 5;

    let mut input_programs = fetch_input_programs()?;

    let analyses: &[(&str, AnalysisSpec)] = &[
        (
            "forward",
            AnalysisSpec {
                path: absolute("./analyses/forward/Cargo.toml")?,
                hooks: Hook::all_hooks(),
            },
        ),
        (
            "generic-apply",
            AnalysisSpec {
                path: absolute("./analyses/generic-apply/Cargo.toml")?,
                hooks: HashSet::from([Hook::GenericApply]),
            },
        ),
    ];

    // [input_program, analysis] -> Vec<(variant, name)>
    let named_variants_for = |input_program: &[u8], analysis: &AnalysisSpec| {
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

fn wastrument(input_program: &[u8], analysis: &AnalysisSpec) -> anyhow::Result<Vec<u8>> {
    use wastrumentation::{Wastrumenter, compiler::Compiles};
    use wastrumentation_rust::compile::compiler::Compiler;
    use wastrumentation_rust::compile::options::RustSource::Manifest;

    let wastrumenter = {
        let an_compiler = Compiler::setup_compiler()?;
        let in_compiler = Compiler::setup_compiler()?;
        Wastrumenter::new(Box::new(an_compiler), Box::new(in_compiler))
    };

    let source = Manifest(
        wastrumentation_rust::compile::options::WasiSupport::Disabled,
        absolute(&analysis.path)?,
    );
    let hooks = analysis.hooks.iter().map(Into::into).collect();
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

struct AnalysisSpec {
    path: PathBuf,
    hooks: HashSet<Hook>,
}

fn charlestrument(
    input_program: &[u8],
    analysis: &AnalysisSpec,
    start: Start,
) -> anyhow::Result<Vec<u8>> {
    use charlestrumentation::{Wastrumenter, compiler::Compiles};
    use charlestrumentation_rust::compile::compiler::Compiler;
    use charlestrumentation_rust::compile::options::RustSource::Manifest;

    let wastrumenter = {
        let an_compiler = Compiler::setup_compiler()?;
        let in_compiler = Compiler::setup_compiler()?;
        Wastrumenter::new(Box::new(an_compiler), Box::new(in_compiler))
    };

    let source = Manifest(
        charlestrumentation_rust::compile::options::WasiSupport::Disabled,
        absolute(&analysis.path)?,
    );
    let hooks = analysis.hooks.iter().map(Into::into).collect();
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

#[derive(Debug, Hash, PartialEq, Eq)]
enum Hook {
    GenericApply,
    CallPre,
    CallPost,
    CallIndirectPre,
    CallIndirectPost,
    IfThen,
    IfThenPost,
    IfThenElse,
    IfThenElsePost,
    Branch,
    BranchIf,
    BranchTable,
    Select,
    Unary,
    Binary,
    Drop,
    Return,
    Const,
    Local,
    Global,
    Store,
    Load,
    MemorySize,
    MemoryGrow,
    MemoryInit,
    MemoryCopy,
    MemoryFill,
    BlockPre,
    BlockPost,
    LoopPre,
    LoopPost,
}

impl Hook {
    fn all_hooks() -> HashSet<Self> {
        use Hook::*;
        HashSet::from([
            GenericApply,
            CallPre,
            CallPost,
            CallIndirectPre,
            CallIndirectPost,
            IfThen,
            IfThenPost,
            IfThenElse,
            IfThenElsePost,
            Branch,
            BranchIf,
            BranchTable,
            Select,
            Unary,
            Binary,
            Drop,
            Return,
            Const,
            Local,
            Global,
            Store,
            Load,
            MemorySize,
            MemoryGrow,
            MemoryInit,
            MemoryCopy,
            MemoryFill,
            BlockPre,
            BlockPost,
            LoopPre,
            LoopPost,
        ])
    }
}

impl Into<charlestrumentation_rust::generate::analysis::Hook> for &Hook {
    fn into(self) -> charlestrumentation_rust::generate::analysis::Hook {
        use charlestrumentation_rust::generate::analysis::Hook as DepHook;
        match self {
            Hook::GenericApply => DepHook::GenericApply,
            Hook::CallPre => DepHook::CallPre,
            Hook::CallPost => DepHook::CallPost,
            Hook::CallIndirectPre => DepHook::CallIndirectPre,
            Hook::CallIndirectPost => DepHook::CallIndirectPost,
            Hook::IfThen => DepHook::IfThen,
            Hook::IfThenPost => DepHook::IfThenPost,
            Hook::IfThenElse => DepHook::IfThenElse,
            Hook::IfThenElsePost => DepHook::IfThenElsePost,
            Hook::Branch => DepHook::Branch,
            Hook::BranchIf => DepHook::BranchIf,
            Hook::BranchTable => DepHook::BranchTable,
            Hook::Select => DepHook::Select,
            Hook::Unary => DepHook::Unary,
            Hook::Binary => DepHook::Binary,
            Hook::Drop => DepHook::Drop,
            Hook::Return => DepHook::Return,
            Hook::Const => DepHook::Const,
            Hook::Local => DepHook::Local,
            Hook::Global => DepHook::Global,
            Hook::Store => DepHook::Store,
            Hook::Load => DepHook::Load,
            Hook::MemorySize => DepHook::MemorySize,
            Hook::MemoryGrow => DepHook::MemoryGrow,
            Hook::MemoryInit => DepHook::MemoryInit,
            Hook::MemoryCopy => DepHook::MemoryCopy,
            Hook::MemoryFill => DepHook::MemoryFill,
            Hook::BlockPre => DepHook::BlockPre,
            Hook::BlockPost => DepHook::BlockPost,
            Hook::LoopPre => DepHook::LoopPre,
            Hook::LoopPost => DepHook::LoopPost,
        }
    }
}

impl Into<wastrumentation_rust::generate::analysis::Hook> for &Hook {
    fn into(self) -> wastrumentation_rust::generate::analysis::Hook {
        use wastrumentation_rust::generate::analysis::Hook as DepHook;
        match self {
            Hook::GenericApply => DepHook::GenericApply,
            Hook::CallPre => DepHook::CallPre,
            Hook::CallPost => DepHook::CallPost,
            Hook::CallIndirectPre => DepHook::CallIndirectPre,
            Hook::CallIndirectPost => DepHook::CallIndirectPost,
            Hook::IfThen => DepHook::IfThen,
            Hook::IfThenPost => DepHook::IfThenPost,
            Hook::IfThenElse => DepHook::IfThenElse,
            Hook::IfThenElsePost => DepHook::IfThenElsePost,
            Hook::Branch => DepHook::Branch,
            Hook::BranchIf => DepHook::BranchIf,
            Hook::BranchTable => DepHook::BranchTable,
            Hook::Select => DepHook::Select,
            Hook::Unary => DepHook::Unary,
            Hook::Binary => DepHook::Binary,
            Hook::Drop => DepHook::Drop,
            Hook::Return => DepHook::Return,
            Hook::Const => DepHook::Const,
            Hook::Local => DepHook::Local,
            Hook::Global => DepHook::Global,
            Hook::Store => DepHook::Store,
            Hook::Load => DepHook::Load,
            Hook::MemorySize => DepHook::MemorySize,
            Hook::MemoryGrow => DepHook::MemoryGrow,
            Hook::MemoryInit => DepHook::MemoryInit,
            Hook::MemoryCopy => DepHook::MemoryCopy,
            Hook::MemoryFill => DepHook::MemoryFill,
            Hook::BlockPre => DepHook::BlockPre,
            Hook::BlockPost => DepHook::BlockPost,
            Hook::LoopPre => DepHook::LoopPre,
            Hook::LoopPost => DepHook::LoopPost,
        }
    }
}
