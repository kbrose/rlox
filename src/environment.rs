use std::collections::HashMap;

use crate::interpreter::RuntimeError;
use crate::{interpreter::LoxValue, parser::IdentifierToken};

pub(crate) struct Environment {
    values: HashMap<String, LoxValue>,
}

impl Environment {
    pub(crate) fn new() -> Self {
        Environment {
            values: HashMap::new(),
        }
    }

    pub(crate) fn define(&mut self, name: &IdentifierToken, value: LoxValue) {
        self.values.insert(name.identifier().to_string(), value);
    }

    pub(crate) fn get(&self, name: &IdentifierToken) -> Result<LoxValue, RuntimeError> {
        // TODO: Any way to avoid the clone here?
        self.values
            .get(name.identifier())
            .ok_or_else(|| RuntimeError::new(name.clone(), "Unknown variable".to_string()))
            .map(|x| x.clone())
    }
}
