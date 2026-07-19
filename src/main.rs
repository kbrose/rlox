use anyhow::{Context, Result, anyhow};
use std::{
    env,
    io::{self, Write as _},
};
use thiserror::Error;

mod scanner;

#[derive(Error, Debug)]
pub enum FormatError {
    #[error("Invalid header (expected {expected:?}, got {found:?})")]
    InvalidHeader {
        expected: String,
        found: String,
    },
    // #[error("Missing attribute: {0}")]
    // MissingAttribute(String),
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 1 {
        Err(anyhow!("Usage: rlox [script]"))
    } else if args.len() == 1 {
        run_file(&args[0])
    } else {
        run_prompt()
    }
}

fn run_file(path: &String) -> Result<()> {
    let input = std::fs::read_to_string(path).with_context(|| format!("Failed to read file {}", path))?;

    run(&input)?;

    Ok(())
}

fn run_prompt() -> Result<()> {
    let mut buffer = String::new();
    let stdin = io::stdin(); // We get `Stdin` here.
    loop {
        print!("> ");
        std::io::stdout().flush()?;
        // Check for ctrl-d
        if stdin.read_line(&mut buffer)? == 0 {
            println!("\nFin.");
            break Ok(());
        }
        match run(&buffer) {
            Ok(()) => {}
            Err(e) => {
                println!("{e:?}");
            }
        }
        buffer.clear(); // clear contents but keep allocated size
    }
}

fn run(source: &str) -> Result<()> {
    let tokens = scanner::scan_tokens(source)?;

    for token in tokens {
        println!("{token:?}");
    }
    Ok(())
}
