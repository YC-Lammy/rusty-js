pub mod nohasher;
pub mod string_interner;

/// offset_of!(type, field); return memory offset of field in usize
#[macro_export]
macro_rules! offset_of {
    ($t:ty, $field:ident) => {
        {
            const a:$t = unsafe{std::mem::zeroed()};
            &a.$field as *const _ as usize - &a as *const _ as usize
        }
    };
}

#[repr(C)]
pub struct List<T>{
    pointer:*mut T,
    len:usize
}

impl<T> AsRef<[T]> for List<T>{
    fn as_ref(&self) -> &[T] {
        unsafe{std::slice::from_raw_parts(self.pointer, self.len)}
    }
}

impl<T> std::ops::Deref for List<T>{
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}