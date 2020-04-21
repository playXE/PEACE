use crate::ir::*;
use crate::module::*;
use std::fs::File;
use std::io::*;
/// writes raw bytes from memory between from_address (inclusive) to to_address
/// (exclusive)
fn write_data_bytes(f: &mut File, from: usize, to: usize) {
    // use std::io::Write;

    if from < to {
        f.write("\t.byte ".as_bytes()).unwrap();

        let mut cursor = from;
        while cursor < to {
            let byte = unsafe { *(cursor as *const u8) };
            f.write_fmt(format_args!("0x{:x}", byte)).unwrap();

            cursor += 1 as usize;
            if cursor != to {
                f.write(",".as_bytes()).unwrap();
            }
        }

        f.write("\n".as_bytes()).unwrap();
    }
}

/// declares a global symbol with .global
fn directive_globl(name: String) -> String {
    format!(".globl {}", name)
}

fn directive_equiv(name: String, target: String) -> String {
    format!(".equiv {}, {}", name, target)
}

#[allow(dead_code)]
fn directive_comm(name: String, size: usize, align: usize) -> String {
    format!(".comm {},{},{}", name, size, align)
}

#[cfg(target_os = "linux")]
pub fn symbol(name: &String) -> String {
    name.clone()
}
#[cfg(target_os = "macos")]
pub fn symbol(name: &String) -> String {
    format!("_{}", name)
}

#[cfg(target_os = "linux")]
pub fn pic_symbol(name: &String) -> String {
    format!("{}@GOTPCREL", name)
}

#[cfg(target_os = "macos")]
pub fn pic_symbol(name: &String) -> String {
    symbol(&name)
}
