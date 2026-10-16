pub(super) fn charlestrument_forward(target: &[u8]) -> anyhow::Result<Vec<u8>> {
    use charlestrumentation::Configuration;
    use charlestrumentation::Wastrumenter;
    use charlestrumentation_wasm::compile::compiler::Compiler;
    use charlestrumentation_wasm::compile::options::WebAssemblySource;
    use charlestrumentation_wasm::generate::analysis::Hook;
    use charlestrumentation_wasm::generate::analysis::WasmAnalysisSpec;

    let in_comp = Box::new(Compiler);
    let an_comp = Box::new(Compiler);
    let wastrumenter = Wastrumenter::new(in_comp, an_comp);

    let forward_analysis = include_bytes!(
        "../../analyses/forward/target/wasm32-unknown-unknown/release/rust_all_hooks_analysis.wasm"
    );
    let analysis = WasmAnalysisSpec {
        source: WebAssemblySource::Module(forward_analysis.into()),
        hooks: Hook::all_hooks(),
    }
    .into();
    let configuration = Configuration {
        target_indices: None,
        primary_selection: None,
        start_disabled: true,
    };

    let instrumented = wastrumenter
        .wastrument(target, analysis, &configuration)
        .map_err(|e| anyhow::format_err!("{e:?}"))?;

    Ok(instrumented)
}

pub(super) fn wastrument_forward(target: &[u8]) -> anyhow::Result<Vec<u8>> {
    use wastrumentation::Configuration;
    use wastrumentation::Wastrumenter;
    use wastrumentation_wasm::compile::compiler::Compiler;
    use wastrumentation_wasm::compile::options::WebAssemblySource;
    use wastrumentation_wasm::generate::analysis::Hook;
    use wastrumentation_wasm::generate::analysis::WasmAnalysisSpec;

    let in_comp = Box::new(Compiler);
    let an_comp = Box::new(Compiler);
    let wastrumenter = Wastrumenter::new(in_comp, an_comp);

    let forward_analysis = include_bytes!(
        "../../analyses/forward/target/wasm32-unknown-unknown/release/rust_all_hooks_analysis.wasm"
    );
    let analysis = WasmAnalysisSpec {
        source: WebAssemblySource::Module(forward_analysis.into()),
        hooks: Hook::all_hooks(),
    }
    .into();
    let configuration = Configuration {
        target_indices: None,
        primary_selection: None,
    };

    let instrumented = wastrumenter
        .wastrument(target, analysis, &configuration)
        .map_err(|e| anyhow::format_err!("{e:?}"))?;

    Ok(instrumented)
}
