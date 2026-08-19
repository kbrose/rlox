use std::fmt;

use crate::interpreter::callables::LoxCallable;
use std::io::Write;
use std::time::SystemTime;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum LoxObject {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    NativeFunction(NativeFunction),
}

impl fmt::Display for LoxObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "nil"),
            Self::Boolean(b) => write!(f, "{b}"),
            Self::Number(x) => {
                let s: String = x.to_string();
                if s.ends_with(".0") {
                    write!(f, "{}", s.split('.').next().unwrap())
                } else {
                    write!(f, "{}", s)
                }
            }
            Self::String(s) => write!(f, "{}", s),
            // TODO: Any other kind of representation here?
            Self::NativeFunction(_) => write!(f, "<native fn>"),
        }
    }
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
        }
    }

    pub(crate) fn get_string(&self) -> Result<&str, String> {
        match self {
            LoxObject::String(s) => Ok(s),
            LoxObject::Number(_) => Err(String::from("Type error: expected String, found Number")),
            LoxObject::Nil => Err(String::from("Type error: expected String, found Nil")),
            LoxObject::Boolean(_) => Err(String::from("Type error: expected String, found Boolean")),
            LoxObject::NativeFunction(_) => Err(String::from("Type error: expected String, found Function")),
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
        }
    }

    pub(crate) fn is_equal(&self, other: &Self) -> bool {
        if let (Ok(a), Ok(b)) = (self.get_number(), other.get_number()) {
            (a.is_nan() && b.is_nan()) || (a == b)
        } else {
            self == other
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
        _interpreter: &mut crate::interpreter::Interpreter<W>,
        _args: &[LoxObject],
    ) -> Result<LoxObject, crate::interpreter::interpreter::RuntimeError> {
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

    fn arity(self: &Self) -> usize {
        match self {
            Self::Clock => 0,
        }
    }

    fn to_string(self: &Self) -> String {
        "<native fn>".to_string()
    }
}
