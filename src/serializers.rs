// This file contains the serializers for the ondisk formats.  These are meant to deserialize from a nice simple user-visible type and serialize into the nice simple user-visible type (can be lossy!).
// Also, serialization can fail if the nice simple user-visible type cannot represent what we are doing.

use crate::ondisk::*;
use core::convert::TryInto;

// Note: This is written such that it will fail if the underlying struct has fields added/removed/renamed, if those have a public setter.
macro_rules! make_serde{($StructName:ident, [$($field_name:ident),* $(,)?]
) => (
	paste::paste!{
		impl<'de> serde::de::Deserialize<'de> for $StructName {
			fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
			where D: serde::de::Deserializer<'de>, {
				let config = [<Serde $StructName>]::deserialize(deserializer)?;
				Ok($StructName::default()
				$(
				.[<with_ $field_name>](config.$field_name)
				)*)
		        }
		}
		impl serde::Serialize for $StructName {
			fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
			where S: serde::Serializer, {
				[<Serde $StructName>] {
					$(
						$field_name: self.$field_name().map_err(|_| serde::ser::Error::custom("value unknown"))?,
					)*
				}.serialize(serializer)
			}
		}
	}
)}

make_serde!(EfhNaplesSpiMode, [read_mode, fast_speed_new, micron_mode]);
make_serde!(EfhRomeSpiMode, [read_mode, fast_speed_new, micron_mode]);
make_serde!(
	Efh,
	[
		signature,
		bhd_directory_table_milan,
		xhci_fw_location,
		gbe_fw_location,
		imc_fw_location,
		low_power_promontory_firmware_location,
		promontory_firmware_location,
		psp_directory_table_location_naples,
		psp_directory_table_location_zen,
		spi_mode_zen_naples,
		spi_mode_zen_rome
	]
);
