use std::collections::HashMap;

use crate::{
    bytecodes::ByteCode,
    bytecodes::{Block, OpCode, Register},
    FuncID, Runtime,
};

pub fn optimize(mut codes: Vec<OpCode>) -> Vec<OpCode> {
    for i in 0..codes.len(){
        println!("{}: {:?}", i, &codes[i]);
    }
    codes = remove_unreachable(codes);
    //codes = remove_unuse_operations(codes);
    //codes = inline_static_function(codes); //todo: bugs to be fixed

    codes = remove_temp_read_from_stack(codes);
    codes = remove_unused_temp(codes);

    //codes = remove_unuse_temp(codes);
    //codes = remove_unuse_operations(codes);
    codes = remove_unused_mov(codes);

    codes = resolve_jump_locations(codes);

    

    return codes;
}

fn source_of(codes: &[OpCode], index: usize) -> Vec<usize> {
    let code = &codes[index];

    let mut srcs = Vec::new();

    let mut r = code.reads_from();
    for r in r {
        if r.0 == 255 {
            continue;
        }

        let mut idx = index - 1;
        while idx >= 0 {
            let c = &codes[idx];
            if let Some(re) = c.writes_to() {
                if re == r {
                    srcs.push(idx);
                    break;
                }
            }
            idx -= 1;
        }
    }
    return srcs;
}

fn is_same_source(a: OpCode, b: OpCode) -> bool {
    let mut a_r = [Register(255); 3];
    let mut b_r = [Register(255); 3];

    match a {
        OpCode::ReadFromStack {
            result,
            stack_offset,
        } => {}
        _ => {}
    };
    return false;
}

fn remove_unreachable(mut codes: Vec<OpCode>) -> Vec<OpCode> {
    let mut is_terminated = false;
    let mut i = 0;

    while i < codes.len() {
        let code = &codes[i];
        match code {
            OpCode::SwitchToBlock(_) => {
                is_terminated = false;
            }
            _ => {}
        };

        if is_terminated {
            codes.remove(i);
            continue;
        };

        match code {
            OpCode::Jump { to: _, line: _ } => {
                is_terminated = true;
            }
            OpCode::Return { value: _ } => {
                is_terminated = true;
            }
            OpCode::Throw { value: _ } => is_terminated = true,
            _ => {}
        }
        i += 1;
    }
    return codes;
}

fn remove_unused_mov(mut codes: Vec<OpCode>) -> Vec<OpCode> {
    let mut i = 0;

    while i < codes.len() {
        let code = &codes[i];

        match code {
            OpCode::Mov { from, to } => {
                if *from == *to {
                    codes.remove(i);
                    continue;
                }
            }
            _ => {}
        };
        i += 1;
    }
    return codes;
}

pub fn remove_unuse_operations(mut codes: Vec<OpCode>) -> Vec<OpCode> {
    let mut i = 0;

    'outer: loop {
        if i + 1 == codes.len() {
            break;
        }

        let code = &mut codes[i];

        if let Some(w) = code.writes_to() {
            for idx in (i + 1)..codes.len() {
                let code = &codes[idx];
                let rs = code.reads_from();

                // the register is readed, continue
                if rs.contains(&w) {
                    i += 1;
                    continue 'outer;
                }

                // the register is overwrited, remove and continue
                if let Some(wr) = code.writes_to() {
                    if wr == w {
                        codes.remove(i);
                        continue 'outer;
                    }
                }
            }
        };
        i += 1;
    }

    return codes;
}

/// remove unecessary temp allocations
fn remove_unused_temp(mut codes: Vec<OpCode>) -> Vec<OpCode> {
    // (index, reads_from, reg_is_writed, count)
    let mut temps: Vec<(usize, Register, bool, Vec<(usize, Register)>, [bool; 3])> = Vec::new();

    let mut i = 0;

    while i < codes.len() {
        let code = &codes[i];
        match code {
            OpCode::StoreTemp { value } => {
                temps.push((i, *value, false, Vec::new(), [false; 3]));
            }
            OpCode::ReadTemp { value } => {
                let v = temps.last_mut().unwrap();
                v.3.push((i, *value));
            }
            OpCode::ReleaseTemp => {
                let (idx, reg, is_writed, count, use_reg) = temps.pop().unwrap();

                if count.len() == 0 {
                    codes.remove(i);
                    codes.remove(idx);

                    i -= 2;
                } else if !is_writed {
                    for (index, write_to) in count {
                        codes[index] = OpCode::Mov {
                            from: reg,
                            to: write_to,
                        };
                    }
                    codes.remove(i);
                    codes.remove(idx);
                    i -= 2;
                    
                } else if use_reg != [true; 3] {
                    let mut not_used_reg = 0;

                    for i in 0..3 {
                        if !use_reg[i as usize] {
                            not_used_reg = i
                        }
                    }

                    codes[idx] = OpCode::Mov {
                        from: reg,
                        to: Register(not_used_reg),
                    };

                    for (index, write_to) in count {
                        codes[index] = OpCode::Mov {
                            from: Register(not_used_reg),
                            to: write_to,
                        };
                    }
                    codes.remove(i);
                    i -= 1;
                }
            }
            code => {
                if let Some((_, reg, is_writed, _, use_reg)) = temps.last_mut() {
                    if let Some(r) = code.writes_to() {
                        if *reg == r {
                            *is_writed = true;
                        }
                        use_reg[r.0 as usize] = true;
                    }
                };
            }
        };
        i += 1;
    }

    return codes;
}

/// remove store_temp that reads from stack and replace them with read stack
fn remove_temp_read_from_stack(mut codes: Vec<OpCode>) -> Vec<OpCode> {
    let mut temps = Vec::new();
    let mut i = 0;

    while i < codes.len() {
        let code = &codes[i];

        match code {
            OpCode::ReadFromStack {
                result,
                stack_offset,
            } => {
                if let Some(c) = codes.get(i + 1) {
                    match c {
                        OpCode::StoreTemp { value } => {
                            if *value == *result {
                                temps.push(Some((i + 1, *stack_offset)))
                            } else {
                                temps.push(None);
                            };
                            // advence index by one
                            i += 1;
                        }
                        _ => {}
                    }
                }
            }
            OpCode::StoreTemp { value: _ } => {
                temps.push(None);
            }
            OpCode::ReadTemp { value } => {
                let v = temps.last().unwrap();

                if let Some((_, stack_offset)) = v {
                    codes[i] = OpCode::ReadFromStack {
                        result: *value,
                        stack_offset: *stack_offset,
                    };
                }
            }
            OpCode::ReleaseTemp => {
                let v = temps.last().unwrap();

                if let Some((line, _)) = v {
                    codes.remove(i);
                    codes.remove(*line);

                    i -= 2;
                }
            }
            _ => {}
        };
        i += 1;
    }

    return codes;
}

fn induction_variable_pass(mut codes: Vec<OpCode>) -> Vec<OpCode> {
    let mut i = 0;
    for code in &codes {
        match code {
            OpCode::LtEqImmI32 {
                result,
                left,
                right,
            }
            | OpCode::LtImmI32 {
                result,
                left,
                right,
            } => {
                let srcs = source_of(&codes, i);
            }
            _ => {}
        };
        i += 1;
    }
    return codes;
}

// some bugs to be fixed
fn inline_static_function(mut codes: Vec<OpCode>) -> Vec<OpCode> {
    let mut static_functions: HashMap<u16, FuncID> = Default::default();
    let mut largest_block = 0;
    let mut current_stack_offset = 0;
    let mut i = 0;

    while i < codes.len() {
        let code = &codes[i];
        match code {
            OpCode::WriteToStack {
                from: _,
                stack_offset,
            } => {
                current_stack_offset = current_stack_offset.max(*stack_offset as u16);
            }
            OpCode::ReadFromStack {
                result: _,
                stack_offset,
            } => {
                current_stack_offset = current_stack_offset.max(*stack_offset as u16);
            }
            OpCode::DeclareDynamicVar {
                from: _,
                kind: _,
                offset,
            } => {
                current_stack_offset = current_stack_offset.max(*offset as u16);
            }
            OpCode::CreateBlock(b) => {
                largest_block = largest_block.max(b.0 + 1);
            }

            OpCode::CreateFunction { result, id } => {
                let decl = &codes[i + 1];
                match decl {
                    OpCode::DeclareDynamicVar {
                        from,
                        kind: _,
                        offset,
                    } => {
                        // this is a static function
                        if *from == *result {
                            static_functions.insert(*offset, *id);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        };
        i += 1;
    }

    let mut i = 0;
    'outer: while i < codes.len() {
        let code = &codes[i];
        match code {
            OpCode::ReadDynamicVarDeclared {
                result: callee_result,
                offset,
            } => {
                let load_this = &codes[i + 1];
                match load_this {
                    OpCode::LoadThis {
                        result: this_result,
                    } => {
                        let call = &codes[i + 2];
                        let call_code_index = i + 2;

                        match call {
                            OpCode::Call {
                                result: return_register,
                                this,
                                callee,
                                stack_offset,
                                args_len,
                            } => {
                                let return_register = *return_register;
                                let call_stack_offset = *stack_offset;

                                if *this == *this_result
                                    && *callee == *callee_result
                                    && static_functions.contains_key(&offset)
                                {
                                    let func_id = static_functions.get(&offset).unwrap();

                                    let mut reverse = 1;

                                    let mut arguments = Vec::new();

                                    // reverse loop until create args reached
                                    loop {
                                        let code = &codes[i - reverse];
                                        match code {
                                            OpCode::CreateArg { stack_offset, len } => {
                                                break;
                                            }
                                            OpCode::PushArg {
                                                value,
                                                stack_offset,
                                            } => {
                                                arguments.insert(
                                                    0,
                                                    (i - reverse, *value, *stack_offset),
                                                );
                                            }
                                            OpCode::PushArgSpread {
                                                value: _,
                                                stack_offset: _,
                                            } => {
                                                // todo: inlining spread
                                                i += 1;
                                                continue 'outer;
                                            }
                                            _ => {}
                                        }
                                        reverse += 1;
                                    }

                                    for i in &arguments {
                                        let c = codes.get_mut(i.0).unwrap();
                                        *c = OpCode::WriteToStack {
                                            from: i.1,
                                            stack_offset: i.2,
                                        };
                                    }

                                    let runtime = Runtime::current();
                                    let func = runtime.get_function(*func_id).unwrap();
                                    let mut func_codes = func.bytecodes.to_vec();

                                    let mut exit = Block(0);

                                    for code in &func_codes {
                                        match code {
                                            OpCode::CreateBlock(b) => {
                                                let i = exit.0.max(b.0 + largest_block);
                                                exit = Block(i);
                                            }
                                            _ => {}
                                        }
                                    }

                                    let func_block = Block(exit.0 + 1);

                                    for code in &mut func_codes {
                                        match code {
                                            OpCode::ReadParam { result, index } => {
                                                let result = *result;

                                                if let Some(arg) = arguments.get(*index as usize) {
                                                    *code = OpCode::ReadFromStack {
                                                        result: result,
                                                        stack_offset: arg.2,
                                                    }
                                                } else {
                                                    *code = OpCode::LoadUndefined { result: result }
                                                }
                                            }
                                            OpCode::Return { value } => {
                                                let v = *value;
                                                *code = OpCode::Jump { to: exit, line: 0 };
                                            }

                                            OpCode::CreateBlock(b) => {
                                                *b = Block(b.0 + largest_block);
                                            }
                                            OpCode::SwitchToBlock(b) => {
                                                *b = Block(b.0 + largest_block);
                                            }
                                            OpCode::Jump { to, line } => {
                                                *to = Block(to.0 + largest_block);
                                            }
                                            OpCode::JumpIfFalse { value, to, line } => {
                                                *to = Block(to.0 + largest_block);
                                            }
                                            OpCode::JumpIfTrue { value, to, line } => {
                                                *to = Block(to.0 + largest_block);
                                            }
                                            OpCode::EnterTry { catch_block, line } => {
                                                *catch_block = Block(catch_block.0 + largest_block);
                                            }

                                            OpCode::ReadFromStack { stack_offset, .. } => {
                                                *stack_offset = *stack_offset
                                                    + call_stack_offset
                                                    + arguments.len() as u16;
                                            }
                                            OpCode::WriteToStack { stack_offset, .. } => {
                                                *stack_offset = *stack_offset
                                                    + call_stack_offset
                                                    + arguments.len() as u16;
                                            }
                                            OpCode::CreateArg { stack_offset, .. } => {
                                                *stack_offset = *stack_offset
                                                    + call_stack_offset
                                                    + arguments.len() as u16;
                                            }
                                            OpCode::PushArg { stack_offset, .. } => {
                                                *stack_offset = *stack_offset
                                                    + call_stack_offset
                                                    + arguments.len() as u16;
                                            }
                                            OpCode::PushArgSpread {
                                                value:_,
                                                stack_offset,
                                            } => {
                                                *stack_offset = *stack_offset
                                                    + call_stack_offset
                                                    + arguments.len() as u16;
                                            }
                                            OpCode::Call { stack_offset, .. } => {
                                                *stack_offset = *stack_offset
                                                    + call_stack_offset
                                                    + arguments.len() as u16;
                                            }
                                            _ => {}
                                        }
                                    }

                                    /*
                                    ...
                                    create_block @exit
                                    create_block @func
                                    jump @func
                                    switch_to_block @func
                                     ...func_codes

                                     switch_to_block @exit
                                      ...
                                    */
                                    codes.remove(call_code_index);
                                    let origin_end = codes.len();

                                    codes.resize(codes.len() + func_codes.len() + 5, OpCode::NoOp);

                                    codes.copy_within(
                                        call_code_index..origin_end,
                                        call_code_index + func_codes.len() + 5,
                                    );

                                    codes[call_code_index] = OpCode::CreateBlock(exit);
                                    codes[call_code_index + 1] = OpCode::CreateBlock(func_block);
                                    codes[call_code_index + 2] = OpCode::Jump {
                                        to: func_block,
                                        line: 0,
                                    };
                                    codes[call_code_index + 3] = OpCode::SwitchToBlock(func_block);

                                    (&mut codes[call_code_index + 4
                                        ..call_code_index + 4 + func_codes.len()])
                                        .copy_from_slice(&func_codes);

                                    codes[call_code_index + 4 + func_codes.len()] =
                                        OpCode::SwitchToBlock(exit);
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        i += 1;
    }

    return codes;
}

fn resolve_jump_locations(mut codes: Vec<OpCode>) -> Vec<OpCode> {
    let mut blocks: HashMap<Block, usize> = Default::default();

    for i in 0..codes.len() {
        let code = &codes[i];
        match code {
            OpCode::SwitchToBlock(b) => {
                blocks.insert(*b, i);
            }
            _ => {}
        }
    }

    for i in 0..codes.len() {
        let code = &mut codes[i];
        match code {
            OpCode::Jump { to, line } => {
                *line = blocks[to] as u32;
            }
            OpCode::JumpIfFalse { value: _, to, line } => {
                *line = blocks[to] as u32;
            }
            OpCode::JumpIfTrue { value: _, to, line } => {
                *line = blocks[to] as u32;
            }
            OpCode::EnterTry { catch_block, line } => {
                *line = blocks[catch_block] as u32;
            }
            _ => {}
        }
    }
    return codes;
}
