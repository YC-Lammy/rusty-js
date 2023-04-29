use crate::{
    bultins::object_property::PropFlag,
    error::Error,
    value::JValue,
    utils::string_interner::{NAMES, SYMBOLS},
    JObject, JSContext, Runtime,
};

macro_rules! builtin {
    ($rt:ident, $obj:ident, $name:tt, $f:ident) => {
        $obj.insert_property_builtin(NAMES[$name], $rt.create_native_function($f).into());
    };
}

pub fn init(rt: &Runtime) -> JObject {
    let proto = rt.prototypes.array;

    let obj = rt.create_constructor(constructor, "Array", proto);

    // get Array[@@species]
    obj.bind_getter(
        SYMBOLS["species"],
        rt.create_native_function(|ctx, _this, _args| Ok(ctx.runtime.prototypes.array.into())),
    );

    builtin!(rt, obj, "from", from); // Array.from
    builtin!(rt, obj, "isArray", is_array); // Array.isArray
    builtin!(rt, obj, "of", of);

    // the Array.prototype is an array
    proto.set_inner(crate::bultins::object::JObjectValue::Array(
        Default::default(),
    ));
    proto.insert_property(NAMES["length"], 0.into(), PropFlag::WRITABLE);

    proto.insert_property_builtin(NAMES["push"], rt.create_native_function(push).into());
    return obj;
}

/// 23.1.1.1 Array ( ...values )
fn constructor(ctx: JSContext, this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    let array = if let Some(array) = ctx.runtime.new_target.as_object() {
        //initializes a new Array when called as a constructor.
        array.set_inner(crate::bultins::object::JObjectValue::Array(
            Default::default(),
        ));
        array.insert_property(
            NAMES["__proto__"],
            ctx.runtime.prototypes.array.into(),
            Default::default(),
        );
        array.insert_property(NAMES["length"], 0.into(), PropFlag::WRITABLE);
        array
    } else {
        JObject::array()
    };

    //3. Let numberOfArgs be the number of elements in values.
    let number_of_args = args.len();

    // 4. If numberOfArgs = 0, then
    if number_of_args == 0 {
        // a. Return ! ArrayCreate(0, proto).
        return Ok(array.into());

    // 5. Else if numberOfArgs = 1, then
    } else if number_of_args == 1 {
        // a. Let len be values[0].
        let len = args[0];

        // c. If len is not a Number, then
        if !len.is_number() {
            array.as_array().unwrap().push((PropFlag::THREE, len));
            array.insert_property(NAMES["length"], 1.into(), PropFlag::WRITABLE);
        } else {
            array.insert_property(
                NAMES["length"],
                len.to_length(ctx)?.into(),
                PropFlag::WRITABLE,
            );
        }

        return Ok(array.into());
    } else {
        // does not follow spec but same result
        array
            .as_array()
            .unwrap()
            .extend(args.iter().map(|v| (Default::default(), *v)));
        array.insert_property(NAMES["length"], args.len().into(), PropFlag::WRITABLE);
        return Ok(array.into());
    }
}

/// Array.from
fn from(ctx: JSContext, _this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    let items = args.get(0);
    let map_fn = args.get(1);
    let this_arg = *args.get(2).unwrap_or(&JValue::UNDEFINED);

    if let Some(f) = map_fn {
        if let Some(v) = f.as_object() {
            if !v.is_function_instance() && !v.is_class() {
                return Err(JValue::from(Error::TypeError(
                    "Array.from: mapfn is not callable".into(),
                )));
            }
        }
    }

    if items.is_none() {
        return Err(JValue::from(Error::TypeError(
            "undefined is not iterable (cannot read property Symbol(Symbol.iterator))".into(),
        )));
    }

    let items = *items.unwrap();
    if !items.is_object() {
        return Err(JValue::from(Error::TypeError(format!(
            "{} is not iterable (cannot read property Symbol(Symbol.iterator))",
            items.typ().as_str()
        ))));
    }
    let items = items.as_object().unwrap();

    if items.has_property(SYMBOLS["iterator"]) {
        let iter_method = items.get_property(SYMBOLS["iterator"], ctx)?;
        let iter = iter_method.call(items.into(), &[], ctx)?;

        let mut values = Vec::new();

        loop {
            let next = iter.get_property("next", ctx)?;
            let next_result = next.call(iter, &[], ctx)?;
            let done = next_result.get_property("done", ctx)?;
            let mut value = next_result.get_property("value", ctx)?;

            if let Some(f) = map_fn {
                value = f.call(this_arg, &[value], ctx)?;
            }

            values.push((PropFlag::THREE, value));

            if done.to_bool() {
                break;
            }
        }

        let array = JObject::array();
        array.insert_property(NAMES["length"], values.len().into(), PropFlag::WRITABLE);
        *array.as_array().unwrap() = values;

        return Ok(array.into());
    } else {
        let length = items.get_property("length", ctx)?.to_number(ctx)?;
        let mut values = Vec::new();

        for i in 0..length as usize {
            let mut value = items.get_property(&i.to_string(), ctx)?;

            if let Some(f) = map_fn {
                value = f.call(this_arg, &[value], ctx)?;
            }

            values.push((PropFlag::THREE, value));
        }

        let array = JObject::array();
        array.insert_property(NAMES["length"], values.len().into(), PropFlag::WRITABLE);
        *array.as_array().unwrap() = values;

        return Ok(array.into());
    }
}

/// Array.isArray
fn is_array(_ctx: JSContext, _this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    if let Some(v) = args.get(0) {
        if let Some(o) = v.as_object() {
            return Ok(o.is_array().into());
        }
    }
    return Ok(JValue::FALSE);
}

/// Array.of
fn of(_ctx: JSContext, _this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    let array = JObject::array();
    array
        .as_array()
        .unwrap()
        .extend(args.iter().map(|v| (PropFlag::THREE, *v)));
    array.insert_property(NAMES["length"], args.len().into(), PropFlag::WRITABLE);
    return Ok(array.into());
}

fn push(ctx: JSContext, this: JValue, args: &[JValue]) -> Result<JValue, JValue> {
    let mut len = 0;

    if let Some(obj) = this.as_object() {
        if let Some(ar) = obj.as_array() {
            ar.extend(args.iter().map(|v| (PropFlag::THREE, *v)));
            len = ar.len();

            obj.insert_property(
                NAMES["length"],
                JValue::create_number(len as f64),
                PropFlag::WRITABLE,
            );
        } else {
        }
    }
    Ok(JValue::create_number(len as f64))
}
