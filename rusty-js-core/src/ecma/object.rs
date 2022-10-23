use crate::bultins::object::JObjectValue;
use crate::error::Error;
use crate::{types::JValue, JSFuncContext};
use crate::{JObject, Runtime};

macro_rules! prop {
    ($rt:ident, $obj:ident, $name:tt, $func:ident) => {
        $obj.insert_property_builtin($name, $rt.create_native_function($func).into());
    };
}

pub fn create_object(rt: &Runtime) -> JObject {
    let obj = rt.create_native_function(constructor);

    prop!(rt, obj, "assign", assign);
    prop!(rt, obj, "create", create);
    return obj;
}

pub fn constructor(_ctx: &JSFuncContext, this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    if this.is_new_target() {
        if args.len() == 0 {
            return Ok(this);
        } else {
            let v = args[0];
            let obj = unsafe { this.value.object };
            unsafe {
                if v.is_string() {
                    obj.set_inner(JObjectValue::String(v.value.string));
                }
                if v.is_bigint() {
                    obj.set_inner(JObjectValue::BigInt(v.value.bigint));
                }
                if v.is_bool() {
                    obj.set_inner(JObjectValue::Boolean(v.is_true()));
                }
                if v.is_number() {
                    obj.set_inner(JObjectValue::Number(v.value.number));
                }

                if v.is_symbol() {
                    obj.set_inner(JObjectValue::Symbol(v.value.symbol));
                }
            }
            return Ok(this);
        }
    } else {
        if args.len() == 0 {
            return Ok(JObject::new().into());
        } else {
            return Ok(args[0].to_object().into());
        }
    }
}

pub fn assign(ctx: &JSFuncContext, _this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    if args.len() > 0 {
        if args[0].is_null() || args[0].is_undefined() {
            return Err(JValue::Error(Error::TypeError(
                "Cannot convert undefined or null to object".to_owned(),
            )));
        }

        let obj = args[0].to_object();
        for i in &args[1..] {
            if i.is_object() {
                unsafe {
                    for (prop, (flag, v)) in &i.value.object.inner.values {
                        if flag.is_enumerable() {
                            let v = i
                                .value
                                .object
                                .get_property_static(prop.0, ctx.stack)
                                .unwrap();
                            obj.insert_property_static(prop.0, v, *flag & Default::default());
                        }
                    }
                }
            }
        }
        return Ok(obj.into());
    }

    return Err(JValue::Error(Error::TypeError(
        "Cannot convert undefined or null to object".to_owned(),
    )));
}

pub fn create(ctx: &JSFuncContext, this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    if args.len() == 0 {
        return Err(JValue::Error(Error::TypeError(
            "Object.create expected null or Object".to_owned(),
        )));
    }

    if !args[0].is_object() || !args[0].is_null() {
        return Err(JValue::Error(Error::TypeError(
            "Object.create expected null or Object".to_owned(),
        )));
    }

    let obj = JObject::new();
    obj.insert_property("__proto__", args[0], Default::default());

    if args.len() >= 2 {
        defineProperties(ctx, this, &[obj.into(), args[1]])?;
    }
    return Ok(obj.into());
}

pub fn defineProperties(
    ctx: &JSFuncContext,
    _this: JValue,
    args: &[JValue],
) -> Result<JValue, JValue> {
    todo!()
}
