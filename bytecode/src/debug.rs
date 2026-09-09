use std::io::Write;

use crate::{
    bytecode::{Chunk, OpCode},
    heap::ObjHeap,
};

#[allow(unused)]
pub(crate) struct Disassembler {
    prev_line: usize,
}

#[allow(unused)]
impl Disassembler {
    pub(crate) fn new() -> Self {
        Self {
            prev_line: usize::MAX,
        }
    }

    pub(crate) fn disassemble_chunk<W: Write>(
        &mut self,
        chunk: &Chunk,
        name: &str,
        obj_heap: &ObjHeap,
        writer: &mut W,
    ) {
        writeln!(writer, "== {} ==", name).unwrap();
        writeln!(writer, "offs line op            cidx cval").unwrap();

        let mut offset = 0;
        while offset < chunk.count() {
            (offset, self.prev_line) =
                self.disassemble_instruction(chunk, offset, obj_heap, writer);
        }
    }

    pub(crate) fn disassemble_instruction<W: Write>(
        &mut self,
        chunk: &Chunk,
        offset: usize,
        obj_heap: &ObjHeap,
        writer: &mut W,
    ) -> (usize, usize) {
        write!(writer, "{offset:04} ");

        let line = chunk.line_at_index(offset);
        if line != self.prev_line {
            write!(writer, "{line:04} ").unwrap();
        } else {
            write!(writer, "   | ").unwrap();
        }

        let maybe_op = chunk.op_at_index(offset);
        let offset = match maybe_op {
            // Simple instructions
            Ok(
                op @ (OpCode::Return
                | OpCode::Negate
                | OpCode::Add
                | OpCode::Subtract
                | OpCode::Multiply
                | OpCode::Divide
                | OpCode::Nil
                | OpCode::True
                | OpCode::False
                | OpCode::Not
                | OpCode::Equal
                | OpCode::Greater
                | OpCode::Less),
            ) => self.simple_instruction(&op.dis_string(), offset, writer),
            // Constant loading instructions
            Ok(op @ OpCode::Constant) => {
                self.constant_instruction(&op.dis_string(), chunk, offset, obj_heap, writer)
            }
            Ok(op @ OpCode::ConstantLong) => {
                self.constant_long_instruction(&op.dis_string(), chunk, offset, obj_heap, writer)
            }
            // Something else?
            Err(byte) => {
                writeln!(writer, "Unknown op code {byte}").unwrap();
                offset + 1
            }
        };
        (offset, line)
    }

    fn constant_instruction<W: Write>(
        &mut self,
        name: &str,
        chunk: &Chunk,
        offset: usize,
        obj_heap: &ObjHeap,
        writer: &mut W,
    ) -> usize {
        let constant_idx = chunk.byte_at_index(offset + 1);
        write!(writer, "{:<14} {:4} ", name, constant_idx).unwrap();
        chunk
            .constant_at_index(constant_idx as usize)
            .debug_print(obj_heap, writer);
        writeln!(writer).unwrap();
        offset + 2
    }

    fn constant_long_instruction<W: Write>(
        &mut self,
        name: &str,
        chunk: &Chunk,
        offset: usize,
        obj_heap: &ObjHeap,
        writer: &mut W,
    ) -> usize {
        let constant_idx = (chunk.byte_at_index(offset + 1) as usize)
            | ((chunk.byte_at_index(offset + 2) as usize) << 8)
            | ((chunk.byte_at_index(offset + 3) as usize) << 16);

        write!(writer, "{:<14} {:4} ", name, constant_idx).unwrap();
        chunk
            .constant_at_index(constant_idx)
            .debug_print(obj_heap, writer);
        writeln!(writer).unwrap();
        offset + 4
    }

    fn simple_instruction<W: Write>(&mut self, name: &str, offset: usize, writer: &mut W) -> usize {
        writeln!(writer, "{name}").unwrap();
        offset + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_long_runs() {
        let obj_heap = ObjHeap::new();
        let mut chunk = Chunk::new();
        for i in 0..300 {
            chunk.write_constant(crate::value::Value::new_number(i as f64), 123);
        }
        // chunk.write_constant(value::Value::new(1.2), 123);
        chunk.write_op(OpCode::Return, 123);

        let mut disassembler = Disassembler::new();
        let mut writer = std::io::sink();
        disassembler.disassemble_chunk(&chunk, "test chunk", &obj_heap, &mut writer);
    }
}
