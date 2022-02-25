use core::convert::TryInto;

use amd_flash::ErasableLocation;
use amd_flash::FlashRead;
use amd_flash::FlashWrite;
use amd_flash::Location;
use amd_flash::Result;

const UPPER_HALF_OFFSET: u32 = 0x100_0000; // 16 MiB
const MODULUS: u32 = 0x200_0000; // 32 MiB

/// This is a flash adapter that allows you to simulate what AMD does when it's using the upper half of a 32 MiB flash chip.
/// Especially, it is the case that if locations are big enough (i.e. bit 24 set), then they refer to the lower half again.
pub struct Upper16MiBFlashAdapter<'a, const ERASABLE_BLOCK_SIZE: usize> {
	underlying_reader: &'a dyn FlashRead<ERASABLE_BLOCK_SIZE>,
	underlying_writer: &'a dyn FlashWrite<ERASABLE_BLOCK_SIZE>,
}

impl<const ERASABLE_BLOCK_SIZE: usize> FlashRead<ERASABLE_BLOCK_SIZE>
	for Upper16MiBFlashAdapter<'_, ERASABLE_BLOCK_SIZE>
{
	fn read_exact(&self, offset: u32, buf: &mut [u8]) -> Result<usize> {
		let offset = (offset + UPPER_HALF_OFFSET) % MODULUS;
		self.underlying_reader.read_exact(offset, buf)
	}
	fn read_erasable_block(
		&self,
		offset: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		buf: &mut [u8; ERASABLE_BLOCK_SIZE],
	) -> Result<()> {
		let offset = Location::from(offset);
		let offset = (offset + UPPER_HALF_OFFSET) % MODULUS;
		self.underlying_reader
			.read_erasable_block(offset.try_into()?, buf)
	}
}

impl<const ERASABLE_BLOCK_SIZE: usize> FlashWrite<ERASABLE_BLOCK_SIZE>
	for Upper16MiBFlashAdapter<'_, ERASABLE_BLOCK_SIZE>
{
	fn erase_block(
		&self,
		offset: ErasableLocation<ERASABLE_BLOCK_SIZE>,
	) -> core::result::Result<(), amd_flash::Error> {
		let offset = Location::from(offset);
		let offset = (offset + UPPER_HALF_OFFSET) % MODULUS;
		self.underlying_writer.erase_block(offset.try_into()?)
	}
	fn erase_and_write_block(
		&self,
		offset: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		buf: &[u8; ERASABLE_BLOCK_SIZE],
	) -> core::result::Result<(), amd_flash::Error> {
		let offset = Location::from(offset);
		let offset = (offset + UPPER_HALF_OFFSET) % MODULUS;
		self.underlying_writer
			.erase_and_write_block(offset.try_into()?, buf)
	}
}

impl<'a, const ERASABLE_BLOCK_SIZE: usize>
	Upper16MiBFlashAdapter<'a, ERASABLE_BLOCK_SIZE>
{
	pub fn new(
		underlying_reader: &'a dyn FlashRead<ERASABLE_BLOCK_SIZE>,
		underlying_writer: &'a dyn FlashWrite<ERASABLE_BLOCK_SIZE>,
	) -> Self {
		Self {
			underlying_reader,
			underlying_writer,
		}
	}
}
