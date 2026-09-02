use std::{io::Write, rc::Rc};

use crate::{
    interpreter::{EvaluationException, Interpreter, lox_object::LoxObject},
    parser::IdentifierToken,
};

pub(crate) struct ArgLengthMismatch {
    expected: usize,
    provided: usize,
}

impl ArgLengthMismatch {
    pub(super) fn new(expected: usize, provided: usize) -> Self {
        Self {
            expected,
            provided,
        }
    }

    #[inline(always)]
    pub(super) fn expected(&self) -> usize {
        self.expected
    }

    #[inline(always)]
    pub(super) fn provided(&self) -> usize {
        self.provided
    }
}

pub(crate) trait LoxCallable<W: Write> {
    fn call(
        self: &Self,
        interpreter: &mut Interpreter<W>,
        parsed_args: Vec<(&IdentifierToken, Rc<LoxObject>)>,
    ) -> Result<Rc<LoxObject>, EvaluationException>;

    fn align_arguments(
        self: &Self,
        arguments: Vec<Rc<LoxObject>>,
    ) -> Result<Vec<(&IdentifierToken, Rc<LoxObject>)>, ArgLengthMismatch>;

    fn to_string(self: &Self) -> String;
}
