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
        let mut pos = 0;
        let tab_cursor = self.tab;
        if tab_cursor.is_null() {
            return Ok(())
        }

        while idx < len && pos < mask {
            let mut pos_ptr = tab_cursor.offset(pos);
            while !pos_ptr.is_null() {
                elem = *pos_ptr;
                write!(f, "({})", elem.hash);
                pos_ptr = pos_ptr.next;
            }
            pos += 1;
        }
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
            let mut hnode = HNode {
                next: core::ptr::null::<HNode>() as *mut HNode,
                hash,
            };
            unsafe { htable.insert(&mut hnode).expect("Failed to insert") };
        }
        assert!(htable.len(), hashes.len());
        println!("HashTable {}", htable);
    }
}
