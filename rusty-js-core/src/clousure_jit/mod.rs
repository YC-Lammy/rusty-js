use std::sync::Arc;
use std::collections::HashMap;

use num_traits::ToPrimitive;
use swc_ecmascript::ast::*;
use swc_atoms::JsWord;

use crate::{JValue, Runtime, utils::string_interner::{SYMBOLS, NAMES}, JSContext, error::Error, JObject};

#[derive(Debug, Clone, Copy)]
pub enum Var{
    Var(u16),
    Let(u16),
    Const(u16),
    Capture(u16)
}

struct Loop {
    label: Option<String>,
    id: u32,
}

#[derive(Default)]
pub struct Context{
    variables: HashMap<JsWord, Var>,
}

pub struct Builder{
    runtime: Arc<Runtime>,

    is_global: bool,
    ctx: Vec<Context>,

    is_async: bool,
    is_generator: bool,

    loops: Vec<Loop>,
    loop_id: u32,

    block_count: u32,

    op_stack_offset: usize
}

#[derive(Clone, Copy, PartialEq)]
pub enum Res{
    Ok,
    Err(JValue),
    Value(JValue),
    Break(u32),
    Continue(u32),
    Return(JValue)
}

type Clousure = Box<dyn FnMut(&Runtime, &mut JValue, &[JValue], &mut [JValue], &mut [JValue;3]) -> Res + 'static>;

impl Builder{
    #[inline]
    pub fn translate_statement(&mut self, label: Option<String>, stmt: &Stmt) -> Clousure{
        match stmt{
            Stmt::Block(b) => {
                self.ctx.push(Default::default());

                let mut stmts = Vec::new();

                for stmt in &b.stmts{
                    let c = self.translate_statement(None, stmt);
                    stmts.push(c);
                }
                self.ctx.pop();

                Box::new(move |rt, this, args, stack, regs|{
                    for i in &mut stmts{
                        let re = (i)(rt, this, args, stack, regs);
                        match re{
                            Res::Ok => {},
                            Res::Value(_) => {},
                            re => return re
                        };
                    };
                    return Res::Ok
                })
            },

            Stmt::Break(b) => {
                if let Some(label) = &b.label{
                    let mut id = 0;
                    for l in &self.loops{
                        if let Some(n) = &l.label{
                            if n == label.sym.as_ref(){
                                id = l.id;
                                break;
                            }
                        }
                    };

                    Box::new(move |_, _, _, _, _|{
                        Res::Break(id)
                    })

                } else{
                    Box::new(|_, _, _, _, _|{
                        Res::Break(0)
                    })
                }
            },

            Stmt::Continue(c) => {
                if let Some(label) = &c.label{
                    let mut id = 0;
                    for l in &self.loops{
                        if let Some(n) = &l.label{
                            if n == label.sym.as_ref(){
                                id = l.id;
                                break;
                            }
                        }
                    };

                    Box::new(move |_, _, _, _, _|{
                        Res::Continue(id)
                    })

                } else{
                    Box::new(|_, _, _, _, _|{
                        Res::Continue(0)
                    })
                }
            },

            Stmt::Debugger(d) => {
                Box::new(|_, _, _, _, _|{
                    Res::Ok
                })
            },

            Stmt::Decl(decl) => {
                match decl{
                    Decl::Class(c) => {
                        todo!()
                    },
                    Decl::Fn(f) => {
                        todo!()
                    },
                    Decl::Var(v) => {
                        let mut decls:Vec<Clousure> = Vec::new();

                        for i in &v.decls{
                            let mut d = self.translate_declare(&i.name, Some(v.kind));

                            let c:Clousure = if let Some(expr) = &i.init{

                                let mut e = self.translate_expr(expr);

                                Box::new(move |rt, this, args, stack, regs|{
                                    let v = (e)(rt, this, args, stack, regs);
                                    match v{
                                        Res::Value(v) => {
                                            regs[0] = v;
                                        },
                                        Res::Ok => {
                                            regs[0] = JValue::UNDEFINED;
                                        }
                                        re => return re
                                    };

                                    (d)(rt, this, args, stack, regs)
                                })

                            } else{
                                Box::new(move |rt, this, args, stack, regs|{
                                    regs[0] = JValue::UNDEFINED;
                                    (d)(rt, this, args, stack, regs)
                                })
                            };

                            decls.push(c);
                        };

                        Box::new(move |rt, this, args, stack, regs|{
                            for i in &mut decls{
                                let re = (i)(rt, this, args, stack, regs);
                                match re{
                                    Res::Value(_) => {},
                                    Res::Ok => {},
                                    re => return re 
                                }
                            };
                            Res::Ok
                        })
                    },
                    _ => unimplemented!("ts declare")
                }
            },

            Stmt::While(w) => {
                self.loop_id += 1;
                self.loops.push(Loop { label: label, id: self.loop_id });

                let mut body = self.translate_statement(None, &w.body);
                let mut test = self.translate_expr(&w.test);

                self.loops.pop();

                Box::new(move |rt, this, args, stack, regs|{
                    loop{
                        let re = (test)(rt, this, args, stack, regs);
                        let test = match re{
                            Res::Value(v) => v,
                            re => return re
                        };

                        if !test.to_bool(){
                            break;
                        }

                        let re = (body)(rt, this, args, stack, regs);
                        match re{
                            Res::Ok => {},
                            Res::Value(_) => {},
                            Res::Break(b) => {
                                if b == 0{
                                    break;
                                } else{
                                    return Res::Break(b)
                                }
                            },
                            Res::Continue(id) => {
                                if id == 0{
                                    continue;
                                } else{
                                    return Res::Continue(id)
                                }
                            },
                            re => return re
                        }
                    }
                    return Res::Ok
                })
            },

            Stmt::DoWhile(d) => {

                self.loop_id += 1;
                self.loops.push(Loop { label, id: self.loop_id });

                let mut body = self.translate_statement(None, &d.body);
                let mut test = self.translate_expr(&d.test);

                self.loops.pop();

                Box::new(move |rt, this, args, stack, regs|{
                    loop{
                        // execute the body
                        let re = (body)(rt, this, args, stack, regs);

                        match re{
                            Res::Err(e) => return Res::Err(e),
                            Res::Ok => {},
                            Res::Value(_) => {},
                            Res::Break(id) => {
                                if id == 0{
                                    break;
                                } else{
                                    return Res::Break(id)
                                }
                            },
                            Res::Continue(id) => {
                                if id == 0{
                                    continue;
                                } else{
                                    return Res::Continue(id);
                                }
                            },
                            Res::Return(r) => return Res::Return(r)
                        };

                        // get the test value
                        let v = (test)(rt, this, args, stack, regs);

                        // test the value
                        match v{
                            Res::Value(v) => {
                                if !v.to_bool(){
                                    break;
                                }
                            },
                            Res::Ok => {},
                            re => return re
                        };
                    };
                    return Res::Ok
                })
            },

            Stmt::Empty(_e) => {
                Box::new(|_rt, _this, _args, _stack, _regs|{
                    Res::Ok
                })
            },

            Stmt::Expr(e) => {
                self.translate_expr(&e.expr)
            },

            Stmt::For(f) => {

                self.loop_id += 1;
                self.loops.push(Loop { label, id: self.loop_id });

                self.ctx.push(Default::default());

                let mut init:Option<Clousure> = 
                
                match &f.init{
                    Some(init) => {
                        Some(
                        match init{
                            VarDeclOrExpr::Expr(e) => {
                                self.translate_expr(&e)
                            },
                            VarDeclOrExpr::VarDecl(v) => {
                                let mut decls:Vec<Clousure> = Vec::new();

                                for i in &v.decls{
                                    let mut d = self.translate_declare(&i.name, Some(v.kind));

                                    let c:Clousure = if let Some(expr) = &i.init{

                                        let mut e = self.translate_expr(expr);

                                        Box::new(move |rt, this, args, stack, regs|{
                                            let v = (e)(rt, this, args, stack, regs);
                                            match v{
                                                Res::Value(v) => {
                                                    regs[0] = v;
                                                },
                                                Res::Ok => {
                                                    regs[0] = JValue::UNDEFINED;
                                                }
                                                re => return re
                                            };

                                            (d)(rt, this, args, stack, regs)
                                        })

                                    } else{
                                        Box::new(move |rt, this, args, stack, regs|{
                                            regs[0] = JValue::UNDEFINED;
                                            (d)(rt, this, args, stack, regs)
                                        })
                                    };

                                    decls.push(c);
                                };

                                Box::new(move |rt, this, args, stack, regs|{
                                    for i in &mut decls{
                                        let re = (i)(rt, this, args, stack, regs);
                                        match re{
                                            Res::Value(_) => {},
                                            Res::Ok => {},
                                            re => return re 
                                        }
                                    };
                                    Res::Ok
                                })
                            }
                        })
                    },
                    None => None
                };
                let mut body = self.translate_statement(None, &f.body);
                let mut test = f.test.as_ref().map(|t|self.translate_expr(&t));
                let mut update = f.update.as_ref().map(|u|self.translate_expr(&u));

                self.ctx.pop();
                self.loops.pop();

                Box::new(move |rt, this, args, stack, regs|{

                    if let Some(init) = &mut init{
                        let re = (init)(rt, this, args, stack, regs);
                        match re{
                            Res::Ok => {},
                            Res::Value(_) => {},
                            re => return re
                        }
                    };

                    loop{

                        if let Some(t) = &mut test{
                            let re = (t)(rt, this, args, stack, regs);

                            match re{
                                Res::Ok => {},
                                Res::Value(v) => {
                                    if !v.to_bool(){
                                        break;
                                    }
                                },
                                re => return re
                            }
                        }

                        let re = (body)(rt, this, args, stack, regs);

                        match re{
                            Res::Err(e) => return Res::Err(e),
                            Res::Ok => {},
                            Res::Value(_) => {},
                            Res::Break(id) => {
                                if id == 0{
                                    break;
                                } else{
                                    return Res::Break(id)
                                }
                            },
                            Res::Continue(id) => {
                                if id == 0{
                                    continue;
                                } else{
                                    return Res::Continue(id);
                                }
                            },
                            Res::Return(r) => return Res::Return(r)
                        };

                        if let Some(u) = &mut update{
                            let re = (u)(rt, this, args, stack, regs);

                            match re{
                                Res::Ok => {},
                                Res::Value(_) => {},
                                re => return re
                            }
                        };
                    }

                    return Res::Ok
                })
            },

            Stmt::ForIn(f) => {
                self.loop_id += 1;
                self.loops.push(Loop { label, id: self.loop_id });

                self.ctx.push(Default::default());

                let mut left = match &f.left{
                    VarDeclOrPat::Pat(p) => {
                        self.translate_declare(&p, None)
                    },
                    VarDeclOrPat::VarDecl(v) => {
                        // ignore initializer and other declarators
                        if let Some(d) = v.decls.first(){
                            self.translate_declare(&d.name, Some(v.kind))
                        } else{
                            unimplemented!("for in loop with multiple bindings or initializer")
                        }
                    }
                };

                let mut right = self.translate_expr(&f.right);
                let mut body = self.translate_statement(None, &f.body);

                self.ctx.pop();
                self.loops.pop();

                Box::new(move |rt, this, args, stack, regs|{
                    let re = (right)(rt, this, args, stack, regs);

                    let value = match re{
                        Res::Value(v) => v,
                        re => return re
                    };

                    if value.is_null() || value.is_undefined(){
                        return Res::Ok
                    }

                    let mut obj = if let Some(obj) = value.as_object(){
                        obj
                    } else{
                        value.prototype(rt).unwrap()
                    };

                    
                    loop{
                        
                        let keys = obj.inner.values.iter()
                        .filter_map(|(k, p)|{ 
                            // enumerable
                            if p.flag.is_enumerable(){
                                Some(k.0)
                            } else{
                                None
                            }
                        });

                        for key in keys{
                            let key = rt.get_field_name(key);

                            let s = rt.allocate_string(key);
                            regs[0] = JValue::create_string(s);
                            
                            let re = (left)(rt, this, args, stack, regs);
                            
                            match re{
                                Res::Value(_) => {},
                                Res::Ok => {},
                                re => return re
                            };

                            let re = (body)(rt, this, args, stack, regs);

                            match re{
                                Res::Value(_) => {},
                                Res::Ok => {},
                                re => return re
                            };
                        };

                        if let Some(p) = obj.inner.__proto__{
                            obj = p;

                        } else{
                            break;
                        }
                    }

                    return Res::Ok
                })
            },

            Stmt::ForOf(f) => {
                if f.await_token.is_some(){
                    todo!()
                }

                self.loop_id += 1;
                self.loops.push(Loop { label, id: self.loop_id });

                self.ctx.push(Default::default());

                let mut left = match &f.left{
                    VarDeclOrPat::Pat(p) => {
                        self.translate_declare(&p, None)
                    },
                    VarDeclOrPat::VarDecl(v) => {
                        // ignore initializer and other declarators
                        if let Some(d) = v.decls.first(){
                            self.translate_declare(&d.name, Some(v.kind))
                        } else{
                            unimplemented!("for in loop with multiple bindings or initializer")
                        }
                    }
                };

                let mut right = self.translate_expr(&f.right);
                let mut body = self.translate_statement(None, &f.body);

                self.ctx.pop();
                self.loops.pop();

                let op_stack_offset = self.op_stack_offset;

                Box::new(move |rt, this, args, stack, regs|{
                    let re = (right)(rt, this, args, stack, regs);
                    let right = match re{
                        Res::Value(v) => v,
                        re => return re
                    };

                    let iter = right.get_property(SYMBOLS["iterator"], JSContext{stack:stack[op_stack_offset..].as_mut_ptr(), runtime:rt});
                    let iter = match iter{
                        Ok(v) => v,
                        Err(e) => return Res::Err(e)
                    };

                    if !iter.is_callable(){
                        return Res::Err(Error::TypeError("right hand side of for...of is not iterator: property '@iterator' is not callable".to_string()).into())
                    }

                    // call Object[@iterator]()
                    let iter = iter.call(right, &[], JSContext{stack:stack[op_stack_offset..].as_mut_ptr(), runtime:rt});

                    let iter = match iter{
                        Ok(v) => v,
                        Err(e) => return Res::Err(e)
                    };

                    loop{
                        // get the 'next' function
                        let next = iter.get_property(NAMES["next"], JSContext{stack:stack[op_stack_offset..].as_mut_ptr(), runtime:rt});
                        let next = match next{
                            Ok(v) => v,
                            Err(e) => return Res::Err(e)
                        };

                        if !next.is_callable(){
                            return Res::Err(Error::TypeError("right hand side of for...of is not iterator: property @iterator().next is not callable".to_string()).into())
                        }

                        // call iterator.next()
                        let next = next.call(iter, &[], JSContext{stack:stack[op_stack_offset..].as_mut_ptr(), runtime:rt});
                        let next = match next{
                            Ok(v) => v,
                            Err(e) => return Res::Err(e)
                        };

                        let next_done = next.get_property(NAMES["done"], JSContext{stack:stack[op_stack_offset..].as_mut_ptr(), runtime:rt});
                        let next_done = match next_done{
                            Ok(v) => v,
                            Err(e) => return Res::Err(e)
                        };

                        let next_value = next.get_property(NAMES["value"], JSContext{stack:stack[op_stack_offset..].as_mut_ptr(), runtime:rt});
                        let next_value = match next_value{
                            Ok(v) => v,
                            Err(e) => return Res::Err(e)
                        };

                        // return if done
                        if next_done.to_bool(){
                            break;
                        }

                        // assign the value
                        regs[0] = next_value;

                        // call the declaration
                        let re = (left)(rt, this, args, stack, regs);

                        match re{
                            Res::Ok => {},
                            Res::Value(_) => {},
                            re => return re
                        };

                        // call the body
                        let re = (body)(rt, this, args, stack, regs);

                        match re{
                            Res::Ok => {},
                            Res::Value(_) => {},
                            re => return re
                        };
                    }

                    return Res::Ok
                })
            }
            Stmt::If(i) => {
                
                let mut cons = self.translate_statement(None, &i.cons);
                let mut alt = i.alt.as_ref().map(|a|self.translate_statement(None, &a));
                let mut test = self.translate_expr(&i.test);

                Box::new(move |rt, this, args, stack, regs|{
                    let re = (test)(rt, this, args, stack, regs);

                    let test = match re{
                        Res::Value(v) => v,
                        re => return re
                    };

                    let re = if test.to_bool(){
                        (cons)(rt, this, args, stack, regs)

                    } else{
                        if let Some(alt) = &mut alt{
                            (alt)(rt, this, args, stack, regs)
                        } else{
                            return Res::Ok
                        }
                    };

                    return re;
                })
            },

            Stmt::Labeled(l) => {
                self.translate_statement(Some(l.label.sym.to_string()), &l.body)
            },

            Stmt::Return(r) => {
                let mut arg = r.arg.as_ref().map(|e|self.translate_expr(&e));

                Box::new(move |rt, this, args, stack, regs|{
                    if let Some(arg) = &mut arg{
                        let re = (arg)(rt, this, args, stack, regs);

                        match re{
                            Res::Value(r) => Res::Return(r),
                            Res::Ok => Res::Return(JValue::UNDEFINED),
                            re => re
                        }
                    } else{
                        return Res::Return(JValue::UNDEFINED)
                    }
                })
            },

            Stmt::Throw(t) => {
                let mut arg = self.translate_expr(&t.arg);

                Box::new(move |rt, this, args, stack, regs|{
                    let re = (arg)(rt, this, args, stack, regs);

                    match re{
                        Res::Value(r) => Res::Err(r),
                        Res::Ok => Res::Err(JValue::UNDEFINED),
                        re => re
                    }
                })
            }

            Stmt::Switch(s) => {
                let mut d = self.translate_expr(&s.discriminant);

                let mut tests = Vec::new();
                let mut cases = Vec::new();

                for i in &s.cases{
                    let test = i.test.as_ref().map(|t|self.translate_expr(&t));
                    let cons:Vec<_> = i.cons.iter().map(|stmt|self.translate_statement(None, stmt)).collect();

                    tests.push(test);
                    cases.push(cons);
                };

                Box::new(move |rt, this, args, stack, regs|{

                    let re = (d)(rt, this, args, stack, regs);

                    let v = match re{
                        Res::Value(v) => v,
                        re => return re
                    };

                    let mut compute_tests = Vec::new();

                    for t in &mut tests{
                        if let Some(t) = t{
                            let re = (t)(rt, this, args, stack, regs);
                            match re{
                                Res::Value(v) => compute_tests.push(Some(v)),
                                re => return re
                            };
                        } else{
                            compute_tests.push(None);
                        }
                    };

                    let mut i = 0;
                    let len = compute_tests.len();
                    while i < len{
                        let test = compute_tests[i];

                        if test == Some(v) || test.is_none(){
                            let cons = &mut cases[i];

                            for i in cons{
                                let re = (i)(rt, this, args, stack, regs);
                                match re{
                                    Res::Ok => {},
                                    Res::Value(_) => {},
                                    re => return re
                                }
                            }
                            break;
                        }
                        i += 1;
                    }
                    return Res::Ok
                })
            },

            Stmt::Try(t) => {
                self.ctx.push(Default::default());

                let mut body = Vec::new();

                for stmt in &t.block.stmts{
                    let s = self.translate_statement(None, stmt);
                    body.push(s);
                }
                self.ctx.pop();

                
                let mut finalize = t.finalizer.as_ref().map(|f|{
                    
                    self.ctx.push(Default::default());

                    let mut finalize = Vec::new();

                    for stmt in &f.stmts{
                        let s = self.translate_statement(None, stmt);
                        finalize.push(s);
                    }

                    self.ctx.pop();

                    finalize
                });
                

                Box::new(move |rt, this, args, stack, regs|{

                    let mut error = None;
                    // in try
                    for s in &mut body{
                        let re = (s)(rt, this, args, stack, regs);

                        match re{
                            Res::Ok => {},
                            Res::Value(_) => {},
                            Res::Err(e) => {
                                error = Some(e);
                                break;
                            },
                            re => return re
                        };
                    };

                    // catch block
                    if let Some(err) = error{
                        regs[0] = err;
                    };

                    // finalize
                    if let Some(finalize) = &mut finalize{
                        for i in finalize{
                            let re = (i)(rt, this, args, stack, regs);

                            match re{
                                Res::Ok => {},
                                Res::Value(_) => {},
                                re => return re
                            }
                        }
                    };

                    return Res::Ok
                })
            },

            Stmt::With(w) => {
                unimplemented!()
            }
        }
    }

    #[inline]
    fn translate_expr(&mut self, expr:&Expr) -> Clousure{
        match expr{
            Expr::Array(a) => {
                let mut elems = Vec::new();

                for i in &a.elems{
                    if let Some(e) = i{
                        let expr = self.translate_expr(&e.expr);
                        if e.spread.is_some(){

                        }

                        elems.push(Some(expr));

                    } else{
                        elems.push(None);
                    }
                };

                Box::new(move |rt, this, args, stack, regs|{
                    let mut ar = Vec::new();

                    for i in &mut elems{
                        let v = if let Some(e) = i{
                            let re = (e)(rt, this, args, stack, regs);
                            match re{
                                Res::Value(v) => v,
                                re => return re
                            }
                        } else{
                            JValue::UNDEFINED
                        };
                        ar.push((Default::default(), v));
                    };

                    let array = JObject::with_array(ar);

                    return Res::Value(JValue::create_object(array));
                })
            },

            Expr::Arrow(a) => {
                todo!()
            },

            Expr::Assign(a) => {
                let mut right = self.translate_expr(&a.right);
                let op = a.op;

                match &a.left{
                    PatOrExpr::Pat(p) => {
                        let mut declare = self.translate_declare(&p, None);

                        return Box::new(move |rt, this, args, stack, regs|{
                            let re = (right)(rt, this, args, stack, regs);
                            let v = match re{
                                Res::Value(v) => v,
                                re => return re
                            };


                            regs[0] = v;

                            let re = (declare)(rt, this, args, stack, regs);
                            match re{
                                Res::Ok => {},
                                Res::Value(_) => {},
                                re => return re
                            };

                            return Res::Value(v)
                        })
                    },
                    PatOrExpr::Expr(e) => {
                        match e.as_ref(){
                            Expr::Member(m) => {
                                let mut obj = self.translate_expr(&e);
                                let mut prop = self.translate_prop(&m.prop);

                                let op_stack_offset = self.op_stack_offset;

                                return Box::new(move |rt, this, args, stack, regs|{
                                    let re = (right)(rt, this, args, stack, regs);
                                    let mut v = match re{
                                        Res::Value(v) => v,
                                        re => return re
                                    };

                                    let re = (obj)(rt, this, args, stack, regs);
                                    let obj = match re{
                                        Res::Value(v) => v,
                                        re => return re
                                    };

                                    let re = (prop)(rt, this, args, stack, regs);
                                    let prop = match re{
                                        Res::Value(v) => v,
                                        re => return re
                                    };

                                    let ctx = JSContext { stack: stack[op_stack_offset..].as_mut_ptr(), runtime: rt};

                                    if op != AssignOp::Assign{
                                        let old = obj.get_property(prop, ctx);
                                        let old = match old{
                                            Ok(v) => v,
                                            Err(e) => return Res::Err(e)
                                        };
                                        let o = ||{
                                            let v = match op{
                                                AssignOp::Assign => JValue::UNDEFINED,
                                                AssignOp::AddAssign => {
                                                    old.add(v, ctx)?
                                                },
                                                AssignOp::AndAssign => {
                                                    (old.to_bool() && v.to_bool()).into()
                                                },
                                                AssignOp::BitAndAssign => {
                                                    (old.to_i32(ctx)? & v.to_i32(ctx)?).into()
                                                },
                                                AssignOp::BitOrAssign => {
                                                    (old.to_i32(ctx)? | v.to_i32(ctx)?).into()
                                                },
                                                AssignOp::BitXorAssign => {
                                                    (old.to_i32(ctx)? ^ v.to_i32(ctx)?).into()
                                                },
                                                AssignOp::DivAssign => {
                                                    old.div(v, ctx)?
                                                },
                                                AssignOp::ExpAssign => {
                                                    old.exp(v, ctx)?
                                                },
                                                AssignOp::LShiftAssign => {
                                                    (old.to_i32(ctx)? << v.to_i32(ctx)?).into()
                                                },
                                                AssignOp::ModAssign => {
                                                    old.rem(v, ctx)?
                                                },
                                                AssignOp::MulAssign => {
                                                    old.mul(v, ctx)?
                                                },
                                                AssignOp::NullishAssign => {
                                                    if old.is_null() || old.is_undefined(){
                                                        v
                                                    } else{
                                                        old
                                                    }
                                                },
                                                AssignOp::OrAssign => {
                                                    if !old.to_bool(){
                                                        v
                                                    } else{
                                                        old
                                                    }
                                                },
                                                AssignOp::RShiftAssign => {
                                                    (old.to_i32(ctx)? >> v.to_i32(ctx)?).into()
                                                },
                                                AssignOp::SubAssign => {
                                                    old.sub(v, ctx)?
                                                },
                                                AssignOp::ZeroFillRShiftAssign => {
                                                    (old.to_i32(ctx)?  as u32 >> v.to_i32(ctx)? as u32).into()
                                                }
                                            };
                                            Ok(v)
                                        };
                                        let re = o();
                                        match re{
                                            Ok(val) => {
                                                v = val;
                                            },
                                            Err(e) => return Res::Err(e)
                                        };
                                    }
                                    
                                    obj.set_property(prop, v, ctx);

                                    return Res::Value(v)
                                })
                            },
                            e => todo!("expr assign: {:?}", e)
                        }
                    }
                }
            },

            Expr::Await(a) => {
                todo!()
            },
            _ => todo!()
        }
    }

    /// the value is stored in regs[0]
    #[inline]
    fn translate_declare(&mut self, name:&Pat, kind:Option<VarDeclKind>) -> Clousure{
        match name{
            Pat::Ident(id) => {

                Box::new(move |rt, this, args, stack, regs|{
                    Res::Value(regs[0])
                })
            },
            _ => todo!()
        }
    }

    fn translate_prop(&mut self, prop:&MemberProp) -> Clousure{
        match prop{
            MemberProp::Ident(id) => {
                let sym = self.runtime.register_field_name(&id.sym);

                Box::new(move |rt, this, args, stack, regs|{
                    return Res::Value(JValue::create_symbol(sym))
                })
            },
            MemberProp::PrivateName(n) => {
                let s = "#".to_owned() + n.id.sym.to_string().as_str();
                let sym = self.runtime.register_field_name(&s);

                Box::new(move |rt, this, args, stack, regs|{
                    return Res::Value(JValue::create_symbol(sym))
                })
            },
            MemberProp::Computed(c) => {
                self.translate_expr(&c.expr)
            }
        }
    }
}