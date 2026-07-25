use std::collections::HashMap;

use crate::interpreter::RuntimeError;
use crate::{interpreter::LoxValue, parser::IdentifierToken};

pub(crate) struct Environment {
    global: HashMap<String, LoxValue>,
    scopes: Vec<HashMap<String, LoxValue>>,
}

impl Environment {
    pub(crate) fn new() -> Self {
        Environment {
            global: HashMap::new(),
            scopes: vec![],
        }
    }

    fn innermost(&mut self) -> &mut HashMap<String, LoxValue> {
        if let Some(scope) = self.scopes.last_mut() {
            scope
        } else {
            &mut self.global
        }
    }

    /// Iterate over the scopes in reverse order (most nested first).
    fn iter_scopes(&self) -> impl Iterator<Item = &HashMap<String, LoxValue>> {
        self.scopes.iter().rev().chain(std::iter::once(&self.global))
    }

    /// Iterate over the scopes in reverse order (most nested first).
    fn iter_scopes_mut(&mut self) -> impl Iterator<Item = &mut HashMap<String, LoxValue>> {
        self.scopes.iter_mut().rev().chain(std::iter::once(&mut self.global))
    }

    pub(crate) fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(crate) fn exit_scope(&mut self) {
        debug_assert!(self.scopes.len() > 0);
        self.scopes.pop();
    }

    pub(crate) fn define(&mut self, name: &IdentifierToken, value: LoxValue) {
        self.innermost().insert(name.identifier().to_string(), value);
    }

    pub(crate) fn assign(&mut self, name: &IdentifierToken, value: LoxValue) -> Result<(), ()> {
        let key = name.identifier().to_string();
        for scope in self.iter_scopes_mut() {
            if scope.contains_key(&key) {
                scope.insert(key, value);
                return Ok(());
            }
        }
        Err(())
    }

    pub(crate) fn get(&self, name: &IdentifierToken) -> Result<LoxValue, RuntimeError> {
        for scope in self.iter_scopes() {
            if let Some(value) = scope.get(name.identifier()) {
                return Ok(value.clone()); // TODO: Can we avoid clone here?
            }
        }

        Err(RuntimeError::new(name, "Unknown variable".to_string()))
    }
}
