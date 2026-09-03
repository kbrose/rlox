use crate::{
    bytecode::{Chunk, OpCode},
    value::Value,
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
    stack: Vec<Value>, // TODO: C version uses an array for this.
}

impl VirtualMachine {
    pub(crate) fn new() -> Self {
        Self {
            stack: Vec::with_capacity(STACK_MAX),
        }
    }

    pub(crate) fn free(&mut self) {}

    fn push(&mut self, value: Value) {
        // Option 1: pointer manipulation, assumes self.stack has capacity
        // This does perform more pointer manipulation than the book, though!
        unsafe {
            let len = self.stack.len();
            let ptr = self.stack.as_mut_ptr().add(len);
            std::ptr::write(ptr, value);
            self.stack.set_len(len + 1);
        }

        // Option 2: No assumptions, but also faster
        // self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        // Option 1: pointer manipulation, assumes self.stack is non-empty
        // This does perform more pointer manipulation than the book, though!
        // unsafe {
        //     let len = self.stack.len();
        //     let ptr = self.stack.as_ptr().add(len - 1);
        //     let out = std::ptr::read(ptr);
        //     self.stack.set_len(len - 1);
        //     out
        // }

        // Option 2: Also assumes self.stack is non-empty.
        // match self.stack.pop() {
        //     Some(value) => value,
        //     None => unsafe {
        //         use std::hint::unreachable_unchecked;
        //         unreachable_unchecked()
        //     },
        // }

        // Option 3: No assumptions, but also faster
        match self.stack.pop() {
            Some(value) => value,
            None => unreachable!(),
        }
    }

    pub(crate) fn interpret(&mut self, chunk: Chunk) -> InterpretResult {
        self.run(chunk)
    }

    fn binary_op(&mut self, f: impl Fn(Value, Value) -> Value) {
        let b = self.pop();
        let a = self.pop();
        self.push(f(a, b));
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
                OpCode::OpConstantLong => {
                    let constant_idx = (chunk.byte_at_index(post_increment(&mut ip)) as usize)
                        | ((chunk.byte_at_index(post_increment(&mut ip)) as usize) << 8)
                        | ((chunk.byte_at_index(post_increment(&mut ip)) as usize) << 16);

                    let constant = unsafe { chunk.constant_at_index_unchecked(constant_idx) };
                    self.push(*constant);
                }
                OpCode::OpConstant => {
                    let constant = unsafe {
                        chunk.constant_at_index_unchecked(
                            chunk.byte_at_index(post_increment(&mut ip)) as usize,
                        )
                    };
                    self.push(*constant);
                }
                OpCode::OpAdd => self.binary_op(|a, b| a + b),
                OpCode::OpSubtract => self.binary_op(|a, b| a - b),
                OpCode::OpMultiply => self.binary_op(|a, b| a * b),
                OpCode::OpDivide => self.binary_op(|a, b| a / b),
                OpCode::OpNegate => {
                    let value = self.pop();
                    self.push(-value)
                }
                OpCode::OpReturn => {
                    #[allow(unused)]
                    let out = self.pop();
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
