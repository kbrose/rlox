mod bytecode;
mod debug;
mod value;

use bytecode::*;
use debug::*;

fn main() {
    let mut chunk = Chunk::new();
    for i in 0..300 {
        chunk.write_constant(value::Value::Number(i as f64), 123);
    }
    chunk.write_op(OpCode::OpReturn, 123);

    disassemble_chunk(&chunk, "test chunk");
    chunk.free_chunk();
}
