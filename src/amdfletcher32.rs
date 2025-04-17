use core::ops::{Add, AddAssign, BitAnd, BitOr, Shl, Shr};
use fletcher::Fletcher;
use fletcher::FletcherAccumulator;

#[derive(Clone, Copy, PartialEq)]
pub struct Wu32(u32);

impl Wu32 {
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Add for Wu32 {
    type Output = Self;
    fn add(self, other: Self) -> <Self as Add<Self>>::Output {
        Self(self.0.add(other.0))
    }
}

impl AddAssign for Wu32 {
    fn add_assign(&mut self, other: Self) {
        self.0 = self.0.add(other.0)
    }
}

impl BitAnd for Wu32 {
    type Output = Self;
    fn bitand(self, other: Self) -> Self::Output {
        Self(self.0.bitand(other.0))
    }
}

impl BitOr for Wu32 {
    type Output = Self;
    fn bitor(self, other: Self) -> Self::Output {
        Self(self.0.bitor(other.0))
    }
}

impl Shr<u16> for Wu32 {
    type Output = Self;
    fn shr(self, bits: u16) -> Self::Output {
        Self(self.0.shr(bits))
    }
}

impl Shl<u16> for Wu32 {
    type Output = Self;
    fn shl(self, bits: u16) -> Self::Output {
        Self(self.0.shl(bits))
    }
}

impl From<u16> for Wu32 {
    fn from(value: u16) -> Self {
        Self(value.into())
    }
}

pub type AmdFletcher32 = Fletcher<Wu32>;

impl FletcherAccumulator for Wu32 {
    type InputType = u16;
    const BIT_MASK: Self = Wu32(0xffff);
    const MAX_CHUNK_SIZE: usize = 359;
    const SHIFT_AMOUNT: u16 = 16;
}

impl Default for Wu32 {
    fn default() -> Self {
        Wu32(0x0000ffff)
    }
}
