use lexikon::hashtable::{HNode, InnerHashTable};

#[test]
fn hnode_default() {
    let hnode = HNode::default();
    assert!(hnode.hash() == 0);
}

#[test]
fn hashtable_init() {
    let htable = InnerHashTable::init(64).expect("Failed to init hashtable");
    assert!(htable.mask() == 63);
    assert!(htable.len() == 0);
}

#[test]
fn hashtable_insert() {
    let hashes = [1, 2, 3, 4, 5];
    let mut htable = InnerHashTable::init(64).expect("Failed to init hashtable");
    for hash in hashes {
        let hnode = HNode::from_hash(hash).as_mut_ptr();
        unsafe { htable.insert(hnode).expect("Failed to insert") };
    }
    assert!(htable.len() == hashes.len());
}

#[test]
fn hashtable_insert_chain() {
    let hashes = [1, 2, 3, 4, 5];
    let mut htable = InnerHashTable::init(2).expect("Failed to init hashtable");
    for hash in hashes {
        let hnode = HNode::from_hash(hash).as_mut_ptr();
        unsafe { htable.insert(hnode).expect("Failed to insert") };
    }
    assert!(htable.len() == hashes.len());
}

#[test]
fn hashtable_lookup() {
    let hashes = [1, 2, 3, 4, 5];
    let mut htable = InnerHashTable::init(2).expect("Failed to init hashtable");
    for hash in hashes {
        let hnode = HNode::from_hash(hash).as_mut_ptr();
        println!("{}", htable);
        unsafe { htable.insert(hnode).expect("Failed to insert") };
    }
    fn eq(left: &HNode, right: &HNode) -> bool {
        left.hash() == right.hash()
    }
    let hnode = HNode::from_hash(3).as_mut_ptr();
    let found = unsafe { htable.lookup(hnode, eq) };
    assert!(found.is_some());

    let hnode = HNode::from_hash(5).as_mut_ptr();
    let found = unsafe { htable.lookup(hnode, eq) };
    assert!(found.is_some());

    let hnode = HNode::from_hash(6).as_mut_ptr();
    let not_found = unsafe { htable.lookup(hnode, eq) };
    assert!(not_found.is_none());
}

#[test]
fn hashtable_deletion() {
    let hashes = [1, 2, 3, 4, 5];
    let mut htable = InnerHashTable::init(2).expect("Failed to init hashtable");
    for hash in hashes {
        let hnode = HNode::from_hash(hash).as_mut_ptr();
        unsafe { htable.insert(hnode).expect("Failed to insert") };
    }
    fn eq(left: &HNode, right: &HNode) -> bool {
        left.hash() == right.hash()
    }

    let hnode = HNode::from_hash(3).as_mut_ptr();
    let found = unsafe { htable.lookup(hnode, eq) };
    let found = found.expect("Failed to get node");
    unsafe { htable.detach(found).expect("Failed to delete node") };
    assert!(htable.len() == 4);

    let hnode = HNode::from_hash(5).as_mut_ptr();
    let found = unsafe { htable.lookup(hnode, eq) };
    let found = found.expect("Failed to get node");
    unsafe { htable.detach(found).expect("Failed to delete node") };
    assert!(htable.len() == 3);

    let hnode = HNode::from_hash(1).as_mut_ptr();
    let found = unsafe { htable.lookup(hnode, eq) };
    let found = found.expect("Failed to get node");
    unsafe { htable.detach(found).expect("Failed to delete node") };
    assert!(htable.len() == 2);
}
