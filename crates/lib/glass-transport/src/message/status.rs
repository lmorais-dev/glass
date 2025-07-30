use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum Status {
    Success = 0,
    Internal = 1,
    Protocol = 2,
    Unknown = 3,

    NoSuchService = 10,
    NoSuchMethod = 11,

    BadRequest = 20,

    Custom(u8),
}

impl From<Status> for u8 {
    fn from(status: Status) -> Self {
        match status {
            Status::Custom(custom) => custom,
            _ => status.into(),
        }
    }
}

impl TryFrom<u8> for Status {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Status::Success),
            1 => Ok(Status::Internal),
            2 => Ok(Status::Protocol),
            3 => Ok(Status::Unknown),

            10 => Ok(Status::NoSuchService),
            11 => Ok(Status::NoSuchMethod),

            20 => Ok(Status::BadRequest),

            custom => Ok(Status::Custom(custom)),
        }
    }
}
