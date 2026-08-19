use std::io::Write;

use crate::interpreter::{EvaluationException, Interpreter, lox_object::LoxObject};

pub(crate) trait LoxCallable<W: Write> {
    fn call(
        self: &Self,
        interpreter: &mut Interpreter<W>,
        arguments: Vec<LoxObject>,
    ) -> Result<LoxObject, EvaluationException>;

    fn arity(self: &Self) -> usize;

    fn to_string(self: &Self) -> String;
}
