
use swc_common::SourceFile;
use swc_common::FileName;
pub fn parse_script(script:&str) -> swc_ecma_ast::Script{

    let src = SourceFile::new(
        FileName::Anon, 
        false, 
        FileName::Anon, 
        script.to_string(), 
        swc_common::BytePos(1)
    );
    let mut v = Vec::new();
    let re = swc_ecma_parser::parse_file_as_script(
        &src, 
        swc_ecma_parser::Syntax::Es(swc_ecma_parser::EsConfig { 
            jsx: false, 
            fn_bind: true, 
            decorators: true, 
            decorators_before_export: true, 
            export_default_from: true, 
            import_assertions: true, 
            private_in_object: true, 
            allow_super_outside_method: false, 
            allow_return_outside_function: false
        }), 
        swc_ecma_ast::EsVersion::Es2022, 
        None, 
        &mut v
    );

    re.unwrap()
}