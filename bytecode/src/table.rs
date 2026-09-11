use crate::value::{
    Obj, Value,
    heap::{TypedHeap, TypedHeapIndex},
};

#[derive(Clone, Copy)]
struct Entry {
    key_pointer: TypedHeapIndex<Obj>,
    value: Value,
}

impl Entry {
    fn to_table_element(self) -> TableElement {
        TableElement::MaybeEntry(Some(self))
    }
}

#[derive(Clone, Copy)]
enum TableElement {
    MaybeEntry(Option<Entry>),
    Tombstone,
}

pub(crate) struct Table {
    entries: Vec<TableElement>,
    count: usize,
}

impl Table {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
            count: 0,
        }
    }

    // Delete the key from the table. Return true if a deletion
    // was necessary (the key existed), otherwise false.
    #[allow(unused)]
    pub(crate) fn delete(
        &mut self,
        key_pointer: TypedHeapIndex<Obj>,
        heap: &TypedHeap<Obj>,
    ) -> bool {
        if self.count == 0 {
            false
        } else {
            let (index, slot_kind) = find_slot(&self.entries, key_pointer, heap);
            if slot_kind == SlotKind::Empty || slot_kind == SlotKind::Tombstone {
                false
            } else {
                self.entries[index] = TableElement::Tombstone;
                true
            }
        }
    }

    #[allow(unused)]
    pub(crate) fn get(
        &self,
        key_pointer: TypedHeapIndex<Obj>,
        heap: &TypedHeap<Obj>,
    ) -> Option<Value> {
        if self.entries.len() == 0 {
            None
        } else {
            let (index, _) = find_slot(&self.entries, key_pointer, heap);
            match self.entries[index] {
                TableElement::MaybeEntry(entry) => entry.map(|e| e.value),
                TableElement::Tombstone => None,
            }
        }
    }

    pub(crate) fn get_key_from_str(
        &self,
        key: &str,
        hash: u32,
        heap: &TypedHeap<Obj>,
    ) -> Option<TypedHeapIndex<Obj>> {
        if self.entries.len() == 0 {
            None
        } else {
            let (index, _) = find_slot_from_str(&self.entries, key, hash, heap);
            match self.entries[index] {
                TableElement::MaybeEntry(entry) => entry.map(|e| e.key_pointer),
                TableElement::Tombstone => None,
            }
        }
    }

    /// Create an entry for the key, value pair. Return
    /// true if this is a new key, otherwise false.
    pub(crate) fn set(
        &mut self,
        key_pointer: TypedHeapIndex<Obj>,
        value: Value,
        heap: &TypedHeap<Obj>,
    ) -> bool {
        let len = self.entries.len();
        // if count + 1 > capacity * 0.75, rewritten into whole-number form
        if (self.count + 1) * 4 > len * 3 {
            self.adjust_capacity((len * 2).max(8), heap);
        }

        let (index, slot_kind) = find_slot(&self.entries, key_pointer, heap);
        self.entries[index] = Entry { key_pointer, value }.to_table_element();
        if slot_kind == SlotKind::Empty {
            self.count += 1;
        }
        slot_kind == SlotKind::Empty || slot_kind == SlotKind::Tombstone
    }

    fn adjust_capacity(&mut self, new_capacity: usize, heap: &TypedHeap<Obj>) {
        debug_assert!(new_capacity > self.entries.len());
        // Initialize the new entries as a vec full of None.
        // But wait! Why does this say old_entries if it's the new ones?
        // Well! It's because we immediately swap it out for self.entries.
        // That way we can iterate over the old_entries in an owned fashion
        // without making the borrow checker mad.
        let mut old_entries = vec![TableElement::MaybeEntry(None); new_capacity];
        std::mem::swap(&mut self.entries, &mut old_entries);

        self.count = 0;
        for table_element in old_entries.into_iter() {
            match table_element {
                TableElement::MaybeEntry(Some(entry)) => {
                    let (index, _) = find_slot(&self.entries, entry.key_pointer, heap);
                    self.entries[index] = entry.to_table_element();
                    self.count += 1;
                }
                TableElement::MaybeEntry(None) | TableElement::Tombstone => {
                    // Do NOT copy empty spaces / tombstones.
                }
            }
        }
    }

    // pub(crate) fn add_to_this_table(&mut self, other: &Table, heap: &TypedHeap<Obj>) {
    //     for table_element in other.entries.iter() {
    //         match table_element {
    //             TableElement::MaybeEntry(Some(Entry {
    //                 key_pointer: key,
    //                 value,
    //             })) => {
    //                 self.set(*key, *value, heap);
    //             }
    //             TableElement::MaybeEntry(None) | TableElement::Tombstone => {
    //                 // Do NOT copy empty spaces / tombstones.
    //             }
    //         }
    //     }
    // }
}

#[derive(PartialEq)]
enum SlotKind {
    Empty,
    Tombstone,
    Full,
}

/// Find the slot for the given key, returning (index, is_new_key):
/// the index where the key exists, or where it _should_ exist,
/// along with a boolean indicating which scenario we are in.
fn find_slot(
    entries: &[TableElement],
    key_pointer: TypedHeapIndex<Obj>,
    heap: &TypedHeap<Obj>,
) -> (usize, SlotKind) {
    let len = entries.len();
    debug_assert!(len > 0, "This function assumes entries is non-empty!");
    let key = unsafe { heap.get_unchecked_objstr(key_pointer) };
    let mut index = (key.hash() as usize) % len;
    let mut first_observed_tombstone: Option<usize> = None;
    loop {
        let table_element = unsafe { entries.get_unchecked(index) };
        match table_element {
            TableElement::MaybeEntry(Some(entry)) => {
                if entry.key_pointer == key_pointer {
                    break (index, SlotKind::Full);
                }
            }
            TableElement::MaybeEntry(None) => match first_observed_tombstone {
                Some(tombstone_index) => break (tombstone_index, SlotKind::Tombstone),
                None => break (index, SlotKind::Empty),
            },
            TableElement::Tombstone => {
                if first_observed_tombstone.is_none() {
                    first_observed_tombstone = Some(index);
                }
                // Keep iterating.
            }
        }
        index = (index + 1) % len;
    }
}

/// Find the slot for the given key, returning (index, is_new_key):
/// the index where the key exists, or where it _should_ exist,
/// along with a boolean indicating which scenario we are in.
fn find_slot_from_str(
    entries: &[TableElement],
    key: &str,
    hash: u32,
    heap: &TypedHeap<Obj>,
) -> (usize, SlotKind) {
    let len = entries.len();
    debug_assert!(len > 0, "This function assumes entries is non-empty!");
    let mut index = (hash as usize) % len;
    let mut first_observed_tombstone: Option<usize> = None;
    loop {
        let table_element = unsafe { entries.get_unchecked(index) };
        match table_element {
            TableElement::MaybeEntry(Some(entry)) => {
                let entry_obj_str = unsafe { heap.get_unchecked_objstr(entry.key_pointer) };
                if hash == entry_obj_str.hash() && entry_obj_str.string() == key {
                    break (index, SlotKind::Full);
                }
            }
            TableElement::MaybeEntry(None) => match first_observed_tombstone {
                Some(tombstone_index) => break (tombstone_index, SlotKind::Tombstone),
                None => break (index, SlotKind::Empty),
            },
            TableElement::Tombstone => {
                if first_observed_tombstone.is_none() {
                    first_observed_tombstone = Some(index);
                }
                // Keep iterating.
            }
        }
        index = (index + 1) % len;
    }
}

#[cfg(test)]
mod tests {
    use crate::value::heap::Heap;

    use super::*;

    fn get_obj_str_pointer(s: &str, heap: &mut Heap) -> TypedHeapIndex<Obj> {
        let value = Value::new_string(s.to_string(), heap);

        match value {
            Value::Nil => panic!("Expected Value::Obj"),
            Value::Number(_) => panic!("Expected Value::Obj"),
            Value::Bool(_) => panic!("Expected Value::Obj"),
            Value::Obj(typed_heap_index) => typed_heap_index,
        }
    }

    #[test]
    fn test_get_on_empty() {
        let table = Table::new();
        let mut heap = Heap::new();

        let key_pointer = get_obj_str_pointer("key", &mut heap);

        assert!(table.get(key_pointer, heap.object_heap()).is_none());
        assert_eq!(table.count, 0);
    }

    #[test]
    fn test_round_trips() {
        let mut table = Table::new();
        let mut heap = Heap::new();

        let key_pointer = get_obj_str_pointer("First", &mut heap);
        let value = Value::Nil;

        table.set(key_pointer, value.clone(), heap.object_heap());
        assert!(value.is_equal_raw(&table.get(key_pointer, heap.object_heap()).unwrap(), &heap));
        assert_eq!(table.count, 1);
    }

    #[test]
    fn test_many_insertions() {
        let mut table = Table::new();
        let mut heap = Heap::new();

        for i in 0..100_000 {
            let key_pointer = get_obj_str_pointer(&format!("{i}"), &mut heap);
            table.set(key_pointer, Value::Number(i as f64), heap.object_heap());
            assert!(
                Value::Number(i as f64)
                    .is_equal_raw(&table.get(key_pointer, heap.object_heap()).unwrap(), &heap)
            );
        }

        assert_eq!(table.count, 100_000);
    }

    #[test]
    fn test_many_insertions_and_deletions() {
        let mut table = Table::new();
        let mut heap = Heap::new();

        for i in 0..100_000 {
            let key = get_obj_str_pointer(&format!("{i}"), &mut heap);
            table.set(key, Value::Number(i as f64), heap.object_heap());
            assert!(
                Value::Number(i as f64)
                    .is_equal_raw(&table.get(key, heap.object_heap()).unwrap(), &heap)
            );
            table.delete(key, heap.object_heap());
        }

        // This one is tricky: we're relying on a somewhat unreliable fact:
        // we will reuse _some_ tombstones, but will not _only_ reuse the exact same one.
        assert!(0 < table.count);
        assert!(table.count < 100_000);
    }
}
