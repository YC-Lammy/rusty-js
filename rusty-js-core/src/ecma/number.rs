use crate::{Runtime, JSFuncContext, types::JValue, JObject, bultins::prop::PropFlag};



pub(crate) fn creat_object(runtime:&Runtime) -> JObject{
    let obj = runtime.create_native_function(constructor);
    let prototype = runtime.prototypes.number;

    obj.insert_property("prototype", prototype.into(), PropFlag::NONE);

    prototype.insert_property_builtin("toFixed", runtime.create_native_function(toFixed).into());
    prototype.insert_property_builtin("toPrecision", runtime.create_native_function(toPrecision).into());
    return obj

}

pub fn constructor(_ctx: &JSFuncContext, this: JValue, args: &[JValue]) -> Result<JValue, JValue>{
    todo!()
}

fn toFixed(ctx:&JSFuncContext, this:JValue, args:&[JValue]) -> Result<JValue, JValue>{
    let p = args.get(0).and_then(|v|Some(v.to_number())).unwrap_or(0.0);
    let f = this.to_number();

    return Ok(JValue::String(format!("{:.*}", p as usize, f).into()));
}

fn toPrecision(ctx:&JSFuncContext, this:JValue, args:&[JValue]) -> Result<JValue, JValue>{
    let p = args.get(0).and_then(|v|Some(v.to_number())).unwrap_or(f64::NAN);

    if p.is_normal() && (p <= 0.0 || p > 100.0){
        return Err(JValue::Error(crate::error::Error::RangeError(format!("Number.prototype.toPrecision: precision must be in range 1 to 100, got {}", p))));
    }
    let f = this.to_number();
    let f1 = f.floor() as u64;
    let mut n = f1.to_string();
    let a = p - n.len() as f64;

    if !p.is_normal(){
        return Ok(JValue::String(n.into()))
    }
    if a < 0.0{
        let mut i = (-a) as usize;
        n.replace_range(n.len() - i - 1.. n.len(), &"0".repeat(i));
        return Ok(JValue::String(n.into()));
    } else if a > 0.0{
        n += &(format!("{:.*}", a as usize, f-f1 as f64)[1..]);
        return Ok(JValue::String(n.into()))

    } else{
        return Ok(JValue::String(n.into()))
    }
    
}