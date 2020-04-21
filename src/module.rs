use crate::ir::*;
use std::collections::HashMap;
pub struct Module {
    pub functions: HashMap<String, LIRFunction>,
    /// Strings and other stuff.
    pub data: HashMap<String, Vec<u8>>,
}

impl Module {
    pub fn add_function(&mut self, f: LIRFunction) -> Result<(), String> {
        if self.functions.contains_key(&f.signature.name) {
            return Err(format!(
                "Function with name '{}' already exists.",
                f.signature.name
            ));
        }
        self.functions.insert(f.signature.name.clone(), f);
        Ok(())
    }

    pub fn add_data(&mut self, name: &str, data: &[u8]) -> Result<(), String> {
        if self.data.contains_key(name) {
            return Err(format!("Data '{}' already exists.", name));
        }
        self.data.insert(name.to_owned(), data.to_vec());
        Ok(())
    }
}
