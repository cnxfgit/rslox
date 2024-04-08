mod chunk;
mod debug;
mod value;
use chunk::{Chunk, OpCode};



fn main() {
    let mut chunk = Chunk::new();

    let constant = chunk.add_constant(1.1122);

    chunk.write_chunk(OpCode::Constant as u8, 123);
    chunk.write_chunk(constant as u8, 123);
    chunk.write_chunk(OpCode::Return as u8, 123);
    chunk.disassemble("test chunk");
}
