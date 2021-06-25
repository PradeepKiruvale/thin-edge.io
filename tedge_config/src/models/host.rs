use std::convert::{TryFrom, TryInto};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Host(pub String);

#[derive(thiserror::Error, Debug)]
#[error("Invalid Host name: '{input}'.")]
pub struct InvalidHostName {
    input: String,
}

impl TryFrom<String> for Host {
    type Error = InvalidHostName;

    fn try_from(input: String) -> Result<Self, Self::Error> {
        Ok(Host(input))
    }
}

// impl TryInto<String> for Host {
//     type Error = std::convert::Infallible;

//     fn try_into(self) -> Result<String, Self::Error> {
//         Ok(self.0)
//     }
// }

impl Into<String> for Host {
    fn into(self) -> String {
        self.0
    }
}
