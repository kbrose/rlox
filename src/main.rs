use anyhow::{Context, Result, anyhow};
use std::{
    env,
    io::{self, Write as _},
};

#[macro_use]
mod define_ast;
mod ast_printer;
mod expr;
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
        let _ = run(&buffer); // Errors should have already been printed.
        buffer.clear(); // clear contents but keep allocated size
    }
}

fn run(source: &str) -> Result<()> {
    let tokens = scanner::scan_tokens(source)?;
    let expression = parser::parse(tokens)?;

    println!("{}", ast_printer::pretty_print_expr(&expression));

    // for token in tokens {
    //     println!("{token:?}");
    // }
    Ok(())
}
