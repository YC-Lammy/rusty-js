use crate::bultins::object::JObjectValue;
use crate::error::Error;
use crate::utils::string_interner::NAMES;
use crate::{types::JValue, JSContext};
use crate::{JObject, JSymbol, Runtime};

macro_rules! builtin {
    ($rt:ident, $obj:ident, $name:tt, $f:ident) => {
        $obj.insert_property_builtin(NAMES[$name], $rt.create_native_function($f).into());
    };
}
pub fn ect(rt: &Runtime) -> JObject {
    let proto = rt.prototypes.object;
    let obj = rt.create_constructor(constructor, "Object", proto);

    builtin!(rt, obj, "assign", assign);
    builtin!(rt, obj, "create", create);

    return obj;
}

pub fn constructor(ctx: JSContext, this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    if let Some(obj) = ctx.runtime.new_target.as_object() {
        if args.len() == 0 {
            return Ok(this);
        } else {
            let v = args[0];

            unsafe {
                if let Some(s) = v.as_string() {
                    obj.set_inner(JObjectValue::String(s));
                }
                if let Some(b) = v.as_bigint() {
                    obj.set_inner(JObjectValue::BigInt(b));
                }
                if v.is_true() || v.is_false() {
                    obj.set_inner(JObjectValue::Boolean(v.is_true()));
                }
                if v.is_number() {
                    obj.set_inner(JObjectValue::Number(v.as_number_uncheck()));
                }
                if let Some(sym) = v.as_symbol() {
                    obj.set_inner(JObjectValue::Symbol(JSymbol(sym)));
                }
            }
            return Ok(this);
        }
    } else {
        if args.len() == 0 {
            return Ok(JObject::new().into());
        } else {
            return args[0].to_object(ctx);
        }
    }
}

pub fn assign(ctx: JSContext, _this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    if args.len() > 0 {
        if args[0].is_null() || args[0].is_undefined() {
            return Err(JValue::from(Error::TypeError(
                "Cannot convert undefined or null to object".to_owned(),
            )));
        }

        let obj = args[0].to_object(ctx)?;
        for i in &args[1..] {
            if let Some(o) = i.as_object() {
                todo!()
            }
        }
        return Ok(obj.into());
    }

    return Err(JValue::from(Error::TypeError(
        "Cannot convert undefined or null to object".to_owned(),
    )));
}

pub fn create(ctx: JSContext, this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    if args.len() == 0 {
        return Err(JValue::from(Error::TypeError(
            "Object.create expected null or Object".to_owned(),
        )));
    }

    if !args[0].is_object() || !args[0].is_null() {
        return Err(JValue::from(Error::TypeError(
            "Object.create expected null or Object".to_owned(),
        )));
    }

    let obj = JObject::new();
    obj.insert_property(NAMES["__proto__"], args[0], Default::default());

    if args.len() >= 2 {
        defineProperties(ctx, this, &[obj.into(), args[1]])?;
    }
    return Ok(obj.into());
}

pub fn defineProperties(ctx: JSContext, _this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    todo!()
}
