use std::io::Write;

use crate::bytecode::{Chunk, OpCode};

#[allow(unused)]
pub(crate) struct Disassembler<W: Write> {
    writer: W,
    prev_line: usize,
}

#[allow(unused)]
impl<W: Write> Disassembler<W> {
    pub(crate) fn new(writer: W) -> Self {
        Self {
            writer,
            prev_line: usize::MAX,
        }
    }

    pub(crate) fn disassemble_chunk(&mut self, chunk: &Chunk, name: &str) {
        writeln!(self.writer, "== {} ==", name).unwrap();

        let mut offset = 0;
        while offset < chunk.count() {
            (offset, self.prev_line) = self.disassemble_instruction(chunk, offset);
        }
    }

    pub(crate) fn disassemble_instruction(
        &mut self,
        chunk: &Chunk,
        offset: usize,
    ) -> (usize, usize) {
        write!(self.writer, "{offset:04} ");

        let line = chunk.line_at_index(offset);
        if line != self.prev_line {
            write!(self.writer, "{line:04} ").unwrap();
        } else {
            write!(self.writer, "   | ").unwrap();
        }

        let maybe_op = chunk.op_at_index(offset);
        let offset = match maybe_op {
            // Simple instructions
            Ok(
                op @ (OpCode::OpReturn
                | OpCode::OpNegate
                | OpCode::OpAdd
                | OpCode::OpSubtract
                | OpCode::OpMultiply
                | OpCode::OpDivide),
            ) => self.simple_instruction(&op.dis_string(), offset),
            // Constant loading instructions
            Ok(op @ OpCode::OpConstant) => {
                self.constant_instruction(&op.dis_string(), chunk, offset)
            }
            Ok(op @ OpCode::OpConstantLong) => {
                self.constant_long_instruction(&op.dis_string(), chunk, offset)
            }
            // Something else?
            Err(byte) => {
                writeln!(self.writer, "Unknown op code {byte}").unwrap();
                offset + 1
            }
        };
        (offset, line)
    }

    fn constant_instruction(&mut self, name: &str, chunk: &Chunk, offset: usize) -> usize {
        let constant_idx = chunk.byte_at_index(offset + 1);
        write!(self.writer, "{:<16} {:4} ", name, constant_idx).unwrap();
        chunk.constant_at_index(constant_idx as usize).print();
        writeln!(self.writer).unwrap();
        offset + 2
    }

    fn constant_long_instruction(&mut self, name: &str, chunk: &Chunk, offset: usize) -> usize {
        let constant_idx = (chunk.byte_at_index(offset + 1) as usize)
            | ((chunk.byte_at_index(offset + 2) as usize) << 8)
            | ((chunk.byte_at_index(offset + 3) as usize) << 16);

        write!(self.writer, "{:<16} {:4} ", name, constant_idx).unwrap();
        chunk.constant_at_index(constant_idx).print();
        writeln!(self.writer).unwrap();
        offset + 4
    }

    fn simple_instruction(&mut self, name: &str, offset: usize) -> usize {
        writeln!(self.writer, "{name}").unwrap();
        offset + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_long_runs() {
        let mut chunk = Chunk::new();
        for i in 0..300 {
            chunk.write_constant(crate::value::Value::new(i as f64), 123);
        }
        // chunk.write_constant(value::Value::new(1.2), 123);
        chunk.write_op(OpCode::OpReturn, 123);

        let mut disassembler = Disassembler::new(std::io::sink());
        disassembler.disassemble_chunk(&chunk, "test chunk");
    }
}
