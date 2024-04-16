use crate::object::{Obj, ObjType};

#[derive(Clone, Copy)]
pub enum Value {
    Nil,
    Boolean(bool),
    Number(f64),
    Object(*mut Obj),
}


#[macro_export]
macro_rules! is_obj {
    ($val:expr) => {{
        match $val {
            Value::Object(_) => true,
            _ => false,
        }
    }};
}

#[macro_export]
macro_rules! is_number {
    ($val:expr) => {{
        match $val {
            Value::Number(_) => true,
            _ => false,
        }
    }};
}

#[macro_export]
macro_rules! as_obj {
    ($val:expr) => {{
        if let Value::Object(obj) = $val {
            obj.clone()
        } else {
            panic!("as_obj! error")
        }
    }};
}

#[macro_export]
macro_rules! as_number {
    ($val:expr) => {{
        if let Value::Number(n) = $val {
            n
        } else {
            panic!("as_number! error")
        }
    }};
}

#[macro_export]
macro_rules! obj_val {
    ($val:expr) => {
        Value::Object($val as *mut Obj) 
    };
}

impl Value {
    pub fn print(&self) {}

    pub fn is_obj_type(&self, type_: ObjType) -> bool {
        is_obj!(self) && (unsafe { *as_obj!(self) }).type_ == type_
    }
}

pub struct ValueArray {
    pub values: Vec<Value>,
}

impl ValueArray {
    pub fn new() -> ValueArray {
        ValueArray { values: vec![] }
    }

    pub fn write_value(&mut self, value: Value) {
        self.values.push(value);
    }

    pub fn count(&self) -> usize {
        self.values.len()
    }
}
