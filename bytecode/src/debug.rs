use crate::bytecode::{Chunk, OpCode};

pub(crate) fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);

    let mut offset = 0;
    let mut prev_line = usize::MAX;
    while offset < chunk.count() {
        (offset, prev_line) = disassemble_instruction(chunk, offset, prev_line);
    }
}

fn disassemble_instruction(chunk: &Chunk, offset: usize, prev_line: usize) -> (usize, usize) {
    print!("{offset:04} ");

    let line = chunk.line_at_index(offset);
    if line != prev_line {
        print!("{line:04} ");
    } else {
        print!("   | ");
    }

    let maybe_op = chunk.op_at_index(offset);
    let offset = match maybe_op {
        Ok(op @ OpCode::OpReturn) => simple_instruction(&op.dis_string(), offset),
        Ok(op @ OpCode::OpConstant) => constant_instruction(&op.dis_string(), chunk, offset),
        Ok(op @ OpCode::OpConstantLong) => {
            constant_long_instruction(&op.dis_string(), chunk, offset)
        }
        Err(byte) => {
            println!("Unknown op code {byte}");
            offset + 1
        }
    };
    (offset, line)
}

fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant_idx = chunk.byte_at_index(offset + 1);
    print!("{:<16} {:4} ", name, constant_idx);
    chunk.constant_at_index(constant_idx as usize).print();
    println!();
    offset + 2
}

fn constant_long_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant_idx = (chunk.byte_at_index(offset + 1) as usize)
        | ((chunk.byte_at_index(offset + 2) as usize) << 8)
        | ((chunk.byte_at_index(offset + 3) as usize) << 16);

    print!("{:<16} {:4} ", name, constant_idx);
    chunk.constant_at_index(constant_idx).print();
    println!();
    offset + 4
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{name}");
    offset + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_long_runs() {
        let mut chunk = Chunk::new();
        for i in 0..300 {
            chunk.write_constant(crate::value::Value::Number(i as f64), 123);
        }
        // chunk.write_constant(value::Value::Number(1.2), 123);
        chunk.write_op(OpCode::OpReturn, 123);

        disassemble_chunk(&chunk, "test chunk");
    }
}
