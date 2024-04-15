pub struct Scanner {
    pub source: String,
    start: usize,
    current: usize,
    line: usize,
}

impl Scanner {
    pub fn new(source: String) -> Scanner {
        Scanner {
            source: source,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_token(&mut self) -> Token {
        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        self.skip_whitespace();

        self.start = self.current;

        let c = self.advance();
        if is_alpha(c) {
            return self.identifier();
        }
        if is_digit(c) {
            return self.number();
        }

        match c {
            '(' => return self.make_token(TokenType::LeftParen),
            ')' => return self.make_token(TokenType::RightParen),
            '{' => return self.make_token(TokenType::LeftBrace),
            '}' => return self.make_token(TokenType::RightBrace),
            ';' => return self.make_token(TokenType::Semicolon),
            ',' => return self.make_token(TokenType::Comma),
            '.' => return self.make_token(TokenType::Dot),
            '-' => return self.make_token(TokenType::Minus),
            '+' => return self.make_token(TokenType::Plus),
            '/' => return self.make_token(TokenType::Slash),
            '*' => return self.make_token(TokenType::Star),
            '!' => {
                if self.match_('=') {
                    return self.make_token(TokenType::BangEqual);
                } else {
                    return self.make_token(TokenType::Bang);
                }
            }
            '=' => {
                if self.match_('=') {
                    return self.make_token(TokenType::EqualEqual);
                } else {
                    return self.make_token(TokenType::Equal);
                }
            }
            '<' => {
                if self.match_('=') {
                    return self.make_token(TokenType::LessEqual);
                } else {
                    return self.make_token(TokenType::Less);
                }
            }
            '>' => {
                if self.match_('=') {
                    return self.make_token(TokenType::GreaterEqual);
                } else {
                    return self.make_token(TokenType::Greater);
                }
            }
            '"' => return self.string(),
            _ => {}
        }

        return self.error_token("Unexpected character.");
    }

    fn identifier(&mut self) -> Token {
        while is_alpha(self.peek()) || is_digit(self.peek()) {
            self.advance();
        }
        let type_ = self.identifier_type();
        return self.make_token(type_);
    }

    fn identifier_type(&mut self) -> TokenType {
        match self.source.as_bytes()[self.start] as char {
            'a' => return self.check_keyword(1, 2, "nd", TokenType::And),
            'c' => return self.check_keyword(1, 4, "lass", TokenType::Class),
            'e' => return self.check_keyword(1, 3, "lse", TokenType::Else),
            'f' => {
                if self.current - self.start > 1 {
                    match self.source.as_bytes()[self.start + 1] as char {
                        'a' => return self.check_keyword(2, 3, "lse", TokenType::False),
                        'o' => return self.check_keyword(2, 1, "r", TokenType::For),
                        'u' => return self.check_keyword(2, 1, "n", TokenType::Fun),
                        _ => {}
                    }
                }
            }
            'i' => return self.check_keyword(1, 1, "f", TokenType::If),
            'n' => return self.check_keyword(1, 2, "il", TokenType::Nil),
            'o' => return self.check_keyword(1, 1, "r", TokenType::Or),
            'p' => return self.check_keyword(1, 4, "rint", TokenType::Print),
            'r' => return self.check_keyword(1, 5, "eturn", TokenType::Return),
            's' => return self.check_keyword(1, 4, "uper", TokenType::Super),
            't' => {
                if self.current - self.start > 1 {
                    match self.source.as_bytes()[self.start + 1] as char {
                        'h' => return self.check_keyword(2, 2, "is", TokenType::This),
                        'r' => return self.check_keyword(2, 2, "ue", TokenType::True),
                        _ => {}
                    }
                }
            }
            'v' => return self.check_keyword(1, 2, "ar", TokenType::Var),
            'w' => return self.check_keyword(1, 4, "hile", TokenType::While),
            _ => {}
        }

        TokenType::Identifier
    }

    fn check_keyword(
        &self,
        start: usize,
        length: usize,
        rest: &str,
        type_: TokenType,
    ) -> TokenType {
        let begin = self.start + start;
        if self.current - self.start == start + length
            && self.sub_current()
                == rest
        {
            return type_;
        }

        TokenType::Identifier
    }

    fn number(&mut self) -> Token {
        while is_digit(self.peek()) {
            self.advance();
        }

        // Look for a fractional part.
        if self.peek() == '.' && is_digit(self.peek_next()) {
            // Consume the ".".
            self.advance();

            while is_digit(self.peek()) {
                self.advance();
            }
        }

        return self.make_token(TokenType::Number);
    }

    fn string(&mut self) -> Token {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            return self.error_token("Unterminated string.");
        }

        // The closing quote.
        self.advance();
        return self.make_token(TokenType::String);
    }

    fn skip_whitespace(&mut self) {
        loop {
            let c = self.peek();
            match c {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                '/' => {
                    if self.peek_next() == '/' {
                        // A comment goes until the end of the line.
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            }
        }
    }

    fn peek_next(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        return self.source.as_bytes()[self.current + 1] as char;
    }

    fn peek(&self) -> char {
        return self.source.as_bytes()[self.current] as char;
    }

    pub fn match_(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.source.as_bytes()[self.current] as char != expected {
            return false;
        }
        self.current += 1;
        true
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.source.as_bytes()[self.current - 1] as char
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len() - 1
    }

    fn make_token(&self, type_: TokenType) -> Token {
        Token {
            type_: type_,
            start: self.start,
            length: self.current - self.start,
            line: self.line,
            message: self.sub_current(),
        }
    }

    fn error_token(&self, message: &str) -> Token {
        Token {
            type_: TokenType::Error,
            start: 0,
            length: message.len(),
            line: self.line,
            message: message.into(),
        }
    }

    fn sub_current(&self) -> String {
        String::from_utf8((self.source.as_bytes()[self.start..self.start + self.current]).to_vec()).unwrap()
    }
}

fn is_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}

fn is_alpha(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum TokenType {
    LeftParen = 0,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Identifier,
    String,
    Number,
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    Error,
    Eof,
}

#[derive(Clone)]
pub struct Token {
    pub type_: TokenType,
    pub start: usize,
    pub length: usize,
    pub line: usize,
    pub message: String,
}

impl Token {
    pub fn default() -> Token {
        Token {
            type_: TokenType::Eof,
            start: 0,
            length: 0,
            line: 0,
            message: String::new(),
        }
    }
}
