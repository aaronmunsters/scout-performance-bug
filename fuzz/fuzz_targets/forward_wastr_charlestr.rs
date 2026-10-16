#![no_main]

use libfuzzer_sys::fuzz_target;
use scout_performance_bugs::ModuleTestCase;

fuzz_target!(|data: ModuleTestCase| {
    data.assert_wasmtime_forward_equality();
});
