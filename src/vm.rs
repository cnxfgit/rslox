use crate::chunk::{Chunk, OpCode};
use crate::value::{print_value, Value};

const STACK_MAX: usize = 256;

enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

struct VM {
    chunk: Option<Chunk>,
    ip: *mut u8,
    stack: [Value; STACK_MAX],
    stack_top: *mut Value,
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
    fn new() -> VM {
        let mut vm = VM {
            chunk: None,
            ip: std::ptr::null_mut(),
            stack: [0.0; STACK_MAX],
            stack_top: std::ptr::null_mut(),
        };
        vm.stack_top = vm.stack.as_mut_ptr();
        vm
    }

    fn interpret(&mut self, chunk: Chunk) -> InterpretResult {
        self.chunk = Some(chunk);
        if let Some(chunk) = &mut self.chunk {
            self.ip = chunk.code.as_mut_ptr();
        } else {
            panic!("vm.chunk None.");
        }
        return self.run();
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
                        print_value(unsafe { *slot });
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
                    print_value(self.pop());
                    println!("");
                    return InterpretResult::Ok;
                }
                _ => {}
            }
        }

        InterpretResult::Ok
    }

    fn push(&mut self, value: Value) {
        unsafe {
            *self.stack_top = value;
            self.stack_top = self.stack_top.add(1);
        }
    }

    fn pop(&mut self) -> Value {
        unsafe {
            self.stack_top = self.stack_top.sub(1);
            *self.stack_top
        }
    }
}
