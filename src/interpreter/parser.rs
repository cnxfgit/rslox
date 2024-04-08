use std::panic::{catch_unwind, UnwindSafe};

use crate::{
    expr::{self, Expr},
    object::Object,
    token::{Token, TokenType},
    util::parse_error,
};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: &Vec<Token>) -> Parser {
        Parser {
            tokens: tokens.clone(),
            current: 0,
        }
    }

    pub fn parse(mut self) -> Option<Box<dyn Expr>> {
        let result = catch_unwind(move || {
            return self.expression();
        });

        if let Ok(r) = result {
            return Some(r);
        }
        return None;
    }

    fn expression(&mut self) -> Box<dyn Expr> {
        self.equality()
    }

    fn equality(&mut self) -> Box<dyn Expr> {
        let mut expr = self.comparison();

        while self.match_(&[TokenType::BangEqual, TokenType::EqualEqual]) {
            let operator = self.previous();
            let right = self.comparison();
            expr = Box::new(expr::Binary::new(expr, operator, right));
        }

        expr
    }

    fn comparison(&mut self) -> Box<dyn Expr> {
        let mut expr = self.term();

        while self.match_(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let operator = self.previous();
            let right = self.term();
            expr = Box::new(expr::Binary::new(expr, operator, right));
        }

        expr
    }

    fn term(&mut self) -> Box<dyn Expr> {
        let mut expr = self.factor();

        while self.match_(&[TokenType::Minus, TokenType::Plus]) {
            let operator = self.previous();
            let right = self.factor();
            expr = Box::new(expr::Binary::new(expr, operator, right));
        }

        expr
    }

    fn factor(&mut self) -> Box<dyn Expr> {
        let mut expr = self.unary();

        while self.match_(&[TokenType::Slash, TokenType::Star]) {
            let operator = self.previous();
            let right = self.unary();
            expr = Box::new(expr::Binary::new(expr, operator, right));
        }

        expr
    }

    fn unary(&mut self) -> Box<dyn Expr> {
        if self.match_(&[TokenType::Bang, TokenType::Minus]) {
            let operator = self.previous();
            let right = self.unary();
            return Box::new(expr::Unary::new(operator, right));
        }

        self.primary()
    }

    fn primary(&mut self) -> Box<dyn Expr> {
        if self.match_(&[TokenType::False]) {
            return Box::new(expr::Literal::new(Object::Boolean(false)));
        }
        if self.match_(&[TokenType::True]) {
            return Box::new(expr::Literal::new(Object::Boolean(true)));
        }
        if self.match_(&[TokenType::Nil]) {
            return Box::new(expr::Literal::new(Object::Nil));
        }

        if self.match_(&[TokenType::Number, TokenType::String]) {
            return Box::new(expr::Literal::new(self.previous().literal));
        }

        if self.match_(&[TokenType::LeftParen]) {
            let expr = self.expression();
            self.consume(
                &TokenType::RightParen,
                "Expect ')' after expression.".into(),
            );
            return Box::new(expr::Grouping::new(expr));
        }

        parse_error(self.peek(), "Expect expression.");
        panic!("error");
    }

    fn match_(&mut self, types: &[TokenType]) -> bool {
        for type_ in types {
            if self.check(type_) {
                self.advance();
                return true;
            }
        }

        false
    }

    fn consume(&mut self, type_: &TokenType, message: &str) -> Token {
        if self.check(type_) {
            return self.advance();
        }

        parse_error(self.peek(), message);
        panic!("error");
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn check(&mut self, type_: &TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        &self.peek().type_ == type_
    }

    fn is_at_end(&mut self) -> bool {
        self.peek().type_ == TokenType::Eof
    }

    fn peek(&mut self) -> &Token {
        self.tokens.get(self.current).unwrap()
    }

    fn previous(&self) -> Token {
        self.tokens.get(self.current - 1).unwrap().clone()
    }

    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if self.previous().type_ == TokenType::Semicolon {
                return;
            }

            match self.peek().type_ {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => {
                    return;
                }
                _ => {}
            }

            self.advance();
        }
    }
}
