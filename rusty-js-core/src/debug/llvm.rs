use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::module::Module;
use inkwell::OptimizationLevel;
use std::error::Error;

use crate::{operations, JValue};

/// Convenience type alias for the `sum` function.
///
/// Calling this is innately `unsafe` because there's no guarantee it doesn't
/// do `unsafe` operations internally.
type SumFunc = unsafe extern "C" fn(u64, u64, u64) -> u64;


#[test]
fn test() {
    use inkwell::targets::{InitializationConfig, Target};
    use inkwell::context::Context;
    use inkwell::OptimizationLevel;
    
    #[repr(C)]
    struct Result{
        value: u64,
        flag: bool,
    }

    #[no_mangle]
    extern fn return_struct() -> Result{
        Result { value: 2, flag: true }
    }

    Target::initialize_native(&InitializationConfig::default()).unwrap();

    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();
    let ee = module.create_jit_execution_engine(OptimizationLevel::None).unwrap();

    let i64t = context.i64_type();
    let bt = context.bool_type();
    let ft = context.f64_type();
    let fnt = i64t.fn_type(&[], false);

    let f = module.add_function("test_fn", fnt, None);
    let b = context.append_basic_block(f, "entry");

    builder.position_at_end(b);

    let func_ty = context.struct_type(&[i64t.into(), bt.into()], false);
    let extf = module.add_function("struct_fn", func_ty.fn_type(&[], false), None);
    ee.add_global_mapping(&extf, return_struct as usize);

    let call_site_value = builder.build_call(extf, &[], "retv");
    let retv = call_site_value.try_as_basic_value().left().unwrap().into_struct_value();

    let value = builder.build_extract_value(retv, 0, "").unwrap().into_int_value();
    let flag = builder.build_extract_value(retv, 1, "").unwrap().into_int_value();

    // do something...

    builder.build_return(Some(&value));

    
    unsafe{
        let value = ee.run_function(f, &[]).as_int(false);
        assert!(value == 2)
    }
}
