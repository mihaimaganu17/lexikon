unsafe extern "C" {
    fn socket(domain: i32, t_type: i32, protocol: i32) -> i32;
    // option_len is a value-result parameter, initially containing the size of the buffer pointed
    // to by option_value, and modified on return to indicate the actual size of the value
    // returned.  If no option value is to be supplied or returned, option_value may be NULL.
    fn getsockopt(
        socket: i32,
        level: i32,
        option_name: i32,
        option_value: *mut core::ffi::c_void,
        option_len: *mut u32,
    ) -> i32;

    fn setsockopt(
        socket: i32,
        level: i32,
        option_name: i32,
        option_value: *mut core::ffi::c_void,
        option_len: u32,
    ) -> i32;
}

/// Comprises types of communication domains whithin which communication will take place. These
/// are also regarded as address families (AF_)
pub(crate) mod domain {
    // Used by Internet Protocols, IPv4: TCP, UDP
    pub const AF_INET: i32 = 2;
}

/// Types of sockets
mod socket_type {
    // Stream socket: TCP
    pub const SOCK_STREAM: i32 = 1;
    // Datagram socket: UDP
    pub const SOCK_DGRAM: i32 = 2;
}

mod level {
    // Seems to be the only viable level
    pub const SOL_SOCKET: i32 = 0xFFFF;
}

mod socket_option {
    // These option flags are a bit mask of all ORed together.
    // Allow local address reuse
    pub const SO_REUSEADDR: i32 = 0x0004;
}

macro_rules! check_status {
    ($status:expr) => {
        if $status == -1 {
            // Should this be a return?
            println!(
                "Status {:?} -> {:?}",
                $status,
                std::io::Error::last_os_error()
            );
        }
    };
}

pub fn start_server() -> Result<(), ServerError> {
    let fd = unsafe { socket(domain::AF_INET, socket_type::SOCK_STREAM, 0) };

    if fd == -1 {
        return Err(ServerError::InvalidSocketHandle);
    }
    // Set socket reuse address to 1
    let mut option_value = 1u32;
    let option_len = core::mem::size_of::<u32>() as u32;

    let status = unsafe {
        setsockopt(
            fd,
            level::SOL_SOCKET,
            socket_option::SO_REUSEADDR,
            &mut option_value as *mut u32 as *mut core::ffi::c_void,
            option_len,
        )
    };
    check_status!(status);

    Ok(())
}

#[derive(Debug)]
pub enum ServerError {
    InvalidSocketHandle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main() {
        start_server().expect("Failed to start server");
    }

    #[test]
    fn test_socket() {
        let fd = unsafe { socket(domain::AF_INET, socket_type::SOCK_STREAM, 0) };
        assert!(fd != -1);

        let mut option_value = 10u32;
        let mut option_len = core::mem::size_of::<u32>() as u32;
        // We want to set the SO_REUSEADDR option to value of 1. We first check the value of the
        // option
        let status = unsafe {
            getsockopt(
                fd,
                level::SOL_SOCKET,
                socket_option::SO_REUSEADDR,
                &mut option_value as *mut u32 as *mut core::ffi::c_void,
                &mut option_len as *mut u32,
            )
        };

        assert!(status == 0);

        // Set socket reuse address to 1
        option_value = 1u32;
        let status = unsafe {
            setsockopt(
                fd,
                level::SOL_SOCKET,
                socket_option::SO_REUSEADDR,
                &mut option_value as *mut u32 as *mut core::ffi::c_void,
                option_len,
            )
        };

        assert!(status == 0);

        // This second call should return 4 as the flags are part of a bit mask, and SO_REUSEADDR
        // holds the third byte of that bitmask.
        let status = unsafe {
            getsockopt(
                fd,
                level::SOL_SOCKET,
                socket_option::SO_REUSEADDR,
                &mut option_value as *mut u32 as *mut core::ffi::c_void,
                &mut option_len as *mut u32,
            )
        };

        assert!(status == 0);
    }
}
