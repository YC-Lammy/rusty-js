use std::str::FromStr;

use proc_macro::TokenStream;
use syn::{ItemImpl, AttributeArgs, ImplItem};

#[proc_macro_attribute]
pub fn prototype(attr:TokenStream, input:TokenStream) -> TokenStream{
    let ast = syn::parse_macro_input!(input as ItemImpl);
    let args = syn::parse_macro_input!(attr as AttributeArgs);
    let re = expand(args, ast);
    match re{
        Ok(v) => v,
        Err(e) => TokenStream::from(e.to_compile_error())
    }
}

fn expand(args:AttributeArgs, input:ItemImpl) -> syn::Result<TokenStream>{
    if !input.generics.params.is_empty(){
        return Err(syn::Error::new_spanned(&input, "JSObject Impl must not have generics"));
    }

    let mut methods = Vec::new();

    for i in &input.items{
        match i{
            ImplItem::Method(m) => {
                if !m.sig.generics.params.is_empty(){
                    return Err(syn::Error::new_spanned(&m, "JSObject Impl must not have generics"));
                }

                let s = m.sig.inputs.first();

                let mut reference_this = false;

                if let Some(s) = s{
                    match s{
                        syn::FnArg::Receiver(r) => {
                            if !r.reference.is_some(){
                                return Err(syn::Error::new_spanned(&r, "self reference must not be owned"));
                            }

                            reference_this = true;
                        },
                        _ => {}
                    }
                }

                let mut args_len = m.sig.inputs.len();
                if reference_this{
                    args_len -= 1;
                }

                let output = match &m.sig.output{
                    syn::ReturnType::Default => false,
                    _ => true
                };

                methods.push((m.sig.ident.to_string(), reference_this, args_len, output));
            },
            _ => {}
        }
    };

    let mut wrapped_methods = Vec::new();

    for (name, this, args_len, output) in &methods{
        
        let args = (0..*args_len).into_iter().map(|v|format!("args[{}].into()", v)).collect::<Vec<_>>().join(",");

        let call_expr = if *this{
            format!("{}(this, {})", name, args)
        } else{
            format!("{}({})", name, args)
        };

        let stream = format!(
            r#"fn {}_f (this:rusty_js::JSValue, args:&[rusty_js::JSValue]) -> Result<rusty_js::JSValue, rusty_js::JValue> {{
                let mut args = args.to_vec();
                if {} > args.len() {{
                    args.resize({}, rusty_js::JSValue::undefined());
                }}
                let v = {};
                v.into()
            }};"#, name, args_len, args_len, call_expr
        );
        wrapped_methods.push(stream);
    };

    let wrapped_methods = wrapped_methods.join("\n");
    let names = methods.iter().map(|v|v.0.as_str()).collect::<Vec<_>>().join(",");
    return Ok(TokenStream::from_str(&format!(r#"
    impl rusty_js::HasPrototype{{
        fn methods() -> &[fn(rusty_js::JSValue, &[rusty_js::JSValue]) -> rusty_js::JSValue]{{
            {}
            return &[{}]
        }}
    }}
    "#, wrapped_methods, names)).unwrap());
}

#[cfg(test)]
mod test{

}