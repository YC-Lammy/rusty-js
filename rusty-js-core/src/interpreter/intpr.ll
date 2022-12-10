
%Result = type{i64, i1}

define void @run_codes(ptr %codes, i64 %codes_len, ptr %result) {
    %R0 = alloca i64;
    %R1 = alloca i64;
    %R2 = alloca i64;

    %counter = alloca i64
    store i64 0, i64* %counter

    %next_opcode = alloca i64

    br label %LoopStart

LoopStart:

    %count = load i64, i64* %counter
    %is_end = icmp eq i64 %count, %codes_len
    %pointer_offset = mul i64 8, %count

    %pointer = ptrtoint ptr %codes to i64
    %pointer_added = add i64 %pointer_offset, %pointer
    %next_op_pointer = inttoptr i64 %pointer_added to ptr

    %loaded_next_opcode = load i64, ptr %next_op_pointer
    store i64 %loaded_next_opcode, i64* %next_opcode

    %added_count = add i64 1, %count
    store i64 %added_count, i64* %counter

    br i1 %is_end, label %RunCode, label %IfEnded

RunCode:
    %opcode = load i64, i64* %next_opcode
    switch i64 %opcode, label %IfUnknownCode [
        i64 0, label %NoOp
        i64 1, label %Span
        i64 2, label %CreatBlock
        i64 3, label %SwitchToBlock
        i64 4, label %Jump
        i64 5, label %JumpIfTrue
    ]
    
IfEnded:
    ret void
IfUnknownCode:
    store %Result {i64 0, i1 1}, ptr %result
    ret void

NoOp:
    br label %LoopStart
Span:
    br label %LoopStart

CreatBlock:
    br label %LoopStart
SwitchToBlock:
    br label %LoopStart
Jump:
    store i64 0, i64* %counter

    br label %LoopStart

JumpIfTrue:
    br label %LoopStart
}