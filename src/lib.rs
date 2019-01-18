#![allow(non_upper_case_globals)]

pub mod compiler;
pub mod context;
pub mod dfg;
pub mod extsymbol;
pub mod function;
pub mod inst;
pub mod module;
pub mod types;
pub mod utils;

#[macro_export]
macro_rules! impl_entity {
    ($e:ident) => {
        impl $crate::EntityRef for $e {
            fn new(idx: usize) -> Self {
                $e(idx as u32)
            }
            fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

#[derive(Clone, Debug, PartialEq, Eq, Copy, PartialOrd, Ord, Hash)]
pub struct Value(u32);

#[derive(Clone, Debug, PartialEq, Eq, Copy, PartialOrd, Ord, Hash)]
pub struct Variable(u32);

impl_entity!(Variable);
impl_entity!(Value);

pub trait EntityRef {
    fn index(self) -> usize;
    fn new(idx: usize) -> Self;
}
