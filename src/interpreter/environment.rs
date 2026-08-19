use std::collections::HashMap;

use crate::interpreter::RuntimeError;
use crate::{interpreter::lox_object::LoxObject, parser::IdentifierToken};

#[derive(Clone)]
pub(crate) struct Environment {
    global: HashMap<String, LoxObject>,
    scopes: Vec<HashMap<String, LoxObject>>,
}

impl Environment {
    pub(crate) fn new() -> Self {
        Environment {
            global: HashMap::new(),
            scopes: vec![],
        }
    }

    pub(crate) fn new_with_global(global: HashMap<String, LoxObject>) -> Self {
        Environment {
            global,
            scopes: vec![],
        }
    }

    pub(super) fn global(&self) -> HashMap<String, LoxObject> {
        self.global.clone()
    }

    fn innermost(&mut self) -> &mut HashMap<String, LoxObject> {
        if let Some(scope) = self.scopes.last_mut() {
            scope
        } else {
            &mut self.global
        }
    }

    /// Iterate over the scopes in reverse order (most nested first).
    fn iter_scopes(&self) -> impl Iterator<Item = &HashMap<String, LoxObject>> {
        self.scopes.iter().rev().chain(std::iter::once(&self.global))
    }

    /// Iterate over the scopes in reverse order (most nested first).
    fn iter_scopes_mut(&mut self) -> impl Iterator<Item = &mut HashMap<String, LoxObject>> {
        self.scopes.iter_mut().rev().chain(std::iter::once(&mut self.global))
    }

    pub(crate) fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(crate) fn exit_scope(&mut self) {
        debug_assert!(self.scopes.len() > 0);
        self.scopes.pop();
    }

    pub(crate) fn define(&mut self, name: &IdentifierToken, value: LoxObject) {
        self.innermost().insert(name.identifier().to_string(), value);
    }

    pub(crate) fn assign(&mut self, name: &IdentifierToken, value: LoxObject) -> Result<(), ()> {
        let key = name.identifier().to_string();
        for scope in self.iter_scopes_mut() {
            if scope.contains_key(&key) {
                scope.insert(key, value);
                return Ok(());
            }
        }
        Err(())
    }

    pub(crate) fn get(&self, name: &IdentifierToken) -> Result<LoxObject, RuntimeError> {
        for scope in self.iter_scopes() {
            if let Some(value) = scope.get(name.identifier()) {
                return Ok(value.clone()); // TODO: Can we avoid clone here?
            }
        }

        Err(RuntimeError::new(name, "Unknown variable".to_string()))
    }
}
