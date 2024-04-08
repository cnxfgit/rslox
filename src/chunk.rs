use std::ptr;

enum OpCode {
    OpReturn
}

struct Chunk {
    count: usize,
    capacity: usize,
    code: *mut u8   
}

impl Chunk {
    fn new() -> Chunk {
        Chunk {
            count: 0,
            capacity: 0,
            code: ptr::null_mut()
        }
    }

    fn write_chunk(&self) {
        
    }
}