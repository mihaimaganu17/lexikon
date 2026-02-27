unsafe extern "C" {
    // File control -> fcntl() provides for control over descriptors.  The argument fildes is a
    // descriptor to be operated on by cmd. In particular, the C version supports variadic
    // arguments, but for our purposes, we only need 1 argument because we only want to set the
    // flags with F_SETFL and F_GETFL. In the case of F_GETFL, the `value` argument is ignored.
    fn fcntl(filedes: i32, cmd: i32, value: i32) -> i32;
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
    let mut flags = unsafe {
        fcntl(fd, cmd::F_GETFL, 0)
    };

    crate::check_status!(flags);
    println!("{:#?}", flags);
    flags |= fd_flags::O_NONBLOCK;
    let status = unsafe {
        fcntl(fd, cmd::F_SETFL, flags)
    };
    crate::check_status!(status);
}
