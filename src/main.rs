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
