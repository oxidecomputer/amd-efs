#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum Error {
    #[cfg_attr(feature = "std", error("io {0}"))]
    Io(amd_flash::Error),
    #[cfg_attr(feature = "std", error("efs header not found"))]
    EfsHeaderNotFound,
    #[cfg_attr(feature = "std", error("efs range check"))]
    EfsRangeCheck,
    #[cfg_attr(feature = "std", error("psp directory header not found"))]
    PspDirectoryHeaderNotFound,
    #[cfg_attr(feature = "std", error("bhd directory header not found"))]
    BhdDirectoryHeaderNotFound,
    #[cfg_attr(
        feature = "std",
        error("directory payload not aligned to 4 kiB")
    )]
    DirectoryPayloadMisaligned,
    #[cfg_attr(feature = "std", error("directory range check"))]
    DirectoryRangeCheck,
    #[cfg_attr(feature = "std", error("directory payload range check"))]
    DirectoryPayloadRangeCheck,
    #[cfg_attr(feature = "std", error("marshal"))]
    Marshal,
    #[cfg_attr(feature = "std", error("overlap"))]
    Overlap,
    #[cfg_attr(feature = "std", error("duplicate"))]
    Duplicate,
    #[cfg_attr(feature = "std", error("misaligned"))]
    Misaligned,
    #[cfg_attr(feature = "std", error("entry type mismatch"))]
    EntryTypeMismatch,
    #[cfg_attr(feature = "std", error("entry not found"))]
    EntryNotFound,
    #[cfg_attr(feature = "std", error("directory type mismatch"))]
    DirectoryTypeMismatch,
    #[cfg_attr(feature = "std", error("spi mode mismatch"))]
    SpiModeMismatch,
}

pub type Result<Q> = core::result::Result<Q, Error>;

impl From<amd_flash::Error> for Error {
    fn from(error: amd_flash::Error) -> Error {
        Error::Io(error)
    }
}
