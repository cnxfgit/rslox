use std::collections::HashMap;
use std::ptr::null_mut;
use std::time::Instant;

use crate::chunk::OpCode;
use crate::compiler::{ClassCompiler, Compiler, FunctionType, Parser};
use crate::object::{
    NativeFn, Obj, ObjBoundMethod, ObjClass, ObjClosure, ObjFunction, ObjInstance, ObjNative,
    ObjString, ObjType, ObjUpvalue,
};
use crate::scanner::Scanner;
use crate::table::Table;
use crate::value::{as_obj, Value};
use crate::{
    as_bound_method, as_class, as_closure, as_function, as_instance, as_native, as_number,
    as_string, is_class, is_instance, is_number, is_obj, is_string, obj_val,
};

pub const UINT8_COUNT: usize = u8::MAX as usize + 1;
const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = UINT8_COUNT * FRAMES_MAX;

static mut VM: *mut VM = null_mut();

pub fn init_vm() {
    let box_vm = Box::new(VM::new());
    unsafe { VM = Box::into_raw(box_vm) };
    vm().stack_top = vm().stack.as_mut_ptr();
    vm().init_string = ObjString::take_string("init".into());
    vm().define_native("clock", clock_native);
}

pub fn drop_vm() {
    unsafe {
        let _ = Box::from_raw(VM);
    }
}

pub fn vm() -> &'static mut VM {
    unsafe { VM.as_mut().unwrap()  as &'static mut VM}
}

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

// 调用帧
#[derive(Clone, Copy)]
pub struct CallFrame {
    pub closure: *mut ObjClosure, // 调用的函数闭包
    ip: *mut u8,                  // 指向字节码数组的指针 指函数执行到哪了
    slots: *mut Value,            // 指向vm栈中该函数使用的第一个局部变量
}

impl CallFrame {
    fn new() -> CallFrame {
        CallFrame {
            closure: null_mut(),
            ip: null_mut(),
            slots: null_mut(),
        }
    }
}

pub struct VM {
    pub frames: [CallFrame; FRAMES_MAX], // 栈帧数组 所有函数调用的执行点
    pub frame_count: usize,              // 当前调用栈数

    pub stack: [Value; STACK_MAX],      // 虚拟机栈
    pub stack_top: *mut Value,          // 栈顶指针 总是指向栈顶
    pub globals: Table,                 // 全局变量表
    pub strings: Table,                 // 全局字符串表
    pub init_string: *mut ObjString,    // 构造器名称
    pub open_upvalues: *mut ObjUpvalue, // 全局提升值

    pub bytes_allocated: usize, // 已经分配的内存
    pub next_gc: usize,         // 出发下一次gc的阈值

    pub objects: *mut Obj,         // 对象根链表
    pub gray_stack: Vec<*mut Obj>, // 灰色对象栈

    pub current_compiler: *mut Compiler,
    pub parser: Parser,
    pub scanner: Option<Scanner>,
    pub class_compiler: *mut ClassCompiler,
}

macro_rules! read_byte {
    ($frame:expr) => {
        unsafe {
            let result = *(*$frame).ip;
            (*$frame).ip = (*$frame).ip.add(1);
            result
        }
    };
}

macro_rules! read_constant {
    ($frame:expr) => {
        unsafe {
            (*(*(*$frame).closure).function).chunk.constants.values[read_byte!($frame) as usize]
        }
    };
}

macro_rules! read_short {
    ($frame:expr) => {
        unsafe {
            (*$frame).ip = (*$frame).ip.add(2);
            (((*((*$frame).ip.sub(2))) as u16) << 8) | *(*$frame).ip.sub(1) as u16
        }
    };
}

macro_rules! read_string {
    ($frame:expr) => {
        as_string!(read_constant!($frame))
    };
}

macro_rules! create_value {
    (f64) => {
        Value::Number
    };
    (bool) => {
        Value::Boolean
    };
}

macro_rules! binary_op {
    ($vm:expr, $value_type:tt, $op:tt) => {{
        match ($vm.peek(0), $vm.peek(1)) {
            (Value::Number(_), Value::Number(_)) => {
                let b = $vm.pop();
                let a = $vm.pop();
                if let (Value::Number(n1), Value::Number(n2)) = (a, b) {
                    let value = n1 $op n2;
                    $vm.push(create_value!($value_type)(value));
                }
            }
            _ => {
                $vm.runtime_error("Operands must be numbers.".into());
                return InterpretResult::RuntimeError;
            }
        }
    }};
}

fn clock_native(_arg_count: usize, _args: *mut Value) -> Value {
    let now = Instant::now();
    let secs = now.elapsed().as_secs_f64();
    Value::Number(secs)
}

fn is_falsey(value: Value) -> bool {
    match value {
        Value::Nil => true,
        Value::Boolean(b) => !b,
        _ => true,
    }
}

fn values_equal(a: Value, b: Value) -> bool {
    match (a, b) {
        (Value::Boolean(bool1), Value::Boolean(bool2)) => bool1 == bool2,
        (Value::Nil, Value::Nil) => true,
        (Value::Number(n1), Value::Number(n2)) => n1 == n2,
        (Value::Object(obj1), Value::Object(obj2)) => obj1 == obj2,
        _ => false, // Unreachable.
    }
}

impl VM {
    pub fn new() -> VM {
        VM {
            frames: [CallFrame::new(); FRAMES_MAX],
            frame_count: 0,

            stack: [Value::Nil; STACK_MAX],
            stack_top: std::ptr::null_mut(),
            globals: Table {
                map: HashMap::new(),
            },
            strings: Table {
                map: HashMap::new(),
            },
            init_string: null_mut(),
            open_upvalues: null_mut(),

            bytes_allocated: 0,
            next_gc: 1024 * 1024,

            objects: null_mut(),
            gray_stack: vec![],

            current_compiler: null_mut(),
            parser: Parser::new(),
            scanner: None,
            class_compiler: null_mut(),
        }
    }

    fn define_native(&mut self, name: &str, function: NativeFn) {
        self.push(obj_val!(ObjString::take_string(name.into())));
        self.push(obj_val!(ObjNative::new(function)));
        self.globals
            .set(as_string!(self.stack[0]), self.stack[1]);
        self.pop();
        self.pop();
    }

    pub fn interpret(&mut self, source: String) -> InterpretResult {
        let function = self.compile(source);
        if function.is_null() {
            return InterpretResult::CompileError;
        }

        self.push(obj_val!(function));
        let closure = ObjClosure::new(function);
        self.pop();
        self.push(obj_val!(closure));
        self.call(closure, 0);

        return self.run();
    }

    fn reset_stack(&mut self) {
        self.stack_top = &mut self.stack as *mut Value;
        self.frame_count = 0;
        self.open_upvalues = null_mut();
    }

    fn runtime_error(&mut self, message: String) {
        eprintln!("{}", message);

        let mut i = self.frame_count as i32 - 1;
        while i >= 0 {
            let frame = &self.frames[i as usize];
            let function = unsafe { (*(*frame).closure).function };
            let instruction =
                frame.ip as usize - unsafe { (*function).chunk.code.as_mut_ptr() } as usize - 1;
            eprint!("[line {}] in ", unsafe {
                (*function).chunk.lines[instruction]
            });
            if unsafe { (*function).name.is_null() } {
                eprintln!("script");
            } else {
                eprintln!("{}()", unsafe { &(*(*function).name).chars });
            }
            i -= 1;
        }
        self.reset_stack();
    }

    fn call(&mut self, closure: *mut ObjClosure, arg_count: usize) -> bool {
        let arity = unsafe { (*(*closure).function).arity };
        if arg_count != arity {
            self.runtime_error(format!(
                "Expected {} arguments but got {}.",
                arity, arg_count
            ));
            return false;
        }
        // 调用栈过长
        if self.frame_count == FRAMES_MAX {
            self.runtime_error("Stack overflow.".into());
            return false;
        }
        // 记录新函数栈帧
        let frame = &mut self.frames[self.frame_count];
        self.frame_count += 1;
        let frame = frame as *mut CallFrame;
        unsafe {
            (*frame).closure = closure;
            (*frame).ip = (*(*closure).function).chunk.code.as_mut_ptr();
            (*frame).slots = self.stack_top.sub(arg_count + 1);
        }

        true
    }

    fn run(&mut self) -> InterpretResult {
        // 拿到vm中的栈帧
        let mut frame = &mut self.frames[self.frame_count - 1] as *mut CallFrame;

        loop {
            #[cfg(feature = "debug_trace_execution")]
            {
                print!("          ");
                let mut slot = self.stack.as_mut_ptr();
                while slot < self.stack_top {
                    print!("[ ");
                    (unsafe { *slot }).print();
                    print!(" ]");
                    slot = unsafe { slot.add(1) };
                }
                println!("");
                unsafe {
                    let chunk = &mut (*(*(*frame).closure).function).chunk;
                    let tmp = chunk.code.as_mut_ptr() as usize;
                    chunk.disassemble_instruction((*frame).ip as usize - tmp);
                }
            }

            let instruction: OpCode = read_byte!(frame).into();

            let op_code: OpCode = instruction.into();
            match op_code {
                OpCode::Constant => {
                    let constant = read_constant!(frame);
                    self.push(constant);
                }
                OpCode::Nil => self.push(Value::Nil),
                OpCode::True => self.push(Value::Boolean(true)),
                OpCode::False => self.push(Value::Boolean(false)),
                OpCode::Pop => {
                    self.pop();
                }
                OpCode::GetLocal => {
                    let slot = read_byte!(frame);
                    unsafe {
                        self.push(*(*frame).slots.add(slot as usize));
                    }
                }
                OpCode::SetLocal => {
                    let slot = read_byte!(frame);
                    unsafe {
                        std::ptr::write((*frame).slots.add(slot as usize), self.peek(0));
                    }
                }
                OpCode::GetGlobal => {
                    let name = read_string!(frame);

                    match self.globals.get(name) {
                        Some(value) => self.push(value.clone()),
                        None => {
                            self.runtime_error(format!("Undefined variable '{}'.", unsafe {
                                &(*name).chars
                            }));
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::DefineGlobal => {
                    let name = read_string!(frame);
                    let p = self.peek(0);
                    self.globals.set(name, p);
                    self.pop();
                }
                OpCode::SetGlobal => {
                    let name = read_string!(frame);
                    let p = self.peek(0);
                    if self.globals.set(name, p) {
                        self.globals.remove(name);
                        self.runtime_error(format!("Undefined variable '{}'.", unsafe {
                            &(*name).chars
                        }));
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::GetUpvalue => {
                    let slot = read_byte!(frame);
                    unsafe {
                        self.push(*(**(*(*frame).closure).upvalues.add(slot as usize)).location);
                    }
                }
                OpCode::SetUpvalue => {
                    let slot = read_byte!(frame);
                    unsafe {
                        std::ptr::write(
                            (**(*(*frame).closure).upvalues.add(slot as usize)).location,
                            self.peek(0),
                        );
                    }
                }
                OpCode::GetProperty => {
                    if !is_instance!(self.peek(0)) {
                        self.runtime_error("Only instances have properties.".into());
                        return InterpretResult::RuntimeError;
                    }

                    let instance = as_instance!(self.peek(0));
                    let name = read_string!(frame);

                    if let Some(value) = self.globals.get(name) {
                        let v = value.clone();
                        self.pop();
                        self.push(v);
                    } else if !self.bind_method(unsafe { (*instance).class }, name) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::SetProperty => {
                    if !is_instance!(self.peek(1)) {
                        self.runtime_error("Only instances have fields.".into());
                        return InterpretResult::RuntimeError;
                    }

                    let instance = as_instance!(self.peek(1));
                    unsafe {
                        (*(*instance).fields).set(read_string!(frame), self.peek(0));
                    }
                    let value = self.pop();
                    self.pop();
                    self.push(value);
                }
                OpCode::GetSuper => {
                    let name = read_string!(frame);
                    let superclass = as_class!(self.pop());

                    if !self.bind_method(superclass, name) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::Equal => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Value::Boolean(values_equal(a, b)));
                }
                OpCode::Greater => binary_op!(self, bool, >),
                OpCode::Less => binary_op!(self, bool, <),
                OpCode::Add => {
                    if is_string!(self.peek(0)) && is_string!(self.peek(1)) {
                        self.concatenate();
                    } else if (is_number!(self.peek(0)) && is_number!(self.peek(1))) {
                        let b = as_number!(self.pop());
                        let a = as_number!(self.pop());
                        self.push(Value::Number(a + b));
                    } else {
                        self.runtime_error("Operands must be two numbers or two strings.".into());
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::Subtract => binary_op!(self, f64, -),
                OpCode::Multiply => binary_op!(self, f64, *),
                OpCode::Divide => binary_op!(self, f64, /),
                OpCode::Not => {
                    let top = self.pop();
                    self.push(Value::Boolean(is_falsey(top)));
                }
                OpCode::Negate => {
                    if !is_number!(self.peek(0)) {
                        self.runtime_error("Operand must be a number.".into());
                        return InterpretResult::RuntimeError;
                    }
                    let top = self.pop();
                    self.push(Value::Number(-as_number!(top)));
                }
                OpCode::Print => {
                    self.pop().print();
                    println!("");
                }
                OpCode::Jump => {
                    let offset = read_short!(frame);
                    unsafe {
                        (*frame).ip = (*frame).ip.add(offset as usize);
                    }
                }
                OpCode::JumpIfFalse => {
                    let offset = read_short!(frame);
                    if is_falsey(self.peek(0)) {
                        unsafe {
                            (*frame).ip = (*frame).ip.add(offset as usize);
                        }
                    }
                }
                OpCode::Loop => {
                    let offset = read_short!(frame);
                    unsafe {
                        (*frame).ip = (*frame).ip.sub(offset as usize);
                    }
                }
                OpCode::Call => {
                    let arg_count = read_byte!(frame);
                    let p = self.peek(arg_count as i32);
                    if !self.call_value(p, arg_count) {
                        return InterpretResult::RuntimeError;
                    }

                    // 调用成功后将栈帧还回去
                    frame = &mut self.frames[self.frame_count - 1];
                }
                OpCode::Invoke => {
                    let method = read_string!(frame);
                    let arg_count = read_byte!(frame);
                    if !self.invoke(method, arg_count) {
                        return InterpretResult::RuntimeError;
                    }
                    frame = &mut self.frames[self.frame_count - 1];
                }
                OpCode::SuperInvoke => {
                    let method = read_string!(frame);
                    let arg_count = read_byte!(frame);
                    let superclass = as_class!(self.pop());
                    if !self.invoke_from_class(superclass, method, arg_count) {
                        return InterpretResult::RuntimeError;
                    }
                    frame = &mut self.frames[self.frame_count - 1];
                }
                OpCode::Closure => {
                    let function = as_function!(read_constant!(frame));
                    let closure = ObjClosure::new(function);
                    self.push(Value::Object(closure as *mut Obj));

                    let mut i = 0;
                    while i < unsafe { (*closure).upvalue_count } {
                        let is_local = read_byte!(frame);
                        let index = read_byte!(frame);
                        unsafe {
                            if is_local != 0 {
                                let ptr = (*closure).upvalues.add(i);
                                *ptr = self.capture_upvalue((*frame).slots.add(index as usize));
                            } else {
                                let ptr = (*closure).upvalues.add(i);
                                *ptr = *(*(*frame).closure).upvalues.add(index as usize);
                            }
                        }
                        i += 1;
                    }
                }
                OpCode::CloseUpvalue => {
                    self.close_upvalues(unsafe { self.stack_top.sub(1) });
                    self.pop();
                }
                OpCode::Return => {
                    let result = self.pop();
                    self.close_upvalues((unsafe { *frame }).slots);
                    self.frame_count -= 1;
                    if self.frame_count == 0 {
                        self.pop();
                        return InterpretResult::Ok;
                    }

                    self.stack_top = (unsafe { *frame }).slots;
                    self.push(result);
                    frame = &mut self.frames[self.frame_count - 1];
                }
                OpCode::Class => {
                    self.push(Value::Object(ObjClass::new(read_string!(frame)) as *mut Obj))
                }
                OpCode::Inherit => {
                    let superclass = self.peek(1);
                    if !is_class!(superclass) {
                        self.runtime_error("Superclass must be a class.".into());
                        return InterpretResult::RuntimeError;
                    }

                    let subclass = as_class!(self.peek(0));
                    unsafe {
                        (*(*subclass).methods).add_all(&*(*as_class!(superclass)).methods);
                    }
                    self.pop(); // Subclass.
                }
                OpCode::Method => self.define_method(read_string!(frame)),
            }
        }

        // InterpretResult::Ok
    }

    fn define_method(&mut self, name: *mut ObjString) {
        let method = self.peek(0);
        let class = as_class!(self.peek(1));
        unsafe { (*(*class).methods).set(name, method) };
        self.pop();
    }

    fn close_upvalues(&mut self, last: *mut Value) {
        unsafe {
            while !self.open_upvalues.is_null() && (*self.open_upvalues).location >= last {
                let upvalue = self.open_upvalues;
                (*upvalue).closed = *(*upvalue).location;
                (*upvalue).location = &mut (*upvalue).closed;
                self.open_upvalues = (*upvalue).next;
            }
        }
    }

    // 捕获提升值
    fn capture_upvalue(&mut self, local: *mut Value) -> *mut ObjUpvalue {
        let mut prev_upvalue: *mut ObjUpvalue = null_mut();
        let mut upvalue = self.open_upvalues;
        while !upvalue.is_null() && unsafe { (*upvalue).location } > local {
            prev_upvalue = upvalue;
            upvalue = unsafe { (*upvalue).next };
        }

        if !upvalue.is_null() && unsafe { (*upvalue).location } == local {
            return upvalue;
        }

        let created_upvalue = ObjUpvalue::new(local);

        unsafe { (*created_upvalue).next = upvalue };

        if prev_upvalue.is_null() {
            self.open_upvalues = created_upvalue;
        } else {
            unsafe { (*prev_upvalue).next = created_upvalue };
        }

        created_upvalue
    }

    fn invoke(&mut self, name: *mut ObjString, arg_count: u8) -> bool {
        let receiver = self.peek(arg_count as i32);

        if !is_instance!(receiver) {
            self.runtime_error("Only instances have methods.".into());
            return false;
        }

        let instance = as_instance!(receiver);
        if let Some(value) = unsafe { (*(*instance).fields).get(name) } {
            unsafe {
                std::ptr::write(
                    self.stack_top.offset(-(arg_count as isize) - 1),
                    value.clone(),
                );
            }
            return self.call_value(value.clone(), arg_count);
        }
        return self.invoke_from_class(unsafe { (*instance).class }, name, arg_count);
    }

    fn invoke_from_class(
        &mut self,
        class: *mut ObjClass,
        name: *mut ObjString,
        arg_count: u8,
    ) -> bool {
        if let Some(method) = unsafe { (*(*class).methods).get(name) } {
            self.call(as_closure!(method.clone()), arg_count as usize)
        } else {
            self.runtime_error(format!("Undefined property '{}'.", unsafe {
                &(*name).chars
            }));
            false
        }
    }

    // 调用 值类型  仅接受 函数 类 方法
    fn call_value(&mut self, callee: Value, arg_count: u8) -> bool {
        if is_obj!(callee) {
            match unsafe { (*as_obj(callee)).type_ } {
                ObjType::BoundMethod => {
                    let bound = as_bound_method!(callee);
                    unsafe {
                        let ptr = self.stack_top.offset(-(arg_count as isize) - 1);
                        std::ptr::write(ptr, (*bound).receiver);
                        return self.call((*bound).method, arg_count as usize);
                    }
                }
                ObjType::Class => {
                    let class = as_class!(callee);
                    unsafe {
                        let ptr = self.stack_top.offset(-(arg_count as isize) - 1);
                        std::ptr::write(ptr, Value::Object(ObjInstance::new(class) as *mut Obj));
                    }

                    match unsafe { (*(*class).methods).get(self.init_string) } {
                        Some(initializer) => {
                            return self.call(as_closure!(initializer.clone()), arg_count as usize);
                        }
                        None => {
                            if arg_count != 0 {
                                self.runtime_error(format!(
                                    "Expected 0 arguments but got {}.",
                                    arg_count
                                ));
                                return false;
                            }
                            return true;
                        }
                    }
                }
                ObjType::Closure => return self.call(as_closure!(callee), arg_count as usize),
                ObjType::Native => {
                    let native = unsafe { as_native!(callee).as_mut().unwrap() }.function;
                    let result = native(arg_count as usize, unsafe {
                        self.stack_top.sub(arg_count as usize)
                    });
                    self.stack_top = unsafe { self.stack_top.sub((arg_count + 1) as usize) };
                    self.push(result);
                    return true;
                }
                _ => {} // Non-callable object type.
            }
        }
        self.runtime_error("Can only call functions and classes.".into());
        false
    }

    // 连接字符串
    fn concatenate(&mut self) {
        let b = as_string!(self.peek(0));
        let a = as_string!(self.peek(1));

        unsafe {
            let result = String::new();
            let result = ObjString::take_string(result + &(*a).chars + &(*b).chars);

            self.pop();
            self.pop();

            self.push(Value::Object(result as *mut Obj));
        }
    }

    fn bind_method(&mut self, class: *mut ObjClass, name: *mut ObjString) -> bool {
        unsafe {
            if let Some(method) = (*(*class).methods).get(name) {
                let bound = ObjBoundMethod::new(self.peek(0), as_closure!(method.clone()));
                self.pop();
                self.push(obj_val!(bound));
                true
            } else {
                self.runtime_error(format!("Undefined property '{}'.", &(*name).chars));
                false
            }
        }
    }

    fn peek(&mut self, distance: i32) -> Value {
        return unsafe { *self.stack_top.offset((-1 - distance) as isize) }.clone();
    }

    fn compile(&mut self, source: String) -> *mut ObjFunction {
        let scanner = Scanner::new(source);
        self.scanner = Some(scanner);
        let mut compiler = Compiler::new(FunctionType::Script);

        self.parser.had_error = false;
        self.parser.panic_mode = false;

        compiler.compile()
    }

    pub fn push(&mut self, value: Value) {
        unsafe {
            *self.stack_top = value;
            self.stack_top = self.stack_top.add(1);
        }
    }

    pub fn pop(&mut self) -> Value {
        unsafe {
            self.stack_top = self.stack_top.sub(1);
            *self.stack_top
        }
    }
}
