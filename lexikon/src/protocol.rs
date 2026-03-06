pub struct LexRequest {
    args: Vec<String>,
}

impl LexRequest {
    pub fn new(maybe_args: Option<Vec<String>>) -> Self {
        let args = if let Some(args) = maybe_args {
            args
        } else {
            vec![]
        };

        Self {
            args
        }
    }

    pub fn to_request(&self) -> Result<Vec<u8>, LexRequestError> {
        let mut request = vec![];
        // Compute the totsal size of the protocol
        // msg_len: u32 le -> 4 bytes
        // n_args: u32 le -> 4 bytes
        // arg elements, each having:
        //   u32 le 4 bytes len
        //   arg len
        let mut total_size = 4 + 4;
        for arg in &self.args {
            total_size += 4 + arg.len();
        }

        request.extend_from_slice(&u32::try_from(total_size)?.to_le_bytes());
        request.extend_from_slice(&u32::try_from(self.args.len())?.to_le_bytes());

        for arg in &self.args {
            request.extend_from_slice(&u32::try_from(arg.len())?.to_le_bytes());
            request.extend_from_slice(&arg.as_bytes());
        }

        Ok(request)
    }
}

#[derive(Debug)]
pub enum LexRequestError {
    TryFromIntError(std::num::TryFromIntError),
}

impl From<std::num::TryFromIntError> for LexRequestError {
    fn from(err: std::num::TryFromIntError) -> Self {
        Self::TryFromIntError(err)
    }
}
