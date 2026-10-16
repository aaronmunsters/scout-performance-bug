use std::assert_eq;

use arbitrary::Arbitrary;

mod gen_smith_config;
mod instrument_forward;
mod some_random_data;

#[derive(Debug)]
pub struct ModuleTestCase {
    pub module: Vec<u8>,
    pub invocations: Vec<(String, Vec<Val>)>,
}

impl ModuleTestCase {
    pub fn from_valid_module(module: Vec<u8>) -> anyhow::Result<Self> {
        let data = &some_random_data::some_random_data();
        let mut unstructured = arbitrary::Unstructured::new(data);
        let invocations = ModuleTestCase::invocations_for(&mut unstructured, &module)?;
        Ok(ModuleTestCase {
            module,
            invocations,
        })
    }
}

impl<'a> ModuleTestCase {
    fn invocations_for(
        u: &mut arbitrary::Unstructured<'a>,
        binary: &[u8],
    ) -> arbitrary::Result<Vec<(String, Vec<Val>)>> {
        let engine = wasmtime::Engine::default();
        let module = wasmtime::Module::from_binary(&engine, binary)
            .map_err(|_| arbitrary::Error::IncorrectFormat)?;

        let applicable: Vec<(String, Vec<ValType>)> = (module)
            .exports()
            .flat_map(|export| {
                export
                    .ty()
                    .func()
                    .iter()
                    .filter_map(|func| {
                        let params = func.params().try_fold(vec![], |mut prs, param| {
                            let ty = match param {
                                wasmtime::ValType::I32 => ValType::I32,
                                wasmtime::ValType::I64 => ValType::I64,
                                wasmtime::ValType::F32 => ValType::F32,
                                wasmtime::ValType::F64 => ValType::F64,
                                wasmtime::ValType::V128 => ValType::V128,
                                wasmtime::ValType::Ref(_) => return None,
                            };
                            prs.push(ty);
                            Some(prs)
                        })?;
                        Some((export.name().to_string(), params))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let applicable_view = applicable
            .iter()
            .map(|(n, vs)| (n.as_str(), &vs[..]))
            .collect::<Vec<(&str, &[ValType])>>()
            .repeat(10);

        applicable_view
            .into_iter()
            .map(|(name, types)| {
                types
                    .iter()
                    .map(|ty| ty.into_with_arbitrary(u))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|values| (name.to_string(), values))
            })
            .collect::<Result<Vec<_>, _>>()
    }
}

impl<'a> arbitrary::Arbitrary<'a> for ModuleTestCase {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        use wasm_smith::Config;
        use wasm_smith::Module;
        let config: Config = gen_smith_config::gen_config();
        let mut module = Module::new(config, u)?;

        module
            .ensure_termination(u32::pow(2, 8))
            .map_err(|_| arbitrary::Error::NotEnoughData)?;
        let module = module.to_bytes();
        let invocations = ModuleTestCase::invocations_for(u, &module)?;

        Ok(ModuleTestCase {
            module,
            invocations,
        })
    }
}

#[derive(Copy, Clone)]
enum ValType {
    I32,
    I64,
    F32,
    F64,
    V128,
}

#[derive(Debug, Arbitrary, PartialEq)]
pub enum Val {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    V128(u128),
}

impl From<&Val> for wasmtime::Val {
    fn from(value: &Val) -> Self {
        match value {
            Val::I32(v) => (*v).into(),
            Val::I64(v) => (*v).into(),
            Val::F32(v) => (*v).into(),
            Val::F64(v) => (*v).into(),
            Val::V128(v) => (*v).into(),
        }
    }
}

impl ValType {
    fn into_with_arbitrary<'a>(
        self,
        u: &mut arbitrary::Unstructured<'a>,
    ) -> arbitrary::Result<Val> {
        Val::arbitrary(u, &self)
    }
}

impl Val {
    fn arbitrary<'a>(u: &mut arbitrary::Unstructured<'a>, ty: &ValType) -> arbitrary::Result<Self> {
        let v = match ty {
            ValType::I32 => Val::I32(i32::arbitrary(u)?),
            ValType::I64 => Val::I64(i64::arbitrary(u)?),
            ValType::F32 => Val::F32(f32::arbitrary(u)?),
            ValType::F64 => Val::F64(f64::arbitrary(u)?),
            ValType::V128 => Val::V128(u128::arbitrary(u)?),
        };
        Ok(v)
    }
}

impl ModuleTestCase {
    pub fn assert_wasmtime_forward_equality_only_success(
        &self,
        engine: &wasmtime::Engine,
    ) -> anyhow::Result<()> {
        let uninstrumented = &self.module;
        let wastrumented = instrument_forward::wastrument_forward(uninstrumented)?;
        let charlestrumented = instrument_forward::charlestrument_forward(uninstrumented)?;

        // 1. Collect 'truths':
        let truths = collect_success_outcomes(engine, uninstrumented, &self.invocations);

        for (platform_outcome, platform_name) in &[
            (wastrumented, "wastrumentation"),
            (charlestrumented, "charlestrumentation"),
        ] {
            // 2. Collect Wastrumentation outcome
            let instrumentation_outcomes =
                collect_success_outcomes(engine, platform_outcome, &self.invocations);

            for (truth_outcome, instrumentation_outcome) in
                truths.iter().zip(instrumentation_outcomes.iter())
            {
                let (truth_name, truth_vals) = truth_outcome;
                let (instrumentation_name, instrumentation_vals) = instrumentation_outcome;

                assert_eq!(truth_name, instrumentation_name);
                for (truth_val, instrumentation_val) in truth_vals.iter().zip(instrumentation_vals)
                {
                    #[cfg(test)]
                    {
                        std::println!("Checking equality on platform {platform_name}");
                    }

                    match (truth_val, instrumentation_val) {
                        (wasmtime::Val::I32(t), wasmtime::Val::I32(o)) => {
                            assert_eq!(t, o, "{platform_name}")
                        }
                        (wasmtime::Val::I64(t), wasmtime::Val::I64(o)) => {
                            assert_eq!(t, o, "{platform_name}")
                        }
                        (wasmtime::Val::F32(t), wasmtime::Val::F32(o)) => {
                            assert_eq!(t, o, "{platform_name}")
                        }
                        (wasmtime::Val::F64(t), wasmtime::Val::F64(o)) => {
                            assert_eq!(t, o, "{platform_name}")
                        }
                        (wasmtime::Val::V128(t), wasmtime::Val::V128(o)) => {
                            assert_eq!(t, o, "{platform_name}")
                        }
                        _ => {} // ignore
                    }
                }
            }
        }

        Ok(())
    }
}

fn collect_success_outcomes(
    engine: &wasmtime::Engine,
    program: &[u8],
    invocations: &[(String, Vec<Val>)],
) -> Vec<(String, Vec<wasmtime::Val>)> {
    invocations
        .iter()
        .filter_map(|invocation| {
            let (name, values) = invocation;
            let name = name.as_str();
            let vals = &values[..];
            match collect_outcome(engine, program, (name, vals)) {
                Err(_) => None,
                Ok(res) => Some((name.to_string(), res)),
            }
        })
        .collect()
}

fn collect_outcome(
    engine: &wasmtime::Engine,
    program: &[u8],
    invocation: (&str, &[Val]),
) -> anyhow::Result<Vec<wasmtime::Val>> {
    let binary = &program;
    let module = wasmtime::Module::from_binary(engine, binary)?;
    let mut store = wasmtime::Store::new(engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[])?;

    let (name, params) = invocation;
    let export = instance
        .get_export(&mut store, name)
        .ok_or(anyhow::format_err!("{name} not found in module"))?;
    let export_func = export
        .into_func()
        .ok_or(anyhow::format_err!("{name} not a function in module"))?;
    let params = params.iter().map(Into::into).collect::<Vec<_>>();
    let mut results: Vec<wasmtime::Val> =
        vec![wasmtime::Val::I32(0); export_func.ty(&mut store).results().len()];
    export_func.call(&mut store, &params, &mut results)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, dbg};

    use super::ModuleTestCase;
    use arbitrary::Arbitrary;
    use wasmtime::Engine;

    #[test]
    fn test() -> anyhow::Result<()> {
        let engine = Engine::default();
        let input_programs = crate::benchmark_comparison::fetch_input_programs()?;
        for (program_name, program) in input_programs {
            std::println!("Working on {program_name:?}");
            if program_name != "boa.wasm" {
                continue;
            }
            let test_case = ModuleTestCase::from_valid_module(program)?;
            test_case.assert_wasmtime_forward_equality_only_success(&engine)?;
        }
        Ok(())
    }

    #[test]
    fn test_crash() -> anyhow::Result<()> {
        let engine = Engine::default();
        let input = include_bytes!(
            "../../fuzz/artifacts/forward_wastr_charlestr/slow-unit-0a6b55e10828755c11e4316d875701164414722b"
        );
        let mut u = arbitrary::Unstructured::new(input);
        let test_case = ModuleTestCase::arbitrary(&mut u)?;
        test_case.assert_wasmtime_forward_equality_only_success(&engine)?;

        Ok(())
    }

    const SIMPLE_ADD_WAT: &str = r#"
            (module
                (func $add (param $a i32) (param $b i32) (result i32)
                    (local.get $a)
                    (local.get $b)
                    (i32.add))
                (export "add" (func $add))
            )"#;

    #[test]
    fn test_simple_add() -> anyhow::Result<()> {
        let engine = Engine::default();
        let input_program = wat::parse_str(SIMPLE_ADD_WAT)?;
        let test_case = ModuleTestCase::from_valid_module(input_program)?;
        std::dbg!(&test_case);
        test_case.assert_wasmtime_forward_equality_only_success(&engine)
    }

    #[test]
    fn test_invokes_are_unique() -> anyhow::Result<()> {
        let input_program = wat::parse_str(SIMPLE_ADD_WAT)?;
        let test_case = ModuleTestCase::from_valid_module(input_program)?;
        let test_cases: HashMap<String, Vec<Vec<super::Val>>> = test_case
            .invocations
            .into_iter()
            .fold(HashMap::new(), |mut m, (n, vs)| {
                m.entry(n).or_default().push(vs);
                m
            });

        #[allow(clippy::zero_prefixed_literal)]
        {
            use super::Val::I32;
            assert_eq!(
                test_cases
                    .get("add")
                    .ok_or(anyhow::format_err!("add not found in module"))?,
                &[
                    [I32(-0982667468), I32(-0345549730)],
                    [I32(-1642258738), I32(00683261732)],
                    [I32(01776651048), I32(01112332628)],
                    [I32(-1983420793), I32(00186682098)],
                    [I32(00086650913), I32(-0167721533)],
                    [I32(-0211406083), I32(01654458037)],
                    [I32(-0654747758), I32(00235270121)],
                    [I32(-1722095731), I32(-0678431394)],
                    [I32(01942301316), I32(00638810663)],
                    [I32(-0592525113), I32(-1869132212)],
                ],
            );
        }
        dbg!(&test_cases);

        for (_name, mut invocations) in test_cases.into_iter() {
            assert!(invocations.len() > 1); // Otherwise the next assertion does not make sense
            while let Some(invocation) = invocations.pop() {
                assert!(!invocations.contains(&invocation));
            }
        }
        Ok(())
    }
}
