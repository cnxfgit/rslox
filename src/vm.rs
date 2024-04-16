use std::collections::HashMap;
use std::ptr::null_mut;
use std::result;
use std::time::Instant;

use crate::chunk::OpCode;
use crate::compiler::{ClassCompiler, Compiler, FunctionType, Parser};
use crate::object::{
    NativeFn, Obj, ObjBoundMethod, ObjClass, ObjClosure, ObjFunction, ObjInstance, ObjNative,
    ObjString, ObjType, ObjUpvalue,
};
use crate::scanner::Scanner;
use crate::table::Table;
use crate::value::Value;
use crate::{
    as_class, as_closure, as_instance, as_number, as_obj, as_string, is_instance, is_number, is_string, obj_val
};

pub const UINT8_COUNT: usize = u8::MAX as usize + 1;
const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = UINT8_COUNT * FRAMES_MAX;

static mut VM: *mut VM = null_mut();

pub fn init_vm() {
    let mut vm = Box::new(VM::new());
    unsafe { VM = Box::into_raw(vm) };
}

pub fn drop_vm() {
    unsafe {
        Box::from_raw(VM);
    }
}

pub fn vm() -> &'static mut VM {
    unsafe { &mut *VM }
}

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

// 调用帧
#[derive(Clone, Copy)]
struct CallFrame {
    closure: *mut ObjClosure, // 调用的函数闭包
    ip: *mut u8,              // 指向字节码数组的指针 指函数执行到哪了
    slots: *mut Value,        // 指向vm栈中该函数使用的第一个局部变量
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
    frames: [CallFrame; FRAMES_MAX], // 栈帧数组 所有函数调用的执行点
    frame_count: usize,              // 当前调用栈数

    stack: [Value; STACK_MAX],      // 虚拟机栈
    stack_top: *mut Value,          // 栈顶指针 总是指向栈顶
    pub globals: Table,             // 全局变量表
    pub strings: Table,             // 全局字符串表
    init_string: *mut ObjString,    // 构造器名称
    open_upvalues: *mut ObjUpvalue, // 全局提升值

    bytes_allocated: usize, // 已经分配的内存
    next_gc: usize,         // 出发下一次gc的阈值

    objects: *mut Obj,         // 对象根链表
    gray_count: usize,         // 灰色对象数量
    gray_capacity: usize,      // 灰色对象容量
    gray_stack: *mut *mut Obj, // 灰色对象栈

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
    }
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

fn clock_native(arg_count: usize, args: *mut Value) -> Value {
    let now = Instant::now();
    let secs = now.elapsed().as_secs_f64();
    Value::Number(secs)
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
        let mut vm = VM {
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
            init_string: ObjString::take_string("init".into()),
            open_upvalues: null_mut(),

            bytes_allocated: 0,
            next_gc: 1024 * 1024,

            objects: null_mut(),
            gray_count: 0,
            gray_capacity: 0,
            gray_stack: null_mut(),

            current_compiler: null_mut(),
            parser: Parser::new(),
            scanner: None,
            class_compiler: null_mut(),
        };
        vm.stack_top = vm.stack.as_mut_ptr();

        vm.define_native("clock", clock_native);

        vm
    }

    fn define_native(&self, name: &str, function: NativeFn) {
        self.push(obj_val!(ObjString::take_string(name.into())));
        self.push(obj_val!(ObjNative::new(function)));
        self.globals.set(as_string!(self.stack[0]), self.stack[1]);
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

        let mut i = self.frame_count - 1;
        while i >= 0 {
            let frame = &self.frames[i];
            let function = (unsafe { *frame.closure }).function;
            let instruction =
                frame.ip as usize - (unsafe { *function }).chunk.code.as_mut_ptr() as usize - 1;
            eprint!(
                "[line {}] in ",
                unsafe { *function }.chunk.lines[instruction]
            );
            if (unsafe { *function }).name.is_null() {
                eprintln!("script");
            } else {
                eprintln!("{}()", unsafe { *(*function).name }.chars);
            }
            i -= 1;
        }
        self.reset_stack();
    }

    fn call(&self, closure: *mut ObjClosure, arg_count: usize) -> bool {
        let arity = (unsafe { *(*closure).function }).arity;
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
        let frame = &self.frames[self.frame_count];
        self.frame_count += 1;
        frame.closure = closure;
        frame.ip = (unsafe { *(*closure).function }).chunk.code.as_mut_ptr();
        frame.slots = unsafe { self.stack_top.sub(arg_count + 1) };

        true
    }

    fn run(&mut self) -> InterpretResult {
        // 拿到vm中的栈帧
        let frame = &mut self.frames[self.frame_count - 1] as *mut CallFrame;

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
                    let chunk = &(*(*(*frame).closure).function).chunk;
                    chunk.disassemble_instruction(
                        (*frame).ip as usize - chunk.code.as_mut_ptr() as usize,
                    );
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
                OpCode::Pop => self.pop(),
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
                    self.globals.set(name, self.peek(0));
                    self.pop();
                }
                OpCode::SetGlobal => {
                    let name = read_string!(frame);
                    if self.globals.set(name, self.peek(0)) {
                        self.globals.remove(name);
                        self.runtime_error(format!(
                            "Undefined variable '{}'.",
                            &(unsafe { *name }).chars
                        ));
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
                        self.pop();
                        self.push(value.clone());
                    } else if !self.bind_method((unsafe { *instance }).class, name) {
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
                        self.runtime_error( "Operands must be two numbers or two strings.".into());
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::Subtract => binary_op!(self, f64, -),
                OpCode::Multiply => binary_op!(self, f64, *),
                OpCode::Divide => binary_op!(self, f64, /),
                _ => {break;}
            }
        }

        InterpretResult::Ok
    }

    // 连接字符串
    fn concatenate(&self) {
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

    fn bind_method(&self, class: *mut ObjClass, name: *mut ObjString) -> bool {
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

    fn peek(&self, distance: i32) -> Value {
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
