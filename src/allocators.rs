use crate::flash::{ErasableRange, Location};
use crate::flash::{Error, Result};

pub trait FlashAllocate {
    fn take_at_least(&mut self, size: usize) -> Option<ErasableRange>;
    fn max_contiguous_capacity(&self) -> usize;
}

pub struct ArenaFlashAllocator {
    _efh_range: ErasableRange,
    free_ranges: [ErasableRange; 2],
}

impl ArenaFlashAllocator {
    /// Creates a new allocator that will use parts of the given ARENA.
    /// Depending on processor generation, a part of it will be cut out
    /// and not given to the user (since it needs to be at a fixed
    /// spot and also is used by us).
    pub fn new(
        efh_beginning: Location,
        efh_size: usize,
        arena: ErasableRange,
    ) -> Result<Self> {
        let mut arena = arena;
        assert!(Location::from(arena.beginning) == 0);
        // Avoid EFH_BEGINNING..(EFH_BEGINNING + EFH_SIZE)
        let a_size = efh_beginning as usize;
        let a = arena.take_at_least(a_size).ok_or(Error::Size)?;
        assert!(Location::from(a.end) as usize == a_size);
        let _efh_range = arena.take_at_least(efh_size).ok_or(Error::Size)?;
        Ok(Self { _efh_range, free_ranges: [a, arena] })
    }
}
impl FlashAllocate for ArenaFlashAllocator {
    /// From the free ranges, take a range of at least SIZE Bytes,
    /// if possible. Otherwise return None.
    fn take_at_least(&mut self, size: usize) -> Option<ErasableRange> {
        self.free_ranges[0]
            .take_at_least(size)
            .or_else(|| self.free_ranges[1].take_at_least(size))
    }
    fn max_contiguous_capacity(&self) -> usize {
        let mut max_capacity = 0usize;
        for range in &self.free_ranges {
            let capacity = range.capacity();
            if capacity > max_capacity {
                max_capacity = capacity
            }
        }
        max_capacity
    }
}

#[cfg(test)]
mod allocator_tests {
    use super::*;
    use crate::flash::FlashAlign;
    fn intersect(
        a: &ErasableRange,
        b: &ErasableRange,
    ) -> Option<(Location, Location)> {
        let new_beginning =
            Location::from(a.beginning).max(Location::from(b.beginning));
        let new_end = Location::from(a.end).min(Location::from(b.end));
        if new_beginning < new_end {
            Some((new_beginning, new_end))
        } else {
            None
        }
    }
    struct Buffer {}
    impl FlashAlign for Buffer {
        fn erasable_block_size(&self) -> usize {
            4
        }
    }
    impl Buffer {
        fn allocator(&self) -> ArenaFlashAllocator {
            let beginning = self.erasable_location(0).unwrap();
            let end = beginning.advance_at_least(0x4_0000).unwrap();
            ArenaFlashAllocator::new(
                0x2_0000,
                0x200,
                ErasableRange::new(beginning, end),
            )
            .unwrap()
        }
        fn efh_range(&self) -> ErasableRange {
            ErasableRange::new(
                self.erasable_location(0x2_0000).unwrap(),
                self.erasable_location(0x2_0000)
                    .unwrap()
                    .advance_at_least(0x200)
                    .unwrap(),
            )
        }
    }
    #[test]
    fn test_allocator_1() {
        let buf = Buffer {};
        let mut allocator = buf.allocator();
        let efh_range = buf.efh_range();
        let a = allocator.take_at_least(42).unwrap();
        let b = allocator.take_at_least(100).unwrap();
        assert!(intersect(&a, &b).is_none());
        assert!(intersect(&a, &efh_range).is_none());
        assert!(intersect(&b, &efh_range).is_none());
        assert!(Location::from(b.end) < 0x2_0000);
    }

    #[test]
    fn test_allocator_2() {
        let buf = Buffer {};
        let mut allocator = buf.allocator();
        let efh_range = buf.efh_range();
        let a = allocator.take_at_least(0x2_0000).unwrap();
        let b = allocator.take_at_least(100).unwrap();
        assert!(intersect(&a, &b).is_none());
        assert!(intersect(&a, &efh_range).is_none());
        assert!(intersect(&b, &efh_range).is_none());
        assert!(Location::from(b.end) < 0x4_0000);
    }

    #[test]
    fn test_allocator_3() {
        let buf = Buffer {};
        let mut allocator = buf.allocator();
        let efh_range = buf.efh_range();
        let a = allocator.take_at_least(0x1_fff8).unwrap();
        let b = allocator.take_at_least(100).unwrap();
        assert!(intersect(&a, &b).is_none());
        assert!(intersect(&a, &efh_range).is_none());
        assert!(intersect(&b, &efh_range).is_none());
        assert!(Location::from(b.end) < 0x4_0000);
        assert!(Location::from(b.beginning) > 0x2_0000);
    }
}
