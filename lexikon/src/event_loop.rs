unsafe extern "C" {
    // File control -> fcntl() provides for control over descriptors.  The argument fildes is a
    // descriptor to be operated on by cmd. In particular, the C version supports variadic
    // arguments, but for our purposes, we only need 1 argument because we only want to set the
    // flags with F_SETFL and F_GETFL. In the case of F_GETFL, the `value` argument is ignored.
    fn fcntl(filedes: i32, cmd: i32, value: i32) -> i32;
    // poll() examines a set of file descriptors to see if some of them are ready for I/O or if
    // certain events have occurred on them.  The fds argument is a pointer to an array of pollfd
    // structures, as defined in ⟨poll.h⟩ (shown below).  The nfds argument specifies the size of
    // the fds array.
    fn poll(fds: &mut PollFd, nfds: u32, timeout: i32) -> i32;
    // Note: Should also define `kqueue` which is used for real projects in BSD
    // Note: For file IO within an event loop, we should use io_uring
}

mod cmd {
    //! File control commands
    // Get file control flags.
    pub const F_GETFL: i32 = 3;
    // Set file control flags.
    pub const F_SETFL: i32 = 3;
}

mod fd_flags {
    //! File status flags. These are used by open(2) and fcntl(2)
    // No delay.
    // Note: Setting this to i32 feels a bit idiotic
    pub const O_NONBLOCK: i32 = 0x0000_0004;
}

// TODO: Handle errors from flags
fn set_nonblock(fd: i32) {
    let mut flags = unsafe { fcntl(fd, cmd::F_GETFL, 0) };

    crate::check_status!(flags);
    println!("{:#?}", flags);
    flags |= fd_flags::O_NONBLOCK;
    let status = unsafe { fcntl(fd, cmd::F_SETFL, flags) };
    crate::check_status!(status);
}

mod poll_flags {
    // Any readable data available
    pub const POLLIN: u16 = 0x0001;
    // File descriptor is writable
    pub const POLLOUT: u16 = 0x0004;
}

#[derive(Debug)]
#[repr(C)]
struct PollFd {
    // File descriptor to poll
    fd: u32,
    // Events to look for
    events: u16,
    // Events returned, which may occure or have occured.
    revents: u16,
}
