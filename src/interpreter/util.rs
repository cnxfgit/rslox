use crate::{
    expr::{self, Expr},
    object::Object,
    token::{Token, TokenType},
};

static mut HAD_ERROR: bool = false;

pub fn had_error_set(value: bool) {
    unsafe {
        HAD_ERROR = value;
    }
}

pub fn had_error_get() -> bool {
    unsafe { HAD_ERROR }
}

pub fn error(line: usize, message: &'static str) {
    report(line, "", message)
}

pub fn parse_error(token: &Token, message: &str) {
    if token.type_ == TokenType::Eof {
        report(token.line, " at end", message);
    } else {
        report(token.line, &format!(" at '{}'", token.lexeme), message);
    }
}

pub fn report(line: usize, where_: &str, message: &str) {
    eprintln!("[line {}] Error {}: {}", line, where_, message);
    had_error_set(true);
}

pub fn is_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}

pub fn is_alpha(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'
}
