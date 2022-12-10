use crate::{error::Error, JObject, JSContext, JValue};

use super::string_interner::{NAMES, SYMBOLS};

pub struct JSIterator<'a> {
    ctx: JSContext<'a>,
    iterator: JObject,
}

impl<'a> JSIterator<'a> {
    pub fn new(obj: JValue, ctx: JSContext<'a>) -> Result<Self, JValue> {
        let iter = obj.get_property(SYMBOLS["iterator"], ctx)?;
        if !iter.is_callable() {
            return Err(Error::TypeError(
                "object is not iterator: @@Iterator must be callable".into(),
            )
            .into());
        }
        let iter = iter.call(obj, &[], ctx)?;
        if !iter.is_object() {
            return Err(Error::TypeError(
                "object is not iterator: [@@Iterator]() must return object".into(),
            )
            .into());
        }
        return Ok(Self {
            ctx: ctx,
            iterator: iter.as_object().unwrap(),
        });
    }
}

impl<'a> Iterator for JSIterator<'a> {
    type Item = Result<JValue, JValue>;
    fn next(&mut self) -> Option<Self::Item> {
        let re = self.iterator.get_property(NAMES["next"], self.ctx);
        let next_fn = match re {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        if !next_fn.is_callable() {
            return Some(Err(Error::TypeError(
                "object is not iterator: Iterator.next() must be callable".into(),
            )
            .into()));
        }
        let re = next_fn.call(self.iterator.into(), &[], self.ctx);
        let v = match re {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        let re = v.get_property(NAMES["done"], self.ctx);
        let done = match re {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        if done.to_bool() {
            return None;
        }
        let re = v.get_property(NAMES["value"], self.ctx);
        let value = match re {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        return Some(Ok(value));
    }
}
