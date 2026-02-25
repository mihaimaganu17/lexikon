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
    // bind() assigns a name to an unnamed socket.  When a socket is created with socket(2) it
    // exists in a name space (address family) but has no name assigned.  bind() requests that
    // address be assigned to the socket
    fn bind(socket_fd: i32, sock_addr: &SockAddr, sock_len: u32) -> i32;
    // Listen to incoming connections for socket, with a queue limit of `backlog`
    fn listen(socket_fd: i32, backlog: u32) -> i32;
    // Accept a connection to a socket. Returns a descriptor for the accepted client socket or
    // -1 in case of an error. The `sock_addr` is populated with the resulting address of the
    // client that was accepted for a connection.
    fn accept(socket_fd: i32, sock_addr: &mut SockAddr, sock_len: &mut u32) -> i32;
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

// Maximum queue length specifiable for a `listen` call on XNU
pub(crate) const SOMAXCONN: u32 = 128;

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

// sockaddr as defined by the xnu kernel, which has a bit of a different layout overall, however
// it does keep compatibility with the historical UNIX sockaddr_in
#[repr(C)]
#[derive(Default)]
struct SockAddr {
    sa_len: u8,
    sa_family: u8,
    // This data is structured as follows:
    // port: u16 -> first 2 bytes (big endian)
    // addr: u32 -> first 4 bytes (big endian)
    // padding: rest of 8 bytes (zero padding)
    sa_data: [u8; 14],
}

impl SockAddr {
    fn new(sa_family: u8, addr: u32, port: u16) -> Self {
        let mut sa_data = [0; 14];
        // Fill port
        sa_data[0..2].copy_from_slice(&port.to_be_bytes());
        sa_data[2..6].copy_from_slice(&addr.to_be_bytes());
        Self {
            // This cast is safe as the size of SockAddr will always be 16
            sa_len: core::mem::size_of::<SockAddr>() as u8,
            sa_family,
            sa_data,
        }
    }
}

pub fn start_server() -> Result<(), ServerError> {
    // 1. Create socket
    let fd = unsafe { socket(domain::AF_INET, socket_type::SOCK_STREAM, 0) };

    if fd == -1 {
        return Err(ServerError::InvalidSocketHandle);
    }
    // 2. Set socket reuse address option to 1
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

    // 3. Bind to an address
    let sock_addr = SockAddr::new(domain::AF_INET as u8, 0, 1234);

    let status = unsafe { bind(fd, &sock_addr, core::mem::size_of::<SockAddr>() as u32) };

    check_status!(status);

    // 4. listen for incoming connetctions
    let status = unsafe { listen(fd, SOMAXCONN) };
    check_status!(status);

    // 5. Accept incoming connections
    loop {
        let mut client_sock_addr = SockAddr::default();
        let mut sock_addr_len: u32 = core::mem::size_of::<SockAddr>() as u32;

        let conn_fd = unsafe { accept(fd, &mut client_sock_addr, &mut sock_addr_len) };

        check_status!(conn_fd);
    }

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
