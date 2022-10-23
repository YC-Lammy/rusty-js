use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArrayType {
    Int8,
    Uint8,
    Uint8Clamped,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
    BigInt64,
    BigUint64,
}

#[derive(Clone)]
pub struct TypedArray<T>
where
    T: 'static,
{
    pub array_buffer: Arc<Vec<u8>>,
    pub ty: PhantomData<T>,
}

impl<T: 'static> TypedArray<T> {
    pub fn len(&self) -> usize {
        return self.array_buffer.len() / std::mem::size_of::<T>();
    }
}
