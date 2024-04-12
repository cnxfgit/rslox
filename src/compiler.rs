use std::{ptr::null_mut, simd::i32x1};

use crate::{
    chunk::{Chunk, OpCode},
    obj_val,
    object::{ObjFunction, ObjString},
    scanner::{Token, TokenType},
    value::Value,
    vm::{vm, UINT8_COUNT},
};

// 函数类型
pub enum FunctionType {
    Function,    // 正常函数
    Initializer, // 构造函数
    Method,      // 方法
    Script,      // 主执行体
}

// 局部变量
struct Local {
    name: Token,       // 变量名
    depth: i32,        // 作用域深度
    is_captured: bool, // 是否被捕获
}

impl Local {
    fn new() -> Local {
        Local {
            name: Token::default(),
            depth: 0,
            is_captured: false,
        }
    }
}

// 提升值
#[derive(Clone, Copy)]
struct Upvalue {
    index: u8,      // 提示值索引
    is_local: bool, // 是否为局部变量
}

impl Upvalue {
    fn new() -> Upvalue {
        Upvalue {
            index: 0,
            is_local: false,
        }
    }
}

// 类编译器
pub struct ClassCompiler {
    enclosing: *mut ClassCompiler, // 上一个类编译器
    has_superclass: bool,          // 是否存在父类
}

impl ClassCompiler {
    fn new() -> ClassCompiler {
        ClassCompiler {
            enclosing: null_mut(),
            has_superclass: false,
        }
    }
}

pub struct Compiler {
    enclosing: *mut Compiler,   // 上一个编译器 用来还原current
    function: *mut ObjFunction, // 当前编译函数对象
    type_: FunctionType,        // 当前函数类型

    locals: Vec<Local>,     // 局部变量数组
    local_count: usize,     // 局部变量数量
    upvalues: Vec<Upvalue>, // 提升值数组
    scope_depth: usize,     // 局部变量作用域深度
}

pub struct Parser {
    current: Token,
    previous: Token,
    pub had_error: bool,
    pub panic_mode: bool,
}

impl Parser {
    pub fn new() -> Parser {
        Parser {
            current: Token::default(),
            previous: Token::default(),
            had_error: false,
            panic_mode: false,
        }
    }
}

fn check(type_: TokenType) -> bool {
    vm().parser.current.type_ == type_
}

fn current_chunk() -> &'static Chunk {
    unsafe { &(*(*vm().current_compiler).function).chunk }
}

fn current() -> &'static Compiler {
    &(unsafe { *vm().current_compiler })
}

fn identifiers_equal(a: &Token, b: &Token) -> bool {
    if a.length != b.length {
        return false;
    }
    a.message == b.message
}

fn mark_initialized() {
    // 全局函数声明时没必要标记
    if current().scope_depth == 0 {
        return;
    }
    current().locals[current().local_count - 1].depth = current().scope_depth as i32;
}

impl Compiler {
    pub fn new(type_: FunctionType) -> Compiler {
        let mut compiler = Compiler {
            enclosing: vm().current_compiler,
            function: ObjFunction::new(),
            type_,
            locals: Vec::with_capacity(UINT8_COUNT),
            local_count: 0,
            upvalues: Vec::with_capacity(UINT8_COUNT),
            scope_depth: 0,
        };

        unsafe { vm().current_compiler = &mut compiler as *mut Compiler }

        if let type_ = FunctionType::Script {
        } else {
            let start = vm().parser.previous.start;
            let length = vm().parser.previous.length;
            (unsafe { *compiler.function }).name = ObjString::take_string(
                String::from_utf8(
                    vm().scanner.unwrap().source.as_bytes()[start..start + length].to_vec(),
                )
                .unwrap(),
            );
        }

        // 局部插槽将空字符串占用 无法显式使用
        let local = &compiler.locals[compiler.local_count];
        compiler.local_count += 1;
        local.depth = 0;
        local.is_captured = false;

        match type_ {
            FunctionType::Function => {
                local.name = Token::default();
            }
            _ => {
                local.name.start = 0;
                local.name.length = 4;
                local.name.message = "this".into();
            }
        }
        compiler
    }

    fn advance(&mut self) {
        vm().parser.previous = vm().parser.current;

        loop {
            vm().parser.current = vm().scanner.unwrap().scan_token();
            if let TokenType::Error = vm().parser.current.type_ {
            } else {
                break;
            }

            self.error_at_current(&vm().parser.current.message);
        }
    }

    fn match_(&self, type_: TokenType) -> bool {
        if !check(type_) {
            return false;
        }
        self.advance();
        true
    }

    fn declaration(&self) {
        if self.match_(TokenType::Class) {
            self.class_declaration();
        } else if self.match_(TokenType::Fun) {
            self.fun_declaration();
        } else if self.match_(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        // 如果处于异常模式  则同步掉异常继续编译
        if vm().parser.panic_mode {
            self.synchronize();
        }
    }

    fn class_declaration(&self) {
        self.consume(TokenType::Identifier, "Expect class name.");
        let class_name = vm().parser.previous;
        let name_constant = self.identifier_constant(&vm().parser.previous);
        self.declare_variable();

        self.emit_bytes(OpCode::Class as u8, name_constant);
        self.define_variable(name_constant);

        let mut class_compiler = ClassCompiler::new();
        class_compiler.has_superclass = false;
        class_compiler.enclosing = vm().current_class;
        vm().current_class = &mut class_compiler as *mut ClassCompiler;

         // 继承
         if self.match_(TokenType::Less) {
        self.consume(TokenType::Identifier, "Expect superclass name.");
        self.variable(false);

        if identifiersEqual(&className, &parser.previous) {
            error("A class can't inherit from itself.");
        }

        beginScope();
        addLocal(syntheticToken("super"));
        defineVariable(0);

        namedVariable(className, false);
        emitByte(OP_INHERIT);
        classCompiler.hasSuperclass = true;
    }
    }

    fn named_variable(&self, name: &Token, can_assign: bool) {
        let mut get_op: u8 = 0;
        let mut set_op: u8 = 0;
        let mut arg = self.resolve_local(current(), &name);
        if arg != -1 {
            get_op = OpCode::GetLocal as u8;
            set_op = OpCode::SetLocal as u8;
        } else {
            arg = self.resolve_upvalue(current(), &name);
            if  arg != -1 {
                get_op = OP_GET_UPVALUE;
                set_op = OP_SET_UPVALUE;
            } else {
                arg = identifierConstant(&name);
                get_op = OP_GET_GLOBAL;
                set_op = OP_SET_GLOBAL;
            }
        }
    }

    fn resolve_upvalue(&self, compiler: &Compiler, name: &Token) -> i32 {
        if compiler.enclosing.is_null() {
            return -1;
        }
        let local = self.resolve_local(&mut (unsafe { *compiler.enclosing }), name);
        if local != -1 {
            (unsafe { *compiler.enclosing }).locals[local as usize].is_captured = true;
            return self.add_upvalue(compiler, local as u8, true);
        }

        let upvalue = self.resolve_upvalue(&mut (unsafe { *compiler.enclosing }), name);
        if upvalue != -1 {
            return self.add_upvalue(compiler, upvalue as u8, false);
        }

        return -1;
    }
    
    fn add_upvalue(&self, compiler: &Compiler, index : u8, is_local: bool) -> i32 {
        let upvalue_count = (unsafe { *compiler.function }).upvalue_count;

        let mut i:i32 = 0;
        while i < upvalue_count as i32 {
            let upvalue = &compiler.upvalues[i as usize];
            if upvalue.index == index && upvalue.is_local == is_local {
                return i;
            }

            i+=1;
        }

        if upvalue_count == UINT8_COUNT {
            self.error("Too many closure variables in function.");
            return 0;
        }
    
        compiler.upvalues[upvalue_count].is_local = is_local;
        compiler.upvalues[upvalue_count].index = index;
        let result = (unsafe { *compiler.function }).upvalue_count;
        (unsafe { *compiler.function }).upvalue_count+=1;
        result as i32
    }

    fn resolve_local(&self, compiler: &Compiler, name: &Token) -> i32 {
        let mut i = (compiler.local_count - 1) as i32;
        while i >= 0 {
            let local = &compiler.locals[i as usize];
            if identifiers_equal(name, &local.name) {
                if local.depth == -1 {
                    self.error("Can't read local variable in its own initializer.");
                }
                return i;
            }

            i-=1;
        }
    
        return -1;
    }

    fn variable(&self, can_assign: bool) {
        self.named_variable(&vm().parser.previous, can_assign);
    }

    fn define_variable(&self, global: u8) {
        if current().scope_depth > 0 {
            mark_initialized();
            return;
        }
        self.emit_bytes(OpCode::DefineGlobal as u8, global);
    }

    fn emit_bytes(&self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    fn emit_byte(&self, byte: u8) {
        current_chunk().write_chunk(byte, vm().parser.previous.line);
    }

    fn declare_variable(&self) {
        if current().scope_depth == 0 {
            return;
        }

        let name = &vm().parser.previous;

        let mut i = current().local_count - 1;
        while i >= 0 {
            let local = &current().locals[i];
            if local.depth != -1 && local.depth < current().scope_depth as i32 {
                break;
            }

            if identifiers_equal(name, &local.name) {
                self.error("Already a variable with this name in this scope.");
            }
        }
        self.add_local(name);

        i -= 1;
    }

    fn add_local(&self, name: &Token) {
        if current().local_count == UINT8_COUNT {
            self.error("Too many local variables in function.");
            return;
        }

        let local = &current().locals[current().local_count];
        current().local_count += 1;
        local.name = name.clone();
        local.depth = -1;
        local.is_captured = false;
    }

    fn identifier_constant(&self, name: &Token) -> u8 {
        self.make_constant(obj_val!(ObjString::take_string(name.message)))
    }

    fn make_constant(&self, value: Value) -> u8 {
        let constant = current_chunk().add_constant(value);
        if constant > u8::MAX as usize {
            self.error("Too many constants in one chunk.");
            return 0;
        }

        constant as u8
    }

    fn synchronize(&self) {
        vm().parser.panic_mode = false;

        while vm().parser.current.type_ != TokenType::Eof {
            if vm().parser.previous.type_ == TokenType::Semicolon {
                return;
            }
            match vm().parser.current.type_ {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => return,
                _ => {} // Do nothing.
            }

            self.advance();
        }
    }

    pub fn compile(&self) -> *mut ObjFunction {
        self.advance();

        while !self.match_(TokenType::Eof) {
            self.declaration();
        }

        let function = self.end_compiler();
        if vm().parser.had_error {
            null_mut()
        } else {
            function
        }
    }

    fn consume(&mut self, type_: TokenType, message: &str) {
        if vm().parser.current.type_ == type_ {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    fn error_at_current(&mut self, message: &str) {
        self.error_at(&self.parser.current.clone(), message);
    }

    fn error(&mut self, message: &str) {
        self.error_at(&self.parser.previous.clone(), message);
    }

    fn error_at(&mut self, token: &Token, message: &str) {
        self.parser.panic_mode = true;

        eprint!("[line {}] Error", token.line);

        if token.type_ == TokenType::Eof {
            eprint!(" at end");
        } else if let TokenType::Error(_) = token.type_ {
            // Nothing.
        } else {
            eprint!(
                " at '{}'",
                String::from_utf8(
                    self.scanner.source.as_bytes()[token.start..token.start + token.length]
                        .to_vec()
                )
                .unwrap()
            );
        }

        eprintln!(": {}", message);
        self.parser.had_error = true;
    }
}
