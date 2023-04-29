use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

use syn::{Item, __private::ToTokens};

fn main() {
    let src = fs::read_dir("src").unwrap();

    for i in src {
        if let Ok(e) = i {
            if e.path().is_file() {
                handle_file(e.path())
            }
        }
    }
}

fn handle_file(path: PathBuf) {
    let mut l = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("hello")
        .unwrap();
    let mut f = fs::OpenOptions::new()
        .read(true)
        .open(path.as_path())
        .unwrap();

    let mut src = String::new();
    f.read_to_string(&mut src);
    let mut parsed = syn::parse_file(&src);

    let mut parsed = match parsed {
        Ok(p) => p,
        Err(e) => {
            let s = e.span();
            panic!("{}, {}", path.as_path().display(), e.to_string())
        }
    };
    let mut need_rewite = false;
    for i in &mut parsed.items {
        match i {
            Item::Fn(f) => {
                f.block.stmts.clear();
                if true {
                    need_rewite = true;
                    let mut args = syn::punctuated::Punctuated::new();

                    for i in &f.sig.inputs {
                        match i {
                            syn::FnArg::Typed(t) => {
                                args.push(syn::Expr::Verbatim(t.pat.to_token_stream()));
                            }
                            syn::FnArg::Receiver(r) => {}
                        }
                    }

                    let mut p = syn::punctuated::Punctuated::new();

                    p.push(syn::PathSegment {
                        ident: syn::Ident::new("llvm_sys", f.sig.ident.span()),
                        arguments: Default::default(),
                    });
                    p.push(syn::PathSegment {
                        ident: syn::Ident::new(
                            path.file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .split_once(".")
                                .unwrap()
                                .0,
                            f.sig.ident.span(),
                        ),
                        arguments: Default::default(),
                    });
                    p.push(syn::PathSegment {
                        ident: f.sig.ident.clone(),
                        arguments: Default::default(),
                    });
                    let func = syn::Expr::Path(syn::ExprPath {
                        attrs: Vec::new(),
                        qself: None,
                        path: syn::Path {
                            leading_colon: None,
                            segments: p,
                        },
                    });
                    f.block
                        .stmts
                        .push(syn::Stmt::Expr(syn::Expr::Call(syn::ExprCall {
                            attrs: Vec::new(),
                            func: Box::new(func),
                            paren_token: Default::default(),
                            args: args,
                        })));
                }
            }
            _ => {}
        }
    }

    if need_rewite {
        let s = parsed.to_token_stream().to_string();

        let mut f = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)
            .unwrap();
        f.write_all(s.as_bytes());
    }
}
