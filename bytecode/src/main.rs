mod bytecode;
mod compiler;
mod debug;
mod scanner;
mod table;
mod value;
mod vm;

use std::{
    env,
    io::{self, Write},
    process::ExitCode,
};

use vm::*;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.len() == 1 {
        run_file(&args[0])
    } else if args.len() == 0 {
        repl()
    } else {
        writeln!(io::stderr(), "Usage: clox [path]\n").expect("Error writing to stderr");
        ExitCode::FAILURE
    }
}

fn run_file(path: &str) -> ExitCode {
    let error_msg = format!("Failed to read file {}", path);
    let input = std::fs::read_to_string(path).expect(&error_msg);

    let mut vm = VirtualMachine::new(std::io::stderr());

    match vm.interpret(input, std::io::stdout()) {
        InterpretResult::InterpretOk(_) => ExitCode::SUCCESS,
        InterpretResult::InterpretCompileError | InterpretResult::InterpretRuntimeError => {
            ExitCode::FAILURE
        }
    }
}

fn repl() -> ExitCode {
    let stdin = io::stdin();
    println!("Welcome to rlox! Press Ctrl-D to exit.");

    let mut vm = VirtualMachine::new(std::io::stderr());

    loop {
        let mut buffer = String::new();

        print!("> ");
        std::io::stdout().flush().expect("Error flushing stdout.");
        // Check for ctrl-d
        if stdin
            .read_line(&mut buffer)
            .expect("Error reading line from stdin")
            == 0
        {
            println!("");
            break ExitCode::SUCCESS;
        }

        vm.interpret(buffer, std::io::stdout());
    }
}
