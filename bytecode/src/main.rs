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

    #[cfg(not(feature = "timings"))]
    {
        chunk.write_constant(Value::new(1.2), 123);
        chunk.write_constant(Value::new(3.4), 123);
        chunk.write_op(OpCode::OpAdd, 123);
        chunk.write_constant(Value::new(5.6), 123);
        chunk.write_op(OpCode::OpDivide, 123);
        chunk.write_op(OpCode::OpNegate, 123);
        chunk.write_op(OpCode::OpReturn, 123);

        use debug::*;
        let mut disassembler = Disassembler::new(std::io::stdout());
        disassembler.disassemble_chunk(&chunk, "test chunk");

        vm.interpret(chunk);
    }

    #[cfg(feature = "timings")]
    {
        chunk.write_constant(Value::new(-124999750000.0), 0);
        for i in 0..500_000 {
            chunk.write_constant(Value::new(i as f64), 0);
            chunk.write_op(OpCode::OpAdd, 0);
        }
        chunk.write_op(OpCode::OpReturn, 0);

        let mut deltas = Vec::new();
        for _ in 0..10_000 {
            let c = chunk.clone();
            let start = std::time::Instant::now();
            vm.interpret(c);
            let end = std::time::Instant::now();
            deltas.push(end - start);
        }

        let avg = deltas.iter().sum::<std::time::Duration>() / deltas.len() as u32;

        deltas.sort();

        let p10 = deltas.get(((deltas.len() as f64) * 0.10) as usize).unwrap();
        let p50 = deltas.get(((deltas.len() as f64) * 0.50) as usize).unwrap();
        let p90 = deltas.get(((deltas.len() as f64) * 0.90) as usize).unwrap();

        println!(
            "Timing: mean={:?}, 10th pctile={:?}, 50th pctile={:?}, 90th pctile={:?}",
            avg, p10, p50, p90
        );
    }

    vm.free();
}
