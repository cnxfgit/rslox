use std::collections::HashMap;
use std::ptr::null_mut;

use crate::chunk::{Chunk, OpCode};
use crate::compiler::Compiler;
use crate::scanner::{Scanner, TokenType};
use crate::table::Table;
use crate::value::Value;

const STACK_MAX: usize = 256;

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

pub struct VM {
    chunk: Option<Chunk>,
    ip: *mut u8,
    stack: [Value; STACK_MAX],
    stack_top: *mut Value,

    pub strings: Table,
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

impl VM {
    pub fn new() -> VM {
        let mut vm = VM {
            chunk: None,
            ip: std::ptr::null_mut(),
            stack: [Value::Nil; STACK_MAX],
            stack_top: std::ptr::null_mut(),

            strings: Table{map: HashMap::new()},
        };
        vm.stack_top = vm.stack.as_mut_ptr();
        vm
    }

    pub fn interpret(&mut self, source: String) -> InterpretResult {
        let chunk = Chunk::new();

        if !self.compile(source) {
            return InterpretResult::CompileError;
        }

        self.chunk = Some(chunk);
        if let Some(chunk) = &mut self.chunk {
            self.ip = chunk.code.as_mut_ptr();
        } else {
            panic!("vm.chunk None.");
        }
        let result = self.run();
        return result;
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

    fn compile(&mut self, source: String) -> bool {
        let scanner = Scanner::new(source);
        let mut compiler = Compiler::new(scanner);

        compiler.advance();
        compiler.consume(TokenType::Eof, "Expect end of expression.");

        !compiler.parser.had_error
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
