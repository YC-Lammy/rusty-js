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
            function p(n) {
                for (let i = 2;i * i <= n;i++) {
                    if (n % i == 0) {
                        return false;
                    }
                }
                return true;
            }
            
            let sum = 0;
            for (let k = 2;k < 100000;k++) {
                if (p(k)) {
                    sum++;
                }
            }
            log(sum)
    "#,
        )
        .unwrap();

    println!("{} ms", t.elapsed().as_secs_f64() * 1000.0);
}
