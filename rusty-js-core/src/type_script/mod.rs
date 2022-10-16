use std::collections::HashMap;


mod object;

use object::ObjectInfo;

pub struct TypeRegister{
    pub names:HashMap<String, TypeId>,
    pub types:Vec<TypeInfo>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeId(u32);

pub struct FunctionInfo{
    pub params:HashMap<String, Type>,
    pub returns:Vec<Type>
}

pub enum TypeInfo{
    Union(Type, Type),
    Object(ObjectInfo),
    Function(FunctionInfo)
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Type{
    Null,
    Undefined,
    Void,
    Unknown,
    Never,
    /// alias of string
    String{
        optional:bool
    },
    /// alias of number
    Number{
        optional:bool
    },
    BigInt{
        optional:bool
    },
    /// alias of boolean
    Boolean{
        optional:bool
    },
    Any,
    Union(TypeId, TypeId),
    Function(TypeId),
    Object{
        optional:bool,
        object:TypeId
    },
    Array(TypeId),
}

impl TypeRegister{
    pub fn new() -> TypeRegister{
        let mut r = TypeRegister{
            names:HashMap::new(),
            types:Default::default()
        };

        r.names.insert("builtin Object.prototype".into(), TypeId(0));
        r.types.push(TypeInfo::Object(ObjectInfo{
            properties:HashMap::from_iter((&[
                ("__proto__", Type::Null),
                ("constructor", Type::Undefined),
            ]).iter().map(|v|(v.0.to_owned(), v.1))),
            prototype:Type::Null,
        }));

        return r;
    }
}