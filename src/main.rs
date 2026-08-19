use anyhow::{Result, anyhow};
#[cfg(feature = "timings")]
use std::time::Instant;
use std::{
    env,
    io::{self, Stdout, Write},
};

use crate::interpreter::{Interpreter, LoxObject};

#[macro_use]
mod define_ast;
mod ast;
mod interpreter;
mod parser;
mod scanner;

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

    let mut interpreter = Interpreter::new(std::io::stdout());

    run(&input, &mut interpreter, false)?;

    Ok(())
}

fn run_prompt() -> Result<()> {
    let mut buffer = String::new();
    let stdin = io::stdin();
    println!("Welcome to rlox! Press Ctrl-D to exit.");
    let mut interpreter = Interpreter::new(std::io::stdout());
    loop {
        print!("> ");
        std::io::stdout().flush()?;
        // Check for ctrl-d
        if stdin.read_line(&mut buffer)? == 0 {
            println!("");
            break Ok(());
        }

        match run(&buffer, &mut interpreter, true) {
            Ok(Some(value)) => println!("{}", value.to_string::<Stdout>()), // Show evaluated expressions
            _ => {} // Errors should have already been printed.
        }
        buffer.clear(); // clear contents but keep allocated size
    }
}

fn run<W: Write>(source: &str, interpreter: &mut Interpreter<W>, is_repl: bool) -> Result<Option<LoxObject>> {
    #[cfg(feature = "timings")]
    let timer = Instant::now();

    let tokens = scanner::scan_tokens(source)?;

    #[cfg(feature = "timings")]
    let timer = {
        println!("Scanning: {:?}", timer.elapsed());
        Instant::now()
    };

    if is_repl {
        let parse = parser::parse_for_repl(tokens, std::io::stderr())?;

        #[cfg(feature = "timings")]
        let timer = {
            println!("Parsing: {:?}", timer.elapsed());
            Instant::now()
        };

        match parse {
            parser::ReplParseOutput::Statements(statements) => {
                interpreter.interpret(&statements)?;

                #[cfg(feature = "timings")]
                println!("Parsing: {:?}", timer.elapsed());

                Ok(None)
            }
            parser::ReplParseOutput::Expr(expr) => {
                let evaluated = interpreter.evaluate(&expr);

                #[cfg(feature = "timings")]
                println!("Parsing: {:?}", timer.elapsed());

                match evaluated {
                    Ok(value) => Ok(Some(value)),
                    Err(_) => Err(anyhow!("Error evaluating expression.")),
                }
            }
        }
    } else {
        let statements = parser::parse(tokens, std::io::stderr())?;

        #[cfg(feature = "timings")]
        let timer = {
            println!("Parsing: {:?}", timer.elapsed());
            Instant::now()
        };

        interpreter.interpret(&statements)?;

        #[cfg(feature = "timings")]
        println!("Parsing: {:?}", timer.elapsed());

        Ok(None)
    }
}
