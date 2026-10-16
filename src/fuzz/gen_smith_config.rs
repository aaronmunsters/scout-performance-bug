use wasm_smith::Config;

pub(super) fn gen_config() -> Config {
    wasm_smith::Config {
        export_everything: true,

        // Disable unsupported extensions
        exceptions_enabled: false,
        gc_enabled: false,
        compact_imports_enabled: false,
        custom_descriptors_enabled: false,
        custom_page_sizes_enabled: false,
        generate_custom_sections: false,
        threads_enabled: false,

        // No imports
        available_imports: None,
        max_imports: 0,
        min_imports: 0,

        allow_start_export: true,
        // max_nesting_depth: 100, // optionally more than the default
        memory64_enabled: false,
        min_exports: 1,

        max_tags: 0,
        min_tags: 0,
        min_types: 0,

        // enabled
        bulk_memory_enabled: true,
        disallow_traps: true,

        // unsure
        multi_value_enabled: false,
        reference_types_enabled: false,
        saturating_float_to_int_enabled: false,
        sign_extension_ops_enabled: false,
        tail_call_enabled: false,
        wide_arithmetic_enabled: false,
        extended_const_enabled: false,

        // disabled
        relaxed_simd_enabled: false,
        shared_everything_threads_enabled: false,
        simd_enabled: false,
        table_max_size_required: false,
        allow_invalid_funcs: false,

        // For the rest; keep default
        ..Default::default()
    }
}
