use crate::value::Value;

#[allow(unused)]
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum OpCode {
    ConstantLong,
    Constant,
    Nil,
    True,
    False,
    Pop,
    Equal,
    Greater,
    Less,
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Negate,
    Print,
    DefineGlobal,
    DefineGlobalLong,
    SetGlobal,
    SetGlobalLong,
    GetGlobal,
    GetGlobalLong,
    SetLocal,
    GetLocal,
    Return,
}

const LAST_OP_CODE: OpCode = OpCode::Return;

#[allow(unused)]
impl OpCode {
    #[inline]
    pub(crate) fn to_byte(self: Self) -> u8 {
        // SAFETY: All values of OpCode are valid u8 because OpCode is repr(u8).
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub(crate) unsafe fn from_byte_unchecked(byte: u8) -> OpCode {
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
            Self::Constant => "CONSTANT",
            Self::Return => "RETURN",
            Self::ConstantLong => "CONSTANT_LONG",
            Self::Nil => "NIL",
            Self::True => "TRUE",
            Self::False => "FALSE",
            Self::Pop => "POP",
            Self::Equal => "EQUAL",
            Self::Greater => "GREATER",
            Self::Less => "LESS",
            Self::Negate => "NEGATE",
            Self::Add => "ADD",
            Self::Subtract => "SUBTRACT",
            Self::Not => "NOT",
            Self::Multiply => "MULTIPLY",
            Self::Divide => "DIVIDE",
            Self::Print => "PRINT",
            Self::DefineGlobal => "DEFINE_GLOBAL",
            Self::DefineGlobalLong => "DEFINE_GLOBAL_LONG",
            Self::GetGlobal => "GET_GLOBAL",
            Self::GetGlobalLong => "GET_GLOBAL_LONG",
            Self::SetGlobal => "SET_GLOBAL",
            Self::SetGlobalLong => "SET_GLOBAL_LONG",
            Self::SetLocal => "SET_LOCAL",
            Self::GetLocal => "GET_LOCAL",
        }
        .to_string()
    }
}

#[derive(Clone)]
struct Lines {
    /// cumulative_run_counts[i] is the total number of instructions that
    /// exist on lines 0 up through i. (This enables binary searching.)
    cumulative_run_counts: Vec<usize>,
    /// This is just used for santiy checking: it is only valid to call
    /// add_instruction_line() with ever-increasing line numbers.
    #[cfg(debug_assertions)]
    prev: usize,
}

#[allow(unused)]
impl Lines {
    fn new() -> Self {
        Lines {
            cumulative_run_counts: vec![],
            #[cfg(debug_assertions)]
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

    /// Gets the (0-indexed) line number. If the requested `instruction_index`
    /// is out of the range, then the _next largest_ line number will be
    /// returned. Note: if no lines have been observed yet, then 0 is
    /// the next largest.
    fn get_line(&self, instruction_index: usize) -> usize {
        match self
            .cumulative_run_counts
            .binary_search(&(instruction_index + 1))
        {
            Ok(mut i) => {
                // Binary search can return any index that matches. We always want
                // the lowest index that matches counts with the returned index.
                let target_count = self.cumulative_run_counts[i];

                while (i > 0) && (self.cumulative_run_counts[i - 1] == target_count) {
                    i -= 1;
                }

                i
            }
            Err(where_to_insert) => where_to_insert,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ConstantIndex {
    Byte(u8),
    Usize(usize),
}

const _CONSTANT_INDEX_BYTE_DISC: std::mem::Discriminant<ConstantIndex> =
    std::mem::discriminant(&ConstantIndex::Byte(0));

pub(crate) struct Chunk {
    pub(crate) code: Vec<u8>,
    constants: Vec<Value>,
    lines: Lines,
}

#[allow(unused)]
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

    /// Get the constant at the given index.
    ///
    /// PANICS if `constant_idx` is out of bounds.
    pub(crate) fn constant_at_index(&self, constant_idx: usize) -> &Value {
        &self.constants[constant_idx]
    }

    pub(crate) unsafe fn constant_at_index_unchecked(&self, constant_idx: usize) -> &Value {
        unsafe { self.constants.get_unchecked(constant_idx) }
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

    fn write_index(&mut self, op: OpCode, constant_index: ConstantIndex, line: usize) {
        self.write_op(op, line);
        match constant_index {
            ConstantIndex::Byte(index) => {
                self.write_byte(index, line);
            }
            ConstantIndex::Usize(index) => {
                // Only 24 bit indexes allowed.
                debug_assert!((index & 0xFFFFFF) == index);
                self.write_byte((index & 0xFF) as u8, line);
                self.write_byte(((index >> 8) & 0xFF) as u8, line);
                self.write_byte(((index >> 16) & 0xFF) as u8, line);
            }
        }
    }

    pub(crate) fn add_to_constants(&mut self, value: Value) -> ConstantIndex {
        let index = self.write_value_to_constants(value);
        if index <= 0xFF {
            ConstantIndex::Byte(index as u8)
        } else {
            ConstantIndex::Usize(index)
        }
    }

    pub(crate) fn write_constant(&mut self, value: Value, line: usize) -> ConstantIndex {
        let index = self.write_value_to_constants(value);
        let (out, op) = if index <= 0xFF {
            (ConstantIndex::Byte(index as u8), OpCode::Constant)
        } else {
            (ConstantIndex::Usize(index as usize), OpCode::ConstantLong)
        };
        self.write_index(op, out, line);
        out
    }

    pub(crate) fn define_global(&mut self, constant_index: ConstantIndex, line: usize) {
        let op = if std::mem::discriminant(&constant_index) == _CONSTANT_INDEX_BYTE_DISC {
            OpCode::DefineGlobal
        } else {
            OpCode::DefineGlobalLong
        };
        self.write_index(op, constant_index, line)
    }

    pub(crate) fn set_global(&mut self, constant_index: ConstantIndex, line: usize) {
        let op = if std::mem::discriminant(&constant_index) == _CONSTANT_INDEX_BYTE_DISC {
            OpCode::SetGlobal
        } else {
            OpCode::SetGlobalLong
        };
        self.write_index(op, constant_index, line)
    }

    pub(crate) fn get_global(&mut self, index: ConstantIndex, line: usize) {
        let op = if std::mem::discriminant(&index) == _CONSTANT_INDEX_BYTE_DISC {
            OpCode::GetGlobal
        } else {
            OpCode::GetGlobalLong
        };
        self.write_index(op, index, line);
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
    fn test_opcode_roundtrips_byte() {
        for i in 0..u8::MAX {
            if let Some(op) = OpCode::from_byte(i) {
                assert_eq!(op.to_byte(), i);
            }
        }
    }

    #[test]
    fn test_line_numbers() {
        let mut lines = Lines::new();
        println!("empty :\n{:?}", lines.cumulative_run_counts);

        assert_eq!(lines.get_line(0), 0);

        // Instruction 1
        lines.add_instruction_line(0);
        println!("instr1:\n{:?}", lines.cumulative_run_counts);
        assert_eq!(lines.get_line(0), 0);
        assert_eq!(lines.get_line(1), 1);

        // Instruction 2
        lines.add_instruction_line(0);
        println!("instr2:\n{:?}", lines.cumulative_run_counts);
        assert_eq!(lines.get_line(0), 0);
        assert_eq!(lines.get_line(1), 0);
        assert_eq!(lines.get_line(2), 1);

        // Instruction 3
        lines.add_instruction_line(1);
        println!("instr3:\n{:?}", lines.cumulative_run_counts);
        assert_eq!(lines.get_line(0), 0);
        assert_eq!(lines.get_line(1), 0);
        assert_eq!(lines.get_line(2), 1);
        assert_eq!(lines.get_line(3), 2);

        // Instruction 4
        lines.add_instruction_line(5);
        println!("instr4:\n{:?}", lines.cumulative_run_counts);
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
            chunk.write_constant(crate::value::Value::new_number(i as f64), 123);
            assert!(chunk.code[chunk.code.len() - 2] == OpCode::Constant.to_byte());
        }
        // All other constants should be OpConstantLong
        for i in 0..=0xFF {
            chunk.write_constant(crate::value::Value::new_number(i as f64), 123);
            assert!(chunk.code[chunk.code.len() - 4] == OpCode::ConstantLong.to_byte());
        }
    }
}
