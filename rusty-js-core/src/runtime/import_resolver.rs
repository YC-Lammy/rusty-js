use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use super::Runtime;

pub struct ImportAssertion {
    pub assertions: HashMap<String, String>,
}

pub trait ImportResolver {
    /// return the script in raw string
    fn import(&mut self, name: &str, asserts: ImportAssertion) -> Result<String, String>;
}

impl From<Option<&swc_ecmascript::ast::ObjectLit>> for ImportAssertion {
    fn from(asserts: Option<&swc_ecmascript::ast::ObjectLit>) -> Self {
        if let Some(o) = asserts {
            let mut assertions = HashMap::default();

            for i in &o.props {
                match i {
                    swc_ecmascript::ast::PropOrSpread::Prop(p) => {
                        if let Some(v) = p.as_key_value() {
                            let name = match &v.key {
                                swc_ecmascript::ast::PropName::BigInt(b) => b.value.to_string(),
                                swc_ecmascript::ast::PropName::Computed(_) => continue,
                                swc_ecmascript::ast::PropName::Ident(i) => i.sym.to_string(),
                                swc_ecmascript::ast::PropName::Num(n) => n.value.to_string(),
                                swc_ecmascript::ast::PropName::Str(s) => s.value.to_string(),
                            };

                            let value = match v.value.as_ref() {
                                swc_ecmascript::ast::Expr::Lit(l) => match l {
                                    swc_ecmascript::ast::Lit::BigInt(b) => b.value.to_string(),
                                    swc_ecmascript::ast::Lit::Bool(b) => b.value.to_string(),
                                    swc_ecmascript::ast::Lit::JSXText(j) => j.value.to_string(),
                                    swc_ecmascript::ast::Lit::Null(_) => "null".to_string(),
                                    swc_ecmascript::ast::Lit::Num(n) => n.value.to_string(),
                                    swc_ecmascript::ast::Lit::Regex(r) => {
                                        format!("/{}/{}", r.exp, r.flags)
                                    }
                                    swc_ecmascript::ast::Lit::Str(s) => s.value.to_string(),
                                },
                                _ => continue,
                            };

                            assertions.insert(name, value);
                        }
                    }
                    swc_ecmascript::ast::PropOrSpread::Spread(_s) => continue,
                };
            }

            Self { assertions }
        } else {
            Self {
                assertions: Default::default(),
            }
        }
    }
}
