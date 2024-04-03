use crate::object::Object;
use crate::token::{Token, TokenType};
use crate::util::error;

pub struct Scanner {
    source: String,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
}

impl Scanner {
    pub fn new(source: String) -> Scanner {
        Scanner {
            source: source,
            tokens: vec![],
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_tokens(&mut self) -> &Vec<Token> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.tokens.push(Token::new(
            TokenType::Eof,
            "".into(),
            Object::Nil,
            self.line,
        ));
        return &self.tokens;
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn scan_token(&mut self) {
        let c: char = self.advance();
        match c {
            '(' => self.addToken(TokenType::LeftParen),
            ')' => self.addToken(TokenType::RightParen),
            '{' => self.addToken(TokenType::LeftBrace),
            '}' => self.addToken(TokenType::RightBrace),
            ',' => self.addToken(TokenType::Comma),
            '.' => self.addToken(TokenType::Dot),
            '-' => self.addToken(TokenType::Minus),
            '+' => self.addToken(TokenType::Plus),
            ';' => self.addToken(TokenType::Semicolon),
            '*' => self.addToken(TokenType::String),
            _ => error(self.line, "Unexpected character."),
        }
    }

    fn advance(&mut self) -> char {
        let result = self.source.as_bytes()[self.current] as char;
        self.current += 1;
        result
    }

    fn addToken(&mut self, type_: TokenType) {
        self.addToken1(type_, Object::Nil)
    }

    fn addToken1(&mut self, type_: TokenType, literal: Object) {
        let slice = &self.source[self.start..self.current];
        let text: String = slice.into();
        self.tokens
            .push(Token::new(type_, text, literal, self.line));
    }
}
