use std::io::{Stderr, Stdout, Write};

use crate::{
    bytecode::{Chunk, OpCode},
    compiler::compile,
    table::Table,
    value::{ObjType, Value, heap::Heap},
    vm::stack::Stack,
};

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

const STACK_MAX: usize = 256;

pub(crate) struct VirtualMachine<Wo: Write, We: Write> {
    /// Owns all heap-allocated objects.
    heap: Heap,
    // strings: HashSet<String>,
    writer: Wo,
    error_writer: We,
    globals: Table,
}

impl VirtualMachine<Stdout, Stderr> {
    pub(crate) fn new_with_std_io() -> Self {
        Self {
            heap: Heap::new(),
            writer: std::io::stdout(),
            error_writer: std::io::stderr(),
            globals: Table::new(),
        }
    }
}

impl<Wo: Write, We: Write> VirtualMachine<Wo, We> {
    #[allow(unused)]
    pub(crate) fn new(writer: Wo, error_writer: We) -> Self {
        Self {
            heap: Heap::new(),
            writer,
            error_writer,
            globals: Table::new(),
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

    #[inline]
    fn define_global(&mut self, value: &Value, stack: &mut Stack) {
        match value {
            Value::Obj(typed_heap_index) => {
                // NOTE! The book code uses .peek(0) here and then pops afterwards.
                // This is because the book can trigger garbage collection any time
                // any allocation happens. I'm pretty sure I'm not going to do that,
                // I see no reason not to do it at the boundary of executing each
                // op code... (yet). If I change my mind, this needs to change!
                self.globals
                    .set(*typed_heap_index, stack.pop(), &self.heap.object_heap());
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn set_global(
        &mut self,
        name_pointer: Value,
        ip: usize,
        chunk: &Chunk,
        stack: &Stack,
    ) -> Result<(), InterpretResult> {
        match name_pointer {
            Value::Obj(typed_heap_index) => {
                if self
                    .globals
                    .set(typed_heap_index, stack.peek(0), self.heap.object_heap())
                {
                    self.globals
                        .delete(typed_heap_index, &self.heap.object_heap());
                    Err(self.runtime_error(
                        ip,
                        &chunk,
                        &format!("Undefined variable '{}'.", unsafe {
                            self.heap.get_unchecked_objstr(typed_heap_index).string()
                        }),
                    ))
                } else {
                    Ok(())
                }
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn get_global(
        &mut self,
        name_pointer: &Value,
        ip: usize,
        chunk: &Chunk,
    ) -> Result<Value, InterpretResult> {
        match name_pointer {
            Value::Obj(typed_heap_index) => {
                let out = self
                    .globals
                    .get(*typed_heap_index, &self.heap.object_heap());
                match out {
                    Some(value) => Ok(value),
                    None => Err(self.runtime_error(
                        ip,
                        chunk,
                        &format!(
                            "Undefined variable '{}'.",
                            match name_pointer {
                                Value::Obj(typed_heap_index) => unsafe {
                                    self.heap.get_unchecked_objstr(*typed_heap_index).string()
                                },
                                _ => unreachable!(),
                            }
                        ),
                    )),
                }
            }
            _ => unreachable!(),
        }
    }

    fn run<W2: Write>(&mut self, chunk: Chunk, mut debug_writer: &mut W2) -> InterpretResult {
        let mut stack: Stack = Stack::new(STACK_MAX);
        let mut ip = 0;
        if chunk.code.len() == 0 {
            return InterpretResult::InterpretOk;
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
                    value.print(&self.heap, &mut debug_writer);
                    write!(debug_writer, " ]").expect("Error writing stack debug strings.");
                }
                writeln!(debug_writer).expect("Error writing stack debug strings.");
                disassembler.disassemble_instruction(&chunk, ip, &self.heap, debug_writer);
            }

            let op = unsafe { chunk.op_unchecked_at_index_unchecked(post_increment(&mut ip)) };
            match op {
                OpCode::ConstantLong => {
                    let constant = get_constant_long(&chunk, &mut ip);
                    stack.push(constant);
                }
                OpCode::Constant => {
                    let constant = get_constant(&chunk, &mut ip);
                    stack.push(constant);
                }
                OpCode::Nil => stack.push(Value::Nil),
                OpCode::True => stack.push(Value::Bool(true)),
                OpCode::False => stack.push(Value::Bool(false)),
                OpCode::Pop => {
                    stack.pop();
                }
                OpCode::GetLocal => {
                    let slot = chunk.byte_at_index(post_increment(&mut ip));
                    stack.push(stack.get(slot));
                }
                OpCode::SetLocal => {
                    let slot = chunk.byte_at_index(post_increment(&mut ip));
                    stack.set(slot, stack.peek(0));
                }
                OpCode::SetGlobal => {
                    let name_pointer = get_constant(&chunk, &mut ip);

                    match self.set_global(name_pointer, ip, &chunk, &stack) {
                        Ok(()) => {}
                        Err(result) => return result,
                    }
                }
                OpCode::SetGlobalLong => {
                    let name_pointer = get_constant_long(&chunk, &mut ip);

                    match self.set_global(name_pointer, ip, &chunk, &stack) {
                        Ok(()) => {}
                        Err(result) => return result,
                    }
                }
                OpCode::GetGlobal => {
                    let name_pointer = get_constant(&chunk, &mut ip);

                    match self.get_global(&name_pointer, ip, &chunk) {
                        Ok(value) => stack.push(value),
                        Err(result) => {
                            return result;
                        }
                    }
                }
                OpCode::GetGlobalLong => {
                    let name_pointer = get_constant_long(&chunk, &mut ip);

                    match self.get_global(&name_pointer, ip, &chunk) {
                        Ok(value) => stack.push(value),
                        Err(result) => {
                            return result;
                        }
                    }
                }
                OpCode::DefineGlobal => {
                    let constant_value = get_constant(&chunk, &mut ip);
                    self.define_global(&constant_value, &mut stack);
                }
                OpCode::DefineGlobalLong => {
                    let constant_value = get_constant_long(&chunk, &mut ip);

                    self.define_global(&constant_value, &mut stack);
                }
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
                OpCode::Print => {
                    let value = stack.pop();
                    value.print(&self.heap, &mut self.writer);
                    writeln!(self.writer, "").expect("Error printing.");
                }
                OpCode::Return => {
                    // #[allow(unused)]
                    // let out = stack.pop();
                    // #[cfg(feature = "debug_trace_execution")]
                    // {
                    //     std::mem::drop(disassembler);
                    //     out.print(&self.heap, &mut debug_writer);
                    //     writeln!(debug_writer).expect("Error writing debug.");
                    // }
                    break InterpretResult::InterpretOk;
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

fn get_constant(chunk: &Chunk, ip: &mut usize) -> Value {
    let constant_idx = chunk.byte_at_index(post_increment(ip)) as usize;

    let constant = unsafe { chunk.constant_at_index_unchecked(constant_idx) };
    *constant
}

fn get_constant_long(chunk: &Chunk, ip: &mut usize) -> Value {
    let constant_idx = (chunk.byte_at_index(post_increment(ip)) as usize)
        | ((chunk.byte_at_index(post_increment(ip)) as usize) << 8)
        | ((chunk.byte_at_index(post_increment(ip)) as usize) << 16);

    let constant = unsafe { chunk.constant_at_index_unchecked(constant_idx) };
    *constant
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

    fn run(source: &str) -> (String, InterpretResult) {
        let mut buffer = Vec::new();
        let mut vm = VirtualMachine::new(&mut buffer, std::io::sink());
        let out = vm.interpret(source.to_string(), std::io::sink());
        (
            String::from_utf8(buffer).expect("The VM printed non-utf8"),
            out,
        )
    }

    fn assert_expression_prints_expected(source: &str, expected: &str) {
        let source = format!("print {source};");
        let (printed, result) = run(&source);
        match result {
            InterpretResult::InterpretOk => assert_eq!(format!("{expected}\n"), printed),
            _ => assert!(false),
        }
    }

    fn assert_statements_print_expected(source: &str, expected: &str) {
        let (printed, result) = run(&source);
        match result {
            InterpretResult::InterpretOk => assert_eq!(format!("{expected}\n"), printed),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_load() {
        assert_expression_prints_expected("1", "1");
    }

    #[test]
    fn test_arithmetic() {
        assert_expression_prints_expected("1 + 1", "2");
        assert_expression_prints_expected("1 / 2", "0.5");
        assert_expression_prints_expected("2 * 3", "6");
        assert_expression_prints_expected("4 - 5", "-1");
    }

    #[test]
    fn test_string_concat() {
        assert_expression_prints_expected(r#""Hello, " + "World!""#, "Hello, World!");
    }

    #[test]
    fn test_string_equality() {
        assert_expression_prints_expected(r#" "abc" == "abc" "#, "true");
    }

    #[test]
    fn test_globals_storing_loading() {
        let source = r#"var breakfast = "beignets";
            var beverage = "cafe au lait";
            breakfast = breakfast + " with " + beverage;

            print breakfast;"#;

        assert_statements_print_expected(source, "beignets with cafe au lait");
    }

    #[test]
    fn test_300_globals_storing_loading() {
        let mut source = String::new();
        for i in 1..=300 {
            source.push_str(&format!("var x{i} = {i}; "));
        }
        source.push_str("var y = ");
        for i in 1..=300 {
            source.push_str(&format!("x{i} + "));
        }
        source.push_str("0; print y;");

        // Triangular sum: n*(n-1) / 2 = 45,150
        assert_statements_print_expected(&source, "45150");
    }

    #[test]
    fn test_lexical_scoping() {
        let source = r#"
        {
            var a = "outer";
            {
                var a = "inner";
                print a;
            }
            print a;
        }
        "#;

        assert_statements_print_expected(source, "inner\nouter");

        let source = r#"
        var a = "global";
        {
            var a = "outer";
            {
                var a = "inner";
                print a;
            }
            print a;
        }
        print a;
        "#;

        assert_statements_print_expected(source, "inner\nouter\nglobal");
    }
}
