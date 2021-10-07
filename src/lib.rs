
#![cfg_attr(not(feature = "std"), no_std)]

mod amdfletcher32;
mod efs;
mod ondisk;
mod types;
pub use types::Result;
pub use types::Error;
pub use crate::efs::Efs;
pub use crate::efs::BhdDirectory;
pub use crate::efs::ProcessorGeneration;
pub use crate::efs::PspDirectory;
pub use ondisk::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
