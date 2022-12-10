use std::sync::Arc;
use std::io::Write;

use swc_common::FileName;
use swc_common::SourceFile;

use crate::error::Error;
use crate::runtime::Runtime;
use crate::types::JValue;

mod bridge;
mod llvm;

macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(feature = "logging")]
        $crate::debug::logging(&format!($($arg)*))
    };
}

pub(crate) use debug;

#[cfg(test)]
#[inline]
pub fn logging(s: &str) {
    //println!("{}", s);
}

#[cfg(not(test))]
#[inline]
pub fn logging(s: &str) {}

pub fn parse_script(script: &str) -> swc_ecmascript::ast::Script {
    let src = SourceFile::new(
        FileName::Anon,
        false,
        FileName::Anon,
        script.to_string(),
        swc_common::BytePos(1),
    );
    let mut v = Vec::new();
    let re = swc_ecmascript::parser::parse_file_as_script(
        &src,
        swc_ecmascript::parser::Syntax::Es(swc_ecmascript::parser::EsConfig {
            jsx: false,
            fn_bind: true,
            decorators: true,
            decorators_before_export: true,
            export_default_from: true,
            import_assertions: true,
            private_in_object: true,
            allow_super_outside_method: false,
            allow_return_outside_function: false,
        }),
        swc_ecmascript::ast::EsVersion::Es2022,
        None,
        &mut v,
    );

    re.unwrap()
}

#[test]
pub fn test_native_function() {
    let runtime = Runtime::new();

    runtime.clone().attach();

    let a = std::sync::Arc::new(std::cell::Cell::new(0));

    let a1 = a.clone();
    let func = runtime.create_native_function(move |ctx, this, args| {
        a1.set(a1.get() + 1);
        // expected 9 and 0
        println!(
            "{:?},{:?},{:?},{:?},{:?}",
            args.get(0).map(|v| v.get_property("o", ctx)),
            args.get(0).map(|v| v.get_property("p", ctx)),
            args.get(1),
            args.get(2),
            args.len()
        );
        println!("hello world!");
        Ok(JValue::UNDEFINED)
    });

    runtime.to_mut().declare_variable("hello", func.into());

    let t = std::time::Instant::now();
    runtime
        .clone()
        .execute(
            "hello_world",
            include_str!("../../bench/benchv8-v7/base.js"),
        )
        .map_err(|e| {
            match e {
                Error::Value(v) => {
                    println!("{}", v.to_string());
                }
                e => {
                    println!("{}", e.to_string());
                }
            };
        })
        .unwrap();
    println!("{}ms", t.elapsed().as_secs_f64() * 1000.0);
    println!("{}", a.get());

    drop(runtime);
}

#[test]
fn test2() {
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

#[test]
fn test3() {
    let runtime = Runtime::new();

    runtime.clone().attach();

    let n = runtime.create_native_function(|_ctx, _this, _args| {
        Ok(JValue::create_number(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as f64,
        ))
    });

    runtime.declare_let_variable("now", n.into());

    let o = runtime.create_native_function(|_ctx, _this, args| {
        let mut o = std::io::stdout();
        o.write(args[0].to_string().as_bytes());
        o.write(b"\n");
        Ok(JValue::UNDEFINED)
    });

    runtime.declare_variable("log", o.into());

    let t = std::time::Instant::now();
    runtime.clone()
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
                if (p(k)){
                    sum++;
                }
            }
            log(sum)
    "#,
        )
        .unwrap();

    println!("{} ms", t.elapsed().as_secs_f64() * 1000.0);
    Runtime::deattach();
}
