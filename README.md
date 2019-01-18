# PEACE

PEACE (Peak Compiler) - library for emitting x86_64 code

# Examples
```rust
module.declare_function("main",Linkage::Local);
module.declare_function("puts",Linkage::Import);

let builder = module.get_function("main");

let str = builder.iconst(I64,b"Hello,world!\0".as_ptr() as i64);
let ret = builder.call("puts",&[str],I32);
builder.ret(ret);


```


# Features
- Register allocation on the fly

```rust
let int = I32;

let v1 = func.iconst(int,4);
let v2 = func.iconst(int,2);
let v3 = func.imul(v1,v2);
let v4 = func.iconst(int,4);
let v5 = func.iadd(v4,v3);
func.ret(v5);

```
Assembly:
```assembly
0x0: pushq %rbp
0x1: movq %rsp, %rbp
0x4: movl $4, %ebx
0x9: movl $2, %r8d
0xf: imull %r8d, %ebx
0x13: movl $4, %r8d
0x19: addl %ebx, %r8d
0x1c: movl %r8d, %ebx
0x1f: movl %ebx, %eax
0x21: popq %rbp
0x22: retq
```

# Limitations

- There are no liveness analysis,after using `Value` just "destroyed"
- Only integers for now supported


