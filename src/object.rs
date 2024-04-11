use std::{
    hash::Hash,
    ptr::{self, null_mut},
};

use crate::{
    as_obj,
    chunk::Chunk,
    is_obj,
    memory::{allocate, allocate_obj, dealloc},
    table::Table,
    value::Value,
    vm::vm,
};

#[derive(PartialEq, Eq)]
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

impl Obj {
    fn is_obj_type(value: Value, type_: ObjType) -> bool {
        is_obj!(value) && (unsafe { *as_obj!(value) }).type_ == type_
    }
}

impl Object for Obj {
    fn obj_type(&self) -> ObjType {
        self.type_
    }
}

struct ObjFunction {
    obj: Obj,             // 公共对象头
    arity: usize,         // 参数数
    upvalue_count: usize, // 提升值数
    chunk: Chunk,         // 函数的字节码块
    name: *mut ObjString, // 函数名
}

impl ObjFunction {
    fn new() -> *mut ObjFunction {
        let ptr = allocate_obj::<ObjFunction>(ObjType::Function);
        let chunk = Chunk::new();
        unsafe {
            (*ptr).arity = 0;
            (*ptr).upvalue_count = 0;
            (*ptr).name = null_mut();
            let chunk_ptr: *mut Chunk = unsafe { &mut (*ptr).chunk };
            std::ptr::write(chunk_ptr, chunk);
        }

        ptr
    }
}

impl Object for ObjFunction {
    fn obj_type(&self) -> ObjType {
        self.obj_type()
    }
}

type NativeFn = fn(usize, *mut Value) -> Value;

struct ObjNative {
    obj: Obj,           // 公共对象头
    function: NativeFn, // 原生函数指针
}

impl ObjNative {
    fn new(function: NativeFn) -> *mut ObjNative {
        let ptr = allocate_obj::<ObjNative>(ObjType::Native);
        unsafe {
            (*ptr).function = function;
        }

        ptr
    }
}

impl Object for ObjNative {
    fn obj_type(&self) -> ObjType {
        self.obj_type()
    }
}

pub struct ObjString {
    obj: Obj,      // 公共对象头
    chars: String, // 字符串
}

impl ObjString {
    fn new(string: String) -> *mut ObjString {
        let ptr = allocate_obj::<ObjString>(ObjType::String);

        unsafe {
            let chars_ptr = &mut (*ptr).chars as *mut String;
            ptr::write(chars_ptr, string);
        }

        ptr
    }

    fn take_string(string: String) -> *mut ObjString {
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
        self.obj_type()
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

struct ObjUpvalue {
    obj: Obj,              // 公共对象头
    location: *mut Value,  // 捕获的局部变量
    closed: Value,         //
    next: *mut ObjUpvalue, // next指针
}

impl ObjUpvalue {
    fn new() -> *mut ObjUpvalue {
        let ptr = allocate_obj::<ObjUpvalue>(ObjType::Upvalue);
        unsafe {
            (*ptr).location = null_mut();
            (*ptr).closed = Value::Nil;
            (*ptr).next = null_mut();
        }

        ptr
    }
}

impl Object for ObjUpvalue {
    fn obj_type(&self) -> ObjType {
        self.obj_type()
    }
}

// 闭包对象
struct ObjClosure {
    obj: Obj,                       // 公共对象头
    function: *mut ObjFunction,     // 裸函数
    upvalues: *mut *mut ObjUpvalue, // 提升值数组
    upvalue_count: usize,           // 提升值数量
}

impl ObjClosure {
    fn new(function: *mut ObjFunction) -> *mut ObjClosure {
        let upvalue_count = (unsafe { *function }).upvalue_count;
        let upvalues = allocate::<*mut ObjUpvalue>(upvalue_count);
        for i in 0..upvalue_count {
            let mut offset_ptr = unsafe { upvalues.add(i) };
            offset_ptr = null_mut();
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
        self.obj_type()
    }
}

// 类对象
struct ObjClass {
    obj: Obj,             // 公共对象头
    name: *mut ObjString, // 类名
    methods: *mut Table,  // 类方法
}

impl ObjClass {
    fn new(name: *mut ObjString) -> *mut ObjClass {
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
        self.obj_type()
    }
}

// 实例对象
struct ObjInstance {
    obj: Obj,
    class: *mut ObjClass,
    fields: *mut Table,
}

impl ObjInstance {
    fn new(class: *mut ObjClass) -> *mut ObjInstance {
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
        self.obj_type()
    }
}

// 绑定方法对象
struct ObjBoundMethod {
    obj: Obj,
    receiver: Value,
    method: *mut ObjClosure,
}

impl ObjBoundMethod {
    fn new(receiver: Value, method: *mut ObjClosure) -> *mut ObjBoundMethod {
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
        self.obj_type()
    }
}
