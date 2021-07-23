
#[derive(Debug)]
pub enum Error {
    IoError(amd_flash::Error),
    HeaderNotFound,
}

pub type Result<Q> = core::result::Result<Q, Error>;

impl From<amd_flash::Error> for Error {
    fn from(error: amd_flash::Error) -> Error {
        Error::IoError(error)
    }
}
