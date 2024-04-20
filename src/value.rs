use crate::object::{Obj, ObjType, Object};

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

pub fn as_obj(value: Value) -> *mut Obj {
    if let Value::Object(obj) = value {
        obj.clone()
    } else {
        panic!("as_obj error")
    }
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
    pub fn print(&self) {
        match self {
            Value::Boolean(b) => print!("{}", if *b { "true" } else { "false" }),
            Value::Nil => print!("nil"),
            Value::Number(n) => print!("{}", n),
            Value::Object(obj) => unsafe { (*(*obj)).print() },
        }
    }

    pub fn is_obj_type(&self, type_: ObjType) -> bool {
        is_obj!(self) && unsafe { (*as_obj(self.clone())).type_ == type_ }
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
