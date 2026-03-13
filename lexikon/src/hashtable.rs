//! C-style chaining hashtable implementation
use core::alloc::Layout;
use std::alloc::alloc_zeroed;

#[derive(Default, Debug)]
struct HNode {
    // Reference to the next node
    next: *mut HNode,
    // Hash value
    hcode: u64,
}

#[derive(Debug, Default)]
struct HashTable {
    // Pointer to the hashtable
    // Should this be a `Vec<Box<HNode>>`?
    tab: *mut *mut HNode,
    // Mask to map the hash according to our desired size
    mask: usize,
    // Number of keys currently in the table
    size: usize,
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
            size,
            tab: tab as *mut *mut HNode,
            ..Self::default()
        })
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
}
