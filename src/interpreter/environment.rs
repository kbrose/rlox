use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::interpreter::RuntimeError;
use crate::interpreter::lox_object::NativeFunction;
use crate::{interpreter::lox_object::LoxObject, parser::IdentifierToken};

pub(crate) type EnvironmentWrapper = Rc<RefCell<Environment>>;

#[derive(Clone, Debug)]
pub(crate) struct Environment {
    scope: HashMap<String, LoxObject>,
    child: Option<EnvironmentWrapper>,
}

pub(super) fn new_scope(environment: &EnvironmentWrapper) -> EnvironmentWrapper {
    Rc::new(RefCell::new(Environment {
        scope: HashMap::new(),
        child: Some(Rc::clone(environment)),
    }))
}

impl Environment {
    pub(crate) fn new_root() -> Self {
        let mut env = Environment {
            scope: HashMap::new(),
            child: None,
        };
        env.define(
            &IdentifierToken::new("clock".to_string(), 0),
            LoxObject::NativeFunction(NativeFunction::Clock),
        );
        env
    }

    pub(crate) fn define(&mut self, name: &IdentifierToken, value: LoxObject) {
        self.scope.insert(name.identifier().to_string(), value);
    }

    pub(crate) fn assign(&mut self, name: &IdentifierToken, value: LoxObject) -> Result<(), ()> {
        let key = name.identifier().to_string();
        self.assign_key_value(key, value)
    }

    fn assign_key_value(&mut self, key: String, value: LoxObject) -> Result<(), ()> {
        if self.scope.contains_key(&key) {
            self.scope.insert(key, value);
            Ok(())
        } else if let Some(child) = &mut self.child {
            child.borrow_mut().assign_key_value(key, value)
        } else {
            Err(())
        }
    }

    pub(crate) fn get(&self, name: &IdentifierToken) -> Result<LoxObject, RuntimeError> {
        let key = name.identifier();
        if let Some(value) = self.scope.get(key) {
            return Ok(value.clone()); // TODO: Can we avoid clone?
        } else if let Some(child) = &self.child {
            child.borrow().get(name)
        } else {
            Err(RuntimeError::new(name, "Unknown variable".to_string()))
        }
    }

    // fn ancestor(&self, distance: usize) -> EnvironmentWrapper {
    //     if distance == 0 {
    //         Rc::new(RefCell::new(self))
    //     }
    // }
}

pub(crate) fn get_from_env_at(
    mut environment: Rc<RefCell<Environment>>,
    name: &IdentifierToken,
    distance: usize,
) -> Result<LoxObject, RuntimeError> {
    let key = name.identifier();

    for _ in 0..distance {
        environment = {
            let borrow = environment.borrow();
            borrow.child.as_ref().expect("distance does not match actual scope depth?").clone()
        };
    }

    match environment.borrow().scope.get(key) {
        Some(value) => Ok(value.clone()), // TODO: Can we avoid clone?
        None => Err(RuntimeError::new(name, "Unknown variable".to_string())),
    }
}

pub(crate) fn set_from_env_at(
    mut environment: Rc<RefCell<Environment>>,
    name: &IdentifierToken,
    value: LoxObject,
    distance: usize,
) {
    let key = name.identifier();

    for _ in 0..distance {
        environment = {
            let borrow = environment.borrow();
            borrow.child.as_ref().expect("distance does not match actual scope depth?").clone()
        };
    }

    environment.borrow_mut().scope.insert(key.to_string(), value);
}
