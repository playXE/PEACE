use crate::align;
use std::mem::size_of;

#[derive(Debug, Clone)]
pub struct DSeg {
    entries: Vec<Entry>,
    size: i32,
}

#[derive(Debug, Clone)]
struct Entry {
    disp: i32,
    value: Value,
}

#[derive(Debug, PartialEq, Clone)]
enum Value {
    Ptr(*const u8),
    Float(f32),
    Double(f64),
    Int(i32),
}

impl Value {
    pub fn size(&self) -> i32 {
        match self {
            &Value::Ptr(_) => size_of::<*const u8>() as i32,
            &Value::Int(_) => size_of::<i32>() as i32,
            &Value::Float(_) => size_of::<f32>() as i32,
            &Value::Double(_) => size_of::<f64>() as i32,
        }
    }
}

impl DSeg {
    pub fn new() -> DSeg {
        DSeg {
            entries: Vec::new(),
            size: 0,
        }
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    fn add_value(&mut self, v: Value) -> i32 {
        let size = v.size();
        self.size = align(self.size() + size, size);
        let entry = Entry {
            disp: self.size(),
            value: v,
        };

        self.entries.push(entry);
        self.size
    }

    pub fn finish(&self, ptr: *const u8) {
        for entry in &self.entries {
            let offset = self.size - entry.disp;

            unsafe {
                let entry_ptr = ptr.offset(offset as isize);

                match entry.value {
                    Value::Ptr(v) => *(entry_ptr as *mut (*const u8)) = v,
                    Value::Float(v) => {
                        *(entry_ptr as *mut f32) = v;
                    }

                    Value::Double(v) => {
                        *(entry_ptr as *mut f64) = v;
                    }

                    Value::Int(v) => {
                        *(entry_ptr as *mut i32) = v;
                    }
                }
            }
        }
    }

    pub fn add_addr_reuse(&mut self, ptr: *const u8) -> i32 {
        for entry in &self.entries {
            if entry.value == Value::Ptr(ptr) {
                return entry.disp;
            }
        }

        self.add_addr(ptr)
    }

    pub fn add_int(&mut self, value: i32) -> i32 {
        self.add_value(Value::Int(value))
    }

    pub fn add_addr(&mut self, value: *const u8) -> i32 {
        self.add_value(Value::Ptr(value))
    }

    pub fn add_double(&mut self, value: f64) -> i32 {
        self.add_value(Value::Double(value))
    }

    pub fn add_float(&mut self, value: f32) -> i32 {
        self.add_value(Value::Float(value))
    }

    pub fn align(&mut self, size: i32) -> i32 {
        assert!(size > 0);
        self.size = align(self.size, size);

        self.size
    }
}
