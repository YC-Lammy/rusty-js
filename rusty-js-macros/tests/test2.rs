use rusty_js_macros::ByteCode;

pub struct Register(u8);

pub trait ByteCode{
    fn writes_to(self) -> Option<Register>;
    fn reads_from(self) -> Vec<Register>;
}

#[derive(ByteCode)]
#[r]
pub enum A{
    OP{
        #[w]
        result:Register
    }
}