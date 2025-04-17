// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::convert::TryInto;

/// This is any Location on the Flash chip
pub type Location = u32;

#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum IoError {
    #[cfg_attr(feature = "std", error("could not open backing store"))]
    Open,
    #[cfg_attr(
        feature = "std",
        error("could not read 0x{size:x} B starting at 0x{start:x} B")
    )]
    Read { start: Location, size: usize },
    #[cfg_attr(
        feature = "std",
        error("could not write 0x{size:x} B starting at 0x{start:x} B")
    )]
    Write { start: Location, size: usize },
    #[cfg_attr(
        feature = "std",
        error("could not erase 0x{size:x} B starting at 0x{start:x} B")
    )]
    Erase { start: Location, size: usize },
    #[cfg_attr(feature = "std", error("could not flush"))]
    Flush,
}

#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum Error {
    #[cfg_attr(feature = "std", error("io"))]
    Io(IoError),
    #[cfg_attr(
        feature = "std",
        error(
            "block not aligned for erase (block size = 0x{erasable_block_size:x} B, intra block offset = 0x{intra_block_offset:x} B)"
        )
    )]
    Alignment { erasable_block_size: usize, intra_block_offset: usize },
    #[cfg_attr(feature = "std", error("requested size is unavailable"))]
    Size,
}

pub type Result<Q> = core::result::Result<Q, Error>;

/// This is a Location which definitely is aligned on an erase block boundary
#[derive(Clone, Copy, Debug)]
pub struct ErasableLocation {
    location: Location,
    erasable_block_size: usize,
}

impl ErasableLocation {
    pub fn erasable_block_size(&self) -> usize {
        self.erasable_block_size
    }
    pub fn erasable_block_mask(&self) -> u32 {
        (self.erasable_block_size as u32) - 1
    }
    /// Note: Assumed beginning <= self, otherwise result will be 0.
    pub fn extent(beginning: Self, end: Self) -> u32 {
        let beginning = beginning.location;
        let end = end.location;
        end.saturating_sub(beginning)
    }
    pub fn advance(&self, amount: usize) -> Result<Self> {
        let mask = self.erasable_block_mask() as usize;
        if amount & mask != 0 {
            return Err(Error::Alignment {
                erasable_block_size: self.erasable_block_size,
                intra_block_offset: amount & mask,
            });
        }
        let pos = (self.location as usize)
            .checked_add(amount)
            .expect("Location within 4 GiB");
        Ok(Self {
            location: pos.try_into().expect("Location within 4 GiB"),
            erasable_block_size: self.erasable_block_size,
        })
    }
    pub fn advance_at_least(&self, amount: usize) -> Result<Self> {
        // Round up to a multiple of erasable_block_size()
        let diff =
            0usize.wrapping_sub(amount) & (self.erasable_block_mask() as usize);
        let amount = amount.checked_add(diff).expect("Location within 4 GiB");
        self.advance(amount)
    }
}

impl From<ErasableLocation> for Location {
    fn from(source: ErasableLocation) -> Self {
        source.location
    }
}

#[derive(Debug)]
pub struct ErasableRange {
    pub beginning: ErasableLocation, // note: same erasable_block_size assumed
    pub end: ErasableLocation,       // note: same erasable_block_size assumed
}
impl ErasableRange {
    pub fn new(beginning: ErasableLocation, end: ErasableLocation) -> Self {
        assert!(Location::from(beginning) <= Location::from(end));
        Self { beginning, end }
    }
    /// Splits the Range after at least SIZE Byte, if possible.
    /// Return the first part. Retain the second part.
    pub fn take_at_least(&mut self, size: usize) -> Option<Self> {
        let x_beginning = self.beginning;
        let x_end = self.beginning.advance_at_least(size).ok()?;
        if Location::from(x_end) <= Location::from(self.end) {
            *self = Self::new(x_end, self.end);
            Some(Self::new(x_beginning, x_end))
        } else {
            None
        }
    }
    /// in Byte
    pub fn capacity(&self) -> usize {
        ErasableLocation::extent(self.beginning, self.end) as usize
    }
}

pub trait FlashRead {
    /// Read exactly the right amount from the location BEGINNING to fill the
    /// entire BUFFER that was passed.
    fn read_exact(&self, beginning: Location, buffer: &mut [u8]) -> Result<()>;
}

pub trait FlashAlign {
    /// Note: Assumed constant for lifetime of instance.
    /// Note: Assumed to be a power of two.
    fn erasable_block_size(&self) -> usize;
    fn erasable_block_mask(&self) -> u32 {
        (self.erasable_block_size() as u32) - 1
    }
    fn is_aligned(&self, location: Location) -> bool {
        (location & self.erasable_block_mask()) == 0
    }
    /// Determine an erasable location, given a location, if possible.
    /// If not possible, return None.
    fn erasable_location(
        &self,
        location: Location,
    ) -> Result<ErasableLocation> {
        let erasable_block_size = self.erasable_block_size();
        if self.is_aligned(location) {
            Ok(ErasableLocation { location, erasable_block_size })
        } else {
            Err(Error::Alignment {
                erasable_block_size: self.erasable_block_size(),
                intra_block_offset: (location & self.erasable_block_mask())
                    as usize,
            })
        }
    }
    /// Given an erasable location, returns the corresponding location
    /// IF the erasable location is compatible with our instance.
    fn location(
        &self,
        erasable_location: ErasableLocation,
    ) -> Result<Location> {
        if erasable_location.erasable_block_size == self.erasable_block_size() {
            Ok(erasable_location.location)
        } else {
            Err(Error::Alignment {
                erasable_block_size: self.erasable_block_size(),
                intra_block_offset: erasable_location.erasable_block_size,
            })
        }
    }
}

pub trait FlashWrite: FlashRead + FlashAlign {
    /// Note: BUFFER.len() == erasable_block_size()
    fn read_erasable_block(
        &self,
        location: ErasableLocation,
        buffer: &mut [u8],
    ) -> Result<()> {
        assert_eq!(buffer.len(), self.erasable_block_size());
        self.read_exact(self.location(location)?, buffer)?;
        Ok(())
    }
    fn erase_block(&self, location: ErasableLocation) -> Result<()>;
    /// Note: If BUFFER.len() < erasable_block_size(), it has to erase the
    /// remainder anyway.
    fn erase_and_write_block(
        &self,
        location: ErasableLocation,
        buffer: &[u8],
    ) -> Result<()>;

    // FIXME: sanity check callers
    fn erase_and_write_blocks(
        &self,
        location: ErasableLocation,
        buf: &[u8],
    ) -> Result<()> {
        let mut location = location;
        let erasable_block_size = self.erasable_block_size();
        for chunk in buf.chunks(erasable_block_size) {
            self.erase_and_write_block(location, chunk)?;
            if chunk.len() != erasable_block_size {
                // TODO: Only allow on last chunk
                break;
            }
            location = location.advance(erasable_block_size)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;
    const KIB: usize = 1024; // B
    const ERASABLE_BLOCK_SIZE: usize = 128 * KIB;

    struct FlashImage<'a> {
        buf: RefCell<&'a mut [u8]>,
        erasable_block_size: usize,
    }

    impl<'a> FlashImage<'a> {
        pub fn new(buf: &'a mut [u8]) -> Self {
            Self {
                buf: RefCell::new(buf),
                erasable_block_size: ERASABLE_BLOCK_SIZE,
            }
        }
        pub fn new_block_size(
            buf: &'a mut [u8],
            erasable_block_size: usize,
        ) -> Self {
            Self { buf: RefCell::new(buf), erasable_block_size }
        }
    }

    impl FlashRead for FlashImage<'_> {
        fn read_exact(
            &self,
            location: Location,
            buffer: &mut [u8],
        ) -> Result<()> {
            let len = buffer.len();
            let buf = self.buf.borrow();
            let block = &buf[location as usize..];
            let block = &block[0..len];
            buffer[..].copy_from_slice(block);
            Ok(())
        }
    }

    impl FlashAlign for FlashImage<'_> {
        fn erasable_block_size(&self) -> usize {
            self.erasable_block_size
        }
    }
    impl FlashWrite for FlashImage<'_> {
        fn read_erasable_block(
            &self,
            location: ErasableLocation,
            buffer: &mut [u8],
        ) -> Result<()> {
            let erasable_block_size = self.erasable_block_size();
            let location: Location = location.into();
            let buf = self.buf.borrow();
            let block = &buf
                [location as usize..(location as usize + erasable_block_size)];
            assert_eq!(buffer.len(), erasable_block_size);
            buffer[..].copy_from_slice(block);
            Ok(())
        }
        fn erase_block(&self, location: ErasableLocation) -> Result<()> {
            let location: Location = location.into();
            let mut buf = self.buf.borrow_mut();
            let block = &mut buf[location as usize
                ..(location as usize + self.erasable_block_size())];
            block.fill(0xff);
            Ok(())
        }
        fn erase_and_write_block(
            &self,
            location: ErasableLocation,
            buffer: &[u8],
        ) -> Result<()> {
            let location: Location = location.into();
            let mut buf = self.buf.borrow_mut();
            let block = &mut buf[location as usize
                ..(location as usize + self.erasable_block_size())];
            block.copy_from_slice(&buffer[..]);
            Ok(())
        }
    }

    #[test]
    fn flash_image_usage() -> Result<()> {
        let mut storage = [0xFFu8; 256 * KIB];
        let flash_image = FlashImage::new(&mut storage[..]);
        let beginning_1 =
            flash_image.erasable_location(Location::from(0u32)).unwrap();
        let erasable_block_size = ERASABLE_BLOCK_SIZE;
        flash_image
            .erase_and_write_block(beginning_1, &[1u8; ERASABLE_BLOCK_SIZE])?;
        let beginning_2 = flash_image
            .erasable_location(Location::from(erasable_block_size as u32))
            .unwrap();
        flash_image
            .erase_and_write_block(beginning_2, &[2u8; ERASABLE_BLOCK_SIZE])?;
        let mut buf: [u8; ERASABLE_BLOCK_SIZE] = [0u8; ERASABLE_BLOCK_SIZE];
        flash_image.read_exact(0, &mut buf)?;
        assert_eq!(buf, [1u8; ERASABLE_BLOCK_SIZE]);
        flash_image
            .read_exact(Location::from(erasable_block_size as u32), &mut buf)?;
        assert_eq!(buf, [2u8; ERASABLE_BLOCK_SIZE]);
        Ok(())
    }

    #[test]
    #[should_panic]
    fn flash_image_misaligned_erasure() {
        let mut storage = [0xFFu8; 256 * KIB];
        let flash_image = FlashImage::new(&mut storage[..]);
        flash_image.erasable_location(Location::from(1u32)).unwrap();
    }

    #[test]
    #[should_panic(expected = "Alignment")]
    fn flash_image_misaligned_advancement() {
        let mut storage = [0xFFu8; 256 * KIB];
        let flash_image = FlashImage::new(&mut storage[..]);
        let beginning_1 =
            flash_image.erasable_location(Location::from(0u32)).unwrap();
        beginning_1.advance(1).unwrap();
    }

    #[test]
    fn flash_image_aligned_advancement() {
        let mut storage = [0xFFu8; 256 * KIB];
        let flash_image = FlashImage::new(&mut storage[..]);
        let beginning_1 =
            flash_image.erasable_location(Location::from(0u32)).unwrap();
        beginning_1.advance_at_least(1).unwrap();
    }

    #[test]
    #[should_panic(expected = "Alignment")]
    fn flash_image_mistaken_storage() {
        let mut storage_0 = [0xFFu8; 256 * KIB];
        let flash_image_0 = FlashImage::new(&mut storage_0[..]);
        let mut storage_1 = [0xFFu8; 256 * KIB];
        let flash_image_1 =
            FlashImage::new_block_size(&mut storage_1[..], 1 * KIB);
        let beginning_0 =
            flash_image_0.erasable_location(Location::from(0u32)).unwrap();
        flash_image_1.location(beginning_0).unwrap();
    }

    #[test]
    #[should_panic(expected = "assertion")]
    fn flash_image_mistaken_buffer() {
        let mut storage_0 = [0xFFu8; 256 * KIB];
        let flash_image_0 = FlashImage::new(&mut storage_0[..]);
        let mut buffer = [0xFFu8; 1 * KIB];
        let beginning_0 =
            flash_image_0.erasable_location(Location::from(0u32)).unwrap();
        flash_image_0.read_erasable_block(beginning_0, &mut buffer).unwrap();
    }

    #[test]
    #[should_panic(expected = "Size")]
    fn flash_image_too_small() {
        use crate::allocators::FlashAllocate;
        let mut storage_0 = [0xFFu8; 256 * KIB];
        let flash_image_0 = FlashImage::new(&mut storage_0[..]);
        let beginning =
            flash_image_0.erasable_location(Location::from(0u32)).unwrap();
        let end = beginning.advance(256 * KIB).unwrap();
        let mut allocator = crate::allocators::ArenaFlashAllocator::new(
            0,
            257 * KIB,
            ErasableRange { beginning, end },
        )
        .unwrap();
        allocator.take_at_least(1).unwrap();
    }
    /* Test that has two different storages with different alignment is not present. */
}
