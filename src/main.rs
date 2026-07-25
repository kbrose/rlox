use anyhow::{Result, anyhow};
#[cfg(feature = "timings")]
use std::time::Instant;
use std::{
    env,
    io::{self, Write as _},
};

use crate::interpreter::Interpreter;

#[macro_use]
mod define_ast;
mod ast_printer;
mod environment;
mod expr;
mod interpreter;
mod parser;
mod scanner;
mod stmt;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 1 {
        Err(anyhow!("Usage: rlox [script]"))
    } else if args.len() == 1 {
        run_file(&args[0]).map_err(|_| anyhow!("See above."))
    } else {
        run_prompt()
    }
}

fn run_file(path: &String) -> Result<()> {
    let input = std::fs::read_to_string(path).map_err(|_| {
        eprintln!("Failed to read file {}", path);
        anyhow!("")
    })?;

    let mut interpreter = Interpreter::new();

    run(&input, &mut interpreter)?;

    Ok(())
}

fn run_prompt() -> Result<()> {
    let mut buffer = String::new();
    let stdin = io::stdin();
    println!("Welcome to rlox! Press Ctrl-D to exit.");
    let mut interpreter = Interpreter::new();
    loop {
        print!("> ");
        std::io::stdout().flush()?;
        // Check for ctrl-d
        if stdin.read_line(&mut buffer)? == 0 {
            println!("");
            break Ok(());
        }
        let _ = run(&buffer, &mut interpreter); // Errors should have already been printed.
        buffer.clear(); // clear contents but keep allocated size
    }
}

fn run(source: &str, interpreter: &mut Interpreter) -> Result<()> {
    #[cfg(feature = "timings")]
    let scanning_timer = Instant::now();

    let tokens = scanner::scan_tokens(source)?;

    #[cfg(feature = "timings")]
    let parsing_timer = {
        println!("Scanning: {:?}", scanning_timer.elapsed());
        Instant::now()
    };

    let statements = parser::parse(tokens)?;

    #[cfg(feature = "timings")]
    println!("Parsing : {:?}", parsing_timer.elapsed());

    interpreter.interpret(statements)?;

    Ok(())
}
