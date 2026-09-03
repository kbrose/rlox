use crate::value::Value;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum OpCode {
    OpConstantLong,
    OpConstant,
    OpReturn,
}

const LAST_OP_CODE: OpCode = OpCode::OpReturn;

impl OpCode {
    #[inline]
    pub(crate) fn to_byte(self: Self) -> u8 {
        // SAFETY: All values of OpCode are valid u8 because OpCode is repr(u8).
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    unsafe fn from_byte_unchecked(byte: u8) -> OpCode {
        // SAFETY: byte must be <= the largest discriminant of OpCode.
        unsafe { std::mem::transmute(byte) }
    }

    #[inline]
    pub(crate) fn from_byte(byte: u8) -> Option<OpCode> {
        if byte > LAST_OP_CODE as u8 {
            None
        } else {
            unsafe { Some(OpCode::from_byte_unchecked(byte)) }
        }
    }

    pub(crate) fn dis_string(&self) -> String {
        match self {
            Self::OpConstant => "OP_CONSTANT".to_string(),
            Self::OpReturn => "OP_RETURN".to_string(),
            Self::OpConstantLong => "OP_CONSTANT_LONG".to_string(),
        }
    }
}

struct Lines {
    /// cumulative_run_counts[i] is the total number of instructions that
    /// exist on lines 0 up through i. (This enables binary searching.)
    cumulative_run_counts: Vec<usize>,
    /// This is just used for santiy checking: it is only valid to call
    /// add_instruction_line() with ever-increasing line numbers.
    #[cfg(debug_assertions)]
    prev: usize,
}

impl Lines {
    fn new() -> Self {
        Lines {
            cumulative_run_counts: vec![],
            prev: 0,
        }
    }

    /// Add line information for the next instruction. This should be called
    /// _every time_ a byte is written to a chunk.
    fn add_instruction_line(&mut self, line: usize) {
        #[cfg(debug_assertions)]
        {
            assert!(self.prev <= line);
            self.prev = line;
        }

        let target_len = line + 1;
        if target_len != self.cumulative_run_counts.len() {
            // Just unwrap the last. We always constructed with at least one element.
            let last_num = *self.cumulative_run_counts.last().unwrap_or(&0);
            self.cumulative_run_counts.resize(target_len, last_num);
        }
        *self.cumulative_run_counts.last_mut().unwrap() += 1;
    }

    /// Gets the (0-indexed) line number. If the requested instruction_index
    /// is out of the range, then the largest line number observed PLUS ONE
    /// is returned, unless no line numbers have been observed in which
    /// case 0 is returned.
    fn get_line(&self, instruction_index: usize) -> usize {
        match self
            .cumulative_run_counts
            .binary_search(&(instruction_index + 1))
        {
            Ok(exact_match) => {
                if exact_match == 0 {
                    exact_match
                } else {
                    // Binary search can return any index that matches. We always want
                    // the lowest index that matches.
                    let mut i = exact_match - 1;

                    while i > 0 {
                        if self.cumulative_run_counts[i] == self.cumulative_run_counts[exact_match]
                        {
                            i -= 1;
                        } else {
                            break;
                        }
                    }

                    i + 1
                }
            }
            Err(where_to_insert) => where_to_insert,
        }
    }
}

pub(crate) struct Chunk {
    code: Vec<u8>,
    constants: Vec<Value>,
    lines: Lines,
}

impl Chunk {
    pub(crate) fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Lines::new(),
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.code.len()
    }

    pub(crate) fn line_at_index(&self, offset: usize) -> usize {
        self.lines.get_line(offset)
    }

    pub(crate) fn byte_at_index(&self, offset: usize) -> u8 {
        self.code[offset]
    }

    /// Attempts to parse the byte located at `offset` into an `OpCode`.
    ///
    /// PANICS if `offset` is out of bounds.
    pub(crate) fn op_at_index(&self, offset: usize) -> Result<OpCode, u8> {
        let byte = self.code[offset];
        OpCode::from_byte(byte).ok_or_else(|| byte)
    }

    /// Parse the byte located at `offset` into an `OpCode`.
    ///
    /// Undefined behavior will occur if either of these conditions are true:
    // 1. offset is out of bounds of self.code, or
    // 2. self.code[offset] is not a valid OpCode discriminant.
    pub(crate) unsafe fn op_unchecked_at_index_unchecked(&self, offset: usize) -> OpCode {
        // SAFETY: This assumes that both of these conditions are true:
        // 1. offset is in bounds of self.code, and
        // 2. self.code[offset] is a valid OpCode discriminant.
        unsafe { OpCode::from_byte_unchecked(*self.code.get_unchecked(offset)) }
    }

    /// Get the constant
    pub(crate) fn constant_at_index(&self, constant_idx: usize) -> &Value {
        &self.constants[constant_idx]
    }

    // I'm a little worried that these write functions hide the allocation. The
    // book uses a helper function for reallocation and states:
    //   > Routing all of those operations through a single function will be
    //   > important later when we add a garbage collector that needs to
    //   > keep track of how much memory is in use.
    pub(crate) fn write_op(&mut self, op: OpCode, line: usize) {
        self.write_byte(op.to_byte(), line);
    }

    pub(crate) fn write_byte(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.add_instruction_line(line);
    }

    pub(crate) fn write_constant(&mut self, value: Value, line: usize) {
        let index = self.write_value_to_constants(value);
        if index <= 0xFF {
            self.write_op(OpCode::OpConstant, line);
            self.write_byte(index as u8, line);
        } else {
            self.write_op(OpCode::OpConstantLong, line);
            self.write_byte((index & 0xFF) as u8, line);
            self.write_byte(((index >> 8) & 0xFF) as u8, line);
            self.write_byte(((index >> 16) & 0xFF) as u8, line);
        }
    }

    fn write_value_to_constants(&mut self, value: Value) -> usize {
        // TODO: Garbage collection has to happen here?
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub(crate) fn free_chunk(&mut self) {
        self.code = Vec::new();
        self.constants = Vec::new();
        self.lines = Lines::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_last_op_code() {
        // Construct the path to the current source file using CARGO_MANIFEST_DIR and file!()
        let file_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file!());
        let content = std::fs::read_to_string(file_path).unwrap();

        // 1. Find the OpCode enum definition block
        let enum_start = content
            .find("pub(crate) enum OpCode")
            .expect("OpCode enum not found");
        let enum_body_start = content[enum_start..].find('{').unwrap() + enum_start;
        let enum_body_end = content[enum_body_start..].find('}').unwrap() + enum_body_start;
        let enum_body = &content[enum_body_start + 1..enum_body_end];

        // 2. Parse out the variants, stripping whitespace and trailing commas
        let variants: Vec<&str> = enum_body
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .map(|line| line.trim_end_matches(','))
            .collect();

        let last_variant = variants.last().expect("OpCode enum has no variants");

        // 3. Find the LAST_OP_CODE constant declaration appearing below the enum
        let const_pos = content[enum_body_end..]
            .find("const LAST_OP_CODE")
            .expect("LAST_OP_CODE constant not found")
            + enum_body_end;
        let const_line = content[const_pos..]
            .lines()
            .next()
            .expect("LAST_OP_CODE line empty")
            .trim();

        // 4. Assert string-level match
        let expected_declaration = format!("const LAST_OP_CODE: OpCode = OpCode::{last_variant};");

        assert_eq!(
            const_line, expected_declaration,
            "LAST_OP_CODE is defined as {} but the last observed enum variant is {}.",
            const_line, last_variant
        );
    }

    #[test]
    fn test_discriminant() {
        assert_eq!(OpCode::OpReturn.to_byte(), 2);
    }

    #[test]
    fn test_line_numbers() {
        let mut lines = Lines::new();
        println!("{:?}", lines.cumulative_run_counts);

        assert_eq!(lines.get_line(0), 0);

        // Instruction 1
        lines.add_instruction_line(0);
        println!("\n{:?}", lines.cumulative_run_counts);
        assert_eq!(lines.get_line(0), 0);
        assert_eq!(lines.get_line(1), 1);

        // Instruction 2
        lines.add_instruction_line(0);
        println!("\n{:?}", lines.cumulative_run_counts);
        assert_eq!(lines.get_line(0), 0);
        assert_eq!(lines.get_line(1), 0);
        assert_eq!(lines.get_line(2), 1);

        // Instruction 3
        lines.add_instruction_line(1);
        println!("\n{:?}", lines.cumulative_run_counts);
        assert_eq!(lines.get_line(0), 0);
        assert_eq!(lines.get_line(1), 0);
        assert_eq!(lines.get_line(2), 1);
        assert_eq!(lines.get_line(3), 2);

        // Instruction 4
        lines.add_instruction_line(5);
        println!("\n{:?}", lines.cumulative_run_counts);
        assert_eq!(lines.get_line(0), 0);
        assert_eq!(lines.get_line(1), 0);
        assert_eq!(lines.get_line(2), 1);
        assert_eq!(lines.get_line(3), 5);
        assert_eq!(lines.get_line(4), 6);
    }

    #[test]
    fn test_constants() {
        let mut chunk = Chunk::new();
        // First 256 constants should be just OpConstant
        for i in 0..=0xFF {
            chunk.write_constant(crate::value::Value::Number(i as f64), 123);
            assert!(chunk.code[chunk.code.len() - 2] == OpCode::OpConstant.to_byte());
        }
        // All other constants should be OpConstantLong
        for i in 0..=0xFF {
            chunk.write_constant(crate::value::Value::Number(i as f64), 123);
            assert!(chunk.code[chunk.code.len() - 4] == OpCode::OpConstantLong.to_byte());
        }
    }
}
