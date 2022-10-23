use crate::bytecodes::OpCode;

pub trait Debugger {
    fn on_code_run(&mut self, code: OpCode);
}
