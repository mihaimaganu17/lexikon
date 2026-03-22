use core::mem::MaybeUninit;

macro_rules! offset_of {
    ($base:path, $field:tt) => {{
        let base_type = MaybeUninit::<$base>::uninit();
        let base_ptr = base_type.as_ptr();
        let field_ptr = field_ptr!(base_ptr, $base, $field);
        let diff: isize = unsafe { (field_ptr as *const u8).offset_from(base_ptr as *const u8) };
        diff
    }};
}

macro_rules! field_ptr {
    ($base_ptr:expr, $base_type:path, $field:tt) => {{
        // Check the field is in the base type. This issues a compile time error if `$field` is not
        // a member of `$base_type`
        let $base_type { $field: _, .. };
        let field_ptr = unsafe { &raw const (*($base_ptr as *const $base_type)).$field };
        field_ptr
    }};
}

macro_rules! container_of {
    ($field_ptr:expr, $base_type:path, $field:tt) => {{
        let field_offset = offset_of!($base_type, $field);
        let base_ptr = unsafe { ($field_ptr as *const u8).sub(field_offset as usize) as *mut $base_type };
        base_ptr
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_of() {
        #[repr(C)]
        struct Node {
            a: u32,
            b: u32,
        }

        let node_ptr: *mut Node = Box::into_raw(Box::new(Node {
            a: 0xb00b,
            b: 0x1337,
        }));

        #[repr(C)]
        struct Entry {
            before_node: u64,
            node: *mut Node,
        }

        let entry_orig_ptr = Box::into_raw(Box::new(Entry { before_node: 0xa01e0, node:node_ptr }));
        let node_ptr = unsafe { (*entry_orig_ptr).node };

        let entry_container: *mut Entry = container_of!(node_ptr, Entry, node);
        println!("{:#?}", entry_orig_ptr);
        println!("{:#?}", node_ptr);
        println!("{:#?}", entry_container);

        assert!(entry_orig_ptr == entry_container);
    }

    #[test]
    fn offset_of() {
        #[repr(C)]
        struct Node {
            a: u32,
            b: u32,
        }

        let node: *mut Node = Box::into_raw(Box::new(Node {
            a: 0xb00b,
            b: 0x1337,
        }));

        #[repr(C)]
        struct Entry {
            before_node: u64,
            node: Node,
            after_node: u64,
            more_after: u8,
            unaligned: u8,
        }

        assert!(offset_of!(Entry, after_node) == 16);
        assert!(offset_of!(Entry, before_node) == 0);
        assert!(offset_of!(Entry, more_after) == 24);
        assert!(offset_of!(Entry, unaligned) == 25);
    }
}
