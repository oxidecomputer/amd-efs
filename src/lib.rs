#![cfg_attr(not(feature = "std"), no_std)]

mod amdfletcher32;
mod efs;
mod ondisk;
mod adapters;
mod struct_accessors;
mod types;
mod serializers;
pub use crate::efs::BhdDirectory;
pub use crate::efs::Efs;
pub use crate::efs::ProcessorGeneration;
pub use crate::efs::PspDirectory;
pub use ondisk::*;
pub use types::Error;
pub use types::Result;
pub use crate::ondisk::ValueOrLocation;
pub use crate::efs::preferred_efh_location;

#[cfg(test)]
mod tests {
	#[test]
	fn it_works() {
		assert_eq!(2 + 2, 4);
	}
}
