use std::io::Write;

use crate::heap::{ObjHeap, ObjHeapIndex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Value {
    Nil,
    Number(f64),
    Bool(bool),
    Obj(ObjHeapIndex),
}

impl Value {
    pub(crate) fn new_number(x: f64) -> Self {
        Self::Number(x)
    }

    pub(crate) fn new_bool(b: bool) -> Self {
        Self::Bool(b)
    }

    pub(crate) fn new_string(s: &str, object_heap: &mut ObjHeap) -> Self {
        Self::Obj(object_heap.allocate(Obj::String(s.to_string())))
    }

    #[inline]
    pub(crate) fn as_number(&self) -> Result<f64, ()> {
        match self {
            Self::Number(x) => Ok(*x),
            _ => Err(()),
        }
    }

    pub(crate) fn debug_print<W: Write>(self: &Self, obj_heap: &ObjHeap, writer: &mut W) {
        match self {
            Self::Nil => {
                write!(writer, "Nil").expect("Error writing debug.");
            }
            Self::Number(x) => {
                write!(writer, "'{}'", x).expect("Error writing debug.");
            }
            Self::Bool(b) => {
                write!(writer, "{}", b).expect("Error writing debug.");
            }
            Self::Obj(index) => match unsafe { obj_heap.get_unchecked(*index) } {
                Obj::GarbageCollected(_) => {
                    write!(writer, "Garbage Collected: Should never be printed!")
                        .expect("Error writing debug.");
                }
                Obj::String(s) => {
                    write!(writer, "{s}").expect("Error writing debug.");
                }
            },
        }
    }

    pub(crate) fn is_falsey(&self) -> Value {
        match self {
            Self::Nil | Self::Bool(false) => Self::Bool(true),
            _ => Self::Bool(false),
        }
    }

    pub(crate) fn is_equal(&self, other: &Value, obj_heap: &ObjHeap) -> Value {
        match (self, other) {
            (Self::Nil, Self::Nil) => Self::Bool(true),
            (Self::Number(a), Self::Number(b)) => Self::Bool(a == b),
            (Self::Bool(a), Self::Bool(b)) => Self::Bool(a == b),
            (Self::Obj(index1), Self::Obj(index2)) => {
                // NOTE: We could compare index1 vs index2 here first, if they are
                // equal then the objects are equal (point to the exact same object).
                // However, my intuition says that most equality checks will not be
                // between the exact same object. I could be wrong, though!
                let (a, b) = unsafe {
                    (
                        obj_heap.get_unchecked(*index1),
                        obj_heap.get_unchecked(*index2),
                    )
                };
                match (a, b) {
                    (Obj::String(s1), Obj::String(s2)) => Self::Bool(s1 == s2),
                    (Obj::GarbageCollected(_), _) | (_, Obj::GarbageCollected(_)) => unreachable!(),
                }
            }
            _ => Self::Bool(false),
        }
    }

    // TODO: Can these two methods be removed? I'm not sure if I'll need them.
    // #[inline]
    // pub(crate) fn as_bool(&self) -> Result<bool, ()> {
    //     match self {
    //         Self::Bool(b) => Ok(*b),
    //         _ => Err(()),
    //     }
    // }

    // #[inline]
    // pub(crate) fn is_nil(&self) -> bool {
    //     self == &Value::Nil
    // }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Obj {
    /// Points to the next "free" (GarbageCollected) spot on the heap,
    /// if one exists.
    #[allow(unused)]
    GarbageCollected(Option<usize>),
    String(String),
}
