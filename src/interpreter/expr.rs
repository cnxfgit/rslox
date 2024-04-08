use crate::{object::Object, token::Token};

pub trait Visitor {
    fn visit_assign_expr(&self, expr: &Assign) -> Object;
    fn visit_binary_expr(&self, expr: &Binary) -> Object;
    fn visit_call_expr(&self, expr: &Call) -> Object;
    fn visit_get_expr(&self, expr: &Get) -> Object;
    fn visit_grouping_expr(&self, expr: &Grouping) -> Object;
    fn visit_literal_expr(&self, expr: &Literal) -> Object;
    fn visit_logical_expr(&self, expr: &Logical) -> Object;
    fn visit_set_expr(&self, expr: &Set) -> Object;
    fn visit_super_expr(&self, expr: &Super) -> Object;
    fn visit_this_expr(&self, expr: &This) -> Object;
    fn visit_unary_expr(&self, expr: &Unary) -> Object;
    fn visit_variable_expr(&self, expr: &Variable) -> Object;
}

pub trait Expr {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object;
}

pub struct Assign {
    name: Token,
    value: Box<dyn Expr>,
}

impl Assign {
    pub fn new(name: Token, value: Box<dyn Expr>) -> Assign {
        Assign { name, value }
    }
}

impl Expr for Assign {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_assign_expr(self)
    }
}

pub struct Binary {
    left: Box<dyn Expr>,
    operator: Token,
    right: Box<dyn Expr>,
}

impl Binary {
    pub fn new(left: Box<dyn Expr>, operator: Token, right: Box<dyn Expr>) -> Binary {
        Binary {
            left,
            operator,
            right,
        }
    }
}

impl Expr for Binary {
     fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_binary_expr(self)
    }
}

pub struct Call {
    callee: Box<dyn Expr>,
    paren: Token,
    arguments: Vec<Box<dyn Expr>>,
}

impl Call {
    pub fn new(callee: Box<dyn Expr>, paren: Token, arguments: Vec<Box<dyn Expr>>) -> Call {
        Call {
            callee,
            paren,
            arguments,
        }
    }
}

impl Expr for Call {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_call_expr(self)
    }
}

pub struct Get {
    object: Box<dyn Expr>,
    name: Token,
}

impl Get {
    pub fn new(object: Box<dyn Expr>, name: Token) -> Get {
        Get { object, name }
    }
}

impl Expr for Get {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_get_expr(self)
    }
}

pub struct Grouping {
    expression: Box<dyn Expr>,
}

impl Grouping {
    pub fn new(expression: Box<dyn Expr>) -> Grouping {
        Grouping { expression }
    }
}

impl Expr for Grouping {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_grouping_expr(self)
    }
}

pub struct Literal {
    value: Object,
}

impl Literal {
    pub fn new(value: Object) -> Literal {
        Literal { value }
    }
}

impl Expr for Literal {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_literal_expr(self)
    }
}

pub struct Logical {
    left: Box<dyn Expr>,
    operator: Token,
    right: Box<dyn Expr>,
}

impl Logical {
    pub fn new(left: Box<dyn Expr>, operator: Token, right: Box<dyn Expr>) -> Logical {
        Logical {
            left,
            operator,
            right,
        }
    }
}

impl Expr for Logical {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_logical_expr(self)
    }
}

pub struct Set {
    object: Box<dyn Expr>,
    name: Token,
    value: Box<dyn Expr>,
}

impl Set {
    pub fn new(object: Box<dyn Expr>, name: Token, value: Box<dyn Expr>) -> Set {
        Set {
            object,
            name,
            value,
        }
    }
}

impl Expr for Set {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_set_expr(self)
    }
}

pub struct Super {
    keyword: Token,
    method: Token,
}

impl Super {
    pub fn new(keyword: Token, method: Token) -> Super {
        Super { keyword, method }
    }
}

impl Expr for Super {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_super_expr(self)
    }
}

pub struct This {
    keyword: Token,
}

impl This {
    pub fn new(keyword: Token) -> This {
        This { keyword }
    }
}

impl Expr for This {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_this_expr(self)
    }
}

pub struct Unary {
    operator: Token,
    right: Box<dyn Expr>,
}

impl Unary {
    pub fn new(operator: Token, right: Box<dyn Expr>) -> Unary {
        Unary { operator, right }
    }
}

impl Expr for Unary {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_unary_expr(self)
    }
}

pub struct Variable {
    operator: Token,
    right: Box<dyn Expr>,
}

impl Variable {
    pub fn new(operator: Token, right: Box<dyn Expr>) -> Variable {
        Variable { operator, right }
    }
}

impl Expr for Variable {
    fn accept(&self, visitor: Box<dyn Visitor>) -> Object {
        visitor.visit_variable_expr(self)
    }
}
