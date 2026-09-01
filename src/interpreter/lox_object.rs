use crate::ast::Function;
use crate::interpreter::callables::{ArgLengthMismatch, LoxCallable};
use crate::interpreter::interpreter::EvaluationException;
use crate::interpreter::{EnvironmentWrapper, Interpreter, RuntimeError};
use crate::parser::IdentifierToken;
use std::cell::RefCell;
use std::collections::HashMap;
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
    LoxClass(Rc<LoxClass>),                // Each instance has a reference to the class
    LoxInstance(Rc<RefCell<LoxInstance>>), // Instances are (the only?) mutable objects
    LoxMethod(Rc<LoxFunction>), // Methods are like functions except we need multiple references to them
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
            LoxObject::LoxClass(_) => Err(String::from("Type error: expected Number, found Class")),
            LoxObject::LoxInstance(_) => Err(String::from("Type error: expected Number, found Instance")),
            LoxObject::LoxMethod(_) => Err(String::from("Type error: expected Number, found Method")),
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
            LoxObject::LoxClass(_) => Err(String::from("Type error: expected String, found Class")),
            LoxObject::LoxInstance(_) => Err(String::from("Type error: expected String, found Instance")),
            LoxObject::LoxMethod(_) => Err(String::from("Type error: expected String, found Method")),
        }
    }

    pub(crate) fn get_callable<W: Write>(&self) -> Result<Box<dyn LoxCallable<W>>, String> {
        match self {
            Self::Nil => Err(String::from("Type error: expected Callable, found Nil")),
            Self::Boolean(_) => Err(String::from("Type error: expected Callable, found Boolean")),
            Self::Number(_) => Err(String::from("Type error: expected Callable, found Number")),
            Self::String(_) => Err(String::from("Type error: expected Callable, found String")),
            Self::LoxInstance(_) => Err(String::from("Type error: expected Callable, found Instance")),
            // TODO: Avoid clone
            Self::LoxClass(lox_class) => Ok(Box::new(lox_class.clone())),
            Self::NativeFunction(native_function) => Ok(Box::new(native_function.clone())),
            Self::LoxFunction(lox_function) => Ok(Box::new(lox_function.clone())),
            Self::LoxMethod(lox_function) => Ok(Box::new(lox_function.clone())),
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
            Self::LoxClass(lox_class) => lox_class.name.clone(),
            Self::LoxInstance(lox_instance) => lox_instance.borrow().to_string(),
            Self::LoxMethod(fun) => <LoxFunction as LoxCallable<W>>::to_string(fun),
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
        _parsed_args: Vec<(&IdentifierToken, Rc<LoxObject>)>,
    ) -> Result<Rc<LoxObject>, EvaluationException> {
        match self {
            Self::Clock => {
                let now = SystemTime::now();
                let epoch = SystemTime::UNIX_EPOCH;
                // TODO: How best to handle possible error here?
                Ok(match now.duration_since(epoch) {
                    Ok(duration) => Rc::new(LoxObject::Number(duration.as_secs_f64())),
                    Err(_) => Rc::new(LoxObject::Number(0.0)),
                })
            }
        }
    }

    fn align_arguments(
        self: &Self,
        arguments: Vec<Rc<LoxObject>>,
    ) -> Result<Vec<(&IdentifierToken, Rc<LoxObject>)>, ArgLengthMismatch> {
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
    is_initializer: bool,
}

impl PartialEq for LoxFunction {
    fn eq(&self, other: &Self) -> bool {
        self.declaration == other.declaration
    }
}

impl LoxFunction {
    pub(crate) fn new(declaration: Function, closure: EnvironmentWrapper, is_initializer: bool) -> Self {
        Self {
            declaration,
            closure,
            is_initializer,
        }
    }

    pub(super) fn bind(&self, instance: &Rc<RefCell<LoxInstance>>) -> LoxFunction {
        let closure = super::environment::new_scope(&Rc::clone(&self.closure));
        // So much extra rc'ing / refcelling / cloning here...
        closure.borrow_mut().define(
            &IdentifierToken::new("this".to_string(), 0),
            &Rc::new(LoxObject::LoxInstance(instance.clone())),
        );

        LoxFunction {
            declaration: self.declaration.clone(),
            closure,
            is_initializer: self.is_initializer,
        }
    }

    fn call_implementation<W: Write>(
        self: &Self,
        interpreter: &mut Interpreter<W>,
        parsed_args: Vec<(&IdentifierToken, Rc<LoxObject>)>,
    ) -> Result<Rc<LoxObject>, EvaluationException> {
        let env = super::environment::new_scope(&Rc::clone(&self.closure));

        // We have verified their lengths match elsewhere.
        // TODO: Any way to use "parse don't validate" here?
        for (param_identifier, param_value) in parsed_args.into_iter() {
            env.borrow_mut().define(param_identifier, &param_value);
        }

        let previous_environment = Rc::clone(&interpreter.environment);
        interpreter.environment = env.clone();
        // TODO: Avoid clone?
        for statement in self.declaration.body.clone().into_iter() {
            match interpreter.execute_stmt(statement) {
                Ok(_) => {}
                Err(e) => match e {
                    EvaluationException::Return(_) => {
                        let out = if self.is_initializer {
                            Err(EvaluationException::Return(
                                env.borrow()
                                    .get(&IdentifierToken::new("this".to_string(), 0))
                                    .expect("We should be in a method (the initializer), so 'this' should always exist!"),
                            ))
                        } else {
                            Err(e)
                        };
                        interpreter.environment = previous_environment;
                        return out;
                    }
                    EvaluationException::RuntimeError(_) => {
                        interpreter.environment = previous_environment;
                        return Err(e);
                    }
                },
            };
        }
        if self.is_initializer {
            let out = Err(EvaluationException::Return(
                env.borrow()
                    .get(&IdentifierToken::new("this".to_string(), 0))
                    .expect("We should be in a method (the initializer), so 'this' should always exist!"),
            ));
            interpreter.environment = previous_environment;
            return out;
        }
        interpreter.environment = previous_environment;

        Ok(Rc::new(LoxObject::Nil))
    }
}

impl<W: Write> LoxCallable<W> for LoxFunction {
    fn call(
        self: &Self,
        interpreter: &mut Interpreter<W>,
        parsed_args: Vec<(&IdentifierToken, Rc<LoxObject>)>,
    ) -> Result<Rc<LoxObject>, EvaluationException> {
        self.call_implementation(interpreter, parsed_args)
    }

    fn align_arguments(
        self: &Self,
        arguments: Vec<Rc<LoxObject>>,
    ) -> Result<Vec<(&IdentifierToken, Rc<LoxObject>)>, ArgLengthMismatch> {
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

impl<W: Write> LoxCallable<W> for Rc<LoxFunction> {
    fn call(
        self: &Self,
        interpreter: &mut Interpreter<W>,
        parsed_args: Vec<(&IdentifierToken, Rc<LoxObject>)>,
    ) -> Result<Rc<LoxObject>, EvaluationException> {
        self.call_implementation(interpreter, parsed_args)
    }

    fn align_arguments(
        self: &Self,
        arguments: Vec<Rc<LoxObject>>,
    ) -> Result<Vec<(&IdentifierToken, Rc<LoxObject>)>, ArgLengthMismatch> {
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

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct LoxClass {
    name: String,
    superclass: Option<Rc<LoxClass>>,
    methods: HashMap<String, Rc<LoxFunction>>,
}

impl LoxClass {
    pub(crate) fn new(
        name: String,
        superclass: Option<Rc<LoxClass>>,
        methods: HashMap<String, LoxFunction>,
    ) -> Self {
        let methods = methods.into_iter().map(|(key, value)| (key, Rc::new(value))).collect();
        Self {
            name,
            superclass,
            methods,
        }
    }

    pub(super) fn find_method(&self, name: &str) -> Option<&Rc<LoxFunction>> {
        if let Some(method) = self.methods.get(name) {
            Some(method)
        } else if let Some(superclass) = &self.superclass {
            superclass.find_method(name)
        } else {
            None
        }
    }
}

impl<W: Write> LoxCallable<W> for Rc<LoxClass> {
    fn call(
        self: &Self,
        interpreter: &mut Interpreter<W>,
        parsed_args: Vec<(&IdentifierToken, Rc<LoxObject>)>,
    ) -> Result<Rc<LoxObject>, EvaluationException> {
        let instance = Rc::new(RefCell::new(LoxInstance::new(self)));

        match self.find_method("init") {
            Some(initializer) => {
                initializer.bind(&instance).call(interpreter, parsed_args)?;
            }
            None => {}
        }

        Ok(Rc::new(LoxObject::LoxInstance(instance)))
    }

    fn align_arguments(
        self: &Self,
        arguments: Vec<Rc<LoxObject>>,
    ) -> Result<Vec<(&IdentifierToken, Rc<LoxObject>)>, ArgLengthMismatch> {
        match self.find_method("init") {
            Some(initializer) => {
                // TODO
                LoxCallable::<W>::align_arguments(initializer.as_ref(), arguments)
            }
            None => {
                if arguments.len() > 0 {
                    Err(ArgLengthMismatch::new(0, arguments.len()))
                } else {
                    Ok(vec![])
                }
            }
        }
    }

    fn to_string(self: &Self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct LoxInstance {
    class: Rc<LoxClass>,
    fields: HashMap<String, Rc<LoxObject>>,
}

impl LoxInstance {
    pub(crate) fn new(class: &Rc<LoxClass>) -> LoxInstance {
        LoxInstance {
            class: class.clone(),
            fields: HashMap::new(),
        }
    }

    pub(crate) fn to_string(self: &Self) -> String {
        format!("{} instance", self.class.name)
    }
}

pub(crate) fn get_from_instance(
    instance: &Rc<RefCell<LoxInstance>>,
    name: &IdentifierToken,
) -> Result<Rc<LoxObject>, RuntimeError> {
    let identifier = name.identifier();
    match instance.borrow().fields.get(identifier) {
        Some(object) => Ok(object.clone()),
        None => {
            let borrowed_instance = instance.borrow();
            let out = borrowed_instance.class.find_method(identifier);
            match out {
                Some(function) => {
                    Ok(Rc::new(LoxObject::LoxMethod(Rc::new(function.bind(instance)))))
                    // Ok(Rc::new(LoxObject::LoxMethod(function.clone())))
                }
                None => Err(RuntimeError::new(name, format!("Undefined property '{}'.", name.identifier()))),
            }
        }
    }
}

pub(crate) fn set_from_instance(
    instance: &Rc<RefCell<LoxInstance>>,
    name: &IdentifierToken,
    value: &Rc<LoxObject>,
) {
    instance.borrow_mut().fields.insert(name.identifier().to_string(), value.clone());
}
