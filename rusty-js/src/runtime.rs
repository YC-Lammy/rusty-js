use std::{sync::Arc, collections::HashMap, any::TypeId};

use parking_lot::RwLock;


use crate::{JSValue, CustomObject, HasPrototype, JSObject, CustomObjectWrapper};


pub struct Runtime{
    runtime:Arc<rusty_js_core::Runtime>,
    // contains the class functions created, these will not be dropped
    classes:Arc<RwLock<HashMap<TypeId, JSObject>>>,
}

impl Runtime{
    pub fn new() -> Self{
        let rt = rusty_js_core::Runtime::new();
        Self { 
            runtime:rt, 
            classes: Default::default()
        }
    }

    pub fn register_class<T:'static>(&mut self, name:&str) where T:CustomObject + HasPrototype{
        

        let mut classes = self.classes.write();

        let proto = if let Some(t) = classes.get(&TypeId::of::<T>()){
            t.obj
        } else{
            let proto = self.runtime.creat_object();
            let counter = self.runtime.user_own_value(proto.into());
            classes.insert(TypeId::of::<T>(), JSObject { obj: proto, counter });
            proto
        };

        // this prototype extends another custom class
        if let Some(t) = T::prototype(){
            let mut w = self.classes.write();
            if let Some(v) = w.get(&t){
                proto.insert_property("__proto__", v.obj.into(), Default::default());
            } else{
                let p = self.runtime.creat_object();
                let counter = self.runtime.user_own_value(p.into());
        
                w.insert(t, JSObject { obj: p, counter});
                proto.insert_property("__proto__", p.into(), Default::default());
            }
        };

        for (name, m) in T::methods(){
            let f = self.runtime.create_native_function(move|ctx, this, args|{
                let this = JSValue { value: this, counter: std::ptr::null() };
                let args = args.iter().map(|v|JSValue{value:*v, counter:std::ptr::null()}).collect::<Vec<JSValue>>();
                let re = (m)(ctx, &this, &args);
                match re{
                    Ok(v) => Ok(v.value),
                    Err(e) => Err(e.value)
                }
            });
            proto.insert_property_builtin(&name, f.into());
        };

        for (name, m) in T::getters(){
            let getter = self.runtime.create_native_function(move|ctx, this, args|{
                let this = JSValue { value: this, counter: std::ptr::null() };
                let args = args.iter().map(|v|JSValue{value:*v, counter:std::ptr::null()}).collect::<Vec<JSValue>>();
                let re = (m)(ctx, &this, &args);
                match re{
                    Ok(v) => Ok(v.value),
                    Err(e) => Err(e.value)
                }
            });
            let id = self.runtime.register_field_name(name);
            proto.bind_getter(id, getter);
        };

        for (name, m) in T::setters(){
            let setter = self.runtime.create_native_function(move|ctx, this, args|{
                let this = JSValue { value: this, counter: std::ptr::null() };
                let args = args.iter().map(|v|JSValue{value:*v, counter:std::ptr::null()}).collect::<Vec<JSValue>>();
                let re = (m)(ctx, &this, &args);
                match re{
                    Ok(v) => Ok(v.value),
                    Err(e) => Err(e.value)
                }
            });
            let id = self.runtime.register_field_name(name);
            proto.bind_setter(id, setter);
        };

        self.runtime.create_constructor(move |ctx, this, args|{
            let this = if this.is_new_target(){
                *this.as_object().unwrap()
            } else{
                let this = ctx.runtime.creat_object();
                this.insert_property("__proto__", proto.into(), Default::default());
                this
            };

            let args = args.iter().map(|v|JSValue{value:*v, counter:std::ptr::null()}).collect::<Vec<JSValue>>();

            let r = T::constructor(&args);

            this.bind_custom_object(Arc::new(CustomObjectWrapper(r)));

            Ok(this.into())
        }, name, proto);
    }
}