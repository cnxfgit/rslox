use std::collections::HashMap;
use std::ptr::null_mut;
use std::time::Instant;

use crate::chunk::{Chunk, OpCode};
use crate::compiler::{ClassCompiler, Compiler, FunctionType, Parser};
use crate::object::{
    NativeFn, Obj, ObjClosure, ObjFunction, ObjNative, ObjString, ObjType, ObjUpvalue,
};
use crate::scanner::{Scanner, TokenType};
use crate::table::Table;
use crate::value::Value;
use crate::{as_string, number_val, obj_val};

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
    pub current_class: *mut ClassCompiler,
}

macro_rules! read_byte {
    ($vm:expr) => {{
        unsafe {
            $vm.ip = $vm.ip.add(1);
            *$vm.ip
        }
    }};
}

macro_rules! read_constant {
    ($vm:expr) => {{
        let instruction = read_byte!($vm);
        let mut value = 0.0;
        if let Some(chunk) = &mut $vm.chunk {
            value = chunk.constants.values[instruction as usize]
        } else {
            panic!("vm.chunk None.");
        }
        value
    }};
}

macro_rules! binary_op {
    ($vm:expr, $op:tt) => {{
        let b = $vm.pop();
        let a = $vm.pop();
        $vm.push(a $op b);
    }};
}

fn clock_native(arg_count: usize, args: *mut Value) -> Value {
    let now = Instant::now();
    let secs = now.elapsed().as_secs_f64();
    number_val!(secs)
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
            current_class: null_mut(),
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
        loop {
            if let Some(chunk) = &mut self.chunk {
                #[cfg(feature = "debug_trace_execution")]
                {
                    print!("          ");
                    let offset = self.stack_top as usize - self.stack.as_ptr() as usize;
                    let mut slot = self.stack.as_mut_ptr();
                    for _ in 0..offset {
                        print!("[ ");
                        (unsafe { *slot }).print();
                        print!(" ]");
                        slot = unsafe { slot.add(1) };
                    }
                    println!("");
                    chunk.disassemble_instruction(self.ip as usize - chunk.code.as_ptr() as usize);
                }
            }

            let instruction = read_byte!(self);

            let op_code: OpCode = instruction.into();
            match op_code {
                OpCode::Constant => {
                    let constant = read_constant!(self);
                    self.push(constant);
                }
                OpCode::Add => binary_op!(self,+),
                OpCode::Subtract => binary_op!(self,-),
                OpCode::Multiply => binary_op!(self,*),
                OpCode::Divide => binary_op!(self,/),
                OpCode::Negate => {
                    let value = -self.pop();
                    self.push(value);
                }
                OpCode::Return => {
                    self.pop().print();
                    println!("");
                    return InterpretResult::Ok;
                }
                _ => {}
            }
        }

        InterpretResult::Ok
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
