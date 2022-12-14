# rusty-js
 a JIT Javascript Engine written in rust. It is an experiental enigine inspired by golang's insterface type.

## Overview

This Javascript Engine is an embeddable written in pure rust. It supports the ES2022 Syntax including modules, asynchronous functions, generators and more.
Unlike traditionsl engines that uses nan-boxing, Rusty uses dynamic dispatch to run operations on value, determination(branching) of types during runtime is no longer necessary, the jitted code size is therefore reduced.

#### value representation
```rust
#[cfg(target_pointer_size = "64")]
struct JSValue{
   value:u64,
   vtable:*const Vtable
}
```
#### Jitted add operation
```rust
let a:JSValue = 0.into();
let b:JSValue = 1.into();

let result = a.vtable.add(a.value, b); // returns 1
```

## JIT stages
|               | Progress |  Backend  |     Missing features     |
| ------------- | -------- | --------- | ------------------------ |
| interpreter   | 70%      | Bytecode  | Speculation              |
| baseline jit  | 50%      | Cranelift | fallback to interpreter  |
| optimise jit  | 0%       | ?         |                          |

## Missing Features
* ### Regex
* ### Class optimisation
* ### TypeScript

## Async and generator support
|         | ELF (Linux, BSD, bare metal, etc) | Darwin (macOS, iOS, etc) | Windows |
| ------- | --------------------------------- | ------------------------ | ------- |
| x86_64  | ✅                                 | ✅                        | ✅       |
| x86     | ✅                                 | ❌                        | ✅       |
| AArch64 | ✅                                 | ✅                        | ❌       |
| ARM     | ✅                                 | ❌                        | ❌       |
| RISC-V  | ✅                                 | ❌                        | ❌       |

## Ecma Roadmap
for more information, see [here](https://github.com/YC-Lammy/rusty-js/projects).
