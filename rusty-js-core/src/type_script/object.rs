use std::collections::HashMap;

use super::Type;

pub struct ObjectInfo{
    pub properties:HashMap<String, Type>,
    pub prototype:Type,
}