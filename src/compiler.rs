use crate::scanner::{Scanner, Token, TokenType};

pub struct Compiler {
    pub parser: Parser,
    scanner: Scanner,
}

pub struct Parser {
    current: Token,
    previous: Token,
    pub had_error: bool,
    pub panic_mode: bool,
}

impl Parser {
    fn new() -> Parser {
        Parser {
            current: Token::default(),
            previous: Token::default(),
            had_error: false,
            panic_mode: false,
        }
    }
}

impl Compiler {
    pub fn new(scanner: Scanner) -> Compiler {
        Compiler {
            parser: Parser::new(),
            scanner: scanner,
        }
    }

    pub fn advance(&mut self) {
        self.parser.previous = self.parser.current.clone();

        loop {
            self.parser.current = self.scanner.scan_token();
            if let TokenType::Error(ref s) = self.parser.current.type_ {
                let s = s.clone();
                self.error_at_current(&s);
            } else {
                break;
            }
        }
    }

    pub fn consume(&mut self, type_: TokenType, message: &str) {
        if self.parser.current.type_ == type_ {
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
