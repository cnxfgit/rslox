use std::{collections::HashMap, ptr::write};

use crate::{memory::allocate, object::ObjString, value::Value};

pub struct Table {
    pub map: HashMap<*mut ObjString, Value>,
}

impl Table {
    pub fn new() -> *mut Table {
        let ptr = allocate::<Table>(1);
        unsafe {
            write(ptr as *mut HashMap<*mut ObjString, Value>, HashMap::new());
        }

        ptr
    }

    pub fn get(&self, key: *mut ObjString) -> Option<&Value> {
        self.map.get(&key)
    }

    pub fn set(&mut self, key: *mut ObjString, value: Value) -> bool {
        match self.map.insert(key, value) {
            Some(_) => false,
            None => true,
        }
    }

    pub fn remove(&mut self, key: *mut ObjString) {
        self.map.remove(&key);
    }

    pub fn get_key(&self, key: *mut ObjString) -> Option<*mut ObjString> {
        match self.map.get_key_value(&key) {
            Some(kv) => Some(kv.0.clone()),
            None => None,
        }
    }

    pub fn add_all(&mut self, from: &Table) {
        self.map.extend(from.map.clone().into_iter())
    }
}
