use crate::chunk::{Chunk, OpCode};

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

    pub fn disassemble(&self, name: &str) {
        println!("== {} ==", name);

        let mut offset = 0 as usize;
        while offset < self.count() {
            offset = self.disassemble_instruction(offset);
        }
    }

    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);
        print!("{:4} ", self.lines[offset]);

        let instruction = self.code[offset];
        let instruction: OpCode = instruction.into();
        match instruction {
            OpCode::Constant => return self.constant_instruction("OP_CONSTANT", offset),
            OpCode::Add => return self.simple_instruction("OP_ADD", offset),
            OpCode::Subtract => return self.simple_instruction("OP_SUBTRACT", offset),
            OpCode::Multiply => return self.simple_instruction("OP_MULTIPLY", offset),
            OpCode::Divide => return self.simple_instruction("OP_DIVIDE", offset),
            OpCode::Negate => return self.simple_instruction("OP_NEGATE", offset),
            OpCode::Return => return self.simple_instruction("OP_RETURN", offset),
            _ => {
                println!("Unknown opcode {}", instruction as u8);
                return offset + 1;
            }
        }
    }

    fn simple_instruction(&self, name: &str, offset: usize) -> usize {
        println!("{} ", name);
        return offset + 1;
    }

    fn constant_instruction(&self, name: &str, offset: usize) -> usize {
        let constant = self.code[offset + 1];
        print!("{:<16} {:>4} '", name, constant);
        self.constants.values[constant as usize].print();
        println!("'");
        return offset + 2;
    }
}
