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
    // Read `nbyte` bytes from the file descriptor `fd` into the given `buffer`. Returns the
    // number of bytes read. If the return is 0, we reached end of file. If the return is -1, we
    // have an error and the global errno is set.
    fn read(fd: i32, buffer: *mut core::ffi::c_void, nbyte: u32) -> i32;
    // Write `nbyte` bytes to the file descriptor `fd` from the given `buffer`. Upon successful
    // completion, returns the number of bytes written. If the return value is -1, we have an error
    // and the global errno is set.
    fn write(socket_fd: i32, buffer: *const core::ffi::c_void, nbyte: u32) -> i32;
    // Close a descriptor
    fn close(fd: i32) -> i32;
    // Initiate a connection using the `socket_fd` to the address specified by `sock_addr`
    fn connect(socket_fd: i32, sock_addr: &SockAddr, sock_len: u32) -> i32;
}

// TODO: Use RawFd for the socket?

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
    pub const _SOCK_DGRAM: i32 = 2;
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
#[derive(Default, Debug)]
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
        println!("{:x?}", sa_data);
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
        println!("Client sock addr: {:?}", client_sock_addr);

        read_and_respond(conn_fd);
        let status = unsafe { close(conn_fd) };
        check_status!(status);
    }
}

fn read_full(fd: i32, expected_len: usize) -> Vec<u8> {
    let mut left_to_read = expected_len;
    let mut full_buffer = vec![];
    let mut buffer = [0u8; 64];

    while left_to_read > 0 {
        let max_bytes_to_read = core::cmp::min(left_to_read, buffer.len());
        let bytes_read = unsafe {
            read(
                fd,
                buffer.as_mut_ptr() as *mut core::ffi::c_void,
                max_bytes_to_read as u32,
            )
        };
        check_status!(bytes_read);
        left_to_read = left_to_read.saturating_sub(bytes_read as usize);
        full_buffer.extend_from_slice(&buffer[0..bytes_read as usize]);
    }
    return full_buffer
}

fn write_full(fd: i32, buffer: &[u8]) -> Result<(), WriteError> {
    let mut left_to_write = buffer.len();
    // How many bytes to write per each write call
    let window_write_len = 64usize;
    let mut start = 0;
    let mut end = 0;

    while end < buffer.len() {
        let end = start + window_write_len;
        let end = core::cmp::min(end, buffer.len());
        let slice = buffer.get(start..end).ok_or(WriteError::InvalidRange(start, end))?;
        let bytes_written = unsafe {
            write(
                fd,
                slice.as_ptr() as *const core::ffi::c_void,
                slice.len() as u32,
            )
        };
        check_status!(bytes_written);
        start = end;
    }
    Ok(())
}

fn read_and_respond(fd: i32) -> Result<(), ReadError> {
    // We preparea a dummy protocol, where each message is preceded by it's length under the form
    // of a little endian 4-bytes unsigned integer.
    // |     len | msg1     |       len | msg2 | ... |
    // 0         4          len + 4
    let buffer = read_full(fd, 4);
    let buffer_len = usize::try_from(u32::from_le_bytes(buffer.get(0..4).ok_or(ReadError::InvalidRange(0, 4))?.try_into()?))?;

    let msg = read_full(fd, buffer_len);

    println!("{}", String::from_utf8_lossy(&msg));

    let write_buffer = String::from("HTTP/1.1 200 OK\n\nhello");

    let bytes_w = unsafe {
        write(
            fd,
            write_buffer.as_ptr() as *const core::ffi::c_void,
            write_buffer.len() as u32,
        )
    };
    check_status!(bytes_w);
    println!("Wrote {} bytes", bytes_w);

    Ok(())
}

#[derive(Debug)]
pub enum ServerError {
    InvalidSocketHandle,
}

#[derive(Debug)]
pub enum ClientError {
    InvalidSocketHandle,
}

#[derive(Debug)]
pub enum ReadError {
    InvalidRange(usize, usize),
    TryFromSliceError(std::array::TryFromSliceError),
    TryFromIntError(std::num::TryFromIntError),
}

impl From<std::array::TryFromSliceError> for ReadError {
    fn from(err: std::array::TryFromSliceError) -> Self {
        Self::TryFromSliceError(err)
    }
}

impl From<std::num::TryFromIntError> for ReadError {
    fn from(err: std::num::TryFromIntError) -> Self {
        Self::TryFromIntError(err)
    }
}

#[derive(Debug)]
pub enum WriteError {
    InvalidRange(usize, usize),
}


pub fn start_client() -> Result<(), ClientError> {
    // 1. Create socket
    let fd = unsafe { socket(domain::AF_INET, socket_type::SOCK_STREAM, 0) };

    if fd == -1 {
        return Err(ClientError::InvalidSocketHandle);
    }

    // 2. Connect to the loopback address -> 127.0.0.1
    let sock_addr = SockAddr::new(domain::AF_INET as u8, 0x7f000001, 1234);

    let status = unsafe { connect(fd, &sock_addr, core::mem::size_of::<SockAddr>() as u32) };
    check_status!(status);

    let msg = String::from("hello");
    let _bytes_w = unsafe {
        write(
            fd,
            msg.as_ptr() as *const core::ffi::c_void,
            msg.len() as u32,
        )
    };

    //let buffer = read_full(fd);

    //println!("{}", String::from_utf8_lossy(&buffer));
    unsafe { close(fd) };
    Ok(())
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
