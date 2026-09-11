use std::io::Write;

pub(crate) mod heap;
use heap::{Heap, TypedHeapIndex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Value {
    Nil,
    Number(f64),
    Bool(bool),
    Obj(TypedHeapIndex<Obj>),
}

impl Value {
    pub(crate) fn new_number(x: f64) -> Self {
        Self::Number(x)
    }

    pub(crate) fn new_bool(b: bool) -> Self {
        Self::Bool(b)
    }

    pub(crate) fn new_string(s: String, heap: &mut Heap) -> Self {
        heap.allocate_string(s)
    }

    #[inline]
    pub(crate) fn as_number(&self) -> Result<f64, ()> {
        match self {
            Self::Number(x) => Ok(*x),
            _ => Err(()),
        }
    }

    pub(crate) fn debug_print<W: Write>(self: &Self, heap: &Heap, writer: &mut W) {
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
            Self::Obj(index) => {
                let obj = unsafe { heap.get_unchecked(*index) };
                match &obj.obj_type {
                    ObjType::String(obj_string) => {
                        write!(writer, "{}", obj_string.string()).expect("Error writing debug.");
                    }
                }
            }
        }
    }

    pub(crate) fn is_falsey(&self) -> Value {
        match self {
            Self::Nil | Self::Bool(false) => Self::Bool(true),
            _ => Self::Bool(false),
        }
    }

    pub(crate) fn is_equal(&self, other: &Value, heap: &Heap) -> Value {
        Self::Bool(self.is_equal_raw(other, heap))
    }

    pub(crate) fn is_equal_raw(&self, other: &Value, heap: &Heap) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Obj(index1), Self::Obj(index2)) => {
                // NOTE: We could compare index1 vs index2 here first, if they are
                // equal then the objects are equal (point to the exact same object).
                // However, my intuition says that most equality checks will not be
                // between the exact same object. I could be wrong, though!
                let (a, b) = unsafe { (heap.get_unchecked(*index1), heap.get_unchecked(*index2)) };
                match (&a.obj_type, &b.obj_type) {
                    // Because every string is interned, it's sufficient to check if the two
                    // pointers are equal (in the case of string equality).
                    (ObjType::String(_), ObjType::String(_)) => index1 == index2,
                    // (ObjType::String(s1), ObjType::String(s2)) => s1.string() == s2.string(),
                }
            }
            _ => false,
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

/// An "Object String". Note: because of string interning, we will never have two distinct
/// `ObjStr` instances that have the same string contents.
#[derive(Debug, Clone)]
pub(crate) struct ObjStr {
    string: String,
    hash: u32,
}

impl ObjStr {
    #[inline]
    pub(crate) fn string(&self) -> &str {
        &self.string
    }

    #[inline]
    pub(crate) fn hash(&self) -> u32 {
        self.hash
    }
}

fn hash_of_str(s: &str) -> u32 {
    let mut hash: u32 = 2166136261;
    for b in s.bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

#[derive(Debug, Clone)]
pub(crate) enum ObjType {
    String(ObjStr),
}

#[derive(Debug, Clone)]
pub(crate) struct Obj {
    obj_type: ObjType,
}

impl Obj {
    pub(crate) fn obj_type(&self) -> &ObjType {
        &self.obj_type
    }
}
