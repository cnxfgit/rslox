mod chunk;
mod debug;
mod value;
mod vm;
use chunk::{Chunk, OpCode};

fn main() {
    let mut chunk = Chunk::new();

    let mut constant = chunk.add_constant(1.1122);

    chunk.write_chunk(OpCode::Constant as u8, 123);
    chunk.write_chunk(constant as u8, 123);

    constant = chunk.add_constant(3.4);
    chunk.write_chunk(OpCode::Constant as u8, 123);
    chunk.write_chunk(constant as u8, 123);

    chunk.write_chunk(OpCode::Add as u8, 123);

    constant = chunk.add_constant(5.6);
    chunk.write_chunk(OpCode::Constant as u8, 123);
    chunk.write_chunk(constant as u8, 123);

    chunk.write_chunk(OpCode::Divide as u8, 123);

    chunk.write_chunk(OpCode::Negate as u8, 123);
    chunk.write_chunk(OpCode::Return as u8, 123);
    chunk.disassemble("test chunk");
}
