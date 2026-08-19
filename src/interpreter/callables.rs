use std::io::Write;

use crate::interpreter::{Interpreter, RuntimeError, lox_object::LoxObject};

pub(crate) trait LoxCallable<W: Write> {
    fn call(
        self: &Self,
        interpreter: &mut Interpreter<W>,
        arguments: &[LoxObject],
    ) -> Result<LoxObject, RuntimeError>;
    fn arity(self: &Self) -> usize;
    fn to_string(self: &Self) -> String;
}
