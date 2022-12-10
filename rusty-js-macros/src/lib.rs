use std::str::FromStr;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{ItemImpl, AttributeArgs, ImplItem};

mod bytecode;

/// example:
/// ```
/// struct MyObject{
///     field1:usize
/// }
/// 
/// #[rusty_js::prototype]
/// impl MyObject{
///     #[getter]
///     pub fn get_field1(&self) -> usize{
///         return self.field1
///     }
/// 
///     #[setter]
///     pub fn set_field1(&mut self, value:usize){
///         self.field1 = value;
///     }
/// 
///     // functions can be getter and setter at the same time
///     #[getter]
///     #[setter]
///     pub fn getset_field1(&mut self, value:Option<&JSValue>) -> usize{
///         if let Some(value) = value{
///             self.field1 = value.into();
///         }
///         return self.field1
///     }
/// 
///     // regular functions
///     pub fn do_something(&self) -> !{
///         todo!()
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn prototype(attr:TokenStream, input:TokenStream) -> TokenStream{
    let ast = syn::parse_macro_input!(input as ItemImpl);
    let args = syn::parse_macro_input!(attr as AttributeArgs);
    let re = expand(args, ast);
    match re{
        Ok(v) => v.into(),
        Err(e) => TokenStream::from(e.to_compile_error())
    }
}

fn expand(args:AttributeArgs, mut input:ItemImpl) -> syn::Result<proc_macro2::TokenStream>{
    if !input.generics.params.is_empty(){
        return Err(syn::Error::new_spanned(&input, "JSObject Impl must not have generics"));
    }

    let type_name = input.self_ty.to_token_stream().to_string();

    let mut methods = Vec::new();

    let mut prototype_fn = None;

    'fn_loop:for i in &mut input.items{
        match i{
            ImplItem::Method(m) => {
                
                let mut c = 0;
                let is_getter = m.attrs.iter().find_map(|attr|{
                    if let Some(p) = attr.path.get_ident(){
                        if p == "getter"{
                            return Some(c)
                        }
                    }
                    c += 1;
                    None
                });
                if let Some(i) = is_getter{
                    m.attrs.remove(i);
                }

                c = 0;
                let is_setter = m.attrs.iter().find_map(|attr|{
                    if let Some(p) = attr.path.get_ident(){
                        if p == "setter"{
                            return Some(c)
                        }
                    }
                    c += 1;
                    None
                });

                if let Some(i) = is_setter{
                    m.attrs.remove(i);
                }
                

                for attr in m.attrs.iter(){
                    if let Some(p) = attr.path.get_ident(){
                        if p == "prototype"{
                            prototype_fn = Some(m.sig.ident.to_string());
                            continue 'fn_loop;
                        }
                    }
                }

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

                

                methods.push((m.sig.ident.to_string(), reference_this, args_len, output, is_getter.is_some(), is_setter.is_some()));
            },
            _ => {}
        }
    };

    let mut wrapped_methods = Vec::new();
    let mut method_names = Vec::new();
    let mut wrapped_setters = Vec::new();
    let mut setter_names = Vec::new();
    let mut wrapped_getters = Vec::new();
    let mut getter_names = Vec::new();

    for (name, this, args_len, output, is_getter, is_setter) in &methods{
        
        let args = (0..*args_len).into_iter().map(|v|format!("args.get({}).unwrap_or_default().into()", v)).collect::<Vec<_>>().join(",");

        let call_expr = if *this{
            format!("{}::{}(this, {})",type_name, name, args)
        } else{
            format!("{}::{}({})",type_name, name, args)
        };

        let stream = format!(
            r#"fn {} (ctx:&rusty_js::JSFuncContext, this:&rusty_js::JSValue, args:&[rusty_js::JSValue]) -> Result<rusty_js::JSValue, rusty_js::JValue> {{
                unsafe{{rusty_js::bind_context(ctx)}};
                let this = match this.as_mut_custom_object(){{
                    Some(v) => v,
                    None => return Err(rusty_js::type_error("expected this to be {}"))
                }};
                let v = {};
                unsafe{{rusty_js::unbind_context()}};
                rusty_js::Resultable::convert_result(v)
            }};"#, name, type_name, call_expr
        );
        
        if *is_getter{
            wrapped_getters.push(stream.clone());
            getter_names.push(format!("(\"{}\", {})", name, name));
        }
        if *is_setter{
            wrapped_setters.push(stream.clone());
            setter_names.push(format!("(\"{}\", {})", name, name));
        }
        if !*is_getter && !*is_setter{
            wrapped_methods.push(stream);
            method_names.push(format!("(\"{}\", {})", name, name));
        }
    };

    let wrapped_methods = wrapped_methods.join("\n");
    let wrapped_setters = wrapped_setters.join("\n");
    let wrapped_getters = wrapped_getters.join("\n");


    
    let mut token = proc_macro2::TokenStream::from_str(&format!(r#"
    impl rusty_js::HasPrototype for {} {{
        fn methods() -> &'static [(&'static str, fn(&rusty_js::JSFuncContext, &rusty_js::JSValue, &[rusty_js::JSValue]) -> Result<rusty_js::JSValue, rusty_js::JSValue>)]{{
            {}
            return &[{}]
        }}
        fn getters() -> &'static [(&'static str, fn(&rusty_js::JSFuncContext, &rusty_js::JSValue, &[rusty_js::JSValue]) -> Result<rusty_js::JSValue, rusty_js::JSValue>)] {{
            {}
            return &[{}]
        }}
        fn setters() -> &'static [(&'static str, fn(&rusty_js::JSFuncContext, &rusty_js::JSValue, &[rusty_js::JSValue]) -> Result<rusty_js::JSValue, rusty_js::JSValue>)] {{
            {}
            return &[{}]
        }}
        fn prototype() -> Option<std::any::TypeId>{{
            {}
        }}
    }}
    "#, 
    type_name, 
    wrapped_methods, method_names.join(","), 
    wrapped_getters, getter_names.join(","),
    wrapped_setters, setter_names.join(","),
    if let Some(f) = prototype_fn{
        format!("Some(Self::{}())", f)
    } else{
        "None".to_string()
    }
    )).unwrap();


    input.to_tokens(&mut token);
    return Ok(token)
}

#[cfg(test)]
mod test{

}

#[proc_macro_derive(ByteCode, attributes(r, w))]
pub fn register_bytecodes(input:TokenStream) -> TokenStream{
    bytecode::register_bytecodes(input)
}