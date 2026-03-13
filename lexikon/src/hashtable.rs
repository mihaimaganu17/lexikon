//! C-style chaining hashtable implementation

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
    tab: *const *const HNode,
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

        Ok(Self {
            size,
            ..Self::default()
        })
    }
}

#[derive(Debug)]
pub enum HashTableError {
    NegativeSize,
    SizeNotPowerOfTwo(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        let hnode = HNode::default();
        println!("HNode {:#?}", hnode);
    }
}
