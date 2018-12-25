# PEACE - Peak Compiler

PEACE is a library for x86_64 machine code generation.

# TODO
- Implement basic binary and unary operations
- Better register allocation
- Write a IR code parser
- Write a AOT backend

# Known problems
- You cannot use pointer that you allocated in your JIT code,e.g
```llvm
struct Point { i32 x,i32 y}

%size = iconst.i64 8

%ptr = call_indirect $malloc(%size)  ; create pointer with size of Point
%v1 = iconst.i32 2
; Store %v1(Point.x)
%ptr + 0 = %v1 ; Segfault there
```
- You can't declare and use statically linked functions


# Supported platforms
- Windows x64
- Linux x64
- Linux x32 ( You must use 32 bit registers  (eax instead of rax and etc and use Int32 instead of Int64)
- Windows x32 ( You must use 32 bit registers (eax instead of rax and etc and use Int32 instead of Int64)

# Example of use
```rust
extern crate peace;

use self::peace::prelude::function::*;
use self::peace::prelude::module::*;
use self::peace::prelude::kind::*;
use self::peace::prelude::abi::Linkage;

extern "C" {
    fn printf();
}

fn main() {
    let mut module = Module::new();

    module.add_function(Function::new("main",Linkage::Local));
    module.add_function(Function::new("printf",Linkage::Extern(printf as *const u8)));

    let main_func = module.get_mut_func("main".to_string());

    let cstring = main_func.iconst(b"Hello,world!\n\0".as_ptr() as i64,Int64); // Int64 is a pointer too
    main_func.call_indirect("printf".into(),&[cstring],Int32);
    let null = main_func.iconst(0,Int32);
    main_func.ret(null);

    module.finish(); // compile all functions or resolve imports

    let data = module.get_data("main".to_string());

    let ptr = data.ptr();

    let func: fn() -> i32 = unsafe {::std::mem::transmute(ptr)};
    func();
}


```

