use std::io::Write;

use crate::{
    bytecode::{Chunk, OpCode},
    compiler::compile,
    value::{ObjType, Value, heap::Heap},
    vm::stack::Stack,
};

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum InterpretResult {
    InterpretOk(Option<Value>),
    InterpretCompileError,
    InterpretRuntimeError,
}

const STACK_MAX: usize = 256;

pub(crate) struct VirtualMachine<W: Write> {
    /// Owns all heap-allocated objects.
    heap: Heap,
    // strings: HashSet<String>,
    error_writer: W,
}

impl<W: Write> VirtualMachine<W> {
    pub(crate) fn new(error_writer: W) -> Self {
        Self {
            heap: Heap::new(),
            error_writer,
        }
    }

    pub(crate) fn interpret<W2: Write>(
        &mut self,
        source: String,
        mut dis_writer: W2,
    ) -> InterpretResult {
        let mut chunk = Chunk::new();
        let out = compile(&source, &mut chunk, &mut self.heap, &mut dis_writer);
        match out {
            Ok(()) => self.run(chunk, &mut dis_writer),
            Err(()) => InterpretResult::InterpretCompileError,
        }
    }

    #[inline]
    fn binary_op_num2num(
        &mut self,
        stack: &mut Stack,
        f: impl Fn(f64, f64) -> f64,
    ) -> Result<(), ()> {
        let b = stack.pop().as_number()?;
        stack.apply_to_top_num2num(|a| f(a, b))?;
        Ok(())
    }

    #[inline]
    fn binary_op_num2bool(
        &mut self,
        stack: &mut Stack,
        f: impl Fn(f64, f64) -> bool,
    ) -> Result<(), ()> {
        let b = stack.pop().as_number()?;
        stack.apply_to_top_num2bool(|a| f(a, b))?;
        Ok(())
    }

    fn run<W2: Write>(&mut self, chunk: Chunk, mut debug_writer: &mut W2) -> InterpretResult {
        let mut stack: Stack = Stack::new(STACK_MAX);
        let mut ip = 0;
        if chunk.code.len() == 0 {
            return InterpretResult::InterpretOk(None);
        }

        #[cfg(feature = "debug_trace_execution")]
        let mut disassembler = {
            use crate::debug::Disassembler;

            Disassembler::new()
        };

        loop {
            #[cfg(feature = "debug_trace_execution")]
            {
                write!(debug_writer, "          ").expect("Error writing stack debug strings.");
                for value in stack.stack().iter() {
                    write!(debug_writer, "[ ").expect("Error writing stack debug strings.");
                    value.debug_print(&self.heap, &mut debug_writer);
                    write!(debug_writer, " ]").expect("Error writing stack debug strings.");
                }
                writeln!(debug_writer).expect("Error writing stack debug strings.");
                disassembler.disassemble_instruction(&chunk, ip, &self.heap, debug_writer);
            }

            let op = unsafe { chunk.op_unchecked_at_index_unchecked(post_increment(&mut ip)) };
            match op {
                OpCode::ConstantLong => {
                    let constant_idx = (chunk.byte_at_index(post_increment(&mut ip)) as usize)
                        | ((chunk.byte_at_index(post_increment(&mut ip)) as usize) << 8)
                        | ((chunk.byte_at_index(post_increment(&mut ip)) as usize) << 16);

                    let constant = unsafe { chunk.constant_at_index_unchecked(constant_idx) };
                    stack.push(*constant);
                }
                OpCode::Constant => {
                    let constant = unsafe {
                        chunk.constant_at_index_unchecked(
                            chunk.byte_at_index(post_increment(&mut ip)) as usize,
                        )
                    };
                    stack.push(*constant);
                }
                OpCode::Nil => stack.push(Value::Nil),
                OpCode::True => stack.push(Value::Bool(true)),
                OpCode::False => stack.push(Value::Bool(false)),
                OpCode::Equal => {
                    let b = stack.pop();
                    stack.apply_to_top(|a| a.is_equal(&b, &self.heap));
                }
                OpCode::Greater => {
                    // TODO: Refactor into this vm function returning a Result<> and updating
                    // the binary_op methods to also use the runtime_error() and return a result
                    // so we can just use ? short circuiting. (I wasn't sure if that would be valid
                    // based on the development of the clox version, but it seems like it will be.)
                    if self.binary_op_num2bool(&mut stack, |a, b| a > b).is_err() {
                        return self.runtime_error(ip, &chunk, "Operands must be numbers.");
                    }
                }
                OpCode::Less => {
                    if self.binary_op_num2bool(&mut stack, |a, b| a < b).is_err() {
                        return self.runtime_error(ip, &chunk, "Operands must be numbers.");
                    }
                }
                OpCode::Add => {
                    let a = stack.peek(1);
                    let b = stack.peek(0);
                    match (a, b) {
                        (Value::Number(x), Value::Number(y)) => {
                            stack.pop();
                            stack.pop();
                            stack.push(Value::Number(x + y));
                        }
                        (Value::Obj(idx1), Value::Obj(idx2)) => {
                            let obj1 = unsafe { self.heap.get_unchecked(idx1) };
                            let obj2 = unsafe { self.heap.get_unchecked(idx2) };

                            match (obj1.obj_type(), obj2.obj_type()) {
                                (ObjType::String(obj_str1), ObjType::String(obj_str2)) => {
                                    stack.pop();
                                    stack.pop();
                                    stack.push(Value::new_string(
                                        format!("{}{}", obj_str1.string(), obj_str2.string()),
                                        &mut self.heap,
                                    ))
                                } // _ => {
                                  //     return self.runtime_error(
                                  //         ip,
                                  //         &chunk,
                                  //         "Operands must be both numbers or both strings.",
                                  //     );
                                  // }
                            }
                        }
                        _ => {
                            return self.runtime_error(
                                ip,
                                &chunk,
                                "Operands must be both numbers or both strings.",
                            );
                        }
                    }
                }
                OpCode::Subtract => {
                    if self.binary_op_num2num(&mut stack, |a, b| a - b).is_err() {
                        return self.runtime_error(ip, &chunk, "Operands must be numbers.");
                    }
                }
                OpCode::Multiply => {
                    if self.binary_op_num2num(&mut stack, |a, b| a * b).is_err() {
                        return self.runtime_error(ip, &chunk, "Operands must be numbers.");
                    }
                }
                OpCode::Divide => {
                    if self.binary_op_num2num(&mut stack, |a, b| a / b).is_err() {
                        return self.runtime_error(ip, &chunk, "Operands must be numbers.");
                    }
                }
                OpCode::Not => stack.apply_to_top(|value| value.is_falsey()),
                OpCode::Negate => {
                    if stack.apply_to_top_num2num(|value| -value).is_err() {
                        return self.runtime_error(ip, &chunk, "Operand must be a number.");
                    }
                }
                OpCode::Return => {
                    #[allow(unused)]
                    let out = stack.pop();
                    #[cfg(feature = "debug_trace_execution")]
                    {
                        std::mem::drop(disassembler);
                        out.debug_print(&self.heap, &mut debug_writer);
                        writeln!(debug_writer).expect("Error writing debug.");
                    }
                    break InterpretResult::InterpretOk(Some(out));
                }
            }
        }
    }

    #[must_use]
    fn runtime_error(&mut self, ip: usize, chunk: &Chunk, message: &str) -> InterpretResult {
        writeln!(self.error_writer, "{}", message).expect("Error writing error.");

        let line = chunk.line_at_index(ip);

        writeln!(self.error_writer, "[line {line}] in script").expect("Error writing error.");

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

    // These tests won't be valid once the VM's run method doesn't return anything,
    // but I wanted to get some in place while we do larger changes to the codebase.
    // It's better than manual tests.

    fn run(source: &str) -> InterpretResult {
        let mut vm = VirtualMachine::new(std::io::sink());
        vm.interpret(source.to_string(), std::io::sink())
    }

    fn assert_outputs_bool(source: &str, expected: bool) {
        match run(source) {
            InterpretResult::InterpretOk(Some(Value::Bool(out))) => assert_eq!(out, expected),
            _ => assert!(false),
        }
    }

    fn assert_outputs_number(source: &str, expected: f64) {
        match run(source) {
            InterpretResult::InterpretOk(Some(Value::Number(out))) => assert_eq!(out, expected),
            _ => assert!(false),
        }
    }

    fn assert_outputs_string(source: &str, expected: &str) {
        let mut vm = VirtualMachine::new(std::io::sink());
        let out = vm.interpret(source.to_string(), std::io::sink());

        match out {
            InterpretResult::InterpretOk(Some(Value::Obj(index))) => {
                let obj = unsafe { vm.heap.get_unchecked(index) };
                match obj.obj_type() {
                    ObjType::String(obj_str) => {
                        assert_eq!(obj_str.string(), expected);
                    } /*
                      _ => assert!(false),
                      */
                }
            }
            _ => assert!(false),
        }
    }
    #[test]
    fn test_load() {
        assert_outputs_number("1", 1.0);
    }

    #[test]
    fn test_arithmetic() {
        assert_outputs_number("1 + 1", 2.0);
        assert_outputs_number("1 / 2", 0.5);
        assert_outputs_number("2 * 3", 6.0);
        assert_outputs_number("4 - 5", -1.0);
    }

    #[test]
    fn test_string_concat() {
        assert_outputs_string(r#""Hello, " + "World!""#, "Hello, World!");
    }

    #[test]
    fn test_string_equality() {
        assert_outputs_bool(r#" "abc" == "abc" "#, true);
    }
}
