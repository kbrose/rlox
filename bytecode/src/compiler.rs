use std::io::Write;

use crate::{
    bytecode::{Chunk, OpCode},
    scanner::{Scanner, Token, TokenType},
    value::Value,
};

struct Parser<'a, W: Write> {
    current: Token<'a>,
    previous: Token<'a>,
    scanner: &'a mut Scanner<'a, W>,
    chunk: &'a mut Chunk,
    had_error: bool,
    panic_mode: bool,
    error_writer: W,
}

impl<'a, W: Write> Parser<'a, W> {
    fn new(scanner: &'a mut Scanner<'a, W>, chunk: &'a mut Chunk, error_writer: W) -> Self {
        // Prime the pump.
        let mut errored = false;
        let current = loop {
            match scanner.scan_token() {
                Ok(token) => {
                    break token;
                }
                Err(()) => {
                    errored = true;
                }
            }
        };

        Self {
            current,
            previous: current, // TODO: We cheat here and just ALSO set previous to the first token.
            scanner,
            chunk,
            had_error: errored,
            panic_mode: errored,
            error_writer,
        }
    }

    // Parsing methods

    fn advance(&mut self) {
        self.previous = self.current;

        self.current = {
            loop {
                match self.scanner.scan_token() {
                    Ok(token) => {
                        break token;
                    }
                    Err(()) => {
                        // We set panic mode here to suppress the message. We've already
                        // reported scanning errors, which clox stuffs into this function
                        // instead. I like my separation of concerns, so I'll leave mine
                        // where they are.
                        self.panic_mode = true;
                        self.error_at_current("");
                    }
                }
            }
        };
    }

    fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.current.token_type() == token_type {
            self.advance();
        } else {
            self.error_at_current(message);
        }
    }

    fn end_compiler(&mut self) {
        #[cfg(feature = "print_code")]
        {
            if !self.had_error {
                use crate::debug::Disassembler;

                let mut disassembler = Disassembler::new(std::io::stdout());
                disassembler.disassemble_chunk(&self.chunk, "code");
            }
        }
        self.emit_return();
    }

    // Vaughan Pratt’s "top-down operator precedence parsing"

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment.to_u8());
    }

    fn number(&mut self) {
        let lexeme = self.previous.lexeme();
        let value = Value::new(lexeme.parse().expect(&format!(
            "Scanning went awry: failed to parse number lexeme '{}' as f64",
            lexeme
        )));

        self.chunk
            .write_constant(value, self.previous.line() as usize)
    }

    fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn unary(&mut self) {
        let operator_type = self.previous.token_type();

        self.parse_precedence(Precedence::Unary.to_u8());

        match operator_type {
            TokenType::Minus => {
                self.emit_op(OpCode::Negate);
            }
            _ => unreachable!(),
        }
    }

    fn binary(&mut self) {
        let operator_type = self.previous.token_type();
        let rule = ParseRule::<'a, W>::from_token_type(operator_type);
        self.parse_precedence(rule.precedence.to_u8() + 1);

        match operator_type {
            TokenType::Plus => self.emit_op(OpCode::Add),
            TokenType::Minus => self.emit_op(OpCode::Subtract),
            TokenType::Star => self.emit_op(OpCode::Multiply),
            TokenType::Slash => self.emit_op(OpCode::Divide),
            _ => unreachable!(),
        }
    }

    fn parse_precedence(&mut self, precedence: u8) {
        self.advance();
        if let Some(prefix_fn) = ParseRule::from_token_type(self.previous.token_type()).prefix {
            prefix_fn(self);

            loop {
                let rule = ParseRule::<'a, W>::from_token_type(self.current.token_type());
                if precedence > rule.precedence.to_u8() {
                    break;
                }

                self.advance();

                let infix_fn = rule
                    .infix
                    .expect("ParseRule table incorrect: infix should never be None here");
                infix_fn(self);
            }

            // while precedence
            //     <= ParseRule::<'a, W>::from_token_type(self.current.token_type())
            //         .precedence
            //         .to_u8()
            // {
            //     self.advance();
            //     let infix_fn = ParseRule::from_token_type(self.previous.token_type())
            //         .infix
            //         .expect("ParseRule table incorrect: infix should never be None here");
            //     infix_fn(self);
            // }
        } else {
            self.error("Expect expression.");
        }
    }

    // Code generation utils

    #[inline]
    fn emit_return(&mut self) {
        self.emit_op(OpCode::Return);
    }

    #[inline]
    fn emit_op(&mut self, op: OpCode) {
        self.emit_byte(op.to_byte())
    }

    #[inline]
    fn emit_byte(&mut self, byte: u8) {
        self.chunk.write_byte(byte, self.previous.line() as usize);
    }

    // #[inline]
    // fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
    //     self.emit_byte(byte1);
    //     self.emit_byte(byte2);
    // }

    // Error reporting & handling

    fn error_at_current(&mut self, message: &str) {
        self.error_at(self.current, message);
    }

    fn error(&mut self, message: &str) {
        self.error_at(self.previous, message);
    }

    fn error_at(&mut self, token: Token<'a>, message: &str) {
        self.had_error = true;
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;

        let expect_msg = "Error writing error to stream.";

        write!(self.error_writer, "[Parse Error line {}]", token.line()).expect(expect_msg);

        match token.token_type() {
            TokenType::Eof => {
                write!(self.error_writer, " At end").expect(expect_msg);
            }
            _ => {
                write!(self.error_writer, " At {}", token.lexeme()).expect(expect_msg);
            }
        }

        writeln!(self.error_writer, ": {}", message).expect(expect_msg);
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[allow(unused)]
enum Precedence {
    None,
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

impl Precedence {
    #[inline]
    pub(crate) fn to_u8(self: Self) -> u8 {
        // SAFETY: All values of Precedence are valid u8 because Precedence is repr(u8).
        unsafe { std::mem::transmute(self) }
    }
}

type ParseFn<'a, W> = fn(&mut Parser<'a, W>);

struct ParseRule<'a, W: Write> {
    prefix: Option<ParseFn<'a, W>>,
    infix: Option<ParseFn<'a, W>>,
    precedence: Precedence,
}

impl<'a, W: Write> ParseRule<'a, W> {
    fn new(
        prefix: Option<ParseFn<'a, W>>,
        infix: Option<ParseFn<'a, W>>,
        precedence: Precedence,
    ) -> Self {
        Self {
            prefix,
            infix,
            precedence,
        }
    }

    #[rustfmt::skip]
    fn from_token_type(token_type: TokenType) -> Self {
        // Looking online, there seems to be contention around whether or not look up tables
        // are actually faster than a good ole match statement. For simplicity & compiler
        // safety, I am going with a match statement, but I'd like to benchmark this once
        // I have a full end to end pipeline working.
        //
        // I will say, rust makes LUTs much harder to code than match statements, even
        // for data-less enums.
        //
        // TODO: Benchmark look up table vs. match statement
        // TODO: I think it's a requirement that when infix is specified, precedence is not None.
        //       Would it be more type safe to combine those and guarantee a non-None precedence?
        //       I think this is related to the fact that I have to .expect(...) the infix_fn
        //       inside of parse_precedence.
        match token_type {
            // Token Type                             prefix                  infix                 precedence
            TokenType::LeftParen    => ParseRule::new(Some(Parser::grouping), None,                 Precedence::None),
            TokenType::RightParen   => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::LeftBrace    => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::RightBrace   => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Comma        => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Dot          => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Minus        => ParseRule::new(Some(Parser::unary),    Some(Parser::binary), Precedence::Term),
            TokenType::Plus         => ParseRule::new(None,                   Some(Parser::binary), Precedence::Term),
            TokenType::Semicolon    => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Slash        => ParseRule::new(None,                   Some(Parser::binary), Precedence::Factor),
            TokenType::Star         => ParseRule::new(None,                   Some(Parser::binary), Precedence::Factor),
            TokenType::Bang         => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::BangEqual    => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Equal        => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::EqualEqual   => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Greater      => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::GreaterEqual => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Less         => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::LessEqual    => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Identifier   => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::String       => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Number       => ParseRule::new(Some(Parser::number),   None,                 Precedence::None),
            TokenType::And          => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Class        => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Else         => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::False        => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::For          => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Fun          => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::If           => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Nil          => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Or           => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Print        => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Return       => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Super        => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::This         => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::True         => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Var          => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::While        => ParseRule::new(None,                   None,                 Precedence::None),
            TokenType::Eof          => ParseRule::new(None,                   None,                 Precedence::None),
        }
    }
}

pub(crate) fn compile(source: &str, chunk: &mut Chunk) -> Result<(), ()> {
    let mut scanner = Scanner::new(source, std::io::stderr());
    let mut parser = Parser::new(&mut scanner, chunk, std::io::stderr());
    parser.expression();
    parser.consume(TokenType::Eof, "Expect end of expression.");
    parser.end_compiler();

    if parser.had_error { Err(()) } else { Ok(()) }
}
