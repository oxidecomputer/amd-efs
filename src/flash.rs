// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::convert::TryInto;

#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum Error {
    #[cfg_attr(feature = "std", error("io"))]
    Io,
    #[cfg_attr(
        feature = "std",
        error("alignment is not good enough for erasability of block")
    )]
    Alignment,
    #[cfg_attr(feature = "std", error("programmer violated an invariant"))]
    Programmer,
    #[cfg_attr(
        feature = "std",
        error("no area of requested size is available")
    )]
    Size,
}

pub type Result<Q> = core::result::Result<Q, Error>;

/// This is any Location on the Flash chip
pub type Location = u32;

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
        if amount & (self.erasable_block_mask() as usize) != 0 {
            return Err(Error::Alignment);
        }
        let pos = (self.location as usize)
            .checked_add(amount)
            .ok_or(Error::Alignment)?;
        Ok(Self {
            location: pos.try_into().map_err(|_| Error::Alignment)?,
            erasable_block_size: self.erasable_block_size,
        })
    }
    pub fn advance_at_least(&self, amount: usize) -> Result<Self> {
        // Round up to a multiple of erasable_block_size()
        let diff =
            0usize.wrapping_sub(amount) & (self.erasable_block_mask() as usize);
        let amount = amount.checked_add(diff).ok_or(Error::Alignment)?;
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
        assert!(Location::from(beginning) <= Location::from(end)); // TODO nicer
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
    ) -> Option<ErasableLocation> {
        let erasable_block_size = self.erasable_block_size();
        self.is_aligned(location)
            .then_some(ErasableLocation { location, erasable_block_size })
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
            Err(Error::Alignment)
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
        if buffer.len() != self.erasable_block_size() {
            return Err(Error::Programmer);
        }
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
            if buffer.len() != erasable_block_size {
                return Err(Error::Programmer);
            }
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
}
