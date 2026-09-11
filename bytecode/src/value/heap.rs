use std::{fmt::Display, marker::PhantomData};

use crate::{
    table::Table,
    value::{Obj, ObjStr, ObjType, Value, hash_of_str},
};

/// A "heap" that will own allocations of type T.
pub(crate) struct TypedHeap<T> {
    objects: Vec<T>,
    // next_free_space: Option<TypedHeapIndex>,
}

#[derive(Debug)]
pub(crate) struct TypedHeapIndex<T> {
    idx: usize,
    _marker: PhantomData<T>,
}

impl<T> TypedHeapIndex<T> {
    fn new(idx: usize) -> Self {
        Self {
            idx,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for TypedHeapIndex<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for TypedHeapIndex<T> {}

impl<T> Display for TypedHeapIndex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("TypedHeapIndex({})", self.idx))
    }
}

impl<T> PartialEq for TypedHeapIndex<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}

impl<T> TypedHeap<T> {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            // next_free_space: None,
        }
    }

    fn allocate(&mut self, obj: T) -> TypedHeapIndex<T> {
        // TODO: Garbage collection will change where we allocate
        self.objects.push(obj);
        TypedHeapIndex::new(self.objects.len() - 1)
    }

    /// Get the entry located at index without any bounds checking.
    /// Then, check that it is indeed a string type, and if so return it.
    /// UB: if the passed index is out of bounds.
    unsafe fn get_unchecked(&self, idx: TypedHeapIndex<T>) -> &T {
        unsafe { self.objects.get_unchecked(idx.idx) }
    }
}

impl TypedHeap<Obj> {
    /// Get the entry located at index without any bounds checking.
    /// Then, assume it is an ObjStr, and return it.
    /// UB: if the passed index is out of bounds, or if the object at the index is NOT an ObjStr.
    pub(crate) unsafe fn get_unchecked_objstr(&self, idx: TypedHeapIndex<Obj>) -> &ObjStr {
        let obj = unsafe { self.get_unchecked(idx) };
        match obj.obj_type() {
            ObjType::String(obj_str) => obj_str,
            // To actually make good on our "undefined behavior" threat,
            // this should be the stronger form.
            // _ => unreachable!(),
        }
    }
}

pub(crate) struct Heap {
    objects: TypedHeap<Obj>,
    interned_strings: Table,
}

// A "heap" where we will allocate everything required for Lox VM.
// This exists to strike a balance between following the book exactly, which
// would require passing around pointers everywhere, vs. Using `Rc` and
// as our garbage collector and passing references to the `Rc` objects.
//
// Because this is managing our allocations, it also makes sense for this
// struct to own the string interning. So that happens here as well.
impl Heap {
    pub(crate) fn new() -> Self {
        Self {
            objects: TypedHeap::new(),
            interned_strings: Table::new(),
        }
    }

    #[allow(unused)]
    pub(crate) fn object_heap(&self) -> &TypedHeap<Obj> {
        &self.objects
    }

    pub(crate) fn allocate_string(&mut self, string: String) -> Value {
        let hash = hash_of_str(&string);
        match self
            .interned_strings
            .get_key_from_str(&string, hash, &self.objects)
        {
            Some(obj_pointer) => Value::Obj(obj_pointer),
            None => {
                let obj_str = ObjStr { string, hash };
                let obj_pointer = self.objects.allocate(Obj {
                    obj_type: ObjType::String(obj_str),
                });
                let out = Value::Obj(obj_pointer);
                self.interned_strings
                    .set(obj_pointer, Value::Nil, &self.objects);
                out
            }
        }
    }

    /// Get the entry located at index without any bounds checking.
    /// Then, check that it is indeed a string type, and if so return it.
    /// UB: if the passed index is out of bounds.
    pub(crate) unsafe fn get_unchecked(&self, idx: TypedHeapIndex<Obj>) -> &Obj {
        unsafe { self.objects.get_unchecked(idx) }
    }

    /// Get the entry located at index without any bounds checking.
    /// Then, assume it is an ObjStr, and return it.
    /// UB: if the passed index is out of bounds, or if the object at the index is NOT an ObjStr.
    #[allow(unused)]
    pub(crate) unsafe fn get_unchecked_objstr(&self, idx: TypedHeapIndex<Obj>) -> &ObjStr {
        unsafe { self.objects.get_unchecked_objstr(idx) }
    }

    // /// Get the entry located at index without any bounds checking.
    // /// Then, check that it is indeed a string type, and if so return it.
    // /// UB: if the passed index is out of bounds.
    // pub(crate) unsafe fn get_string_at_unchecked(&self, idx: TypedHeapIndex<String>) -> &str {
    //     unsafe { self.strings.get_unchecked(idx) }
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_index_from_allocation(value: Value) -> TypedHeapIndex<Obj> {
        match value {
            Value::Obj(typed_heap_index) => typed_heap_index,
            _ => panic!("Expected Value::Obj"),
        }
    }

    #[test]
    fn test_simple_allocation() {
        let mut heap = Heap::new();
        heap.allocate_string("Abc".to_string());
    }

    #[test]
    fn test_round_trip() {
        let s = "Abc";
        let mut heap = Heap::new();
        let pointer = get_index_from_allocation(heap.allocate_string(s.to_string()));
        let obj = unsafe { heap.get_unchecked(pointer) };
        match obj.obj_type() {
            ObjType::String(obj_str) => assert_eq!(obj_str.string(), s),
        }
        let obj_str = unsafe { heap.get_unchecked_objstr(pointer) };
        assert_eq!(obj_str.string(), s);
    }

    #[test]
    fn test_string_interning_deduplicates() {
        let mut heap = Heap::new();
        let ptr1 = get_index_from_allocation(heap.allocate_string("Abc".to_string()));
        let ptr2 = get_index_from_allocation(heap.allocate_string("Def".to_string()));
        let ptr3 = get_index_from_allocation(heap.allocate_string("Abc".to_string()));

        assert_eq!(ptr1, ptr3);
        assert!(ptr2 != ptr1);
    }
}
