use swc_common::FileName;
use swc_common::SourceFile;

use crate::runtime::Runtime;
use crate::types::JValue;
use crate::error::Error;

mod bridge;

macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::debug::logging(&format!($($arg)*))
    };
}

pub(crate) use debug;

#[cfg(test)]
#[inline]
pub fn logging(s: &str) {
    println!("{}", s);
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
            allow_return_outside_function:false,
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
    let func = runtime.create_native_function(move|ctx, this, args| {

        a1.set(a1.get() + 1);
        println!("{:#?}", args.get(0).map(|v|v.get_property_str("o")));
        println!("hello world!");
        Ok(JValue::UNDEFINED)
    });

    let id = runtime.regester_dynamic_var_name("hello");
    runtime.to_mut().set_variable(id, func.into());

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
    a = {p:0, o:9};
    hello(a)
    "#,
        ).map_err(|e|{
            match e{
                Error::Value(v) => {
                    println!("{}", e.to_string());
                },
                e => {
                    println!("{}", e.to_string());
                }
            };
        });

    println!("{}", a.get());
}
