use std::io::Write;

use crate::{
    bytecode::{Chunk, OpCode},
    scanner::{Scanner, Token, TokenType},
    value::Value,
    value::heap::Heap,
};

struct Parser<'a, W: Write> {
    current: Token<'a>,
    previous: Token<'a>,
    scanner: &'a mut Scanner<'a, W>,
    chunk: &'a mut Chunk,
    had_error: bool,
    panic_mode: bool,
    error_writer: W,
    heap: &'a mut Heap,
}

impl<'a, W: Write> Parser<'a, W> {
    fn new(
        scanner: &'a mut Scanner<'a, W>,
        chunk: &'a mut Chunk,
        error_writer: W,
        heap: &'a mut Heap,
    ) -> Self {
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
            heap,
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
                        // We set panic mode here to suppress scanner errors. We've already
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

    fn end_compiler<W2: Write>(&mut self, dis_writer: &mut W2) {
        #[cfg(feature = "print_code")]
        {
            if !self.had_error {
                use crate::debug::Disassembler;

                let mut disassembler = Disassembler::new();
                disassembler.disassemble_chunk(&self.chunk, "code", self.heap, dis_writer);
            }
        }
        self.emit_return();
    }

    // Vaughan Pratt’s "top-down operator precedence parsing"

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment.to_u8());
    }

    fn false_(&mut self) {
        self.emit_op(OpCode::False);
    }

    fn true_(&mut self) {
        self.emit_op(OpCode::True);
    }

    fn nil(&mut self) {
        self.emit_op(OpCode::Nil);
    }

    fn number(&mut self) {
        let lexeme = self.previous.lexeme();
        let value = Value::new_number(lexeme.parse().expect(&format!(
            "Scanning went awry: failed to parse number lexeme '{}' as f64",
            lexeme
        )));

        self.chunk
            .write_constant(value, self.previous.line() as usize)
    }

    fn string(&mut self) {
        let lexeme = self.previous.lexeme();
        self.chunk.write_constant(
            Value::new_string(lexeme[1..lexeme.len() - 1].to_string(), self.heap),
            self.previous.line() as usize,
        );
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
            TokenType::Bang => {
                self.emit_op(OpCode::Not);
            }
            _ => unreachable!(),
        }
    }

    // TODO: This is good for reducing boilerplate, but I think it's better
    // (more optimal, more type safe) to have one function per operator that
    // hard codes its precedence and token, rather than looking it up dynamically
    // in the previous token.
    fn binary(&mut self) {
        let operator_type = self.previous.token_type();
        let rule = ParseRule::<'a, W>::from_token_type(operator_type);
        self.parse_precedence(rule.infix_and_precedence.unwrap().1.to_u8() + 1);

        match operator_type {
            TokenType::BangEqual => {
                self.emit_op(OpCode::Equal);
                self.emit_op(OpCode::Not);
            }
            TokenType::EqualEqual => {
                self.emit_op(OpCode::Equal);
            }
            TokenType::Greater => {
                self.emit_op(OpCode::Greater);
            }
            TokenType::GreaterEqual => {
                self.emit_op(OpCode::Less);
                self.emit_op(OpCode::Not);
            }
            TokenType::Less => {
                self.emit_op(OpCode::Less);
            }
            TokenType::LessEqual => {
                self.emit_op(OpCode::Greater);
                self.emit_op(OpCode::Not);
            }
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
                match ParseRule::<'a, W>::from_token_type(self.current.token_type())
                    .infix_and_precedence
                {
                    Some((infix_fn, rule_precedence)) => {
                        if precedence > rule_precedence.to_u8() {
                            break;
                        }

                        self.advance();

                        infix_fn(self);
                    }
                    None => {
                        break;
                    }
                }
            }
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
    infix_and_precedence: Option<(ParseFn<'a, W>, Precedence)>,
}

impl<'a, W: Write> ParseRule<'a, W> {
    fn new(
        prefix: Option<ParseFn<'a, W>>,
        infix_and_precedence: Option<(ParseFn<'a, W>, Precedence)>,
    ) -> Self {
        Self {
            prefix,
            infix_and_precedence,
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
            // Token Type                             prefix                  infix_and_precedence
            TokenType::LeftParen    => ParseRule::new(Some(Parser::grouping), None                                           ),
            TokenType::RightParen   => ParseRule::new(None,                   None                                           ),
            TokenType::LeftBrace    => ParseRule::new(None,                   None                                           ),
            TokenType::RightBrace   => ParseRule::new(None,                   None                                           ),
            TokenType::Comma        => ParseRule::new(None,                   None                                           ),
            TokenType::Dot          => ParseRule::new(None,                   None                                           ),
            TokenType::Minus        => ParseRule::new(Some(Parser::unary),    Some((Parser::binary, Precedence::Term))       ),
            TokenType::Plus         => ParseRule::new(None,                   Some((Parser::binary, Precedence::Term))       ),
            TokenType::Semicolon    => ParseRule::new(None,                   None                                           ),
            TokenType::Slash        => ParseRule::new(None,                   Some((Parser::binary, Precedence::Factor))     ),
            TokenType::Star         => ParseRule::new(None,                   Some((Parser::binary, Precedence::Factor))     ),
            TokenType::Bang         => ParseRule::new(Some(Parser::unary),    None                                           ),
            TokenType::BangEqual    => ParseRule::new(None,                   Some((Parser::binary, Precedence::Equality))   ),
            TokenType::Equal        => ParseRule::new(None,                   None                                           ),
            TokenType::EqualEqual   => ParseRule::new(None,                   Some((Parser::binary, Precedence::Comparison)) ),
            TokenType::Greater      => ParseRule::new(None,                   Some((Parser::binary, Precedence::Comparison)) ),
            TokenType::GreaterEqual => ParseRule::new(None,                   Some((Parser::binary, Precedence::Comparison)) ),
            TokenType::Less         => ParseRule::new(None,                   Some((Parser::binary, Precedence::Comparison)) ),
            TokenType::LessEqual    => ParseRule::new(None,                   Some((Parser::binary, Precedence::Comparison)) ),
            TokenType::Identifier   => ParseRule::new(None,                   None                                           ),
            TokenType::String       => ParseRule::new(Some(Parser::string),   None                                           ),
            TokenType::Number       => ParseRule::new(Some(Parser::number),   None                                           ),
            TokenType::And          => ParseRule::new(None,                   None                                           ),
            TokenType::Class        => ParseRule::new(None,                   None                                           ),
            TokenType::Else         => ParseRule::new(None,                   None                                           ),
            TokenType::False        => ParseRule::new(Some(Parser::false_),   None                                           ),
            TokenType::For          => ParseRule::new(None,                   None                                           ),
            TokenType::Fun          => ParseRule::new(None,                   None                                           ),
            TokenType::If           => ParseRule::new(None,                   None                                           ),
            TokenType::Nil          => ParseRule::new(Some(Parser::nil),      None                                           ),
            TokenType::Or           => ParseRule::new(None,                   None                                           ),
            TokenType::Print        => ParseRule::new(None,                   None                                           ),
            TokenType::Return       => ParseRule::new(None,                   None                                           ),
            TokenType::Super        => ParseRule::new(None,                   None                                           ),
            TokenType::This         => ParseRule::new(None,                   None                                           ),
            TokenType::True         => ParseRule::new(Some(Parser::true_),    None                                           ),
            TokenType::Var          => ParseRule::new(None,                   None                                           ),
            TokenType::While        => ParseRule::new(None,                   None                                           ),
            TokenType::Eof          => ParseRule::new(None,                   None                                           ),
        }
    }
}

pub(crate) fn compile<W: Write>(
    source: &str,
    chunk: &mut Chunk,
    obj_heap: &mut Heap,
    dis_writer: &mut W,
) -> Result<(), ()> {
    let mut scanner = Scanner::new(source, std::io::stderr());
    let mut parser = Parser::new(&mut scanner, chunk, std::io::stderr(), obj_heap);
    parser.expression();
    parser.consume(TokenType::Eof, "Expect end of expression.");
    parser.end_compiler(dis_writer);

    if parser.had_error { Err(()) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile_fresh(source: &str) -> Result<Chunk, ()> {
        let mut chunk = Chunk::new();
        let mut obj_heap = Heap::new();
        let mut sink = std::io::sink();
        compile(source, &mut chunk, &mut obj_heap, &mut sink)?;
        Ok(chunk)
    }

    #[test]
    fn test_simple() {
        let chunk = compile_fresh("1").expect("Compile error");

        // This code should get parsed into 3 bytes:
        // 1. CONSTANT
        // 2. index into constants table (should be 0 since this is the one and only constant)
        // 3. Return

        assert!(chunk.code.len() == 3);

        assert_eq!(OpCode::from_byte(chunk.code[0]), Some(OpCode::Constant));
        assert_eq!(chunk.code[1], 0);
        assert_eq!(OpCode::from_byte(chunk.code[2]), Some(OpCode::Return));
    }

    #[test]
    fn test_complex() {
        let chunk = compile_fresh("1 * (2 + 3)").expect("Compile error");

        // This code should get parsed into 9 bytes:
        // 1. CONSTANT
        // 2. index into constants table (0)
        // 3. CONSTANT
        // 4. index into constants table (1)
        // 5. CONSTANT
        // 6. index into constants table (2)
        // 7. Add
        // 8. Multiply
        // 9. Return

        assert!(chunk.code.len() == 9);

        assert_eq!(OpCode::from_byte(chunk.code[0]), Some(OpCode::Constant));
        assert_eq!(chunk.code[1], 0);
        assert_eq!(chunk.constant_at_index(0), &Value::Number(1.0));

        assert_eq!(OpCode::from_byte(chunk.code[2]), Some(OpCode::Constant));
        assert_eq!(chunk.code[3], 1);
        assert_eq!(chunk.constant_at_index(1), &Value::Number(2.0));

        assert_eq!(OpCode::from_byte(chunk.code[4]), Some(OpCode::Constant));
        assert_eq!(chunk.code[5], 2);
        assert_eq!(chunk.constant_at_index(2), &Value::Number(3.0));

        assert_eq!(OpCode::from_byte(chunk.code[6]), Some(OpCode::Add));
        assert_eq!(OpCode::from_byte(chunk.code[7]), Some(OpCode::Multiply));
        assert_eq!(OpCode::from_byte(chunk.code[8]), Some(OpCode::Return));
    }
}
