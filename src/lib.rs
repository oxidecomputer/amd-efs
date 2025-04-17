#![cfg_attr(not(feature = "std"), no_std)]

mod adapters;
pub mod allocators;
mod amdfletcher32;
mod efs;
pub mod flash;
mod ondisk;
mod serializers;
mod struct_accessors;
mod types;
pub use crate::efs::BhdDirectory;
pub use crate::efs::ComboDirectory;
pub use crate::efs::Efs;
pub use crate::efs::ProcessorGeneration;
pub use crate::efs::PspDirectory;
pub use crate::efs::preferred_efh_location;
pub use crate::ondisk::ValueOrLocation;
pub use ondisk::*;
pub use types::Error;
pub use types::Result;
