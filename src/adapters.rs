use crate::flash;
use flash::ErasableLocation;
use flash::FlashAlign;
use flash::FlashRead;
use flash::FlashWrite;
use flash::Result;

const UPPER_HALF_OFFSET: u32 = 0x100_0000; // 16 MiB
const MODULUS: u32 = 0x200_0000; // 32 MiB

/// This is a flash adapter that allows you to simulate what AMD does when it's using the upper half of a 32 MiB flash chip.
/// Especially, it is the case that if locations are big enough (i.e. bit 24 set), then they refer to the lower half again.
pub struct Upper16MiBFlashAdapter<'a> {
    underlying_reader: &'a dyn FlashRead,
    underlying_writer: &'a dyn FlashWrite,
}

impl FlashRead for Upper16MiBFlashAdapter<'_> {
    fn read_exact(&self, offset: u32, buf: &mut [u8]) -> Result<()> {
        let offset = (offset + UPPER_HALF_OFFSET) % MODULUS;
        self.underlying_reader.read_exact(offset, buf)
    }
}

impl FlashAlign for Upper16MiBFlashAdapter<'_> {
    fn erasable_block_size(&self) -> usize {
        self.underlying_writer.erasable_block_size()
    }
}

impl FlashWrite for Upper16MiBFlashAdapter<'_> {
    fn erase_block(
        &self,
        offset: ErasableLocation,
    ) -> core::result::Result<(), flash::Error> {
        let offset = self.location(offset)?;
        let offset = (offset + UPPER_HALF_OFFSET) % MODULUS;
        self.underlying_writer.erase_block(self.erasable_location(offset)?)
    }
    fn erase_and_write_block(
        &self,
        offset: ErasableLocation,
        buf: &[u8],
    ) -> core::result::Result<(), flash::Error> {
        let offset = self.location(offset)?;
        let offset = (offset + UPPER_HALF_OFFSET) % MODULUS;
        self.underlying_writer
            .erase_and_write_block(self.erasable_location(offset)?, buf)
    }
}

impl<'a> Upper16MiBFlashAdapter<'a> {
    #[allow(dead_code)]
    pub fn new(
        underlying_reader: &'a dyn FlashRead,
        underlying_writer: &'a dyn FlashWrite,
    ) -> Self {
        Self { underlying_reader, underlying_writer }
    }
}
