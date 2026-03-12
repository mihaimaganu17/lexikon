mod event_loop;
mod protocol;

use event_loop::ParseError;
pub use event_loop::run_server;
use protocol::LexRequest;

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

fn check_status(status: i32) -> Result<(), std::io::Error> {
    if status == -1 {
        // Should this be a return?
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

// sockaddr as defined by the xnu kernel, which has a bit of a different layout overall, however
// it does keep compatibility with the historical UNIX sockaddr_in.
// Any change in the layout and or size of this structure should take into account that it's length
// cannot ever exceed a u8 -> 256 bytes in size
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

fn setup_socket() -> Result<i32, ServerError> {
    // 1. Create socket
    let fd = unsafe { socket(domain::AF_INET, socket_type::SOCK_STREAM, 0) };
    if fd == -1 {
        return Err(ServerError::InvalidSocketHandle);
    }

    // 2. Set socket reuse address option to 1
    let mut option_value = 1u32;
    let option_len = u32::try_from(core::mem::size_of::<u32>())?;
    let status = unsafe {
        setsockopt(
            fd,
            level::SOL_SOCKET,
            socket_option::SO_REUSEADDR,
            &mut option_value as *mut u32 as *mut core::ffi::c_void,
            option_len,
        )
    };
    crate::check_status(status)?;

    // 3. Bind to an address
    let sock_addr = SockAddr::new(u8::try_from(domain::AF_INET)?, 0, 1234);
    let status = unsafe {
        bind(
            fd,
            &sock_addr,
            u32::try_from(core::mem::size_of::<SockAddr>())?,
        )
    };
    crate::check_status(status)?;

    // 4. listen for incoming connetctions
    let status = unsafe { listen(fd, SOMAXCONN) };
    crate::check_status(status)?;

    Ok(fd)
}

pub fn start_server() -> Result<(), ServerError> {
    let fd = setup_socket()?;

    // 5. Accept incoming connections
    loop {
        let mut client_sock_addr = SockAddr::default();
        let mut sock_addr_len: u32 = u32::try_from(core::mem::size_of::<SockAddr>())?;

        let conn_fd = unsafe { accept(fd, &mut client_sock_addr, &mut sock_addr_len) };

        check_status(conn_fd)?;
        println!("Client sock addr: {:?}", client_sock_addr);

        loop {
            match read_and_respond(conn_fd) {
                Ok(bytes_written) => {
                    if bytes_written == 0 {
                        break;
                    }
                }
                Err(err) => {
                    println!("Client Error: {:?}", err);
                    break;
                }
            }
        }
        let status = unsafe { close(conn_fd) };
        check_status(status)?;
    }
}

fn read_msg(fd: i32) -> Result<Vec<u8>, ReadError> {
    // We prepared a dummy protocol, where each message is preceded by it's length under the form
    // of a little endian 4-bytes unsigned integer.
    // |     len | msg1     |       len | msg2 | ... |
    // 0         4          len + 4
    // TODO: We should check the buffer that we read
    let buffer = read_full(fd, 4)?;
    if buffer.is_empty() {
        return Ok(buffer);
    }
    let buffer_len = usize::try_from(u32::from_le_bytes(
        buffer
            .get(0..4)
            .ok_or(ReadError::InvalidRange(0, 4))?
            .try_into()?,
    ))?;

    read_full(fd, buffer_len)
}

fn write_msg(fd: i32, write_buffer: &[u8]) -> Result<usize, WriteError> {
    // We prepared a dummy protocol, where each message is preceded by it's length under the form
    // of a little endian 4-bytes unsigned integer.
    // |     len | msg1     |       len | msg2 | ... |
    // 0         4          len + 4
    let write_buffer_len = u32::try_from(write_buffer.len())?.to_le_bytes();
    let mut bytes_written = write_full(fd, &write_buffer_len)?;
    bytes_written += write_full(fd, write_buffer)?;

    Ok(bytes_written)
}

fn read_full(fd: i32, expected_len: usize) -> Result<Vec<u8>, ReadError> {
    let mut left_to_read = expected_len;
    let mut full_buffer = vec![];
    let mut buffer = [0u8; 64];

    while left_to_read > 0 {
        let max_bytes_to_read = core::cmp::min(left_to_read, buffer.len());
        let bytes_read = unsafe {
            read(
                fd,
                buffer.as_mut_ptr() as *mut core::ffi::c_void,
                u32::try_from(max_bytes_to_read)?,
            )
        };
        check_status(bytes_read)?;
        let bytes_read = usize::try_from(bytes_read)?;
        // TODO: Should we pace this based on number of pulls?
        // read can also be interrupted by a signal because it must wait if the buffer is empty.
        // In this case, 0 bytes are read, but the return value is -1 and errno is EINTR.
        if bytes_read == 0 {
            return Ok(full_buffer);
        }
        left_to_read = left_to_read.saturating_sub(bytes_read);
        full_buffer.extend_from_slice(
            buffer
                .get(0..bytes_read)
                .ok_or(ReadError::InvalidRange(0, bytes_read))?,
        );
    }
    Ok(full_buffer)
}

fn write_full(fd: i32, buffer: &[u8]) -> Result<usize, WriteError> {
    // How many bytes to write per each write call
    let window_write_len = 64usize;
    let mut start = 0;
    let mut end = 0;

    while end < buffer.len() {
        end = start + window_write_len;
        end = core::cmp::min(end, buffer.len());
        let slice = buffer
            .get(start..end)
            .ok_or(WriteError::InvalidRange(start, end))?;
        let bytes_written = unsafe {
            write(
                fd,
                slice.as_ptr() as *const core::ffi::c_void,
                u32::try_from(slice.len())?,
            )
        };
        check_status(bytes_written)?;
        start = end;
    }
    Ok(end)
}

fn read_and_respond(fd: i32) -> Result<usize, ServerError> {
    // 1. Read client message
    let msg = read_msg(fd)?;
    if msg.is_empty() {
        return Ok(0);
    }
    println!("{}", String::from_utf8_lossy(&msg));

    // 2. Write message back to client
    let write_buffer = String::from("HTTP/1.1 200 OK\n\nhello");
    let bytes_written = write_msg(fd, write_buffer.as_bytes())?;
    println!("Wrote {} bytes", bytes_written);

    Ok(bytes_written)
}

#[derive(Debug)]
pub enum ServerError {
    InvalidSocketHandle,
    ReadError(ReadError),
    WriteError(WriteError),
    ParseError(ParseError),
    StdIOError(std::io::Error),
    TryFromSliceError(std::array::TryFromSliceError),
    TryFromIntError(std::num::TryFromIntError),
}

#[derive(Debug)]
pub enum ClientError {
    InvalidSocketHandle,
    ReadError(ReadError),
    WriteError(WriteError),
    StdIOError(std::io::Error),
    TryFromIntError(std::num::TryFromIntError),
}

#[derive(Debug)]
pub enum ReadError {
    NoMessage,
    InvalidIdx(usize),
    InvalidRange(usize, usize),
    StdIOError(std::io::Error),
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

impl From<std::num::TryFromIntError> for WriteError {
    fn from(err: std::num::TryFromIntError) -> Self {
        Self::TryFromIntError(err)
    }
}

impl From<std::num::TryFromIntError> for ServerError {
    fn from(err: std::num::TryFromIntError) -> Self {
        Self::TryFromIntError(err)
    }
}

impl From<std::num::TryFromIntError> for ClientError {
    fn from(err: std::num::TryFromIntError) -> Self {
        Self::TryFromIntError(err)
    }
}

impl From<std::array::TryFromSliceError> for ServerError {
    fn from(err: std::array::TryFromSliceError) -> Self {
        Self::TryFromSliceError(err)
    }
}

impl From<std::io::Error> for ReadError {
    fn from(err: std::io::Error) -> Self {
        Self::StdIOError(err)
    }
}

impl From<std::io::Error> for WriteError {
    fn from(err: std::io::Error) -> Self {
        Self::StdIOError(err)
    }
}

impl From<std::io::Error> for ServerError {
    fn from(err: std::io::Error) -> Self {
        Self::StdIOError(err)
    }
}

impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        Self::StdIOError(err)
    }
}

impl From<ReadError> for ClientError {
    fn from(err: ReadError) -> Self {
        Self::ReadError(err)
    }
}

impl From<ReadError> for ServerError {
    fn from(err: ReadError) -> Self {
        Self::ReadError(err)
    }
}

impl From<ParseError> for ServerError {
    fn from(err: ParseError) -> Self {
        Self::ParseError(err)
    }
}

impl From<WriteError> for ClientError {
    fn from(err: WriteError) -> Self {
        Self::WriteError(err)
    }
}

impl From<WriteError> for ServerError {
    fn from(err: WriteError) -> Self {
        Self::WriteError(err)
    }
}

#[derive(Debug)]
pub enum WriteError {
    StdIOError(std::io::Error),
    InvalidRange(usize, usize),
    TryFromIntError(std::num::TryFromIntError),
}

pub fn pipeline_test_client() -> Result<(), ClientError> {
    // 1. Create socket
    let fd = unsafe { socket(domain::AF_INET, socket_type::SOCK_STREAM, 0) };

    if fd == -1 {
        return Err(ClientError::InvalidSocketHandle);
    }

    // 2. Connect to the loopback address -> 127.0.0.1
    let sock_addr = SockAddr::new(u8::try_from(domain::AF_INET)?, 0x7f000001, 1234);

    let status = unsafe {
        connect(
            fd,
            &sock_addr,
            u32::try_from(core::mem::size_of::<SockAddr>())?,
        )
    };
    check_status(status)?;

    // Create a big request that takes multiple iterations to process.
    let k_max_msg_size = 32 * 100;
    let mut big_boy: Vec<u8> = vec![];
    big_boy.resize(k_max_msg_size, 0x5A);
    // Build a collection of queries we want to make to the server according to protocol
    let queries = vec![
        vec![
        "set".to_string(),
        "money".to_string(),
        "132,334".to_string(),
        ],
        vec![
        "get".to_string(),
        "money".to_string(),
        ],
        vec![
        "del".to_string(),
        "money".to_string(),
        ],
        vec![
        "get".to_string(),
        "money".to_string(),
        ]
    ];
    for query_list in queries {
        let lex_request = LexRequest::new(Some(query_list.clone()));
        let bytes = lex_request
            .to_request()
            .expect("Failed to convert args to request");

        println!("Bytes len {:?}", bytes.len());

        let bytes_written = write_msg(fd, &bytes)?;
        println!("{} bytes written", bytes_written);

        // TODO: Need to parse buffer in a Response format
        let buffer = read_msg(fd)?;
        println!("Response {}", String::from_utf8_lossy(&buffer));
    }

    unsafe { close(fd) };
    Ok(())
}

pub fn start_client() -> Result<(), ClientError> {
    // 1. Create socket
    let fd = unsafe { socket(domain::AF_INET, socket_type::SOCK_STREAM, 0) };

    if fd == -1 {
        return Err(ClientError::InvalidSocketHandle);
    }

    // 2. Connect to the loopback address -> 127.0.0.1
    let sock_addr = SockAddr::new(u8::try_from(domain::AF_INET)?, 0x7f000001, 1234);

    let status = unsafe {
        connect(
            fd,
            &sock_addr,
            u32::try_from(core::mem::size_of::<SockAddr>())?,
        )
    };
    check_status(status)?;

    let msg = String::from("Good morning!");
    let _bytes_written = write_msg(fd, msg.as_bytes())?;
    let buffer = read_msg(fd)?;
    println!("{}", String::from_utf8_lossy(&buffer));

    let msg = "How are you?";
    let _bytes_written = write_msg(fd, msg.as_bytes())?;
    let buffer = read_msg(fd)?;
    println!("{}", String::from_utf8_lossy(&buffer));

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
