
use std::str::FromStr;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{ItemImpl, AttributeArgs, ItemEnum};

struct Varient{
    name:String,
    fields:Vec<String>,
    reads:Vec<String>,
    write:Option<String>
}


pub fn register_bytecodes(input:TokenStream) -> TokenStream{
    let mut ast = syn::parse_macro_input!(input as ItemEnum);

    let mut varients: Vec<Varient> = Vec::new();

    'outer:for v in &mut ast.variants{
        let mut fields = Vec::new();
        let mut reads = Vec::new();
        let mut write = None;

        for f in &mut v.fields{
            if f.ident.is_none(){
                continue 'outer
            };
            let mut idx = 0;
            let mut need_remove = Vec::new();

            for i in &mut f.attrs{
                if let Some(id) = i.path.get_ident(){

                    if id == "r"{
                        reads.push(f.ident.as_ref().unwrap().to_string());
                        need_remove.push(idx);
                    }
                    if id == "w"{
                        write = Some(f.ident.as_ref().unwrap().to_string());
                        need_remove.push(idx);
                    }
                };
                idx += 1;
            };
            
            for i in need_remove{
                f.attrs.remove(i);
            }
            

            fields.push(f.ident.as_ref().unwrap().to_string());
        };

        if fields.len() != 0{
            varients.push(Varient { 
                name: v.ident.to_string(), 
                fields, 
                reads, 
                write
            });
        }
        
    };

    let mut write_to = String::new();
    let mut reads_from = String::new();

    for v in varients{
        if let Some(s) = &v.write{
            if v.fields.len() > 1{
                write_to += &format!("        Self::{} {{ {} , .. }} => Some({}),\n", v.name, s, s);
            } else{
                write_to += &format!("        Self::{} {{ {} }} => Some({}),\n", v.name, s, s);
            }
        }
        if v.reads.len() != 0{
            let reads = v.reads.join(",");
            if (v.fields.len() - v.reads.len()) != 0{
                reads_from += &format!("        Self::{} {{ {} ,..}} => vec![{}],", v.name, reads, reads);
            } else{
                reads_from += &format!("        Self::{} {{ {} }} => vec![{}],", v.name, reads, reads);
            }
            
        }
    };

    let stream = format!(r#"
impl ByteCode for {} {{
    fn writes_to(self) -> Option<Register>{{
        match self {{
{}
            _ => None
        }}
    }}

    fn reads_from(self) -> Vec<Register>{{
        match self{{
{}
            _ => vec![]
        }}
    }}
}}
"#, 
    ast.ident, write_to, reads_from
    );

    
    match proc_macro2::TokenStream::from_str(&stream){
        Ok(v) => v.into(),
        Err(e) => {
            println!("{}", e);
            syn::Error::from(e).to_compile_error().into()
        }
    }
}