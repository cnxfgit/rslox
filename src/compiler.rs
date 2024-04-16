use std::ptr::null_mut;

use crate::{
    chunk::{Chunk, OpCode},obj_val,
    object::{ObjFunction, ObjString, Obj},
    scanner::{Token, TokenType},
    value::Value,
    vm::{vm, UINT8_COUNT},
};

static RULES: [ParseRule; 40] = [
    ParseRule {
        token: "(",
        prefix: Some(Compiler::grouping),
        infix: Some(Compiler::call),
        precedence: Precedence::Call,
    },
    ParseRule {
        token: ")",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "{",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "}",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: ",",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: ".",
        prefix: None,
        infix: Some(Compiler::dot),
        precedence: Precedence::Call,
    },
    ParseRule {
        token: "-",
        prefix: Some(Compiler::unary),
        infix: Some(Compiler::binary),
        precedence: Precedence::Term,
    },
    ParseRule {
        token: "+",
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::Term,
    },
    ParseRule {
        token: ";",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "/",
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::Factor,
    },
    ParseRule {
        token: "*",
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::Factor,
    },
    ParseRule {
        token: "!",
        prefix: Some(Compiler::unary),
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "!=",
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::Equality,
    },
    ParseRule {
        token: "=",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "==",
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::Equality,
    },
    ParseRule {
        token: ">",
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::Comparison,
    },
    ParseRule {
        token: ">=",
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::Comparison,
    },
    ParseRule {
        token: "<",
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::Comparison,
    },
    ParseRule {
        token: "<=",
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::Comparison,
    },
    ParseRule {
        token: "IDENTIFIER",
        prefix: Some(Compiler::variable),
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "STRING",
        prefix: Some(Compiler::string),
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "NUMBER",
        prefix: Some(Compiler::number),
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "and",
        prefix: None,
        infix: Some(Compiler::and),
        precedence: Precedence::And,
    },
    ParseRule {
        token: "class",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "else",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "false",
        prefix: Some(Compiler::literal),
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "for",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "fun",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "if",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "nil",
        prefix: Some(Compiler::literal),
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "or",
        prefix: None,
        infix: Some(Compiler::or),
        precedence: Precedence::Or,
    },
    ParseRule {
        token: "print",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "return",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "super",
        prefix: Some(Compiler::super_),
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "this",
        prefix: Some(Compiler::this),
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "true",
        prefix: Some(Compiler::literal),
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "var",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "while",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "ERROR",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
    ParseRule {
        token: "EOF",
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    },
];

#[derive(PartialEq, Eq)]
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

enum Precedence {
    None = 0,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

impl Into<i32> for Precedence {
    fn into(self) -> i32 {
        self as i32
    }
}

impl From<i32> for Precedence {
    fn from(value: i32) -> Self {
        match value {
            0 => Precedence::None,
            1 => Precedence::Assignment,
            2 => Precedence::Or,
            3 => Precedence::And,
            4 => Precedence::Equality,
            5 => Precedence::Comparison,
            6 => Precedence::Term,
            7 => Precedence::Factor,
            8 => Precedence::Unary,
            9 => Precedence::Call,
            10 => Precedence::Primary,
        }
    }
}

// 声明返回值为 void 函数指针 ParseFn
type ParseFn = fn(&'static mut Compiler, bool) -> ();

// 解析规则
struct ParseRule {
    token: &'static str,
    prefix: Option<ParseFn>, // 前缀
    infix: Option<ParseFn>,  // 中缀
    precedence: Precedence,  // 优先级
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

// 同步token
fn synthetic_token(text: &str) -> Token {
    let mut token = Token::default();
    token.message = text.into();
    token.length = text.len();
    token
}

fn get_rule(type_: TokenType) -> ParseRule {
    RULES[type_ as usize]
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

    // 语句
    fn statement(&self) {
        if self.match_(TokenType::Print) {
            self.print_statement();
        } else if self.match_(TokenType::For) {
            self.for_statement();
        } else if self.match_(TokenType::If) {
            self.if_statement();
        } else if self.match_(TokenType::Return) {
            self.return_statement();
        } else if self.match_(TokenType::While) {
            self.while_statement();
        } else if self.match_(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    // 表达式语句
    fn expression_statement(&self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after expression.");
        self.emit_byte(OpCode::Pop as u8);
    }


    // while 语句
    fn while_statement(&self) {
        // 循环起点
        let loop_start = current_chunk().count();
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        // 如果为false直接跳到下面的pop
        let exit_jump = self.emit_jump(OpCode::JumpIfFalse as u8);
        self.emit_byte(OpCode::Pop as u8);
        self.statement();
        // 循环节点
        self.emit_loop(loop_start as i32);

        self.patch_jump(exit_jump);
        // false的跳入点
        self.emit_byte(OpCode::Pop as u8);
    }

    // 返回语句
    fn return_statement(&self) {
        if current().type_ == FunctionType::Script {
            self.error("Can't return from top-level code.");
        }

        if self.match_(TokenType::Semicolon) {
            self.emit_return();
        } else {
            if current().type_ == FunctionType::Initializer {
                self.error("Can't return a value from an initializer.");
            }

            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after return value.");
            self.emit_byte(OpCode::Return as u8);
        }
    }


    // if 语句
    fn if_statement(&self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        // then 分支跳转点
        let then_jump = self.emit_jump(OpCode::JumpIfFalse as u8);
        // 如果为false 这个 pop不会被执行  会执行下面的pop
        // 如果为 true 执行这个pop之后 跳过实体else 或者空else(只有一个pop)
        // 弹出条件表达式
        self.emit_byte(OpCode::Pop as u8);
        self.statement();

        // else 分支跳转点
        let else_jump = self.emit_jump(OpCode::Jump as u8);
        // 回写then分支跳转的长度回写
        self.patch_jump(then_jump);

        // 弹出条件表达式
        self.emit_byte(OpCode::Pop as u8);
        // then 分支过后探查 是否有else 这个if不触发的话则跳转一个 空else
        if self.match_(TokenType::Else) {
            self.statement();
        }
        // else分支跳转长度回写
        self.patch_jump(else_jump);
    }


    // for语句
    fn for_statement(&self) {
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.");
        // for 第一语句 只执行一次
        if self.match_(TokenType::Semicolon) {
            // No initializer.
        } else if self.match_(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }
        // 循环起点
        let mut loop_start = current_chunk().count() as i32;
        // for的第二语句  表达式语句
        let mut exit_jump = -1;
        if !self.match_(TokenType::Semicolon) {
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after loop condition.");

            // Jump out of the loop if the condition is false.
            exit_jump = self.emit_jump(OpCode::JumpIfFalse as u8) as i32;
            self.emit_byte(OpCode::Pop as u8); // Condition.
        }

        // for的第三语句 增量子句
        if !self.match_(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::Jump as u8);
            let increment_start = current_chunk().count() as i32;
            self.expression();
            self.emit_byte(OpCode::Pop as u8);
            self.consume(TokenType::RightParen, "Expect ')' after for clauses.");

            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }

        // for 主体
        self.statement();
        self.emit_loop(loop_start);

        // 修复跳跃
        if exit_jump != -1 {
            self.patch_jump(exit_jump as usize);
            self.emit_byte(OpCode::Pop as u8);
        }

        self.end_scope();
    }

    // 写入循环指令
    fn emit_loop(&self, loop_start: i32) {
        self.emit_byte(OpCode::Loop as u8);

        let offset = (current_chunk().count() - loop_start as usize) + 2;
        if offset > u16::MAX as usize {
            self.error("Loop body too large.");
        }

        self.emit_byte(((offset >> 8) & 0xff) as u8);
        self.emit_byte((offset & 0xff) as u8);
    }

    // print 语句
    fn print_statement(&self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after value.");
        self.emit_byte(OpCode::Print as u8);
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn grouping(&'static mut self, can_assign: bool) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn call(&'static mut self, can_assign: bool) {
        let arg_count = self.argument_list();
        self.emit_bytes(OpCode::Call as u8, arg_count);
    }

    fn dot(&'static mut self, can_assign: bool) {
        self.consume(TokenType::Identifier, "Expect property name after '.'.");
        let name = self.identifier_constant(&vm().parser.previous);

        if can_assign && self.match_(TokenType::Equal) {
            self.expression();
            self.emit_bytes(OpCode::SetProperty as u8, name);
        } else if self.match_(TokenType::LeftParen) {
            let arg_count = self.argument_list();
            self.emit_bytes(OpCode::Invoke as u8, name);
            self.emit_byte(arg_count);
        } else {
            self.emit_bytes(OpCode::GetProperty as u8, name);
        }
    }

    // 一元表达式
    fn unary(&'static mut self, can_assign: bool) {
        let operator_type = vm().parser.previous.type_;

        // Compile the operand.
        self.parse_precedence(Precedence::Unary);

        // Emit the operator instruction.
        match operator_type {
            TokenType::Bang => self.emit_byte(OpCode::Not as u8),
            TokenType::Minus => self.emit_byte(OpCode::Negate as u8),
            _ => return, // Unreachable.
        }
    }

    // 二元表达式
    fn binary(&'static mut self, can_assign: bool) {
        let operator_type = vm().parser.previous.type_;
        let rule = get_rule(operator_type);
        self.parse_precedence((rule.precedence as i32 + 1).into());

        match operator_type {
            TokenType::BangEqual => self.emit_bytes(OpCode::Equal as u8, OpCode::Not as u8),
            TokenType::EqualEqual => self.emit_byte(OpCode::Equal as u8),
            TokenType::Greater => self.emit_byte(OpCode::Greater as u8),
            TokenType::GreaterEqual => self.emit_bytes(OpCode::Less as u8, OpCode::Not as u8),
            TokenType::Less => self.emit_byte(OpCode::Less as u8),
            TokenType::LessEqual => self.emit_bytes(OpCode::Greater as u8, OpCode::Not as u8),
            TokenType::Plus => self.emit_byte(OpCode::Add as u8),
            TokenType::Minus => self.emit_byte(OpCode::Subtract as u8),
            TokenType::Star => self.emit_byte(OpCode::Multiply as u8),
            TokenType::Slash => self.emit_byte(OpCode::Divide as u8),
            _ => return, // Unreachable.
        }
    }

    // 标识符表达式
    fn variable(&'static mut self, can_assign: bool) {
        self.named_variable(&vm().parser.previous, can_assign);
    }

    // 字符串表达式
    fn string(&'static mut self, can_assign: bool) {
        self.emit_constant(obj_val!(ObjString::take_string(
            vm().parser.previous.message
        )));
    }

    // 数字表达式
    fn number(&'static mut self, can_assign: bool) {
        let value = vm().parser.previous.message.parse::<f64>().unwrap();
        self.emit_constant(Value::Number(value));
    }

    // 逻辑与
    fn and(&'static mut self, can_assign: bool) {
        let end_jump = self.emit_jump(OpCode::JumpIfFalse as u8);

        self.emit_byte(OpCode::Pop as u8);
        self.parse_precedence(Precedence::And);

        self.patch_jump(end_jump);
    }

    // 字符表达式
    fn literal(&'static mut self, can_assign: bool) {
        match vm().parser.previous.type_ {
            TokenType::False => self.emit_byte(OpCode::False as u8),
            TokenType::Nil => self.emit_byte(OpCode::Nil as u8),
            TokenType::True => self.emit_byte(OpCode::True as u8),
            _ => return, // Unreachable.
        }
    }

    // 逻辑或
    fn or(&'static mut self, can_assign: bool) {
        let else_jump = self.emit_jump(OpCode::JumpIfFalse as u8);
        let end_jump = self.emit_jump(OpCode::Jump as u8);

        self.patch_jump(else_jump);
        self.emit_byte(OpCode::Pop as u8);

        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    // 父类
    fn super_(&'static mut self, can_assign: bool) {
        if vm().class_compiler.is_null() {
            self.error("Can't use 'super' outside of a class.");
        } else if !(unsafe { *vm().class_compiler }).has_superclass {
            self.error("Can't use 'super' in a class with no superclass.");
        }

        self.consume(TokenType::Dot, "Expect '.' after 'super'.");
        self.consume(TokenType::Identifier, "Expect superclass method name.");
        let name = self.identifier_constant(&vm().parser.previous);

        self.named_variable(&synthetic_token("this"), false);
        if self.match_(TokenType::LeftParen) {
            let arg_count = self.argument_list();
            self.named_variable(&synthetic_token("super"), false);
            self.emit_bytes(OpCode::SuperInvoke as u8, name);
            self.emit_byte(arg_count);
        } else {
            self.named_variable(&synthetic_token("super"), false);
            self.emit_bytes(OpCode::GetSuper as u8, name);
        }
    }

    // this局部变量
    fn this(&'static mut self, can_assign: bool) {
        if vm().class_compiler.is_null() {
            self.error("Can't use 'this' outside of a class.");
            return;
        }

        self.variable(false);
    }

    fn emit_constant(&self, value: Value) {
        self.emit_bytes(OpCode::Constant as u8, self.make_constant(value));
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        // 获取上一格token的前缀表达式 为null的话错误
        let prefix_rule = get_rule(vm().parser.previous.type_).prefix;
        if let None = prefix_rule {
            self.error("Expect expression.");
            return;
        }
        // 执行前缀表达式  传入等号的优先级表示是否能赋值
        let can_assign = precedence as u8 <= Precedence::Assignment as u8;
        prefix_rule.unwrap()(self, can_assign);
        // 获取当前token优先级 比较传递进的优先级 传递小于等于当前的话 执行中缀表达式
        while precedence as u8 <= get_rule(vm().parser.current.type_).precedence as u8 {
            self.advance();
            let infix_rule = get_rule(vm().parser.previous.type_).infix;
            infix_rule.unwrap()(self, can_assign);
        }

        // 可以赋值且后接等号
        if can_assign && self.match_(TokenType::Equal) {
            self.error("Invalid assignment target.");
        }
    }

    fn argument_list(&mut self) -> u8 {
        let mut arg_count = 0;
        if !check(TokenType::RightParen) {
            loop {
                self.expression();
                if arg_count == 255 {
                    self.error("Can't have more than 255 arguments.")
                }
                arg_count += 1;
                if !self.match_(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after arguments.");
        arg_count
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expect variable name.");

        if self.match_(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_byte(OpCode::Nil as u8);
        }
        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        );

        self.define_variable(global);
    }

    // 函数声明
    fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expect function name.");
        mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
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
        class_compiler.enclosing = vm().class_compiler;
        vm().class_compiler = &mut class_compiler as *mut ClassCompiler;

        // 继承
        if self.match_(TokenType::Less) {
            self.consume(TokenType::Identifier, "Expect superclass name.");
            self.variable(false);

            if identifiers_equal(&class_name, &vm().parser.previous) {
                self.error("A class can't inherit from itself.");
            }

            self.begin_scope();
            self.add_local(&synthetic_token("super"));
            self.define_variable(0);

            self.named_variable(&class_name, false);
            self.emit_byte(OpCode::Inherit as u8);
            class_compiler.has_superclass = true;
        }

        self.named_variable(&class_name, false);
        self.consume(TokenType::LeftBrace, "Expect '{' before class body.");
        while !check(TokenType::RightBrace) && !check(TokenType::Eof) {
            self.method();
        }
        self.consume(TokenType::RightBrace, "Expect '}' after class body.");
        self.emit_byte(OpCode::Pop as u8);

        if class_compiler.has_superclass {
            self.end_scope();
        }

        vm().class_compiler = class_compiler.enclosing;
    }

    // 解析变量
    fn parse_variable(&mut self, error_message: &str) -> u8 {
        self.consume(TokenType::Identifier, error_message);

        self.declare_variable();
        if current().scope_depth > 0 {
            return 0;
        }

        return self.identifier_constant(&vm().parser.previous);
    }

    fn emit_return(&mut self) {
        if let FunctionType::Initializer = current().type_ {
            self.emit_bytes(OpCode::GetLocal as u8, 0);
        } else {
            self.emit_byte(OpCode::Nil as u8);
        }
        self.emit_byte(OpCode::Return as u8);
    }

    // 结束编译
    fn end_compiler(&mut self) -> *mut ObjFunction {
        self.emit_return();
        let function = current().function;

        #[cfg(feature = "debug_print_code")]
        if !vm().parser.had_error {
            let mut name;
            if (unsafe { *function }).name.is_null() {
                name = "<script>"
            } else {
                unsafe {
                    name = (*(*function).name).chars.as_str();
                }
            }
            current_chunk().disassemble_chunk(name);
        }

        // 编译结束还原 上个编译器
        vm().current_compiler = current().enclosing;
        function
    }

    fn block(&mut self) {
        while !check(TokenType::RightBrace) && !check(TokenType::Eof) {
            self.declaration();
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    // 函数定义
    fn function(&mut self, type_: FunctionType) {
        let compiler = Compiler::new(type_);
        self.begin_scope();
        // 函数参数
        self.consume(TokenType::LeftParen, "Expect '(' after function name.");
        if !check(TokenType::RightParen) {
            loop {
                (unsafe { *current().function }).arity += 1;
                if (unsafe { *current().function }).arity > 255 {
                    self.error_at_current("Can't have more than 255 parameters.");
                }
                let constant = self.parse_variable("Expect parameter name.");
                self.define_variable(constant);
                if self.match_(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after parameters.");
        self.consume(TokenType::LeftBrace, "Expect '{' before function body.");
        self.block();

        let function = self.end_compiler();
        self.emit_bytes(
            OpCode::Closure as u8,
            self.make_constant(obj_val!(function)),
        );

        let mut i = 0;
        loop {
            if i >= (unsafe { *function }).upvalue_count {
                break;
            }

            let b = if compiler.upvalues[i].is_local { 1 } else { 0 };
            self.emit_byte(b);
            self.emit_byte(compiler.upvalues[i].index);

            i += 1;
        }
    }

    fn method(&mut self) {
        self.consume(TokenType::Identifier, "Expect method name.");
        let constant = self.identifier_constant(&vm().parser.previous);

        let mut type_ = FunctionType::Method;
        if vm().parser.previous.message == "init".to_string() {
            type_ = FunctionType::Initializer;
        }
        self.function(type_);
        self.emit_bytes(OpCode::Method as u8, constant);
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
            if arg != -1 {
                get_op = OpCode::GetUpvalue as u8;
                set_op = OpCode::SetUpvalue as u8;
            } else {
                arg = self.identifier_constant(&name) as i32;
                get_op = OpCode::GetGlobal as u8;
                set_op = OpCode::SetGlobal as u8;
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

    fn add_upvalue(&self, compiler: &Compiler, index: u8, is_local: bool) -> i32 {
        let upvalue_count = (unsafe { *compiler.function }).upvalue_count;

        let mut i: i32 = 0;
        while i < upvalue_count as i32 {
            let upvalue = &compiler.upvalues[i as usize];
            if upvalue.index == index && upvalue.is_local == is_local {
                return i;
            }

            i += 1;
        }

        if upvalue_count == UINT8_COUNT {
            self.error("Too many closure variables in function.");
            return 0;
        }

        compiler.upvalues[upvalue_count].is_local = is_local;
        compiler.upvalues[upvalue_count].index = index;
        let result = (unsafe { *compiler.function }).upvalue_count;
        (unsafe { *compiler.function }).upvalue_count += 1;
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

            i -= 1;
        }

        return -1;
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

    // 写入跳转分支 使用两个字节占位符做操作数
    fn emit_jump(&self, instruction: u8) -> usize {
        self.emit_byte(instruction);
        self.emit_byte(0xff);
        self.emit_byte(0xff);
        current_chunk().count() - 2
    }

    fn patch_jump(&self, offset: usize) {
        // -offset得到 字节指令的位置  -2 再得到then语句的位置
        let jump = current_chunk().count() - offset - 2;

        // 最大只能跳转两个字节的字节码
        if jump > u16::MAX as usize {
            self.error("Too much code to jump over.");
        }

        // 回写需要跳过的大小
        current_chunk().code[offset] = ((jump >> 8) & 0xff) as u8;
        current_chunk().code[offset + 1] = (jump & 0xff) as u8;
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

            i -= 1;
        }
        self.add_local(name);
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

    fn begin_scope(&self) {
        current().scope_depth += 1;
    }

    fn end_scope(&mut self) {
        current().scope_depth -= 1;

        while current().local_count > 0
            && current().locals[current().local_count - 1].depth as usize > current().scope_depth
        {
            // 被捕获的需要推送到闭包
            if current().locals[current().local_count - 1].is_captured {
                self.emit_byte(OpCode::CloseUpvalue as u8);
            } else {
                self.emit_byte(OpCode::Pop as u8);
            }
            current().local_count -= 1;
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
        self.error_at(&vm().parser.current.clone(), message);
    }

    fn error(&mut self, message: &str) {
        self.error_at(&vm().parser.previous.clone(), message);
    }

    fn error_at(&mut self, token: &Token, message: &str) {
        vm().parser.panic_mode = true;

        eprint!("[line {}] Error", token.line);

        if token.type_ == TokenType::Eof {
            eprint!(" at end");
        } else if let TokenType::Error = token.type_ {
            // Nothing.
        } else {
            eprint!(
                " at '{}'",
                String::from_utf8(
                    vm().scanner.unwrap().source.as_bytes()
                        [token.start..token.start + token.length]
                        .to_vec()
                )
                .unwrap()
            );
        }

        eprintln!(": {}", message);
        vm().parser.had_error = true;
    }
}
