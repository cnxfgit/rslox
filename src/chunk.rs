use crate::value::{Value, ValueArray};

pub enum OpCode {
    Constant,     // 写入常量
    Nil,          // 空指令 nil
    True,         // true指令
    False,        // false指令
    Pop,          // 弹出指令
    GetLocal,     // 获取局部变量
    SetLocal,     // 赋值局部变量
    GetGlobal,    // 获取全局变量
    DefineGlobal, // 定义全局变量
    SetGlobal,    // 赋值全局变量
    GetUpvalue,   // 获取升值指令
    SetUpvalue,   // 赋值升值指令
    GetProperty,  // 获取属性指令
    SetProperty,  // 赋值属性指令
    GetSuper,     // 获取父类指令
    Equal,        // 赋值指令 =
    Greater,      // 大于指令 >
    Less,         // 小于指令 <
    Add,          // 加指令 +
    Subtract,     // 减指令 -
    Multiply,     // 乘指令 *
    Divide,       // 除指令 /
    Not,          // 非指令 !
    Negate,       // 负指令 -
    Print,        // 打印指令
    Jump,         // 分支跳转指令
    JumpIfFalse,  // if false分支跳转指令
    Loop,         // 循环指令
    Call,         // 调用指令
    Invoke,       // 执行指令
    SuperInvoke,  // 父类执行指令
    Closure,      // 闭包指令
    CloseUpvalue, // 关闭提升值
    Return,       // 返回指令
    Class,        // 类指令
    Inherit,      // 继承指令
    Method,       // 方法指令
}

impl Into<OpCode> for u8 {
    fn into(self) -> OpCode {
        match self {
            0 => OpCode::Constant,
            1 => OpCode::Nil,
            2 => OpCode::True,
            3 => OpCode::False,
            4 => OpCode::Pop,
            5 => OpCode::GetLocal,
            6 => OpCode::SetLocal,
            7 => OpCode::GetGlobal,
            8 => OpCode::DefineGlobal,
            9 => OpCode::SetGlobal,
            10 => OpCode::GetUpvalue,
            11 => OpCode::SetUpvalue,
            12 => OpCode::GetProperty,
            13 => OpCode::SetProperty,
            14 => OpCode::GetSuper,
            15 => OpCode::Equal,
            16 => OpCode::Greater,
            17 => OpCode::Less,
            18 => OpCode::Add,
            19 => OpCode::Subtract,
            20 => OpCode::Multiply,
            21 => OpCode::Divide,
            22 => OpCode::Not,
            23 => OpCode::Negate,
            24 => OpCode::Print,
            25 => OpCode::Jump,
            26 => OpCode::JumpIfFalse,
            27 => OpCode::Loop,
            28 => OpCode::Call,
            29 => OpCode::Invoke,
            30 => OpCode::SuperInvoke,
            31 => OpCode::Closure,
            32 => OpCode::CloseUpvalue,
            33 => OpCode::Return,
            34 => OpCode::Class,
            35 => OpCode::Inherit,
            36 => OpCode::Method,
            _ => panic!("Invalid Opcode."),
        }
    }
}

pub struct Chunk {
    pub code: Vec<u8>,
    pub lines: Vec<usize>,
    pub constants: ValueArray,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk {
            code: vec![],
            lines: vec![],
            constants: ValueArray::new(),
        }
    }

    pub fn write_chunk(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.write_value(value);
        return self.constants.count() - 1;
    }

    pub fn count(&self) -> usize {
        self.code.len()
    }
}
