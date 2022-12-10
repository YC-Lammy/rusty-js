use rusty_js_core::{JValue, Runtime};

fn main() {
    let runtime = Runtime::new();

    runtime.clone().attach();

    let o = runtime.create_native_function(|_ctx, _this, args| {
        println!("{}", args[0].as_number_uncheck());
        Ok(JValue::UNDEFINED)
    });
    runtime.declare_variable("log", o.into());

    let t = std::time::Instant::now();
    runtime
        .execute(
            "",
            r#"
    let i = 0;
    var a = 9;
    for (i=0;i<10000;i++){
        a += 1;
    };
    log(a);
    "#,
        )
        .unwrap();

    println!("{} ms", t.elapsed().as_secs_f64() * 1000.0);
}
