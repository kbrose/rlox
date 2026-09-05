use crate::value::Value;

#[cfg(not(feature = "unsafe_stack"))]
pub(super) struct Stack {
    stack: Vec<Value>,
}

#[cfg(not(feature = "unsafe_stack"))]
impl Stack {
    pub(super) fn new(initial_capacity: usize) -> Self {
        Stack {
            stack: Vec::with_capacity(initial_capacity),
        }
    }

    pub(super) fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    pub(super) fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    #[inline]
    pub(super) fn apply_to_top(&mut self, f: impl Fn(Value) -> Value) {
        let idx = self.stack.len() - 1;
        self.stack[idx] = f(self.stack[idx]);
    }

    pub(super) fn stack(&self) -> &Vec<Value> {
        &self.stack
    }
}

// Below is a more direct implementation of the book's version of the stack using just
// pointer manipulation with no bounds checking to be found. At least so far,
// I cannot measure any noticeable impact.

#[cfg(feature = "unsafe_stack")]
pub(super) struct Stack {
    stack_top: *mut Value,
    _storage: Vec<Value>, // Owns the buffer and handles automatic cleanup (Drop)
}

#[cfg(feature = "unsafe_stack")]
impl Stack {
    pub(super) fn new(initial_capacity: usize) -> Self {
        let mut storage = Vec::with_capacity(initial_capacity);
        let stack_top = storage.as_mut_ptr();
        Self {
            stack_top,
            _storage: storage,
        }
    }

    #[inline]
    pub(super) fn push(&mut self, value: Value) {
        unsafe {
            // SAFETY: Relies on the assumption that _storage has necessary capacity.
            self.stack_top.write(value);
            self.stack_top = self.stack_top.add(1);
        }
    }

    #[inline]
    pub(super) fn pop(&mut self) -> Value {
        unsafe {
            // SAFETY: Assumes the stack is NOT empty
            self.stack_top = self.stack_top.sub(1);
            self.stack_top.read()
        }
    }

    #[inline]
    pub(super) fn apply_to_top(&mut self, f: impl Fn(Value) -> Value) {
        unsafe {
            // SAFETY: Assumes the stack is NOT emptpy
            let head = self.stack_top.sub(1);
            head.write(f(head.read()));
        }
    }

    pub(super) fn stack(&self) -> &Vec<Value> {
        todo!()
    }
}
