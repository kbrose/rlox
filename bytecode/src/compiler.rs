use crate::scanner::{Scanner, TokenType};

pub(crate) fn compile(source: &str) {
    let mut scanner = Scanner::new(source, std::io::stderr());

    let mut line = 0;
    loop {
        let token = scanner.scan_token().unwrap();
        if token.line() != line {
            print!("{: >4} ", token.line());
            line = token.line();
        } else {
            print!("   | ");
        }

        println!(
            "{: >2} '{}'",
            token.token_type().to_debug_num(),
            token.lexeme()
        );

        if token.token_type() == TokenType::Eof {
            break;
        }
    }
}
