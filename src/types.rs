
#[derive(Debug)]
pub enum Error {
    Io(amd_flash::Error),
    EfsHeaderNotFound,
    PspDirectoryHeaderNotFound,
    PspDirectoryEntryTypeMismatch,
    BhdDirectoryHeaderNotFound,
    BhdDirectoryEntryTypeMismatch,
    DirectoryRangeCheck,
    DirectoryPayloadRangeCheck,
    Marshal,
    Overlap,
    Duplicate,
    Misaligned,
}

pub type Result<Q> = core::result::Result<Q, Error>;

impl From<amd_flash::Error> for Error {
    fn from(error: amd_flash::Error) -> Error {
        Error::Io(error)
    }
}

#[derive(Debug)]
pub enum ValueOrLocation {
    Value(u64),
    Location(u64),
}
