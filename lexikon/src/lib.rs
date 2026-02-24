unsafe extern "C" {
    fn socket(domain: i32, t_type: i32, protocol: i32) -> i32;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket() {
        let fd = unsafe {
            socket(domain::AF_INET, socket_type::SOCK_STREAM, 0)
        };
        println!("{}", fd);
        assert!(fd != -1);
    }
}
