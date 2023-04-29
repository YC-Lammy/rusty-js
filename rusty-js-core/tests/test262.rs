use std::io::*;

use rusty_js_core::Runtime;
use rusty_js_core::JValue;

#[test]
fn test(){
    let runtime = Runtime::new();

    runtime.clone().attach();

    let o = runtime.create_native_function(|_ctx, _this, args| {
        let mut o = std::io::stdout();
        o.write(args[0].to_string().as_bytes());
        o.write(b"\n");
        Ok(JValue::UNDEFINED)
    });

    runtime.declare_variable("log", o.into());

    let src = include_str!("harness.js");
    runtime.execute("", src);
}

