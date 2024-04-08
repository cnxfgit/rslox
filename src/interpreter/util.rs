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

#[derive(Clone)]
pub struct AstPrinter {}

impl AstPrinter {
    pub fn new() -> AstPrinter {
        AstPrinter {}
    }

    pub fn print(&self, expr: Box<dyn Expr>) -> String {
        let object = expr.accept(Box::new(self.clone()));
        match object {
            Object::String(s) => s,
            _ => "".into()
        }
    }

    fn parenthesize(&self, name: String, exprs: &[&Box<dyn Expr>]) -> String {
        let mut string = String::new();

        string.push_str("(");
        string.push_str(&name);
        for expr in exprs {
            string.push_str(" ");
            string.push_str(&expr.accept(Box::new(self.clone())).to_string());
        }

        string.push_str(")");
        return string;
    }
}

impl expr::Visitor for AstPrinter {
    fn visit_assign_expr(&self, expr: &expr::Assign) -> Object {
        Object::Nil
    }

    fn visit_binary_expr(&self, expr: &expr::Binary) -> Object {
        Object::String(self.parenthesize(expr.operator.lexeme.clone(), &[&expr.left, &expr.right]))
    }

    fn visit_call_expr(&self, expr: &expr::Call) -> Object {
        Object::Nil
    }

    fn visit_get_expr(&self, expr: &expr::Get) -> Object {
        Object::Nil
    }

    fn visit_grouping_expr(&self, expr: &expr::Grouping) -> Object {
        Object::String(self.parenthesize("group".into(), &[&expr.expression]))
    }

    fn visit_literal_expr(&self, expr: &expr::Literal) -> Object {
        if let Object::Nil = expr.value {
            return Object::String(Object::Nil.to_string());
        }
        Object::String(expr.value.to_string())
    }

    fn visit_logical_expr(&self, expr: &expr::Logical) -> Object {
        Object::Nil
    }

    fn visit_set_expr(&self, expr: &expr::Set) -> Object {
        Object::Nil
    }

    fn visit_super_expr(&self, expr: &expr::Super) -> Object {
        Object::Nil
    }

    fn visit_this_expr(&self, expr: &expr::This) -> Object {
        Object::Nil
    }

    fn visit_unary_expr(&self, expr: &expr::Unary) -> Object {
        Object::String(self.parenthesize(expr.operator.lexeme.clone(), &[&expr.right]))
    }

    fn visit_variable_expr(&self, expr: &expr::Variable) -> Object {
        Object::Nil
    }
}
