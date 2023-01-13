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

impl std::fmt::Display for JSType{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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
