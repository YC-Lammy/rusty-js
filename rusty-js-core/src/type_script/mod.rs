use std::collections::HashMap;
use std::sync::Arc;

mod function;
mod interface;
mod object;
use object::ObjectInfo;

use interface::InterfaceInfo;

use crate::{utils::string_interner::StringInterner, JSString};

use self::{interface::TSInterface, object::TSObject};

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct TypeName {
    name_space: u32,
    priority: u32,
    name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterfaceId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionId(u32);

pub struct TypeRegister {
    pub names: StringInterner,

    object_names: HashMap<TypeName, u32>,
    objects: Vec<ObjectInfo>,

    interface_names: HashMap<TypeName, u32>,
    interfaces: Vec<InterfaceInfo>,

    function_names: HashMap<TypeName, u32>,
    functions: Vec<FunctionInfo>,
}

pub struct FunctionInfo {
    pub params: HashMap<String, Type>,
    pub returns: Vec<Type>,
}

#[derive(Clone, PartialEq)]
#[repr(u8)]
pub enum Type {
    Null,
    Undefined,
    Void,
    Unknown,
    Never,
    /// alias of string
    String {
        optional: bool,
    },
    /// alias of number
    Number {
        optional: bool,
    },
    BigInt {
        optional: bool,
    },
    /// alias of boolean
    Boolean {
        optional: bool,
    },
    ///equivelent to javascript value
    Any,
    Union(Box<Type>, Box<Type>),
    Function {
        optional: bool,
        id: FunctionId,
    },
    Object {
        optional: bool,
        object: Arc<ObjectInfo>,
    },
    Array {
        ty: Box<Type>,
        optional: bool,
    },
    Interface {
        interface: InterfaceId,
        optional: bool,
    },
}