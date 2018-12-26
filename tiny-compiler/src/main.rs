extern crate tiny_compiler;

use self::tiny_compiler::parser::*;
use capstone::arch::*;
use capstone::*;

extern "C" {
    fn puts(c: *const u32) -> i32;
    fn putc();
    fn getc();
}

fn rust_fn<'a>(x: &'a [u8]) -> i32 {
    unsafe { puts(x.as_ptr() as *const _) };
    return 0;
}

use std::{fs::File, io::prelude::*, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Options {
    #[structopt(name = "FILE", parse(from_os_str))]
    file: Option<PathBuf>,
    #[structopt(short = "d", long = "debug")]
    debug: bool,
}

fn main() {
    let mut src = String::new();

    let ops = Options::from_args();

    if let Some(path) = ops.file {
        File::open(path).unwrap().read_to_string(&mut src).unwrap();
    } else {
        panic!("You should enter file path");
    }

    let lex = lex(&src);
    let parse = parse(&mut lex.peekable()).unwrap();

    let mut compiler = Compiler::new();
    let ptr = puts as *const u8;
    //println!("{:?}",ptr);
    compiler.debug = ops.debug;
    compiler.declare_ptr_func(
        rust_fn as *const u8,
        "print",
        TypeSpec {
            name: String::from("int"),
            is_ref: false,
        },
    );
    compiler.declare_ptr_func(
        putc as *const u8,
        "putc",
        TypeSpec {
            name: String::from("int"),
            is_ref: false,
        },
    );
    compiler.declare_ptr_func(
        getc as *const u8,
        "getc",
        TypeSpec {
            name: String::from("int"),
            is_ref: false,
        },
    );
    compiler.declare_ptr_func(
        ptr as *const u8,
        "printf",
        TypeSpec {
            name: String::from("int"),
            is_ref: false,
        },
    );
    let mem = compiler.compile(parse);
    let f: fn() -> i32 = unsafe { ::std::mem::transmute(mem.ptr()) };

    println!("{}", f());
}

extern crate peace;

use self::abi::*;
use self::function::*;
use self::kind::*;
use self::module::*;
use self::peace::prelude::*;
use self::sink::Memory;

pub struct Compiler {
    fdecls: HashMap<String, FnDef>,
    fvars: HashMap<String, HashMap<String, Variable>>,
    module: Module,
    pub debug: bool,
    is_builtin: HashMap<String, bool>,
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler {
            fdecls: HashMap::new(),
            fvars: HashMap::new(),
            module: Module::new(),
            is_builtin: HashMap::new(),
            debug: false,
        }
    }

    pub fn declare_ptr_func(&mut self, ptr: *const u8, name: &str, ret: TypeSpec) {
        self.module
            .add_function(Function::new(name, Linkage::Extern(ptr)));
        let fndef = FnDef {
            name: Box::new(Expr::Identifier(name.to_string())),
            ret_ty: ret,
            body: Box::new(Stmt::Block(vec![])),
            params_: HashMap::new(),
            params: vec![],
        };
        self.is_builtin.insert(name.to_owned(), true);
        self.fdecls.insert(name.to_owned(), fndef);
    }

    pub fn compile<'r>(&'r mut self, globals: Vec<Global>) -> &'r Memory {
        let mut vidxs = HashMap::new();
        //self.module.add_function(Function::new("printf",Linkage::Extern(printf as *const u8)));

        for global in globals.iter() {
            match global {
                Global::FnDefenition(fdef) => {
                    let fname = *fdef.name.clone();
                    let name = match fname {
                        Expr::Identifier(name) => name.clone(),
                        _ => unimplemented!(),
                    };
                    self.fdecls.insert(name.clone(), fdef.clone());
                    self.module
                        .add_function(Function::new(&name.clone(), Linkage::Local));

                    let func = self.module.get_mut_func(name.clone());
                    let mut vidx: usize = 0;
                    let params = {
                        let mut fvars = HashMap::new();
                        let mut temp = vec![];
                        for (name, typ) in fdef.params_.iter() {
                            let kind = to_peace_type(typ.clone());
                            temp.push(kind);

                            fvars.insert(name.clone(), Variable::new(vidx));
                            vidx += 1;
                        }

                        self.fvars.insert(name.clone(), fvars);
                        temp
                    };

                    vidxs.insert(name.clone(), vidx);

                    func.add_params(params);
                }
                _ => unimplemented!(),
            }
        }

        for (name, func) in self.fdecls.iter() {
            if is_builtin_func(name) {
                continue;
            }

            if !self.is_builtin.contains_key(name) {
                let vars = self.fvars.get(name).expect("Vars not found");
                let vidx = *vidxs.get(name).expect("VIDX not found");
                let mut translator = FunctionTranslator {
                    module: &mut self.module,
                    fdecls: self.fdecls.clone(),
                    variables: vars.clone(),
                    vidx: vidx as u32,
                    layouts: Vec::new(),
                    fname: name.clone(),
                };

                translator.translate_stmt(func.body.clone());
            } //translator.translate_stmt(Box::new(Stmt::Return));
        }

        self.module.finish();
        if self.debug {
            for (name, _) in self.fdecls.iter() {
                if self.is_builtin.contains_key(name) {
                    continue;
                }
                println!("Disassemble of `{}` function", name);
                let mem = self.module.get_data(name.to_string());
                let buf: &[u8] = unsafe { ::std::slice::from_raw_parts(mem.ptr(), mem.size()) };

                let mut cs = Capstone::new()
                    .x86()
                    .mode(arch::x86::ArchMode::Mode64)
                    .syntax(arch::x86::ArchSyntax::Intel)
                    .detail(true)
                    .build()
                    .expect("Failed to create Capstone object");

                let insns = cs.disasm_all(buf, mem.ptr() as u64);
                for i in insns.iter() {
                    println!("{}", i);
                }
            }
        }

        let mem = self.module.get_data("main".into());
        mem
    }
}

use std::collections::HashMap;

pub struct FunctionTranslator<'a> {
    pub module: &'a mut Module,
    variables: HashMap<String, Variable>,
    layouts: Vec<(Value, Kind)>,
    fdecls: HashMap<String, FnDef>,
    vidx: u32,
    fname: String,
}

fn is_builtin_func(s: &str) -> bool {
    match s {
        "printf" => true,
        "puts" => true,
        _ => false,
    }
}

fn to_peace_type(spec: TypeSpec) -> Kind {
    match spec.name.as_str() {
        "int" => Int32,
        "int32" => Int32,
        "int64" => Int64,
        "pointer" => Pointer,
        "float" => Float32,
        "float32" => Float32,
        "float64" => Float64,
        _ => panic!("Unknown type"),
    }
}

impl<'a> FunctionTranslator<'a> {
    pub fn translate_stmt(&mut self, stmt: Box<Stmt>) {
        let stmt: Stmt = *stmt;
        match stmt {
            Stmt::Return => {
                let func = self.module.get_mut_func(self.fname.clone());
                func.ret_void();
            }
            Stmt::ReturnWithVal(val) => {
                self.translate_expr(val);
                let val = self.layouts.pop().unwrap();

                let func = self.module.get_mut_func(self.fname.clone());
                func.ret(val.0);
            }
            Stmt::Expr(expr) => {
                self.translate_expr(expr);
            }

            Stmt::Block(stmts) => {
                for stmt in stmts.iter() {
                    self.translate_stmt(Box::new(stmt.clone()));
                }
            }
            Stmt::Var(name, typ, init) => {
                let kind = to_peace_type(typ);
                if init.is_some() {
                    self.translate_expr(init.clone().expect("init is none"));
                }
                let val = self.layouts.pop().unwrap();
                let func = self.module.get_mut_func(self.fname.clone());
                let var = func.declare_variable(self.vidx, kind);
                self.variables.insert(name.clone(), var);

                self.vidx += 1;
                if init.is_some() {
                    let func = self.module.get_mut_func(self.fname.clone());

                    func.def_var(var, val.0);
                }
            }

            _ => unimplemented!(),
        }
    }

    pub fn translate_expr(&mut self, expr: Box<Expr>) {
        let expr: Expr = *expr;

        match expr {
            Expr::FnCall(fname, args) => {
                let args = {
                    let mut temp = vec![];
                    for arg in args.iter() {
                        self.translate_expr(Box::new(arg.clone()));
                        let val = self.layouts.pop().expect("arg not found");
                        temp.push(val.0);
                    }

                    temp
                };
                let fdef = self
                    .fdecls
                    .get(&fname)
                    .expect(&format!("Function definition not found `{}`", fname));
                let kind = to_peace_type(fdef.ret_ty.clone());
                let func = self.module.get_mut_func(self.fname.clone());
                let ret = func.call_indirect(&fname, &args, kind);
                self.layouts.push((ret, kind));
                /*} else {
                    let func = self.module.get_mut_func(self.fname.clone());
                    let ret = func.call_indirect(&fname,&args,Int64);
                    self.layouts.push((ret,Int64));
                }*/
            }

            Expr::IntConst(i) => {
                let func = self.module.get_mut_func(self.fname.clone());

                let value = func.iconst(i, Int32);
                self.layouts.push((value, Int32));
            }
            Expr::Op(op, lhs, rhs) => {
                self.translate_expr(lhs);
                self.translate_expr(rhs);
                let rhs = self.layouts.pop().unwrap();
                let lhs = self.layouts.pop().unwrap();

                if rhs.1 != lhs.1 {
                    panic!("Expected `{:?}`,got `{:?}`", lhs.1, rhs.1);
                }
                let func = self.module.get_mut_func(self.fname.clone());
                let value = match op {
                    Op::Add => func.iadd(lhs.0, rhs.0),
                    Op::Sub => func.isub(lhs.0, rhs.0),
                    Op::Div => func.idiv(lhs.0, rhs.0),
                    Op::Mul => func.imul(lhs.0, rhs.0),
                    _ => unimplemented!(),
                };
                self.layouts.push((value, lhs.1));
            }

            Expr::Identifier(ref name) => {
                let func = self.module.get_mut_func(self.fname.clone());
                let var = self
                    .variables
                    .get(name)
                    .expect(&format!("Not found {}", name))
                    .clone();
                let val = func.use_var(var);

                self.layouts.push((val, Int32));
            }
            Expr::Assignment(ref from, ref to) => {
                let from: Expr = *from.clone();
                match from {
                    Expr::Identifier(ref name) => {
                        let var = self
                            .variables
                            .get(name)
                            .expect("Variable doens't exitsts")
                            .clone();
                        self.translate_expr(Box::new(*to.clone()));
                        let val = self.layouts.pop().expect("Value doesn't exists");
                        let func = self.module.get_mut_func(self.fname.clone());

                        func.def_var(var, val.0);
                    }
                    _ => unimplemented!(),
                }
            }
            Expr::StringConst(ref string) => {
                let func = self.module.get_mut_func(self.fname.clone());

                let ptr = string.as_bytes().as_ptr();
                let val = func.iconst(ptr as i64, Int64);
                self.layouts.push((val, Pointer));
            }
            v => panic!("Unimplemented {:?}",v),
        }
    }
}
