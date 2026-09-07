#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Value {
    Nil,
    Number(f64),
    Bool(bool),
}

impl Value {
    pub(crate) fn new_number(x: f64) -> Self {
        Self::Number(x)
    }

    pub(crate) fn new_bool(b: bool) -> Value {
        Self::Bool(b)
    }

    #[inline]
    pub(crate) fn as_number(&self) -> Result<f64, ()> {
        match self {
            Self::Number(x) => Ok(*x),
            _ => Err(()),
        }
    }

    #[inline]
    pub(crate) fn as_bool(&self) -> Result<bool, ()> {
        match self {
            Self::Bool(b) => Ok(*b),
            _ => Err(()),
        }
    }

    #[inline]
    pub(crate) fn is_nil(&self) -> bool {
        self == &Value::Nil
    }

    pub(crate) fn print(self: &Self) {
        match self {
            Self::Nil => {
                print!("Nil");
            }
            Self::Number(x) => {
                print!("'{}'", x);
            }
            Self::Bool(b) => {
                print!("{}", b);
            }
        }
    }

    pub(crate) fn is_falsey(&self) -> Value {
        match self {
            Self::Nil | Self::Bool(false) => Self::Bool(true),
            _ => Self::Bool(false),
        }
    }

    pub(crate) fn is_equal(&self, other: &Value) -> Value {
        match (self, other) {
            (Self::Nil, Self::Nil) => Self::Bool(true),
            (Self::Number(a), Self::Number(b)) => Self::Bool(a == b),
            (Self::Bool(a), Self::Bool(b)) => Self::Bool(a == b),
            _ => Self::Bool(false),
        }
    }
}
