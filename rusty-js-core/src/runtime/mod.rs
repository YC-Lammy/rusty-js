use std::alloc::Layout;
use std::sync::Arc;
use std::collections::HashMap;

use string_interner::{StringInterner, Symbol, DefaultSymbol, DefaultBackend};

//mod runtime_context;
mod profiler;
mod object_allocater;
mod gc;
mod async_executor;

pub use gc::GcFlag;

pub use profiler::Profiler;
pub use async_executor::*;

use crate::bultins;
use crate::types::JValue;
use crate::bultins::object::{JObject, JObjectInner};
use crate::bultins::function::JSFunction;
use crate::bultins::class::JSClass;
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

thread_local! {
    pub static JS_RUNTIME: Option<Arc<Runtime>> = None;
}

const DEFAULT_STACK_SIZE:usize = 1048576/std::mem::size_of::<JValue>();

pub struct Runtime{

    obj_field_names:StringInterner,
    dynamic_var_names:StringInterner,

    constants:Vec<JValue>,
    regexs:Vec<(String, String)>,
    strings:StringInterner<DefaultBackend<usize>>,

    pub(crate) stack:Box<[JValue;DEFAULT_STACK_SIZE]>,

    object_allocator:object_allocater::ObjectAllocator,

    variables:HashMap<u32, JValue, nohasher::NoHasherBuilder>,
    functions:Vec<Option<Arc<JSFunction>>>,
    classes:Vec<Option<Arc<JSClass>>>,

    pub global:Option<JObject>,

    pub(crate) async_executor:async_executor::AsyncExecutor<JValue>,
    pub(crate) generator_executor:async_executor::AsyncExecutor<JValue>,
}

unsafe impl Sync for Runtime{}
unsafe impl Send for Runtime{}

impl Runtime{
    pub fn new() -> Arc<Self>{
        // allocate without writing on stack to prevent stackoverflow
        let stack = unsafe{    
            let ptr = std::alloc::alloc(Layout::new::<[JValue; DEFAULT_STACK_SIZE]>()) as *mut [JValue; DEFAULT_STACK_SIZE];
            Box::from_raw(ptr)
        };
        let runtime = Arc::new(Self { 
            obj_field_names: StringInterner::new(), 
            dynamic_var_names: StringInterner::new(), 

            stack:stack,

            object_allocator:Default::default(),

            constants: vec![], 
            regexs: vec![], 
            strings: StringInterner::new(), 
            variables: HashMap::default(), 
            functions: vec![], 
            classes: vec![], 
            global: None, 
            async_executor:async_executor::AsyncExecutor::new(),
            generator_executor:async_executor::AsyncExecutor::new()
        });

        runtime
    }
    pub fn attach(self:Arc<Self>){
        JS_RUNTIME.with(|runtime| unsafe{
            *(runtime as *const Option<Arc<Runtime>> as *mut Option<Arc<Runtime>>) = Some(self);
        })
    }

    pub fn deattach(){
        JS_RUNTIME.with(|runtime| unsafe{
            *(runtime as *const Option<Arc<Runtime>> as *mut Option<Arc<Runtime>>) = None;
        })
    }

    pub fn current() -> Arc<Runtime>{
        JS_RUNTIME.with(|runtime|{
            if let Some(r) = runtime{
                r.clone()
            } else{
                panic!("js runtime not attached on thread {:#?}.", std::thread::current().id())
            }
        })
    }

    pub fn execute<T>(self:Arc<Self>,filename:&str, script:&str) -> Result<JValue, crate::error::Error>{
        use swc_common::{SourceFile, FileName};

        let src = SourceFile::new(
            FileName::Custom(filename.to_string()), 
            false, 
            FileName::Anon, 
            script.to_string(), 
            swc_common::BytePos(1)
        );
        let mut v = Vec::new();
        let re = swc_ecma_parser::parse_file_as_script(
            &src, 
            swc_ecma_parser::Syntax::Es(swc_ecma_parser::EsConfig { 
                jsx: false, 
                fn_bind: true, 
                decorators: true, 
                decorators_before_export: true, 
                export_default_from: true, 
                import_assertions: true, 
                private_in_object: true, 
                allow_super_outside_method: false, 
                allow_return_outside_function: false
            }), 
            swc_ecma_ast::EsVersion::Es2022, 
            None, 
            &mut v
        );
    
        let script = re.unwrap();
        let mut builder = crate::baseline::bytecode_builder::FunctionBuilder::new(self.clone());
        
        for i in &script.body{
            builder.translate_statement(None, i)?;
        };

        let bytecodes = builder.bytecode;

        let mut intpr = crate::interpreter::Interpreter::global(&self, self.to_mut().stack.as_mut_slice());
        let re = intpr.run(JValue::Object(self.global.unwrap()), &[], &bytecodes);

        self.to_mut().finish_async();

        match re{
            Ok(v) => Ok(v),
            Err(e) => return Err(crate::error::Error::Value(e))
        }
    }

    pub(crate) fn to_mut(&self) -> &mut Self{
        unsafe{(self as *const Self as *mut Self).as_mut().unwrap()}
    }

    pub(crate) fn allocate_obj(&self) -> &'static mut JObjectInner{
        unsafe{self.to_mut().object_allocator.allocate()}
    }

    pub(crate) fn regester_dynamic_var_name(&self, name:&str) -> u32{
        self.to_mut().dynamic_var_names.get_or_intern(name).to_usize() as u32
    }

    pub(crate) fn register_field_name(&self, name:&str) -> u32{
        self.to_mut().obj_field_names.get_or_intern(name).to_usize() as u32
    }

    pub(crate) fn get_field_name(&self, id:u32) -> &str{
        self.obj_field_names.resolve(DefaultSymbol::try_from_usize(id as usize).unwrap()).unwrap()
    }



    ///////////////////////////////////////////////////////////////////
    //          async
    //////////////////////////////////////////////////////////////////

    pub fn call_async<F>(&mut self, f:F) -> bultins::promise::Promise where F:Fn() -> Result<JValue, JValue> +'static{

        let id = self.async_executor.run(f, true);

        bultins::promise::Promise::Pending{
            id:id
        }
    }

    pub fn poll_async(&mut self, id:AsyncId, input:JValue) -> AsyncResult<JValue>{
        self.async_executor.poll_result(id, input)
    }

    pub fn finish_async(&mut self) {
        self.async_executor.finish_all(JValue::UNDEFINED);
    }



    //////////////////////////////////////////////////////////////////
    //          GC
    //////////////////////////////////////////////////////////////////

    /// garbage collector must call this function to clean functions
    pub(crate) fn clean_functions(&mut self){
        for i in &mut self.functions{
            if let Some(a) = i{
                if Arc::strong_count(a) == 1{
                    *i = None;
                }
            }
        }
        for i in &mut self.classes{
            if let Some(a) = i{
                if Arc::strong_count(a) == 1{
                    *i = None;
                }
            }
        }
    }

    pub(crate) fn new_function(&self, func:Arc<JSFunction>) -> FuncID{
        self.to_mut().functions.push(Some(func));
        return FuncID((self.functions.len() -1) as u32)
    }

    pub(crate) fn get_function(&self, id:FuncID) -> Option<Arc<JSFunction>>{
        self.functions[id.0 as usize].clone()
    }


    ////////////////////////////////////////////////////////////////////
    //          Class
    ////////////////////////////////////////////////////////////////////

    pub(crate) fn new_class(&self) -> ClassID{
        self.to_mut().classes.push(Some(Arc::new(JSClass::new())));
        return ClassID((self.classes.len() -1) as u32)
    }

    pub(crate) fn bind_class_constructor(&self, class_id:ClassID, func_id:FuncID){
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        c.to_mut().constructor = Some(self.get_function(func_id).unwrap());
    }

    pub(crate) fn bind_class_method(&self, class_id:ClassID, func_name:&str, func_id:FuncID){
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        c.to_mut().methods.insert(name, self.get_function(func_id).unwrap());
    }

    pub(crate) fn bind_class_getter(&self, class_id:ClassID, func_name:&str, func_id:FuncID){
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();
        let f = self.get_function(func_id).unwrap();
        if let Some(gs) = c.get_setters.get_mut(&name){
            gs.0 = Some(f);
        } else{
            c.get_setters.insert(name, (Some(f), None));
        }
    }

    pub(crate) fn bind_class_setter(&self, class_id:ClassID, func_name:&str, func_id:FuncID){
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();
        let f = self.get_function(func_id).unwrap();
        if let Some(gs) = c.get_setters.get_mut(&name){
            gs.1 = Some(f);
        } else{
            c.get_setters.insert(name, (None, Some(f)));
        }
    }

    pub(crate) fn bind_class_static_method(&self, class_id:ClassID, func_name:&str, func_id:FuncID){
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        c.to_mut().static_methods.insert(name, self.get_function(func_id).unwrap());
    }

    pub(crate) fn bind_class_static_getter(&self, class_id:ClassID, func_name:&str, func_id:FuncID){
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();
        let f = self.get_function(func_id).unwrap();
        if let Some(gs) = c.static_get_setters.get_mut(&name){
            gs.0 = Some(f);
        } else{
            c.static_get_setters.insert(name, (Some(f), None));
        }
    }

    pub(crate) fn bind_class_static_setter(&self, class_id:ClassID, func_name:&str, func_id:FuncID){
        let name = self.to_mut().register_field_name(func_name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();
        let f = self.get_function(func_id).unwrap();
        if let Some(gs) = c.static_get_setters.get_mut(&name){
            gs.1 = Some(f);
        } else{
            c.static_get_setters.insert(name, (None, Some(f)));
        }
    }

    pub(crate) fn bind_class_prop(&self, class_id:ClassID, name:&str) -> u32{
        let name = self.to_mut().register_field_name(name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();

        c.props.push(name);
        name
    }

    pub(crate) fn bind_class_static_prop(&self, class_id:ClassID, name:&str) -> u32{
        let name = self.to_mut().register_field_name(name);
        let c = self.classes[class_id.0 as usize].clone().unwrap();
        let c = c.to_mut();

        c.static_props.push(name);
        name
    }

    pub fn default_constructor(&self) -> FuncID{
        FuncID(0)
    }



    ////////////////////////////////////////////////////////////////
    //         variables
    ////////////////////////////////////////////////////////////////

    pub fn declare_variable(&self, name:&str, value:JValue){
        let id = self.to_mut().regester_dynamic_var_name(name);
        self.declare_variable_static(id, value)
    }

    pub(crate) fn declare_variable_static(&self, id:u32, value:JValue){
        self.to_mut().variables.insert(id, value);
    }

    pub fn declare_constant(&self, name:&str, value:JValue){
        let id = self.to_mut().regester_dynamic_var_name(name);
        self.to_mut().variables.insert(id, value);
    }

    pub fn unamed_constant(&mut self, value:JValue) -> ConstID{
        self.constants.push(value);

        ConstID(self.constants.len() as u32 -1)
    }

    pub fn get_unamed_constant(&self, id:ConstID) -> JValue{
        *self.constants.get(id.0 as usize).unwrap()
    }

    pub fn register_regex(&mut self, reg:&str, flags:&str) -> RegexID{
        self.regexs.push((reg.to_string(), flags.to_string()));
        RegexID(self.regexs.len() as u32 -1)
    }

    pub fn get_regex(&self, id:RegexID) -> (&str, &str){
        let s = self.regexs.get(id.0 as usize).unwrap();
        (&s.0, &s.1)
    }

    pub fn register_string(&mut self, string:&str) -> StringID{
        StringID(self.strings.get_or_intern(string) as u32)
    }

    pub fn get_string(&self, id:StringID) -> &'static str{
        let r = self.strings.resolve(id.0 as usize).unwrap();
        unsafe{std::mem::transmute_copy(&r)}
    }

    pub fn global(&self) -> &JObject{
        if let Some(obj) = &self.global{
            obj
        } else{
            panic!()
        }
    }

    pub fn get_variable(&self, key:u32) -> (JValue, bool){
        if let Some(v) = self.variables.get(&key){
            (*v, false)
        } else{
            let key = self.dynamic_var_names.resolve(DefaultSymbol::try_from_usize(key as usize).unwrap()).unwrap();
            self.global.unwrap().inner.get_property(key, std::ptr::null_mut())
        }
    } 

    pub fn set_variable(&mut self, key:u32, value:JValue){
        if self.variables.contains_key(&key){
            self.variables.insert(key, value);
        } else{
            let key = self.dynamic_var_names.resolve(DefaultSymbol::try_from_usize(key as usize).unwrap()).unwrap();
            self.global.unwrap().inner.to_mut().set_property(key, value, std::ptr::null_mut());
        }
    }


    ///////////////////////////////////////////////////////////////
    //          GC
    //////////////////////////////////////////////////////////////
    
    pub fn run_gc(&mut self){
        self.object_allocator.marking();

        // scan root and stack

        self.object_allocator.garbage_collect();
        self.clean_functions();
    }
}

