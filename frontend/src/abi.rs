use crate::kind::Kind;

#[derive(Clone, Copy, Debug)]
pub struct Param {
    pub kind: Kind,
}

impl Param {
    pub fn new(kind: Kind) -> Param {
        Param { kind: kind }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Linkage {
    /// Declared outside of module
    Import(String),
    /// Declared in module
    Local,
    Extern(*const u8),
    /// Declared in dynamic library
    DynamicImport(String),
    /// Declared in static linked library
    /// WARNING: Unimplemented
    StaticImport,
}

impl Linkage {
    pub fn is_dynamic(&self) -> bool {
        match self.clone() {
            Linkage::DynamicImport(_) => true,
            _ => false,
        }
    }

    pub fn is_import(&self) -> bool {
        match self.clone() {
            Linkage::Import(_) => true,
            _ => false,
        }
    }

    pub fn is_extern(&self) -> bool {
        match self.clone() {
            Linkage::Extern(_) => true,
            _ => false,
        }
    }
}
