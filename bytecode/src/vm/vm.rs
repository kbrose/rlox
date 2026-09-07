use std::io::Write;

use crate::{
    bytecode::{Chunk, OpCode},
    compiler::compile,
    value::Value,
    vm::stack::Stack,
};

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

const STACK_MAX: usize = 256;

pub(crate) struct VirtualMachine<W: Write> {
    stack: Stack,
    error_writer: W,
}

impl<W: Write> VirtualMachine<W> {
    pub(crate) fn new(error_writer: W) -> Self {
        Self {
            stack: Stack::new(STACK_MAX),
            error_writer,
        }
    }

    pub(crate) fn interpret(&mut self, source: String) -> InterpretResult {
        let mut chunk = Chunk::new();
        match compile(&source, &mut chunk) {
            Ok(()) => self.run(chunk),
            Err(()) => InterpretResult::InterpretCompileError,
        }
    }

    #[inline]
    fn binary_op_num2num(&mut self, f: impl Fn(f64, f64) -> f64) -> Result<(), ()> {
        let b = self.stack.pop().as_number()?;
        self.stack.apply_to_top_num2num(|a| f(a, b))?;
        Ok(())
    }

    #[inline]
    fn binary_op_num2bool(&mut self, f: impl Fn(f64, f64) -> bool) -> Result<(), ()> {
        let b = self.stack.pop().as_number()?;
        self.stack.apply_to_top_num2bool(|a| f(a, b))?;
        Ok(())
    }

    fn run(&mut self, chunk: Chunk) -> InterpretResult {
        let mut ip = 0;
        if chunk.code.len() == 0 {
            return InterpretResult::InterpretOk;
        }

        #[cfg(feature = "debug_trace_execution")]
        let mut disassembler = {
            use crate::debug::Disassembler;

            Disassembler::new(std::io::stdout())
        };

        loop {
            #[cfg(feature = "debug_trace_execution")]
            {
                print!("          ");
                for value in self.stack.stack().iter() {
                    print!("[ ");
                    value.print();
                    print!(" ]");
                }
                println!();
                disassembler.disassemble_instruction(&chunk, ip);
            }

            let op = unsafe { chunk.op_unchecked_at_index_unchecked(post_increment(&mut ip)) };
            match op {
                OpCode::ConstantLong => {
                    let constant_idx = (chunk.byte_at_index(post_increment(&mut ip)) as usize)
                        | ((chunk.byte_at_index(post_increment(&mut ip)) as usize) << 8)
                        | ((chunk.byte_at_index(post_increment(&mut ip)) as usize) << 16);

                    let constant = unsafe { chunk.constant_at_index_unchecked(constant_idx) };
                    self.stack.push(*constant);
                }
                OpCode::Constant => {
                    let constant = unsafe {
                        chunk.constant_at_index_unchecked(
                            chunk.byte_at_index(post_increment(&mut ip)) as usize,
                        )
                    };
                    self.stack.push(*constant);
                }
                OpCode::Nil => self.stack.push(Value::Nil),
                OpCode::True => self.stack.push(Value::Bool(true)),
                OpCode::False => self.stack.push(Value::Bool(false)),
                OpCode::Equal => {
                    let b = self.stack.pop();
                    self.stack.apply_to_top(|a| a.is_equal(&b));
                }
                OpCode::Greater => {
                    // TODO: Refactor into this vm function returning a Result<> and updating
                    // the binary_op methods to also use the runtime_error() and return a result
                    // so we can just use ? short circuiting. (I wasn't sure if that would be valid
                    // based on the development of the clox version, but it seems like it will be.)
                    if self.binary_op_num2bool(|a, b| a > b).is_err() {
                        return self.runtime_error(ip, &chunk, "Operands must be numbers.");
                    }
                }
                OpCode::Less => {
                    if self.binary_op_num2bool(|a, b| a < b).is_err() {
                        return self.runtime_error(ip, &chunk, "Operands must be numbers.");
                    }
                }
                OpCode::Add => {
                    if self.binary_op_num2num(|a, b| a + b).is_err() {
                        return self.runtime_error(ip, &chunk, "Operands must be numbers.");
                    }
                }
                OpCode::Subtract => {
                    if self.binary_op_num2num(|a, b| a - b).is_err() {
                        return self.runtime_error(ip, &chunk, "Operands must be numbers.");
                    }
                }
                OpCode::Multiply => {
                    if self.binary_op_num2num(|a, b| a * b).is_err() {
                        return self.runtime_error(ip, &chunk, "Operands must be numbers.");
                    }
                }
                OpCode::Divide => {
                    if self.binary_op_num2num(|a, b| a / b).is_err() {
                        return self.runtime_error(ip, &chunk, "Operands must be numbers.");
                    }
                }
                OpCode::Not => self.stack.apply_to_top(|value| value.is_falsey()),
                OpCode::Negate => {
                    if self.stack.apply_to_top_num2num(|value| -value).is_err() {
                        return self.runtime_error(ip, &chunk, "Operand must be a number.");
                    }
                }
                OpCode::Return => {
                    #[allow(unused)]
                    let out = self.stack.pop();
                    #[cfg(feature = "debug_trace_execution")]
                    {
                        out.print();
                        println!();
                    }
                    break InterpretResult::InterpretOk;
                }
            }
        }
    }

    #[must_use]
    fn runtime_error(&mut self, ip: usize, chunk: &Chunk, message: &str) -> InterpretResult {
        writeln!(self.error_writer, "{}", message).expect("Error writing error.");

        let line = chunk.line_at_index(ip);

        writeln!(self.error_writer, "[line {line}] in script").expect("Error writing error.");

        self.stack.reset();

        InterpretResult::InterpretRuntimeError
    }
}

/// An implementation of C's `x++`
fn post_increment(x: &mut usize) -> usize {
    let out = *x;
    *x += 1;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addition() {
        assert_eq!(1 + 1, 2);
    }
}
