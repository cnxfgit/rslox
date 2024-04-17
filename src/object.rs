use std::{
    hash::Hash,
    ptr::{self, null_mut},
};

use crate::{
    chunk::Chunk,
    memory::{allocate, allocate_obj, dealloc},
    table::Table,
    value::Value,
    vm::vm,
};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ObjType {
    BoundMethod, // 绑定方法对象
    Class,       // 类对象
    Closure,     // 闭包对象
    Function,    // 函数对象
    Instance,    // 实例对象
    Native,      // 原生函数对象
    String,      // 字符串对象
    Upvalue,     // 闭包提升值对象
}

#[macro_export]
macro_rules! as_string {
    ($val:expr) => {{
        if let Value::Object(obj) = $val {
            if unsafe { (*obj).type_ } == ObjType::String {
                obj as *mut ObjString
            } else {
                panic!("as_string! error.");
            }
        } else {
            panic!("as_string! error.");
        }
    }};
}

#[macro_export]
macro_rules! is_instance {
    ($val:expr) => {
        $val.is_obj_type(ObjType::Instance)
    };
}

#[macro_export]
macro_rules! is_string {
    ($val:expr) => {
        $val.is_obj_type(ObjType::String)
    };
}

#[macro_export]
macro_rules! is_class {
    ($val:expr) => {
        $val.is_obj_type(ObjType::Class)
    };
}

#[macro_export]
macro_rules! as_instance {
    ($val:expr) => {
        as_obj!($val) as *mut ObjInstance
    };
}

#[macro_export]
macro_rules! as_native {
    ($val:expr) => {
        unsafe {
            let native = as_obj!($val) as *mut ObjNative;
            (*native).function
        }
    };
}

#[macro_export]
macro_rules! as_function {
    ($val:expr) => {
        as_obj!($val) as *mut ObjFunction
    };
}

#[macro_export]
macro_rules! as_bound_method {
    ($val:expr) => {
        as_obj!($val) as *mut ObjBoundMethod
    };
}

#[macro_export]
macro_rules! as_class {
    ($val:expr) => {
        as_obj!($val) as *mut ObjClass
    };
}

#[macro_export]
macro_rules! as_closure {
    ($val:expr) => {
        as_obj!($val) as *mut ObjClosure
    };
}

pub trait Object {
    fn obj_type(&self) -> ObjType;
}

macro_rules! obj_val {
    ($obj:expr) => {
        Value::Object($obj as *mut Obj)
    };
}

pub struct Obj {
    pub type_: ObjType,  // 对象类型
    pub is_marked: bool, // 是否被标记
    pub next: *mut Obj,  // 下一个对象
}

impl Object for Obj {
    fn obj_type(&self) -> ObjType {
        self.type_
    }
}

pub struct ObjFunction {
    obj: Obj,                 // 公共对象头
    pub arity: usize,         // 参数数
    pub upvalue_count: usize, // 提升值数
    pub chunk: Chunk,         // 函数的字节码块
    pub name: *mut ObjString, // 函数名
}

impl ObjFunction {
    pub fn new() -> *mut ObjFunction {
        let ptr = allocate_obj::<ObjFunction>(ObjType::Function);
        let chunk = Chunk::new();
        unsafe {
            (*ptr).arity = 0;
            (*ptr).upvalue_count = 0;
            (*ptr).name = null_mut();
            let chunk_ptr = &mut (*ptr).chunk;
            std::ptr::write(chunk_ptr, chunk);
        }

        ptr
    }
}

impl Object for ObjFunction {
    fn obj_type(&self) -> ObjType {
        self.obj.obj_type()
    }
}

pub type NativeFn = fn(usize, *mut Value) -> Value;

pub struct ObjNative {
    obj: Obj,               // 公共对象头
    pub function: NativeFn, // 原生函数指针
}

impl ObjNative {
    pub fn new(function: NativeFn) -> *mut ObjNative {
        let ptr = allocate_obj::<ObjNative>(ObjType::Native);
        unsafe {
            (*ptr).function = function;
        }

        ptr
    }
}

impl Object for ObjNative {
    fn obj_type(&self) -> ObjType {
        self.obj.obj_type()
    }
}

pub struct ObjString {
    obj: Obj,          // 公共对象头
    pub chars: String, // 字符串
}

impl ObjString {
    pub fn new(string: String) -> *mut ObjString {
        let ptr = allocate_obj::<ObjString>(ObjType::String);

        unsafe {
            let chars_ptr = &mut (*ptr).chars as *mut String;
            ptr::write(chars_ptr, string);
        }

        ptr
    }

    pub fn take_string(string: String) -> *mut ObjString {
        let new_string = ObjString::new(string);

        let result = vm().strings.get_key(new_string);
        if let Some(s) = result {
            dealloc(new_string, 1);
            return s;
        }

        vm().push(obj_val!(new_string));
        vm().strings.set(new_string, Value::Nil);
        vm().pop();
        new_string
    }
}

impl Object for ObjString {
    fn obj_type(&self) -> ObjType {
        self.obj.obj_type()
    }
}

impl Hash for ObjString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.chars.hash(state);
    }
}

impl PartialEq for ObjString {
    fn eq(&self, other: &Self) -> bool {
        self.chars == other.chars
    }
}

pub struct ObjUpvalue {
    obj: Obj,                  // 公共对象头
    pub location: *mut Value,  // 捕获的局部变量
    pub closed: Value,         //
    pub next: *mut ObjUpvalue, // next指针
}

impl ObjUpvalue {
    pub fn new(slot: *mut Value) -> *mut ObjUpvalue {
        let ptr = allocate_obj::<ObjUpvalue>(ObjType::Upvalue);
        unsafe {
            (*ptr).location = slot;
            (*ptr).closed = Value::Nil;
            (*ptr).next = null_mut();
        }

        ptr
    }
}

impl Object for ObjUpvalue {
    fn obj_type(&self) -> ObjType {
        self.obj.obj_type()
    }
}

// 闭包对象
pub struct ObjClosure {
    obj: Obj,                           // 公共对象头
    pub function: *mut ObjFunction,     // 裸函数
    pub upvalues: *mut *mut ObjUpvalue, // 提升值数组
    pub upvalue_count: usize,           // 提升值数量
}

impl ObjClosure {
    pub fn new(function: *mut ObjFunction) -> *mut ObjClosure {
        let upvalue_count = unsafe { (*function).upvalue_count };
        let upvalues = allocate::<*mut ObjUpvalue>(upvalue_count);
        for i in 0..upvalue_count {
            let offset_ptr = unsafe { upvalues.add(i) };
            unsafe { *offset_ptr = null_mut() };
        }

        let ptr = allocate_obj::<ObjClosure>(ObjType::Closure);
        unsafe {
            (*ptr).function = function;
            (*ptr).upvalues = upvalues;
            (*ptr).upvalue_count = upvalue_count;
        }

        ptr
    }
}

impl Object for ObjClosure {
    fn obj_type(&self) -> ObjType {
        self.obj.obj_type()
    }
}

// 类对象
pub struct ObjClass {
    obj: Obj,                 // 公共对象头
    pub name: *mut ObjString, // 类名
    pub methods: *mut Table,  // 类方法
}

impl ObjClass {
    pub fn new(name: *mut ObjString) -> *mut ObjClass {
        let ptr = allocate_obj::<ObjClass>(ObjType::Class);
        unsafe {
            (*ptr).name = name;
            (*ptr).methods = Table::new();
        }

        ptr
    }
}

impl Object for ObjClass {
    fn obj_type(&self) -> ObjType {
        self.obj.obj_type()
    }
}

// 实例对象
pub struct ObjInstance {
    obj: Obj,
    pub class: *mut ObjClass,
    pub fields: *mut Table,
}

impl ObjInstance {
    pub fn new(class: *mut ObjClass) -> *mut ObjInstance {
        let ptr = allocate_obj::<ObjInstance>(ObjType::Instance);

        unsafe {
            (*ptr).class = class;
            (*ptr).fields = Table::new();
        }

        ptr
    }
}

impl Object for ObjInstance {
    fn obj_type(&self) -> ObjType {
        self.obj.obj_type()
    }
}

// 绑定方法对象
pub struct ObjBoundMethod {
    obj: Obj,
    pub receiver: Value,
    pub method: *mut ObjClosure,
}

impl ObjBoundMethod {
    pub fn new(receiver: Value, method: *mut ObjClosure) -> *mut ObjBoundMethod {
        let ptr = allocate_obj::<ObjBoundMethod>(ObjType::BoundMethod);

        unsafe {
            (*ptr).method = method;
            (*ptr).receiver = receiver;
        }
        ptr
    }
}

impl Object for ObjBoundMethod {
    fn obj_type(&self) -> ObjType {
        self.obj.obj_type()
    }
}
