use crate::{
    as_function,
    chunk::{Chunk, OpCode},
    object::ObjFunction,
    value::as_obj,
};

impl Chunk {
    pub fn disassemble_chunk(&self, name: &str) {
        println!("== {} ==", name); // 打印字节码块名

        // 遍历字节码块中的字节码
        let mut offset = 0;
        loop {
            if offset >= self.count() {
                break;
            }
            offset = self.disassemble_instruction(offset);
        }
    }

    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        let mut offset = offset;

        print!("{:04} ", offset);
        print!("{:4} ", self.lines[offset]);

        let instruction = self.code[offset];
        let instruction: OpCode = instruction.into();
        match instruction {
            OpCode::Constant => self.constant_instruction("OP_CONSTANT", offset),
            OpCode::Nil => self.simple_instruction("OP_NIL", offset),
            OpCode::True => self.simple_instruction("OP_TRUE", offset),
            OpCode::False => self.simple_instruction("OP_FALSE", offset),
            OpCode::Pop => self.simple_instruction("OP_POP", offset),
            OpCode::GetLocal => self.byte_instruction("OP_GET_LOCAL", offset),
            OpCode::SetLocal => self.byte_instruction("OP_SET_LOCAL", offset),
            OpCode::GetGlobal => self.constant_instruction("OP_GET_GLOBAL", offset),
            OpCode::DefineGlobal => self.constant_instruction("OP_DEFINE_GLOBAL", offset),
            OpCode::SetGlobal => self.constant_instruction("OP_SET_GLOBAL", offset),
            OpCode::GetUpvalue => self.byte_instruction("OP_GET_UPVALUE", offset),
            OpCode::SetUpvalue => self.byte_instruction("OP_SET_UPVALUE", offset),
            OpCode::GetProperty => self.constant_instruction("OP_GET_PROPERTY", offset),
            OpCode::SetProperty => self.constant_instruction("OP_SET_PROPERTY", offset),
            OpCode::GetSuper => self.constant_instruction("OP_GET_SUPER", offset),
            OpCode::Equal => self.simple_instruction("OP_EQUAL", offset),
            OpCode::Greater => self.simple_instruction("OP_GREATER", offset),
            OpCode::Less => self.simple_instruction("OP_LESS", offset),
            OpCode::Add => self.simple_instruction("OP_ADD", offset),
            OpCode::Subtract => self.simple_instruction("OP_SUBTRACT", offset),
            OpCode::Multiply => self.simple_instruction("OP_MULTIPLY", offset),
            OpCode::Divide => self.simple_instruction("OP_DIVIDE", offset),
            OpCode::Not => self.simple_instruction("OP_NOT", offset),
            OpCode::Negate => self.simple_instruction("OP_NEGATE", offset),
            OpCode::Print => self.simple_instruction("OP_PRINT", offset),
            OpCode::Jump => self.jump_instruction("OP_JUMP", 1, offset),
            OpCode::JumpIfFalse => self.jump_instruction("OP_JUMP_IF_FALSE", 1, offset),
            OpCode::Loop => self.jump_instruction("OP_LOOP", -1, offset),
            OpCode::Call => self.byte_instruction("OP_CALL", offset),
            OpCode::Invoke => self.invoke_instruction("OP_INVOKE", offset),
            OpCode::SuperInvoke => self.invoke_instruction("OP_SUPER_INVOKE", offset),
            OpCode::Closure => {
                offset += 1;
                let constant = self.code[offset];
                offset += 1;
                print!("{:<16} {:>4} ", "OP_CLOSURE", constant);
                self.constants.values[constant as usize].print();
                println!("");
                let function = as_function!(self.constants.values[constant as usize]);
                for _ in unsafe { 0..(*function).upvalue_count } {
                    let is_local = self.code[offset];
                    offset += 1;
                    let index = self.code[offset];
                    offset += 1;
                    println!(
                        "{:04}      |                     {} {}",
                        offset - 2,
                        if is_local != 0 { "local" } else { "upvalue" },
                        index
                    );
                }
                offset
            }
            OpCode::CloseUpvalue => self.simple_instruction("OP_CLOSE_UPVALUE", offset),
            OpCode::Return => self.simple_instruction("OP_RETURN", offset),
            OpCode::Class => self.constant_instruction("OP_CLASS", offset),
            OpCode::Inherit => self.simple_instruction("OP_INHERIT", offset),
            OpCode::Method => self.constant_instruction("OP_METHOD", offset),
        }
    }

    fn simple_instruction(&self, name: &str, offset: usize) -> usize {
        println!("{} ", name);
        return offset + 1;
    }

    // 字节指令 打印出slot的偏移量
    fn byte_instruction(&self, name: &str, offset: usize) -> usize {
        let slot = self.code[offset + 1];
        println!("{:<16} {:>4}", name, slot);
        offset + 2
    }

    fn constant_instruction(&self, name: &str, offset: usize) -> usize {
        let constant = self.code[offset + 1];
        print!("{:<16} {:>4} '", name, constant);
        self.constants.values[constant as usize].print();
        println!("'");
        offset + 2
    }

    // 跳转指令 操作数为两个字节
    fn jump_instruction(&self, name: &str, sign: i32, offset: usize) -> usize {
        let mut jump = (self.code[offset + 1] as u16) << 8;
        jump |= self.code[offset + 2] as u16;
        println!(
            "{:<16} {:>4} -> {}",
            name,
            offset,
            offset + 3 + (sign * jump as i32) as usize
        );
        offset + 3
    }

    // 解释执行字节码块
    fn invoke_instruction(&self, name: &str, offset: usize) -> usize {
        let constant = self.code[offset + 1];
        let arg_count = self.code[offset + 2];
        print!("{:<16} ({} args) {:>4} '", name, arg_count, constant);
        self.constants.values[constant as usize].print();
        println!("'");
        offset + 3
    }
}
