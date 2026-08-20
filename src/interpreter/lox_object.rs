use crate::ast::Function;
use crate::interpreter::callables::{ArgLengthMismatch, LoxCallable};
use crate::interpreter::{EnvironmentWrapper, EvaluationException, Interpreter};
use crate::parser::IdentifierToken;
use std::io::Write;
use std::rc::Rc;
use std::time::SystemTime;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum LoxObject {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    NativeFunction(NativeFunction),
    LoxFunction(LoxFunction),
}

impl LoxObject {
    pub(crate) fn truthiness(&self) -> bool {
        match self {
            LoxObject::Nil => false,
            LoxObject::Boolean(b) => *b,
            // Everything else is truthy
            _ => true,
        }
    }

    pub(crate) fn get_number(&self) -> Result<f64, String> {
        match self {
            LoxObject::Number(x) => Ok(*x),
            LoxObject::Nil => Err(String::from("Type error: expected Number, found Nil")),
            LoxObject::Boolean(_) => Err(String::from("Type error: expected Number, found Boolean")),
            LoxObject::String(_) => Err(String::from("Type error: expected Number, found String")),
            LoxObject::NativeFunction(_) => Err(String::from("Type error: expected Number, found Function")),
            LoxObject::LoxFunction(_) => Err(String::from("Type error: expected Number, found Function")),
        }
    }

    pub(crate) fn get_string(&self) -> Result<&str, String> {
        match self {
            LoxObject::String(s) => Ok(s),
            LoxObject::Number(_) => Err(String::from("Type error: expected String, found Number")),
            LoxObject::Nil => Err(String::from("Type error: expected String, found Nil")),
            LoxObject::Boolean(_) => Err(String::from("Type error: expected String, found Boolean")),
            LoxObject::NativeFunction(_) => Err(String::from("Type error: expected String, found Function")),
            LoxObject::LoxFunction(_) => Err(String::from("Type error: expected String, found Function")),
        }
    }

    pub(crate) fn get_callable<W: Write>(&self) -> Result<Box<dyn LoxCallable<W>>, String> {
        match self {
            Self::Nil => Err(String::from("Type error: expected Callable, found Nil")),
            Self::Boolean(_) => Err(String::from("Type error: expected Callable, found Boolean")),
            Self::Number(_) => Err(String::from("Type error: expected Callable, found Number")),
            Self::String(_) => Err(String::from("Type error: expected Callable, found String")),
            // TODO: Avoid clone
            Self::NativeFunction(native_function) => Ok(Box::new(native_function.clone())),
            Self::LoxFunction(lox_function) => Ok(Box::new(lox_function.clone())),
        }
    }

    pub(crate) fn is_equal(&self, other: &Self) -> bool {
        if let (Ok(a), Ok(b)) = (self.get_number(), other.get_number()) {
            (a.is_nan() && b.is_nan()) || (a == b)
        } else {
            self == other
        }
    }

    pub(crate) fn to_string<W: Write>(&self) -> String {
        match self {
            Self::Nil => "nil".to_string(),
            Self::Boolean(b) => b.to_string(),
            Self::Number(x) => {
                let s: String = x.to_string();
                if s.ends_with(".0") {
                    s.split('.').next().unwrap().to_string()
                } else {
                    s
                }
            }
            Self::String(s) => s.clone(),
            Self::NativeFunction(fun) => <NativeFunction as LoxCallable<W>>::to_string(fun),
            Self::LoxFunction(fun) => <LoxFunction as LoxCallable<W>>::to_string(fun),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum NativeFunction {
    Clock,
}

impl<W: Write> LoxCallable<W> for NativeFunction {
    fn call(
        self: &Self,
        _interpreter: &mut Interpreter<W>,
        _parsed_args: Vec<(&IdentifierToken, LoxObject)>,
    ) -> Result<LoxObject, EvaluationException> {
        match self {
            Self::Clock => {
                let now = SystemTime::now();
                let epoch = SystemTime::UNIX_EPOCH;
                // TODO: How best to handle possible error here?
                Ok(match now.duration_since(epoch) {
                    Ok(duration) => LoxObject::Number(duration.as_secs_f64()),
                    Err(_) => LoxObject::Number(0.0),
                })
            }
        }
    }

    fn align_arguments(
        self: &Self,
        arguments: Vec<LoxObject>,
    ) -> Result<Vec<(&IdentifierToken, LoxObject)>, ArgLengthMismatch> {
        if arguments.len() > 0 {
            Err(ArgLengthMismatch::new(0, arguments.len()))
        } else {
            Ok(vec![])
        }
    }

    fn to_string(self: &Self) -> String {
        "<native fn>".to_string()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LoxFunction {
    declaration: Function,
    closure: EnvironmentWrapper,
}

impl PartialEq for LoxFunction {
    fn eq(&self, other: &Self) -> bool {
        self.declaration == other.declaration
    }
}

impl LoxFunction {
    pub(crate) fn new(declaration: Function, closure: EnvironmentWrapper) -> Self {
        Self {
            declaration,
            closure: closure,
        }
    }
}

impl<W: Write> LoxCallable<W> for LoxFunction {
    fn call(
        self: &Self,
        interpreter: &mut Interpreter<W>,
        parsed_args: Vec<(&IdentifierToken, LoxObject)>,
    ) -> Result<LoxObject, crate::interpreter::interpreter::EvaluationException> {
        let mut env = super::environment::new_scope(&Rc::clone(&self.closure));

        // We have verified their lengths match elsewhere.
        // TODO: Any way to use "parse don't validate" here?
        for (param_identifier, param_value) in parsed_args.into_iter() {
            env.borrow_mut().define(param_identifier, param_value);
        }

        let out = interpreter.execute_stmt(&self.declaration.body, &mut env);

        out?;

        Ok(LoxObject::Nil)
    }

    fn align_arguments(
        self: &Self,
        arguments: Vec<LoxObject>,
    ) -> Result<Vec<(&IdentifierToken, LoxObject)>, ArgLengthMismatch> {
        let arity = self.declaration.params.len();
        let num_args = arguments.len();

        if arity != num_args {
            Err(ArgLengthMismatch::new(arity, num_args))
        } else {
            Ok(self.declaration.params.iter().zip(arguments.into_iter()).collect())
        }
    }

    fn to_string(self: &Self) -> String {
        format!("<fn {}>", self.declaration.name.identifier())
    }
}
