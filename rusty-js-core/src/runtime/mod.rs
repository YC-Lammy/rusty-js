use std::alloc::Layout;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use string_interner::{DefaultBackend, DefaultSymbol, StringInterner, Symbol};

//mod runtime_context;
mod async_executor;
mod gc;
mod object_allocater;
mod profiler;
mod string_allocator;

pub use gc::GcFlag;

pub use async_executor::*;
pub use profiler::Profiler;

use crate::bultins;
use crate::bultins::class::JSClass;
use crate::bultins::function::{JSFuncContext, JSFunction};
use crate::bultins::object::{JObject, JObjectInner};
use crate::types::JValue;
use crate::utils::nohasher;

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

thread_local! {
    pub static JS_RUNTIME: Option<Arc<Runtime>> = None;
}

const DEFAULT_STACK_SIZE: usize = 1048576 / std::mem::size_of::<JValue>();

pub struct Runtime {
    obj_field_names: StringInterner,
    dynamic_var_names: StringInterner,

    constants: Vec<JValue>,

    strings: StringInterner<DefaultBackend<usize>>,

    pub(crate) stack: Box<[JValue; DEFAULT_STACK_SIZE]>,
    pub(crate) async_stack: Box<[JValue; DEFAULT_STACK_SIZE]>,
    aync_stack_offset: usize,

    object_allocator: object_allocater::ObjectAllocator,
    string_allocator: string_allocator::StringAllocator,

    variables: HashMap<u32, JValue, nohasher::NoHasherBuilder>,

    functions: Vec<Option<Arc<JSFunction>>>,
    classes: Vec<Option<Arc<JSClass>>>,
    regexs: Vec<(String, String)>,
    templates: Vec<bultins::strings::Template>,

    pub global: Option<JObject>,

    pub(crate) async_executor: async_executor::AsyncExecutor<JValue>,
    pub(crate) generator_executor: async_executor::AsyncExecutor<JValue>,

    user_owned: HashMap<JValue, AtomicUsize>,
}

unsafe impl Sync for Runtime {}
unsafe impl Send for Runtime {}

impl Runtime {
    pub fn new() -> Arc<Self> {
        // allocate without writing on stack to prevent stackoverflow
        let stack = unsafe {
            let ptr = std::alloc::alloc(Layout::new::<[JValue; DEFAULT_STACK_SIZE]>())
                as *mut [JValue; DEFAULT_STACK_SIZE];
            Box::from_raw(ptr)
        };

        let async_stack = unsafe {
            let ptr = std::alloc::alloc(Layout::new::<[JValue; DEFAULT_STACK_SIZE]>())
                as *mut [JValue; DEFAULT_STACK_SIZE];
            Box::from_raw(ptr)
        };
        let runtime = Arc::new(Self {
            obj_field_names: StringInterner::new(),
            dynamic_var_names: StringInterner::new(),

            stack: stack,
            async_stack,
            aync_stack_offset: 0,

            object_allocator: Default::default(),
            string_allocator: Default::default(),

            constants: vec![],
            regexs: vec![],
            strings: StringInterner::new(),
            variables: HashMap::default(),
            functions: vec![],
            classes: vec![],
            templates: vec![],
            global: None,
            async_executor: async_executor::AsyncExecutor::new(),
            generator_executor: async_executor::AsyncExecutor::new(),

            user_owned: Default::default(),
        });

        runtime.to_mut().global = Some(JObject {
            inner: runtime.allocate_obj(),
        });

        runtime
    }

    pub fn attach(self: Arc<Self>) {
        JS_RUNTIME.with(|runtime| unsafe {
            *(runtime as *const Option<Arc<Runtime>> as *mut Option<Arc<Runtime>>) = Some(self);
        })
    }

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
        let re = swc_ecmascript::parser::parse_file_as_script(
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
        let mut builder = crate::baseline::bytecode_builder::FunctionBuilder::new(self.clone());

        for i in &script.body {
            builder.translate_statement(None, i)?;
        }

        let bytecodes = builder.bytecode;

        let mut intpr =
            crate::interpreter::Interpreter::global(&self, self.to_mut().stack.as_mut_slice());
        let re = intpr.run(JValue::Object(self.global.unwrap()), &[], &bytecodes);

        self.to_mut().finish_async();

        match re {
            Ok(v) => Ok(v),
            Err(e) => return Err(crate::error::Error::Value(e)),
        }
    }

    #[inline]
    pub(crate) fn to_mut(&self) -> &mut Self {
        unsafe { (self as *const Self as *mut Self).as_mut().unwrap() }
    }

    #[inline]
    pub(crate) fn allocate_obj(&self) -> &'static mut JObjectInner {
        unsafe { self.to_mut().object_allocator.allocate() }
    }

    #[inline]
    pub(crate) fn allocate_string(&self, size: usize) -> *mut u8 {
        self.to_mut().string_allocator.allocate(size) as *mut u8
    }

    #[inline]
    pub(crate) fn regester_dynamic_var_name(&self, name: &str) -> u32 {
        self.to_mut()
            .dynamic_var_names
            .get_or_intern(name)
            .to_usize() as u32
    }

    #[inline]
    pub fn register_field_name(&self, name: &str) -> u32 {
        self.to_mut().obj_field_names.get_or_intern(name).to_usize() as u32
    }

    #[inline]
    pub fn get_field_name(&self, id: u32) -> &str {
        self.obj_field_names
            .resolve(DefaultSymbol::try_from_usize(id as usize).unwrap())
            .unwrap()
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

    #[inline]
    pub fn call_async<F>(&mut self, f: F) -> bultins::promise::Promise
    where
        F: Fn() -> Result<JValue, JValue> + 'static,
    {
        let id = self.async_executor.run(f, true);

        bultins::promise::Promise::Pending { id: id }
    }

    #[inline]
    pub fn poll_async(&mut self, id: AsyncId, input: JValue) -> AsyncResult<JValue> {
        self.async_executor.poll_result(id, input)
    }

    #[inline]
    pub fn finish_async(&mut self) {
        self.async_executor.finish_all(JValue::UNDEFINED);
    }

    #[inline]
    pub fn get_async_stack(&mut self, stack_size: usize) -> &mut [JValue] {
        let stack = &mut self.async_stack[self.aync_stack_offset..];
        self.aync_stack_offset += stack_size;
        return stack;
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

    #[inline]
    pub fn create_native_function<F>(&self, func: F) -> JObject
    where
        F: Fn(JSFuncContext, JValue, &[JValue]) -> Result<JValue, JValue> + 'static,
    {
        let f: Arc<JSFunction> = Arc::new(JSFunction::Native(Arc::new(func)));
        let obj = JObject::with_function(f.create_instance(None));
        obj
    }

    ////////////////////////////////////////////////////////////////////
    //          Class
    ////////////////////////////////////////////////////////////////////

    pub(crate) fn new_class(&self) -> ClassID {
        self.to_mut().classes.push(Some(Arc::new(JSClass::new())));
        return ClassID((self.classes.len() - 1) as u32);
    }

    pub(crate) fn bind_class_constructor(&self, class_id: ClassID, func_id: FuncID) {
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        c.to_mut().constructor = Some(self.get_function(func_id).unwrap());
    }

    pub(crate) fn bind_class_method(&self, class_id: ClassID, func_name: &str, func_id: FuncID) {
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        c.to_mut()
            .methods
            .insert(name, self.get_function(func_id).unwrap());
    }

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

    pub(crate) fn bind_class_prop(&self, class_id: ClassID, name: &str) -> u32 {
        let name = self.to_mut().register_field_name(name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();

        c.props.push(name);
        name
    }

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
        self.declare_variable_static(id, value)
    }

    #[inline]
    pub(crate) fn declare_variable_static(&self, id: u32, value: JValue) {
        self.to_mut().variables.insert(id, value);
    }

    pub fn declare_constant(&self, name: &str, value: JValue) {
        let id = self.to_mut().regester_dynamic_var_name(name);
        self.to_mut().variables.insert(id, value);
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
    pub fn register_regex(&mut self, reg: &str, flags: &str) -> RegexID {
        self.regexs.push((reg.to_string(), flags.to_string()));
        RegexID(self.regexs.len() as u32 - 1)
    }

    #[inline]
    pub fn get_regex(&self, id: RegexID) -> (&str, &str) {
        let s = self.regexs.get(id.0 as usize).unwrap();
        (&s.0, &s.1)
    }

    #[inline]
    pub fn register_string(&mut self, string: &str) -> StringID {
        StringID(self.strings.get_or_intern(string) as u32)
    }

    #[inline]
    pub fn get_string(&self, id: StringID) -> &'static str {
        let r = self.strings.resolve(id.0 as usize).unwrap();
        unsafe { std::mem::transmute_copy(&r) }
    }

    #[inline]
    pub fn global(&self) -> &JObject {
        if let Some(obj) = &self.global {
            obj
        } else {
            panic!()
        }
    }

    #[inline]
    pub fn get_variable(&self, key: u32) -> (JValue, bool) {
        if let Some(v) = self.variables.get(&key) {
            (*v, false)
        } else {
            let key = self
                .dynamic_var_names
                .resolve(DefaultSymbol::try_from_usize(key as usize).unwrap())
                .unwrap();
            self.global
                .unwrap()
                .inner
                .get_property(key, std::ptr::null_mut())
        }
    }

    #[inline]
    pub fn set_variable(&mut self, key: u32, value: JValue) {
        if self.variables.contains_key(&key) {
            self.variables.insert(key, value);
        } else {
            let key = self
                .dynamic_var_names
                .resolve(DefaultSymbol::try_from_usize(key as usize).unwrap())
                .unwrap();
            self.global
                .unwrap()
                .inner
                .to_mut()
                .set_property(key, value, std::ptr::null_mut());
        }
    }

    ///////////////////////////////////////////////////////////////
    //          GC
    //////////////////////////////////////////////////////////////

    #[inline]
    pub unsafe fn run_gc(&mut self) {
        self.object_allocator.marking();

        // scan root and stack
        self.global.unwrap().trace();
        self.user_owned.keys().into_iter().for_each(|v| v.trace());
        self.variables.values().into_iter().for_each(|v| v.trace());
        self.stack.iter().for_each(|v|v.trace());
        self.async_stack.iter().for_each(|v|v.trace());

        self.object_allocator.garbage_collect();
        self.clean_functions();
    }

    /// return the reference counter
    #[inline]
    pub fn user_own_value(&self, v: JValue) -> *mut AtomicUsize {
        if let Some(count) = self.to_mut().user_owned.get_mut(&v) {
            count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return count as *mut AtomicUsize;
        } else {
            self.to_mut().user_owned.insert(v, AtomicUsize::new(0));
            self.user_owned.get(&v).unwrap() as *const AtomicUsize as *mut AtomicUsize
        }
    }

    #[inline]
    pub fn user_drop_value(&self, v: JValue) {
        if let Some(count) = self.to_mut().user_owned.get_mut(&v) {
            let c = count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            if c >= 1 {
                self.to_mut().user_owned.remove(&v);
            }
        }
    }
}
