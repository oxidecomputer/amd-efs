use core::ops::{Add, BitAnd, BitOr, Shl, Shr};
use fletcher::generic_fletcher::Fletcher;
use fletcher::generic_fletcher::FletcherAccumulator;

#[derive(Clone, Copy)]
pub struct Wu32(u32);

impl Wu32 {
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Add for Wu32 {
    type Output = Self;
    fn add(self, other: Self) -> <Self as Add<Self>>::Output { todo!() }
}

impl BitAnd for Wu32 {
    type Output = Self;
    fn bitand(self, other: Self) -> Self::Output { todo!() }
}

impl BitOr for Wu32 {
    type Output = Self;
    fn bitor(self, other: Self) -> Self::Output { todo!() }
}

impl Shr for Wu32 {
    type Output = Self;
    fn shr(self, other: Self) -> Self::Output { todo!() }
}

impl Shl for Wu32 {
    type Output = Self;
    fn shl(self, other: Self) -> Self::Output { todo!() }
}

impl From<u16> for Wu32 {
    fn from(value: u16) -> Self {
        Self(value.into())
    }
}

pub type AmdFletcher32 = Fletcher<Wu32, u16>;

impl FletcherAccumulator<u16> for Wu32 {
    fn default_value() -> Self {
        Wu32(0x0000ffff)
    }

    fn max_chunk_size() -> usize {
        359
    }

    fn combine(lower: &Self, upper: &Self) -> Self {
        *lower | (*upper << Wu32(16))
    }

    fn reduce(self) -> Self {
        (self & Wu32(0xffff)) + (self >> Wu32(16))
    }
}
