//! C-style chaining hashtable implementation
use core::alloc::Layout;
use std::alloc::alloc_zeroed;
use std::fmt;

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
pub struct HashTable {
    // Pointer to the hashtable
    // Should this be a `Vec<Box<HNode>>`?
    tab: *mut *mut HNode,
    // Mask to map the hash according to our desired size
    mask: usize,
    // Number of keys currently in the table
    len: usize,
}

impl fmt::Display for HashTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut idx = 0;
        let mut pos = 0isize;
        let tab_cursor = self.tab;
        if tab_cursor.is_null() {
            return Ok(());
        }

        while idx < self.len() && pos <= self.mask as isize {
            unsafe {
                let mut pos_ptr = tab_cursor.offset(pos);
                println!("Slot {:#?}", pos_ptr);
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
                    );
                    hash_ptr = (*elem).next;
                    idx += 1;
                }
                pos += 1;
            }
        }

        Ok(())
    }
}

impl HashTable {
    pub fn init(size: usize) -> Result<Self, HashTableError> {
        // Make sure the size is not 0 or negative. `usize` prevents this but we are extra
        if size <= 0 {
            return Err(HashTableError::NegativeSize);
        }
        // Make sure size is a power of 2
        if size - 1 & size != 0 {
            return Err(HashTableError::SizeNotPowerOfTwo(size));
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

    /// Insert `node` in the hashtable in the first position that matches its hash. If the position
    /// is already taken, `node`'s next will point to the already existing chain in the slot.
    pub unsafe fn insert(&mut self, node: *mut HNode) -> Result<(), HashTableError> {
        // New item are inserted at the front of their respective position
        let pos = ((*node).hash & self.mask as u64) as isize;
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
        let pos = ((*node).hash & self.mask as u64) as isize;

        let mut slot = self.tab.offset(pos);

        if slot.is_null() {
            return None;
        }

        while !(*slot).is_null() {
            if (*(*slot)).hash == (*node).hash && eq(&*(*slot), &*node) {
                // We might need to return the slot here in order to delete it in an easier manner.
                return Some(slot);
            }
            slot = (&mut (*(*slot)).next) as *mut *mut HNode;
        }
        None
    }

    /// Delete the `node` from the hashtable. If `node` pointer is not in the has table, or has
    /// already been dealocated, this panicks.
    pub unsafe fn detach(&mut self, node: *mut *mut HNode) -> Option<*const HNode> {
        // TODO: Do we really want this willy nilly approach?
        if node.is_null() || (*node).is_null() || self.len < 1 {
            return None;
        }

        let to_return = *node;

        (*node) = (*(*node)).next;
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

/// A resizable hashmap based on the fixed-size `HashTable`. It contains 2 of them for the
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
    new: HashTable,
    old: Option<HashTable>,
    migrate_pos: usize,
}

/// The maximum number of keys a single slot can hold.
const K_MAX_LOAD_FACTOR: usize = 8;
/// The number of keys to rehash after the table has been rehashed
const K_REHASHING_WORK: usize = 128;

impl HashMap {
    // When the load factor is too high, the `new` hash map is marked as `old` reallocated as
    // doubl the size of its previous size
    pub fn trigger_rehashing(&mut self) -> Result<(), HashMapError> {
        // Make sure old was deallocated
        if let Some(old) = self.old {
            return Err(HashMapError::OldTableNotEmpty(old.len()));
        }

        self.old = Some(self.new);
        self.new = HashTable::init((self.new.mask() + 1) << 2)?;
        self.migrate_pos = 0;

        Ok(())
    }

    pub unsafe fn help_rehashing(&mut self) -> Result<(), HashMapError> {
        let mut keys_moved = 0;

        if let Some(mut old) = self.old {
            while keys_moved < K_REHASHING_WORK && old.len() > 0 {
                // Find an non-empty slot.
                let from = old.tab.offset(self.migrate_pos as isize);
                self.migrate_pos += 1;
                if from.is_null() {
                    continue;
                }

                // Move the first lsit item to the newer table
                self.new
                    .insert(old.detach(from).ok_or(HashMapError::NodeNotFound)? as *mut HNode)?;
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
        let node = if let Some(node) = self.new.lookup(node, eq) {
            Some(*node)
        } else {
            let Some(old) = self.old else {
                return None;
            };
            let Some(node) = old.lookup(node, eq) else {
                return None;
            };
            Some(*node)
        };
        node
    }

    pub unsafe fn insert(&mut self, node: *const HNode) -> Result<(), HashMapError> {
        // Always insert in the new table
        self.new.insert(node as *mut HNode)?;

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
        self.help_rehashing()
    }

    pub unsafe fn delete(
        &mut self,
        node: *const HNode,
        eq: fn(&HNode, &HNode) -> bool,
    ) -> Option<*const HNode> {
        if let Some(node) = self.new.lookup(node, eq) {
            self.new.detach(node)
        } else {
            let Some(mut old) = self.old else {
                return None;
            };
            if let Some(node) = old.lookup(node, eq) {
                old.detach(node)
            } else {
                None
            }
        }
    }
}

#[derive(Debug)]
pub enum HashTableError {
    NegativeSize,
    SizeNotPowerOfTwo(usize),
    LayoutError(core::alloc::LayoutError),
}

impl From<core::alloc::LayoutError> for HashTableError {
    fn from(err: core::alloc::LayoutError) -> Self {
        Self::LayoutError(err)
    }
}

#[derive(Debug)]
pub enum HashMapError {
    OldTableNotEmpty(usize),
    HashTableError(HashTableError),
    NodeNotFound,
}

impl From<HashTableError> for HashMapError {
    fn from(err: HashTableError) -> Self {
        Self::HashTableError(err)
    }
}
