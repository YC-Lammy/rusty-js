use std::collections::VecDeque;
use std::io::Write;
use std::sync::Arc;

use num_traits::ToPrimitive;
use swc_ecma_ast::*;

use crate::bultins::function::JSFunction;
use crate::bytecodes::*;
use crate::error::Error;
use crate::runtime::{FuncID, Runtime};
use crate::types::JValue;
use super::function_builder_context::{FunctionBuilderContext, DeclareKind};

type Res = Result<(), Error>;

struct Loop{
    label:Option<String>,
    exit:Block,
    continue_:Block
}

pub struct FunctionBuilder{
    runtime:Arc<Runtime>,

    ctx:FunctionBuilderContext,
    loops:VecDeque<Loop>,

    block_count:u32,

    r1:Register,
    r2:Register,
    r3:Register,

    catch_block:Option<Block>,

    pub bytecode:Vec<OpCode>
}

#[allow(unused)]
impl FunctionBuilder{
    pub fn new(runtime:Arc<Runtime>) -> Self{
        return Self { 
            runtime,

            ctx:  FunctionBuilderContext::new(),

            loops:VecDeque::new(),

            block_count:0,
            bytecode:Vec::new(),

            catch_block:None,

            r1:Register(1),
            r2:Register(2),
            r3:Register(3)
        }
    }

    pub fn new_with_context(runtime:Arc<Runtime>,mut ctx:FunctionBuilderContext) -> Self{
        ctx.new_function();
        return Self { 
            ctx: ctx, 
            ..Self::new(runtime) 
        }
    }

    fn build_function(&mut self, func:&Function) -> Result<(), Error>{
        let mut i = 0;
        for p in &func.params{

            self.translate_param(p, i)?;
            i += 1;
        };
        if let Some(body) = &func.body{
            for stmt in &body.stmts{
                println!("0");
                self.translate_statement(None, stmt)?;
            }
        }
        Ok(())
    }

    pub fn translate_statement(&mut self, label:Option<&str>, stmt:&Stmt) -> Res{
        #[cfg(test)]
        print!("translate statment ");

        std::io::stdout().flush();

        match stmt{
            Stmt::Block(b) => {
                self.ctx.new_context();

                let mut v = Vec::new();
                for stmt in &b.stmts{
                    v.push(self.translate_statement(None, stmt)?);
                }
                self.ctx.close_context();

            },
            Stmt::Break(b) => {
                if let Some(break_label) = &b.label{
                    for l in &self.loops{
                        if let Some(label) = &l.label{
                            if label.as_str() == &break_label.sym{

                                self.bytecode.push(OpCode::Jump { to: l.exit });
                                return Ok(());
                            }
                        }
                    }

                    return Err(Error::LabelUndefined(break_label.sym.to_string()));
                } else{
                    if let Some(l) = self.loops.front(){
                        self.bytecode.push(OpCode::Jump { to: l.exit });
                    } else{
                        return Err(Error::IllegalBreak)
                    }
                }
            },
            Stmt::Continue(c) => {
                if let Some(break_label) = &c.label{
                    for l in &self.loops{
                        if let Some(label) = &l.label{
                            if label.as_str() == &break_label.sym{

                                self.bytecode.push(OpCode::Jump { to: l.continue_ });
                                return Ok(());
                            }
                        }
                    }

                    return Err(Error::LabelUndefined(break_label.sym.to_string()));
                } else{
                    if let Some(l) = self.loops.front(){
                        self.bytecode.push(OpCode::Jump { to: l.continue_ });
                    } else{
                        return Err(Error::IllegalBreak)
                    }
                }
            },
            Stmt::Debugger(d) => {
                self.bytecode.push(OpCode::Debugger);
            },
            Stmt::Decl(d) => {
                #[cfg(test)]
                print!("decl");

                match d{
                    Decl::Class(class) => {
                        let c = self.translate_class(&class.class)?;
                        let code = self.ctx.declare(class.ident.to_id(), c, DeclareKind::Let);
                        self.bytecode.push(code);
                    },
                    Decl::Fn(f) => {

                        #[cfg(test)]
                        println!("function");

                        let mut builder = FunctionBuilder::new_with_context(self.runtime.clone(), self.ctx.clone());
                        builder.build_function(&f.function)?;
                        let id = builder.finish()?;

                        self.bytecode.extend(self.ctx.need_done());

                        self.bytecode.push(OpCode::CreateFunction { result: self.r1, id: id });
                        
                        let code = self.ctx.declare(f.ident.to_id(), self.r1, DeclareKind::Let);
                        self.bytecode.push(code);
                    },
                    Decl::Var(v) => {

                        #[cfg(test)]
                        println!("var");

                        for d in &v.decls{
                            self.translate_vardeclare(d, v.kind)?;
                        }
                    },
                    Decl::TsEnum(_e) => {
                        todo!("TS enum")
                    },
                    Decl::TsInterface(_i) => {
                        todo!("TS interface")
                    },
                    Decl::TsModule(_m) => {
                        todo!("TS module")
                    },
                    Decl::TsTypeAlias(_t) => {
                        todo!("TS type alias")
                    },
                }
            },
            Stmt::DoWhile(d) => {
                let (header, exit) = self.start_loop(label);
                self.ctx.new_context();

                

                self.translate_statement(None, &d.body)?;
                
                let v = self.translate_expr(&d.test)?;
                self.bytecode.push(OpCode::JumpIfTrue { value: v, to: header });

                self.bytecode.push(OpCode::Jump { to: exit});

                self.ctx.close_context();
                self.end_loop();
            },
            Stmt::Empty(e) => {
                #[cfg(test)]
                println!("empty");
            },
            Stmt::Expr(e) => {
                #[cfg(test)]
                println!("expr");

                self.translate_expr(&e.expr)?;
            },
            Stmt::For(f) => {

                #[cfg(test)]
                println!("for");
                
                if let Some(v) = &f.init{
                    self.ctx.new_context();

                    match v{
                        VarDeclOrExpr::Expr(e) => {
                            self.translate_expr(&e)?;
                        },
                        VarDeclOrExpr::VarDecl(v) => {
                            for i in &v.decls{
                                self.translate_vardeclare(i, v.kind)?;
                            }
                        }
                    }
                }

                let (header, exit) = self.start_loop(label);
                self.ctx.new_context();

                if let Some(e) = &f.test{
                    let test = self.translate_expr(&e)?;
                    self.bytecode.push(OpCode::JumpIfFalse { value: test, to: exit });
                }

                self.translate_statement(None, &f.body)?;
                
                if let Some(e) = &f.update{
                    self.translate_expr(&e)?;
                }

                self.bytecode.push(OpCode::Jump { to: header });

                self.ctx.close_context();
                self.end_loop();

                if f.init.is_some(){
                    self.ctx.close_context();
                }
            },
            Stmt::ForIn(f) => {

                #[cfg(test)]
                println!("for in");

                let v = self.translate_expr(&f.right)?;
                self.bytecode.push(OpCode::IntoIter { 
                    target: v, 
                    hint: LoopHint::ForIn
                });

                let (header, exit) = self.start_loop(label);
                self.ctx.new_context();

                self.bytecode.push(OpCode::IterNext { 
                    result: self.r2, 
                    hint: LoopHint::ForIn,
                    stack_offset:self.ctx.current_stack_offset()
                });

                match &f.left{
                    VarDeclOrPat::Pat(p) => {
                        self.translate_pat_assign(p, self.r2, DeclareKind::None)?;
                    },
                    VarDeclOrPat::VarDecl(v) => {
                        let kind = match v.kind{
                            VarDeclKind::Const => DeclareKind::Const,
                            VarDeclKind::Let => DeclareKind::Let,
                            VarDeclKind::Var => DeclareKind::Var,
                        };
                        for i in &v.decls{
                            self.translate_pat_assign(&i.name, self.r2, kind)?;
                        }
                    }
                }

                self.translate_statement(None, &f.body)?;
                self.bytecode.push(OpCode::JumpIfIterDone { to: exit });

                self.ctx.close_context();
                self.end_loop();

                self.bytecode.push(OpCode::IterDrop);

            },
            Stmt::ForOf(f) => {

                #[cfg(test)]
                println!("for of");

                let v = self.translate_expr(&f.right)?;
                self.bytecode.push(OpCode::IntoIter { 
                    target: v, 
                    hint: LoopHint::ForOf
                });

                let (header, exit) = self.start_loop(label);
                self.ctx.new_context();

                self.bytecode.push(OpCode::IterNext { 
                    result: self.r2, 
                    hint: LoopHint::ForOf,
                    stack_offset:self.ctx.current_stack_offset()
                });

                match &f.left{
                    VarDeclOrPat::Pat(p) => {
                        self.translate_pat_assign(p, self.r2, DeclareKind::None)?;
                    },
                    VarDeclOrPat::VarDecl(v) => {
                        let kind = match v.kind{
                            VarDeclKind::Const => DeclareKind::Const,
                            VarDeclKind::Let => DeclareKind::Let,
                            VarDeclKind::Var => DeclareKind::Var
                        };
                        for i in &v.decls{
                            self.translate_pat_assign(&i.name, self.r2, kind)?;
                        }
                    }
                }

                self.translate_statement(None, &f.body)?;
                self.bytecode.push(OpCode::JumpIfIterDone { to: exit });

                self.ctx.close_context();
                self.end_loop();

                self.bytecode.push(OpCode::IterDrop);

            },
            Stmt::If(i) => {

                #[cfg(test)]
                println!("if");

                let cond = self.create_block();
                let exit = self.create_block();

                let value = self.translate_expr(&i.test)?;

                self.bytecode.push(OpCode::JumpIfTrue { value, to: cond });
                // jump to exit if not true
                self.bytecode.push(OpCode::Jump { to: exit });

                // if block
                self.bytecode.push(OpCode::SwitchToBlock(cond));
                self.ctx.new_context();

                self.translate_statement(None, &i.cons)?;

                self.ctx.close_context();
                self.bytecode.push(OpCode::Jump { to: exit });

                // else block
                if let Some(stmt) = &i.alt{
                    self.ctx.new_context();

                    self.translate_statement(None, &stmt)?;

                    self.ctx.close_context();
                }
            },
            Stmt::Labeled(l) => {
                self.translate_statement(Some(&l.label.sym), &l.body)?;
            },
            Stmt::Return(r) => {

                #[cfg(test)]
                println!("return");

                let r = if let Some(e) = &r.arg{
                    self.translate_expr(&e)?
                } else{
                    self.bytecode.push(OpCode::LoadUndefined { result: self.r1 });
                    self.r1
                };
                self.bytecode.push(OpCode::Return { value: r});
            },
            Stmt::Switch(s) => {

                #[cfg(test)]
                println!("switch");

                let target = self.translate_expr(&s.discriminant)?;

                let exit = self.create_block();

                let mut bs = Vec::new();

                for i in &s.cases{
                    

                    if let Some(cmp ) = &i.test{
                        
                        let block = self.create_block();
                        let cmp = self.translate_expr(cmp)?;
                        self.bytecode.push(OpCode::ReadTemp { value: self.r2 });

                        self.bytecode.push(OpCode::EqEqEq { result: self.r1, left: cmp, right: self.r2 });
                        self.bytecode.push(OpCode::JumpIfTrue { value: self.r1, to: block });

                        bs.push((block, i));
                    } else{
                        // default
                        self.ctx.new_context();

                        for stmt in &i.cons{
                            self.translate_statement(None, stmt)?;
                        }

                        self.ctx.close_context();
                        self.bytecode.push(OpCode::Jump { to: exit });
                    }    
                }

                for (block, i) in bs{
                    if let Some(cmp ) = &i.test{
                        
                        self.bytecode.push(OpCode::SwitchToBlock(block));
                        
                        self.ctx.new_context();
                        for stmt in &i.cons{
                            self.translate_statement(None, stmt)?;
                        }
                        self.ctx.close_context();

                        self.bytecode.push(OpCode::Jump { to: exit });
                    }
                }
                self.bytecode.push(OpCode::SwitchToBlock(exit));
            },
            Stmt::Throw(t) => {

                #[cfg(test)]
                println!("throw");

                let r = self.translate_expr(&t.arg)?;
                self.bytecode.push(OpCode::Throw { value: r });
            },
            Stmt::Try(t) => {

                #[cfg(test)]
                println!("try");

                let old = self.catch_block;

                self.catch_block = Some(self.create_block());
                let exit = self.create_block();

                // try statment
                self.ctx.new_context();
                for stmt in &t.block.stmts{
                    self.translate_statement(None, stmt)?;
                }
                self.ctx.close_context();

                self.bytecode.push(OpCode::Jump { to: exit });
                
                // catch handler
                self.bytecode.push(OpCode::SwitchToBlock(self.catch_block.unwrap()));

                if let Some(h) = &t.handler{
                    self.ctx.new_context();

                    if let Some(p) = &h.param{
                        self.translate_pat_assign(p, self.r1, DeclareKind::Var)?;
                    }

                    for stmt in &h.body.stmts{
                        self.translate_statement(None, stmt)?;
                    }

                    self.ctx.close_context();
                }

                // finalizer
                self.bytecode.push(OpCode::SwitchToBlock(exit));

                if let Some(f) = &t.finalizer{
                    self.ctx.new_context();

                    for stmt in &f.stmts{
                        self.translate_statement(None, stmt)?;
                    }

                    self.ctx.close_context();
                }
            },
            Stmt::While(w) => {

                #[cfg(test)]
                println!("while");

                let (header, exit) = self.start_loop(label);
                self.ctx.new_context();

                let v = self.translate_expr(&w.test)?;
                self.bytecode.push(OpCode::JumpIfFalse { value: v, to: exit });

                self.translate_statement(None, &w.body)?;

                self.bytecode.push(OpCode::Jump { to: header});

                self.ctx.close_context();
                self.end_loop();
            },
            Stmt::With(w) => {
                todo!("With statment not supported")
            }
        };
        Ok(())
    }



    fn create_block(&mut self) -> Block{
        let b = Block(self.block_count);
        self.block_count += 1;

        self.bytecode.push(OpCode::CreateBlock(b));
        return b;
    }

    fn start_loop(&mut self, label:Option<&str>) -> (Block, Block){
        let exit = self.create_block();
        let header = self.create_block();

        if label.is_some(){
            self.loops.push_front(Loop { 
                label: Some(label.unwrap().to_string()), 
                exit: exit, 
                continue_: header })
        } else{
            self.loops.push_front(Loop { 
                label: None, 
                exit: exit, 
                continue_: header
            })
        };
        self.bytecode.push(OpCode::Jump { to: header });
        self.bytecode.push(OpCode::SwitchToBlock(header));
        return (header, exit);
    }

    fn end_loop(&mut self){
        let l = self.loops.pop_front().unwrap();
        self.bytecode.push(OpCode::Jump { to: l.exit });
        self.bytecode.push(OpCode::SwitchToBlock(l.exit));
    }

    fn try_check_error(&mut self, throw_value:Register){
        if self.catch_block.is_some(){
            self.bytecode.push(OpCode::JumpIfError { 
                to: self.catch_block.unwrap(), 
                throw_value: throw_value
            });
        }
    }


    fn translate_expr(&mut self, expr:&Expr) -> Result<Register, Error>{
        #[cfg(test)]
        print!("translate expr ");

        match expr{
            Expr::Array(a) => {
                #[cfg(test)]
                println!("array");

                let args_t = std::mem::size_of::<TempAllocValue>();
                let a_s = (a.elems.len() * args_t) as u32;
                

                // create an array of TempAlloc
                self.bytecode.push(OpCode::TempAlloc { size: a_s});

                let mut i = 0;
                for e in &a.elems{
                    if let Some(e) = &e{
                        
                        let elem = self.translate_expr(&e.expr)?;

                        self.bytecode.push(OpCode::StoreTempAlloc { 
                            offset: (i* args_t) as u16, 
                            flag:e.spread.is_some() as u8,
                            value: elem 
                        });

                    } else{

                        self.bytecode.push(OpCode::LoadUndefined { result: self.r1 });

                        self.bytecode.push(OpCode::StoreTempAlloc { 
                            offset: (i*JValue::SIZE) as u16, 
                            flag:0,
                            value: self.r1 
                        });
                    }

                    i +=1;
                }

                // create array will read from temp alloc
                self.bytecode.push(OpCode::CreateArray { 
                    result: self.r1,
                });

                self.bytecode.push(OpCode::TempDealloc { size:  a_s});
            },

            Expr::Arrow(a) => {
                #[cfg(test)]
                println!("arrow");

                let mut builder = FunctionBuilder::new_with_context(self.runtime.clone(), self.ctx.clone());

                let mut i = 0;
                for p in &a.params{
                    builder.translate_param_pat(p, i)?;
                    i += 1;
                }

                match &a.body{
                    BlockStmtOrExpr::BlockStmt(b) => {
                        for stmt in &b.stmts{
                            builder.translate_statement(None, stmt)?;
                        }
                    },
                    BlockStmtOrExpr::Expr(e) => {
                        let re = builder.translate_expr(&e)?;
                        builder.bytecode.push(OpCode::Return { value: re });
                    }
                };

                let id = builder.finish()?;

                self.bytecode.extend(self.ctx.need_done());

                self.bytecode.push(OpCode::LoadThis { result: self.r2 });

                self.bytecode.push(OpCode::CreateArrow { 
                    result: self.r1, 
                    this:self.r2,
                    id,
                });
            },

            Expr::Assign(a) => {

                #[cfg(test)]
                println!("assign");
                
                let v = self.translate_expr(&a.right)?;

                if a.op == AssignOp::Assign{

                    match &a.left{
                        PatOrExpr::Pat(p) => {
                            //self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                            let r = self.translate_pat_assign(&p, v, DeclareKind::None)?;
                            self.bytecode.push(OpCode::Mov { from: v, to: self.r1 });
                        },
                        PatOrExpr::Expr(expr) => {

                            self.bytecode.push(OpCode::StoreTemp { value: v });
                            
                            match expr.as_ref(){
                                Expr::Member(m) => {

                                    let obj = self.translate_expr(&m.obj)?;

                                    match &m.prop{
                                        MemberProp::Ident(i) => {
                                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                            let id = self.runtime.register_field_name(&i.sym);

                                            self.bytecode.push(OpCode::WriteFieldStatic { 
                                                obj_value: (obj, self.r3).into(), 
                                                field_id: id, 
                                                stack_offset: self.ctx.current_stack_offset()
                                            });
                                        },
                                        MemberProp::PrivateName(p) => {
                                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                            let id = self.runtime.register_field_name(&format!("#{}", p.id.sym));

                                            self.bytecode.push(OpCode::WriteFieldStatic { 
                                                obj_value:(obj, self.r3).into(), 
                                                field_id: id, 
                                                stack_offset:self.ctx.current_stack_offset()
                                            });
                                        },
                                        MemberProp::Computed(c) => {
                                            
                                            let field = self.translate_expr(&c.expr)?;
                                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                            self.bytecode.push(OpCode::WriteField { 
                                                obj: obj, 
                                                field,
                                                value:self.r3,
                                                stack_offset:self.ctx.current_stack_offset()
                                            });
                                        },
                                    };
                                    
                                },
                                e => todo!("assign expr as target\n{:#?}", e),
                            };

                            self.bytecode.push(OpCode::ReleaseTemp);
                        }
                    }

                } else {
                    
                    self.bytecode.push(OpCode::StoreTemp { value: v });

                    match &a.left{
                        PatOrExpr::Pat(p) => {
                            match p.as_ref(){
                                Pat::Ident(i) => {
                                    self.bytecode.push(self.ctx.get(&i.id.to_id(), self.r2));

                                },
                                _ => unreachable!("{} operation on pattern", a.op)
                            }
                        }
                        PatOrExpr::Expr(e) => {
                            let r = self.translate_expr(&e)?;
                            self.bytecode.push(OpCode::Mov { from: r, to: self.r2 });
                        }
                    }

                    // left as self.r2
                    // read right into self.r1
                    self.bytecode.push(OpCode::ReadTemp { value: self.r2 });

                    self.bytecode.push(
                    match a.op{
                        AssignOp::AddAssign => {
                            OpCode::Add { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::AndAssign => {
                            OpCode::And{ 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::Assign => unreachable!(),
                        AssignOp::BitAndAssign => {
                            OpCode::BitAnd { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::BitOrAssign => {
                            OpCode::BitOr { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::BitXorAssign => {
                            OpCode::BitXor { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::DivAssign => {
                            OpCode::Div { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::ExpAssign => {
                            OpCode::Exp { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::LShiftAssign => {
                            OpCode::LShift { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::ModAssign => {
                            OpCode::Rem { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::MulAssign => {
                            OpCode::Mul { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::NullishAssign => {
                            OpCode::Nullish { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::OrAssign => {
                            OpCode::Or { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::RShiftAssign => {
                            OpCode::RShift { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::SubAssign => {
                            OpCode::Sub { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                        AssignOp::ZeroFillRShiftAssign => {
                            OpCode::ZeroFillRShift { 
                                result: self.r1, 
                                left: self.r2, 
                                right:self.r1
                            }
                        },
                    });

                    self.bytecode.push(OpCode::ReleaseTemp);

                    match &a.left{
                        PatOrExpr::Pat(p) => {
                            match p.as_ref(){
                                Pat::Ident(i) => {
                                    self.bytecode.push(self.ctx.set(i.id.to_id(), self.r1));

                                },
                                _ => unreachable!("{} operation on pattern", a.op)
                            }
                        }
                        PatOrExpr::Expr(e) => {
                            match e.as_ref(){
                                Expr::Member(m) => {

                                    self.bytecode.push(OpCode::StoreTemp { value: self.r1 });

                                    let obj = self.translate_expr(&m.obj)?;

                                    match &m.prop{
                                        MemberProp::Ident(i) => {
                                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                            let id = self.runtime.register_field_name(&i.sym);

                                            self.bytecode.push(OpCode::WriteFieldStatic { 
                                                obj_value:(obj, self.r3).into(),
                                                field_id: id, 
                                                stack_offset:self.ctx.current_stack_offset()
                                            });
                                        },
                                        MemberProp::PrivateName(p) => {
                                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                            let id = self.runtime.register_field_name(&format!("#{}", p.id.sym));

                                            self.bytecode.push(OpCode::WriteFieldStatic { 
                                                obj_value:(obj, self.r3).into(),
                                                field_id: id, 
                                                stack_offset:self.ctx.current_stack_offset()
                                            });
                                        },
                                        MemberProp::Computed(c) => {
                                            
                                            let field = self.translate_expr(&c.expr)?;
                                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                            self.bytecode.push(OpCode::WriteField { 
                                                obj: obj, 
                                                field,
                                                value:self.r3,
                                                stack_offset:self.ctx.current_stack_offset()
                                            });
                                        },
                                    };

                                    self.bytecode.push(OpCode::ReleaseTemp);
                                },
                                Expr::Ident(i) => {
                                    self.bytecode.push(self.ctx.set(i.to_id(), self.r1));
                                },
                                _ => unimplemented!()
                            }
                        }
                    };
                }
                
            },

            Expr::Await(a) => {

                #[cfg(test)]
                println!("await");

                let v = self.translate_expr(&a.arg)?;
                self.bytecode.push(OpCode::Await { result: self.r1, future: v });

                return Ok(self.r1)
            },

            Expr::Bin(b) => {

                #[cfg(test)]
                println!("bin");

                let v = self.translate_expr(&b.left)?;
                self.bytecode.push(OpCode::StoreTemp { value: v });

                let r = self.translate_expr(&b.right)?;
                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                match b.op{
                    BinaryOp::Add => self.bytecode.push(OpCode::Add { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::BitAnd => self.bytecode.push(OpCode::BitAnd { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::BitOr => self.bytecode.push(OpCode::BitOr { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::BitXor => self.bytecode.push(OpCode::BitXor { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::Div => self.bytecode.push(OpCode::Div { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::EqEq => self.bytecode.push(OpCode::EqEq { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::EqEqEq => self.bytecode.push(OpCode::EqEqEq { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::Exp => self.bytecode.push(OpCode::Exp { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::Gt => self.bytecode.push(OpCode::Gt { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::GtEq => self.bytecode.push(OpCode::Add { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::In => self.bytecode.push(OpCode::In { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::InstanceOf => self.bytecode.push(OpCode::InstanceOf { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::LShift => self.bytecode.push(OpCode::LShift { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::LogicalAnd => self.bytecode.push(OpCode::And { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::LogicalOr => self.bytecode.push(OpCode::Or { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::Lt => self.bytecode.push(OpCode::Lt { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::LtEq => self.bytecode.push(OpCode::LtEq { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::Mod => self.bytecode.push(OpCode::Rem { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::Mul => self.bytecode.push(OpCode::Mul { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::NotEq => self.bytecode.push(OpCode::NotEq { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::NotEqEq => self.bytecode.push(OpCode::NotEqEq { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::NullishCoalescing => self.bytecode.push(OpCode::Nullish { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::RShift => self.bytecode.push(OpCode::RShift { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::Sub => self.bytecode.push(OpCode::Sub { result: self.r1, left: self.r3, right: r }),
                    BinaryOp::ZeroFillRShift => self.bytecode.push(OpCode::ZeroFillRShift { result: self.r1, left: self.r3, right: r }),
                };
            },

            Expr::Call(c) => {

                #[cfg(test)]
                println!("call");

                self.translate_args(&c.args)?;

                match &c.callee{
                    Callee::Expr(e) => {

                        if let Some(m) = e.as_member(){

                            let obj = self.translate_expr(&m.obj)?;

                            self.bytecode.push(OpCode::StoreTemp { value: obj });

                            let callee = self.translate_expr(&e)?;
                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                            self.bytecode.push(OpCode::Call { 
                                result: self.r1, 
                                this: self.r3, 
                                callee: callee,
                                stack_offset:self.ctx.current_stack_offset()
                            });

                            self.bytecode.push(OpCode::ReleaseTemp);

                        } else{
                            let r = self.translate_expr(&e)?;
                            self.bytecode.push(OpCode::LoadThis { result: self.r3 });

                            self.bytecode.push(OpCode::Call { 
                                result: self.r1, 
                                this: self.r3, 
                                callee: r ,
                                stack_offset:self.ctx.current_stack_offset()
                            });
                        }
                    },
                    Callee::Super(s) => {
                        todo!()
                    },
                    Callee::Import(i) => {
                        todo!()
                    }
                }

                self.try_check_error(self.r1);
            },
            Expr::Class(c) => {

                #[cfg(test)]
                println!("class");

                self.translate_class(&c.class)?;

            },
            Expr::Cond(c) => {
                // test ? a: b
                #[cfg(test)]
                println!("cond");

                let test = self.translate_expr(&c.test)?;
                self.bytecode.push(OpCode::StoreTemp { value: test });

                let a = self.translate_expr(&c.cons)?;
                self.bytecode.push(OpCode::StoreTemp { value: a });

                let b = self.translate_expr(&c.alt)?;
                self.bytecode.push(OpCode::Mov { from: b, to: self.r3 });

                self.bytecode.push(OpCode::ReadTemp { value: self.r2 });
                self.bytecode.push(OpCode::ReleaseTemp);

                self.bytecode.push(OpCode::ReadTemp { value: self.r1 });
                self.bytecode.push(OpCode::ReleaseTemp);

                self.bytecode.push(OpCode::CondSelect { 
                    t:self.r1,
                    a: self.r2, 
                    b: self.r3, 
                    result: self.r1
                });
            },
            Expr::Fn(f) => {

                #[cfg(test)]
                println!("fn");

                let mut builder = FunctionBuilder::new_with_context(self.runtime.clone(), self.ctx.clone());

                builder.build_function(&f.function)?;

                let id = builder.finish()?;

                self.bytecode.extend(self.ctx.need_done());

                self.bytecode.push(OpCode::CreateFunction { 
                    result: self.r1, 
                    id,
                });
            },

            Expr::Ident(i) => {
                #[cfg(test)]
                println!("ident");

                self.bytecode.push(self.ctx.get(&i.to_id(), self.r1));
            },

            Expr::Invalid(i) => {
                return Err(Error::InvalidExpression { 
                    pos:  i.span.lo.0..i.span.hi.0
                })
            },
            Expr::Lit(l) => {

                #[cfg(test)]
                println!("lit");

                match l{
                    Lit::BigInt(b) => {
                        if b.value < i32::MAX.into(){
                            self.bytecode.push(OpCode::LoadStaticBigInt32 { 
                                result: self.r1, 
                                value:  b.value.to_i32().unwrap()
                            });
                        } else{
                            let id = self.runtime.to_mut().unamed_constant(JValue::BigInt(b.value.to_i64().unwrap()));
                            self.bytecode.push(OpCode::LoadStaticBigInt { 
                                result: self.r1, 
                                id,
                            });
                        }
                    },
                    Lit::Bool(b) => {
                        if b.value{
                            self.bytecode.push(OpCode::LoadTrue { result: self.r1 });
                        } else {
                            self.bytecode.push(OpCode::LoadFalse { result: self.r1 });
                        }
                    },
                    Lit::JSXText(j) => {
                        todo!()
                    },
                    Lit::Null(n) => {
                        self.bytecode.push(OpCode::LoadNull { result: self.r1 });
                    },
                    Lit::Num(n) => {

                        if n.value > f32::MAX as f64{
                            let id = self.runtime.to_mut().unamed_constant(JValue::Number(n.value));
                        
                            self.bytecode.push(OpCode::LoadStaticFloat { 
                                result: self.r1, 
                                id, 
                            });
                        } else{
                            self.bytecode.push(OpCode::LoadStaticFloat32 { 
                                result: self.r1, 
                                value: n.value as f32
                            });
                        }
                        
                    },
                    Lit::Regex(r) => {
                        let id = self.runtime.to_mut().register_regex(&r.exp, &r.flags);
                        self.bytecode.push(OpCode::CreateRegExp { 
                            result: self.r1, 
                            reg_id: id
                        });
                    },
                    Lit::Str(s) => {
                        let id = self.runtime.to_mut().register_string(&s.value);
                        self.bytecode.push(OpCode::LoadStaticString { 
                            result: self.r1, 
                            id: id
                        });
                    },
                }
            },
            Expr::Member(m) => {
                let obj = self.translate_expr(&m.obj)?;
                match &m.prop{
                    MemberProp::Computed(c) => {
                        self.bytecode.push(OpCode::StoreTemp { value: obj });

                        let p = self.translate_expr(&c.expr)?;
                        self.bytecode.push(OpCode::Mov { from: p, to: self.r1 });

                        self.bytecode.push(OpCode::ReadTemp { value: self.r2 });
                        self.bytecode.push(OpCode::ReleaseTemp);

                        self.bytecode.push(OpCode::ReadField { 
                            obj: self.r2, 
                            field: p, 
                            result: self.r1,
                            stack_offset:self.ctx.current_stack_offset()
                        });
                    },
                    MemberProp::Ident(i) => {
                        let id = self.runtime.register_field_name(&i.sym);
                        self.bytecode.push(OpCode::ReadFieldStatic { 
                            obj_result:(obj, self.r1).into(), 
                            field_id: id, 
                            stack_offset:self.ctx.current_stack_offset()
                        });
                    },
                    MemberProp::PrivateName(p) => {
                        let id = self.runtime.register_field_name(&format!("#{}", &p.id));
                        self.bytecode.push(OpCode::ReadFieldStatic { 
                            obj_result:(obj, self.r1).into(),
                            field_id: id, 
                            stack_offset:self.ctx.current_stack_offset()
                        });
                    },
                };
                
            },
            Expr::MetaProp(m) => {
                match m.kind{
                    MetaPropKind::ImportMeta => {

                    },
                    MetaPropKind::NewTarget => {

                    }
                };
                todo!("meta prop {}", m.kind)
            },
            Expr::New(n) => {
                if let Some(args) = &n.args{
                    self.translate_args(args)?;
                }
                let callee = self.translate_expr(&n.callee)?;
                
                self.bytecode.push(OpCode::New { 
                    result: self.r1, 
                    callee,
                    stack_offset:self.ctx.current_stack_offset()
                });

                self.try_check_error(self.r1);
                return Ok(self.r1)
            },

            Expr::Object(o) => {

                self.bytecode.push(OpCode::CreateObject { result: self.r3 });
                self.bytecode.push(OpCode::StoreTemp { value: self.r3 });

                for prop in &o.props{
                    match prop{
                        PropOrSpread::Prop(p) => {
                            match p.as_ref(){
                                Prop::Assign(a) => {

                                },
                                Prop::Getter(g) => {

                                },
                                Prop::KeyValue(k) => {

                                },
                                Prop::Method(m) => {
                                
                                },
                                Prop::Setter(s) => {
                                    let mut builder = FunctionBuilder::new_with_context(self.runtime.clone(), self.ctx.clone());
                                    builder.translate_param_pat(&s.param, 0)?;
                                    if let Some(v) = &s.body{
                                        for i in &v.stmts{
                                            builder.translate_statement(None, i)?;
                                        }
                                    }
                                    let id = builder.finish()?;

                                    self.bytecode.extend(self.ctx.need_done());

                                    self.bytecode.push(OpCode::CreateFunction { result: self.r2, id: id });

                                    let key = self.propname_to_str(&s.key);
                                    let id = self.runtime.register_field_name(&key);
                                    self.bytecode.push(OpCode::BindSetter { 
                                        obj: self.r3, 
                                        field_id: id, 
                                        setter:self.r2
                                    });
                                },
                                Prop::Shorthand(s) => {
                                    self.bytecode.push(self.ctx.get(&s.to_id(), self.r2));
                                    let id = self.runtime.register_field_name(&s.sym);
                                    self.bytecode.push(OpCode::WriteFieldStatic { 
                                        obj_value:(self.r3, self.r2).into(),
                                        field_id: id, 
                                        stack_offset:self.ctx.current_stack_offset()
                                    });
                                }
                            }
                        },
                        PropOrSpread::Spread(s) => {
                            todo!()
                        }
                    };
                };
                self.bytecode.push(OpCode::ReleaseTemp);
                self.bytecode.push(OpCode::Mov { from: self.r3, to: self.r1 })
            },
            Expr::OptChain(o) => {
                match &o.base{
                    OptChainBase::Member(m) => {
                        let obj = self.translate_expr(&m.obj)?;
                        match &m.prop{
                            MemberProp::Computed(c) => {
                                self.bytecode.push(OpCode::StoreTemp { value: obj });

                                let p = self.translate_expr(&c.expr)?;
                                self.bytecode.push(OpCode::Mov { from: p, to: self.r1 });

                                self.bytecode.push(OpCode::ReadTemp { value: self.r2 });
                                self.bytecode.push(OpCode::ReleaseTemp);

                                self.bytecode.push(OpCode::ReadFieldOptChain { 
                                    obj: self.r2, 
                                    field: p, 
                                    result: self.r1,
                                    stack_offset:self.ctx.current_stack_offset()
                                });
                            },
                            MemberProp::Ident(i) => {
                                let id = self.runtime.register_field_name(&i.sym);
                                self.bytecode.push(OpCode::ReadFieldStaticOptChain { 
                                    obj_result:(obj, self.r1).into(),
                                    field_id: id, 
                                    stack_offset:self.ctx.current_stack_offset()
                                });
                            },
                            MemberProp::PrivateName(p) => {
                                let id = self.runtime.register_field_name(&format!("#{}", &p.id));
                                self.bytecode.push(OpCode::ReadFieldStaticOptChain { 
                                    obj_result:(obj, self.r1).into(),
                                    field_id: id, 
                                    stack_offset:self.ctx.current_stack_offset()
                                });
                            },
                        };
                    },
                    OptChainBase::Call(c) => {
                        self.translate_args(&c.args)?;

                        if let Some(m) = c.callee.as_member(){

                            let obj = self.translate_expr(&m.obj)?;

                            self.bytecode.push(OpCode::StoreTemp { value: obj });

                            let callee = self.translate_expr(&c.callee)?;
                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                            self.bytecode.push(OpCode::CallOptChain { 
                                result: self.r1, 
                                this: self.r3, 
                                callee: callee,
                                stack_offset:self.ctx.current_stack_offset()
                            });

                            self.bytecode.push(OpCode::ReleaseTemp);

                        } else{
                            let r = self.translate_expr(&c.callee)?;
                            self.bytecode.push(OpCode::LoadThis { result: self.r3 });

                            self.bytecode.push(OpCode::CallOptChain { 
                                result: self.r1, 
                                this: self.r3, 
                                callee: r ,
                                stack_offset:self.ctx.current_stack_offset()
                            });
                        }
                    }
                }
            },
            Expr::Paren(p) => {
                self.translate_expr(&p.expr)?;
            },
            Expr::PrivateName(p) => {
                todo!("private name expr {}", &p.id)
            },
            Expr::Seq(s) => {
                let mut r = self.r1;
                for i in &s.exprs{
                    r = self.translate_expr(&i)?;
                };
                return Ok(r)
            },
            Expr::SuperProp(s) => {
                todo!("super prop expr")
            },
            Expr::TaggedTpl(t) => {

            },
            Expr::This(t) => {
                self.bytecode.push(OpCode::LoadThis { result: self.r1 });
            },
            Expr::Tpl(t) => {

            },
            Expr::Unary(u) => {
                let arg = self.translate_expr(&u.arg)?;
                match u.op{
                    UnaryOp::Bang => {
                        self.bytecode.push(OpCode::Not { result: self.r1, right: arg });
                    },
                    UnaryOp::Delete => {
                        // todo!()
                    },
                    UnaryOp::Minus => {
                        self.bytecode.push(OpCode::Minus { result: self.r1, right: arg });
                    },
                    UnaryOp::Plus => {
                        self.bytecode.push(OpCode::Plus { result: self.r1, right: arg });
                    },
                    UnaryOp::Tilde => {
                        self.bytecode.push(OpCode::BitNot { result: self.r1, right: arg })
                    },
                    UnaryOp::TypeOf => {
                        self.bytecode.push(OpCode::TypeOf { result: self.r1, right: arg });
                    },
                    UnaryOp::Void => {
                        self.bytecode.push(OpCode::LoadUndefined { result: self.r1 });
                    },
                }
            },
            Expr::Update(u) => {
                let value = self.translate_expr(&u.arg)?;
                self.bytecode.push(OpCode::LoadStaticFloat32 { result: self.r2, value: 1.0 });
                self.bytecode.push(OpCode::Add { result: self.r1, left: value, right: self.r2 });

                match u.arg.as_ref(){
                    Expr::Member(m) => {
                        let obj = self.translate_expr(&m.obj)?;
                        todo!()
                    },
                    Expr::Ident(i) => {
                        todo!()
                    },
                    _ => todo!()
                }
            },
            Expr::Yield(y) => {
                todo!("yield expr")
            },

            Expr::TsAs(t) => {
                todo!()
            },
            Expr::TsConstAssertion(c) => {
                todo!()
            },
            Expr::TsInstantiation(i) => {
                todo!()
            },
            Expr::TsNonNull(n) => {
                todo!()
            },
            Expr::TsTypeAssertion(t) => {
                todo!()
            }

            Expr::JSXElement(e) => {
                todo!()
            },
            Expr::JSXEmpty(e) => {
                todo!()
            },
            Expr::JSXFragment(f) => {
                todo!()
            },
            Expr::JSXMember(m) => {
                todo!()
            },
            Expr::JSXNamespacedName(n) => {
                todo!()
            }
            
        };
        return Ok(self.r1)
    }

    fn translate_class(&mut self, class:&Class) -> Result<Register, Error>{
        let class_id = self.runtime.new_class();

        let sup = if let Some(s) = &class.super_class{
            let i = self.translate_expr(s)?;
            if i != self.r1{
                self.bytecode.push(OpCode::Mov { from: i, to: self.r1 });
            }
            true
        } else{ 
            false
        };

        self.bytecode.push(OpCode::CreateClass { 
            result: self.r3, 
            class_id,
        });

        if sup{
            self.bytecode.push(OpCode::ClassBindSuper { 
                class: self.r3, 
                super_: self.r1
            });
        };

        self.bytecode.push(OpCode::StoreTemp { value: self.r3 });

        for i in &class.body{
            match i{
                ClassMember::Constructor(c) => {
                    let mut builder = FunctionBuilder::new_with_context(self.runtime.clone(), self.ctx.clone());
                    
                    if let Some(b) = &c.body{
                        
                        let mut i = 0;
                        for p in &c.params{
                            match p{
                                ParamOrTsParamProp::Param(p) => {
                                    builder.translate_param(p, i)?;
                                },
                                ParamOrTsParamProp::TsParamProp(t) => {
                                    todo!()
                                }
                            };
                            i += 1;
                        }
                        for i in &b.stmts{
                            builder.translate_statement(None, &i)?;
                        }
                    }

                    let func_id = builder.finish()?;
                    self.bytecode.extend(self.ctx.need_done());
                    self.runtime.bind_class_constructor(class_id, func_id);
                },

                ClassMember::Method(m) => {

                    let mut builder = FunctionBuilder::new_with_context(self.runtime.clone(), self.ctx.clone());
                    builder.build_function(&m.function)?;
                    let func_id = builder.finish()?;

                    self.bytecode.extend(self.ctx.need_done());

                    let name = self.propname_to_str(&m.key);
                    if m.is_static{
                        if m.kind == MethodKind::Method{
                            self.runtime.bind_class_static_method(class_id, &name, func_id);

                        } else if m.kind == MethodKind::Getter{
                            self.runtime.bind_class_static_getter(class_id, &name, func_id);

                        } else if m.kind == MethodKind::Setter{
                            self.runtime.bind_class_static_setter(class_id, &name, func_id);
                        };
                    } else{
                        if m.kind == MethodKind::Method{
                            self.runtime.bind_class_method(class_id, &name, func_id);

                        } else if m.kind == MethodKind::Getter{
                            self.runtime.bind_class_getter(class_id, &name, func_id);

                        } else if m.kind == MethodKind::Setter{
                            self.runtime.bind_class_setter(class_id, &name, func_id);
                        };
                    }
                    
                },

                ClassMember::PrivateMethod(m) => {

                    let mut builder = FunctionBuilder::new_with_context(self.runtime.clone(), self.ctx.clone());
                    builder.build_function(&m.function)?;
                    let func_id = builder.finish()?;

                    self.bytecode.extend(self.ctx.need_done());

                    let name = format!("#{}", &m.key.id.sym);
                    if m.is_static{
                        if m.kind == MethodKind::Method{
                            self.runtime.bind_class_static_method(class_id, &name, func_id);

                        } else if m.kind == MethodKind::Getter{
                            self.runtime.bind_class_static_getter(class_id, &name, func_id);

                        } else if m.kind == MethodKind::Setter{
                            self.runtime.bind_class_static_setter(class_id, &name, func_id);
                        };
                    } else{
                        if m.kind == MethodKind::Method{
                            self.runtime.bind_class_method(class_id, &name, func_id);

                        } else if m.kind == MethodKind::Getter{
                            self.runtime.bind_class_getter(class_id, &name, func_id);

                        } else if m.kind == MethodKind::Setter{
                            self.runtime.bind_class_setter(class_id, &name, func_id);
                        };
                    }
                },

                ClassMember::ClassProp(p) => {

                    let name = self.propname_to_str(&p.key);

                    let field_id = if p.is_static{
                        self.runtime.bind_class_static_prop(class_id, &name)
                    } else{
                        self.runtime.bind_class_prop(class_id, &name)
                    };

                    if let Some(e) = &p.value{
                        let v = self.translate_expr(&e)?;

                        if v != self.r1{
                            self.bytecode.push(OpCode::Mov { from: v, to: self.r1 });
                        }
                        self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                        if !p.is_static{
                            // read 'prototype' into self.r3
                            self.bytecode.push(OpCode::ReadFieldStatic { 
                                obj_result:(self.r3, self.r3).into() ,
                                field_id: self.runtime.register_field_name("prototype"), 
                                stack_offset:self.ctx.current_stack_offset(),
                            });
                        }

                        self.bytecode.push(OpCode::WriteFieldStatic { 
                            obj_value:(self.r3, self.r1).into(),
                            field_id, 
                            stack_offset:self.ctx.current_stack_offset()
                        });
                    }
                },
                ClassMember::PrivateProp(p) => {
                    let name = format!("#{}", &p.key.id.sym);

                    let field_id = if p.is_static{
                        self.runtime.bind_class_static_prop(class_id, &name)
                    } else{
                        self.runtime.bind_class_prop(class_id, &name)
                    };

                    if let Some(e) = &p.value{
                        let v = self.translate_expr(&e)?;

                        if v != self.r1{
                            self.bytecode.push(OpCode::Mov { from: v, to: self.r1 });
                        }
                        self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                        if !p.is_static{
                            // read 'prototype' into self.r3
                            self.bytecode.push(OpCode::ReadFieldStatic { 
                                obj_result:(self.r3, self.r3).into(), 
                                field_id: self.runtime.register_field_name("prototype"), 
                                stack_offset:self.ctx.current_stack_offset()
                            });
                        }

                        // write value into class.prototype.field
                        self.bytecode.push(OpCode::WriteFieldStatic { 
                            obj_value:(self.r3, self.r1).into(),
                            field_id, 
                            stack_offset:self.ctx.current_stack_offset()
                        });
                    }
                },
                ClassMember::Empty(_e) => {
                    // empty
                },
                ClassMember::StaticBlock(s) => {
                    
                    self.bytecode.push(OpCode::ReadTemp { value: self.r1 });

                    // load the old this into temp
                    self.bytecode.push(OpCode::LoadThis { result: self.r3 });
                    self.bytecode.push(OpCode::StoreTemp { value: self.r3 });
                    // set 'this' to class
                    self.bytecode.push(OpCode::SetThis { value: self.r1 });

                    self.ctx.new_context();

                    for stmt in &s.body.stmts{
                        self.translate_statement(None, stmt)?;
                    }

                    self.ctx.close_context();

                    self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                    self.bytecode.push(OpCode::ReleaseTemp);

                    // set 'this' to the old
                    self.bytecode.push(OpCode::SetThis { value: self.r3 });

                },
                ClassMember::TsIndexSignature(i) => {
                    todo!()
                }
            }
        }

        self.bytecode.push(OpCode::ReadTemp { value: self.r1 });
        self.bytecode.push(OpCode::ReleaseTemp);

        Ok(self.r1)
    }

    fn translate_vardeclare(&mut self, d:&VarDeclarator, kind:VarDeclKind) -> Result<(), Error>{
        let value = if let Some(e) = &d.init{
            self.translate_expr(&e)?
        } else{
            self.bytecode.push(OpCode::LoadUndefined { result: self.r1 });
            self.r1
        };
        
        let kind = match kind {
            VarDeclKind::Const => DeclareKind::Const,
            VarDeclKind::Let => DeclareKind::Let,
            VarDeclKind::Var => DeclareKind::Var
        };
        self.translate_pat_assign(&d.name, value, kind);

        return Ok(())
    }

    fn translate_param(&mut self, param:&Param, index:u32) -> Result<(), Error>{
        #[cfg(test)]
        println!("translate param {}", index);
        self.translate_param_pat(&param.pat, index)
    }

    fn translate_param_pat(&mut self, pat:&Pat, index:u32) -> Result<(), Error>{
        self.bytecode.push(OpCode::ReadParam { 
            result: self.r1, 
            index: index
        });

        if let Some(p) = pat.as_rest(){
            self.bytecode.push(OpCode::CollectParam { result: self.r2, start: index });
            self.translate_pat_assign(&p.arg, self.r2, DeclareKind::Var)?;
        } else{
            self.translate_pat_assign(pat, self.r1, DeclareKind::Var)?;
        }
        Ok(())
    }

    fn translate_pat_assign(&mut self, pat:&Pat, value:Register, declare:DeclareKind) -> Result<Register, Error>{
        #[cfg(test)]
        print!("translate pat assign ");

        match pat{
            Pat::Ident(i) => {
                #[cfg(test)]
                println!("ident");

                let code = self.ctx.declare(i.to_id(), value, declare);
                self.bytecode.push(code);
                Ok(value)
            },
            // default value if value is undefined
            Pat::Assign(a) => {
                #[cfg(test)]
                println!("assign");

                let v = self.translate_expr(&a.right)?;

                self.bytecode.push(OpCode::Select { 
                    a: value, 
                    b: v, 
                    result: self.r1
                });

                self.translate_pat_assign(&a.left, self.r1, declare)?;
                
                Ok(self.r1)
            },
            
            Pat::Expr(e) => {
                #[cfg(test)]
                println!("expr");

                todo!("expression assign pattern")
            },

            Pat::Invalid(_i) => {
                #[cfg(test)]
                print!("invalid");

                Ok(self.r1)
            },

            Pat::Array(a) => {
                #[cfg(test)]
                println!("array");

                self.bytecode.push(OpCode::IntoIter { 
                    target: value, 
                    hint: LoopHint::ForOf
                });

                let mut i = 0;
                let mut re = value;
                for p in &a.elems{

                    let p = match p{
                        Some(p) => p,
                        None => continue
                    };

                    let re = if p.is_rest(){

                        let r = p.as_rest().unwrap();
                        self.bytecode.push(OpCode::IterCollect { 
                            result: self.r1 ,
                            stack_offset:self.ctx.current_stack_offset()
                        });
                        self.translate_pat_assign(&r.arg, self.r1, declare)?
                        
                    } else{
                        self.bytecode.push(OpCode::IterNext { 
                            result: self.r1, 
                            hint: LoopHint::ForOf,
                            stack_offset:self.ctx.current_stack_offset()
                        });
                        self.translate_pat_assign(p, self.r1, declare)?
                    };
                    i += 1;
                };
                self.bytecode.push(OpCode::IterDrop);
                Ok(re)
            },

            Pat::Object(o) => {
                #[cfg(test)]
                println!("object");

                let mut names = vec![];

                for p in &o.props{
                    match p{
                        ObjectPatProp::KeyValue(k) => {
                            // prop = value.get(field);
                            let field_id = self.runtime.register_field_name(&self.propname_to_str(&k.key));
                            names.push(field_id);

                            self.bytecode.push(OpCode::ReadFieldStatic { 
                                obj_result:(value, self.r1).into(),
                                field_id, 
                                stack_offset:self.ctx.current_stack_offset()
                            });

                            self.translate_pat_assign(&k.value, self.r1, declare)?;
                        },
                        ObjectPatProp::Assign(a) => {
                            let field_id = self.runtime.register_field_name(&a.key.sym);
                            names.push(field_id);

                            if let Some(v) = &a.value{
                                let v = self.translate_expr(&v)?;

                                self.bytecode.push(OpCode::Select { 
                                    a: value, 
                                    b: v, 
                                    result: self.r1
                                });

                                let code = self.ctx.declare(a.key.to_id(), self.r1, declare);
                                self.bytecode.push(code);

                            } else{
                                let code = self.ctx.declare(a.key.to_id(), value, declare);
                                self.bytecode.push(code);
                            } 
                        },
                        ObjectPatProp::Rest(r) => {
                            self.bytecode.push(OpCode::CloneObject { 
                                obj: value, 
                                result: self.r1 
                            });

                            for i in &names{
                                self.bytecode.push(OpCode::RemoveFieldStatic { 
                                    obj: self.r1, 
                                    field_id: *i
                                });
                            }

                            self.translate_pat_assign(&r.arg, self.r1, declare)?;
                        }
                    }
                };
                Ok(self.r1)
            },
            Pat::Rest(r) => {
                /// a param rest pattern
                #[cfg(test)]
                println!("rest");

                unimplemented!("rest pattern assign")
            }
        }
    }

    /// register arguments for a function call
    fn translate_args(&mut self, args:&[ExprOrSpread]) -> Result<(), Error>{

        self.bytecode.push(OpCode::CreateArg { 
            stack_offset:self.ctx.current_stack_offset(),
            len: args.len() as u32 
        });

        for arg in args{
            let a = self.translate_expr(&arg.expr)?;
            if arg.spread.is_some(){
                self.bytecode.push(OpCode::PushArgSpread { value: a });
            } else{
                self.bytecode.push(OpCode::PushArg { value: a });
            }
        }
        Ok(())
    }

    fn propname_to_str(&self, propname:&PropName) -> String{
        match propname{
            PropName::BigInt(b) => {
                b.value.to_string()
            },
            PropName::Ident(i) => {
                i.sym.to_string()
            },
            PropName::Num(n) => {
                n.value.to_string()
            },
            PropName::Str(s) => {
                s.value.to_string()
            },
            PropName::Computed(c) => {
                unimplemented!("computed propname")
            }
        }
    }

    fn finish(&mut self) -> Result<FuncID, Error>{
        self.bytecode.push(OpCode::LoadUndefined { result: self.r1 });
        self.bytecode.push(OpCode::Return { value: self.r1 });
        self.bytecode.extend(self.ctx.need_done());
        let id = self.runtime.new_function(
            Arc::new(JSFunction::ByteCodes{
                call_count:0,
                capture_stack_size:self.ctx.capture_len(),
                bytecodes:self.bytecode.clone()
            }
        ));
        self.ctx.close_context();
        Ok(id)
    }
}


#[cfg(test)]
mod test{
    use crate::runtime::Runtime;
    use crate::testing;

    #[test]
    pub fn test1(){
        let rt = Runtime::new();

        rt.clone().attach();

        let mut builder = super::FunctionBuilder::new(rt.clone());
        
        let script = testing::parse_script("
        let b = 0;
        function a(u, o){
            let c = 9;
            b = 0;
            r = function() {
                c += 1;
                return b
            };
            c += 1;
            return r
        }
        ");

        println!("parsed script");

        for i in &script.body{
            match builder.translate_statement(None, i){
                Ok(()) => {},
                Err(e) => {println!("error {:?}", e);}
            };
            println!("translated");
        }

        for i in &builder.bytecode{
            println!("{:?}", i);
        }

        println!("");

        for i in rt.get_function(crate::runtime::FuncID(1)){
            match i.as_ref(){
                crate::bultins::function::JSFunction::ByteCodes{bytecodes, ..} => {
                    for i in bytecodes.iter(){
                        println!("{:?}", i);
                    }
                },
                _ => unreachable!()
            }
        }
    }
}