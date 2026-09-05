use crate::{
    bytecode::{Chunk, OpCode},
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
    stack: Stack, // TODO: C version uses an array for this.
}

impl VirtualMachine {
    pub(crate) fn new() -> Self {
        Self {
            stack: Stack::new(STACK_MAX),
        }
    }

    pub(crate) fn free(&mut self) {}

    pub(crate) fn interpret(&mut self, chunk: Chunk) -> InterpretResult {
        self.run(chunk)
    }

    #[inline]
    fn binary_op(&mut self, f: impl Fn(Value, Value) -> Value) {
        let b = self.stack.pop();
        self.stack.apply_to_top(|a| f(a, b));
    }

    fn run(&mut self, chunk: Chunk) -> InterpretResult {
        // let mut ip = 0;
        let mut ip = chunk.code.as_ptr();

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
                disassembler.disassemble_instruction(&chunk, unsafe {
                    ip.offset_from_unsigned(chunk.code.as_ptr())
                });
            }

            let op = unsafe { OpCode::from_byte_unchecked(*post_increment(&mut ip)) };
            match op {
                OpCode::OpConstantLong => {
                    let constant_idx = (unsafe { *(post_increment(&mut ip)) } as usize)
                        | ((unsafe { *(post_increment(&mut ip)) } as usize) << 8)
                        | ((unsafe { *(post_increment(&mut ip)) } as usize) << 16);

                    let constant = unsafe { chunk.constant_at_index_unchecked(constant_idx) };
                    self.stack.push(*constant);
                }
                OpCode::OpConstant => {
                    let constant = unsafe {
                        chunk.constant_at_index_unchecked(*(post_increment(&mut ip)) as usize)
                    };
                    self.stack.push(*constant);
                }
                OpCode::OpAdd => self.binary_op(|a, b| a + b),
                OpCode::OpSubtract => self.binary_op(|a, b| a - b),
                OpCode::OpMultiply => self.binary_op(|a, b| a * b),
                OpCode::OpDivide => self.binary_op(|a, b| a / b),
                OpCode::OpNegate => {
                    self.stack.apply_to_top(|value| -value);
                }
                OpCode::OpReturn => {
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

// /// An implementation of C's `x++`
// fn post_increment(x: &mut usize) -> usize {
//     let out = *x;
//     *x += 1;
//     out
// }

/// An implementation of C's `x++`
/// SAFETY: Assumes x.add(1) is still in bounds
fn post_increment(x: &mut *const u8) -> *const u8 {
    let out = *x;
    *x = unsafe { x.add(1) };
    out
}
