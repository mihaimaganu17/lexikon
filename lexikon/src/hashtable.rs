//! C-style chaining hashtable implementation
use core::alloc::Layout;
use std::alloc::alloc_zeroed;
use std::fmt;

#[derive(Default, Debug)]
struct HNode {
    // Reference to the next node
    next: *mut HNode,
    // Hash value
    hash: u64,
}

#[derive(Debug, Default)]
struct HashTable {
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
    pub unsafe fn lookup(&self, node: *const HNode, eq: fn(&HNode, &HNode) -> bool) -> Option<*mut *mut HNode> {
        let pos = ((*node).hash & self.mask as u64) as isize;

        let mut slot = self.tab.offset(pos);

        if slot.is_null() {
            return None;
        }

        println!("Slot {:#?}", slot);
        while !(*slot).is_null() {
            if (*(*slot)).hash == (*node).hash && eq(&*(*slot), &*node) {
                // We might need to return the slot here in order to delete it in an easier manner.
                return Some(slot);
            }
            slot = (&mut (*(*slot)).next) as *mut *mut HNode;
        }
        None
    }

    pub unsafe fn detach(&mut self, node: *mut *const HNode) -> Option<*const HNode> {
        // Check node is not null
        if node.is_null() || (*node).is_null() || self.len < 1{
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hnode_default() {
        let hnode = HNode::default();
        println!("HNode {:#?}", hnode);
    }

    #[test]
    fn hashtable_init() {
        let htable = HashTable::init(64).expect("Failed to init hashtable");
        println!("HashTable {:#?}", htable);
    }

    #[test]
    fn hashtable_insert() {
        let hashes = [1, 2, 3, 4, 5];
        let mut htable = HashTable::init(64).expect("Failed to init hashtable");
        for hash in hashes {
            let mut hnode = Box::new(HNode {
                next: core::ptr::null::<HNode>() as *mut HNode,
                hash,
            });
            unsafe {
                htable
                    .insert(Box::into_raw(hnode))
                    .expect("Failed to insert")
            };
        }
        assert!(htable.len() == hashes.len());
        println!("{}", htable);
    }

    #[test]
    fn hashtable_insert_chain() {
        let hashes = [1, 2, 3, 4, 5];
        let mut htable = HashTable::init(2).expect("Failed to init hashtable");
        for hash in hashes {
            let mut hnode = Box::new(HNode {
                next: core::ptr::null::<HNode>() as *mut HNode,
                hash,
            });
            unsafe {
                htable
                    .insert(Box::into_raw(hnode))
                    .expect("Failed to insert")
            };
        }
        assert!(htable.len() == hashes.len());
        println!("{}", htable);
    }

    #[test]
    fn hashtable_lookup() {
        let hashes = [1, 2, 3, 4, 5];
        let mut htable = HashTable::init(2).expect("Failed to init hashtable");
        for hash in hashes {
            let mut hnode = Box::new(HNode {
                next: core::ptr::null::<HNode>() as *mut HNode,
                hash,
            });
            unsafe {
                htable
                    .insert(Box::into_raw(hnode))
                    .expect("Failed to insert")
            };
        }
        fn eq(left: &HNode, right: &HNode) -> bool {
            left.hash == right.hash
        }
        let mut hnode = Box::new(HNode {
            next: core::ptr::null::<HNode>() as *mut HNode,
            hash: 3,
        });
        let found = unsafe { htable.lookup(Box::into_raw(hnode), eq) };
        assert!(found.is_some());

        let mut hnode = Box::new(HNode {
            next: core::ptr::null::<HNode>() as *mut HNode,
            hash: 5,
        });
        let found = unsafe { htable.lookup(Box::into_raw(hnode), eq) };
        assert!(found.is_some());

        let mut hnode = Box::new(HNode {
            next: core::ptr::null::<HNode>() as *mut HNode,
            hash: 6,
        });
        let not_found = unsafe { htable.lookup(Box::into_raw(hnode), eq) };
        assert!(not_found.is_none());

        println!("{:#?}", htable);
    }
}
