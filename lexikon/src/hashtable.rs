//! C-style chaining hashtable implementation
use crate::container_of;
use core::alloc::Layout;
use std::alloc::alloc_zeroed;
use std::fmt;

const DEFAULT_HASH_TABLE_INIT_SIZE: usize = 4;

#[derive(Debug)]
pub struct HashTable {
    _inner: HashMap,
}

impl Default for HashTable {
    fn default() -> Self {
        Self::new(DEFAULT_HASH_TABLE_INIT_SIZE)
    }
}

impl HashTable {
    pub fn new(size: usize) -> Self {
        Self {
            _inner: HashMap::init(size).expect("Internal failure to create a `HashMap`"),
        }
    }

    pub fn insert(&mut self, key: String, value: String) -> Result<(), HashMapError> {
        // TODO: Replace with a real hash function
        let hash = key.len() as u64;

        let entry: *mut Entry = Box::into_raw(Box::new(Entry {
            node: HNode::from_hash(hash),
            key,
            value,
        }));

        unsafe { self._inner.insert(&mut (*entry).node as *mut HNode) }
    }

    pub fn iter(&self) -> HashTableIter<'_> {
        HashTableIter::new(&self)
    }

    pub fn get(&self) {}

    pub fn len(&self) -> usize {
        let mut len = self._inner.new.len();
        if let Some(old) = self._inner.old {
            len += old.len();
        }
        len
    }
}

pub struct HashTableIter<'a> {
    table: &'a HashTable,
    pos: usize,
    len: usize,
}

impl<'a> HashTableIter<'a> {
    pub fn new(ht: &'a HashTable) -> Self {
        Self {
            table: ht,
            pos: 0,
            len: ht.len(),
        }
    }
}

impl<'a> Iterator for HashTableIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

// KV pair with an embedded hashtable node.
#[derive(Default, Debug)]
#[repr(C)]
#[allow(unused)]
struct Entry {
    node: HNode,
    // Key and Value need to be generic
    key: String,
    value: String,
}

impl PartialEq for HNode {
    fn eq(&self, other: &Self) -> bool {
        let lhs = container_of!(self as *const HNode, Entry, node);
        let rhs = container_of!(other as *const HNode, Entry, node);
        if lhs.is_null() || rhs.is_null() {
            return false;
        }
        unsafe { (*lhs).key == (*rhs).key }
    }
}

impl Entry {}

#[derive(Default, Debug)]
pub struct HNode {
    // Reference to the next node
    next: *mut HNode,
    // Hash value
    hash: u64,
}

impl HNode {
    pub fn hash(&self) -> u64 {
        self.hash
    }

    pub fn from_hash(hash: u64) -> Self {
        Self {
            next: core::ptr::null::<HNode>() as *mut HNode,
            hash,
        }
    }

    /// Move self to heap and returns a raw pointer to it
    pub fn as_ptr(self) -> *const Self {
        Box::into_raw(Box::new(self))
    }

    /// Move self to heap and returns a raw mutable pointer to it
    pub fn as_mut_ptr(self) -> *mut Self {
        Box::into_raw(Box::new(self)) as *mut Self
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct InnerHashTable {
    // Pointer to the hashtable
    // Should this be a `Vec<Box<HNode>>`?
    tab: *mut *mut HNode,
    // Mask to map the hash according to our desired size
    mask: usize,
    // Number of keys currently in the table
    len: usize,
}

pub struct InnerHashTableIter<'a> {
    table: &'a InnerHashTable,
    pos: usize,
}

impl<'a> InnerHashTableIter<'a> {
    pub fn new(table: &'a InnerHashTable) -> Self {
        Self { table, pos: 0 }
    }
}

impl<'a> Iterator for InnerHashTableIter<'a> {
    type Item = &'a HNode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.table.tab.is_null() {
            return None;
        }

        // If we processed all the pointers, return None
        if self.pos >= self.table.len {
            return None;
        }

        let mut slot = 0;
        let mut next = 0;
        let mut node_ptr: *mut HNode = core::ptr::null::<HNode>() as *mut HNode;

        unsafe {
            // While we still have slots to process and we have not processed all the keys
            while next <= self.pos {
                let slot_ptr = self.table.tab.offset(slot);
                // If the current slot is empty or we reached its end, go to the next slot
                if slot_ptr.is_null() {
                    slot = slot.saturating_add(1);
                    continue;
                }

                node_ptr = *slot_ptr;
                while !node_ptr.is_null() && next <= self.pos {
                    next = next.saturating_add(1);
                    node_ptr = (*node_ptr).next;
                }

                println!("Next {:#?} node_ptr {:#?}", next, node_ptr);

                // We go to the next slot
                if next != self.pos {
                    slot = slot.saturating_add(1);
                }
            }
            // Update for the next iteration
            self.pos = self.pos.saturating_add(1);
            Some(&*node_ptr)
        }
    }
}

impl fmt::Display for InnerHashTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut idx = 0;
        let mut pos = 0isize;
        let tab_cursor = self.tab;
        if tab_cursor.is_null() {
            return Ok(());
        }

        while idx < self.len() && pos <= self.mask as isize {
            unsafe {
                let pos_ptr = tab_cursor.offset(pos);
                write!(f, "Slot {:#?}", pos_ptr)?;
                if pos_ptr.is_null() {
                    pos += 1;
                    continue;
                }

                let mut hash_ptr = *pos_ptr;
                while !hash_ptr.is_null() {
                    let elem = hash_ptr;
                    write!(
                        f,
                        "({:#?} -> {}) next: {:#?}\n",
                        elem as *mut u64,
                        (*elem).hash,
                        (*elem).next
                    )?;
                    hash_ptr = (*elem).next;
                    idx += 1;
                }
                pos += 1;
            }
        }

        Ok(())
    }
}

impl InnerHashTable {
    pub fn init(size: usize) -> Result<Self, InnerHashTableError> {
        // Make sure the size is not 0 or negative. `usize` prevents this but we are extra
        if size <= 0 {
            return Err(InnerHashTableError::NegativeSize);
        }
        // Make sure size is a power of 2
        if size - 1 & size != 0 {
            return Err(InnerHashTableError::SizeNotPowerOfTwo(size));
        }

        let layout = Layout::from_size_align(
            size * core::mem::size_of::<*const HNode>(),
            core::mem::size_of::<usize>(),
        )?;

        let tab = unsafe { alloc_zeroed(layout) };

        Ok(Self {
            tab: tab as *mut *mut HNode,
            mask: size - 1,
            len: 0,
        })
    }

    pub fn iter(&self) -> InnerHashTableIter<'_> {
        InnerHashTableIter::new(self)
    }

    /// Insert `node` in the hashtable in the first position that matches its hash. If the
    /// position is already taken, `node`'s next will point to the already existing chain in the
    /// slot.
    pub unsafe fn insert(&mut self, node: *mut HNode) -> Result<(), InnerHashTableError> {
        // New item are inserted at the front of their respective position
        let pos = unsafe { ((*node).hash & self.mask as u64) as isize };
        // Get the first element at that position
        unsafe {
            let next: *mut HNode = *self.tab.offset(pos);
            // Make the new insert node point to it
            (*node).next = next;
            // Insert the new node
            *self.tab.offset(pos) = node;
        }
        self.len += 1;
        Ok(())
    }

    /// Search for `node` in the current hashtable, making sure the found node (if any) satifies
    /// the `eq` equality check. If lookup does not return a node, it returns `None`.
    pub unsafe fn lookup(
        &self,
        node: *const HNode,
        eq: fn(&HNode, &HNode) -> bool,
    ) -> Option<*mut *mut HNode> {
        let pos = unsafe { ((*node).hash & self.mask as u64) as isize };

        let mut slot = unsafe { self.tab.offset(pos) };

        if slot.is_null() {
            return None;
        }

        unsafe {
            while !(*slot).is_null() {
                if (*(*slot)).hash == (*node).hash && eq(&*(*slot), &*node) {
                    return Some(slot);
                }
                slot = (&mut (*(*slot)).next) as *mut *mut HNode;
            }
        }
        None
    }

    /// Delete the `node` from the hashtable. If `node` pointer is not in the has table, or has
    /// already been dealocated, this panicks.
    pub unsafe fn detach(&mut self, node: *mut *mut HNode) -> Option<*const HNode> {
        // TODO: Do we really want this willy nilly approach?
        if node.is_null() || (unsafe { *node }).is_null() || self.len < 1 {
            return None;
        }

        let to_return = unsafe { *node };

        unsafe { *node = (*(*node)).next };
        self.len -= 1;

        Some(to_return)
    }

    /// Returns the number of keys in the hashtable
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the mask for the number of slots in the hashtable
    pub fn mask(&self) -> usize {
        self.mask
    }
}

/// A resizable hashmap based on the fixed-size `InnerHashTable`. It contains 2 of them for the
/// progressive rehashing.
///
/// There are 2 types of scalability problems: throughput and latency.
/// Throughput problems have generic solutions, such as sharding and read_only replicas and are an
/// average case problem.
/// Latency problems are often domain-specific and harder, being a worst-case problem.
///
/// For hashtables, the largest latency issues comes from insertion, which may trigger an O(N)
/// resize. The solution to this problem is to do it progressively. After allocating the new
/// hashtable, only move a fixed number of keys and each time the hashtable is used, move some
/// more keys. This can slow down lookups during resizing because there are 2 hashtables to query.
#[derive(Debug, Default)]
pub struct HashMap {
    new: InnerHashTable,
    old: Option<InnerHashTable>,
    migrate_pos: usize,
}

/// The maximum number of keys a single slot can hold.
const K_MAX_LOAD_FACTOR: usize = 8;
/// The number of keys to rehash after the table has been rehashed
const K_REHASHING_WORK: usize = 128;

impl HashMap {
    pub fn init(size: usize) -> Result<Self, HashMapError> {
        Ok(Self {
            new: InnerHashTable::init(size)?,
            old: None,
            migrate_pos: 0,
        })
    }

    // When the load factor is too high, the `new` hash map is marked as `old` reallocated as
    // doubl the size of its previous size
    pub fn trigger_rehashing(&mut self) -> Result<(), HashMapError> {
        // Make sure old was deallocated
        if let Some(old) = self.old {
            return Err(HashMapError::OldTableNotEmpty(old.len()));
        }

        self.old = Some(self.new);
        self.new = InnerHashTable::init((self.new.mask() + 1) << 2)?;
        self.migrate_pos = 0;

        Ok(())
    }

    pub unsafe fn help_rehashing(&mut self) -> Result<(), HashMapError> {
        let mut keys_moved = 0;

        if let Some(mut old) = self.old {
            while keys_moved < K_REHASHING_WORK && old.len() > 0 {
                // Find an non-empty slot.
                let from = unsafe { old.tab.offset(self.migrate_pos as isize) };
                self.migrate_pos += 1;
                if from.is_null() {
                    continue;
                }

                // Move the first lsit item to the newer table
                unsafe {
                    self.new
                        .insert(old.detach(from).ok_or(HashMapError::NodeNotFound)? as *mut HNode)?
                };
                keys_moved += 1;
            }
            if old.len() == 0 {
                self.old = None
            }
        }

        Ok(())
    }

    pub unsafe fn lookup(
        &self,
        node: *const HNode,
        eq: fn(&HNode, &HNode) -> bool,
    ) -> Option<*mut HNode> {
        // During rehashind we have to lookup for the element in both tables
        let node = if let Some(node) = unsafe { self.new.lookup(node, eq) } {
            Some(unsafe { *node })
        } else {
            let Some(old) = self.old else {
                return None;
            };
            let Some(node) = (unsafe { old.lookup(node, eq) }) else {
                return None;
            };
            Some(unsafe { *node })
        };
        node
    }

    pub unsafe fn insert(&mut self, node: *const HNode) -> Result<(), HashMapError> {
        // Always insert in the new table
        unsafe { self.new.insert(node as *mut HNode)? };

        // Check if we need to rehash.
        if let None = self.old {
            // Check if we reached our threshold
            let threshold = (self.new.mask() + 1) * K_MAX_LOAD_FACTOR;
            // If the current number of keys is greater, trigger rehashing
            if self.new.len() >= threshold {
                self.trigger_rehashing()?;
            }
        }

        // Move some keys between the 2 tables
        unsafe { self.help_rehashing() }
    }

    pub unsafe fn delete(
        &mut self,
        node: *const HNode,
        eq: fn(&HNode, &HNode) -> bool,
    ) -> Option<*const HNode> {
        if let Some(node) = unsafe { self.new.lookup(node, eq) } {
            unsafe { self.new.detach(node) }
        } else {
            let Some(mut old) = self.old else {
                return None;
            };
            if let Some(node) = unsafe { old.lookup(node, eq) } {
                unsafe { old.detach(node) }
            } else {
                None
            }
        }
    }
}

#[derive(Debug)]
pub enum InnerHashTableError {
    NegativeSize,
    SizeNotPowerOfTwo(usize),
    LayoutError(core::alloc::LayoutError),
}

impl From<core::alloc::LayoutError> for InnerHashTableError {
    fn from(err: core::alloc::LayoutError) -> Self {
        Self::LayoutError(err)
    }
}

#[derive(Debug)]
pub enum HashMapError {
    OldTableNotEmpty(usize),
    InnerHashTableError(InnerHashTableError),
    NodeNotFound,
}

impl From<InnerHashTableError> for HashMapError {
    fn from(err: InnerHashTableError) -> Self {
        Self::InnerHashTableError(err)
    }
}
