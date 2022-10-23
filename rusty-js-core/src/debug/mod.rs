use swc_common::FileName;
use swc_common::SourceFile;

use crate::error::Error;
use crate::runtime::Runtime;
use crate::types::JValue;

mod bridge;


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
            "{:#?},{:#?},{:#?},{:#?}",
            args.get(0).map(|v| v.get_property_str("o")),
            args.get(0).map(|v| v.get_property_str("p")),
            args.get(1),
            args.get(2)
        );
        println!("hello world!");
        Ok(JValue::UNDEFINED)
    });

    runtime.to_mut().declare_variable("hello", func.into());

    let t = std::time::Instant::now();
    runtime
        .execute(
            "hello_world",
            r#"
    function i(){
        let a = hello;
        if ((typeof a) == "bject"){
            
        } else{
            return ()=>{a()}
        }
    }
    let y = i();
    y();
    for (i=0;i<9;i++){
        hello();
    }
    a = {p:0.09.toPrecision(8), o:"io"};
    hello(a, 0, 9, ...[])
    "#,
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
    println!("{}", t.elapsed().as_secs_f64()*1000.0);
    println!("{}", a.get());
}

#[test]
fn test2() {
    let runtime = Runtime::new();

    runtime.clone().attach();

    

    let o = runtime.create_native_function(|ctx, this, args|{
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
    let a = 9;
    for (i=0;i<100000;i++){
        a += 1;
    };
    log(a);
    "#,
        )
        .unwrap();

    println!("{} ms", t.elapsed().as_secs_f64()*1000.0);
}
