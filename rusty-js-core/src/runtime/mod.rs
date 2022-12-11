use std::alloc::Layout;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Weak;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use parking_lot::RwLock;

//mod runtime_context;
//mod async_executor;
mod bigint_allocator;
mod finalize_registry;
mod gc;
mod import_resolver;
mod object_allocater;
mod profiler;
mod string_allocator;

use import_resolver::ImportAssertion;
use import_resolver::ImportResolver;

use finalize_registry::FinalizeRegistry;

pub use gc::GcFlag;

//pub use async_executor::*;
pub use profiler::Profiler;

use crate::bultins;
use crate::bultins::class::JSClass;
use crate::bultins::function::{JSContext, JSFunction};
use crate::bultins::object::{JObject, JObjectInner};
use crate::bultins::strings::JSString;
use crate::bultins::JSBigInt;
use crate::bytecodes::function_builder_context::FunctionBuilderContext;
use crate::types::JValue;
use crate::utils::string_interner::StringInterner;
use crate::utils::string_interner::NAMES;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FuncID(pub(crate) u32);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClassID(pub(crate) u32);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConstID(pub(crate) u32);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegexID(pub(crate) u32);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct StringID(pub(crate) u32);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TemplateID(pub(crate) u32);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleId(u32);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsyncId(usize);

/// a Variable declared on the global context
#[derive(Debug, Clone, Copy, Hash)]
pub(crate) enum Variable {
    /// var declare
    Var(JValue),
    /// let declare
    Let(JValue),
    /// const declare
    Const(JValue),
    // todo: var on stack
    /// a variable imported from a module
    Import {
        /// id of the import module
        id: ModuleId,
        /// the variable name
        original_name: u32,
    },
    /// a default export of a module
    ImportDefault(ModuleId),
    // a module object
    ImportModule {
        id: ModuleId,
    },
}

enum ImportVariable {
    /// a variable imported from a module
    Import {
        /// id of the import module
        id: ModuleId,
        /// the variable name
        original_name: u32,
    },
    /// a default export of a module
    ImportDefault(ModuleId),
    // a module object
    ImportModule {
        id: ModuleId,
    },
}

/// a variable exported by the parent module
enum ExportVariable {
    Var(u32),
    Let(u32),
    Const(u32),
    /// an export imported from somewhere wlse
    Import {
        /// id of the module
        id: ModuleId,
        /// the variable name
        original_name: u32,
    },
    /// export the default of another module
    ImportDefault {
        id: ModuleId,
    },
    /// import * from module
    ///
    /// import the namespace of a module
    ImportModule {
        id: ModuleId,
    },
}

pub(crate) struct Module {
    name: String,
    exports: HashMap<u32, ExportVariable>,
    default_export: JValue,
}

thread_local! {
    pub static JS_RUNTIME: Option<Arc<Runtime>> = None;
}

pub const DEFAULT_STACK_SIZE: usize = 4096 * 256 / std::mem::size_of::<JValue>();
const NUM_WORKER_THREAD: usize = 3;

pub struct Runtime {
    weak_ref:Option<Weak<Self>>,

    strict_mode: bool,

    parser_globals: swc_common::Globals,
    function_builder_context:FunctionBuilderContext,

    obj_field_names: RwLock<StringInterner>,
    dynamic_var_names: RwLock<StringInterner>,

    dynamic_var_suffix: Option<String>,

    constants: Vec<JValue>,

    strings: StringInterner,

    /// the stack is used for storing variables and call frames,
    pub(crate) stack: Box<[JValue; DEFAULT_STACK_SIZE]>,
    /// the operational stack is used for operations
    /// such as calling getters and setters,
    /// all values are disposed after one call
    pub(crate) operational_stack: Box<[JValue; DEFAULT_STACK_SIZE]>,

    object_allocator: object_allocater::ObjectAllocator,
    string_allocator: string_allocator::StringAllocator,
    bigint_allocator: bigint_allocator::BigIntAllocator,

    /// variable that belongs to a module are formatted with { name @ moduleID }
    pub(crate) variables: HashMap<u32, Variable>,

    functions: Vec<Option<Arc<JSFunction>>>,
    classes: Vec<Option<Arc<JSClass>>>,
    regexs: Vec<Box<bultins::regex::RegExp>>,
    templates: Vec<bultins::strings::Template>,

    pub global: JValue,
    pub global_this: JObject,
    pub prototypes: bultins::BuiltinPrototypes,

    pub new_target: JValue,
    pub(crate) import_meta: JValue,

    import_resolver: Option<Arc<RwLock<dyn ImportResolver>>>,
    modules: Vec<Module>,

    /// temporary map to store exports
    exported_variables: HashMap<String, ExportVariable>,

    futures: Vec<Pin<Box<dyn Future<Output = Result<JValue, JValue>>>>>,
    async_stacks: Vec<&'static [JValue]>,

    pub(crate) finalize_registry: FinalizeRegistry,

    /// a reference counted user owned value
    user_owned: HashMap<JValue, AtomicUsize>,

    pub(crate) worker_task_sender: crossbeam_channel::Sender<Box<dyn FnOnce() + Sync + Send>>,

    pub(crate) baseline_context: inkwell::context::Context,
    pub(crate) baseline_module: inkwell::module::Module<'static>,
    pub(crate) baseline_engine: Option<inkwell::execution_engine::ExecutionEngine<'static>>,
}

unsafe impl Sync for Runtime {}
unsafe impl Send for Runtime {}

impl Runtime {
    pub fn new() -> Arc<Self> {
        // allocate without writing on stack to prevent stackoverflow
        let stack = unsafe {
            let ptr = std::alloc::alloc_zeroed(Layout::new::<[JValue; DEFAULT_STACK_SIZE]>())
                as *mut [JValue; DEFAULT_STACK_SIZE];
            Box::from_raw(ptr)
        };
        let op_stack = unsafe {
            let ptr = std::alloc::alloc_zeroed(Layout::new::<[JValue; DEFAULT_STACK_SIZE]>())
                as *mut [JValue; DEFAULT_STACK_SIZE];
            Box::from_raw(ptr)
        };

        let (worker_send, worker_recv) = crossbeam_channel::unbounded();

        static mut LLVM_TARGET_INIT: bool = false;

        if !unsafe { LLVM_TARGET_INIT } {
            inkwell::targets::Target::initialize_native(&Default::default())
                .expect("failed to initialize llvm");
            unsafe { LLVM_TARGET_INIT = true };
        }

        let baseline_context = inkwell::context::Context::create();
        let baseline_module: inkwell::module::Module =
            unsafe { std::mem::transmute(baseline_context.create_module("baseline")) };
        let baseline_engine = unsafe {
            std::mem::transmute(
                baseline_module
                    .create_jit_execution_engine(inkwell::OptimizationLevel::None)
                    .unwrap(),
            )
        };

        let runtime = Arc::new(Self {
            weak_ref:None,

            strict_mode: false,

            parser_globals: swc_common::Globals::new(),
            function_builder_context: FunctionBuilderContext::new(),

            obj_field_names: RwLock::new(crate::utils::string_interner::INTERNER.clone()),
            dynamic_var_names: RwLock::new(crate::utils::string_interner::INTERNER.clone()),
            dynamic_var_suffix: None,

            stack: stack,
            operational_stack: op_stack,

            object_allocator: Default::default(),
            string_allocator: Default::default(),
            bigint_allocator: Default::default(),

            constants: vec![],
            regexs: vec![],
            strings: StringInterner::new(),
            variables: Default::default(),

            functions: vec![],
            classes: vec![],
            templates: vec![],
            global: JValue::UNDEFINED,
            global_this: unsafe {
                #[allow(invalid_value)]
                std::mem::MaybeUninit::uninit().assume_init()
            },
            prototypes: bultins::BuiltinPrototypes::zero(),

            new_target: JValue::UNDEFINED,
            import_meta: JValue::UNDEFINED,

            import_resolver: None,
            modules: Default::default(),
            exported_variables: Default::default(),

            futures: Vec::new(),
            async_stacks: Vec::new(),

            finalize_registry: Default::default(),
            user_owned: Default::default(),

            worker_task_sender: worker_send,

            baseline_context: baseline_context,
            baseline_engine: baseline_engine,
            baseline_module: baseline_module,
        });

        runtime.to_mut().weak_ref = Some(Arc::downgrade(&runtime));

        // allocate global object
        runtime.to_mut().global_this = runtime.create_object();

        runtime.to_mut().global = runtime.global_this.into();

        runtime.to_mut().import_meta = runtime.create_object().into();

        // allocate builtin prototypes
        runtime.to_mut().prototypes.init(runtime.to_mut());

        let weak_rt = Arc::downgrade(&runtime);

        for _ in 0..NUM_WORKER_THREAD {
            let recv = worker_recv.clone();

            let weak_rt = weak_rt.clone();

            std::thread::spawn(move || loop {

                match recv.recv() {
                    Ok(task) => {
                    
                        let rt = match weak_rt.upgrade(){
                            Some(rt) => rt,
                            None => return
                        };

                        rt.attach();
                        (task)();
                        Runtime::deattach();
                    }
                    Err(_e) => {
                        break;
                    }
                };
            });
        }

        runtime.clone().attach();
        // enable ecma features
        // todo: let user decide to enable or not
        crate::ecma::enable(&runtime);

        Runtime::deattach();

        runtime
    }

    #[inline]
    pub fn is_attached() -> bool {
        JS_RUNTIME.with(|runtime| runtime.is_some())
    }

    #[inline]
    pub fn attach(self: Arc<Self>) {
        JS_RUNTIME.with(|runtime| unsafe {
            *(runtime as *const Option<Arc<Runtime>> as *mut Option<Arc<Runtime>>) = Some(self);
        })
    }

    #[inline]
    pub fn deattach() {
        JS_RUNTIME.with(|runtime| unsafe {
            *(runtime as *const Option<Arc<Runtime>> as *mut Option<Arc<Runtime>>) = None;
        })
    }

    #[inline]
    pub fn current() -> Arc<Runtime> {
        JS_RUNTIME.with(|runtime| {
            if let Some(r) = runtime {
                r.clone()
            } else {
                panic!(
                    "js runtime not attached on thread {:#?}.",
                    std::thread::current().id()
                )
            }
        })
    }

    #[inline]
    pub fn execute(
        self: Arc<Self>,
        filename: &str,
        script: &str,
    ) -> Result<JValue, crate::error::Error> {
        use swc_common::{FileName, SourceFile};

        let src = SourceFile::new(
            FileName::Custom(filename.to_string()),
            false,
            FileName::Anon,
            script.to_string(),
            swc_common::BytePos(1),
        );
        let mut v = Vec::new();
        let re = swc_ecmascript::parser::parse_file_as_module(
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
                allow_return_outside_function: false,
            }),
            swc_ecmascript::ast::EsVersion::Es2022,
            None,
            &mut v,
        );

        let script = re.unwrap();
        self.run_module(script)
    }

    /// optimize the sourse code before executing
    ///
    /// optimized code does not guarantee variable declarations,
    /// smaller programs are recommanded to use execute()
    /// as optimization increases startup time
    pub fn execute_optimized(
        self: Arc<Self>,
        filename: &str,
        script: &str,
    ) -> Result<JValue, crate::error::Error> {
        use swc_common::{FileName, SourceFile};
        use swc_ecmascript::visit::Fold;

        let src = SourceFile::new(
            FileName::Custom(filename.to_string()),
            false,
            FileName::Anon,
            script.to_string(),
            swc_common::BytePos(1),
        );
        let mut v = Vec::new();
        let re = swc_ecmascript::parser::parse_file_as_module(
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
                allow_return_outside_function: false,
            }),
            swc_ecmascript::ast::EsVersion::Es2022,
            None,
            &mut v,
        );

        let module = re.unwrap();

        let module = swc_common::GLOBALS.set(&self.parser_globals, || {
            let unresolved_mark = swc_common::Mark::new();
            let top_level_mark = swc_common::Mark::new();

            let mut r = swc_common::chain!(
                swc_ecmascript::transforms::resolver(unresolved_mark, top_level_mark, false),
                swc_ecmascript::transforms::optimization::simplifier(
                    unresolved_mark,
                    swc_ecmascript::transforms::optimization::simplify::Config {
                        dce: swc_ecmascript::transforms::optimization::simplify::dce::Config {
                            module_mark: None,
                            top_level: true,
                            top_retain: Default::default(),
                        },
                        inlining: Default::default(),
                        expr: Default::default(),
                    },
                )
            );

            r.fold_module(module)
        });
        self.run_module(module)
    }

    fn run_module(
        self: Arc<Self>,
        module: swc_ecmascript::ast::Module,
    ) -> Result<JValue, crate::error::Error> {
        let mut builder = crate::bytecodes::bytecode_builder::FunctionBuilder::new_with_context(self.clone(), self.function_builder_context.clone(), false, false, 0);

        for i in module.body {
            match i {
                swc_ecmascript::ast::ModuleItem::Stmt(s) => {
                    builder.translate_statement(None, &s)?;
                }

                swc_ecmascript::ast::ModuleItem::ModuleDecl(m) => {
                    match m {
                        swc_ecmascript::ast::ModuleDecl::ExportAll(e) => {
                            let id = self
                                .clone()
                                .import(&e.src.value, e.asserts.as_ref().map(|v| v.as_ref()))?;

                            for (name, _v) in &self.modules[id.0 as usize].exports {
                                // get the raw name without module suffix
                                // this is just incase a programing error happens, not requied
                                // todo: remove split
                                let n = self.get_dynamic_var_name(*name).split('@').next().unwrap();

                                self.to_mut().exported_variables.insert(
                                    n.to_string(),
                                    ExportVariable::Import {
                                        // the id of module
                                        id: id,
                                        original_name: *name,
                                    },
                                );
                            }
                        }
                        swc_ecmascript::ast::ModuleDecl::ExportDecl(e) => {
                            match &e.decl {
                                swc_ecmascript::ast::Decl::Class(c) => {
                                    // a name with @moduleID suffix
                                    let id = self.regester_dynamic_var_name(&c.ident.sym);

                                    self.to_mut()
                                        .exported_variables
                                        .insert(c.ident.sym.to_string(), ExportVariable::Let(id));
                                }
                                swc_ecmascript::ast::Decl::Fn(f) => {
                                    // a name with @moduleID suffix
                                    let id = self.regester_dynamic_var_name(&f.ident.sym);

                                    self.to_mut()
                                        .exported_variables
                                        .insert(f.ident.sym.to_string(), ExportVariable::Let(id));
                                }
                                swc_ecmascript::ast::Decl::TsEnum(t) => {
                                    // a name with @moduleID suffix
                                    let id = self.regester_dynamic_var_name(&t.id.sym);

                                    self.to_mut()
                                        .exported_variables
                                        .insert(t.id.sym.to_string(), ExportVariable::Let(id));
                                }
                                swc_ecmascript::ast::Decl::TsInterface(t) => {
                                    // a name with @moduleID suffix
                                    let id = self.regester_dynamic_var_name(&t.id.sym);

                                    self.to_mut()
                                        .exported_variables
                                        .insert(t.id.sym.to_string(), ExportVariable::Let(id));
                                }
                                swc_ecmascript::ast::Decl::TsTypeAlias(t) => {
                                    // a name with @moduleID suffix
                                    let id = self.regester_dynamic_var_name(&t.id.sym);

                                    self.to_mut()
                                        .exported_variables
                                        .insert(t.id.sym.to_string(), ExportVariable::Let(id));
                                }
                                swc_ecmascript::ast::Decl::TsModule(t) => {
                                    let s = match &t.id {
                                        swc_ecmascript::ast::TsModuleName::Ident(i) => {
                                            i.sym.to_string()
                                        }
                                        swc_ecmascript::ast::TsModuleName::Str(s) => {
                                            s.value.to_string()
                                        }
                                    };
                                    // a name with @moduleID suffix
                                    let id = self.regester_dynamic_var_name(&s);

                                    self.to_mut()
                                        .exported_variables
                                        .insert(s, ExportVariable::Let(id));
                                }

                                swc_ecmascript::ast::Decl::Var(v) => {
                                    for i in &v.decls {
                                        let names = builder.pat_to_names(&i.name);
                                        names.iter().for_each(|n| {
                                            // a name with @moduleID suffix
                                            let id = self.regester_dynamic_var_name(n);

                                            match v.kind {
                                                swc_ecmascript::ast::VarDeclKind::Const => {
                                                    self.to_mut().exported_variables.insert(
                                                        n.to_string(),
                                                        ExportVariable::Const(id),
                                                    );
                                                }
                                                swc_ecmascript::ast::VarDeclKind::Let => {
                                                    self.to_mut().exported_variables.insert(
                                                        n.to_string(),
                                                        ExportVariable::Let(id),
                                                    );
                                                }
                                                swc_ecmascript::ast::VarDeclKind::Var => {
                                                    self.to_mut().exported_variables.insert(
                                                        n.to_string(),
                                                        ExportVariable::Var(id),
                                                    );
                                                }
                                            }
                                        });
                                    }
                                }
                            };

                            builder.translate_statement(
                                None,
                                &swc_ecmascript::ast::Stmt::Decl(e.decl),
                            )?;
                        }
                        swc_ecmascript::ast::ModuleDecl::ExportDefaultDecl(d) => {
                            match d.decl {
                                swc_ecmascript::ast::DefaultDecl::Class(c) => {
                                    builder.translate_statement(
                                        None,
                                        &swc_ecmascript::ast::Stmt::Decl(
                                            swc_ecmascript::ast::Decl::Class(
                                                swc_ecmascript::ast::ClassDecl {
                                                    // workaround: use a builtin variable name to store the default export
                                                    ident: swc_ecmascript::ast::Ident::new(
                                                        swc_atoms::JsWord::from("#default export"),
                                                        Default::default(),
                                                    ),
                                                    declare: false,
                                                    class: c.class,
                                                },
                                            ),
                                        ),
                                    )?;
                                }
                                swc_ecmascript::ast::DefaultDecl::Fn(f) => {
                                    builder.translate_statement(
                                        None,
                                        &swc_ecmascript::ast::Stmt::Decl(
                                            swc_ecmascript::ast::Decl::Fn(
                                                swc_ecmascript::ast::FnDecl {
                                                    // workaround: use a builtin variable name to store the default export
                                                    ident: swc_ecmascript::ast::Ident::new(
                                                        swc_atoms::JsWord::from("#default export"),
                                                        Default::default(),
                                                    ),
                                                    declare: false,
                                                    function: f.function,
                                                },
                                            ),
                                        ),
                                    )?;
                                }
                                swc_ecmascript::ast::DefaultDecl::TsInterfaceDecl(_i) => {
                                    todo!()
                                }
                            }
                        }
                        swc_ecmascript::ast::ModuleDecl::ExportDefaultExpr(e) => {
                            builder.translate_statement(
                                None,
                                &swc_ecmascript::ast::Stmt::Decl(swc_ecmascript::ast::Decl::Var(
                                    Box::new(swc_ecmascript::ast::VarDecl {
                                        span: Default::default(),
                                        kind: swc_ecmascript::ast::VarDeclKind::Var,
                                        declare: false,
                                        decls: vec![swc_ecmascript::ast::VarDeclarator {
                                            span: Default::default(),
                                            name: swc_ecmascript::ast::Pat::Ident(
                                                swc_ecmascript::ast::BindingIdent {
                                                    id: swc_ecmascript::ast::Ident::new(
                                                        swc_atoms::JsWord::from("#default export"),
                                                        Default::default(),
                                                    ),
                                                    type_ann: None,
                                                },
                                            ),
                                            init: Some(e.expr),
                                            definite: false,
                                        }],
                                    }),
                                )),
                            )?;
                        }
                        swc_ecmascript::ast::ModuleDecl::ExportNamed(n) => {
                            if let Some(name) = &n.src {
                                let module_id = self
                                    .clone()
                                    .import(&name.value, n.asserts.as_ref().map(|v| v.as_ref()))?;

                                for i in &n.specifiers {
                                    match i {
                                        swc_ecmascript::ast::ExportSpecifier::Default(d) => {
                                            self.to_mut().exported_variables.insert(
                                                d.exported.sym.to_string(),
                                                ExportVariable::ImportDefault { id: module_id },
                                            );
                                        }
                                        swc_ecmascript::ast::ExportSpecifier::Named(n) => {
                                            let mut name = match &n.orig {
                                                swc_ecmascript::ast::ModuleExportName::Ident(i) => {
                                                    i.sym.to_string()
                                                }
                                                swc_ecmascript::ast::ModuleExportName::Str(s) => {
                                                    s.value.to_string()
                                                }
                                            };

                                            // variable name in Module without suffix
                                            let old = self.to_mut().dynamic_var_suffix.take();
                                            let id = self.regester_dynamic_var_name(&name);
                                            self.to_mut().dynamic_var_suffix = old;

                                            if let Some(n) = &n.exported {
                                                name = match n{
                                                    swc_ecmascript::ast::ModuleExportName::Ident(i) => {
                                                        i.sym.to_string()
                                                    },
                                                    swc_ecmascript::ast::ModuleExportName::Str(s) => {
                                                        s.value.to_string()
                                                    }
                                                };
                                            };

                                            self.to_mut().exported_variables.insert(
                                                name,
                                                ExportVariable::Import {
                                                    id: module_id,
                                                    original_name: id,
                                                },
                                            );
                                        }
                                        swc_ecmascript::ast::ExportSpecifier::Namespace(n) => {
                                            // export * as foo

                                            let name = match &n.name {
                                                swc_ecmascript::ast::ModuleExportName::Ident(i) => {
                                                    i.sym.as_ref()
                                                }
                                                swc_ecmascript::ast::ModuleExportName::Str(s) => {
                                                    s.value.as_ref()
                                                }
                                            };
                                            self.to_mut().exported_variables.insert(
                                                name.to_string(),
                                                ExportVariable::ImportModule { id: module_id },
                                            );
                                        }
                                    }
                                }
                            } else {
                                for i in &n.specifiers {
                                    match i {
                                        swc_ecmascript::ast::ExportSpecifier::Default(_d) => {
                                            unimplemented!("export default without module source.")
                                        }
                                        swc_ecmascript::ast::ExportSpecifier::Named(n) => {
                                            let mut name = match &n.orig {
                                                swc_ecmascript::ast::ModuleExportName::Ident(i) => {
                                                    i.sym.to_string()
                                                }
                                                swc_ecmascript::ast::ModuleExportName::Str(s) => {
                                                    s.value.to_string()
                                                }
                                            };

                                            // variable name with suffix
                                            let id = self.regester_dynamic_var_name(&name);

                                            if let Some(n) = &n.exported {
                                                name = match n{
                                                    swc_ecmascript::ast::ModuleExportName::Ident(i) => {
                                                        i.sym.to_string()
                                                    },
                                                    swc_ecmascript::ast::ModuleExportName::Str(s) => {
                                                        s.value.to_string()
                                                    }
                                                };
                                            };

                                            // export variable
                                            self.to_mut()
                                                .exported_variables
                                                .insert(name, ExportVariable::Var(id));
                                        }
                                        swc_ecmascript::ast::ExportSpecifier::Namespace(_n) => {
                                            // no module is imported
                                            unimplemented!(
                                                "export module namespace without module source."
                                            )
                                        }
                                    }
                                }
                            };
                        }
                        swc_ecmascript::ast::ModuleDecl::Import(i) => {
                            let module_id = self
                                .clone()
                                .import(&i.src.value, i.asserts.as_ref().map(|v| v.as_ref()))?;

                            for i in &i.specifiers {
                                match i {
                                    swc_ecmascript::ast::ImportSpecifier::Default(d) => {
                                        let key = self.regester_dynamic_var_name(&d.local.sym);

                                        self.to_mut()
                                            .variables
                                            .insert(key, Variable::ImportDefault(module_id));
                                    }
                                    swc_ecmascript::ast::ImportSpecifier::Named(n) => {
                                        let import_name = if let Some(n) = &n.imported {
                                            match n {
                                                swc_ecmascript::ast::ModuleExportName::Ident(i) => {
                                                    i.sym.as_ref()
                                                }
                                                swc_ecmascript::ast::ModuleExportName::Str(s) => {
                                                    s.value.as_ref()
                                                }
                                            }
                                        } else {
                                            n.local.sym.as_ref()
                                        };

                                        let key = self
                                            .to_mut()
                                            .dynamic_var_names
                                            .write()
                                            .get_or_intern(import_name)
                                            as u32;

                                        if !self.modules[module_id.0 as usize]
                                            .exports
                                            .contains_key(&key)
                                        {
                                            return Err(crate::error::Error::ImportError(format!("The requested module '{}' does not provide an export named '{}'", self.modules[module_id.0 as usize].name.as_str(), import_name)));
                                        };

                                        let key = self.regester_dynamic_var_name(&n.local.sym);

                                        self.to_mut().variables.insert(
                                            key,
                                            Variable::Import {
                                                id: module_id,
                                                original_name: key,
                                            },
                                        );
                                    }
                                    swc_ecmascript::ast::ImportSpecifier::Namespace(n) => {
                                        let key = self.regester_dynamic_var_name(&n.local.sym);

                                        self.to_mut()
                                            .variables
                                            .insert(key, Variable::ImportModule { id: module_id });
                                    }
                                };
                            }
                        }
                        swc_ecmascript::ast::ModuleDecl::TsExportAssignment(_t) => {
                            todo!()
                        }
                        swc_ecmascript::ast::ModuleDecl::TsImportEquals(_i) => {
                            todo!()
                        }
                        swc_ecmascript::ast::ModuleDecl::TsNamespaceExport(_n) => {
                            todo!()
                        }
                    }
                }
            };
        }

        let bytecodes = builder.bytecode;
        let op_stack = builder.ctx.max_stack_offset() as usize;

        let bytecodes = crate::baseline::optimize(bytecodes);

        let mut cl = crate::interpreter::clousure::Clousure::create(&bytecodes);
        let re = cl.run(
            &self,
            self.to_mut().stack.as_mut_slice(),
            &mut self.to_mut().stack.as_mut_slice()[op_stack..],
            None,
            None,
            self.global,
            &[],
        );

        /*
        let mut intpr =
            crate::interpreter::Interpreter::global(&self, self.to_mut().stack.as_mut_slice(), op_stack);

        let re = intpr.run(self.global, &[], &bytecodes);
        */

        // finish all the async tasks
        self.finish_async();

        match re {
            Ok(v) => Ok(v),
            Err(e) => return Err(crate::error::Error::Value(e)),
        }
    }

    #[inline]
    fn import(
        self: Arc<Self>,
        name: &str,
        asserts: Option<&swc_ecmascript::ast::ObjectLit>,
    ) -> Result<ModuleId, crate::error::Error> {
        if self.import_resolver.is_none() {
            return Err(crate::error::Error::ImportError(
                "cannot import module: import resolver is not defined.".to_owned(),
            ));
        }

        let re = self
            .import_resolver
            .as_ref()
            .unwrap()
            .write()
            .import(name, ImportAssertion::from(asserts));

        let script = match re {
            Ok(s) => s,
            Err(e) => return Err(crate::error::Error::ImportError(e)),
        };

        let old_strict = self.strict_mode;
        self.to_mut().strict_mode = true;

        let old_global = self.global;
        self.to_mut().global = JValue::UNDEFINED;

        let mut oldsuffix = Some(format!("@{}", self.modules.len()));
        std::mem::swap(&mut oldsuffix, &mut self.to_mut().dynamic_var_suffix);

        // variables in module doesn't matter anyway, so we optimize it out
        self.clone().execute_optimized(name, &script)?;

        self.to_mut().global = old_global;
        self.to_mut().strict_mode = old_strict;

        // reverse to the oldsuffix
        self.to_mut().dynamic_var_suffix = oldsuffix;

        let mut exports = HashMap::default();

        let mut e = HashMap::default();
        std::mem::swap(&mut e, &mut self.to_mut().exported_variables);

        for (name, v) in e {
            // a name without @moduleID suffix
            let key = self.to_mut().dynamic_var_names.write().get_or_intern(name) as u32;
            exports.insert(key, v);
        }

        let default_export_key = NAMES["#default export"];

        self.to_mut().modules.push(Module {
            name: name.to_owned(),
            exports: exports,
            default_export: self.get_variable(default_export_key.0).unwrap(),
        });

        return Ok(ModuleId(self.modules.len() as u32 - 1));
    }

    /// a lazy workaround to mutate runtime
    #[inline]
    pub(crate) fn to_mut(&self) -> &mut Self {
        unsafe { &mut *(self as *const Self as *mut Self) }
    }

    /// allocate object from the allocater
    #[inline]
    pub(crate) fn allocate_obj(&self) -> &'static mut JObjectInner {
        let inner = unsafe { self.to_mut().object_allocator.allocate(self.weak_ref.as_ref().unwrap().upgrade().unwrap()) };
        inner.__proto__ = None;
        inner
    }

    pub fn create_object(&self) -> JObject {
        let inner = self.allocate_obj();
        inner.__proto__ = Some(self.prototypes.object);
        return inner.into()
    }

    #[inline]
    pub(crate) fn allocate_string(&self, s: &str) -> JSString {
        if s.len() == 0 {
            return JSString(std::ptr::null_mut());
        }
        self.to_mut().string_allocator.allocate(s)
    }

    pub fn allocate_bigint(&self) -> &'static mut JSBigInt {
        self.to_mut().bigint_allocator.alloc()
    }

    #[inline]
    pub(crate) fn regester_dynamic_var_name(&self, name: &str) -> u32 {
        if let Some(v) = &self.dynamic_var_suffix {
            self.to_mut()
                .dynamic_var_names
                .write()
                .get_or_intern(format!("{}{}", name, v)) as u32
        } else {
            self.to_mut().dynamic_var_names.write().get_or_intern(name) as u32
        }
    }

    pub(crate) fn get_dynamic_var_name<'a>(&'a self, id: u32) -> &'a str {
        let guard = self.dynamic_var_names.read();
        let s = guard.resolve(id as usize).unwrap();
        unsafe { std::mem::transmute_copy(&s) }
    }

    #[inline]
    pub fn register_field_name(&self, name: &str) -> u32 {
        self.obj_field_names.write().get_or_intern(name) as u32
    }

    #[inline]
    pub fn get_field_name<'a>(&'a self, id: u32) -> &'a str {
        let guard = self.obj_field_names.read();
        let s = guard.resolve(id as usize).unwrap();

        unsafe { std::mem::transmute_copy(&s) }
    }

    #[inline]
    pub(crate) fn register_template(&self, tpl: bultins::strings::Template) -> TemplateID {
        let id = self.templates.len();
        self.to_mut().templates.push(tpl);
        TemplateID(id as u32)
    }

    #[inline]
    pub(crate) fn get_template(&self, id: TemplateID) -> &bultins::strings::Template {
        self.templates.get(id.0 as usize).unwrap()
    }

    ///////////////////////////////////////////////////////////////////
    //          async
    //////////////////////////////////////////////////////////////////

    pub fn run_async<F>(&mut self, future: F) -> bultins::promise::Promise
    where
        F: Future<Output = Result<JValue, JValue>> + 'static,
    {
        self.futures.push(Box::pin(future));
        bultins::promise::Promise::Pending {
            id: AsyncId(self.futures.len() - 1),
        }
    }

    pub fn get_future<'a>(
        &'a mut self,
        id: AsyncId,
    ) -> Pin<&'a mut dyn Future<Output = Result<JValue, JValue>>> {
        self.futures.get_mut(id.0 as usize).unwrap().as_mut()
    }

    #[inline]
    pub fn poll_async(&mut self, id: AsyncId) -> std::task::Poll<Result<JValue, JValue>> {
        let f = self.get_future(id);
        let handle = tokio::runtime::Handle::current();
        let p = handle.block_on(async { futures::poll!(f) });
        return p;
    }

    #[inline]
    pub fn finish_async(self: Arc<Self>) {
        if self.futures.len() > 0 {
            self.worker_task_sender
                .clone()
                .send(Box::new(move || {
                    self.clone().attach();
                    futures::executor::block_on(async {
                        let mut i = 0;
                        for f in &mut self.to_mut().futures {
                            let p = futures::poll!(f.as_mut());
                            if p.is_ready() {
                                i += 1;
                            };
                            if i >= self.futures.len() {
                                break;
                            }
                        }
                    })
                }))
                .expect("failed to spawn task on worker thread");
        }
    }

    #[inline]
    pub fn get_async_stack(&mut self, stack_size: usize) -> &'static mut [JValue] {
        let size = stack_size / JValue::SIZE;
        let data =
            unsafe { std::alloc::alloc(std::alloc::Layout::array::<u8>(stack_size).unwrap()) }
                as *mut _;
        self.async_stacks
            .push(unsafe { std::slice::from_raw_parts(data, size) });
        return unsafe { std::slice::from_raw_parts_mut(data, size) };
    }

    pub fn drop_async_stack(&self, stack: &mut [JValue]) {
        let re = self
            .async_stacks
            .binary_search_by(|s| stack.as_ptr().cmp(&s.as_ptr()));
        if let Ok(idx) = re {
            self.to_mut().async_stacks.remove(idx);
            unsafe {
                std::alloc::dealloc(
                    stack.as_mut_ptr() as *mut u8,
                    Layout::array::<JValue>(stack.len()).unwrap(),
                )
            };
        }
    }

    //////////////////////////////////////////////////////////////////
    //          GC
    //////////////////////////////////////////////////////////////////

    /// garbage collector must call this function to clean functions
    #[inline]
    pub(crate) fn clean_functions(&mut self) {
        for i in &mut self.functions {
            if let Some(a) = i {
                if Arc::strong_count(a) == 1 {
                    *i = None;
                }
            }
        }
        for i in &mut self.classes {
            if let Some(a) = i {
                if Arc::strong_count(a) == 1 {
                    *i = None;
                }
            }
        }
    }

    #[inline]
    pub(crate) fn new_function(&self, func: Arc<JSFunction>) -> FuncID {
        self.to_mut().functions.push(Some(func));
        return FuncID((self.functions.len() - 1) as u32);
    }

    #[inline]
    pub(crate) fn get_function(&self, id: FuncID) -> Option<Arc<JSFunction>> {
        self.functions[id.0 as usize].clone()
    }

    pub fn create_constructor<F>(&self, func: F, name: &str, prototype: JObject) -> JObject
    where
        F: Fn(JSContext, JValue, &[JValue]) -> Result<JValue, JValue> + 'static,
    {
        let f = Arc::new(RwLock::new(func));

        let inner = self.allocate_obj();
        inner.__proto__ = Some(self.prototypes.function);

        let obj = JObject { inner };
        obj.set_inner(bultins::object::JObjectValue::NativeFunction(f));
        obj.insert_property(
            NAMES["prototype"],
            prototype.into(),
            bultins::flag::PropFlag::NONE,
        );
        obj.insert_property(
            NAMES["length"],
            JValue::create_number(0.0),
            bultins::flag::PropFlag::CONFIGURABLE,
        );
        obj.insert_property(
            NAMES["name"],
            JValue::create_string(name.into()),
            bultins::flag::PropFlag::CONFIGURABLE,
        );
        JObject { inner: inner }
    }

    #[inline]
    pub fn create_native_function<F>(&self, func: F) -> JObject
    where
        F: Fn(JSContext, JValue, &[JValue]) -> Result<JValue, JValue> + 'static,
    {
        let f = Arc::new(RwLock::new(func));

        let prototype = self.create_object();
        let obj = self.create_object();

        obj.set_inner(bultins::object::JObjectValue::NativeFunction(f));
        obj.insert_property_builtin(NAMES["__proto__"], self.prototypes.function.into());
        obj.insert_property(
            NAMES["prototype"],
            prototype.into(),
            bultins::flag::PropFlag::NONE,
        );
        obj.insert_property(
            NAMES["length"],
            JValue::create_number(0.0),
            bultins::flag::PropFlag::CONFIGURABLE,
        );
        obj.insert_property(
            NAMES["name"],
            JValue::create_static_string(""),
            bultins::flag::PropFlag::CONFIGURABLE,
        );
        obj
    }

    ////////////////////////////////////////////////////////////////////
    //          Class
    ////////////////////////////////////////////////////////////////////

    #[inline]
    pub(crate) fn new_class(&self, name: String) -> ClassID {
        self.to_mut()
            .classes
            .push(Some(Arc::new(JSClass::new(name))));
        return ClassID((self.classes.len() - 1) as u32);
    }

    #[inline]
    pub(crate) fn get_class(&self, id: ClassID) -> Arc<JSClass> {
        self.classes.get(id.0 as usize).unwrap().clone().unwrap()
    }

    #[inline]
    pub(crate) fn bind_class_constructor(&self, class_id: ClassID, func_id: FuncID) {
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        c.to_mut().constructor = Some(self.get_function(func_id).unwrap());
    }

    #[inline]
    pub(crate) fn bind_class_method(&self, class_id: ClassID, func_name: &str, func_id: FuncID) {
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        c.to_mut()
            .methods
            .insert(name, self.get_function(func_id).unwrap());
    }

    #[inline]
    pub(crate) fn bind_class_getter(&self, class_id: ClassID, func_name: &str, func_id: FuncID) {
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();
        let f = self.get_function(func_id).unwrap();
        if let Some(gs) = c.get_setters.get_mut(&name) {
            gs.0 = Some(f);
        } else {
            c.get_setters.insert(name, (Some(f), None));
        }
    }

    #[inline]
    pub(crate) fn bind_class_setter(&self, class_id: ClassID, func_name: &str, func_id: FuncID) {
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();
        let f = self.get_function(func_id).unwrap();
        if let Some(gs) = c.get_setters.get_mut(&name) {
            gs.1 = Some(f);
        } else {
            c.get_setters.insert(name, (None, Some(f)));
        }
    }

    #[inline]
    pub(crate) fn bind_class_static_method(
        &self,
        class_id: ClassID,
        func_name: &str,
        func_id: FuncID,
    ) {
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        c.to_mut()
            .static_methods
            .insert(name, self.get_function(func_id).unwrap());
    }

    #[inline]
    pub(crate) fn bind_class_static_getter(
        &self,
        class_id: ClassID,
        func_name: &str,
        func_id: FuncID,
    ) {
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();
        let f = self.get_function(func_id).unwrap();
        if let Some(gs) = c.static_get_setters.get_mut(&name) {
            gs.0 = Some(f);
        } else {
            c.static_get_setters.insert(name, (Some(f), None));
        }
    }

    #[inline]
    pub(crate) fn bind_class_static_setter(
        &self,
        class_id: ClassID,
        func_name: &str,
        func_id: FuncID,
    ) {
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();
        let f = self.get_function(func_id).unwrap();
        if let Some(gs) = c.static_get_setters.get_mut(&name) {
            gs.1 = Some(f);
        } else {
            c.static_get_setters.insert(name, (None, Some(f)));
        }
    }

    #[inline]
    pub(crate) fn bind_class_prop(&self, class_id: ClassID, name: &str) -> u32 {
        let name = self.to_mut().register_field_name(name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();

        c.props.push(name);
        name
    }

    #[inline]
    pub(crate) fn bind_class_static_prop(&self, class_id: ClassID, name: &str) -> u32 {
        let name = self.to_mut().register_field_name(name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();

        c.static_props.push(name);
        name
    }

    pub fn default_constructor(&self) -> FuncID {
        FuncID(0)
    }

    ////////////////////////////////////////////////////////////////
    //         variables
    ////////////////////////////////////////////////////////////////

    #[inline]
    pub fn declare_variable(&self, name: &str, value: JValue) {
        let id = self.to_mut().regester_dynamic_var_name(name);
        self.declare_variable_static(id, value);
    }

    #[inline]
    pub fn declare_let_variable(&self, name: &str, value: JValue) {
        let id = self.to_mut().regester_dynamic_var_name(name);
        self.declare_let_variable_static(id, value)
    }

    #[inline]
    pub(crate) fn declare_variable_static(&self, id: u32, value: JValue) {
        self.to_mut().variables.insert(id, Variable::Var(value));
    }

    #[inline]
    pub(crate) fn declare_let_variable_static(&self, id: u32, value: JValue) {
        self.to_mut().variables.insert(id, Variable::Var(value));
    }

    pub fn declare_constant(&self, name: &str, value: JValue) {
        let id = self.to_mut().regester_dynamic_var_name(name);

        self.to_mut().variables.insert(id, Variable::Const(value));
    }

    #[inline]
    pub fn unamed_constant(&mut self, value: JValue) -> ConstID {
        self.constants.push(value);

        ConstID(self.constants.len() as u32 - 1)
    }

    #[inline]
    pub fn get_unamed_constant(&self, id: ConstID) -> JValue {
        *self.constants.get(id.0 as usize).unwrap()
    }

    #[inline]
    pub fn register_regex(&mut self, reg: &str, flags: &str) -> Result<RegexID, String> {
        match bultins::regex::RegExp::with_flags(reg, flags) {
            Ok(v) => {
                self.regexs.push(Box::new(v));
            }
            Err(e) => return Err(e.to_string()),
        }

        Ok(RegexID(self.regexs.len() as u32 - 1))
    }

    // runtime must be attached before calling this function
    pub unsafe fn get_regex_object(&self, id: RegexID) -> JObject {
        JObject::with_regex(self.get_regex(id))
    }

    pub(crate) fn get_regex(&self, id: RegexID) -> Box<bultins::regex::RegExp> {
        let s = self.regexs.get(id.0 as usize).unwrap();
        s.clone()
    }

    #[inline]
    pub fn register_string(&mut self, string: &str) -> StringID {
        StringID(self.strings.get_or_intern(string) as u32)
    }

    pub fn get_string(&self, id: StringID) -> &'static str {
        let r = self.strings.resolve(id.0 as usize).unwrap();
        unsafe { std::mem::transmute_copy(&r) }
    }

    pub fn global_this(&self) -> &JObject {
        &self.global_this
    }

    #[inline]
    pub fn get_variable(&self, key: u32) -> Result<JValue, JValue> {
        if let Some(v) = self.variables.get(&key) {
            let v = match v {
                Variable::Const(v) => *v,
                Variable::Let(v) => *v,
                Variable::Var(v) => *v,
                Variable::Import { id, original_name } => {
                    self.get_exported(*id, *original_name).unwrap()
                }
                Variable::ImportDefault(id) => self.modules[id.0 as usize].default_export,
                Variable::ImportModule { id } => self.get_module_objet(*id).into(),
            };
            return Ok(v);
        } else {
            let guard = self.dynamic_var_names.read();
            let key = guard.resolve(key as usize).unwrap();

            if self.strict_mode {
                return Err(JValue::from(crate::error::Error::ReferenceError(format!(
                    "{} is not defined",
                    key
                ))));
            };

            self.global_this.get_property(
                key,
                JSContext {
                    stack: (&self.stack[self.stack.len() / 2..]).as_ptr() as *mut JValue,
                    runtime: &self,
                },
            )
        }
    }

    pub(crate) fn get_exported(&self, module_id: ModuleId, key: u32) -> Option<JValue> {
        if let Some(m) = self.modules.get(module_id.0 as usize) {
            if let Some(v) = m.exports.get(&key) {
                match v {
                    ExportVariable::Const(c) => match self.get_variable(*c) {
                        Ok(v) => return Some(v),
                        Err(_e) => return None,
                    },
                    ExportVariable::Let(c) => match self.get_variable(*c) {
                        Ok(v) => return Some(v),
                        Err(_e) => return None,
                    },
                    ExportVariable::Var(c) => match self.get_variable(*c) {
                        Ok(v) => return Some(v),
                        Err(_e) => return None,
                    },
                    ExportVariable::Import { id, original_name } => {
                        return self.get_exported(*id, *original_name);
                    }
                    ExportVariable::ImportDefault { id } => {
                        return Some(self.get_exported_default(*id))
                    }
                    ExportVariable::ImportModule { id } => {
                        // create a Module Object
                        return Some(self.get_module_objet(*id).into());
                    }
                };
            };
        }
        return None;
    }

    pub(crate) fn get_exported_default(&self, module_id: ModuleId) -> JValue {
        self.modules[module_id.0 as usize].default_export
    }

    pub(crate) fn get_module_objet(&self, module_id: ModuleId) -> JObject {
        JObject::with_module(module_id)
    }

    #[inline]
    pub fn set_variable(&mut self, key: u32, value: JValue) -> Result<(), JValue> {
        if let Some(v) = self.variables.get_mut(&key) {
            match v {
                Variable::Const(_v) => {}
                Variable::Let(v) => *v = value,
                Variable::Var(v) => *v = value,
                _ => {}
            };
            Ok(())
        } else {
            let guard = self.dynamic_var_names.read();
            let key = guard.resolve(key as usize).unwrap();

            self.global_this.set_property(
                key,
                value,
                JSContext {
                    stack: self.operational_stack.as_mut_ptr(),
                    runtime: self,
                },
            )
        }
    }

    ///////////////////////////////////////////////////////////////
    //          GC
    //////////////////////////////////////////////////////////////

    #[inline]
    pub unsafe fn run_gc(self: Arc<Self>) {
        // scan root and stack
        self.finalize_registry.trace();

        self.constants.iter().for_each(|v| v.trace());
        self.global.trace();
        self.global_this.trace();
        self.prototypes.trace();
        self.new_target.trace();

        self.user_owned.keys().into_iter().for_each(|v| v.trace());

        for i in 0..self.variables.len() {
            match self.get_variable(i as u32) {
                Ok(v) => v.trace(),
                Err(e) => e.trace(),
            };
        }
        self.stack.iter().for_each(|v| v.trace());
        self.async_stacks
            .iter()
            .for_each(|v| v.iter().for_each(|v| v.trace()));

        self.modules.iter().for_each(|v| {
            v.default_export.trace();
        });

        self.to_mut().finalize_registry.garbage_collect(&self);

        let rt = self.clone();

        self.worker_task_sender
            .send(Box::new(move || {
                rt.to_mut().string_allocator.garbage_collect();
                rt.to_mut().object_allocator.garbage_collect();
                rt.to_mut().bigint_allocator.garbage_collect();
                rt.to_mut().clean_functions();
            }))
            .unwrap();
    }

    /// return the reference counter
    #[inline]
    pub fn user_own_value(&self, v: JValue) {
        if let Some(count) = self.to_mut().user_owned.get_mut(&v) {
            count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.to_mut().user_owned.insert(v, AtomicUsize::new(1));
        }
    }

    #[inline]
    pub fn user_drop_value(&self, v: JValue) {
        if let Some(count) = self.to_mut().user_owned.get_mut(&v) {
            let c = count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            if c <= 1 {
                self.to_mut().user_owned.remove(&v);
            }
        }
    }
}

impl Drop for Runtime{
    fn drop(&mut self) {
        //println!("runtime dropped")
        // the egine segfaults when drop, more debugging is needed
        unsafe{(&mut self.baseline_engine as *mut Option<_>).write(None)};
    }
}