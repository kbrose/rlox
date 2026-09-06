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

pub(crate) struct VirtualMachine {
    stack: Stack,
}

impl VirtualMachine {
    pub(crate) fn new() -> Self {
        Self {
            stack: Stack::new(STACK_MAX),
        }
    }

    pub(crate) fn interpret(&mut self, source: String) -> InterpretResult {
        compile(&source);
        InterpretResult::InterpretOk
        // self.run(chunk)
    }

    #[inline]
    fn binary_op(&mut self, f: impl Fn(Value, Value) -> Value) {
        let b = self.stack.pop();
        self.stack.apply_to_top(|a| f(a, b));
    }

    fn run(&mut self, chunk: Chunk) -> InterpretResult {
        let mut ip = 0;

        #[cfg(feature = "debug_trace_execution")]
        let mut disassembler = {
            use crate::debug::Disassembler;

            Disassembler::new(std::io::stdout())
        };

        loop {
            #[cfg(feature = "debug_trace_execution")]
            {
                print!("          ");
                for value in self.stack.iter() {
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
                OpCode::Add => self.binary_op(|a, b| a + b),
                OpCode::Subtract => self.binary_op(|a, b| a - b),
                OpCode::Multiply => self.binary_op(|a, b| a * b),
                OpCode::Divide => self.binary_op(|a, b| a / b),
                OpCode::Negate => {
                    self.stack.apply_to_top(|value| -value);
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
}

/// An implementation of C's `x++`
fn post_increment(x: &mut usize) -> usize {
    let out = *x;
    *x += 1;
    out
}
