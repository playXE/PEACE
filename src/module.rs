use std::collections::HashMap;
use string_interner::Sym;

pub struct Module {
    functions: HashMap<Sym, Sym>,
}
