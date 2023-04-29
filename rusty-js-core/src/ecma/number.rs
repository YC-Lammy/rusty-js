use crate::{bultins::object_property::PropFlag, value::JValue, JObject, JSContext, Runtime};

use crate::utils::string_interner::NAMES;

macro_rules! builtin {
    ($rt:ident, $obj:ident, $name:tt, $f:ident) => {
        $obj.insert_property_builtin(NAMES[$name], $rt.create_native_function($f).into());
    };
}

pub(crate) fn creat_object(rt: &Runtime) -> JObject {
    let prototype = rt.prototypes.number;

    let obj = rt.create_constructor(constructor, "Number", prototype);

    builtin!(rt, prototype, "toFixed", to_fixed);
    builtin!(rt, prototype, "toPrecision", to_precision);

    return obj;
}

pub fn constructor(_ctx: JSContext, this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    todo!()
}

fn to_fixed(ctx: JSContext, this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    let p = args.get(0).unwrap_or(&JValue::ZERO);
    let p = p.to_number(ctx)?;
    let f = this.to_number(ctx)?;

    return Ok(JValue::create_string(
        ctx.runtime
            .allocate_string(&format!("{:.*}", p as usize, f)),
    ));
}

fn to_precision(ctx: JSContext, this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    let p = *args.get(0).unwrap_or(&JValue::UNDEFINED);

    let p = p.to_number(ctx)?;

    if p.is_normal() && (p <= 0.0 || p > 100.0) {
        return Err(JValue::from(crate::error::Error::RangeError(format!(
            "Number.prototype.toPrecision: precision must be in range 1 to 100, got {}",
            p
        ))));
    }
    let f = this.to_number(ctx)?;
    let f1 = f.floor() as u64;
    let mut n = f1.to_string();
    let a = p - n.len() as f64;

    if !p.is_normal() {
        return Ok(JValue::create_string(ctx.runtime.allocate_string(&n)));
    }
    if a < 0.0 {
        let i = (-a) as usize;
        n.replace_range(n.len() - i - 1..n.len(), &"0".repeat(i));
        return Ok(JValue::create_string(ctx.runtime.allocate_string(&n)));
    } else if a > 0.0 {
        n += &(format!("{:.*}", a as usize, f - f1 as f64)[1..]);
        return Ok(JValue::create_string(ctx.runtime.allocate_string(&n)));
    } else {
        return Ok(JValue::create_string(ctx.runtime.allocate_string(&n)));
    }
}
