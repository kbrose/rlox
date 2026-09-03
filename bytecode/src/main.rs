mod bytecode;
mod debug;
mod value;
mod vm;

use bytecode::*;
use vm::*;

use crate::value::Value;

fn main() {
    let mut vm = VirtualMachine::new();

    let mut chunk = Chunk::new();
    // chunk.write_constant(Value::new(1.2), 123);
    // chunk.write_constant(Value::new(3.4), 123);
    // chunk.write_op(OpCode::OpAdd, 123);
    // chunk.write_constant(Value::new(5.6), 123);
    // chunk.write_op(OpCode::OpDivide, 123);
    // chunk.write_op(OpCode::OpNegate, 123);
    // chunk.write_op(OpCode::OpReturn, 123);

    chunk.write_constant(Value::new(-499999500000.0), 0);
    for i in 0..500_000 {
        chunk.write_constant(Value::new(i as f64), 0);
        chunk.write_op(OpCode::OpAdd, 0);
    }
    chunk.write_op(OpCode::OpReturn, 0);

    // use debug::*;
    // let mut disassembler = Disassembler::new(std::io::stdout());
    // disassembler.disassemble_chunk(&chunk, "test chunk");

    let mut deltas = Vec::new();
    for _ in 0..500 {
        let c = chunk.clone();
        let start = std::time::Instant::now();
        vm.interpret(c);
        let end = std::time::Instant::now();
        deltas.push(end - start);
    }

    let avg = deltas.iter().sum::<std::time::Duration>() / deltas.len() as u32;
    println!("Timing: {:?}", avg);

    vm.free();
}
