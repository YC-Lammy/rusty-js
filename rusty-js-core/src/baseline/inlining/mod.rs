use crate::bytecodes::{Block, OpCode, Register};

pub fn prepare_inlining(
    cs: &[OpCode],
    result: Register,
    this: Register,
    mut block_count: u16,
    base_stack_offset: u16,
) -> Vec<OpCode> {
    let exit = Block(block_count);

    let mut codes = vec![
        OpCode::PrepareInlinedCall {
            stack_offset: base_stack_offset,
        },
        OpCode::SetThis { value: this },
        OpCode::CreateBlock(exit),
    ];

    codes.extend_from_slice(cs);

    block_count += 1;

    let mut i = 2;
    while i < codes.len() {
        let code = codes[i];
        match code {
            OpCode::Return { value } => {
                codes[i] = OpCode::Mov {
                    from: value,
                    to: result,
                };
                codes.insert(i + 1, OpCode::Jump { to: exit, line: 0 });
                i += 1;
            }
            OpCode::ReadFromStack {
                result,
                stack_offset,
            } => {
                codes[i] = OpCode::ReadFromStack {
                    result,
                    stack_offset: stack_offset + base_stack_offset,
                };
            }
            OpCode::WriteToStack { from, stack_offset } => {
                codes[i] = OpCode::WriteToStack {
                    from,
                    stack_offset: stack_offset + base_stack_offset,
                };
            }
            OpCode::CreateBlock(b) => {
                codes[i] = OpCode::CreateBlock(Block(b.0 + block_count));
            }
            OpCode::SwitchToBlock(b) => {
                codes[i] = OpCode::SwitchToBlock(Block(b.0 + block_count));
            }
            OpCode::Jump { to, line: _ } => {
                codes[i] = OpCode::Jump {
                    to: Block(to.0 + block_count),
                    line: 0,
                };
            }
            OpCode::JumpIfFalse { value, to, line } => {
                codes[i] = OpCode::JumpIfFalse {
                    value: value,
                    to: Block(to.0 + block_count),
                    line: 0,
                }
            }
            OpCode::JumpIfTrue { value, to, line } => {
                codes[i] = OpCode::JumpIfTrue {
                    value: value,
                    to: Block(to.0 + block_count),
                    line: 0,
                }
            }
            _ => {}
        };
        i += 1;
    }

    codes.push(OpCode::LoadUndefined { result: result });
    codes.push(OpCode::Jump { to: exit, line: 0 });
    codes.push(OpCode::SwitchToBlock(exit));

    let codes = super::optimize(codes);
    return codes;
}
