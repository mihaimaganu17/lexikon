//! C-style chaining hashtable implementation

#[derive(Default, Debug)]
struct HNode {
    // Reference to the next node
    next: *mut HNode,
    // Hash value
    hcode: u64,
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
