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
        let field_ptr = unsafe { &raw const (*($base_ptr as *const $base_type)).$field };
        field_ptr
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container() {
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
