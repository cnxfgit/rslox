use crate::{
    chunk::{Chunk, OpCode},
    value::print_value,
};

impl Chunk {
    pub fn disassemble(&self, name: &str) {
        println!("== {} ==", name);

        let mut offset = 0 as usize;
        while offset < self.count() {
            offset = self.disassemble_instruction(offset);
        }
    }

    fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);
        print!("{:4} ", self.lines[offset]);
        

        let instruction = self.code[offset];
        let instruction: OpCode = instruction.into();
        match instruction {
            OpCode::Return => return self.simple_instruction("OP_RETURN", offset),
            OpCode::Constant => return self.constant_instruction("OP_CONSTANT", offset),
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
        print_value(self.constants.values[constant as usize]);
        println!("'");
        return offset + 2;
    }
}
