use std::collections::VecDeque;
use std::sync::Arc;

use num_traits::ToPrimitive;
use swc_ecmascript::ast::*;

use super::function_builder_context::{DeclareKind, FunctionBuilderContext};
use crate::bultins::function::JSFunction;
use crate::error::Error;
use crate::runtime::{FuncID, Runtime};
use crate::types::JValue;
use crate::{bultins, bytecodes::*};

const SUPER_CONSTRUCTOR_VAR_NAME: &'static str = "SUPER CONSTRUCTOR";

type Res = Result<(), Error>;

struct Loop {
    label: Option<String>,
    exit: Block,
    continue_: Block,
}

pub struct FunctionBuilder {
    runtime: Arc<Runtime>,

    is_async: bool,
    is_generator: bool,
    args_len: u32,

    pub ctx: FunctionBuilderContext,
    loops: VecDeque<Loop>,

    block_count: u32,

    r1: Register,
    r2: Register,
    r3: Register,

    catch_block: Option<Block>,

    pub bytecode: Vec<OpCode>,
}

#[allow(unused)]
impl FunctionBuilder {
    fn new(runtime: Arc<Runtime>) -> Self {
        return Self {
            runtime,

            is_async: false,
            is_generator: false,
            args_len: 0,

            ctx: FunctionBuilderContext::new(),

            loops: VecDeque::new(),

            block_count: 0,
            bytecode: Vec::new(),

            catch_block: None,

            r1: Register(0),
            r2: Register(1),
            r3: Register(2),
        };
    }

    pub fn new_with_context(
        runtime: Arc<Runtime>,
        mut ctx: FunctionBuilderContext,
        is_async: bool,
        is_generator: bool,
        params:usize,
    ) -> Self {
        ctx.new_function(params);
        return Self {
            ctx: ctx,
            is_async,
            is_generator,
            ..Self::new(runtime)
        };
    }

    fn build_function(&mut self, func: &Function) -> Result<(), Error> {
        let mut i = 0;
        for p in &func.params {
            self.translate_param(p, i)?;
            i += 1;
        }
        if let Some(body) = &func.body {
            for stmt in &body.stmts {
                self.translate_statement(None, stmt)?;
            }
        }
        Ok(())
    }

    pub fn translate_statement(&mut self, label: Option<&str>, stmt: &Stmt) -> Res {
        match stmt {
            Stmt::Block(b) => {
                self.ctx.new_context();

                let mut v = Vec::new();
                for stmt in &b.stmts {
                    v.push(self.translate_statement(None, stmt)?);
                }
                self.ctx.close_context();
            }
            Stmt::Break(b) => {
                if let Some(break_label) = &b.label {
                    for l in &self.loops {
                        if let Some(label) = &l.label {
                            if label.as_str() == &break_label.sym {
                                self.bytecode.push(OpCode::Jump {
                                    to: l.exit,
                                    line: 0,
                                });
                                return Ok(());
                            }
                        }
                    }

                    return Err(Error::LabelUndefined(break_label.sym.to_string()));
                } else {
                    if let Some(l) = self.loops.front() {
                        self.bytecode.push(OpCode::Jump {
                            to: l.exit,
                            line: 0,
                        });
                    } else {
                        return Err(Error::IllegalBreak);
                    }
                }
            }
            Stmt::Continue(c) => {
                if let Some(break_label) = &c.label {
                    for l in &self.loops {
                        if let Some(label) = &l.label {
                            if label.as_str() == &break_label.sym {
                                self.bytecode.push(OpCode::Jump {
                                    to: l.continue_,
                                    line: 0,
                                });
                                return Ok(());
                            }
                        }
                    }

                    return Err(Error::LabelUndefined(break_label.sym.to_string()));
                } else {
                    if let Some(l) = self.loops.front() {
                        self.bytecode.push(OpCode::Jump {
                            to: l.continue_,
                            line: 0,
                        });
                    } else {
                        return Err(Error::IllegalBreak);
                    }
                }
            }
            Stmt::Debugger(d) => {
                self.bytecode.push(OpCode::Debugger);
            }
            Stmt::Decl(d) => match d {
                Decl::Class(class) => {
                    let c =
                        self.translate_class(&class.class, Some(class.ident.sym.to_string()))?;
                    let code = self.ctx.declare(class.ident.to_id(), c, DeclareKind::Let);
                    self.bytecode.push(code);
                }
                Decl::Fn(f) => {
                    let mut builder = FunctionBuilder::new_with_context(
                        self.runtime.clone(),
                        self.ctx.clone(),
                        f.function.is_async,
                        f.function.is_generator,
                        f.function.params.len()
                    );
                    builder.build_function(&f.function)?;
                    let id = builder.finish()?;

                    self.bytecode.extend(self.ctx.need_done());

                    self.bytecode.push(OpCode::CreateFunction {
                        result: self.r1,
                        id: id,
                    });

                    let code = self.ctx.declare(f.ident.to_id(), self.r1, DeclareKind::Let);
                    self.bytecode.push(code);
                }
                Decl::Var(v) => {
                    for d in &v.decls {
                        self.translate_vardeclare(d, v.kind)?;
                    }
                }
                Decl::TsEnum(_e) => {
                    todo!("TS enum")
                }
                Decl::TsInterface(_i) => {
                    todo!("TS interface")
                }
                Decl::TsModule(_m) => {
                    todo!("TS module")
                }
                Decl::TsTypeAlias(_t) => {
                    todo!("TS type alias")
                }
            },
            Stmt::DoWhile(d) => {
                let (header, exit) = self.start_loop(label);
                self.ctx.new_context();

                let start_len = self.bytecode.len();

                self.translate_statement(None, &d.body)?;

                let v = self.translate_expr(&d.test)?;

                //self.bytecode.push(OpCode::BreakIfFalse { value: v, exit:exit });
                self.bytecode.push(OpCode::JumpIfFalse {
                    value: v,
                    to: exit,
                    line: 0,
                });

                let code_len = self.bytecode.len() - start_len;

                //if let OpCode::Loop { body_start, body_len } = &mut self.bytecode[loop_pos]{
                //    *body_len = code_len as u16;
                //};

                self.bytecode.push(OpCode::Jump {
                    to: header,
                    line: 0,
                });

                self.ctx.close_context();
                self.end_loop();
            }
            Stmt::Empty(e) => {}
            Stmt::Expr(e) => {
                self.translate_expr(&e.expr)?;
            }
            Stmt::For(f) => {
                
                self.ctx.new_context();

                if let Some(v) = &f.init {
                    match v {
                        VarDeclOrExpr::Expr(e) => {
                            self.translate_expr(&e)?;
                        }
                        VarDeclOrExpr::VarDecl(v) => {
                            for i in &v.decls {
                                self.translate_vardeclare(i, v.kind)?;
                            }
                        }
                    }
                }
                
                let first_loop = self.create_block();
                let loop_body = self.create_block();
                let exit = self.create_block();

                // register a loop start
                self.loops.push_front(Loop {
                    label: label.map(|v|v.to_string()), 
                    exit: exit,
                    continue_: loop_body
                });

                // the first loop
                if let Some(e) = &f.test {
                    let test = self.translate_expr(&e)?;

                    self.bytecode.push(OpCode::JumpIfFalse {
                        value: test,
                        to: exit,
                        line: 0,
                    });
                }

                // run the first loop if condition true
                self.bytecode.push(OpCode::Jump { 
                    to: first_loop, 
                    line: 0
                });

                // switch to first loop
                self.bytecode.push(OpCode::SwitchToBlock(first_loop));

                self.translate_statement(None, &f.body)?;

                // jump to the second loop
                self.bytecode.push(OpCode::Jump { 
                    to: loop_body, 
                    line: 0
                });


                // switch the second and onward loops
                self.bytecode.push(OpCode::SwitchToBlock(loop_body));

                // update at first
                if let Some(e) = &f.update {
                    self.translate_expr(&e)?;
                };

                // jump to exit if test false
                if let Some(e) = &f.test {
                    let test = self.translate_expr(&e)?;

                    self.bytecode.push(OpCode::JumpIfFalse {
                        value: test,
                        to: exit,
                        line: 0,
                    });
                }

                // exacute body
                self.translate_statement(None, &f.body)?;

                // loop
                self.bytecode.push(OpCode::Jump {
                    to: loop_body,
                    line: 0,
                });

                self.loops.pop_front();

                // switch to exit
                self.bytecode.push(OpCode::SwitchToBlock(exit));
                
                self.ctx.close_context();

            }
            Stmt::ForIn(f) => {
                let v = self.translate_expr(&f.right)?;
                self.bytecode.push(OpCode::PrepareForIn { target: v });

                let (header, exit) = self.start_loop(label);
                self.ctx.new_context();

                let loop_pos = self.bytecode.len();

                self.bytecode.push(OpCode::IterNext {
                    result: self.r2,
                    done: self.r3,
                    hint: LoopHint::ForIn,
                    stack_offset: self.ctx.current_stack_offset(),
                });
                
                // exit if iter done
                self.bytecode
                    .push(OpCode::JumpIfTrue { value: self.r3, to: exit, line: 0 });

                match &f.left {
                    VarDeclOrPat::Pat(p) => {
                        self.translate_pat_assign(p, self.r2, DeclareKind::None)?;
                    }
                    VarDeclOrPat::VarDecl(v) => {
                        let kind = match v.kind {
                            VarDeclKind::Const => DeclareKind::Const,
                            VarDeclKind::Let => DeclareKind::Let,
                            VarDeclKind::Var => DeclareKind::Var,
                        };
                        for i in &v.decls {
                            self.translate_pat_assign(&i.name, self.r2, kind)?;
                        }
                    }
                }

                self.translate_statement(None, &f.body)?;

                // loop again
                self.bytecode.push(OpCode::Jump {
                    to: header,
                    line: 0,
                });

                self.ctx.close_context();
                self.end_loop();

                self.bytecode.push(OpCode::IterDrop);
            }
            Stmt::ForOf(f) => {
                let v = self.translate_expr(&f.right)?;
                self.bytecode.push(OpCode::PrepareForOf { target: v });

                let (header, exit) = self.start_loop(label);
                self.ctx.new_context();

                let loop_pos = self.bytecode.len();
                //self.bytecode.push(OpCode::Loop { body_start: self.bytecode.len() as u32 + 1, body_len: 0 });

                let start_len = self.bytecode.len();

                self.bytecode.push(OpCode::IterNext {
                    result: self.r2,
                    done: self.r3,
                    hint: LoopHint::ForOf,
                    stack_offset: self.ctx.current_stack_offset(),
                });

                self.bytecode
                    .push(OpCode::JumpIfTrue { 
                        value: self.r3,
                        to: exit, 
                        line: 0 
                    });

                match &f.left {
                    VarDeclOrPat::Pat(p) => {
                        self.translate_pat_assign(p, self.r2, DeclareKind::None)?;
                    }
                    VarDeclOrPat::VarDecl(v) => {
                        let kind = match v.kind {
                            VarDeclKind::Const => DeclareKind::Const,
                            VarDeclKind::Let => DeclareKind::Let,
                            VarDeclKind::Var => DeclareKind::Var,
                        };
                        for i in &v.decls {
                            self.translate_pat_assign(&i.name, self.r2, kind)?;
                        }
                    }
                }

                self.translate_statement(None, &f.body)?;

                let code_len = self.bytecode.len() - start_len;

                //if let OpCode::Loop { body_start, body_len } = &mut self.bytecode[loop_pos]{
                //    *body_len = code_len as u16;
                //};

                self.bytecode.push(OpCode::Jump {
                    to: header,
                    line: 0,
                });

                self.ctx.close_context();
                self.end_loop();

                self.bytecode.push(OpCode::IterDrop);
            }
            Stmt::If(i) => {
                let cond = self.create_block();
                let exit = self.create_block();

                let value = self.translate_expr(&i.test)?;

                self.bytecode.push(OpCode::JumpIfTrue {
                    value,
                    to: cond,
                    line: 0,
                });

                // fallthrough else block
                if let Some(stmt) = &i.alt {
                    self.ctx.new_context();

                    self.translate_statement(None, &stmt)?;

                    self.ctx.close_context();
                }
                self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                // if block
                self.bytecode.push(OpCode::SwitchToBlock(cond));
                self.ctx.new_context();

                self.translate_statement(None, &i.cons)?;

                self.ctx.close_context();
                self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                self.bytecode.push(OpCode::SwitchToBlock(exit));
            }
            Stmt::Labeled(l) => {
                self.translate_statement(Some(&l.label.sym), &l.body)?;
            }
            Stmt::Return(r) => {
                let r = if let Some(e) = &r.arg {
                    self.translate_expr(&e)?
                } else {
                    self.bytecode
                        .push(OpCode::LoadUndefined { result: self.r1 });
                    self.r1
                };
                self.bytecode.push(OpCode::Return { value: r });
            }
            Stmt::Switch(s) => {
                let target = self.translate_expr(&s.discriminant)?;

                let exit = self.create_block();

                let mut bs = Vec::new();

                for i in &s.cases {
                    if let Some(cmp) = &i.test {
                        let block = self.create_block();
                        let cmp = self.translate_expr(cmp)?;
                        self.bytecode.push(OpCode::ReadTemp { value: self.r2 });

                        self.bytecode.push(OpCode::EqEqEq {
                            result: self.r1,
                            left: cmp,
                            right: self.r2,
                        });
                        self.bytecode.push(OpCode::JumpIfTrue {
                            value: self.r1,
                            to: block,
                            line: 0,
                        });

                        bs.push((block, i));
                    } else {
                        // default
                        self.ctx.new_context();

                        for stmt in &i.cons {
                            self.translate_statement(None, stmt)?;
                        }

                        self.ctx.close_context();
                        self.bytecode.push(OpCode::Jump { to: exit, line: 0 });
                    }
                }
                // incase there is not default
                self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                for (block, i) in bs {
                    if let Some(cmp) = &i.test {
                        self.bytecode.push(OpCode::SwitchToBlock(block));

                        self.ctx.new_context();
                        for stmt in &i.cons {
                            self.translate_statement(None, stmt)?;
                        }
                        self.ctx.close_context();

                        self.bytecode.push(OpCode::Jump { to: exit, line: 0 });
                    }
                }
                self.bytecode.push(OpCode::SwitchToBlock(exit));
            }
            Stmt::Throw(t) => {
                let r = self.translate_expr(&t.arg)?;
                self.bytecode.push(OpCode::Throw { value: r });
            }
            Stmt::Try(t) => {
                let old = self.catch_block;

                self.catch_block = Some(self.create_block());
                let exit = self.create_block();

                self.bytecode.push(OpCode::EnterTry {
                    catch_block: self.catch_block.unwrap(),
                    line: 0,
                });

                // try statment
                self.ctx.new_context();
                for stmt in &t.block.stmts {
                    self.translate_statement(None, stmt)?;
                }
                self.ctx.close_context();

                self.bytecode.push(OpCode::ExitTry);
                self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                // catch handler
                self.bytecode
                    .push(OpCode::SwitchToBlock(self.catch_block.unwrap()));

                self.bytecode.push(OpCode::ExitTry);

                if let Some(h) = &t.handler {
                    self.ctx.new_context();

                    if let Some(p) = &h.param {
                        self.translate_pat_assign(p, self.r1, DeclareKind::Var)?;
                    }

                    for stmt in &h.body.stmts {
                        self.translate_statement(None, stmt)?;
                    }

                    self.ctx.close_context();
                }

                // finalizer
                self.bytecode.push(OpCode::SwitchToBlock(exit));

                if let Some(f) = &t.finalizer {
                    self.ctx.new_context();

                    for stmt in &f.stmts {
                        self.translate_statement(None, stmt)?;
                    }

                    self.ctx.close_context();
                }
            }
            Stmt::While(w) => {
                let (header, exit) = self.start_loop(label);
                self.ctx.new_context();

                let loop_pos = self.bytecode.len();
                //self.bytecode.push(OpCode::Loop { body_start: self.bytecode.len() as u32 + 1, body_len: 0 });

                let start_len = self.bytecode.len();

                let v = self.translate_expr(&w.test)?;

                // interpreter will jump to exit immediately after break,
                // while the compiler ignores breaks
                //self.bytecode.push(OpCode::BreakIfFalse { value: v, exit: exit });
                self.bytecode.push(OpCode::JumpIfFalse {
                    value: v,
                    to: exit,
                    line: 0,
                });

                self.translate_statement(None, &w.body)?;

                let code_len = self.bytecode.len() - start_len;

                //if let OpCode::Loop { body_start, body_len } = &mut self.bytecode[loop_pos]{
                //    *body_len = code_len as u16;
                //};

                // incase of an interpreter, this instruction will never be reached
                self.bytecode.push(OpCode::Jump {
                    to: header,
                    line: 0,
                });

                self.ctx.close_context();
                self.end_loop();
            }
            Stmt::With(w) => {
                todo!("with statment is deprecated.")
            }
        };
        Ok(())
    }

    fn create_block(&mut self) -> Block {
        let b = Block(self.block_count as u16);
        self.block_count += 1;

        self.bytecode.push(OpCode::CreateBlock(b));
        return b;
    }

    fn start_loop(&mut self, label: Option<&str>) -> (Block, Block) {
        let exit = self.create_block();
        let header = self.create_block();

        if label.is_some() {
            self.loops.push_front(Loop {
                label: Some(label.unwrap().to_string()),
                exit: exit,
                continue_: header,
            })
        } else {
            self.loops.push_front(Loop {
                label: None,
                exit: exit,
                continue_: header,
            })
        };
        self.bytecode.push(OpCode::Jump {
            to: header,
            line: 0,
        });
        self.bytecode.push(OpCode::SwitchToBlock(header));
        return (header, exit);
    }

    fn end_loop(&mut self) {
        let l = self.loops.pop_front().unwrap();
        self.bytecode.push(OpCode::Jump {
            to: l.exit,
            line: 0,
        });
        self.bytecode.push(OpCode::SwitchToBlock(l.exit));
    }

    fn try_check_error(&mut self, throw_value: Register) {}

    fn translate_expr(&mut self, expr: &Expr) -> Result<Register, Error> {
        match expr {
            Expr::Array(a) => {

                let base = self.ctx.current_stack_offset();
                let len = a.elems.len() as u32;

                self.bytecode.push(OpCode::CreateArg { stack_offset: base, len});

                let mut i = 0;

                for e in &a.elems {
                    
                    self.ctx.increment_stack_offset();

                    if let Some(e) = &e {
                        let elem = self.translate_expr(&e.expr)?;

                        

                        if e.spread.is_some(){
                            self.bytecode.push(OpCode::PushArgSpread { value: elem, stack_offset: self.ctx.current_stack_offset() });
                        } else{
                            self.bytecode.push(OpCode::PushArg { value: elem, stack_offset: self.ctx.current_stack_offset() });
                        }
                        
                    } else {
                        self.bytecode
                            .push(OpCode::LoadUndefined { result: self.r1 });

                        self.bytecode.push(OpCode::PushArg { value: self.r1, stack_offset: self.ctx.current_stack_offset() });
                    }

                    i += 1;
                };

                self.bytecode.push(OpCode::FinishArgs { base_stack_offset: base, len: len as u16});

                // create array will read from args
                self.bytecode.push(OpCode::CreateArray { 
                    result: self.r1,
                    stack_offset: base
                });
                
                self.ctx.decrease_stack_offset(a.elems.len());
            }

            Expr::Arrow(a) => {
                let mut builder = FunctionBuilder::new_with_context(
                    self.runtime.clone(),
                    self.ctx.clone(),
                    a.is_async,
                    a.is_generator,
                    a.params.len()
                );

                let mut i = 0;
                for p in &a.params {
                    builder.translate_param_pat(p, i)?;
                    i += 1;
                }

                match &a.body {
                    BlockStmtOrExpr::BlockStmt(b) => {
                        for stmt in &b.stmts {
                            builder.translate_statement(None, stmt)?;
                        }
                    }
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
                    this: self.r2,
                    id,
                });
            }

            Expr::Assign(a) => {
                if a.op == AssignOp::Assign {
                    let rhs = self.translate_expr(&a.right)?;

                    match &a.left {
                        PatOrExpr::Pat(p) => {
                            //self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                            let r = self.translate_pat_assign(&p, rhs, DeclareKind::None)?;
                            self.bytecode.push(OpCode::Mov {
                                from: rhs,
                                to: self.r1,
                            });
                        }
                        PatOrExpr::Expr(expr) => {
                            self.bytecode.push(OpCode::StoreTemp { value: rhs });

                            match expr.as_ref() {
                                Expr::Member(m) => {
                                    let obj = self.translate_expr(&m.obj)?;

                                    match &m.prop {
                                        MemberProp::Ident(i) => {
                                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                            let id = self.runtime.register_field_name(&i.sym);

                                            self.bytecode.push(OpCode::WriteFieldStatic {
                                                obj: obj,
                                                value: self.r3,
                                                field_id: id,
                                            });
                                        }
                                        MemberProp::PrivateName(p) => {
                                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                            let id = self
                                                .runtime
                                                .register_field_name(&format!("#{}", p.id.sym));

                                            self.bytecode.push(OpCode::WriteFieldStatic {
                                                obj: obj,
                                                value: self.r3,
                                                field_id: id,
                                            });
                                        }
                                        MemberProp::Computed(c) => {
                                            let field = self.translate_expr(&c.expr)?;
                                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                            self.bytecode.push(OpCode::WriteField {
                                                obj: obj,
                                                field,
                                                value: self.r3,
                                                stack_offset: self.ctx.current_stack_offset(),
                                            });
                                        }
                                    };
                                }
                                e => todo!("assign expr as target\n{:#?}", e),
                            };

                            self.bytecode.push(OpCode::ReleaseTemp);
                        }
                    }
                } else {
                    let rhs = self.translate_expr(&a.right)?;

                    self.bytecode.push(OpCode::StoreTemp { value: rhs });

                    let op = match a.op {
                        AssignOp::AddAssign => OpCode::Add {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::AndAssign => OpCode::And {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::Assign => unreachable!(),
                        AssignOp::BitAndAssign => OpCode::BitAnd {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::BitOrAssign => OpCode::BitOr {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::BitXorAssign => OpCode::BitXor {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::DivAssign => OpCode::Div {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::ExpAssign => OpCode::Exp {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::LShiftAssign => OpCode::LShift {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::ModAssign => OpCode::Rem {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::MulAssign => OpCode::Mul {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::NullishAssign => OpCode::Nullish {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::OrAssign => OpCode::Or {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::RShiftAssign => OpCode::RShift {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::SubAssign => OpCode::Sub {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                        AssignOp::ZeroFillRShiftAssign => OpCode::ZeroFillRShift {
                            result: self.r1,
                            left: self.r2,
                            right: self.r1,
                        },
                    };

                    match &a.left {
                        PatOrExpr::Pat(p) => match p.as_ref() {
                            Pat::Ident(i) => {
                                self.bytecode.push(self.ctx.get(&i.id.to_id(), self.r2));
                            }
                            _ => unreachable!("{} operation on pattern", a.op),
                        },
                        PatOrExpr::Expr(e) => {
                            let r = self.translate_expr(&e)?;
                            self.bytecode.push(OpCode::Mov {
                                from: r,
                                to: self.r2,
                            });
                        }
                    }

                    // left as self.r2
                    // read right into self.r1
                    self.bytecode.push(OpCode::ReadTemp { value: self.r2 });

                    self.bytecode.push(op);

                    self.bytecode.push(OpCode::ReleaseTemp);

                    match &a.left {
                        PatOrExpr::Pat(p) => match p.as_ref() {
                            Pat::Ident(i) => {
                                self.bytecode.push(self.ctx.set(i.id.to_id(), self.r1));
                            }
                            _ => unreachable!("{} operation on pattern", a.op),
                        },
                        PatOrExpr::Expr(e) => match e.as_ref() {
                            Expr::Member(m) => {
                                self.bytecode.push(OpCode::StoreTemp { value: self.r1 });

                                let obj = self.translate_expr(&m.obj)?;

                                match &m.prop {
                                    MemberProp::Ident(i) => {
                                        self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                        let id = self.runtime.register_field_name(&i.sym);

                                        self.bytecode.push(OpCode::WriteFieldStatic {
                                            obj: obj,
                                            value: self.r3,
                                            field_id: id,
                                        });
                                    }
                                    MemberProp::PrivateName(p) => {
                                        self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                        let id = self
                                            .runtime
                                            .register_field_name(&format!("#{}", p.id.sym));

                                        self.bytecode.push(OpCode::WriteFieldStatic {
                                            obj: obj,
                                            value: self.r3,
                                            field_id: id,
                                        });
                                    }
                                    MemberProp::Computed(c) => {
                                        let field = self.translate_expr(&c.expr)?;
                                        self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                        self.bytecode.push(OpCode::WriteField {
                                            obj: obj,
                                            field,
                                            value: self.r3,
                                            stack_offset: self.ctx.current_stack_offset(),
                                        });
                                    }
                                };

                                self.bytecode.push(OpCode::ReleaseTemp);
                            }
                            Expr::Ident(i) => {
                                self.bytecode.push(self.ctx.set(i.to_id(), self.r1));
                            }
                            _ => unimplemented!(),
                        },
                    };
                }
            }

            Expr::Await(a) => {
                if a.arg.is_lit()
                    || a.arg.is_array()
                    || a.arg.is_arrow()
                    || a.arg.is_class()
                    || a.arg.is_fn_expr()
                    || a.arg.is_object()
                    || a.arg.is_update()
                {
                    self.translate_expr(&a.arg)?;
                } else {
                    let v = self.translate_expr(&a.arg)?;
                    self.bytecode.push(OpCode::Await {
                        result: self.r1,
                        future: v,
                    });
                }

                return Ok(self.r1);
            }

            Expr::Bin(b) => {
                // #name in object
                if b.left.is_private_name() && b.op == BinaryOp::In {
                    let name = b.left.as_private_name().unwrap();
                    let id = self
                        .runtime
                        .register_field_name(&format!("#{}", &name.id.sym));
                    let r = self.translate_expr(&b.right)?;

                    self.bytecode.push(OpCode::PrivateIn {
                        result: self.r1,
                        name: id,
                        right: r,
                    });
                    return Ok(self.r1);
                }

                let lhs = self.translate_expr(&b.left)?;

                // both are literals, use a constant value
                if b.left.is_lit() && b.right.is_lit() {
                    todo!()
                } else if let Some(lit) = b.right.as_lit() {
                    // the left hand side is a literal, use immediate ops

                    match lit {
                        Lit::BigInt(big) => {
                            // todo: bigint imm values

                            if let Some(v) = big.value.to_i32() {

                                //return Ok(self.r1);
                            } else if let Some(v) = big.value.to_u32() {

                                //return Ok(self.r1)
                            }
                        }

                        Lit::Num(n) => {
                            if (n.value as i32) as f64 == n.value {
                                let v = n.value as i32;
                                match b.op {
                                    BinaryOp::Add => self.bytecode.push(OpCode::AddImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::BitAnd => self.bytecode.push(OpCode::BitAndImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::BitOr => self.bytecode.push(OpCode::BitOrImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::BitXor => self.bytecode.push(OpCode::BitXorImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::Div => self.bytecode.push(OpCode::DivImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::EqEq => self.bytecode.push(OpCode::EqEqImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::EqEqEq => self.bytecode.push(OpCode::EqEqEqImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::Exp => self.bytecode.push(OpCode::ExpImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::Gt => self.bytecode.push(OpCode::GtImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::GtEq => self.bytecode.push(OpCode::GtEqImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::LShift => self.bytecode.push(OpCode::LShiftImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::LogicalAnd => self.bytecode.push(OpCode::AndImm {
                                        result: self.r1,
                                        left: lhs,
                                        right: v != 0,
                                    }),
                                    BinaryOp::LogicalOr => {
                                        self.bytecode.push(OpCode::LoadStaticFloat32 {
                                            result: self.r3,
                                            value: v as f32,
                                        });
                                        self.bytecode.push(OpCode::Or {
                                            result: self.r1,
                                            left: lhs,
                                            right: self.r3,
                                        });
                                    }
                                    BinaryOp::Lt => self.bytecode.push(OpCode::LtImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::LtEq => self.bytecode.push(OpCode::LtEqImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::Mul => self.bytecode.push(OpCode::MulImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::Mod => self.bytecode.push(OpCode::RemImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::NotEq => self.bytecode.push(OpCode::NotEqImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::NotEqEq => self.bytecode.push(OpCode::NotEqImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::NullishCoalescing => {
                                        self.bytecode.push(OpCode::LoadStaticFloat32 {
                                            result: self.r3,
                                            value: v as f32,
                                        });
                                        self.bytecode.push(OpCode::Nullish {
                                            result: self.r1,
                                            left: lhs,
                                            right: self.r3,
                                        });
                                    }
                                    BinaryOp::RShift => self.bytecode.push(OpCode::RShiftImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::Sub => self.bytecode.push(OpCode::SubImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::ZeroFillRShift => {
                                        self.bytecode.push(OpCode::ZeroFillRShiftImmI32 {
                                            result: self.r1,
                                            left: lhs,
                                            right: v,
                                        })
                                    }

                                    BinaryOp::In => {
                                        return Err(Error::TypeError(format!(
                                            "Cannot use 'in' operator to search in {}",
                                            v
                                        )))
                                    }
                                    BinaryOp::InstanceOf => {
                                        return Err(Error::TypeError(format!(
                                            "Right-hand side of 'instanceof' is not an object"
                                        )))
                                    }
                                };
                                return Ok(self.r1);
                            } else if (n.value as f32) as f64 == n.value {
                                let v = n.value as f32;

                                match b.op {
                                    BinaryOp::Add => self.bytecode.push(OpCode::AddImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::BitAnd => self.bytecode.push(OpCode::BitAndImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v as i32,
                                    }),
                                    BinaryOp::BitOr => self.bytecode.push(OpCode::BitOrImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v as i32,
                                    }),
                                    BinaryOp::BitXor => self.bytecode.push(OpCode::BitXorImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v as i32,
                                    }),
                                    BinaryOp::Div => self.bytecode.push(OpCode::DivImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::EqEq => self.bytecode.push(OpCode::EqEqImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::EqEqEq => self.bytecode.push(OpCode::EqEqEqImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::Exp => self.bytecode.push(OpCode::ExpImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::Gt => self.bytecode.push(OpCode::GtImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::GtEq => self.bytecode.push(OpCode::GtEqImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::LShift => self.bytecode.push(OpCode::LShiftImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v as i32,
                                    }),
                                    BinaryOp::LogicalAnd => self.bytecode.push(OpCode::AndImm {
                                        result: self.r1,
                                        left: lhs,
                                        right: v != 0.0,
                                    }),
                                    BinaryOp::LogicalOr => {
                                        self.bytecode.push(OpCode::LoadStaticFloat32 {
                                            result: self.r3,
                                            value: v as f32,
                                        });
                                        self.bytecode.push(OpCode::Or {
                                            result: self.r1,
                                            left: lhs,
                                            right: self.r3,
                                        });
                                    }
                                    BinaryOp::Lt => self.bytecode.push(OpCode::LtImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::LtEq => self.bytecode.push(OpCode::LtEqImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::Mul => self.bytecode.push(OpCode::MulImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::Mod => self.bytecode.push(OpCode::RemImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::NotEq => self.bytecode.push(OpCode::NotEqImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::NotEqEq => self.bytecode.push(OpCode::NotEqImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::NullishCoalescing => {
                                        self.bytecode.push(OpCode::LoadStaticFloat32 {
                                            result: self.r3,
                                            value: v as f32,
                                        });
                                        self.bytecode.push(OpCode::Nullish {
                                            result: self.r1,
                                            left: lhs,
                                            right: self.r3,
                                        });
                                    }
                                    BinaryOp::RShift => self.bytecode.push(OpCode::RShiftImmI32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v as i32,
                                    }),
                                    BinaryOp::Sub => self.bytecode.push(OpCode::SubImmF32 {
                                        result: self.r1,
                                        left: lhs,
                                        right: v,
                                    }),
                                    BinaryOp::ZeroFillRShift => {
                                        self.bytecode.push(OpCode::ZeroFillRShiftImmI32 {
                                            result: self.r1,
                                            left: lhs,
                                            right: v as i32,
                                        })
                                    }

                                    BinaryOp::In => {
                                        return Err(Error::TypeError(format!(
                                            "Cannot use 'in' operator to search in {}",
                                            v
                                        )))
                                    }
                                    BinaryOp::InstanceOf => {
                                        return Err(Error::TypeError(format!(
                                            "Right-hand side of 'instanceof' is not an object"
                                        )))
                                    }
                                };
                                return Ok(self.r1);
                            };
                        }
                        Lit::Str(s) => match b.op {
                            BinaryOp::Add => {
                                let id = self.runtime.to_mut().register_string(&s.value);
                                self.bytecode.push(OpCode::AddImmStr {
                                    result: self.r1,
                                    left: lhs,
                                    str: id,
                                });

                                return Ok(self.r1);
                            }
                            BinaryOp::LogicalAnd => {
                                if s.value.len() == 0 {
                                    self.bytecode.push(OpCode::LoadFalse { result: self.r1 });
                                } else {
                                    self.bytecode.push(OpCode::AndImm {
                                        result: self.r1,
                                        left: lhs,
                                        right: true,
                                    });
                                }

                                return Ok(self.r1);
                            }
                            _ => {}
                        },
                        Lit::Bool(bo) => match b.op {
                            BinaryOp::LogicalAnd => {
                                if !bo.value {
                                    self.bytecode.push(OpCode::LoadFalse { result: self.r1 });
                                    return Ok(self.r1);
                                } else {
                                    self.bytecode.push(OpCode::AndImm {
                                        result: self.r1,
                                        left: lhs,
                                        right: true,
                                    });
                                    return Ok(self.r1);
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                };
                // slow path

                let mut store_temp = true;

                let r = if b.left.is_ident() && b.right.is_ident() {
                    let l = b.left.as_ident().unwrap();
                    let rhs = b.right.as_ident().unwrap();

                    if l.sym == rhs.sym {
                        store_temp = false;
                        self.bytecode.push(OpCode::Mov {
                            from: lhs,
                            to: self.r3,
                        });
                        lhs
                    } else {
                        self.bytecode.push(OpCode::StoreTemp { value: lhs });

                        let r = self.translate_expr(&b.right)?;
                        self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                        r
                    }
                } else {
                    self.bytecode.push(OpCode::StoreTemp { value: lhs });

                    let r = self.translate_expr(&b.right)?;
                    self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                    r
                };

                match b.op {
                    BinaryOp::Add => self.bytecode.push(OpCode::Add {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::BitAnd => self.bytecode.push(OpCode::BitAnd {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::BitOr => self.bytecode.push(OpCode::BitOr {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::BitXor => self.bytecode.push(OpCode::BitXor {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::Div => self.bytecode.push(OpCode::Div {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::EqEq => self.bytecode.push(OpCode::EqEq {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::EqEqEq => self.bytecode.push(OpCode::EqEqEq {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::Exp => self.bytecode.push(OpCode::Exp {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::Gt => self.bytecode.push(OpCode::Gt {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::GtEq => self.bytecode.push(OpCode::Add {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::In => self.bytecode.push(OpCode::In {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::InstanceOf => self.bytecode.push(OpCode::InstanceOf {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::LShift => self.bytecode.push(OpCode::LShift {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::LogicalAnd => self.bytecode.push(OpCode::And {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::LogicalOr => self.bytecode.push(OpCode::Or {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::Lt => self.bytecode.push(OpCode::Lt {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::LtEq => self.bytecode.push(OpCode::LtEq {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::Mod => self.bytecode.push(OpCode::Rem {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::Mul => self.bytecode.push(OpCode::Mul {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::NotEq => self.bytecode.push(OpCode::NotEq {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::NotEqEq => self.bytecode.push(OpCode::NotEqEq {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::NullishCoalescing => self.bytecode.push(OpCode::Nullish {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::RShift => self.bytecode.push(OpCode::RShift {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::Sub => self.bytecode.push(OpCode::Sub {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                    BinaryOp::ZeroFillRShift => self.bytecode.push(OpCode::ZeroFillRShift {
                        result: self.r1,
                        left: self.r3,
                        right: r,
                    }),
                };

                if store_temp {
                    self.bytecode.push(OpCode::ReleaseTemp);
                }
            }

            Expr::Call(c) => {
                self.translate_args(&c.args)?;

                match &c.callee {
                    Callee::Expr(e) => {
                        if let Some(m) = e.as_member() {
                            let obj = self.translate_expr(&m.obj)?;

                            self.bytecode.push(OpCode::StoreTemp { value: obj });

                            let callee = self.translate_expr(&e)?;
                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                            self.bytecode.push(OpCode::Call {
                                result: self.r1,
                                this: self.r3,
                                callee: callee,
                                stack_offset: self.ctx.current_stack_offset(),
                                args_len: c.args.len() as u16
                            });

                            self.bytecode.push(OpCode::ReleaseTemp);

                        } else {
                            let r = self.translate_expr(&e)?;
                            self.bytecode.push(OpCode::LoadThis { result: self.r3 });

                            self.bytecode.push(OpCode::Call {
                                result: self.r1,
                                this: self.r3,
                                callee: r,
                                stack_offset: self.ctx.current_stack_offset(),
                                args_len: c.args.len() as u16
                            });
                        }
                    }
                    Callee::Super(s) => {
                        self.bytecode.push(OpCode::NewTarget { result: self.r1 });
                        self.bytecode.push(OpCode::LoadThis { result: self.r2 });
                        self.bytecode.push(OpCode::Call {
                            result: self.r1,
                            this: self.r2,
                            callee: self.r1,
                            stack_offset: self.ctx.current_stack_offset(),
                            args_len: c.args.len() as u16
                        });
                    }
                    Callee::Import(i) => {
                        todo!("dynamic import")
                    }
                }

                self.try_check_error(self.r1);
            }
            Expr::Class(c) => {
                self.translate_class(&c.class, c.ident.as_ref().map(|v| v.sym.to_string()))?;
            }
            Expr::Cond(c) => {
                // test ? a: b

                let test = self.translate_expr(&c.test)?;
                self.bytecode.push(OpCode::StoreTemp { value: test });

                let a = self.translate_expr(&c.cons)?;
                self.bytecode.push(OpCode::StoreTemp { value: a });

                let b = self.translate_expr(&c.alt)?;
                self.bytecode.push(OpCode::Mov {
                    from: b,
                    to: self.r3,
                });

                self.bytecode.push(OpCode::ReadTemp { value: self.r2 });
                self.bytecode.push(OpCode::ReleaseTemp);

                self.bytecode.push(OpCode::ReadTemp { value: self.r1 });
                self.bytecode.push(OpCode::ReleaseTemp);

                self.bytecode.push(OpCode::CondSelect {
                    t: self.r1,
                    a: self.r2,
                    b: self.r3,
                    result: self.r1,
                });
            }
            Expr::Fn(f) => {
                let mut builder = FunctionBuilder::new_with_context(
                    self.runtime.clone(),
                    self.ctx.clone(),
                    f.function.is_async,
                    f.function.is_generator,
                    f.function.params.len()
                );

                builder.build_function(&f.function)?;

                let id = builder.finish()?;

                self.bytecode.extend(self.ctx.need_done());

                self.bytecode.push(OpCode::CreateFunction {
                    result: self.r1,
                    id,
                });
            }

            Expr::Ident(i) => {
                self.bytecode.push(self.ctx.get(&i.to_id(), self.r1));
            }

            Expr::Invalid(i) => {
                return Err(Error::InvalidExpression {
                    pos: i.span.lo.0..i.span.hi.0,
                })
            }
            Expr::Lit(l) => match l {
                Lit::BigInt(b) => {
                    if b.value.bits() <= 32 {
                        self.bytecode.push(OpCode::LoadStaticBigInt32 {
                            result: self.r1,
                            value: b.value.to_i32().unwrap(),
                        });
                    } else {
                        let id = self.runtime.to_mut().unamed_constant(JValue::create_bigint(
                            b.value
                                .to_i128()
                                .expect("todo: support bigint larger then i128"),
                        ));
                        self.bytecode.push(OpCode::LoadStaticBigInt {
                            result: self.r1,
                            id,
                        });
                    }
                }
                Lit::Bool(b) => {
                    if b.value {
                        self.bytecode.push(OpCode::LoadTrue { result: self.r1 });
                    } else {
                        self.bytecode.push(OpCode::LoadFalse { result: self.r1 });
                    }
                }
                Lit::JSXText(j) => {
                    panic!("jsx text not supported")
                }
                Lit::Null(n) => {
                    self.bytecode.push(OpCode::LoadNull { result: self.r1 });
                }
                Lit::Num(n) => {
                    if n.value > f32::MAX as f64 {
                        let id = self
                            .runtime
                            .to_mut()
                            .unamed_constant(JValue::create_number(n.value));

                        self.bytecode.push(OpCode::LoadStaticFloat {
                            result: self.r1,
                            id,
                        });
                    } else {
                        self.bytecode.push(OpCode::LoadStaticFloat32 {
                            result: self.r1,
                            value: n.value as f32,
                        });
                    }
                }
                Lit::Regex(r) => {
                    let re = self.runtime.to_mut().register_regex(&r.exp, &r.flags);

                    let id = match re {
                        Ok(v) => v,
                        Err(e) => return Err(Error::SyntaxError(e)),
                    };
                    self.bytecode.push(OpCode::CreateRegExp {
                        result: self.r1,
                        reg_id: id,
                    });
                }
                Lit::Str(s) => {
                    let id = self.runtime.to_mut().register_string(&s.value);
                    self.bytecode.push(OpCode::LoadStaticString {
                        result: self.r1,
                        id: id,
                    });
                }
            },
            Expr::Member(m) => {
                let obj = self.translate_expr(&m.obj)?;
                match &m.prop {
                    MemberProp::Computed(c) => {
                        self.bytecode.push(OpCode::StoreTemp { value: obj });

                        let p = self.translate_expr(&c.expr)?;
                        self.bytecode.push(OpCode::Mov {
                            from: p,
                            to: self.r1,
                        });

                        self.bytecode.push(OpCode::ReadTemp { value: self.r2 });
                        self.bytecode.push(OpCode::ReleaseTemp);

                        self.bytecode.push(OpCode::ReadField {
                            obj: self.r2,
                            field: p,
                            result: self.r1,
                            stack_offset: self.ctx.current_stack_offset(),
                        });
                    }
                    MemberProp::Ident(i) => {
                        let id = self.runtime.register_field_name(&i.sym);
                        self.bytecode.push(OpCode::ReadFieldStatic {
                            obj: obj,
                            result: self.r1,
                            field_id: id,
                        });
                    }
                    MemberProp::PrivateName(p) => {
                        let id = self.runtime.register_field_name(&format!("#{}", &p.id));
                        self.bytecode.push(OpCode::ReadFieldStatic {
                            obj: obj,
                            result: self.r1,
                            field_id: id,
                        });
                    }
                };
            }
            Expr::MetaProp(m) => {
                match m.kind {
                    MetaPropKind::ImportMeta => {
                        self.bytecode.push(OpCode::ImportMeta { result: self.r1 });
                    }
                    MetaPropKind::NewTarget => {
                        self.bytecode.push(OpCode::NewTarget { result: self.r1 });
                    }
                };
            }
            Expr::New(n) => {
                if let Some(args) = &n.args {
                    self.translate_args(args)?;
                }
                let callee = self.translate_expr(&n.callee)?;

                self.bytecode.push(OpCode::New {
                    result: self.r1,
                    callee,
                    stack_offset: self.ctx.current_stack_offset(),
                    args_len: n.args.as_ref().map(|n|n.len() as u16).unwrap_or(0)
                });

                self.try_check_error(self.r1);
                return Ok(self.r1);
            }

            Expr::Object(o) => {
                self.bytecode.push(OpCode::CreateObject { result: self.r3 });
                self.bytecode.push(OpCode::StoreTemp { value: self.r3 });

                for prop in &o.props {
                    match prop {
                        PropOrSpread::Prop(p) => match p.as_ref() {
                            Prop::Assign(a) => {
                                unreachable!("invalid object literal")
                            }
                            Prop::Getter(g) => {
                                let mut builder = FunctionBuilder::new_with_context(
                                    self.runtime.clone(),
                                    self.ctx.clone(),
                                    false,
                                    false,
                                    0
                                );

                                if let Some(v) = &g.body {
                                    for i in &v.stmts {
                                        builder.translate_statement(None, i)?;
                                    }
                                }
                                let id = builder.finish()?;

                                self.bytecode.extend(self.ctx.need_done());

                                self.bytecode.push(OpCode::CreateFunction {
                                    result: self.r2,
                                    id: id,
                                });

                                let key = self.propname_to_str(&g.key);
                                let id = self.runtime.register_field_name(&key);

                                // read the object
                                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                self.bytecode.push(OpCode::BindGetter {
                                    obj: self.r3,
                                    field_id: id,
                                    getter: self.r2,
                                });
                            }
                            Prop::KeyValue(k) => {
                                let v = self.translate_expr(&k.value)?;

                                let key = self.propname_to_str(&k.key);
                                let id = self.runtime.register_field_name(&key);

                                // read the object
                                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                                self.bytecode.push(OpCode::WriteFieldStatic {
                                    obj: self.r3,
                                    value: v,
                                    field_id: id,
                                });
                            }
                            Prop::Method(m) => {
                                let mut builder = FunctionBuilder::new_with_context(
                                    self.runtime.clone(),
                                    self.ctx.clone(),
                                    m.function.is_async,
                                    m.function.is_generator,
                                    m.function.params.len()
                                );

                                builder.build_function(&m.function)?;
                                let id = builder.finish()?;

                                self.bytecode.extend(self.ctx.need_done());
                                self.bytecode.push(OpCode::CreateFunction {
                                    result: self.r2,
                                    id: id,
                                });

                                let key = self.propname_to_str(&m.key);
                                let id = self.runtime.register_field_name(&key);

                                // read the object
                                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                self.bytecode.push(OpCode::WriteFieldStatic {
                                    obj: self.r3,
                                    value: self.r2,
                                    field_id: id,
                                });
                            }
                            Prop::Setter(s) => {
                                let mut builder = FunctionBuilder::new_with_context(
                                    self.runtime.clone(),
                                    self.ctx.clone(),
                                    false,
                                    false,
                                    1
                                );

                                builder.translate_param_pat(&s.param, 0)?;
                                if let Some(v) = &s.body {
                                    for i in &v.stmts {
                                        builder.translate_statement(None, i)?;
                                    }
                                }
                                let id = builder.finish()?;

                                self.bytecode.extend(self.ctx.need_done());

                                self.bytecode.push(OpCode::CreateFunction {
                                    result: self.r2,
                                    id: id,
                                });

                                let key = self.propname_to_str(&s.key);
                                let id = self.runtime.register_field_name(&key);

                                // read the object
                                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                                self.bytecode.push(OpCode::BindSetter {
                                    obj: self.r3,
                                    field_id: id,
                                    setter: self.r2,
                                });
                            }
                            Prop::Shorthand(s) => {
                                self.bytecode.push(self.ctx.get(&s.to_id(), self.r2));
                                let id = self.runtime.register_field_name(&s.sym);

                                // read the object
                                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                self.bytecode.push(OpCode::WriteFieldStatic {
                                    obj: self.r3,
                                    value: self.r2,
                                    field_id: id,
                                });
                            }
                        },
                        PropOrSpread::Spread(s) => {
                            let e = self.translate_expr(&s.expr)?;
                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                            self.bytecode.push(OpCode::ExtendObject {
                                obj: self.r3,
                                from: e,
                            });
                        }
                    };
                }
                self.bytecode.push(OpCode::ReadTemp { value: self.r1 });
                self.bytecode.push(OpCode::ReleaseTemp);
            }

            Expr::OptChain(o) => match &o.base {
                OptChainBase::Member(m) => {
                    let obj = self.translate_expr(&m.obj)?;
                    match &m.prop {
                        MemberProp::Computed(c) => {
                            self.bytecode.push(OpCode::StoreTemp { value: obj });

                            let p = self.translate_expr(&c.expr)?;;
                            
                            // read the object to r3
                            self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                            self.bytecode.push(OpCode::ReleaseTemp);

                            let should_read = self.create_block();
                            let exit = self.create_block();

                            // check if is nullish
                            self.bytecode.push(OpCode::IsNullish { result: self.r2, value: self.r3 });
                            // jump to read field if not nullish
                            self.bytecode.push(OpCode::JumpIfFalse { value: self.r2, to: should_read, line: 0 });

                            // set result to undefined if nullish
                            self.bytecode.push(OpCode::LoadUndefined { result: self.r1 });
                            // jump to exit
                            self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                            // switch to the block if not nullish
                            self.bytecode.push(OpCode::SwitchToBlock(should_read));
                            
                            // read the field
                            self.bytecode.push(OpCode::ReadField {
                                obj: self.r3,
                                field: p,
                                result: self.r1,
                                stack_offset: self.ctx.current_stack_offset(),
                            });

                            // jump to exit
                            self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                            // switch to exit block
                            self.bytecode.push(OpCode::SwitchToBlock(exit));
                        }
                        MemberProp::Ident(i) => {
                            let id = self.runtime.register_field_name(&i.sym);

                            let should_read = self.create_block();
                            let exit = self.create_block();

                            self.bytecode.push(OpCode::IsNullish { result: self.r2, value: obj });
                            self.bytecode.push(OpCode::JumpIfFalse { value: self.r2, to: should_read, line: 0 });

                            self.bytecode.push(OpCode::LoadUndefined { result: self.r1 });
                            self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                            self.bytecode.push(OpCode::SwitchToBlock(should_read));

                            self.bytecode.push(OpCode::ReadFieldStatic{
                                obj: obj,
                                result: self.r1,
                                field_id: id,
                            });

                            self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                            self.bytecode.push(OpCode::SwitchToBlock(exit));
                        }
                        MemberProp::PrivateName(p) => {
                            let id = self.runtime.register_field_name(&format!("#{}", &p.id));
                            
                            let should_read = self.create_block();
                            let exit = self.create_block();

                            self.bytecode.push(OpCode::IsNullish { result: self.r2, value: obj });
                            self.bytecode.push(OpCode::JumpIfFalse { value: self.r2, to: should_read, line: 0 });

                            self.bytecode.push(OpCode::LoadUndefined { result: self.r1 });
                            self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                            self.bytecode.push(OpCode::SwitchToBlock(should_read));

                            self.bytecode.push(OpCode::ReadFieldStatic{
                                obj: obj,
                                result: self.r1,
                                field_id: id,
                            });

                            self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                            self.bytecode.push(OpCode::SwitchToBlock(exit));
                        }
                    };
                }
                OptChainBase::Call(c) => {
                    self.translate_args(&c.args)?;

                    if let Some(m) = c.callee.as_member() {
                        let obj = self.translate_expr(&m.obj)?;

                        self.bytecode.push(OpCode::StoreTemp { value: obj });

                        let callee = self.translate_expr(&c.callee)?;

                        self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                        self.bytecode.push(OpCode::ReleaseTemp);

                        let should_call = self.create_block();
                        let exit = self.create_block();

                        self.bytecode.push(OpCode::IsNullish { result: self.r2, value: callee });

                        self.bytecode.push(OpCode::JumpIfFalse { 
                            value: self.r2, 
                            to: should_call, 
                            line: 0 
                        });
                        
                        // load undefined if nullish
                        self.bytecode.push(OpCode::LoadUndefined { result: self.r1 });

                        self.bytecode.push(OpCode::Jump { 
                            to: exit, 
                            line: 0
                        });

                        self.bytecode.push(OpCode::SwitchToBlock(should_call));

                        self.bytecode.push(OpCode::Call {
                            result: self.r1,
                            this: self.r3,
                            callee: callee,
                            args_len: c.args.len() as _,
                            stack_offset: self.ctx.current_stack_offset(),
                        });

                        self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                        self.bytecode.push(OpCode::SwitchToBlock(exit));

                        
                    } else {

                        let callee = self.translate_expr(&c.callee)?;
                        
                        self.bytecode.push(OpCode::LoadThis { result: self.r3 });

                        let should_call = self.create_block();
                        let exit = self.create_block();

                        self.bytecode.push(OpCode::IsNullish { result: self.r2, value: callee });

                        self.bytecode.push(OpCode::JumpIfFalse { 
                            value: self.r2, 
                            to: should_call, 
                            line: 0 
                        });
                        
                        // load undefined if nullish
                        self.bytecode.push(OpCode::LoadUndefined { result: self.r1 });

                        self.bytecode.push(OpCode::Jump { 
                            to: exit, 
                            line: 0
                        });

                        self.bytecode.push(OpCode::SwitchToBlock(should_call));

                        self.bytecode.push(OpCode::Call {
                            result: self.r1,
                            this: self.r3,
                            callee: callee,
                            args_len: c.args.len() as _,
                            stack_offset: self.ctx.current_stack_offset(),
                        });

                        self.bytecode.push(OpCode::Jump { to: exit, line: 0 });

                        self.bytecode.push(OpCode::SwitchToBlock(exit));
                    }
                }
            },
            Expr::Paren(p) => {
                self.translate_expr(&p.expr)?;
            }
            Expr::PrivateName(p) => {
                todo!("private name expr {}", &p.id)
            }
            Expr::Seq(s) => {
                let mut r = self.r1;
                for i in &s.exprs {
                    r = self.translate_expr(&i)?;
                }
                return Ok(r);
            }
            Expr::SuperProp(s) => match &s.prop {
                SuperProp::Ident(i) => {
                    let id = self.runtime.register_field_name(&i.sym);

                    // capture the constructor
                    self.bytecode.push(self.ctx.get(
                        &(
                            swc_atoms::JsWord::from(SUPER_CONSTRUCTOR_VAR_NAME),
                            Default::default(),
                        ),
                        self.r2,
                    ));

                    self.bytecode.push(OpCode::ReadSuperFieldStatic {
                        constructor: self.r2,
                        result: self.r1,
                        field_id: id,
                    });
                }
                SuperProp::Computed(c) => {
                    let v = self.translate_expr(&c.expr)?;

                    // capture the constructor
                    self.bytecode.push(self.ctx.get(
                        &(
                            swc_atoms::JsWord::from(SUPER_CONSTRUCTOR_VAR_NAME),
                            Default::default(),
                        ),
                        self.r2,
                    ));

                    self.bytecode.push(OpCode::ReadSuperField {
                        constructor: self.r2,
                        result: self.r1,
                        field: v,
                        stack_offset: self.ctx.current_stack_offset(),
                    });
                }
            },

            Expr::This(t) => {
                self.bytecode.push(OpCode::LoadThis { result: self.r1 });
            }
            Expr::Tpl(t) => {
                let exprs = t.exprs.iter();
                self.bytecode.push(OpCode::CreateArg {
                    stack_offset: self.ctx.current_stack_offset(),
                    len: t.exprs.len() as u32,
                });

                let base = self.ctx.current_stack_offset();
                let mut count = 0;
                for i in exprs {
                    let v = self.translate_expr(&i)?;
                    self.bytecode.push(OpCode::PushArg {
                        value: v,
                        stack_offset: base + count as u16,
                    });
                    self.ctx.increment_stack_offset();
                    count += 1;
                }

                self.ctx.decrease_stack_offset(count);

                self.bytecode.push(OpCode::FinishArgs {
                    base_stack_offset: base,
                    len: count as u16,
                });

                let mut strs = Vec::new();
                for i in &t.quasis {
                    let s = i.raw.to_string();
                    strs.push(s);
                }

                let id = self.runtime.register_template(bultins::strings::Template {
                    strings: strs,
                    tagged: false,
                });

                self.bytecode.push(OpCode::CreateTemplate {
                    result: self.r1,
                    id,
                    stack_offset: self.ctx.current_stack_offset(),
                });
            }
            Expr::TaggedTpl(t) => {
                let exprs = t.tpl.exprs.iter();
                
                // the start of the arguments
                let base = self.ctx.current_stack_offset();

                // build the array of strings
                self.bytecode.push(OpCode::CreateArg { stack_offset: base, len: t.tpl.quasis.len() as _ });

                let mut i = 0;

                // load the string and push to args
                for e in &t.tpl.quasis{
                    let id = self.runtime.to_mut().register_string(&e.raw);
                    self.bytecode.push(OpCode::LoadStaticString { result: self.r1, id});
                    self.bytecode.push(OpCode::PushArg { value: self.r1, stack_offset: base + i});
                    i += 1;
                }
                self.bytecode.push(OpCode::FinishArgs { base_stack_offset: base, len: t.tpl.quasis.len() as u16 });
                // create the array
                self.bytecode.push(OpCode::CreateArray { result: self.r3, stack_offset: base });


                // create arguments for the tag call
                self.bytecode.push(OpCode::CreateArg {
                    stack_offset: base,
                    len: t.tpl.exprs.len() as u32 + 1,
                });
                // push array to args
                self.bytecode.push(OpCode::PushArg {
                    value: self.r3,
                    stack_offset: base,
                });

                // increase offset by 1
                self.ctx.increment_stack_offset();

                // number of arguments
                let mut count = 1;

                // translate all arguments
                for i in exprs {

                    let v = self.translate_expr(&i)?;

                    // push value to args
                    self.bytecode.push(OpCode::PushArg {
                        value: v,
                        stack_offset: base + count as u16,
                    });
                    self.ctx.increment_stack_offset();
                    count += 1;
                }

                // reverse the stack offset
                self.ctx.decrease_stack_offset(count as usize);

                // finish arguments
                self.bytecode.push(OpCode::FinishArgs {
                    base_stack_offset: base,
                    len: count as u16,
                });
                
                // get the tag function
                let tag = self.translate_expr(&t.tag)?;

                // load this into register
                self.bytecode.push(OpCode::LoadThis { result: self.r2 });

                // call the tag function
                self.bytecode.push(OpCode::Call { 
                    result: self.r1, 
                    this: self.r2, 
                    callee: tag, 
                    stack_offset: base, 
                    args_len: count
                });
            }
            Expr::Unary(u) => {
                let arg = self.translate_expr(&u.arg)?;
                match u.op {
                    UnaryOp::Bang => {
                        self.bytecode.push(OpCode::Not {
                            result: self.r1,
                            right: arg,
                        });
                    }
                    UnaryOp::Delete => {
                        // todo!()
                    }
                    UnaryOp::Minus => {
                        self.bytecode.push(OpCode::Minus {
                            result: self.r1,
                            right: arg,
                        });
                    }
                    UnaryOp::Plus => {
                        self.bytecode.push(OpCode::Plus {
                            result: self.r1,
                            right: arg,
                        });
                    }
                    UnaryOp::Tilde => self.bytecode.push(OpCode::BitNot {
                        result: self.r1,
                        right: arg,
                    }),
                    UnaryOp::TypeOf => {
                        self.bytecode.push(OpCode::TypeOf {
                            result: self.r1,
                            right: arg,
                        });
                    }
                    UnaryOp::Void => {
                        self.bytecode
                            .push(OpCode::LoadUndefined { result: self.r1 });
                    }
                }
            }
            Expr::Update(u) => {
                let value = self.translate_expr(&u.arg)?;

                if u.op == UpdateOp::PlusPlus {
                    self.bytecode.push(OpCode::AddImmI32 {
                        result: self.r3,
                        left: value,
                        right: 1,
                    });
                } else {
                    self.bytecode.push(OpCode::SubImmI32 {
                        result: self.r3,
                        left: value,
                        right: 1,
                    });
                };

                match u.arg.as_ref() {
                    Expr::Member(m) => {
                        // store the value before add
                        if !u.prefix {
                            self.bytecode.push(OpCode::StoreTemp { value: value });
                        };

                        self.bytecode.push(OpCode::StoreTemp { value: self.r3 });

                        let obj = self.translate_expr(&m.obj)?;

                        match &m.prop {
                            MemberProp::Ident(i) => {
                                self.bytecode.push(OpCode::ReadTemp { value: self.r2 });

                                let id = self.runtime.register_field_name(&i.sym);

                                self.bytecode.push(OpCode::WriteFieldStatic {
                                    obj: obj,
                                    value: self.r2,
                                    field_id: id,
                                });
                            }
                            MemberProp::PrivateName(p) => {
                                self.bytecode.push(OpCode::ReadTemp { value: self.r2 });

                                let id =
                                    self.runtime.register_field_name(&format!("#{}", &p.id.sym));

                                self.bytecode.push(OpCode::WriteFieldStatic {
                                    obj: obj,
                                    value: self.r2,
                                    field_id: id,
                                });
                            }
                            MemberProp::Computed(c) => {
                                self.bytecode.push(OpCode::StoreTemp { value: obj });

                                let key = self.translate_expr(&c.expr)?;

                                self.bytecode.push(OpCode::ReadTemp { value: self.r2 });
                                self.bytecode.push(OpCode::ReleaseTemp);
                                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                self.bytecode.push(OpCode::WriteField {
                                    obj: self.r2,
                                    field: key,
                                    value: self.r3,
                                    stack_offset: self.ctx.current_stack_offset(),
                                });
                            }
                        };

                        // read the added value as result
                        if u.prefix {
                            self.bytecode.push(OpCode::ReadTemp { value: self.r1 });
                        }
                        self.bytecode.push(OpCode::ReleaseTemp);

                        // read the value before add as result
                        if !u.prefix {
                            self.bytecode.push(OpCode::ReadTemp { value: self.r1 });
                            self.bytecode.push(OpCode::ReleaseTemp);
                        }
                    }

                    Expr::Ident(i) => {
                        self.bytecode.push(self.ctx.set(i.to_id(), self.r3));

                        // return value before add
                        if !u.prefix {
                            self.bytecode.push(OpCode::Mov {
                                from: value,
                                to: self.r1,
                            });

                        // return value after add
                        } else {
                            self.bytecode.push(OpCode::Mov {
                                from: self.r3,
                                to: self.r1,
                            });
                        }
                    }
                    _ => todo!(),
                };
            }
            Expr::Yield(y) => {
                let arg = if let Some(a) = &y.arg {
                    self.translate_expr(&a)?
                } else {
                    self.bytecode
                        .push(OpCode::LoadUndefined { result: self.r2 });
                    self.r2
                };

                if y.delegate {
                    todo!()
                } else {
                    self.bytecode.push(OpCode::Yield {
                        result: self.r1,
                        arg: arg,
                    });
                }
            }

            Expr::TsAs(t) => {
                todo!()
            }
            Expr::TsConstAssertion(c) => {
                todo!()
            }
            Expr::TsInstantiation(i) => {
                todo!()
            }
            Expr::TsNonNull(n) => {
                todo!()
            }
            Expr::TsTypeAssertion(t) => {
                todo!()
            }
            Expr::TsSatisfaction(s) => {}

            Expr::JSXElement(e) => {
                todo!()
            }
            Expr::JSXEmpty(e) => {
                todo!()
            }
            Expr::JSXFragment(f) => {
                todo!()
            }
            Expr::JSXMember(m) => {
                todo!()
            }
            Expr::JSXNamespacedName(n) => {
                todo!()
            }
        };
        return Ok(self.r1);
    }

    fn translate_class(&mut self, class: &Class, name: Option<String>) -> Result<Register, Error> {
        let class_id = self.runtime.new_class(name.unwrap_or(String::new()));

        let sup = if let Some(s) = &class.super_class {
            let i = self.translate_expr(s)?;
            if i != self.r1 {
                self.bytecode.push(OpCode::Mov {
                    from: i,
                    to: self.r1,
                });
            }
            true
        } else {
            false
        };

        self.bytecode.push(OpCode::CreateClass {
            result: self.r3,
            class_id,
        });

        // workaround: declare a builtin variable to be captured
        self.bytecode.push(self.ctx.declare(
            (
                swc_atoms::JsWord::from(SUPER_CONSTRUCTOR_VAR_NAME),
                swc_common::SyntaxContext::empty(),
            ),
            self.r3,
            DeclareKind::Var,
        ));

        if sup {
            self.bytecode.push(OpCode::ClassBindSuper {
                class: self.r3,
                super_: self.r1,
            });
        };

        self.bytecode.push(OpCode::StoreTemp { value: self.r3 });

        for i in &class.body {
            match i {
                ClassMember::Constructor(c) => {
                    let mut builder = FunctionBuilder::new_with_context(
                        self.runtime.clone(),
                        self.ctx.clone(),
                        false,
                        false,
                        c.params.len()
                    );

                    if let Some(b) = &c.body {
                        let mut i = 0;
                        for p in &c.params {
                            match p {
                                ParamOrTsParamProp::Param(p) => {
                                    builder.translate_param(p, i)?;
                                }
                                ParamOrTsParamProp::TsParamProp(t) => {
                                    todo!()
                                }
                            };
                            i += 1;
                        }
                        for i in &b.stmts {
                            builder.translate_statement(None, &i)?;
                        }
                    }

                    let func_id = builder.finish()?;
                    self.bytecode.extend(self.ctx.need_done());
                    self.runtime.bind_class_constructor(class_id, func_id);
                }

                ClassMember::Method(m) => {
                    let mut builder = FunctionBuilder::new_with_context(
                        self.runtime.clone(),
                        self.ctx.clone(),
                        m.function.is_async,
                        m.function.is_generator,
                        m.function.params.len()
                    );
                    builder.build_function(&m.function)?;
                    let func_id = builder.finish()?;

                    self.bytecode.extend(self.ctx.need_done());

                    let name = self.propname_to_str(&m.key);
                    if m.is_static {
                        if m.kind == MethodKind::Method {
                            self.runtime
                                .bind_class_static_method(class_id, &name, func_id);
                        } else if m.kind == MethodKind::Getter {
                            self.runtime
                                .bind_class_static_getter(class_id, &name, func_id);
                        } else if m.kind == MethodKind::Setter {
                            self.runtime
                                .bind_class_static_setter(class_id, &name, func_id);
                        };
                    } else {
                        if m.kind == MethodKind::Method {
                            self.runtime.bind_class_method(class_id, &name, func_id);
                        } else if m.kind == MethodKind::Getter {
                            self.runtime.bind_class_getter(class_id, &name, func_id);
                        } else if m.kind == MethodKind::Setter {
                            self.runtime.bind_class_setter(class_id, &name, func_id);
                        };
                    }
                }

                ClassMember::PrivateMethod(m) => {
                    let mut builder = FunctionBuilder::new_with_context(
                        self.runtime.clone(),
                        self.ctx.clone(),
                        m.function.is_async,
                        m.function.is_generator,
                        m.function.params.len()
                    );

                    builder.build_function(&m.function)?;
                    let func_id = builder.finish()?;

                    self.bytecode.extend(self.ctx.need_done());

                    let name = format!("#{}", &m.key.id.sym);
                    if m.is_static {
                        if m.kind == MethodKind::Method {
                            self.runtime
                                .bind_class_static_method(class_id, &name, func_id);
                        } else if m.kind == MethodKind::Getter {
                            self.runtime
                                .bind_class_static_getter(class_id, &name, func_id);
                        } else if m.kind == MethodKind::Setter {
                            self.runtime
                                .bind_class_static_setter(class_id, &name, func_id);
                        };
                    } else {
                        if m.kind == MethodKind::Method {
                            self.runtime.bind_class_method(class_id, &name, func_id);
                        } else if m.kind == MethodKind::Getter {
                            self.runtime.bind_class_getter(class_id, &name, func_id);
                        } else if m.kind == MethodKind::Setter {
                            self.runtime.bind_class_setter(class_id, &name, func_id);
                        };
                    }
                }

                ClassMember::ClassProp(p) => {
                    let name = self.propname_to_str(&p.key);

                    let field_id = if p.is_static {
                        self.runtime.bind_class_static_prop(class_id, &name)
                    } else {
                        self.runtime.bind_class_prop(class_id, &name)
                    };

                    if let Some(e) = &p.value {
                        let v = self.translate_expr(&e)?;

                        if v != self.r1 {
                            self.bytecode.push(OpCode::Mov {
                                from: v,
                                to: self.r1,
                            });
                        }
                        self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                        if !p.is_static {
                            // read 'prototype' into self.r3
                            self.bytecode.push(OpCode::ReadFieldStatic {
                                obj: self.r3,
                                result: self.r3,
                                field_id: self.runtime.register_field_name("prototype"),
                            });
                        }

                        self.bytecode.push(OpCode::WriteFieldStatic {
                            obj: self.r3,
                            value: self.r1,
                            field_id,
                        });
                    }
                }
                ClassMember::PrivateProp(p) => {
                    let name = format!("#{}", &p.key.id.sym);

                    let field_id = if p.is_static {
                        self.runtime.bind_class_static_prop(class_id, &name)
                    } else {
                        self.runtime.bind_class_prop(class_id, &name)
                    };

                    if let Some(e) = &p.value {
                        let v = self.translate_expr(&e)?;

                        if v != self.r1 {
                            self.bytecode.push(OpCode::Mov {
                                from: v,
                                to: self.r1,
                            });
                        }
                        self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                        if !p.is_static {
                            // read 'prototype' into self.r3
                            self.bytecode.push(OpCode::ReadFieldStatic {
                                obj: self.r3,
                                result: self.r3,
                                field_id: self.runtime.register_field_name("prototype"),
                            });
                        }

                        // write value into class.prototype.field
                        self.bytecode.push(OpCode::WriteFieldStatic {
                            obj: self.r3,
                            value: self.r1,
                            field_id,
                        });
                    }
                }
                ClassMember::Empty(_e) => {
                    // empty
                }
                ClassMember::StaticBlock(s) => {
                    self.bytecode.push(OpCode::ReadTemp { value: self.r1 });

                    // load the old this into temp
                    self.bytecode.push(OpCode::LoadThis { result: self.r3 });
                    self.bytecode.push(OpCode::StoreTemp { value: self.r3 });
                    // set 'this' to class
                    self.bytecode.push(OpCode::SetThis { value: self.r1 });

                    self.ctx.new_context();

                    for stmt in &s.body.stmts {
                        self.translate_statement(None, stmt)?;
                    }

                    self.ctx.close_context();

                    self.bytecode.push(OpCode::ReadTemp { value: self.r3 });
                    self.bytecode.push(OpCode::ReleaseTemp);

                    // set 'this' to the old
                    self.bytecode.push(OpCode::SetThis { value: self.r3 });
                }
                ClassMember::TsIndexSignature(i) => {
                    todo!()
                }
            }
        }

        self.bytecode.push(OpCode::ReadTemp { value: self.r1 });
        self.bytecode.push(OpCode::ReleaseTemp);

        Ok(self.r1)
    }

    fn translate_vardeclare(&mut self, d: &VarDeclarator, kind: VarDeclKind) -> Result<(), Error> {
        let value = if let Some(e) = &d.init {
            self.translate_expr(&e)?
        } else {
            self.bytecode
                .push(OpCode::LoadUndefined { result: self.r1 });
            self.r1
        };

        let kind = match kind {
            VarDeclKind::Const => DeclareKind::Const,
            VarDeclKind::Let => DeclareKind::Let,
            VarDeclKind::Var => DeclareKind::Var,
        };
        self.translate_pat_assign(&d.name, value, kind);

        return Ok(());
    }

    fn translate_param(&mut self, param: &Param, index: u32) -> Result<(), Error> {
        self.translate_param_pat(&param.pat, index)
    }

    fn translate_param_pat(&mut self, pat: &Pat, index: u32) -> Result<(), Error> {
        if self.args_len <= index {
            self.args_len = index + 1;
        }

        self.bytecode.push(OpCode::ReadParam {
            result: self.r1,
            index: index,
        });

        if let Some(p) = pat.as_rest() {
            self.bytecode.push(OpCode::CollectParam {
                result: self.r2,
                start: index,
            });
            self.translate_pat_assign(&p.arg, self.r2, DeclareKind::Var)?;
        } else {
            self.translate_pat_assign(pat, self.r1, DeclareKind::Var)?;
        }
        Ok(())
    }

    fn translate_pat_assign(
        &mut self,
        pat: &Pat,
        value: Register,
        declare: DeclareKind,
    ) -> Result<Register, Error> {
        match pat {
            Pat::Ident(i) => {
                let code = self.ctx.declare(i.to_id(), value, declare);
                self.bytecode.push(code);
                Ok(value)
            }
            // default value if value is undefined
            Pat::Assign(a) => {
                let v = self.translate_expr(&a.right)?;

                self.bytecode.push(OpCode::Select {
                    a: value,
                    b: v,
                    result: self.r1,
                });

                self.translate_pat_assign(&a.left, self.r1, declare)?;

                Ok(self.r1)
            }

            Pat::Expr(e) => {
                self.bytecode.push(OpCode::StoreTemp { value: value });

                match e.as_ref() {
                    Expr::Member(m) => {
                        let obj = self.translate_expr(&m.obj)?;

                        match &m.prop {
                            MemberProp::Ident(i) => {
                                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                let id = self.runtime.register_field_name(&i.sym);

                                self.bytecode.push(OpCode::WriteFieldStatic {
                                    obj: obj,
                                    value: self.r3,
                                    field_id: id,
                                });
                            }
                            MemberProp::PrivateName(p) => {
                                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                let id =
                                    self.runtime.register_field_name(&format!("#{}", p.id.sym));

                                self.bytecode.push(OpCode::WriteFieldStatic {
                                    obj: obj,
                                    value: self.r3,
                                    field_id: id,
                                });
                            }
                            MemberProp::Computed(c) => {
                                let field = self.translate_expr(&c.expr)?;
                                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                self.bytecode.push(OpCode::WriteField {
                                    obj: obj,
                                    field,
                                    value: self.r3,
                                    stack_offset: self.ctx.current_stack_offset(),
                                });
                            }
                        };
                    }
                    Expr::SuperProp(p) => {
                        match &p.prop {
                            SuperProp::Ident(i) => {
                                // capture the constructor
                                self.bytecode.push(self.ctx.get(
                                    &(
                                        swc_atoms::JsWord::from(SUPER_CONSTRUCTOR_VAR_NAME),
                                        Default::default(),
                                    ),
                                    self.r2,
                                ));
                                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                let key = self.runtime.register_field_name(&i.sym);

                                self.bytecode.push(OpCode::WriteSuperFieldStatic {
                                    constructor: self.r2,
                                    value: self.r3,
                                    field: key,
                                });
                            }
                            SuperProp::Computed(c) => {
                                let prop = self.translate_expr(&c.expr)?;
                                // capture the constructor
                                self.bytecode.push(self.ctx.get(
                                    &(
                                        swc_atoms::JsWord::from(SUPER_CONSTRUCTOR_VAR_NAME),
                                        Default::default(),
                                    ),
                                    self.r2,
                                ));
                                self.bytecode.push(OpCode::ReadTemp { value: self.r3 });

                                self.bytecode.push(OpCode::WriteSuperField {
                                    constructor: self.r2,
                                    value: self.r3,
                                    field: prop,
                                });
                            }
                        }
                    }
                    e => todo!("expression pattern assign {:#?}", e),
                };

                self.bytecode.push(OpCode::ReadTemp { value: self.r1 });
                self.bytecode.push(OpCode::ReleaseTemp);
                Ok(self.r1)
            }

            Pat::Invalid(_i) => Ok(self.r1),

            Pat::Array(a) => {
                self.bytecode.push(OpCode::PrepareForOf { target: value });

                let mut i = 0;
                let mut re = value;
                for p in &a.elems {
                    let p = match p {
                        Some(p) => p,
                        None => continue,
                    };

                    let re = if p.is_rest() {
                        let r = p.as_rest().unwrap();
                        self.bytecode.push(OpCode::IterCollect {
                            result: self.r1,
                            stack_offset: self.ctx.current_stack_offset(),
                        });
                        self.translate_pat_assign(&r.arg, self.r1, declare)?
                    } else {
                        self.bytecode.push(OpCode::IterNext {
                            result: self.r1,
                            done: self.r2,
                            hint: LoopHint::ForOf,
                            stack_offset: self.ctx.current_stack_offset(),
                        });
                        self.translate_pat_assign(p, self.r1, declare)?
                    };
                    i += 1;
                }
                self.bytecode.push(OpCode::IterDrop);
                Ok(re)
            }

            Pat::Object(o) => {
                let mut names = vec![];

                for p in &o.props {
                    match p {
                        ObjectPatProp::KeyValue(k) => {
                            // prop = value.get(field);
                            let field_id = self
                                .runtime
                                .register_field_name(&self.propname_to_str(&k.key));
                            names.push(field_id);

                            self.bytecode.push(OpCode::ReadFieldStatic {
                                obj: value,
                                result: self.r1,
                                field_id,
                            });

                            self.translate_pat_assign(&k.value, self.r1, declare)?;
                        }
                        ObjectPatProp::Assign(a) => {
                            let field_id = self.runtime.register_field_name(&a.key.sym);
                            names.push(field_id);

                            if let Some(v) = &a.value {
                                let v = self.translate_expr(&v)?;

                                self.bytecode.push(OpCode::Select {
                                    a: value,
                                    b: v,
                                    result: self.r1,
                                });

                                let code = self.ctx.declare(a.key.to_id(), self.r1, declare);
                                self.bytecode.push(code);
                            } else {
                                let code = self.ctx.declare(a.key.to_id(), value, declare);
                                self.bytecode.push(code);
                            }
                        }
                        ObjectPatProp::Rest(r) => {
                            self.bytecode.push(OpCode::CloneObject {
                                obj: value,
                                result: self.r1,
                            });

                            for i in &names {
                                self.bytecode.push(OpCode::RemoveFieldStatic {
                                    obj: self.r1,
                                    field_id: *i,
                                });
                            }

                            self.translate_pat_assign(&r.arg, self.r1, declare)?;
                        }
                    }
                }
                Ok(self.r1)
            }
            Pat::Rest(r) => {
                /// a param rest pattern
                unimplemented!("rest pattern assign")
            }
        }
    }

    /// register arguments for a function call
    fn translate_args(&mut self, args: &[ExprOrSpread]) -> Result<(), Error> {
        self.bytecode.push(OpCode::CreateArg {
            stack_offset: self.ctx.current_stack_offset(),
            len: args.len() as u32,
        });

        let base = self.ctx.current_stack_offset();
        let mut count: usize = 0;

        let mut need_spread = Vec::new();

        for arg in args {
            if arg.spread.is_some() {
                // expand the array lit without creating array
                fn spread_expr(
                    builder: &mut FunctionBuilder,
                    expr: &Expr,
                    base: u16,
                    count: &mut usize,
                    need_spread: &mut Vec<u16>,
                ) -> Res {
                    match expr {
                        Expr::Array(a) => {
                            for i in &a.elems {
                                match i {
                                    None => {
                                        builder.bytecode.push(OpCode::LoadUndefined {
                                            result: Register(0),
                                        });
                                        builder.bytecode.push(OpCode::PushArg {
                                            value: Register(0),
                                            stack_offset: base + *count as u16,
                                        });
                                        *count += 1;
                                        builder.ctx.increment_stack_offset();
                                    }
                                    Some(v) => {
                                        if v.spread.is_some() {
                                            spread_expr(builder, expr, base, count, need_spread)?;
                                        } else {
                                            let v = builder.translate_expr(&v.expr)?;
                                            *count += 1;
                                            builder.ctx.increment_stack_offset();
                                        }
                                    }
                                };
                            }
                        }
                        Expr::Paren(p) => {
                            spread_expr(builder, &p.expr, base, count, need_spread)?;
                        }
                        expr => {
                            let v = builder.translate_expr(expr)?;
                            need_spread.push(base + *count as u16);
                            *count += 1;
                            builder.ctx.increment_stack_offset();
                        }
                    };
                    Ok(())
                };
                spread_expr(self, &arg.expr, base, &mut count, &mut need_spread)?;
            } else {
                let a = self.translate_expr(&arg.expr)?;

                self.bytecode.push(OpCode::PushArg {
                    value: a,
                    stack_offset: base + count as u16,
                });
                count += 1;
                self.ctx.increment_stack_offset();
            }
        }

        if count > u16::MAX as usize {
            return Err(Error::FunctionCallArgumentsOverflow);
        }

        self.ctx.decrease_stack_offset(count as usize);

        for i in need_spread {
            self.bytecode.push(OpCode::SpreadArg {
                base_stack_offset: base,
                stack_offset: i,
                args_len: count as u16,
            });
        }

        self.bytecode.push(OpCode::FinishArgs {
            base_stack_offset: base,
            len: count as u16,
        });
        Ok(())
    }

    fn propname_to_str(&self, propname: &PropName) -> String {
        match propname {
            PropName::BigInt(b) => b.value.to_string(),
            PropName::Ident(i) => i.sym.to_string(),
            PropName::Num(n) => n.value.to_string(),
            PropName::Str(s) => s.value.to_string(),
            PropName::Computed(c) => {
                unimplemented!("computed propname")
            }
        }
    }

    pub fn pat_to_names<'a>(&self, pat: &'a Pat) -> Vec<&'a str> {
        let mut names = Vec::new();
        match pat {
            Pat::Array(a) => {
                for i in &a.elems {
                    if let Some(p) = i {
                        names.extend(self.pat_to_names(pat));
                    }
                }
            }
            Pat::Assign(a) => {
                names.extend(self.pat_to_names(&a.left));
            }
            Pat::Expr(e) => {
                unimplemented!()
            }
            Pat::Ident(i) => {
                names.push(&i.id.sym);
            }
            Pat::Invalid(i) => {}
            Pat::Object(o) => {
                for i in &o.props {
                    match i {
                        ObjectPatProp::Assign(a) => {
                            names.push(&a.key.sym);
                        }
                        ObjectPatProp::KeyValue(k) => {
                            names.extend_from_slice(&self.pat_to_names(&k.value));
                        }
                        ObjectPatProp::Rest(r) => {
                            names.extend_from_slice(&self.pat_to_names(&r.arg))
                        }
                    }
                }
            }
            Pat::Rest(r) => {
                names.extend_from_slice(&self.pat_to_names(&r.arg));
            }
        };
        names
    }

    fn finish(&mut self) -> Result<FuncID, Error> {
        self.bytecode
            .push(OpCode::LoadUndefined { result: self.r1 });
        self.bytecode.push(OpCode::Return { value: self.r1 });
        self.bytecode.extend(self.ctx.need_done());

        let codes = crate::bytecodes::optimize::optimize(self.bytecode.clone());

        let clousure = if !self.is_async && !self.is_generator {
            Some(crate::interpreter::clousure::Clousure::create(&codes))
        } else {
            None
        };

        let id = self.runtime.new_function(Arc::new(JSFunction {
            is_async: self.is_async,
            is_generator: self.is_generator,
            args_len: self.args_len as _,
            var_count: self.ctx.max_stack_offset(),
            call_count: 0,
            capture_stack_size: self.ctx.capture_len(),
            bytecodes: Arc::new(codes),

            largest_stack_offset: self.ctx.max_stack_offset() as u32,

            baseline_clousure: clousure,
            baseline_jit: None,
        }));
        self.ctx.close_context();
        Ok(id)
    }
}
