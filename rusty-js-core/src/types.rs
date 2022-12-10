use num_traits::ToPrimitive;

use crate::bultins::function::JSContext;
use crate::bultins::object::PropKey;
use crate::bultins::object::{JObject, JObjectInner, JObjectValue, ToProperyKey};
use crate::bultins::strings::JSString;
use crate::bultins::JSBigInt;
use crate::error::Error;
use crate::runtime::Runtime;
use crate::utils::string_interner::{NAMES, SYMBOLS};

// Any function in this module cannot be inlined
// inlining functions of JValue will cause undefined behaviar due to the duplication of constants

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct JSType(pub u8);

impl JSType {
    pub const Null: Self = Self(0b00000001);
    pub const Undefined: Self = Self(0b00000010);
    pub const Number: Self = Self(0b00000100);
    pub const Boolean: Self = Self(0b00001000);
    pub const Bigint: Self = Self(0b00010000);
    pub const Symbol: Self = Self(0b00100000);
    pub const Object: Self = Self(0b01000000);
    pub const String: Self = Self(0b10000000);

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Object | Self::Null => "[Object object]",
            Self::Undefined => "undefined",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Bigint => "bigint",
            Self::Symbol => "symbol",
            Self::String => "string",
            _ => "unknown"
        }
    }
}

impl std::ops::BitOr for JSType{
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for JSType{
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash)]
pub struct JValue(pub(crate) u64);

impl JValue {
    pub const SIZE: usize = std::mem::size_of::<Self>();

    pub const NAN_BITS: u64 = 0b0111111111111000000000000000000000000000000000000000000000000000;
    pub const DATA_BITS: u64 = 0b0000000000000000111111111111111111111111111111111111111111111111;
    pub const TAG_BITS: u64 = 0b1111111111111111000000000000000000000000000000000000000000000000;

    const MASK_FALSE: u64 = 0b0000000000001001000000000000000000000000000000000000000000000000;
    const MASK_TRUE: u64 = 0b0000000000001010000000000000000000000000000000000000000000000001;
    const MASK_NULL: u64 = 0b0000000000001011000000000000000000000000000000000000000000000000;
    const MASK_INT: u64 = 0b0000000000001100000000000000000000000000000000000000000000000000;
    const MASK_UNDEFINED: u64 = 0b0000000000001101000000000000000000000000000000000000000000000000;
    const MASK_SYMBOL: u64 = 0b0000000000001110000000000000000000000000000000000000000000000000;
    //const MASK_BIGINT32: u64 =  0b0000000000001111000000000000000000000000000000000000000000000000;
    const MASK_OBJECT: u64 = 0b1000000000001001000000000000000000000000000000000000000000000000;
    const MASK_STRING: u64 = 0b1000000000001010000000000000000000000000000000000000000000000000;
    const MASK_BIGINT: u64 = 0b1000000000001011000000000000000000000000000000000000000000000000;
    const MASK_1: u64 = 0b1000000000001100000000000000000000000000000000000000000000000000;
    const MASK_2: u64 = 0b1000000000001101000000000000000000000000000000000000000000000000;
    const MASK_3: u64 = 0b1000000000001110000000000000000000000000000000000000000000000000;
    const MASK_4: u64 = 0b1000000000001111000000000000000000000000000000000000000000000000;

    pub const FALSE_TAG: u64 = Self::MASK_FALSE | Self::NAN_BITS;
    pub const TRUE_TAG: u64 = Self::MASK_TRUE | Self::NAN_BITS;
    pub const NULL_TAG: u64 = Self::MASK_NULL | Self::NAN_BITS;
    pub const UNDEFINED_TAG: u64 = Self::MASK_UNDEFINED | Self::NAN_BITS;
    pub const INT_TAG: u64 = Self::MASK_INT | Self::NAN_BITS;
    //pub const BIGINT32_TAG: u64 = Self::MASK_BIGINT32 | Self::NAN_BITS;
    pub const SYMBOL_TAG: u64 = Self::MASK_SYMBOL | Self::NAN_BITS;
    pub const OBJECT_TAG: u64 = Self::MASK_OBJECT | Self::NAN_BITS;
    pub const STRING_TAG: u64 = Self::MASK_STRING | Self::NAN_BITS;
    pub const BIGINT_TAG: u64 = Self::MASK_BIGINT | Self::NAN_BITS;

    pub const FALSE: Self = Self(Self::FALSE_TAG);
    pub const TRUE: Self = Self(Self::TRUE_TAG);
    pub const NULL: Self = Self(Self::NULL_TAG);
    pub const UNDEFINED: Self = Self(Self::UNDEFINED_TAG);
    pub const NAN: Self = Self(Self::NAN_BITS);
    pub const ZERO: Self = Self(0);

    pub fn to_bits(&self) -> u64 {
        return self.0;
    }

    pub fn is_nan(&self) -> bool {
        self.is_number() && f64::from_bits(self.0).is_nan()
    }

    pub fn is_number(&self) -> bool {
        self.0 & Self::NAN_BITS != Self::NAN_BITS || self.0 == Self::NAN_BITS
    }

    pub fn number(n: f64) -> Self {
        Self(n.to_bits())
    }

    pub fn is_false(&self) -> bool {
        self.0 == Self::FALSE_TAG
    }

    pub fn is_true(&self) -> bool {
        self.0 == Self::TRUE.0
    }

    pub fn is_null(&self) -> bool {
        self.0 == Self::NULL_TAG
    }

    pub fn is_undefined(&self) -> bool {
        self.0 == Self::UNDEFINED_TAG
    }

    pub fn is_int(&self) -> bool {
        (self.0 >> 48) == (Self::INT_TAG >> 48)
    }

    pub fn is_symbol(&self) -> bool {
        (self.0 >> 48) == (Self::SYMBOL_TAG >> 48)
    }

    //pub fn is_bigint32(&self) -> bool{
    //    (self.0 >> 48) == (Self::BIGINT32_TAG >> 48)
    //}

    pub fn is_object(&self) -> bool {
        (self.0 >> 48) == (Self::OBJECT_TAG >> 48)
    }

    pub fn is_bigint(&self) -> bool {
        (self.0 >> 48) == (Self::BIGINT_TAG >> 48)
    }

    pub fn is_string(&self) -> bool {
        (self.0 >> 48) == (Self::STRING_TAG >> 48)
    }

    pub fn is_empty_string(&self) -> bool {
        if let Some(s) = self.as_string() {
            if s.len() == 0 {
                return true;
            }
        }
        return false;
    }

    pub const fn create_number(f: f64) -> Self {
        Self(unsafe { std::mem::transmute(f) })
    }

    pub const fn create_int(i: i32) -> Self {
        let i = i as u32 as u64;
        Self(Self::INT_TAG | i)
    }

    pub fn create_symbol(i: u32) -> Self {
        let i = i as u64;
        Self(Self::SYMBOL_TAG | i)
    }

    pub fn create_bigint<T: Into<i128>>(n: T) -> Self {
        let rt = Runtime::current();
        let b = rt.allocate_bigint();
        b.set_value(n.into());

        Self(Self::BIGINT_TAG | (b as *mut _ as u64))
    }

    pub fn create_bigint_allocated(b: *const JSBigInt) -> Self {
        let ptr = b as u64;
        Self(Self::BIGINT_TAG | ptr)
    }

    pub fn create_object(obj: JObject) -> Self {
        let i = obj.inner as *const _ as u64;
        Self(Self::OBJECT_TAG | i)
    }

    pub fn create_string(s: JSString) -> Self {
        let i = s.0 as u64;
        Self(Self::STRING_TAG | i)
    }

    pub fn create_static_string(s: &'static str) -> Self {
        let rt = Runtime::current();
        Self::create_string(rt.allocate_string(s))
    }

    pub fn as_int(&self) -> Option<i32> {
        if self.is_int() {
            Some(((self.0 << 32) >> 32) as i32)
        } else {
            None
        }
    }

    pub fn as_int_unchecked(&self) -> i32 {
        ((self.0 << 32) >> 32) as i32
    }

    pub fn as_number(&self) -> Option<f64> {
        if self.is_number() {
            Some(unsafe { std::mem::transmute(self.0) })
        } else {
            None
        }
    }

    /// this operation is safe because f64 will be NaN if not number
    pub fn as_number_uncheck(&self) -> f64 {
        return f64::from_bits(self.0);
    }

    pub fn as_symbol(&self) -> Option<u32> {
        if self.is_symbol() {
            Some(((self.0 << 32) >> 32) as u32)
        } else {
            None
        }
    }

    //pub fn as_bigint32(&self) -> Option<i32>{
    //    if self.is_bigint32(){
    //        Some(((self.0 << 32) >> 32) as i32)
    //    } else{
    //        None
    //    }
    //}

    pub fn as_bigint(&self) -> Option<&'static mut JSBigInt> {
        if self.is_bigint() {
            let p = self.0 & Self::DATA_BITS;
            Some(unsafe { &mut *(p as usize as *mut JSBigInt) })
        } else {
            None
        }
    }

    /// convert self to i128 if value is bigint
    pub fn as_num_bigint(&self) -> Option<num_bigint::BigInt> {
        //if let Some(v) = self.as_bigint32(){
        //    return num_bigint::BigInt::from_i32(v)
        //}
        if let Some(v) = self.as_bigint() {
            return Some(v.value.clone());
        }
        None
    }

    pub fn as_object(&self) -> Option<JObject> {
        if self.is_object() {
            let ptr = (self.0 << 16) >> 16;
            Some(unsafe { &*(ptr as usize as *const JObjectInner) }.into())
        } else {
            None
        }
    }

    pub fn as_string(&self) -> Option<JSString> {
        if self.is_string() {
            let ptr = (self.0 << 16) >> 16;
            Some(JSString(ptr as *mut u8))
        } else {
            None
        }
    }

    pub fn typ(&self) -> JSType {
        if self.is_bigint() {
            //|| self.is_bigint32(){
            return JSType::Bigint;
        }
        if self.is_false() || self.is_true() {
            return JSType::Boolean;
        }
        if self.is_int() || self.is_number() {
            return JSType::Number;
        }
        if self.is_null() {
            return JSType::Null;
        }
        if self.is_symbol() {
            return JSType::Symbol;
        }
        if self.is_object() {
            return JSType::Object;
        }
        if self.is_string() {
            return JSType::String;
        }
        if self.is_undefined() {
            return JSType::Undefined;
        }
        panic!("unknown type {:#b}", self.0);
    }

    pub fn prototype(&self, rt:&Runtime) -> Option<JObject>{
        if let Some(obj) = self.as_object(){
            return obj.inner.__proto__
        }
        if self.is_bigint(){
            return Some(rt.prototypes.bigint)
        }
        if self.is_false() || self.is_true(){
            return Some(rt.prototypes.boolean)
        }
        if self.is_int() || self.is_number(){
            return Some(rt.prototypes.number)
        }
        if self.is_symbol(){
            return Some(rt.prototypes.symbol)
        }
        if self.is_string(){
            Some(rt.prototypes.string)
        } else{
            None
        }
    }

    pub unsafe fn trace(&self) {
        if let Some(obj) = self.as_object() {
            obj.trace();
        }
        if let Some(s) = self.as_string() {
            s.trace();
        }
    }
}

impl PartialEq for JValue {
    fn eq(&self, other: &Self) -> bool {
        if self.0 == self.0{
            return true
        }

        if self.0 >> 48 == other.0 >> 48 {
            return self.0 == other.0;
        }
        if self.is_int() && other.is_number() {
            return self.as_int_unchecked() as f64 == other.as_number_uncheck();
        }
        if other.is_int() && self.is_number() {
            return other.as_int_unchecked() as f64 == self.as_number_uncheck();
        }
        return false;
    }
}

impl Eq for JValue {}

// operators
impl JValue {
    pub fn add(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, "+")
    }

    pub fn sub(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, "-")
    }

    pub fn mul(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, "*")
    }

    pub fn div(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, "/")
    }

    pub fn rem(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, "%")
    }

    pub fn exp(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, "**")
    }

    pub fn rshift(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, ">>")
    }

    pub fn unsigned_rshift(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, ">>>")
    }

    pub fn lshift(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, "<<")
    }

    pub fn bitand(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, "&")
    }

    pub fn bitor(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, "|")
    }

    pub fn bitxor(self, other: Self, ctx: JSContext) -> Result<Self, Self> {
        Self::ApplyStringOrNumericBinaryOperator(self, other, ctx, "^")
    }

    // let the compiler optimize this part
    #[inline]
    #[allow(non_snake_case)]
    fn ApplyStringOrNumericBinaryOperator(
        self,
        other: Self,
        ctx: JSContext,
        op: &str,
    ) -> Result<Self, Self> {
        let lprim = self.to_primitive(ctx, None)?;
        let rprim = other.to_primitive(ctx, None)?;
        let mut lval = self;
        let mut rval = other;

        if op == "+" {
            if let Some(s) = lprim.as_string() {
                let s = s.to_string() + rprim.to_string().as_str();
                let s = ctx.runtime.allocate_string(&s);
                return Ok(Self::create_string(s));
            } else if let Some(s) = rprim.as_string() {
                let s = lprim.to_string() + s.as_str();
                let s = ctx.runtime.allocate_string(&s);
                return Ok(Self::create_string(s));
            } else {
                lval = lprim;
                rval = rprim;
            }
        };
        let mut lnum = lval.to_numeric(ctx)?;
        let mut rnum = rval.to_numeric(ctx)?;

        if lnum.typ() != rnum.typ() {
            return Err(Error::TypeError("cannot add bigint to number".into()).into());
        }

        if (lnum.is_bigint()/*|| lnum.is_bigint32()*/)
            && (rnum.is_bigint()/*|| rnum.is_bigint32()*/)
        {
            let lnum = lnum.as_num_bigint().unwrap();
            let rnum = rnum.as_num_bigint().unwrap();
            if op == "+" {
                let b = ctx.runtime.allocate_bigint();
                b.set_value(lnum + rnum);
                return Ok(Self::create_bigint_allocated(b));
            }
            if op == "-" {
                let b = ctx.runtime.allocate_bigint();
                b.set_value(lnum - rnum);
                return Ok(Self::create_bigint_allocated(b));
            }
            if op == "**" {
                let l = lnum;
                let (sign, r) = rnum.into_parts();

                if sign == num_bigint::Sign::Minus {
                    return Err(Error::RangeError(
                        "cannot exponentiate bigint with negative exponent".into(),
                    )
                    .into());
                }
                let re = num_traits::Pow::pow(&l, &r);
                let b = ctx.runtime.allocate_bigint();
                b.set_value(re);
                return Ok(Self::create_bigint_allocated(b));
            };
            if op == "*" {
                let b = ctx.runtime.allocate_bigint();
                b.set_value(lnum * rnum);
                return Ok(Self::create_bigint_allocated(b));
            }
            if op == "/" {
                let b = ctx.runtime.allocate_bigint();
                b.set_value(lnum / rnum);
                return Ok(Self::create_bigint_allocated(b));
            };
            if op == "%" {
                let b = ctx.runtime.allocate_bigint();
                b.set_value(lnum % rnum);
                return Ok(Self::create_bigint_allocated(b));
            }
            if op == ">>>" {
                let (sign, l) = lnum.into_parts();
                let (_rsign, r) = rnum.into_parts();
                let r = l >> r.to_u128().unwrap();

                let b = ctx.runtime.allocate_bigint();
                b.set_value(num_bigint::BigInt::from_biguint(sign, r));
                return Ok(Self::create_bigint_allocated(b));
            }

            if op == ">>" {}
            if op == "<<" {}
            if op == "&" {
                let b = ctx.runtime.allocate_bigint();
                b.set_value(lnum & rnum);
                return Ok(Self::create_bigint_allocated(b));
            }
            if op == "^" {
                let b = ctx.runtime.allocate_bigint();
                b.set_value(lnum ^ rnum);
                return Ok(Self::create_bigint_allocated(b));
            }
            if op == "|" {
                let b = ctx.runtime.allocate_bigint();
                b.set_value(lnum | rnum);
                return Ok(Self::create_bigint_allocated(b));
            }
        };

        if lnum.is_int() && rnum.is_int() {
            if op == "+" {
                return Ok(Self::create_int(
                    lnum.as_int_unchecked() + rnum.as_int_unchecked(),
                ));
            }
            if op == "-" {
                return Ok(Self::create_int(
                    lnum.as_int_unchecked() - rnum.as_int_unchecked(),
                ));
            }
            if op == "*" {
                return Ok(Self::create_int(
                    lnum.as_int_unchecked() * rnum.as_int_unchecked(),
                ));
            }

            //if op == "/"{
            //    return Ok(Self::create_int(lnum.as_int_unchecked() / rnum.as_int_unchecked()))
            //}
            if op == "%" {
                return Ok(Self::create_int(
                    lnum.as_int_unchecked() + rnum.as_int_unchecked(),
                ));
            }
            if op == ">>>" {
                return Ok(Self::create_int(
                    (lnum.as_int_unchecked() as u32 >> rnum.as_int_unchecked() as u32) as i32,
                ));
            }
            if op == ">>" {
                return Ok(Self::create_int(
                    lnum.as_int_unchecked() >> rnum.as_int_unchecked(),
                ));
            }
            if op == "<<" {
                return Ok(Self::create_int(
                    lnum.as_int_unchecked() << rnum.as_int_unchecked(),
                ));
            }
            if op == "&" {
                return Ok(Self::create_int(
                    lnum.as_int_unchecked() & rnum.as_int_unchecked(),
                ));
            }
            if op == "|" {
                return Ok(Self::create_int(
                    lnum.as_int_unchecked() | rnum.as_int_unchecked(),
                ));
            }
            if op == "^" {
                return Ok(Self::create_int(
                    lnum.as_int_unchecked() ^ rnum.as_int_unchecked(),
                ));
            }
        }

        if let Some(i) = lnum.as_int() {
            lnum = Self::create_number(i as f64);
        }
        if let Some(i) = rnum.as_int() {
            rnum = Self::create_number(i as f64);
        }

        let l = lnum.as_number_uncheck();
        let r = rnum.as_number_uncheck();

        if op == "+" {
            return Ok(Self::create_number(l + r));
        }
        if op == "-" {
            return Ok(Self::create_number(l - r));
        }
        if op == "*" {
            return Ok(Self::create_number(l * r));
        }
        if op == "/" {
            return Ok(Self::create_number(l / r));
        }
        if op == "%" {
            return Ok(Self::create_number(l % r));
        }
        if op == "**" {
            return Ok(Self::create_number(l.powf(r)));
        }
        if op == ">>>" {
            return Ok(Self::create_int(((l as u32) >> (r as u32)) as i32));
        }
        if op == ">>" {
            return Ok(Self::create_int((l as i32) >> (r as i32)));
        }
        if op == "<<" {
            return Ok(Self::create_int((l as i32) << (r as i32)));
        }
        if op == "&" {
            return Ok(Self::create_int((l as i32) & (r as i32)));
        }
        if op == "|" {
            return Ok(Self::create_int((l as i32) | (r as i32)));
        }
        if op == "^" {
            return Ok(Self::create_int((l as i32) ^ (r as i32)));
        }

        unsafe { core::hint::unreachable_unchecked() }
    }

    pub fn instance_of(self, target: Self, ctx: JSContext) -> Result<Self, Self> {
        if target.is_object() {
            let handler = target.get_method(SYMBOLS["hasInstance"], ctx)?;
            if !handler.is_undefined() {
                return Ok(handler.call(target, &[self], ctx)?.to_bool().into());
            }
            if !target.is_callable() {
                return Err(Error::TypeError(
                    "Right-hand side of 'instanceof' is not an callable".into(),
                )
                .into());
            }
            let p = target.get_property(NAMES["prototype"], ctx)?;
            let mut o = self;
            loop {
                if o.is_null() {
                    return Ok(false.into());
                }
                o = o.get_property(NAMES["__proto__"], ctx)?;
                if o == p {
                    return Ok(true.into());
                }
            }
        } else {
            Err(Error::TypeError("Right-hand side of 'instanceof' is not an object".into()).into())
        }
    }
}

impl ToString for JValue {
    fn to_string(&self) -> String {
        if let Some(s) = self.as_string() {
            s.to_string()
        } else if self.is_undefined() {
            "undefined".into()
        } else if self.is_null() {
            "null".into()
        } else if self.is_true() {
            "true".into()
        } else if self.is_false() {
            "false".into()
        } else if self.is_symbol() {
            "Symbol()".into()
        } else if let Some(i) = self.as_int() {
            i.to_string()
        } else if let Some(i) = self.as_number() {
            i.to_string()
        //} else if let Some(i) = self.as_bigint32(){
        //    i.to_string()
        } else if let Some(i) = self.as_bigint() {
            i.to_string()
        } else if self.is_object() {
            "[Object object]".into()
        } else {
            "unknown".into()
        }
    }
}

impl JValue {
    /// https://tc39.es/ecma262/#sec-toprimitive
    pub fn to_primitive(
        self,
        ctx: JSContext,
        preferred_type: Option<JSType>,
    ) -> Result<Self, Self> {
        if self.is_object() {
            let to_prim = self.get_method(SYMBOLS["toPrimitive"], ctx)?;

            if !to_prim.is_undefined() {
                let hint = if preferred_type.is_none() {
                    "default"
                } else if Some(JSType::String) == preferred_type {
                    "string"
                } else {
                    "number"
                };

                let re = to_prim.call(self, &[Self::create_static_string(hint)], ctx)?;
                if !re.is_object() {
                    return Ok(re);
                }
            } else {
                let names = ["valueOf", "toString"];
                for name in names {
                    let m = self.get_method(name, ctx)?;
                    if m.is_callable() {
                        let re = m.call(self, &[], ctx)?;
                        if !re.is_object() {
                            return Ok(re);
                        }
                    }
                }
            };
            return Err(
                Error::TypeError(format!("cannot convert [Object object] to primitive")).into(),
            );
        };
        return Ok(self);
    }

    pub fn to_bool(self) -> bool {
        if self.is_false() || self.is_null() || self.is_undefined() {
            return false;
        }
        if self.is_empty_string() {
            return false;
        }
        if let Some(f) = self.as_number() {
            if f == 0.0 || f == -0.0 || f.is_nan() {
                return false;
            }
        }
        return true;
    }

    pub fn to_numeric(self, ctx: JSContext) -> Result<Self, Self> {
        let prim = self.to_primitive(ctx, Some(JSType::Number))?;

        if prim.is_bigint() {
            //|| prim.is_bigint32(){
            return Ok(prim);
        }
        Ok(Self::create_number(self.to_number(ctx)?))
    }

    pub fn to_number(self, ctx: JSContext) -> Result<f64, Self> {
        if let Some(f) = self.as_number() {
            return Ok(f);
        }
        if self.is_symbol() {
            return Err(Error::TypeError("cannot convert symbol to primitive".into()).into());
        }
        if self.is_bigint() {
            //|| self.is_bigint32(){
            return Err(Error::TypeError("cannot convert bigint to number".into()).into());
        }
        if self.is_undefined() {
            return Ok(f64::NAN);
        }
        if self.is_null() || self.is_false() {
            return Ok(0.0);
        }
        if self.is_true() {
            return Ok(1.0);
        }
        if let Some(s) = self.as_string() {
            if let Ok(v) = fast_float::parse::<f64, _>(s.as_ref()) {
                return Ok(v);
            } else {
                return Ok(f64::NAN);
            }
        }
        debug_assert!(self.is_object());

        let prim = self.to_primitive(ctx, Some(JSType::Number))?;

        debug_assert!(!prim.is_object());

        prim.to_number(ctx)
    }

    pub fn to_integer_or_infinity(self, ctx: JSContext) -> Result<f64, Self> {
        let f = self.to_number(ctx)?;

        if f.is_nan() || f == 0.0 || f == -0.0 {
            return Ok(0.0);
        }
        if f == f64::INFINITY {
            return Ok(f64::INFINITY);
        }
        if f == f64::NEG_INFINITY {
            return Ok(f64::NEG_INFINITY);
        }
        let mut f = f.abs().floor();
        if f > 0.0 {
            f = -f;
        }
        return Ok(f);
    }

    pub fn to_i32(self, ctx: JSContext) -> Result<i32, JValue> {
        let f = self.to_number(ctx)?;

        if !f.is_finite() || f == 0.0 || f == -0.0 {
            return Ok(0);
        }
        return Ok(f.abs().floor() as i32);
    }

    pub fn to_u32(self, ctx: JSContext) -> Result<u32, JValue> {
        let f = self.to_number(ctx)?;

        if !f.is_finite() || f == 0.0 || f == -0.0 {
            return Ok(0);
        }
        return Ok(f.abs().floor() as u32);
    }

    pub fn to_i16(self, ctx: JSContext) -> Result<i16, JValue> {
        let f = self.to_number(ctx)?;

        if !f.is_finite() || f == 0.0 || f == -0.0 {
            return Ok(0);
        }
        return Ok(f.abs().floor() as i16);
    }

    pub fn to_u16(self, ctx: JSContext) -> Result<u16, JValue> {
        let f = self.to_number(ctx)?;

        if !f.is_finite() || f == 0.0 || f == -0.0 {
            return Ok(0);
        }
        return Ok(f.abs().floor() as u16);
    }

    pub fn to_i8(self, ctx: JSContext) -> Result<i8, JValue> {
        let f = self.to_number(ctx)?;

        if !f.is_finite() || f == 0.0 || f == -0.0 {
            return Ok(0);
        }
        return Ok(f.abs().floor() as i8);
    }

    pub fn to_u8(self, ctx: JSContext) -> Result<u8, JValue> {
        let f = self.to_number(ctx)?;

        if !f.is_finite() || f == 0.0 || f == -0.0 {
            return Ok(0);
        }
        return Ok(f.abs().floor() as u8);
    }

    pub fn to_u8_clamp(self, ctx: JSContext) -> Result<u8, JValue> {
        let f = self.to_number(ctx)?;

        if f.is_nan() {
            return Ok(0);
        }
        if f <= 0.0 {
            return Ok(0);
        }
        if f >= 255.0 {
            return Ok(255);
        }
        let num = f.floor();
        if num + 0.5 < f {
            return Ok((num + 1.0) as u8);
        }
        if f < num + 0.5 {
            return Ok(num as u8);
        }

        if num % 2.0 != 0.0 {
            return Ok((num + 1.0) as u8);
        }
        return Ok(num as u8);
    }

    pub fn to_bigint(self, ctx: JSContext) -> Result<Self, Self> {
        let prim = self.to_primitive(ctx, Some(JSType::Number))?;
        if prim.is_undefined() {
            return Err(Error::TypeError(format!("cannot convert undefined to bigint")).into());
        }
        if prim.is_null() {
            return Err(Error::TypeError(format!("cannot convert null to bigint")).into());
        }
        if prim.is_number() {
            return Err(Error::TypeError(format!("cannot convert number to bigint")).into());
        }
        if prim.is_symbol() {
            return Err(Error::TypeError(format!("cannot convert symbol to bigint")).into());
        }
        if prim.is_true() {
            return Ok(Self::create_bigint(1));
        }
        if prim.is_false() {
            return Ok(Self::create_bigint(0));
        }
        if let Some(s) = prim.as_string() {
            match s.parse::<i128>() {
                Ok(v) => return Ok(Self::create_bigint(v)),
                Err(e) => {
                    return Err(Error::SyntaxError(format!(
                        "cannot convert string to bigint: {}",
                        e.to_string()
                    ))
                    .into())
                }
            };
        }
        return Ok(prim);
    }

    pub fn to_bigint64(self, ctx: JSContext) -> Result<i64, Self> {
        let v = self.to_bigint(ctx)?;
        //if let Some(v) = v.as_bigint32(){
        //    return Ok(v as i64)
        //}
        let b = v.as_bigint().unwrap();
        Ok(b.to_i128() as i64)
    }

    pub fn to_biguint64(self, ctx: JSContext) -> Result<u64, Self> {
        let v = self.to_bigint(ctx)?;
        //if let Some(v) = v.as_bigint32(){
        //    return Ok(v as u64)
        //}
        let b = v.as_bigint().unwrap();
        Ok(b.to_i128() as u64)
    }

    pub fn to_jsstring(self, ctx: JSContext) -> Result<Self, Self> {
        if self.is_string() {
            Ok(self)
        } else if self.is_undefined() {
            Ok(Self::create_static_string("undefined"))
        } else if self.is_null() {
            Ok(Self::create_static_string("null"))
        } else if self.is_true() {
            Ok(Self::create_static_string("true"))
        } else if self.is_false() {
            Ok(Self::create_static_string("false"))
        } else if self.is_symbol() {
            return Err(Error::TypeError("cannot convert Symbol to string".into()).into());
        } else if let Some(i) = self.as_int() {
            Ok(Self::create_string(i.to_string().into()))
        } else if let Some(i) = self.as_number() {
            Ok(Self::create_string(i.to_string().into()))
        //} else if let Some(i) = self.as_bigint32(){
        //    Ok(Self::create_string(i.to_string().into()))
        } else if let Some(i) = self.as_bigint() {
            Ok(Self::create_string(i.to_string().into()))
        } else {
            debug_assert!(self.is_object());

            let prim = self.to_primitive(ctx, Some(JSType::String))?;

            debug_assert!(!prim.is_object());

            return prim.to_jsstring(ctx);
        }
    }

    pub fn to_object(self, ctx: JSContext) -> Result<JValue, JValue> {
        if self.is_null() || self.is_undefined() {
            return Err(
                Error::TypeError("cannot convert null or undefined to object".into()).into(),
            );
        }
        if self.is_object() {
            return Ok(self);
        }
        let obj = ctx.runtime.create_object();

        if let Some(s) = self.as_string() {
            obj.set_inner(JObjectValue::String(s));
            obj.inner.to_mut().__proto__ = Some(ctx.runtime.prototypes.string);
        }
        if let Some(b) = self.as_bigint() {
            obj.set_inner(JObjectValue::BigInt(b));
            obj.inner.to_mut().__proto__ = Some(ctx.runtime.prototypes.bigint);
        }
        if self.is_true() || self.is_false() {
            obj.set_inner(JObjectValue::Boolean(self.is_true()));
            obj.inner.to_mut().__proto__ = Some(ctx.runtime.prototypes.boolean);
        }
        if self.is_number() {
            obj.set_inner(JObjectValue::Number(self.as_number_uncheck()));
            obj.inner.to_mut().__proto__ = Some(ctx.runtime.prototypes.number);
        }
        if self.is_int() {
            obj.set_inner(JObjectValue::Number(self.as_int_unchecked() as f64));
            obj.inner.to_mut().__proto__ = Some(ctx.runtime.prototypes.number);
        }
        if let Some(sym) = self.as_symbol() {
            obj.set_inner(JObjectValue::Symbol(crate::JSymbol(sym)));
            obj.inner.to_mut().__proto__ = Some(ctx.runtime.prototypes.symbol);
        }
        return Ok(obj.into());
    }

    pub fn to_property_key(self, ctx: JSContext) -> Result<PropKey, JValue> {
        let key = self.to_primitive(ctx, Some(JSType::String))?;
        if let Some(sym) = key.as_symbol() {
            return Ok(PropKey(sym));
        } else {
            let s = key.to_jsstring(ctx)?.as_string().unwrap();
            let id = ctx.runtime.register_field_name(s.as_ref());
            Ok(PropKey(id))
        }
    }

    pub fn to_length(self, ctx: JSContext) -> Result<usize, Self> {
        let num = self.to_integer_or_infinity(ctx)?;
        if num <= 0.0 {
            return Ok(0);
        }
        return Ok((num as usize).min(2 ^ 53 - 1));
    }

    pub fn to_index(self, ctx: JSContext) -> Result<usize, Self> {
        if self.is_undefined() {
            return Ok(0);
        } else {
            let i = self.to_length(ctx)?;
            return Ok(i);
        }
    }

    pub fn is_array(self) -> bool {
        if let Some(obj) = self.as_object() {
            //if obj.is_proxy()
            return obj.is_array();
        } else {
            return false;
        }
    }

    pub fn is_callable(self) -> bool {
        if let Some(obj) = self.as_object() {
            return obj.is_function_instance() || obj.is_class() || obj.is_native_function();
        }
        return false;
    }

    pub fn is_constructor(self) -> bool {
        if let Some(obj) = self.as_object() {
            if obj.is_class() {
                return true;
            }

            if obj.is_function_instance() {
                return true;
            }
        }
        return false;
    }

    pub fn is_extensible(self) -> bool {
        if let Some(obj) = self.as_object() {
            return obj.is_extensible();
        }
        return false;
    }

    pub fn is_integral_number(self) -> bool {
        if self.is_int() {
            return true;
        }

        if let Some(f) = self.as_number() {
            if !f.is_finite() {
                return false;
            }
            if f.abs().floor() != f.abs() {
                return false;
            }
            return true;
        }

        return false;
    }

    pub fn is_property_key(self) -> bool {
        return self.is_string() || self.is_symbol();
    }

    pub fn same_value(self, other: Self) -> bool {
        self == other
    }

    pub fn is_loosely_equal(self, rhs: Self, ctx: JSContext) -> Result<bool, JValue> {
        if self.typ() == rhs.typ() {
            return Ok(self == rhs);
        }
        if (self.is_undefined() || self.is_null()) && (rhs.is_null() || rhs.is_undefined()) {
            return Ok(true);
        }

        if self.is_number() && rhs.is_string() {
            return self.eqeq(rhs.to_number(ctx)?.into(), ctx);
        }
        if rhs.is_number() && self.is_string() {
            return JValue::create_number(self.to_number(ctx)?).eqeq(rhs, ctx);
        }
        if (self.is_bigint()/*|| self.is_bigint32()*/) && rhs.is_string() {
            let s = rhs.as_string().unwrap();
            if let Ok(v) = s.as_str().parse::<i128>() {
                //if let Some(b) = self.as_bigint32(){
                //    return Ok(b as i128 == v)
                //}
                if let Some(b) = self.as_bigint() {
                    return Ok(b.to_i128() == v);
                }
            } else {
                return Ok(false);
            }
        }

        if self.is_string() && (rhs.is_bigint()/*|| rhs.is_bigint32()*/) {
            return rhs.eqeq(self, ctx);
        }
        if self.is_true() {
            return JValue::create_number(1.0).eqeq(rhs, ctx);
        }
        if self.is_false() {
            return JValue::create_number(0.0).eqeq(rhs, ctx);
        }
        if rhs.is_true() {
            return self.eqeq(JValue::create_number(1.0), ctx);
        }
        if rhs.is_false() {
            return self.eqeq(JValue::create_number(0.0), ctx);
        }

        if (self.is_string() || self.is_bigint() /*|| self.is_bigint32()*/ || self.is_number() || self.is_int() || self.is_symbol())
            && rhs.is_object()
        {
            return self.eqeq(rhs.to_primitive(ctx, None)?, ctx);
        }
        if (rhs.is_string() || rhs.is_bigint() /*|| rhs.is_bigint32()*/ || rhs.is_number() || rhs.is_int() || rhs.is_symbol())
            && self.is_object()
        {
            return self.to_primitive(ctx, None)?.eqeq(rhs, ctx);
        }

        if let Some(b) = rhs.as_bigint() {
            if let Some(f) = self.as_int() {
                return Ok(b.to_i128() == f as i128);
            }
            if let Some(f) = self.as_number() {
                if f.is_finite() {
                    return Ok(b.to_i128() == f as i128);
                }
            }
        }
        if let Some(b) = self.as_bigint() {
            if let Some(f) = rhs.as_int() {
                return Ok(b.to_i128() == f as i128);
            }
            if let Some(f) = rhs.as_number() {
                if f.is_finite() {
                    return Ok(b.to_i128() == f as i128);
                }
            }
        }

        return Ok(false);
    }

    pub fn eqeq(self, rhs: Self, ctx: JSContext) -> Result<bool, Self> {
        self.is_loosely_equal(rhs, ctx)
    }

    pub fn get_property<K: ToProperyKey>(self, key: K, ctx: JSContext) -> Result<Self, Self> {
        if let Some(obj) = self.as_object() {
            return obj.get_property(key, ctx);
        }
        if self.is_undefined() {
            let key = key.to_key(&ctx.runtime);
            let name = ctx.runtime.get_field_name(key.0);
            return Err(Error::TypeError(format!(
                "Cannot read properties of undefined (reading '{}')",
                name
            ))
            .into());
        }
        if self.is_null() {
            let key = key.to_key(&ctx.runtime);
            let name = ctx.runtime.get_field_name(key.0);
            return Err(Error::TypeError(format!(
                "Cannot read properties of null (reading '{}')",
                name
            ))
            .into());
        }
        if self.is_bigint() {
            //|| self.is_bigint32(){
            return ctx.runtime.prototypes.bigint.get_property(key, ctx);
        }
        if self.is_false() || self.is_true() {
            return ctx.runtime.prototypes.boolean.get_property(key, ctx);
        }
        if self.is_int() || self.is_number() {
            return ctx.runtime.prototypes.number.get_property(key, ctx);
        }
        if self.is_string() {
            return ctx.runtime.prototypes.string.get_property(key, ctx);
        }
        if self.is_symbol() {
            return ctx.runtime.prototypes.symbol.get_property(key, ctx);
        } else {
            #[cfg(not(test))]
            unsafe {
                core::hint::unreachable_unchecked()
            };

            #[cfg(test)]
            {
                // unknown value
                let key = key.to_key(&ctx.runtime);
                let name = ctx.runtime.get_field_name(key.0);
                return Err(Error::TypeError(format!(
                    "Cannot read properties of unknown (reading '{}')",
                    name
                ))
                .into());
            }
        }
    }

    pub fn set_property<K: ToProperyKey>(
        self,
        key: K,
        value: Self,
        ctx: JSContext,
    ) -> Result<(), Self> {
        if let Some(obj) = self.as_object() {
            return obj.set_property(key, value, ctx);
        }
        if self.is_undefined() {
            let key = key.to_key(&ctx.runtime);
            let name = ctx.runtime.get_field_name(key.0);
            return Err(Error::TypeError(format!(
                "Cannot set properties of undefined (setting '{}')",
                name
            ))
            .into());
        }
        if self.is_null() {
            let key = key.to_key(&ctx.runtime);
            let name = ctx.runtime.get_field_name(key.0);
            return Err(Error::TypeError(format!(
                "Cannot set properties of null (setting '{}')",
                name
            ))
            .into());
        }

        Ok(())
    }

    pub fn get_method<K: ToProperyKey>(self, key: K, ctx: JSContext) -> Result<Self, Self> {
        let func = self.get_property(key, ctx)?;
        if func.is_null() || func.is_undefined() {
            return Ok(Self::UNDEFINED);
        }
        if !func.is_callable() {
            return Err(Error::ExpectedFunction.into());
        }
        return Ok(func);
    }

    #[inline]
    pub fn call(self, this: Self, args: &[Self], ctx: JSContext) -> Result<Self, Self> {
        if !self.is_callable() {
            return Err(Error::CallOnNonFunction.into());
        }
        if let Some(obj) = self.as_object() {
            unsafe { std::ptr::copy_nonoverlapping(args.as_ptr(), ctx.stack, args.len()) };
            let (v, err) = obj.call(&ctx.runtime, this, ctx.stack, args.len());
            if err {
                return Err(v);
            }
            Ok(v)
        } else {
            return Err(Error::CallOnNonFunction.into());
        }
    }
}

/*
#[derive(Clone, Copy)]
#[repr(C)]
pub struct JValue {
    pub(crate) value: JValueUnion,
    pub(crate) type_pointer: &'static JTypeVtable,
}

unsafe impl Sync for JValue {}
unsafe impl Send for JValue {}

impl std::hash::Hash for JValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.is_string() {
            unsafe { self.value.string.as_bytes().hash(state) }
        } else {
            state.write_usize(unsafe { std::mem::transmute(self.value) });
            state.write_usize(self.type_pointer as *const _ as usize);
        }
    }
}

impl std::fmt::Debug for JValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            if self.is_bigint() {
                f.write_fmt(format_args!("BigInt({})", self.value.bigint))
            } else if self.is_bool() {
                f.write_fmt(format_args!("Boolean({})", self.is_true()))
            } else if self.is_null() {
                f.write_str("null")
            } else if self.is_number() {
                f.write_fmt(format_args!("Number({})", self.value.number))
            } else if self.is_object() {
                f.write_str("[object Object]")
            } else if self.is_string() {
                f.write_fmt(format_args!("String({})", self.value.string.as_ref()))
            } else if self.is_symbol() {
                f.write_str("Symbol")
            } else if self.is_undefined() {
                f.write_str("undefined")
            } else {
                f.write_str("unknown")
            }
        }
    }
}

#[cfg(target_pointer_width = "64")]
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) union JValueUnion {
    pub number: f64,
    pub bigint: i64,
    pub string: JSString,
    pub symbol: JSymbol,
    pub object: JObject,

    pub null: u64,
    pub undefined: u64,
    pub real_undefined: u64,
    pub true_: u64,
    pub false_: u64,
}

#[cfg(target_pointer_width = "32")]
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) union JValueUnion {
    pub number: f32,
    pub bigint: i32,
    pub string: JSString,
    pub symbol: JSymbol,
    pub object: JObject,

    pub null: u32,
    pub undefined: u32,
    pub real_undefined: u32,
    pub true_: u32,
    pub false_: u32,
}

/*
pub trait JSValuable{
    unsafe fn add(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn sub(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn mul(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn div(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn rem(value:JValueUnion, rhs:JValue) -> (JValue, bool);

    unsafe fn lshift(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn rshift(value:JValueUnion, rhs:JValue) -> (JValue, bool);

    unsafe fn gt(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn gteq(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn lt(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn lteq(value:JValueUnion, rhs:JValue) -> (JValue, bool);

    unsafe fn instance_of(value:JValueUnion, rhs:JValue) -> (JValue, bool);
    unsafe fn In(value:JValueUnion, rhs:JValue) -> (JValue, bool);

    unsafe fn set(obj:JValueUnion, key:JValue, value:JValue) -> (JValue, bool);
    unsafe fn set_static(obj:JValueUnion, key:u32, value:JValue) -> (JValue, bool);
    unsafe fn get(obj:JValueUnion, key:JValue) -> (JValue, bool);
    unsafe fn get_static(obj:JValueUnion, key:u32) -> (JValue, bool);
    unsafe fn remove_key_static(obj:JValueUnion, key:u32);
}
*/

#[derive(Debug)]
#[repr(C)]
pub struct JTypeVtable {
    pub tag: u8,
    add: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    sub: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    mul: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    div: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    rem: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    exp: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    eqeq: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    noteq: unsafe fn(JValueUnion, JValue) -> (JValue, bool),

    set: unsafe fn(JValueUnion, JValue, JValue, *mut JValue) -> (JValue, bool),
    set_static: unsafe fn(JValueUnion, u32, JValue, *mut JValue) -> (JValue, bool),
    get: unsafe fn(JValueUnion, JValue, *mut JValue) -> (JValue, bool),
    get_static: unsafe fn(JValueUnion, u32, *mut JValue) -> (JValue, bool),
    remove_key_static: unsafe fn(JValueUnion, u32),

    gt: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    gteq: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    lt: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    lteq: unsafe fn(JValueUnion, JValue) -> (JValue, bool),

    instance_of: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
    /// fn (obj, field) -> (result, error)
    In: unsafe fn(JValueUnion, JValue) -> (JValue, bool),
}

impl JTypeVtable {
    const T: Self = NULL_TYPE_POINTER;

    pub const TRUE: Self = TRUE_TYPE_POINTER;
    pub const FALSE: Self = FALSE_TYPE_POINTER;
    pub const NULL: Self = NULL_TYPE_POINTER;
    pub const UNDEFINED: Self = UNDEFINED_TYPE_POINTER;
    pub const BIGINT: Self = BIGINT_TYPE_POINTER;
    pub const NUMBER: Self = NUMBER_TYPE_POINTER;
    pub const STRING: Self = STRING_TYPE_POINTER;
    pub const SYMBOL: Self = SYMBOL_TYPE_POINTER;
    pub const OBJECT: Self = OBJECT_TYPE_POINTER;

    pub const VTABLES: &'static [Self] = &[
        Self::NULL,
        Self::UNDEFINED,
        Self::TRUE,
        Self::FALSE,
        Self::NUMBER,
        Self::BIGINT,
        Self::STRING,
        Self::SYMBOL,
        Self::OBJECT,
    ];

    // only string and object can be spread
    pub const SPREAD_OBJECT: Self = JTypeVtable {
        tag: 10,
        ..Self::OBJECT
    };
    pub const SPREAD_STRING: Self = JTypeVtable {
        tag: 11,
        ..Self::STRING
    };

    pub fn offset_add() -> i32 {
        (&Self::T.add as *const _ as usize - &Self::T as *const _ as usize) as i32
    }
    pub fn offset_sub() -> i32 {
        (&Self::T.sub as *const _ as usize - &Self::T as *const _ as usize) as i32
    }
    pub fn offset_mul() -> i32 {
        (&Self::T.mul as *const _ as usize - &Self::T as *const _ as usize) as i32
    }
    pub fn offset_div() -> i32 {
        (&Self::T.div as *const _ as usize - &Self::T as *const _ as usize) as i32
    }
    pub fn offset_rem() -> i32 {
        (&Self::T.rem as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_exp() -> i32 {
        (&Self::T.exp as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_eqeq() -> i32 {
        (&Self::T.eqeq as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_noteq() -> i32 {
        (&Self::T.noteq as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_set() -> i32 {
        (&Self::T.set as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_set_static() -> i32 {
        (&Self::T.set_static as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_get() -> i32 {
        (&Self::T.get as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_get_static() -> i32 {
        (&Self::T.get_static as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_remove_key_static() -> i32 {
        (&Self::T.remove_key_static as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_gt() -> i32 {
        (&Self::T.gt as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_gteq() -> i32 {
        (&Self::T.gteq as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_lt() -> i32 {
        (&Self::T.lt as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_lteq() -> i32 {
        (&Self::T.lteq as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_instance_of() -> i32 {
        (&Self::T.instance_of as *const _ as usize - &Self::T as *const _ as usize) as i32
    }

    pub fn offset_In() -> i32 {
        (&Self::T.In as *const _ as usize - &Self::T as *const _ as usize) as i32
    }
}

const NULL_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: null::add,
    sub: null::sub,
    mul: null::mul,
    div: null::div,
    rem: null::rem,
    exp: null::exp,
    eqeq: null::eqeq,
    noteq: null::noteq,
    gt: null::gt,
    gteq: null::gteq,
    lt: null::lt,
    lteq: null::lteq,

    get: not_object::get,
    get_static: not_object::get_static,
    set: not_object::set,
    set_static: not_object::set_static,
    remove_key_static: not_object::remove_key_static,
    instance_of: not_object::instance_of,
    In: not_object::In,

    tag: 0,
};

const UNDEFINED_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: undefined::add,
    sub: undefined::sub,
    mul: undefined::mul,
    div: undefined::div,
    rem: undefined::rem,
    exp: undefined::exp,
    eqeq: undefined::eqeq,
    noteq: undefined::noteq,
    gt: undefined::gt,
    gteq: undefined::gteq,
    lt: undefined::lt,
    lteq: undefined::lteq,

    get: not_object::get,
    get_static: not_object::get_static,
    set: not_object::set,
    set_static: not_object::set_static,
    remove_key_static: not_object::remove_key_static,
    instance_of: not_object::instance_of,
    In: not_object::In,

    tag: 1,
};

const TRUE_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: true_::add,
    sub: true_::sub,
    mul: true_::mul,
    div: true_::div,
    rem: true_::rem,
    exp: true_::exp,
    eqeq: true_::eqeq,
    noteq: true_::noteq,
    gt: true_::gt,
    gteq: true_::gteq,
    lt: true_::lt,
    lteq: true_::lteq,

    get: not_object::get,
    get_static: not_object::get_static,
    set: not_object::set,
    set_static: not_object::set_static,
    remove_key_static: not_object::remove_key_static,
    instance_of: not_object::instance_of,
    In: not_object::In,

    tag: 2,
};

const FALSE_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: false_::add,
    sub: false_::sub,
    mul: false_::mul,
    div: false_::div,
    rem: false_::rem,
    exp: false_::exp,
    eqeq: false_::eqeq,
    noteq: false_::noteq,
    gt: false_::gt,
    gteq: false_::gteq,
    lt: false_::lt,
    lteq: false_::lteq,

    get: not_object::get,
    get_static: not_object::get_static,
    set: not_object::set,
    set_static: not_object::set_static,
    remove_key_static: not_object::remove_key_static,
    instance_of: not_object::instance_of,
    In: not_object::In,

    tag: 3,
};

const NUMBER_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: number::add,
    sub: number::sub,
    mul: number::mul,
    div: number::div,
    rem: number::rem,
    exp: number::exp,
    eqeq: number::eqeq,
    noteq: number::noteq,
    gt: number::gt,
    gteq: number::gteq,
    lt: number::lt,
    lteq: number::lteq,

    get: number::get,
    get_static: number::get_static,
    set: not_object::set,
    set_static: not_object::set_static,
    remove_key_static: not_object::remove_key_static,
    instance_of: not_object::instance_of,
    In: not_object::In,

    tag: 4,
};

const BIGINT_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: bigint::add,
    sub: bigint::sub,
    mul: bigint::mul,
    div: bigint::div,
    rem: bigint::rem,
    exp: bigint::exp,
    eqeq: bigint::eqeq,
    noteq: bigint::noteq,
    gt: bigint::gt,
    gteq: bigint::gteq,
    lt: bigint::lt,
    lteq: bigint::lteq,

    get: not_object::get,
    get_static: not_object::get_static,
    set: not_object::set,
    set_static: not_object::set_static,
    remove_key_static: not_object::remove_key_static,
    instance_of: not_object::instance_of,
    In: not_object::In,

    tag: 5,
};

const STRING_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: string::add,
    sub: string::sub,
    mul: string::mul,
    div: string::div,
    rem: string::rem,
    exp: string::exp,
    eqeq: string::eqeq,
    noteq: string::noteq,
    gt: string::gt,
    gteq: string::gteq,
    lt: string::lt,
    lteq: string::lteq,

    get: string::get,
    get_static: string::get_static,
    set: string::set,
    set_static: string::set_static,
    remove_key_static: string::remove_key_static,

    instance_of: not_object::instance_of,
    In: not_object::In,

    tag: 6,
};

const SYMBOL_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: symbol::throw,
    sub: symbol::throw,
    mul: symbol::throw,
    div: symbol::throw,
    rem: symbol::throw,
    eqeq: symbol::throw,
    noteq: symbol::throw,
    exp: symbol::throw,
    gt: symbol::throw,
    gteq: symbol::throw,
    lt: symbol::throw,
    lteq: symbol::throw,

    get: not_object::get,
    get_static: not_object::get_static,
    set: not_object::set,
    set_static: not_object::set_static,
    remove_key_static: not_object::remove_key_static,
    instance_of: not_object::instance_of,
    In: not_object::In,

    tag: 7,
};

const OBJECT_TYPE_POINTER: JTypeVtable = JTypeVtable {
    add: object::add,
    sub: object::sub,
    mul: object::mul,
    div: object::div,
    rem: object::rem,
    exp: object::exp,
    eqeq: object::eqeq,
    noteq: object::noteq,
    get: object::get,
    get_static: object::get_static,
    set: object::set,
    set_static: object::set_static,
    remove_key_static: object::remove_key_static,

    gt: object::gt,
    gteq: object::gteq,
    lt: object::lt,
    lteq: object::lteq,

    instance_of: object::instance_of,
    In: object::In,

    tag: 8,
};

#[allow(non_snake_case)]
impl JValue {
    pub const SIZE: usize = std::mem::size_of::<Self>();
    pub const VALUE_SIZE: usize = std::mem::size_of::<JValueUnion>();
    pub const VTABLE_SIZE: usize = std::mem::size_of::<*const JTypeVtable>();

    pub const NULL: JValue = JValue {
        value: JValueUnion { null: 0 },
        type_pointer: &NULL_TYPE_POINTER,
    };

    pub const TRUE: JValue = JValue {
        value: JValueUnion { true_: 1 },
        type_pointer: &TRUE_TYPE_POINTER,
    };

    pub const FALSE: JValue = JValue {
        value: JValueUnion { false_: 0 },
        type_pointer: &FALSE_TYPE_POINTER,
    };

    pub const UNDEFINED: JValue = JValue {
        value: JValueUnion { undefined: 0 },
        type_pointer: &UNDEFINED_TYPE_POINTER,
    };

    /*
    pub const REAL_UNDEFINED:JValue = JValue{
        value:JValueUnion { real_undefined: 0 },
        type_pointer:&REAL_UNDEFINED_TYPE_POINTER
    };
    */

    pub fn is_null(&self) -> bool {
        return self.type_pointer.tag == NULL_TYPE_POINTER.tag;
    }

    pub fn is_undefined(&self) -> bool {
        return self.type_pointer.tag == UNDEFINED_TYPE_POINTER.tag;
    }

    //pub fn is_real_undefined(&self) -> bool{
    //return self.type_pointer as *const _ == &REAL_UNDEFINED_TYPE_POINTER
    //}

    pub fn is_bool(&self) -> bool {
        return self.type_pointer.tag == TRUE_TYPE_POINTER.tag
            || self.type_pointer.tag == FALSE_TYPE_POINTER.tag;
    }

    pub fn is_true(&self) -> bool {
        return self.type_pointer.tag == TRUE_TYPE_POINTER.tag;
    }

    pub fn is_false(&self) -> bool {
        return self.type_pointer.tag == FALSE_TYPE_POINTER.tag;
    }

    pub fn is_number(&self) -> bool {
        return self.type_pointer.tag == NUMBER_TYPE_POINTER.tag;
    }

    pub fn is_bigint(&self) -> bool {
        return self.type_pointer.tag == BIGINT_TYPE_POINTER.tag;
    }

    pub fn is_string(&self) -> bool {
        return self.type_pointer.tag == STRING_TYPE_POINTER.tag
            || self.type_pointer.tag == JTypeVtable::SPREAD_STRING.tag;
    }

    pub fn is_symbol(&self) -> bool {
        return self.type_pointer.tag == SYMBOL_TYPE_POINTER.tag;
    }

    pub fn is_object(&self) -> bool {
        return self.type_pointer.tag == OBJECT_TYPE_POINTER.tag;
        //|| self.type_pointer.tag == JTypeVtable::SPREAD_OBJECT.tag;
    }

    pub fn is_new_target(&self) -> bool {
        if self.is_object() {
            unsafe { self.value.object.is_new_target() }
        } else {
            false
        }
    }

    pub fn Number(n: f64) -> JValue {
        return JValue {
            value: JValueUnion { number: n },
            type_pointer: &NUMBER_TYPE_POINTER,
        };
    }

    pub fn BigInt(n: i64) -> JValue {
        return JValue {
            value: JValueUnion { bigint: n },
            type_pointer: &BIGINT_TYPE_POINTER,
        };
    }

    pub fn String(s: JSString) -> JValue {
        return JValue {
            value: JValueUnion { string: s },
            type_pointer: &STRING_TYPE_POINTER,
        };
    }

    pub fn Object(o: JObject) -> JValue {
        return JValue {
            value: JValueUnion { object: o },
            type_pointer: &OBJECT_TYPE_POINTER,
        };
    }

    pub fn Error(e: Error) -> JValue {
        let obj = JObject::with_error(e);
        return JValue {
            value: JValueUnion { object: obj },
            type_pointer: &OBJECT_TYPE_POINTER,
        };
    }

    pub fn as_object(&self) -> Option<&JObject> {
        if self.is_object() {
            Some(unsafe { &self.value.object })
        } else {
            None
        }
    }

    pub fn as_number_uncheck(&self) -> f64 {
        unsafe { self.value.number }
    }

    pub fn as_string<'a>(&'a self) -> Option<&'a JSString> {
        if self.is_string() {
            Some(unsafe { &self.value.string })
        } else {
            None
        }
    }

    pub fn as_promise<'a>(&'a self) -> Option<&'a mut crate::bultins::promise::Promise> {
        if self.is_object() {
            match unsafe { &mut self.value.object.inner.to_mut().wrapped_value } {
                crate::bultins::object::JObjectValue::Promise(p) => Some(p.as_mut()),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn to_bool(self) -> bool {
        if unsafe { std::mem::transmute::<_, u64>(self.value) } == 0 {
            false
        } else {
            true
        }
    }

    pub fn to_number(self) -> f64 {
        if self.is_number() {
            unsafe { self.value.number }
        } else if self.is_bigint() {
            unsafe { self.value.bigint as f64 }
        } else if self.is_true() {
            1.0
        } else if self.is_false() {
            0.0
        } else if self.is_null() {
            0.0
        } else if self.is_object() {
            if let Some(f) = unsafe { self.value.object.inner.wrapped_value.number() } {
                f
            } else if let Some(s) = unsafe { self.value.object.inner.wrapped_value.string() } {
                if let Ok(v) = fast_float::parse(s.as_str()) {
                    v
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else if self.is_string() {
            if unsafe { self.value.string.len() } == 0 {
                return 0.0;
            }
            if let Ok(v) = unsafe { fast_float::parse(self.value.string.as_str()) } {
                v
            } else {
                0.0
            }
        } else if self.is_symbol() {
            0.0
        } else {
            0.0
        }
    }

    pub fn to_i32(self) -> i32 {
        self.to_number() as i32
    }

    pub fn bitnot(self) -> Self {
        JValue::create_number((!self.to_i32()) as f64)
    }

    pub fn zerofillRshift(self, rhs: Self) -> Self {
        JValue::create_number(((self.to_i32() as u32) << (rhs.to_i32() as u32)) as f64)
    }

    /// * wait for a value, block until future is fulfilled
    /// * return immediately if value is not future
    pub(crate) fn wait(self) -> (Self, bool) {
        if let Some(p) = self.as_promise() {
            match p {
                Promise::Fulfilled(f) => (*f, false),
                Promise::Rejected(v) => (*v, true),
                Promise::Pending { id } => {
                    let runtime = Runtime::current();
                    loop {
                        let re = runtime.to_mut().poll_async(*id);
                        match re {
                            std::task::Poll::Pending => {}
                            std::task::Poll::Ready(re) => match re {
                                Ok(v) => return (v, false),
                                Err(e) => return (e, true),
                            },
                        }
                    }
                }
                Promise::ForeverPending => {
                    (JValue::from(Error::AwaitOnForeverPendingPromise), true)
                }
            }
        } else {
            return (self, false);
        }
    }

    pub unsafe fn private_in(self, name: u32) -> JValue {
        if self.is_object() {
            self.value.object.has_owned_property_static(name).into()
        } else {
            JValue::FALSE
        }
    }

    pub unsafe fn add_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.add)(self.value, rhs)
    }

    pub unsafe fn sub_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.sub)(self.value, rhs)
    }

    pub unsafe fn mul_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.mul)(self.value, rhs)
    }

    pub unsafe fn div_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.div)(self.value, rhs)
    }

    pub unsafe fn rem_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.rem)(self.value, rhs)
    }

    pub unsafe fn exp_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.exp)(self.value, rhs)
    }

    pub unsafe fn eqeq_(self, rhs: Self) -> Self {
        (self.type_pointer.eqeq)(self.value, rhs).0
    }

    pub unsafe fn gt_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.gt)(self.value, rhs)
    }

    pub unsafe fn gteq_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.gteq)(self.value, rhs)
    }

    pub unsafe fn lt_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.lt)(self.value, rhs)
    }

    pub unsafe fn lteq_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.lteq)(self.value, rhs)
    }

    pub unsafe fn In_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.In)(self.value, rhs)
    }

    pub unsafe fn instance_of_(self, rhs: Self) -> (Self, bool) {
        (self.type_pointer.instance_of)(self.value, rhs)
    }

    pub unsafe fn remove_key_static_(self, id: u32) {
        (self.type_pointer.remove_key_static)(self.value, id);
    }

    pub fn type_str(self) -> &'static str {
        if self.is_bigint() {
            "bigint"
        } else if self.is_bool() {
            "boolean"
        } else if self.is_number() {
            "number"
        } else if self.is_string() {
            "string"
        } else if self.is_symbol() {
            "symbol"
        } else if self.is_undefined() {
            "undefined"
        } else if self.is_object() {
            "object"
        } else {
            "unknown"
        }
    }

    /// called by the jitted code
    pub unsafe fn call_(
        self,
        runtime: &Runtime,
        this: JValue,
        argv: *const TempAllocValue,
        argc: u32,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        let mut len = 0;
        let args = std::slice::from_raw_parts(argv, argc as usize);

        for i in args {
            // spread
            if i.flag == 1 {
                let iter = FastIterator::new(i.value, crate::bytecodes::LoopHint::ForOf);

                loop {
                    let (done, error, value) = iter.next(this, stack);
                    if error {
                        (value, error);
                    }
                    let ptr = (stack as *mut JValue).add(len as usize);
                    *ptr = value;
                    len += 1;
                    if done {
                        break;
                    }
                }
                iter.drop_();
            } else {
                let ptr = (stack as *mut JValue).add(len as usize);
                *ptr = i.value;
                len += 1;
            }
        }
        self.call_raw(runtime, this, stack, len)
    }

    pub unsafe fn call_raw(
        self,
        runtime: &Runtime,
        this: JValue,
        stack: *mut JValue,
        argc: usize,
    ) -> (JValue, bool) {
        if self.is_object() {
            self.value.object.call(runtime, this, stack, argc)
        } else {
            (JValue::from(Error::CallOnNonFunction), true)
        }
    }

    pub fn call(
        self,
        ctx: JSContext,
        this: JValue,
        args: &[JValue],
    ) -> Result<JValue, JValue> {
        if !self.is_object() {
            return Err(JValue::from(Error::TypeError(format!(
                "cannot call on non function, got {}, value {}",
                self.type_str(),
                unsafe { self.value.null }
            ))));
        }
        let (result, error) = unsafe {
            std::ptr::copy(args.as_ptr(), ctx.stack as *mut JValue, args.len());
            self.call_raw(&ctx.runtime, this, ctx.stack, args.len())
        };
        if error {
            Err(result)
        } else {
            Ok(result)
        }
    }

    pub fn get_property(self, field: JValue) -> Result<JValue, JValue> {
        if field.is_string() {
            let s = unsafe { field.value.string.as_str() };
            self.get_property_str(s)
        } else {
            let s = field.to_string();
            self.get_property_str(&s)
        }
    }

    pub fn get_property_str(self, field: &str) -> Result<JValue, JValue> {
        let rt = Runtime::current();
        let id = rt.register_field_name(field);
        let (result, error) = self.get_property_static_(id);
        if error {
            Err(result)
        } else {
            Ok(result)
        }
    }

    pub(crate) fn get_property_static_(self, field_id: u32) -> (JValue, bool) {
        let mut stack = Vec::with_capacity(128);
        unsafe { (self.type_pointer.get_static)(self.value, field_id, stack.as_mut_ptr()) }
    }

    pub fn get_property_raw(self, field_id: u32, stack: *mut JValue) -> (JValue, bool) {
        unsafe { (self.type_pointer.get_static)(self.value, field_id, stack) }
    }

    pub fn set_property(self, field: JValue, value: JValue) -> Result<(), JValue> {
        if field.is_string() {
            let s = unsafe { field.value.string.as_str() };
            self.set_property_str(s, value)
        } else {
            let s = field.to_string();
            self.set_property_str(&s, value)
        }
    }

    pub fn set_property_str(self, key: &str, value: JValue) -> Result<(), JValue> {
        let runtime = Runtime::current();
        let id = runtime.register_field_name(key);

        self.set_property_static(id, value)
    }

    pub fn set_property_static(self, field_id: u32, value: JValue) -> Result<(), JValue> {
        let mut stack = Vec::with_capacity(128);
        let (v, err) = unsafe {
            (self.type_pointer.set_static)(self.value, field_id, value, stack.as_mut_ptr())
        };
        if err {
            Err(v)
        } else {
            Ok(())
        }
    }

    pub fn set_property_raw(
        self,
        field_id: u32,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        unsafe { (self.type_pointer.set_static)(self.value, field_id, value, stack) }
    }

    pub fn to_object(self) -> JObject {
        use crate::bultins::object::JObjectValue;

        if self.is_object() {
            return unsafe { self.value.object };
        }

        let obj = JObject::new();
        unsafe {
            if self.is_string() {
                obj.set_inner(JObjectValue::String(self.value.string));
            }
            if self.is_bigint() {
                obj.set_inner(JObjectValue::BigInt(self.value.bigint));
            }
            if self.is_bool() {
                obj.set_inner(JObjectValue::Boolean(self.is_true()));
            }
            if self.is_number() {
                obj.set_inner(JObjectValue::Number(self.value.number));
            }

            if self.is_symbol() {
                obj.set_inner(JObjectValue::Symbol(self.value.symbol));
            }
        }
        return obj;
    }

    pub(crate) unsafe fn trace(self) {
        if self.is_object() {
            self.value.object.trace();
        } else if self.is_string() {
            self.value.string.trace();
        }
    }
}

impl std::ops::Add for JValue {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        unsafe {
            if self.is_bigint() {
                bigint::add(self.value, rhs)
            } else if self.is_true() {
                true_::add(self.value, rhs)
            } else if self.is_false() {
                false_::add(self.value, rhs)
            } else if self.is_null() {
                null::add(self.value, rhs)
            } else if self.is_number() {
                number::add(self.value, rhs)
            } else if self.is_object() {
                object::add(self.value, rhs)
            } else if self.is_string() {
                string::add(self.value, rhs)
            } else if self.is_symbol() {
                todo!("throw when add symbol")
            } else {
                // undefined
                undefined::add(self.value, rhs)
            }
            .0
        }
    }
}

impl std::ops::Sub for JValue {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        unsafe {
            if self.is_bigint() {
                bigint::sub(self.value, rhs)
            } else if self.is_true() {
                true_::sub(self.value, rhs)
            } else if self.is_false() {
                false_::sub(self.value, rhs)
            } else if self.is_null() {
                null::sub(self.value, rhs)
            } else if self.is_number() {
                number::sub(self.value, rhs)
            } else if self.is_object() {
                object::sub(self.value, rhs)
            } else if self.is_string() {
                string::sub(self.value, rhs)
            } else if self.is_symbol() {
                todo!("throw when sub symbol")
            } else {
                // undefined
                undefined::sub(self.value, rhs)
            }
            .0
        }
    }
}

impl std::ops::Mul for JValue {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        unsafe {
            if self.is_bigint() {
                bigint::mul(self.value, rhs)
            } else if self.is_true() {
                true_::mul(self.value, rhs)
            } else if self.is_false() {
                false_::mul(self.value, rhs)
            } else if self.is_null() {
                null::mul(self.value, rhs)
            } else if self.is_number() {
                number::mul(self.value, rhs)
            } else if self.is_object() {
                object::mul(self.value, rhs)
            } else if self.is_string() {
                string::mul(self.value, rhs)
            } else if self.is_symbol() {
                todo!("throw when multiplying symbol")
            } else {
                // undefined
                undefined::mul(self.value, rhs)
            }
            .0
        }
    }
}

impl std::ops::Div for JValue {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        unsafe {
            if self.is_bigint() {
                bigint::div(self.value, rhs)
            } else if self.is_true() {
                true_::div(self.value, rhs)
            } else if self.is_false() {
                false_::div(self.value, rhs)
            } else if self.is_null() {
                null::div(self.value, rhs)
            } else if self.is_number() {
                number::div(self.value, rhs)
            } else if self.is_object() {
                object::div(self.value, rhs)
            } else if self.is_string() {
                string::div(self.value, rhs)
            } else if self.is_symbol() {
                todo!("throw when div symbol")
            } else {
                // undefined
                undefined::div(self.value, rhs)
            }
            .0
        }
    }
}

impl std::ops::Rem for JValue {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self::Output {
        unsafe {
            if self.is_bigint() {
                bigint::rem(self.value, rhs)
            } else if self.is_true() {
                true_::rem(self.value, rhs)
            } else if self.is_false() {
                false_::rem(self.value, rhs)
            } else if self.is_null() {
                null::rem(self.value, rhs)
            } else if self.is_number() {
                number::rem(self.value, rhs)
            } else if self.is_object() {
                object::rem(self.value, rhs)
            } else if self.is_string() {
                string::rem(self.value, rhs)
            } else if self.is_symbol() {
                todo!("throw when rem symbol")
            } else {
                // undefined
                undefined::rem(self.value, rhs)
            }
            .0
        }
    }
}

impl PartialEq for JValue {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            std::mem::transmute_copy::<_, [usize; 2]>(self)
                == std::mem::transmute_copy::<_, [usize; 2]>(other)
        }
    }
}

impl Eq for JValue {}

impl std::cmp::PartialOrd for JValue {
    fn ge(&self, other: &Self) -> bool {
        self.to_number() >= other.to_number()
    }

    fn gt(&self, other: &Self) -> bool {
        self.to_number() > other.to_number()
    }

    fn le(&self, other: &Self) -> bool {
        self.to_number() <= other.to_number()
    }

    fn lt(&self, other: &Self) -> bool {
        self.to_number() < other.to_number()
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self > other {
            Some(std::cmp::Ordering::Greater)
        } else if self < other {
            Some(std::cmp::Ordering::Less)
        } else {
            Some(std::cmp::Ordering::Equal)
        }
    }
}

impl std::ops::Not for JValue {
    type Output = Self;
    fn not(self) -> Self::Output {
        (!self.to_bool()).into()
    }
}

impl std::ops::Neg for JValue {
    type Output = Self;
    fn neg(self) -> Self::Output {
        (-self.to_number()).into()
    }
}

impl std::ops::Shl for JValue {
    type Output = Self;
    fn shl(self, rhs: Self) -> Self::Output {
        JValue::create_number(((self.to_number() as i32) << (rhs.to_number() as i32)) as f64)
    }
}

impl std::ops::Shr for JValue {
    type Output = Self;
    fn shr(self, rhs: Self) -> Self::Output {
        JValue::create_number(((self.to_number() as i32) >> (rhs.to_number() as i32)) as f64)
    }
}

impl std::ops::BitAnd for JValue {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        JValue::create_number((self.to_i32() & rhs.to_i32()) as f64)
    }
}

impl std::ops::BitOr for JValue {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        JValue::create_number((self.to_i32() | rhs.to_i32()) as f64)
    }
}

impl std::ops::BitXor for JValue {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        JValue::create_number((self.to_i32() ^ rhs.to_i32()) as f64)
    }
}

impl ToString for JValue {
    fn to_string(&self) -> String {
        unsafe {
            if self.is_string() {
                self.value.string.to_string()
            } else if self.is_object() {
                self.value.object.to_string()
            } else if self.is_bigint() {
                self.value.bigint.to_string()
            } else if self.is_false() {
                "false".to_string()
            } else if self.is_null() {
                "null".to_string()
            } else if self.is_number() {
                self.value.number.to_string()
            } else if self.is_symbol() {
                self.value.symbol.to_string()
            } else if self.is_true() {
                "true".to_string()
            } else {
                "undefined".to_string()
            }
        }
    }
}

mod number {

    use crate::Runtime;

    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::create_number(value.number + rhs.as_number_uncheck())
        } else if rhs.is_string() {
            JValue::create_string((value.number.to_string() + rhs.value.string).into())
        } else if rhs.is_false() {
            JValue::create_number(value.number)
        } else if rhs.is_null() {
            JValue::create_number(value.number)
        } else if rhs.is_undefined() {
            JValue::create_number(f64::NAN)
        } else if rhs.is_true() {
            JValue::create_number(value.number + 1.0)
        } else if rhs.is_object() {
            JValue::create_string(
                (value.number.to_string() + rhs.value.object.to_string().as_str()).into(),
            )
        } else if rhs.is_bigint() {
            JValue::create_number(value.number + rhs.value.bigint as f64)
        } else if rhs.is_symbol() {
            // symbol
            // todo: throw TypeError: cannot convert symbol to primitive
            return super::symbol::throw(value, rhs);
        } else {
            panic!("unknown Jvalue")
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::create_number(value.number - rhs.value.number)
        } else if rhs.is_bigint() {
            JValue::create_number(value.number - rhs.value.bigint as f64)
        } else if rhs.is_false() {
            JValue::create_number(value.number)
        } else if rhs.is_null() {
            JValue::create_number(value.number)
        } else if rhs.is_undefined() {
            JValue::create_number(f64::NAN)
        } else if rhs.is_true() {
            JValue::create_number(value.number - 1.0)
        } else if rhs.is_string() {
            if let Ok(v) = fast_float::parse::<f64, _>(rhs.value.string.as_str()) {
                JValue::create_number(value.number - v)
            } else {
                JValue::create_number(f64::NAN)
            }
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            JValue::create_number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn mul(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::create_number(value.number * rhs.value.number)
        } else if rhs.is_bigint() {
            JValue::create_number(value.number * rhs.value.bigint as f64)
        } else if rhs.is_false() {
            JValue::create_number(0.0)
        } else if rhs.is_null() {
            JValue::create_number(0.0)
        } else if rhs.is_undefined() {
            JValue::create_number(f64::NAN)
        } else if rhs.is_true() {
            JValue::create_number(1.0)
        } else if rhs.is_string() {
            if let Ok(v) = fast_float::parse::<f64, _>(rhs.value.string.as_str()) {
                JValue::create_number(value.number * v)
            } else {
                JValue::create_number(f64::NAN)
            }
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            JValue::create_number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn div(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::create_number(value.number / rhs.value.number)
        } else if rhs.is_bigint() {
            JValue::create_number(value.number / rhs.value.bigint as f64)
        } else if rhs.is_false() {
            JValue::create_number(f64::INFINITY)
        } else if rhs.is_null() {
            JValue::create_number(f64::INFINITY)
        } else if rhs.is_undefined() {
            JValue::create_number(f64::NAN)
        } else if rhs.is_true() {
            JValue::create_number(value.number)
        } else if rhs.is_string() {
            if let Ok(v) = fast_float::parse::<f64, _>(rhs.value.string.as_str()) {
                JValue::create_number(value.number / v)
            } else {
                JValue::create_number(f64::NAN)
            }
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            JValue::create_number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn rem(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::create_number(value.number % rhs.value.number)
        } else if rhs.is_bigint() {
            JValue::create_number(value.number % rhs.value.bigint as f64)
        } else if rhs.is_false() {
            JValue::create_number(f64::NAN)
        } else if rhs.is_null() {
            JValue::create_number(f64::NAN)
        } else if rhs.is_undefined() {
            JValue::create_number(f64::NAN)
        } else if rhs.is_true() {
            JValue::create_number(value.number % 1.0)
        } else if rhs.is_string() {
            if let Ok(v) = fast_float::parse::<f64, _>(rhs.value.string.as_str()) {
                JValue::create_number(value.number % v)
            } else {
                JValue::create_number(f64::NAN)
            }
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            JValue::create_number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn exp(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            JValue::create_number(value.number.powf(rhs.value.number))
        } else if rhs.is_bigint() {
            JValue::create_number(value.number.powf(rhs.value.bigint as f64))
        } else if rhs.is_false() {
            JValue::create_number(f64::NAN)
        } else if rhs.is_null() {
            JValue::create_number(f64::NAN)
        } else if rhs.is_undefined() {
            JValue::create_number(f64::NAN)
        } else if rhs.is_true() {
            JValue::create_number(value.number)
        } else if rhs.is_string() {
            if let Ok(v) = fast_float::parse::<f64, _>(rhs.value.string.as_str()) {
                JValue::create_number(value.number.powf(v))
            } else {
                JValue::create_number(f64::NAN)
            }
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            JValue::create_number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn eqeq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            value.number == rhs.value.number
        } else if rhs.is_bigint() {
            value.number == rhs.value.bigint as f64
        } else if rhs.is_false() {
            value.number == 0.0
        } else if rhs.is_null() {
            value.number == 0.0
        } else if rhs.is_undefined() {
            value.number == f64::NAN
        } else if rhs.is_true() {
            value.number == 1.0
        } else if rhs.is_string() {
            if let Ok(v) = fast_float::parse::<f64, _>(rhs.value.string.as_str()) {
                value.number == v
            } else {
                false
            }
        } else if rhs.is_symbol() {
            false
        } else {
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn noteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let (v, error) = eqeq(value, rhs);
        if error {
            return (v, true);
        }
        if v.is_true() {
            (JValue::FALSE, false)
        } else {
            (JValue::TRUE, false)
        }
    }

    pub(crate) unsafe fn gt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            value.number > rhs.value.number
        } else if rhs.is_bigint() {
            value.number as i64 > rhs.value.bigint
        } else if rhs.is_false() {
            value.number > 0.0
        } else if rhs.is_true() {
            value.number > 1.0
        } else if rhs.is_null() {
            value.number > 0.0
        } else if rhs.is_object() {
            false
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.number > 0.0
            } else {
                if let Ok(v) = fast_float::parse::<f64, _>(rhs.value.string.as_str()) {
                    value.number > v
                } else {
                    false
                }
            }
        } else if rhs.is_undefined() {
            value.number > f64::NAN
        } else {
            false
        };

        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn gteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            value.number >= rhs.value.number
        } else if rhs.is_bigint() {
            value.number as i64 >= rhs.value.bigint
        } else if rhs.is_false() {
            value.number >= 0.0
        } else if rhs.is_true() {
            value.number >= 1.0
        } else if rhs.is_null() {
            value.number >= 0.0
        } else if rhs.is_object() {
            false
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.number >= 0.0
            } else {
                if let Ok(v) = fast_float::parse::<f64, _>(rhs.value.string.as_str()) {
                    value.number >= v
                } else {
                    false
                }
            }
        } else if rhs.is_undefined() {
            value.number > f64::NAN
        } else {
            false
        };

        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            value.number < rhs.value.number
        } else if rhs.is_bigint() {
            (value.number as i64) < (rhs.value.bigint)
        } else if rhs.is_false() {
            value.number < 0.0
        } else if rhs.is_true() {
            value.number < 1.0
        } else if rhs.is_null() {
            value.number < 0.0
        } else if rhs.is_object() {
            false
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.number < 0.0
            } else {
                if let Ok(v) = fast_float::parse::<f64, _>(rhs.value.string.as_str()) {
                    value.number < v
                } else {
                    false
                }
            }
        } else if rhs.is_undefined() {
            value.number < f64::NAN
        } else {
            false
        };

        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            value.number <= rhs.value.number
        } else if rhs.is_bigint() {
            value.number as i64 <= rhs.value.bigint
        } else if rhs.is_false() {
            value.number <= 0.0
        } else if rhs.is_true() {
            value.number <= 1.0
        } else if rhs.is_null() {
            value.number <= 0.0
        } else if rhs.is_object() {
            false
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.number <= 0.0
            } else {
                if let Ok(v) = fast_float::parse::<f64, _>(rhs.value.string.as_str()) {
                    value.number <= v
                } else {
                    false
                }
            }
        } else if rhs.is_undefined() {
            value.number <= f64::NAN
        } else {
            false
        };

        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) fn get(value: JValueUnion, field: JValue, stack: *mut JValue) -> (JValue, bool) {
        let rt = Runtime::current();
        let field = if field.is_string() {
            rt.register_field_name(unsafe { field.value.string.as_str() })
        } else {
            rt.regester_dynamic_var_name(&field.to_string())
        };
        get_static(value, field, stack)
    }

    pub(crate) fn get_static(value: JValueUnion, field: u32, stack: *mut JValue) -> (JValue, bool) {
        use crate::utils::string_interner::NAMES;
        let rt = Runtime::current();
        let re = if field == NAMES["toExponential"].0 {
            rt.prototypes.number.get_property_static(field, stack)
        } else if field == NAMES["toFixed"].0 {
            rt.prototypes.number.get_property_static(field, stack)
        } else if field == NAMES["toLocaleString"].0 {
            rt.prototypes.number.get_property_static(field, stack)
        } else if field == NAMES["toPrecision"].0 {
            rt.prototypes.number.get_property_static(field, stack)
        } else if field == NAMES["toString"].0 {
            rt.prototypes.number.get_property_static(field, stack)
        } else if field == NAMES["valueOf"].0 {
            rt.prototypes.number.get_property_static(field, stack)
        } else {
            Ok(JValue::UNDEFINED)
        };

        match re {
            Ok(v) => (v, false),
            Err(v) => (v, true),
        }
    }
}

mod null {
    use super::JValue;
    use super::JValueUnion;

    pub(crate) unsafe fn add(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_number() {
            rhs
        } else if rhs.is_bigint() {
            rhs
        } else if rhs.is_false() {
            JValue::create_number(0.0)
        } else if rhs.is_undefined() {
            JValue::create_number(0.0)
        } else if rhs.is_true() {
            JValue::create_number(1.0)
        } else if rhs.is_string() {
            JValue::create_string(format!("null{}", rhs.value.string.as_ref()).into())
        } else if rhs.is_object() {
            JValue::create_string(format!("null{}", rhs.value.object.to_string()).into())
        } else {
            // symbol
            JValue::create_number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::sub(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn mul(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_string() {
            if let Ok(_v) = fast_float::parse::<f64, _>(rhs.value.string.as_str()) {
                JValue::create_number(0.0)
            } else {
                JValue::create_number(f64::NAN)
            }
        } else if rhs.is_object() {
            JValue::create_number(f64::NAN)
        } else if rhs.is_symbol() {
            // todo: throw TypeError
            JValue::create_number(f64::NAN)
        } else {
            JValue::create_number(0.0)
        };
        (v, false)
    }

    pub(crate) unsafe fn div(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::div(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn rem(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::rem(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn exp(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::exp(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn eqeq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_null() {
            JValue::TRUE
        } else if rhs.is_undefined() {
            JValue::TRUE
        } else {
            JValue::FALSE
        };
        (v, false)
    }

    pub(crate) unsafe fn noteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_null() {
            JValue::FALSE
        } else if rhs.is_undefined() {
            JValue::FALSE
        } else {
            JValue::TRUE
        };
        (v, false)
    }

    pub(crate) unsafe fn gt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gt(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn gteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gteq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn lt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn lteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 0.0 }, rhs)
    }
}

mod undefined {
    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_string() {
            JValue::create_string(format!("undefined{}", rhs.value.string.as_ref()).into())
        } else if rhs.is_object() {
            JValue::create_string(format!("undefined{}", rhs.value.object.to_string()).into())
        } else if rhs.is_symbol() {
            // symbol
            JValue::create_number(f64::NAN)
        } else {
            JValue::create_number(f64::NAN)
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        return (JValue::create_number(f64::NAN), false);
    }

    pub(crate) unsafe fn mul(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        return (JValue::create_number(f64::NAN), false);
    }

    pub(crate) unsafe fn div(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        return (JValue::create_number(f64::NAN), false);
    }

    pub(crate) unsafe fn rem(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        return (JValue::create_number(f64::NAN), false);
    }

    pub(crate) unsafe fn exp(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        return (JValue::create_number(f64::NAN), false);
    }

    pub(crate) unsafe fn eqeq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_undefined() {
            (JValue::TRUE, false)
        } else if rhs.is_null() {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn noteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_undefined() {
            (JValue::FALSE, false)
        } else if rhs.is_null() {
            (JValue::FALSE, false)
        } else {
            (JValue::TRUE, false)
        }
    }

    pub(crate) unsafe fn gt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gt(JValueUnion { number: f64::NAN }, rhs)
    }

    pub(crate) unsafe fn gteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gteq(JValueUnion { number: f64::NAN }, rhs)
    }

    pub(crate) unsafe fn lt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lt(JValueUnion { number: f64::NAN }, rhs)
    }

    pub(crate) unsafe fn lteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: f64::NAN }, rhs)
    }
}

mod true_ {
    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_string() {
            JValue::create_string(format!("true{}", rhs.value.string.as_ref()).into())
        } else if rhs.is_object() {
            JValue::create_string(format!("true{}", rhs.value.object.to_string()).into())
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            return super::number::add(JValueUnion { number: 1.0 }, rhs);
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::sub(JValueUnion { number: 1.0 }, rhs);
    }

    pub(crate) unsafe fn mul(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::mul(JValueUnion { number: 1.0 }, rhs);
    }

    pub(crate) unsafe fn div(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::div(JValueUnion { number: 1.0 }, rhs);
    }

    pub(crate) unsafe fn rem(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::sub(JValueUnion { number: 1.0 }, rhs);
    }

    pub(crate) unsafe fn exp(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::exp(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn eqeq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::eqeq(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn noteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::noteq(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn gt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gt(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn gteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gteq(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn lt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 1.0 }, rhs)
    }

    pub(crate) unsafe fn lteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 1.0 }, rhs)
    }
}

mod false_ {
    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_string() {
            JValue::create_string(format!("false{}", rhs.value.string.as_ref()).into())
        } else if rhs.is_object() {
            JValue::create_string(format!("false{}", rhs.value.object.to_string()).into())
        } else if rhs.is_symbol() {
            return super::symbol::throw(value, rhs);
        } else {
            return super::number::add(JValueUnion { number: 0.0 }, rhs);
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::sub(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn mul(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::mul(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn div(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::div(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn rem(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        return super::number::sub(JValueUnion { number: 0.0 }, rhs);
    }

    pub(crate) unsafe fn exp(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::exp(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn eqeq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::eqeq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn noteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::noteq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn gt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gt(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn gteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::gteq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn lt(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 0.0 }, rhs)
    }

    pub(crate) unsafe fn lteq(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        super::number::lteq(JValueUnion { number: 0.0 }, rhs)
    }
}

mod string {
    use crate::runtime::Runtime;

    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_string() {
            JValue::create_string(value.string + rhs.value.string)
        } else if rhs.is_number() {
            JValue::create_string(value.string + rhs.value.number.to_string().as_ref())
        } else if rhs.is_bigint() {
            JValue::create_string(value.string + rhs.value.bigint.to_string().as_ref())
        } else if rhs.is_false() {
            JValue::create_string(value.string + "false")
        } else if rhs.is_true() {
            JValue::create_string(value.string + "true")
        } else if rhs.is_null() {
            JValue::create_string(value.string + "null")
        } else if rhs.is_undefined() {
            JValue::create_string(value.string + "undefined")
        } else if rhs.is_object() {
            JValue::create_string(value.string + rhs.value.object.to_string().as_ref())
        } else {
            // symbol
            // todo: throw TypeError
            JValue::create_string("".into())
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            return super::number::sub(JValueUnion { number: 0.0 }, rhs);
        }
        if let Ok(v) = fast_float::parse::<f64, _>(value.string.as_str()) {
            return super::number::sub(JValueUnion { number: v }, rhs);
        } else {
            (JValue::create_number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn mul(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            return super::number::mul(JValueUnion { number: 0.0 }, rhs);
        }
        if let Ok(v) = fast_float::parse::<f64, _>(value.string.as_str()) {
            return super::number::mul(JValueUnion { number: v }, rhs);
        } else {
            (JValue::create_number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn div(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            return super::number::div(JValueUnion { number: 0.0 }, rhs);
        }
        if let Ok(v) = fast_float::parse::<f64, _>(value.string.as_str()) {
            return super::number::div(JValueUnion { number: v }, rhs);
        } else {
            (JValue::create_number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn rem(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            return super::number::rem(JValueUnion { number: 0.0 }, rhs);
        }
        if let Ok(v) = fast_float::parse::<f64, _>(value.string.as_str()) {
            return super::number::rem(JValueUnion { number: v }, rhs);
        } else {
            (JValue::create_number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn exp(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            return super::number::exp(JValueUnion { number: 0.0 }, rhs);
        }
        if let Ok(v) = fast_float::parse::<f64, _>(value.string.as_str()) {
            return super::number::exp(JValueUnion { number: v }, rhs);
        } else {
            (JValue::create_number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn eqeq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            if let Ok(i) = value.string.parse::<i64>() {
                i == rhs.value.bigint
            } else {
                false
            }
        } else if rhs.is_false() || rhs.is_null() {
            if value.string.len() == 0 {
                true
            } else {
                if let Ok(v) = fast_float::parse::<f64, _>(value.string.as_str()) {
                    v == 0.0
                } else {
                    false
                }
            }
        } else if rhs.is_true() {
            if let Ok(v) = fast_float::parse::<f64, _>(value.string.as_str()) {
                v == 1.0
            } else {
                false
            }
        } else if rhs.is_object() {
            rhs.to_string().as_str() == value.string.as_ref()
        } else if rhs.is_string() {
            value.string.as_ref() == rhs.value.string.as_ref()
        } else if rhs.is_symbol() {
            false
        } else {
            false
        };

        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn noteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let (v, error) = eqeq(value, rhs);
        if error {
            return (v, error);
        }
        if v.is_true() {
            (JValue::FALSE, false)
        } else {
            (JValue::TRUE, false)
        }
    }

    pub(crate) unsafe fn gt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            super::number::gt(JValueUnion { number: 0.0 }, rhs)
        } else {
            if let Ok(v) = fast_float::parse::<f64, _>(value.string.as_str()) {
                super::number::gt(JValueUnion { number: v }, rhs)
            } else {
                (JValue::FALSE, false)
            }
        }
    }

    pub(crate) unsafe fn gteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            super::number::gteq(JValueUnion { number: 0.0 }, rhs)
        } else {
            if let Ok(v) = fast_float::parse::<f64, _>(value.string.as_str()) {
                super::number::gteq(JValueUnion { number: v }, rhs)
            } else {
                (JValue::FALSE, false)
            }
        }
    }

    pub(crate) unsafe fn lt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            super::number::lt(JValueUnion { number: 0.0 }, rhs)
        } else {
            if let Ok(v) = fast_float::parse::<f64, _>(value.string.as_str()) {
                super::number::lt(JValueUnion { number: v }, rhs)
            } else {
                (JValue::FALSE, false)
            }
        }
    }

    pub(crate) unsafe fn lteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.string.len() == 0 {
            super::number::lteq(JValueUnion { number: 0.0 }, rhs)
        } else {
            if let Ok(v) = fast_float::parse::<f64, _>(value.string.as_str()) {
                super::number::lteq(JValueUnion { number: v }, rhs)
            } else {
                (JValue::FALSE, false)
            }
        }
    }

    pub(crate) unsafe fn set(
        _obj: JValueUnion,
        _field: JValue,
        value: JValue,
        _stack: *mut JValue,
    ) -> (JValue, bool) {
        (value, false)
    }

    pub(crate) unsafe fn set_static(
        _obj: JValueUnion,
        _field: u32,
        value: JValue,
        _stack: *mut JValue,
    ) -> (JValue, bool) {
        (value, false)
    }

    pub(crate) unsafe fn remove_key_static(_obj: JValueUnion, _field: u32) {
        // do nothing
    }

    pub(crate) unsafe fn get(
        obj: JValueUnion,
        field: JValue,
        _stack: *mut JValue,
    ) -> (JValue, bool) {
        let v = if field.is_number() {
            let s = obj.string.as_ref().chars().nth(field.value.number as usize);
            match s {
                Some(v) => v.to_string().into(),
                None => JValue::UNDEFINED,
            }
        } else if field.is_bigint() {
            let s = obj.string.as_ref().chars().nth(field.value.bigint as usize);
            match s {
                Some(v) => v.to_string().into(),
                None => JValue::UNDEFINED,
            }
        } else if field.is_string() {
            if let Ok(v) = fast_float::parse::<f64, _>(field.value.string.as_str()) {
                let s = obj.string.as_ref().chars().nth(v as usize);
                match s {
                    Some(v) => v.to_string().into(),
                    None => JValue::UNDEFINED,
                }
            } else {
                JValue::UNDEFINED
            }
        } else {
            JValue::UNDEFINED
        };

        (v, false)
    }

    pub(crate) unsafe fn get_static(
        obj: JValueUnion,
        field: u32,
        _stack: *mut JValue,
    ) -> (JValue, bool) {
        let runtime = Runtime::current();
        let field = runtime.get_field_name(field);

        if let Ok(n) = fast_float::parse::<f64, _>(field) {
            if n < 0.0 {
                return (JValue::UNDEFINED, false);
            }
            let s = obj.string.as_ref().chars().nth(n as usize);
            match s {
                Some(v) => (v.to_string().into(), false),
                None => (JValue::UNDEFINED, false),
            }
        } else {
            (JValue::UNDEFINED, false)
        }
    }
}

mod bigint {
    use super::*;

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            JValue::create_bigint(value.bigint + rhs.value.bigint)
        } else if rhs.is_string() {
            JValue::create_string((value.bigint.to_string() + rhs.value.string).into())
        } else {
            return (
                JValue::from(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions".into(),
                )),
                true,
            );
        };
        (v, false)
    }

    pub(crate) unsafe fn sub(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_bigint() {
            (JValue::create_bigint(value.bigint - rhs.value.bigint), false)
        } else {
            (
                JValue::from(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions"
                        .to_owned(),
                )),
                true,
            )
        }
    }

    pub(crate) unsafe fn mul(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_bigint() {
            (JValue::create_bigint(value.bigint * rhs.value.bigint), false)
        } else {
            (
                JValue::from(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions"
                        .to_owned(),
                )),
                true,
            )
        }
    }

    pub(crate) unsafe fn div(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_bigint() {
            (JValue::create_bigint(value.bigint / rhs.value.bigint), false)
        } else {
            (
                JValue::from(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions"
                        .to_owned(),
                )),
                true,
            )
        }
    }

    pub(crate) unsafe fn rem(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_bigint() {
            (JValue::create_bigint(value.bigint % rhs.value.bigint), false)
        } else {
            (
                JValue::from(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions"
                        .to_owned(),
                )),
                true,
            )
        }
    }

    pub(crate) unsafe fn exp(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if rhs.is_bigint() {
            (
                JValue::create_bigint(value.bigint.pow(rhs.value.bigint as u32)),
                false,
            )
        } else {
            (
                JValue::from(Error::TypeError(
                    "TypeError: Cannot mix BigInt and other types, use explicit conversions"
                        .to_owned(),
                )),
                true,
            )
        }
    }

    pub(crate) unsafe fn eqeq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            value.bigint == rhs.value.bigint
        } else if rhs.is_false() {
            value.bigint == 0
        } else if rhs.is_true() {
            value.bigint == 1
        } else if rhs.is_null() {
            value.bigint == 0
        } else if rhs.is_number() {
            value.bigint == rhs.value.number as i64
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.bigint == 0
            } else {
                if let Ok(v) = rhs.value.string.parse::<i64>() {
                    value.bigint == v
                } else {
                    false
                }
            }
        } else {
            // object, symbol, undefined
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn noteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let (v, error) = eqeq(value, rhs);
        if error {
            return (v, true);
        }
        if v.is_true() {
            (JValue::FALSE, false)
        } else {
            (JValue::TRUE, false)
        }
    }

    pub(crate) unsafe fn gt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            value.bigint > rhs.value.bigint
        } else if rhs.is_false() {
            value.bigint > 0
        } else if rhs.is_true() {
            value.bigint > 1
        } else if rhs.is_null() {
            value.bigint > 0
        } else if rhs.is_number() {
            value.bigint > rhs.value.number as i64
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.bigint > 0
            } else {
                if let Ok(v) = rhs.value.string.parse::<i64>() {
                    value.bigint > v
                } else {
                    false
                }
            }
        } else {
            // object, symbol, undefined
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn gteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            value.bigint >= rhs.value.bigint
        } else if rhs.is_false() {
            value.bigint >= 0
        } else if rhs.is_true() {
            value.bigint >= 1
        } else if rhs.is_null() {
            value.bigint >= 0
        } else if rhs.is_number() {
            value.bigint >= rhs.value.number as i64
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.bigint >= 0
            } else {
                if let Ok(v) = rhs.value.string.parse::<i64>() {
                    value.bigint >= v
                } else {
                    false
                }
            }
        } else {
            // object, symbol, undefined
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            value.bigint < rhs.value.bigint
        } else if rhs.is_false() {
            value.bigint < 0
        } else if rhs.is_true() {
            value.bigint < 1
        } else if rhs.is_null() {
            value.bigint < 0
        } else if rhs.is_number() {
            value.bigint < rhs.value.number as i64
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.bigint < 0
            } else {
                if let Ok(v) = rhs.value.string.parse::<i64>() {
                    value.bigint < v
                } else {
                    false
                }
            }
        } else {
            // object, symbol, undefined
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v = if rhs.is_bigint() {
            value.bigint <= rhs.value.bigint
        } else if rhs.is_false() {
            value.bigint <= 0
        } else if rhs.is_true() {
            value.bigint <= 1
        } else if rhs.is_null() {
            value.bigint <= 0
        } else if rhs.is_number() {
            value.bigint <= rhs.value.number as i64
        } else if rhs.is_string() {
            if rhs.value.string.len() == 0 {
                value.bigint <= 0
            } else {
                if let Ok(v) = rhs.value.string.parse::<i64>() {
                    value.bigint <= v
                } else {
                    false
                }
            }
        } else {
            // object, symbol, undefined
            false
        };
        if v {
            (JValue::TRUE, false)
        } else {
            (JValue::FALSE, false)
        }
    }
}

mod object {
    use crate::JSContext;

    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn add(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().add_(rhs)
        } else {
            (
                (value.object.to_string() + rhs.to_string().as_str()).into(),
                false,
            )
        }
    }

    pub(crate) unsafe fn sub(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().sub_(rhs)
        } else {
            (JValue::create_number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn mul(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().mul_(rhs)
        } else {
            (JValue::create_number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn div(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().div_(rhs)
        } else {
            (JValue::create_number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn rem(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().rem_(rhs)
        } else {
            (JValue::create_number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn exp(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().exp_(rhs)
        } else {
            (JValue::create_number(f64::NAN), false)
        }
    }

    pub(crate) unsafe fn eqeq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v: bool = if rhs.is_object() {
            value.object == rhs.value.object
        } else if value.object.is_primitive() {
            return (value.object.to_primitive().unwrap().eqeq_(rhs), false);
        } else {
            false
        };
        (JValue::from(v), false)
    }

    pub(crate) unsafe fn noteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let (v, error) = eqeq(value, rhs);
        if error {
            return (v, true);
        }
        if v.is_true() {
            (JValue::FALSE, false)
        } else {
            (JValue::TRUE, false)
        }
    }

    pub(crate) unsafe fn gt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().gt_(rhs)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lt(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().lt_(rhs)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn gteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().gteq_(rhs)
        } else if rhs.is_object() {
            (JValue::from(value.object == rhs.value.object), false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn lteq(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if value.object.is_primitive() {
            value.object.to_primitive().unwrap().lteq_(rhs)
        } else if rhs.is_object() {
            (JValue::from(value.object == rhs.value.object), false)
        } else {
            (JValue::FALSE, false)
        }
    }

    pub(crate) unsafe fn get(
        obj: JValueUnion,
        field: JValue,
        ctx:JSContext
    ) -> (JValue, bool) {
        if field.is_string() {
            let s = field.value.string.as_str();
            match obj.object.get_property(s, ctx) {
                Ok(v) => (v, false),
                Err(e) => (e, true),
            }
        } else {
            match obj.object.get_property(&field.to_string(), ctx) {
                Ok(v) => (v, false),
                Err(e) => (e, true),
            }
        }
    }

    pub(crate) unsafe fn get_static(
        obj: JValueUnion,
        field: u32,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        match obj.object.get_property_static(field, stack) {
            Ok(v) => (v, false),
            Err(e) => (e, true),
        }
    }

    pub(crate) unsafe fn set(
        obj: JValueUnion,
        field: JValue,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        if field.is_string() {
            let s = field.value.string.as_str();
            obj.object.set_property(s, value, stack)
        } else {
            obj.object.set_property(&field.to_string(), value, stack)
        }
    }

    pub(crate) unsafe fn set_static(
        obj: JValueUnion,
        field: u32,
        value: JValue,
        stack: *mut JValue,
    ) -> (JValue, bool) {
        obj.object.set_property_static(field, value, stack)
    }

    pub(crate) unsafe fn remove_key_static(obj: JValueUnion, field: u32) {
        obj.object.inner.to_mut().remove_key_static(field);
    }

    pub(crate) unsafe fn instance_of(value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        let v: bool = if rhs.is_object() {
            if rhs.value.object.inner.wrapped_value.is_function() {
                let v = rhs
                    .value
                    .object
                    .get_property("prototype", std::ptr::null_mut())
                    .unwrap();
                value
                    .object
                    .get_property("__proto__", std::ptr::null_mut())
                    .unwrap()
                    == v
            } else {
                false
            }
        } else {
            return (JValue::FALSE, false);
        };
        (JValue::from(v), false)
    }

    pub(crate) unsafe fn In(obj: JValueUnion, field: JValue) -> (JValue, bool) {
        let v = obj.object.has_owned_property(&field.to_string());
        (JValue::from(v), false)
    }

    pub(crate) unsafe fn False(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        (JValue::FALSE, false)
    }
}

mod not_object {
    use super::*;

    pub(crate) fn get(_value: JValueUnion, _rhs: JValue, _stack: *mut JValue) -> (JValue, bool) {
        (
            JValue::from(Error::TypeError(
                "cannot read property of non object".into(),
            )),
            true,
        )
    }

    pub(crate) fn get_static(
        _value: JValueUnion,
        _rhs: u32,
        _stack: *mut JValue,
    ) -> (JValue, bool) {
        (
            JValue::from(Error::TypeError(
                "cannot read property of non object".into(),
            )),
            true,
        )
    }

    pub(crate) fn set(
        _obj: JValueUnion,
        _field: JValue,
        _value: JValue,
        _stack: *mut JValue,
    ) -> (JValue, bool) {
        (
            JValue::from(Error::TypeError(format!(
                "cannot set property of non object 1",
            ))),
            true,
        )
    }

    pub(crate) fn set_static(
        _obj: JValueUnion,
        _field: u32,
        _value: JValue,
        _stack: *mut JValue,
    ) -> (JValue, bool) {
        (
            JValue::from(Error::TypeError("cannot set property of non object".into())),
            true,
        )
    }

    pub(crate) fn remove_key_static(_obj: JValueUnion, _field: u32) {}

    pub(crate) unsafe fn instance_of(_value: JValueUnion, rhs: JValue) -> (JValue, bool) {
        if !rhs.is_object() {
            return (
                JValue::from(Error::TypeError(
                    "Right-hand side of 'instance_of' is not callable".into(),
                )),
                true,
            );
        }
        (JValue::FALSE, false)
    }

    pub(crate) unsafe fn In(_obj: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        (JValue::FALSE, false)
    }
}

mod symbol {
    use super::{JValue, JValueUnion};

    pub(crate) unsafe fn throw(_value: JValueUnion, _rhs: JValue) -> (JValue, bool) {
        todo!("TypeError: cannot convert Symbol to primitives.")
    }
}



*/

#[test]
fn test_jvalue_size() {
    assert!(std::mem::size_of::<JValue>() == 16);
}

#[test]
fn iadd() {
    let a = JValue::create_int(87389);
    let b = JValue::create_int(632754);

    println!("{:#b}", a.0 + b.as_int_unchecked() as u64);
    println!(
        "{:#b}",
        JValue::create_int(a.as_int_unchecked() + b.as_int_unchecked()).0
    );
}
