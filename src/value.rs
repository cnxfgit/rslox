use crate::object::Obj;

#[derive(Clone, Copy)]
pub enum Value {
    Nil,
    Boolean(bool),
    Number(f64),
    Object(*mut Obj),
}

impl Value {
    pub fn print(&self) {}
}

#[macro_export]
macro_rules! is_obj {
    ($val:expr) => {{
        match $val {
            Value::Nil | Value::Boolean(_) | Value::Number(_) => false,
            Value::Object(_) => true,
        }
    }};
}

#[macro_export]
macro_rules! as_obj {
    ($val:expr) => {{
        if let Value::Object(obj) =  $val {
            obj
        } else {
            panic!("as_obj! error")
        }
    }};
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
