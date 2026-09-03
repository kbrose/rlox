pub(crate) enum Value {
    Number(f64),
}

impl Value {
    pub(crate) fn print(self: &Self) {
        match self {
            Self::Number(x) => print!("'{x}'"),
        }
    }
}
