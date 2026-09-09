use std::fmt::Display;

use crate::value::Obj;

/// A "heap" that will own allocations of dynamically sized lox objects.
pub(crate) struct ObjHeap {
    objects: Vec<Obj>,
    // next_free_space: Option<ObjHeapIndex>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ObjHeapIndex(usize);

impl Display for ObjHeapIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("ObjHeapIndex({})", self.0))
    }
}

impl ObjHeap {
    pub(crate) fn new() -> Self {
        Self {
            objects: Vec::new(),
            // next_free_space: None,
        }
    }

    pub(crate) fn allocate(&mut self, obj: Obj) -> ObjHeapIndex {
        // TODO: Garbage collection will change where we allocate
        self.objects.push(obj);
        ObjHeapIndex(self.objects.len() - 1)
    }

    pub(crate) unsafe fn get_unchecked(&self, idx: ObjHeapIndex) -> &Obj {
        unsafe { self.objects.get_unchecked(idx.0) }
    }
}
